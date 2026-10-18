"""Source-ledger and targeted-contour controls for the extra real VMB corpus."""
import copy
import json
import shutil
import tempfile
import unittest
from decimal import Decimal
from pathlib import Path

from vmb_book_inputs import canonical_bytes,digest
from vmb_book_thin import (CASE_IDS,damage,damage_controls,prepare,render,source_inputs,verify_job,verify_pdf)


FIXTURE=Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book'


def update_inventory(root):
    path=root/'fixture-index.json'
    index=json.loads(path.read_bytes())
    index['files']=[{'path':str(p.relative_to(root)),'byte_length':p.stat().st_size,'sha256':digest(p.read_bytes())}
        for p in sorted(root.rglob('*')) if p.is_file() and p!=path]
    path.write_bytes(canonical_bytes(index))


class ThinInputTests(unittest.TestCase):
    def copied(self, directory):
        result=Path(directory)/'fixture'
        shutil.copytree(FIXTURE,result)
        return result

    def mutate(self, root, name, action, *, canonical=True):
        path=root/name
        value=json.loads(path.read_bytes())
        action(value)
        path.write_bytes(canonical_bytes(value) if canonical else (json.dumps(value,indent=2)+'\n').encode())
        update_inventory(root)

    def test_all_eight_authored_cases_have_the_real_thin_contribution(self):
        facts=source_inputs(FIXTURE)
        self.assertEqual([case['case_id'] for case in facts['cases']],CASE_IDS)
        self.assertEqual([case['thin_path_index'] for case in facts['cases']],[2]*8)
        self.assertEqual(len({case['source_svg_sha256'] for case in facts['cases']}),4)
        self.assertEqual(len({case['admitted_svg_sha256'] for case in facts['cases']}),6)
        self.assertEqual([row['node_id'] for row in facts['occurrences']],[2,4,5,6,8,10,11,12])

    def test_missing_cases_reordered_sources_and_other_contours_are_rejected(self):
        changes=[lambda rows:rows.pop(),lambda rows:rows.reverse(),
            lambda rows:rows[0].__setitem__('thin_path_index',1),
            lambda rows:rows[4].__setitem__('thin_path_index',False),
            lambda rows:rows[4].__setitem__('thin_path_kind','negation'),
            lambda rows:rows[4].__setitem__('alt','not P')]
        for action in changes:
            with self.subTest(action=repr(action)),tempfile.TemporaryDirectory() as directory:
                fixture=self.copied(directory)
                self.mutate(fixture,'thin-strokes/cases.json',action)
                with self.assertRaises(ValueError):
                    source_inputs(fixture)

    def test_occurrence_aliases_boolean_ids_and_source_pointers_cannot_change(self):
        changes=[lambda rows:rows[1].__setitem__('image_id',0),
            lambda rows:rows[0].__setitem__('node_id',True),
            lambda rows:rows[1].__setitem__('source_pointer','/cases/0'),lambda rows:rows.pop()]
        for action in changes:
            with self.subTest(action=repr(action)),tempfile.TemporaryDirectory() as directory:
                fixture=self.copied(directory)
                self.mutate(fixture,'thin-strokes/occurrences.json',action)
                with self.assertRaises(ValueError):
                    source_inputs(fixture)

    def test_ungrouped_precomposed_negation_is_not_the_overlay_fixture(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture=self.copied(directory)
            self.mutate(fixture,'thin-strokes/engine/fixture-index.json',
                lambda value:value['cases'][4].__setitem__('tex',r'a\not=b'),canonical=False)
            # Update the outer provenance hash as well: the inner authored
            # request requirement must still reject this substitution.
            self.mutate(fixture,'thin-strokes/provenance.json',lambda value:value.__setitem__(
                'engine_index_sha256',digest((fixture/'thin-strokes/engine/fixture-index.json').read_bytes())))
            with self.assertRaisesRegex(ValueError,'source and semantics'):
                source_inputs(fixture)

    def test_engine_bundle_identity_cannot_be_relabelled_at_the_same_version(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture=self.copied(directory)
            self.mutate(fixture,'thin-strokes/engine/fixture-index.json',lambda value:value['cases'][0][
                'engine_artifact'].__setitem__('EngineBundleSHA256','sha256:'+'a'*64),canonical=False)
            self.mutate(fixture,'thin-strokes/provenance.json',lambda value:value.__setitem__(
                'engine_index_sha256',digest((fixture/'thin-strokes/engine/fixture-index.json').read_bytes())))
            with self.assertRaisesRegex(ValueError,'fixed thin-stroke engine artifact'):
                source_inputs(fixture)

    def test_missing_registration_or_changed_producer_source_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture=self.copied(directory)
            self.mutate(fixture,'fixture-index.json',lambda value:value['provenance'].pop('additional_corpora'))
            with self.assertRaisesRegex(ValueError,'registered complete'):
                source_inputs(fixture)
        with tempfile.TemporaryDirectory() as directory:
            fixture=self.copied(directory)
            path=fixture/'thin-strokes/producer-overlay.go'
            path.write_bytes(path.read_bytes()+b'\n')
            update_inventory(fixture)
            with self.assertRaisesRegex(ValueError,'generator source'):
                source_inputs(fixture)

    def test_prepared_job_cannot_change_source_metrics_text_or_font_bytes(self):
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)/'job'
            result=prepare(FIXTURE,job)
            self.assertEqual((result['cases'],result['placements']),(8,8))
            path=job/'document-package.json'
            original=path.read_bytes()
            package=json.loads(original)
            changes=[lambda p:p['document']['blocks'][0]['children'][0]['metrics'].__setitem__('baseline',1),
                lambda p:p['document']['blocks'][0]['children'][0].__setitem__('image_id',True),
                lambda p:p['text_buffers'][0].__setitem__('utf8','different'),
                lambda p:p['page_masters']['masters'][0].__setitem__('width',1)]
            for action in changes:
                value=copy.deepcopy(package)
                action(value)
                path.write_bytes(canonical_bytes(value))
                with self.assertRaises(ValueError):
                    verify_job(FIXTURE,job)
            path.write_bytes(original)
            font=job/package['resources']['font_faces'][0]['uri']
            font.write_bytes(font.read_bytes()+b'\n')
            with self.assertRaisesRegex(ValueError,'font bytes'):
                verify_job(FIXTURE,job)

    def test_all_output_entry_points_protect_the_fixture_and_job(self):
        with self.assertRaisesRegex(ValueError,'outside source inputs'):
            prepare(FIXTURE,FIXTURE/'bad-output')
        with tempfile.TemporaryDirectory() as directory:
            root,job=Path(directory)/'fixture',Path(directory)/'job'
            root.mkdir();job.mkdir()
            for source in (root,job):
                with self.assertRaisesRegex(ValueError,'outside all source inputs'):
                    render(root,job,job/'missing.pdf',{},source/'bad',tools={})
                with self.assertRaisesRegex(ValueError,'outside all source inputs'):
                    damage_controls(root,job,job/'missing.pdf',{},source/'bad',tools={})
                with self.assertRaisesRegex(ValueError,'outside all source inputs'):
                    verify_pdf(root,job,job/'missing.pdf',source/'bad',tools={})


class ContourDamageTests(unittest.TestCase):
    DATA=(b'q\n0 0 10 20 re W n\nq\n1 1 m\n2 1 l\n2 2 l\nh\nf\nQ\n'
          b'q\n3 3 m\n4 3 l\n4 4 l\nh\nf\nQ\nq\n5 5 m\n6 6 l\n6 7 l\nh\nf\nQ\nQ\n')

    def test_only_target_fill_or_contour_changes_and_other_operands_are_exact(self):
        deleted=damage(self.DATA,'deleted',2,Decimal(6),Decimal(20),3)
        self.assertEqual(deleted.replace(b'h\nn\nQ',b'h\nf\nQ'),self.DATA)
        duplicated=damage(self.DATA,'duplicated',2,Decimal(6),Decimal(20),3)
        added=b'\n5 5 m\n6 6 l\n6 7 l\nh\nf'
        self.assertEqual(duplicated.replace(added,b'',1),self.DATA)
        clipped=damage(self.DATA,'clipped',2,Decimal(6),Decimal(20),3)
        self.assertEqual(clipped.replace(b'q\n0 0 6 20 re W n\n',b'',1).replace(b'f\nQ\nQ\nQ',b'f\nQ\nQ'),self.DATA)
        self.assertEqual(self.DATA.count(b'\nf\n'),3)
        self.assertEqual(duplicated.count(b'\nf\n'),4)

    def test_wrong_case_count_noninteger_target_and_unknown_mutation_fail(self):
        for args in [('deleted',2,4),('deleted',True,3),('deleted',3,3),('other',2,3)]:
            with self.subTest(args=args),self.assertRaises(ValueError):
                damage(self.DATA,args[0],args[1],Decimal(6),Decimal(20),args[2])


if __name__=='__main__':
    unittest.main()
