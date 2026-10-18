"""Source-inventory and zero-test controls for the authored engine ledger."""
import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from prepare_vmb_book_fixtures import prepare
from vmb_book_inputs import canonical_bytes, digest, geometry, REQUIRED_CLI_TESTS
from vmb_book_inputs import pointer, read_canonical, verify_fixture_inputs, verify_prepared_job, verify_test_names


FIXTURE = Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book'


class VmbBookInputsTests(unittest.TestCase):
    def fixture(self, directory):
        target = Path(directory)/'fixture'
        shutil.copytree(FIXTURE,target)
        return target

    def reindex(self, fixture):
        index = read_canonical(fixture,'fixture-index.json')
        index['files'] = [{'path':str(path.relative_to(fixture)),'byte_length':path.stat().st_size,
                           'sha256':digest(path.read_bytes())}
                          for path in sorted(fixture.rglob('*')) if path.is_file() and path.name != 'fixture-index.json']
        # Include nested engine and raster indices, which are ordinary input files.
        for path in sorted(fixture.rglob('fixture-index.json')):
            if path != fixture/'fixture-index.json':
                index['files'].append({'path':str(path.relative_to(fixture)),'byte_length':path.stat().st_size,
                                       'sha256':digest(path.read_bytes())})
        index['files'].sort(key=lambda item:item['path'])
        (fixture/'fixture-index.json').write_bytes(canonical_bytes(index))

    def mutate_ledger(self, fixture, name, change):
        value = read_canonical(fixture,name)
        change(value)
        (fixture/name).write_bytes(canonical_bytes(value))
        self.reindex(fixture)

    def test_real_inputs_and_prepared_package_preserve_all_source_occurrences(self):
        facts = verify_fixture_inputs(FIXTURE)
        self.assertEqual((len(facts['cases']),len(facts['occurrences'])),(20,20))
        self.assertEqual(sum(item['paths'] for item in facts['geometry']),176)
        self.assertTrue(all(item['stored_segments'] == item['source_segments']+5
                            for item in facts['geometry']))
        with tempfile.TemporaryDirectory() as directory:
            job = Path(directory)/'job'
            first = prepare(FIXTURE,job)
            self.assertEqual(first,verify_prepared_job(job,FIXTURE))
            other = Path(directory)/'other'
            self.assertEqual(first,prepare(FIXTURE,other))
            self.assertEqual((job/'document-package.json').read_bytes(),(other/'document-package.json').read_bytes())
            with self.assertRaises(FileExistsError):
                prepare(FIXTURE,job)

    def test_canonical_ledgers_use_utf16_order_and_reject_floats_or_pretty_json(self):
        self.assertEqual(canonical_bytes({'\ue000':1,'\U00010000':2}),'{"𐀀":2,"\ue000":1}\n'.encode())
        for value in (1.0,float('nan'),2**53,'\ud800',{'\udfff':0}):
            with self.subTest(value=repr(value)),self.assertRaises(ValueError):
                canonical_bytes(value)
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.fixture(directory)
            value = read_canonical(fixture,'cases.json')
            (fixture/'cases.json').write_text(json.dumps(value,indent=2)+'\n')
            self.reindex(fixture)
            with self.assertRaisesRegex(ValueError,'canonical JCS'):
                verify_fixture_inputs(fixture)

    def test_file_inventory_rejects_added_missing_modified_and_symlink_inputs(self):
        for kind in ('added','missing','modified','symlink'):
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as directory:
                fixture = self.fixture(directory)
                path = fixture/'tex/fraction-inline-720896.tex'
                if kind == 'added':
                    (fixture/'unexpected.txt').write_bytes(b'new')
                elif kind == 'missing':
                    path.unlink()
                elif kind == 'modified':
                    path.write_bytes(path.read_bytes()+b' ')
                else:
                    data = path.read_bytes()
                    external = Path(directory)/'outside.tex'
                    external.write_bytes(data)
                    path.unlink();path.symlink_to(external)
                with self.assertRaises((ValueError,FileNotFoundError)):
                    verify_fixture_inputs(fixture)

    def test_case_set_cannot_shrink_duplicate_or_lose_its_source_binding(self):
        changes = [lambda cases:cases.pop(),lambda cases:cases.__setitem__(1,copy.deepcopy(cases[0])),
            lambda cases:cases[0].__setitem__('engine_source_pointer','/cases/01'),
            lambda cases:cases[0].__setitem__('alt','onehalf equals two quarters'),
            lambda cases:cases[0]['metrics'].__setitem__('origin_x',0),
            lambda cases:cases[0].__setitem__('expected_path_work',106)]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory:
                fixture = self.fixture(directory)
                self.mutate_ledger(fixture,'cases.json',change)
                with self.assertRaises(ValueError):
                    verify_fixture_inputs(fixture)
        with tempfile.TemporaryDirectory() as directory:
            fixture = self.fixture(directory)
            self.mutate_ledger(fixture,'fixture-index.json',lambda index:index.__setitem__('case_ids',index['case_ids'][:-1]))
            with self.assertRaisesRegex(ValueError,'required fixture case set'):
                verify_fixture_inputs(fixture)

    def test_occurrences_reject_boolean_ids_duplicate_nodes_reordering_and_wrong_pointers(self):
        changes = [lambda rows:rows[0].__setitem__('image_id',False),
            lambda rows:rows[1].__setitem__('node_id',rows[0]['node_id']),
            lambda rows:rows[2].__setitem__('node_id',rows[2]['node_id']+100),
            lambda rows:rows.reverse(),lambda rows:rows[0].__setitem__('source_pointer','/cases/1'),
            lambda rows:rows[0].__setitem__('source_file','../outside.json'),
            lambda rows:rows[0].__setitem__('placement_kind','block')]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory:
                fixture = self.fixture(directory)
                self.mutate_ledger(fixture,'occurrences.json',change)
                with self.assertRaises(ValueError):
                    verify_fixture_inputs(fixture)

    def test_prepared_package_rejects_tex_span_resource_semantics_and_template_tampering(self):
        changes = [lambda package:package['document']['blocks'][0]['children'][0]['source_tex']['text_span'].__setitem__('end_byte',1),
            lambda package:package['document']['blocks'][0]['children'][0]['span'].__setitem__('start_byte',1),
            lambda package:package['document']['blocks'][0]['children'][0].__setitem__('actual_text','different'),
            lambda package:package['document']['blocks'][0].__setitem__('node_id',True),
            lambda package:package['sources'][0].__setitem__('source_id',False),
            lambda package:package['text_buffers'][0].__setitem__('utf8','wrong'),
            lambda package:package['resources']['images'].reverse(),
            lambda package:package['resources']['font_faces'][0].__setitem__('family','wrong'),
            lambda package:package['page_masters']['masters'][0].__setitem__('width',1000)]
        with tempfile.TemporaryDirectory() as directory:
            job = Path(directory)/'job'
            prepare(FIXTURE,job)
            original = read_canonical(job,'document-package.json')
            for change in changes:
                package = copy.deepcopy(original)
                change(package)
                (job/'document-package.json').write_bytes(canonical_bytes(package))
                with self.subTest(change=repr(change)),self.assertRaises(ValueError):
                    verify_prepared_job(job,FIXTURE)

    def test_geometry_control_points_and_semantics_are_independent_from_hashes(self):
        case = read_canonical(FIXTURE,'cases.json')[0]
        source = (FIXTURE/case['source_svg_file']).read_bytes()
        admitted = (FIXTURE/case['admitted_svg_file']).read_bytes()
        tex = (FIXTURE/case['source_tex_file']).read_text()
        wrong = admitted.replace(b' L ',b' C ',1)
        for original,derived in [(source.replace(b'aria-label="one half',b'aria-label="bad half'),admitted),
                                 (source,wrong)]:
            with self.assertRaises(ValueError):
                geometry(original,derived,case,tex)

    def test_json_pointer_rejects_ambiguous_array_indices_and_escapes(self):
        document = {'a/b':[{'~x':7}]}
        self.assertEqual(pointer(document,'/a~1b/0/~0x'),7)
        for path in ('a/b','/a~1b/00','/a~1b/-','/a~1b/1','/a~2b'):
            with self.subTest(path=path),self.assertRaises(ValueError):
                pointer(document,path)

    def test_required_test_names_reject_zero_ignored_missing_extra_and_duplicate_runs(self):
        names = sorted(REQUIRED_CLI_TESTS)
        listing = '\n'.join(name+': test' for name in names)+'\n'
        output = '\n'.join('test '+name+' ... ok' for name in names)+'\n'
        summary = 'test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out; finished in 1s\n'
        self.assertEqual(verify_test_names(listing,output+summary),names)
        bad = [('',output+summary),(listing,'test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 29 filtered out;'),
            (listing,output+summary.replace('3 passed','2 passed').replace('0 ignored','1 ignored')),
            (listing,output.split('\n',1)[1]+summary),(listing,output+output+summary),
            (listing+'vmb_book_unknown: test\n',output+summary),
            (listing,output+'test vmb_book_unknown ... ok\n'+summary)]
        for discovered,executed in bad:
            with self.subTest(executed=executed[:80]),self.assertRaises(ValueError):
                verify_test_names(discovered,executed)


if __name__ == '__main__':
    unittest.main()
