"""Failure and evidence-binding controls for the local VMB book runner."""
import copy
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from prepare_vmb_book_fixtures import prepare
from vmb_book_inputs import REQUIRED_CLI_TESTS, canonical_bytes
from verify_vmb_book import (BASE, ROOT, Executor, book_inputs, check_file, checked,
    cli_receipts, main, parse_ua_report, sha, source_inventory, verify_fixture_evidence, write_json)
from vmb_book_scale import formula_speech, indexed_files, render_cache, reused_render
from vmb_probe_io import extracted_occurrences
from verify_vmb_scale_render import POLICY


class RunnerCommandTests(unittest.TestCase):
    def test_actual_stdout_stderr_nonzero_and_missing_executable_are_retained(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            executor = Executor(root)
            with self.assertRaisesRegex(ValueError,'unexpected command output'):
                checked(executor,'warning',[sys.executable,'-c','import sys;sys.stderr.write("warning\\n")'])
            with self.assertRaisesRegex(ValueError,'command failed'):
                checked(executor,'failure',[sys.executable,'-c','raise SystemExit(7)'])
            with self.assertRaisesRegex(ValueError,'command failed'):
                checked(executor,'missing',[root/'absent-program'])
            warning,failure,missing = executor.records
            self.assertEqual(warning['stderr_bytes'],8)
            self.assertEqual(failure['exit_code'],7)
            self.assertEqual(missing['state'],'spawn-failed')
            for record in executor.records:
                self.assertEqual(sha(root/record['stdout']),record['stdout_sha256'])
                self.assertEqual(sha(root/record['stderr']),record['stderr_sha256'])
            self.assertEqual(len(list((root/'commands').glob('*.json'))),3)

    def test_timeout_kills_the_owned_process_and_records_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            executor = Executor(Path(directory),timeout=0.1)
            with self.assertRaisesRegex(ValueError,'command failed'):
                checked(executor,'timeout',[sys.executable,'-c','import time;time.sleep(30)'])
            record = executor.records[0]
            self.assertTrue(record['timed_out'])
            self.assertEqual(record['exit_code'],-9)
            self.assertGreater(record['elapsed_ns'],100_000_000)

    def test_ambient_probe_and_java_overrides_do_not_reach_child(self):
        with tempfile.TemporaryDirectory() as directory,patch.dict(os.environ,{
            'TYPAXIS_VMB_TEST_OVERRIDE':'unused','VMB_BOOK_TYPAXIS':'unused',
            'JAVA_TOOL_OPTIONS':'unused','JAVACMD':'unused'}):
            executor = Executor(Path(directory))
            _,out,_ = checked(executor,'environment',[sys.executable,'-c',
                'import os,json;print(json.dumps({k:os.environ.get(k) for k in '
                '["TYPAXIS_VMB_TEST_OVERRIDE","VMB_BOOK_TYPAXIS","JAVA_TOOL_OPTIONS","JAVACMD"]}))'],stdout=True)
            self.assertEqual(json.loads(out),{key:None for key in
                ('TYPAXIS_VMB_TEST_OVERRIDE','VMB_BOOK_TYPAXIS','JAVA_TOOL_OPTIONS','JAVACMD')})

    def test_input_overlap_and_existing_output_fail_before_any_command(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture = Path(directory)/'fixture'
            fixture.mkdir()
            for output in (fixture/'result',Path(directory)):
                with self.subTest(output=output),self.assertRaises((ValueError,FileExistsError)):
                    main(['--typaxis',sys.executable,'--fixture-root',str(fixture),
                          '--output-root',str(output),'--mode','fixtures'])
            self.assertEqual(list(fixture.iterdir()),[])


class BookExpectationTests(unittest.TestCase):
    def fixture(self, root):
        job = root/'job'
        prepare(BASE/'vmb-book',job)
        package = json.loads((job/'document-package.json').read_bytes())
        (job/'config.toml').write_text('[machine]\n')
        def record(path, **extra):
            source = job/path
            return {'path':path,'byte_length':source.stat().st_size,'sha256':sha(source),**extra}
        expected = {'algorithm':'typaxis.vmb-book-expectation/1',
            'package':record('document-package.json'),
            'config':record('config.toml',effective_sha256='0'*64),
            'source_occurrences':source_inventory(package),'fixture_evidence':{'path':'unused','sha256':'0'*64}}
        for member,kind,id_key in [('font_files','font_faces','font_face_id'),
                ('image_files','images','image_id'),('source_files','sources','source_id')]:
            rows = package['sources'] if kind == 'sources' else package['resources'][kind]
            expected[member] = [record(row['uri'],**{id_key:row[id_key]}) for row in rows]
        return job,package,expected

    def check(self, root, job, expected):
        path = root/'expected.json'
        path.write_bytes(canonical_bytes(expected))
        return book_inputs(job,path)

    def test_complete_hash_and_source_inventory_is_required_before_public_build(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            job,package,expected = self.fixture(root)
            _,_,inventory = self.check(root,job,expected)
            self.assertEqual(len(inventory),20)
            variants = []
            for member in ('package','config','font_files','image_files','source_files','source_occurrences'):
                value = copy.deepcopy(expected)
                del value[member]
                variants.append(value)
            for member in ('font_files','image_files','source_files','source_occurrences'):
                value = copy.deepcopy(expected)
                value[member].pop()
                variants.append(value)
            for value in variants:
                with self.subTest(fields=list(value)),self.assertRaises(ValueError):
                    self.check(root,job,value)

    def test_wrong_resource_font_config_hash_or_occurrence_order_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            job,package,expected = self.fixture(root)
            for member in ('package','config','font_files','image_files','source_files'):
                value = copy.deepcopy(expected)
                record = value[member][0] if isinstance(value[member],list) else value[member]
                record['sha256'] = 'f'*64
                with self.subTest(member=member),self.assertRaises(ValueError):
                    self.check(root,job,value)
            value = copy.deepcopy(expected)
            value['source_occurrences'][0],value['source_occurrences'][1] = value['source_occurrences'][1],value['source_occurrences'][0]
            with self.assertRaisesRegex(ValueError,'occurrence inventory'):
                self.check(root,job,value)
            expected['image_files'][0]['image_id'] = False
            with self.assertRaisesRegex(ValueError,'declaration/file join'):
                self.check(root,job,expected)

    def test_duplicate_or_boolean_source_owners_are_not_integer_occurrences(self):
        with tempfile.TemporaryDirectory() as directory:
            _,package,_ = self.fixture(Path(directory))
            package['document']['node_id'] = False
            with self.assertRaisesRegex(ValueError,'unique typed book source node'):
                source_inventory(package)


class InvocationReceiptTests(unittest.TestCase):
    def receipts(self, root):
        directory = root/'receipts'
        directory.mkdir()
        binary = root/'typaxis'
        binary.write_bytes(b'unit control executable identity')
        for name in REQUIRED_CLI_TESTS:
            write_json(directory/(name+'.json'),{'algorithm':'typaxis.vmb-book-cli-test-invocation/1',
                'test_name':name,'binary_path':str(binary),'binary_sha256':sha(binary),
                'test_source_sha256':sha(ROOT/'workspace/crates/typaxis-cli/tests/cli_end_to_end.rs'),
                'fixture_index_sha256':'a'*64,'source_cases':20,'source_occurrences':20})
        return directory,binary

    def test_missing_stale_wrong_binary_and_untyped_receipts_fail(self):
        changes = [('binary_sha256','b'*64),('test_source_sha256','c'*64),
                   ('source_cases',True),('source_occurrences',19),('fixture_index_sha256','d'*64),
                   ('binary_path','/does/not/exist')]
        for key,replacement in changes:
            with self.subTest(key=key),tempfile.TemporaryDirectory() as directory:
                receipts,binary = self.receipts(Path(directory))
                self.assertEqual(len(cli_receipts(receipts,binary,'a'*64)),3)
                path = next(receipts.iterdir())
                value = json.loads(path.read_bytes())
                value[key] = replacement
                path.write_bytes(canonical_bytes(value))
                with self.assertRaises(ValueError):
                    cli_receipts(receipts,binary,'a'*64)
        with tempfile.TemporaryDirectory() as directory:
            receipts,binary = self.receipts(Path(directory))
            next(receipts.iterdir()).unlink()
            with self.assertRaisesRegex(ValueError,'exact CLI invocation receipt set'):
                cli_receipts(receipts,binary,'a'*64)


class UaReportTests(unittest.TestCase):
    REPORT = (b'<report><jobs><job><validationReport jobEndStatus="normal" isCompliant="true">'
        b'<details passedChecks="3" failedChecks="0"/></validationReport></job></jobs>'
        b'<batchSummary totalJobs="1" failedToParse="0" encrypted="0" outOfMemory="0" veraExceptions="0"/></report>')

    def test_zero_jobs_failures_exceptions_and_duplicate_reports_cannot_pass(self):
        self.assertEqual(parse_ua_report(self.REPORT)['passed_checks'],3)
        for old,new in [(b'totalJobs="1"',b'totalJobs="0"'),(b'failedChecks="0"',b'failedChecks="1"'),
                (b'passedChecks="3"',b'passedChecks="0"'),(b'veraExceptions="0"',b'veraExceptions="1"'),
                (b'isCompliant="true"',b'isCompliant="false"'),(b'jobEndStatus="normal"',b'jobEndStatus="exception"'),
                (b'</jobs>',b'<job><validationReport/></job></jobs>')]:
            with self.subTest(new=new),self.assertRaises(ValueError):
                parse_ua_report(self.REPORT.replace(old,new))


class ScaleEvidenceTests(unittest.TestCase):
    def test_figure_alternatives_do_not_invent_extra_extracted_formula_occurrences(self):
        package = {'document':{'blocks':[{'kind':'paragraph','children':[
            {'kind':'math_vector','alt':'a b','actual_text':None}]},
            {'kind':'figure','alt':'Figure one'},
            {'kind':'math_vector_block','alt':'unused alternative','actual_text':'c d'}]}}
        expected = formula_speech(package)
        self.assertEqual(extracted_occurrences(b'a b\n\nc d\n',expected,'unit-control'),2)
        with self.assertRaisesRegex(ValueError,'text or source spaces differ'):
            extracted_occurrences(b'a b\nFigure one\nc d\n',expected,'unit-control')

    def test_missing_duplicate_or_failed_raster_rows_do_not_satisfy_complete_coverage(self):
        for mutation in ('missing','duplicate','wrong-owner','missing-ink','distance','area','boolean-ordinal'):
            with self.subTest(mutation=mutation),tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                cached,case,corpus = root/'cache',root/'case',root/'corpus'
                (cached/'distinct').mkdir(parents=True)
                (cached/'producer').mkdir()
                case.mkdir()
                corpus.mkdir()
                (corpus/'fixture-index.json').write_bytes(b'unit corpus identity')
                tool = root/'unit-tool'
                tool.write_bytes(b'unit tool identity')
                tools = {name:str(tool) for name in ('mutool','pdftoppm','rsvg-convert')}
                placements = {'pdf_sha256':'a'*64,'source_package_sha256':'b'*64,'pages':[{}],
                    'placements':[{'ordinal':0,'page_index':0,'kind':'math_vector'}]}
                summary = {'algorithm':POLICY['algorithm'],'policy':POLICY,'case':'distinct',
                    'passed':True,'failed_comparisons':0,'pdf_sha256':'a'*64,'source_package_sha256':'b'*64,
                    'corpus_sha256':sha(corpus/'fixture-index.json'),'all_case_pages':True,'selected_pages':[0],
                    'tools':{name:{'sha256':sha(tool)} for name in tools},'commands':[],'comparisons':6}
                rows = [{'ordinal':0,'page_index':0,'renderer':renderer,'dpi':dpi,'reference_ink':100,
                    'observed_ink':100,'area_difference':0,'missing_over_one_pixel':0,'extra_over_one_pixel':0,
                    'passed':True} for renderer in ('MuPDF','Poppler') for dpi in (72,144,288)]
                if mutation == 'missing':
                    rows.pop()
                elif mutation == 'duplicate':
                    rows[-1] = copy.deepcopy(rows[0])
                elif mutation == 'wrong-owner':
                    rows[0]['ordinal'] = 1
                elif mutation == 'boolean-ordinal':
                    rows[0]['ordinal'] = False
                elif mutation == 'missing-ink':
                    rows[0].update(observed_ink=0,area_difference=100)
                elif mutation == 'distance':
                    rows[0]['missing_over_one_pixel'] = 1
                elif mutation == 'area':
                    rows[0].update(observed_ink=105,area_difference=5)
                write_json(cached/'distinct/summary.json',summary)
                write_json(cached/'distinct/comparisons.json',rows)
                args = SimpleNamespace(scale_render_evidence=cached)
                selected = {'paths':tools,'policy':{'render_cache':{}}}
                names = {'distinct/summary.json','distinct/comparisons.json'}
                with self.assertRaisesRegex(ValueError,'cached (raster|source formula masks)'):
                    reused_render(args,'distinct',case,selected,placements,corpus,names,(sha,write_json))

    def test_passed_flag_and_small_case_cannot_replace_the_complete_fixture_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root/'typaxis'
            binary.write_bytes(b'unit identity')
            observed = root/'observed.json'
            write_json(observed,{'algorithm':'typaxis.vmb-book-verification/1','mode':'fixtures','passed':True,
                'fixture_index_sha256':'a'*64,'typaxis':{'sha256':sha(binary)},
                'groups':{'engine-small':{'passed':True}}})
            evidence = {'path':str(observed),'sha256':sha(observed),'artifact_index_sha256':'b'*64}
            with self.assertRaisesRegex(ValueError,'complete fixture gate'):
                verify_fixture_evidence(evidence,'a'*64,'c'*64,binary)

    def test_index_hash_every_file_and_duplicate_member_are_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            data = root/'data'
            data.write_bytes(b'bound source')
            record = {'path':'data','sha256':sha(data),'byte_length':data.stat().st_size}
            index = root/'artifact-index.json'
            index.write_bytes(canonical_bytes({'algorithm':'unit-evidence','files':[record]}))
            pinned = sha(index)
            self.assertEqual(indexed_files(root,pinned,'unit-evidence',sha,check_file)[1],{'data'})
            data.write_bytes(b'changed source')
            with self.assertRaisesRegex(ValueError,'file hash/length'):
                indexed_files(root,pinned,'unit-evidence',sha,check_file)
            data.write_bytes(b'bound source')
            index.write_bytes(canonical_bytes({'algorithm':'unit-evidence','files':[record,record]}))
            with self.assertRaisesRegex(ValueError,'unique evidence'):
                indexed_files(root,sha(index),'unit-evidence',sha,check_file)
            with self.assertRaisesRegex(ValueError,'pinned evidence'):
                indexed_files(root,pinned,'unit-evidence',sha,check_file)

    def test_cached_renderer_dependencies_cannot_be_relabelled_as_current(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            producer = root/'producer'
            producer.mkdir()
            ledger = producer/'numeric-producer-source.json'
            write_json(ledger,{'tools/unknown.py':'a'*64})
            write_json(root/'artifact-index.json',{'algorithm':'typaxis.vmb-book-render-cache/1',
                'files':[{'path':str(ledger.relative_to(root)),'byte_length':ledger.stat().st_size,'sha256':sha(ledger)}],
                'render_policy':POLICY,'producer_source_ledger_sha256':sha(ledger),
                'producer_dependencies':{'tools/unknown.py':'a'*64}})
            selected = {'policy':{'render_cache':{'artifact_index_sha256':sha(root/'artifact-index.json'),
                'producer_source_ledger_sha256':sha(ledger)}}}
            args = SimpleNamespace(scale_render_evidence=root,repository_root=ROOT)
            with self.assertRaisesRegex(ValueError,'complete producer dependency set'):
                render_cache(args,selected,sha,check_file)


if __name__ == '__main__':
    unittest.main()
