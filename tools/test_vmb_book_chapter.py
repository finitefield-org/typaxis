"""Numbered Formula topology and exact authored number mappings."""
import copy
import json
import shutil
import tempfile
import unittest
from pathlib import Path

from pypdf.generic import ArrayObject,BooleanObject,DictionaryObject,NameObject,NumberObject,TextStringObject

import test_vmb_book_pdf as graph_fixture
from vmb_book_pdf import equation_text,source_tree,verify_source_structure
from vmb_book_inputs import canonical_bytes,digest
from vmb_book_chapter import (prepare,source_inputs,source_wire,verify_job,verify_layout,layout_model,
    extraction_tokens,verify_extraction,verify_pdf,render,damage_controls)


FIXTURE=Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book'


class NumberStructureTests(unittest.TestCase):
    def package(self):
        span=lambda start,end:{'source_id':0,'start_byte':start,'end_byte':end}
        number={'node_id':3,'minimum_gap':65536,'span':span(1,4),
            'text_span':{'text_id':0,'start_byte':1,'end_byte':4}}
        return {'contract':'typaxis.contract/1.4','outline':{'entries':[]},'document':{
            'node_id':0,'language':'en-US','footnotes':[],'blocks':[{'kind':'semantic_container',
                'semantic_kind':'result','node_id':1,'blocks':[{'kind':'math_vector_block','node_id':2,
                    'alt':'a b','span':span(0,4),'equation_number':number}]}]},
            'text_buffers':[{'text_id':0,'utf8':'x(1)','mappings':[
                {'kind':'identity','source_span':span(0,1),'text_range':{'start_byte':0,'end_byte':1}},
                {'kind':'identity','source_span':span(1,4),'text_range':{'start_byte':1,'end_byte':4}}]}]}

    def pdf(self, directory, change=None):
        def numbered(bundle):
            bundle['para'][NameObject('/S')]=NameObject('/Result')
            label=DictionaryObject({NameObject('/Type'):NameObject('/StructElem'),
                NameObject('/S'):NameObject('/Span'),NameObject('/P'):bundle['formula_ref'],
                NameObject('/K'):ArrayObject([DictionaryObject({NameObject('/Type'):NameObject('/MCR'),
                    NameObject('/Pg'):bundle['page'].indirect_reference,NameObject('/MCID'):NumberObject(1)})])})
            ref=bundle['writer']._add_object(label)
            bundle['formula']['/K'].append(ref);bundle['array'].append(ref)
            bundle['label'],bundle['label_ref']=label,ref
            if change:change(bundle)
        return graph_fixture.VmbSourceStructureTests().pdf(directory,numbered)

    def test_number_is_a_formula_child_with_its_own_mcr_and_parent_tree_slot(self):
        with tempfile.TemporaryDirectory() as directory:
            result=verify_source_structure(self.pdf(directory),self.package())
            self.assertEqual((result['source_structure_nodes'],result['source_occurrences']),(4,2))
            self.assertEqual([row['node_id'] for row in result['source_mcr_bindings']],[2,3])
            self.assertEqual([row['mcid'] for row in result['source_mcr_bindings']],[0,1])

    def test_number_role_parent_language_alt_mcr_and_child_order_cannot_change(self):
        changes=[lambda b:b['label'].__setitem__(NameObject('/P'),b['para_ref']),
            lambda b:b['label'].__setitem__(NameObject('/S'),NameObject('/Formula')),
            lambda b:b['label'].__setitem__(NameObject('/Lang'),TextStringObject('fr')),
            lambda b:b['label'].__setitem__(NameObject('/Alt'),TextStringObject('(1)')),
            lambda b:b['label']['/K'][0].__setitem__(NameObject('/MCID'),NumberObject(0)),
            lambda b:b['label']['/K'][0].__setitem__(NameObject('/MCID'),BooleanObject(True)),
            lambda b:b['formula']['/K'].reverse(),
            lambda b:b['formula']['/K'].append(b['label_ref'])]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory,self.assertRaises(ValueError):
                verify_source_structure(self.pdf(directory,change),self.package())

    def test_number_source_requires_the_separate_exact_mapping(self):
        package=self.package();number=package['document']['blocks'][0]['blocks'][0]['equation_number']
        self.assertEqual(equation_text(package,number),'(1)')
        changes=[lambda p:p['text_buffers'][0]['mappings'].__setitem__(1,{
            'kind':'identity','source_span':{'source_id':0,'start_byte':0,'end_byte':4},
            'text_range':{'start_byte':0,'end_byte':4}}),
            lambda p:p['text_buffers'][0]['mappings'][1]['source_span'].__setitem__('source_id',True),
            lambda p:p['text_buffers'][0].__setitem__('text_id',False),
            lambda p:p['text_buffers'][0].__setitem__('utf8',None),
            lambda p:p['text_buffers'][0]['mappings'].append(copy.deepcopy(p['text_buffers'][0]['mappings'][1]))]
        for change in changes:
            private=copy.deepcopy(package);change(private)
            with self.subTest(change=repr(change)),self.assertRaises(ValueError):
                source_tree(private)

    def test_number_owner_gap_span_and_meaningful_text_are_required(self):
        for field,value in [('node_id',True),('minimum_gap',False),('minimum_gap',0),
                ('span',{'source_id':0,'start_byte':1,'end_byte':5}),
                ('text_span',{'text_id':False,'start_byte':1,'end_byte':4})]:
            package=self.package();package['document']['blocks'][0]['blocks'][0]['equation_number'][field]=value
            with self.subTest(field=field,value=value),self.assertRaises(ValueError):source_tree(package)
        for label in ('   ','\n1\n'):
            package=self.package();package['text_buffers'][0]['utf8']='x'+label
            with self.assertRaises(ValueError):source_tree(package)

    def test_source_soft_break_has_no_structure_but_keeps_its_unique_owner(self):
        package=graph_fixture.VmbSourceStructureTests().package()
        children=package['document']['blocks'][0]['children']
        children.append({'kind':'soft_break','node_id':3,'span':{'source_id':0,'start_byte':1,'end_byte':2}})
        children.append({'kind':'math_vector','node_id':4,'alt':'c d'})
        tree=source_tree(package)
        self.assertEqual([node['node_id'] for node in tree['children'][0]['children']],[2,4])
        children[2]['node_id']=3
        with self.assertRaisesRegex(ValueError,'unique typed'):source_tree(package)
        children[2]['node_id']=4;children[1]['span']['start_byte']=True
        with self.assertRaises(ValueError):source_tree(package)

    def test_fake_number_wire_kind_and_body_text_still_cannot_enter_formula_scope(self):
        for kind in ('equation_number','text'):
            package=self.package();package['document']['blocks'][0]['blocks'][0]={'kind':kind,'node_id':2}
            with self.subTest(kind=kind),self.assertRaises(ValueError):source_tree(package)


