"""Controls for source-derived synthetic resource boundaries and failure evidence."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from pypdf import PdfWriter
from pypdf.generic import ArrayObject,DecodedStreamObject,DictionaryObject,NameObject,NumberObject,TextStringObject

from vmb_book_boundaries import (REQUIRED_CASES,expected_diagnostics,form_events,prepare,render,
    source_model,specs,svg,validate_failure,verify_job)
from vmb_book_inputs import canonical_bytes
from vmb_book_pdf import source_tree,verify_source_structure
import test_vmb_book_pdf as graph_fixture


FIXTURE=Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book'


def case(name):
    return next(row for row in specs() if row['case_id'] == name)


class BoundarySourceTests(unittest.TestCase):
    def test_required_public_matrix_has_every_positive_and_negative_boundary(self):
        rows=specs()
        self.assertEqual(len(rows),len(REQUIRED_CASES))
        self.assertEqual({row['case_id'] for row in rows},REQUIRED_CASES)
        self.assertEqual(sum(row['failure'] is None for row in rows),10)
        self.assertEqual([case('images-'+label+'-'+kind)['declarations']
            for label in ('default','config') for kind in ('exact','over')],[8192,8193,1024,1025])

    def test_authored_clip_definitions_replay_viewport_and_depth_are_charged(self):
        simple,deep,clips=(source_model(svg(kind)) for kind in ('simple','deep','clips'))
        self.assertEqual((simple['nodes'],simple['stored_segments'],simple['path_work'],simple['allocation_charge']),(3,10,10,1136))
        self.assertEqual((deep['max_depth'],deep['path_work'],deep['allocation_charge']),(4,10,1200))
        self.assertEqual((clips['nodes'],clips['stored_segments'],clips['clip_replay'],clips['path_work'],
            clips['commands'],clips['source_clip_id_bytes'],clips['allocation_charge']),(8,20,10,30,8,3,2499))
        self.assertEqual(len(clips['draws']),2)
        self.assertEqual([len(draw['clips']) for draw in clips['draws']],[1,1])
        # Every source fact must be serializable by the strict evidence writer.
        for model in (simple,deep,clips):
            self.assertEqual(json.loads(canonical_bytes(model)),model)

    def test_aliases_charge_each_declaration_and_depth_is_per_resource(self):
        with tempfile.TemporaryDirectory() as directory:
            alias=prepare(FIXTURE,Path(directory)/'alias',case('aliases-work-exact'))
            depth=prepare(FIXTURE,Path(directory)/'depth',case('depth-per-resource'))
        self.assertEqual((alias['declarations'],alias['distinct_svg_bytes'],alias['source_occurrences']),(2,1,1))
        self.assertEqual((alias['declared_work']['path_work'],alias['declared_work']['allocation_charge']),(20,2272))
        self.assertEqual((depth['declared_work']['nodes'],depth['declared_work']['max_depth']),(6,3))

    def test_unused_declarations_boolean_ids_and_relabelled_inputs_are_rejected(self):
        spec=case('aliases-work-exact')
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)/'job';prepare(FIXTURE,job,spec)
            path=job/'document-package.json';original=path.read_bytes();package=json.loads(original)
            changes=[lambda p:p['resources']['images'].pop(0),
                lambda p:p['resources']['images'][0].__setitem__('image_id',False),
                lambda p:p['document']['blocks'][0].__setitem__('image_id',True),
                lambda p:p['resources']['images'][0]['vector_provenance'].__setitem__('engine_id','vmb-go-math-svg'),
                lambda p:p['document']['blocks'][0].__setitem__('alt','a real VMB formula')]
            for change in changes:
                value=copy.deepcopy(package);change(value);path.write_bytes(canonical_bytes(value))
                with self.subTest(change=repr(change)),self.assertRaises(ValueError):
                    verify_job(FIXTURE,job,spec)
            path.write_bytes(original)
            (job/'z.svg').write_bytes(svg('inset'))
            with self.assertRaisesRegex(ValueError,'authored synthetic SVG bytes'):
                verify_job(FIXTURE,job,spec)

    def test_boolean_case_parameters_and_outputs_inside_inputs_are_rejected(self):
        spec=case('nodes-exact');spec['declarations']=True
        with tempfile.TemporaryDirectory() as directory,self.assertRaisesRegex(ValueError,'exact boundary case'):
            prepare(FIXTURE,Path(directory)/'job',spec)
        with self.assertRaisesRegex(ValueError,'outside source fixtures'):
            prepare(FIXTURE,FIXTURE/'bad',case('nodes-exact'))
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)
            with self.assertRaisesRegex(ValueError,'outside source inputs'):
                render(job,job/'missing.pdf',{},job/'render',tools={})


class BoundaryFailureTests(unittest.TestCase):
    def status(self, spec):
        return {'exit_code':1 if spec['failure']['kind']=='images' else 5,'timed_out':False,'state':'complete'}

    def check(self, spec, diagnostics=None, record=None, out=b'',err=None):
        expected=expected_diagnostics(spec);row=expected['diagnostics'][0]
        stderr=('error: '+row['code']+': '+row['message']+'\n').encode() if err is None else err
        validate_failure(spec,record or self.status(spec),out,stderr,expected if diagnostics is None else diagnostics)

    def test_all_failure_locations_are_derived_from_source_order_and_tokens(self):
        for spec in specs():
            if spec['failure'] is not None:self.check(spec)
        z=expected_diagnostics(case('order-z-first'))['diagnostics'][0]
        a=expected_diagnostics(case('order-a-first'))['diagnostics'][0]
        self.assertEqual(z['location']['json_pointer'],a['location']['json_pointer'])
        self.assertEqual((z['notes'][0]['message'],a['notes'][0]['message']),('resource=z.svg','resource=a.svg'))
        self.assertIn('svg_byte=106..143',z['notes'][1]['message'])
        self.assertIn('svg_byte=106..167',a['notes'][1]['message'])
        replay=expected_diagnostics(case('clip-replay-over'))['diagnostics'][0]
        self.assertIn('element_number=7; attribute=clip-path',replay['notes'][1]['message'])
        self.assertIn('token_percent=%75%72%6C%28%23%63%29',replay['notes'][1]['message'])
        outer=expected_diagnostics(case('outer-clip-over'))['diagnostics'][0]
        self.assertIn('element=svg; element_number=1; attribute=viewBox',outer['notes'][1]['message'])
        self.assertNotIn('; path=',outer['notes'][1]['message'])

    def test_wrong_exit_timeout_spawn_failure_or_output_cannot_pass_a_negative(self):
        for name in ('images-config-over','segments-over'):
            spec=case(name)
            changes=[{'exit_code':0},{'exit_code':5 if name.startswith('images') else 1},
                     {'exit_code':True},{'timed_out':True},{'state':'spawn-failed'}]
            for change in changes:
                with self.subTest(name=name,change=change),self.assertRaises(ValueError):
                    self.check(spec,record={**self.status(spec),**change})
            with self.assertRaises(ValueError):self.check(spec,out=b'partial output')
            with self.assertRaises(ValueError):self.check(spec,err=b'warning\n')

    def test_wrong_resource_pointer_span_token_limit_or_extra_diagnostic_fails(self):
        spec=case('segments-total-over');original=expected_diagnostics(spec)
        changes=[lambda d:d['diagnostics'][0]['location'].__setitem__('json_pointer','/resources/images/0'),
            lambda d:d['diagnostics'][0]['notes'][0].__setitem__('message','resource=z.svg'),
            lambda d:d['diagnostics'][0]['notes'][1].__setitem__('message','element=path; svg_byte=0..1'),
            lambda d:d['diagnostics'][0]['notes'][2].__setitem__('message','scope=resource-local; limit=19'),
            lambda d:d['diagnostics'].append(copy.deepcopy(d['diagnostics'][0]))]
        for change in changes:
            value=copy.deepcopy(original);change(value)
            with self.subTest(change=repr(change)),self.assertRaises(ValueError):self.check(spec,diagnostics=value)


class BoundaryPaintTests(unittest.TestCase):
    PATH=b'0 0 m\n1 0 l\n1 1 l\n0 1 l\nh\nf\n'
    DATA=b'q\n0 0 1 1 re W n\n1 0 0 1 0 0 cm\nq\n/GS0 gs\n'+PATH+b'Q\nQ\n'

    def events(self, data, state=None):
        form=DecodedStreamObject();form.set_data(data)
        form.update({NameObject('/Subtype'):NameObject('/Form'),
            NameObject('/BBox'):ArrayObject([NumberObject(v) for v in (0,0,1,1)]),
            NameObject('/Resources'):DictionaryObject({NameObject('/ExtGState'):DictionaryObject({
                NameObject('/GS0'):DictionaryObject(state or {})})})})
        return form_events(form,PdfWriter())

    def test_actual_form_events_preserve_fill_count_and_active_clip_chains(self):
        expected=source_model(svg('simple'))['draws']
        self.assertEqual(self.events(self.DATA),expected)
        duplicate=self.DATA.replace(self.PATH,self.PATH*2)
        self.assertNotEqual(self.events(duplicate),expected)
        local=self.DATA.replace(b'q\n/GS0',b'q\n0 0 0.5 1 re W n\n/GS0')
        self.assertNotEqual(self.events(local),expected)
        with self.assertRaises(ValueError):self.events(self.DATA.replace(b'h\nf',b'h\nn'))
        with self.assertRaises(ValueError):self.events(self.DATA.replace(b'1 0 0 1 0 0 cm',b'1 0 0 1 1 0 cm'))
        with self.assertRaises(ValueError):self.events(self.DATA,{NameObject('/ca'):NumberObject(0)})

    def test_vector_figure_is_a_source_figure_with_exact_alt_and_mcr(self):
        fixture=graph_fixture.VmbSourceStructureTests();package=fixture.package()
        node=package['document']['blocks'][0]['children'][0]
        node.update(kind='vector_figure',caption=[])
        self.assertEqual(source_tree(package)['children'][0]['children'][0]['role'],'/Figure')
        with tempfile.TemporaryDirectory() as directory:
            def figure(bundle):bundle['formula'][NameObject('/S')]=NameObject('/Figure')
            path=fixture.pdf(directory,figure)
            self.assertEqual(verify_source_structure(path,package)['source_occurrences'],1)
            node['alt']='changed alternative'
            with self.assertRaises(ValueError):verify_source_structure(path,package)


if __name__=='__main__':
    unittest.main()