class ChapterInputTests(unittest.TestCase):
    def copied(self, root):
        result=Path(root)/'fixture';shutil.copytree(FIXTURE,result);return result

    def inventory(self, fixture):
        path=fixture/'fixture-index.json';value=json.loads(path.read_bytes())
        value['files']=[{'path':str(p.relative_to(fixture)),'byte_length':p.stat().st_size,'sha256':digest(p.read_bytes())}
            for p in sorted(fixture.rglob('*')) if p.is_file() and p!=path]
        path.write_bytes(canonical_bytes(value))

    def test_plan_has_all_source_owners_numbers_and_original_font_bytes(self):
        facts=source_inputs(FIXTURE);package,data,events=source_wire(FIXTURE,facts)
        self.assertEqual((len(data),len(events),len(package['text_buffers'])),(8252,600,400))
        self.assertEqual([event['text'] for event in events if event['kind']=='number'],[f'({i})' for i in range(1,201)])
        self.assertEqual(len({event['node']['node_id'] for event in events}),600)
        self.assertEqual(len({row['flow_group'] for row in events}),250)

    def test_bool_ids_missing_cases_reordered_blocks_and_number_relabels_fail(self):
        changes=[lambda p:p['blocks'].pop(),lambda p:p['blocks'].reverse(),
            lambda p:p['blocks'][0]['children'][0].__setitem__('node_id',True),
            lambda p:p['blocks'][1]['equation_number'].__setitem__('text','equation'),
            lambda p:p.__setitem__('original_book',True)]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory:
                fixture=self.copied(directory);path=fixture/'chapter-layout/plan.json';value=json.loads(path.read_bytes())
                change(value);path.write_bytes(canonical_bytes(value))
                provenance=fixture/'chapter-layout/provenance.json';value=json.loads(provenance.read_bytes())
                value['plan_sha256']=digest(path.read_bytes());provenance.write_bytes(canonical_bytes(value));self.inventory(fixture)
                with self.assertRaises(ValueError):source_inputs(fixture)

    def test_original_font_cannot_be_replaced_by_an_empty_synthetic_font(self):
        with tempfile.TemporaryDirectory() as directory:
            fixture=self.copied(directory);font=fixture/'chapter-layout/DejaVuSans.ttf'
            font.write_bytes((fixture/'fixture-fonts/body.ttf').read_bytes())
            path=fixture/'chapter-layout/provenance.json';value=json.loads(path.read_bytes())
            value['font'].update(sha256=digest(font.read_bytes()),byte_length=font.stat().st_size)
            path.write_bytes(canonical_bytes(value));self.inventory(fixture)
            with self.assertRaisesRegex(ValueError,'visible original number font'):source_inputs(fixture)

    def test_prepared_job_rejects_changed_number_source_metrics_or_unused_svg(self):
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)/'job';facts=prepare(FIXTURE,job)
            self.assertEqual((facts['formulas'],facts['equation_numbers'],facts['distinct_images']),(400,200,14))
            path=job/'document-package.json';original=path.read_bytes();package=json.loads(original)
            changes=[lambda p:p['document']['blocks'][0]['blocks'][1]['equation_number'].__setitem__('minimum_gap',1),
                lambda p:p['document']['blocks'][0]['blocks'][0]['children'][0]['metrics'].__setitem__('baseline',1),
                lambda p:p['text_buffers'][4].__setitem__('utf8','different')]
            for change in changes:
                value=copy.deepcopy(package);change(value);path.write_bytes(canonical_bytes(value))
                with self.assertRaises(ValueError):verify_job(FIXTURE,job)
            path.write_bytes(original)
            image=job/package['resources']['images'][-1]['uri'];image.write_bytes(image.read_bytes()+b' ')
            with self.assertRaisesRegex(ValueError,'SVG source bytes'):verify_job(FIXTURE,job)

    def test_output_does_not_enter_source_fixture(self):
        with self.assertRaisesRegex(ValueError,'outside source fixture'):prepare(FIXTURE,FIXTURE/'bad-output')

    def test_extraction_preserves_payloads_and_source_group_boundaries(self):
        expected=[{'node_id':3,'group':2,'text':'one half'},{'node_id':5,'group':2,'text':'two quarters'},
                  {'node_id':11,'group':10,'text':'(1)'}]
        for raw in (b'one halftwo quarters\n(1)\n',b'one half two quarters\n(1)\n'):
            self.assertEqual(verify_extraction(raw,expected,'test')['required_group_boundaries'],1)
        for raw in (b'onehalftwo quarters\n(1)',b'one halftwo quarters(1)',
                    b'one halftwo quarters\n(1) extra',b'one halftwo quarters\n(1)(1)'):
            with self.subTest(raw=raw),self.assertRaises(ValueError):verify_extraction(raw,expected,'test')
        _,_,events=source_wire(FIXTURE,source_inputs(FIXTURE));tokens=extraction_tokens(events)
        self.assertEqual((len(tokens),len({row['group'] for row in tokens})),(600,250))

    def test_all_physical_output_entries_protect_fixture_and_job(self):
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)/'job';job.mkdir()
            for output in (FIXTURE/'bad',job/'bad'):
                with self.assertRaisesRegex(ValueError,'outside source inputs'):
                    verify_pdf(FIXTURE,job,job/'missing.pdf',output)
                with self.assertRaisesRegex(ValueError,'outside source inputs'):
                    render(FIXTURE,job,job/'missing.pdf',{},output,tools={})
                with self.assertRaisesRegex(ValueError,'outside source inputs'):
                    damage_controls(FIXTURE,job,job/'missing.pdf',output)

    def test_source_layout_is_pdf_independent_and_rejects_shifted_or_wrong_page_positions(self):
        model=layout_model(FIXTURE);self.assertEqual((model['pages'],model['inline_lines'],len(model['number_positions'])),(19,80,200))
        master_height=20000000
        proof={'pages':[{} for _ in range(model['pages'])],'placements':[],'equation_numbers':[]}
        for row in model['formula_positions']:
            proof['placements'].append({'node_id':row['node_id'],'page_index':row['page_index'],'baseline_raw':row['baseline'],
                'bounds_raw':[row['x'],master_height-row['y']-row['height'],row['x']+row['width'],master_height-row['y']]})
        for row in model['number_positions']:
            proof['equation_numbers'].append({'node_id':row['node_id'],'page_index':row['page_index'],'source_width_raw':row['width'],
                'glyphs':[{'world_matrix':['1','0','0','1',str(row['x']/65536),str((master_height-row['baseline'])/65536)]}]})
        self.assertEqual(verify_layout(FIXTURE,proof)['maximum_observed_coordinate_error_raw'],0)
        changes=[lambda p:p['placements'][0]['bounds_raw'].__setitem__(0,p['placements'][0]['bounds_raw'][0]+2),
            lambda p:p['placements'][0].__setitem__('page_index',1),
            lambda p:p['placements'].pop(),lambda p:p['equation_numbers'][0].__setitem__('source_width_raw',1)]
        for change in changes:
            value=copy.deepcopy(proof);change(value)
            with self.subTest(change=repr(change)),self.assertRaises(ValueError):verify_layout(FIXTURE,value)


if __name__=='__main__':
    unittest.main()
