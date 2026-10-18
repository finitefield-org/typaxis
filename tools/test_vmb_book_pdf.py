"""Synthetic graph controls and real-source scale controls for the VMB runner."""
import copy
import json
import tempfile
import unittest
from pathlib import Path

from pypdf import PdfWriter
from pypdf.generic import ArrayObject, BooleanObject, DictionaryObject, NameObject, NumberObject, TextStringObject

from vmb_book_pdf import ROLE_MAP, source_tree, verify_source_structure
from verify_vmb_scale_render import original_svg
from verify_vmb_scale_pdf import verify_source_pdf


ROOT = Path(__file__).resolve().parents[1]


class VmbSourceStructureTests(unittest.TestCase):
    def package(self):
        return {'contract':'typaxis.contract/1.4','outline':{'entries':[]},'document':{
            'node_id':0,'language':'en-US','footnotes':[],'blocks':[{'kind':'paragraph','node_id':1,
            'children':[{'kind':'math_vector','node_id':2,'alt':'a b'}]}]}}

    def pdf(self, directory, mutate=None):
        writer = PdfWriter()
        page = writer.add_blank_page(width=100,height=100)
        page[NameObject('/StructParents')] = NumberObject(0)
        catalog = writer._root_object
        catalog.update({NameObject('/Lang'):TextStringObject('en-US'),
            NameObject('/MarkInfo'):DictionaryObject({NameObject('/Marked'):BooleanObject(True)}),
            NameObject('/ViewerPreferences'):DictionaryObject({NameObject('/DisplayDocTitle'):BooleanObject(True)})})
        tree,parent,doc,para,formula = (DictionaryObject() for _ in range(5))
        tr,pt,dr,pr,fr = [writer._add_object(value) for value in (tree,parent,doc,para,formula)]
        catalog[NameObject('/StructTreeRoot')] = tr
        array = ArrayObject([fr])
        parent[NameObject('/Nums')] = ArrayObject([NumberObject(0),array])
        tree.update({NameObject('/Type'):NameObject('/StructTreeRoot'),
            NameObject('/ParentTree'):pt,NameObject('/ParentTreeNextKey'):NumberObject(1),
            NameObject('/RoleMap'):DictionaryObject({NameObject(k):NameObject(v) for k,v in ROLE_MAP.items()}),
            NameObject('/K'):ArrayObject([dr])})
        doc.update({NameObject('/Type'):NameObject('/StructElem'),NameObject('/S'):NameObject('/Document'),
            NameObject('/P'):tr,NameObject('/Lang'):TextStringObject('en-US'),NameObject('/K'):ArrayObject([pr])})
        para.update({NameObject('/Type'):NameObject('/StructElem'),NameObject('/S'):NameObject('/P'),
            NameObject('/P'):dr,NameObject('/K'):ArrayObject([fr])})
        mcr = DictionaryObject({NameObject('/Type'):NameObject('/MCR'),NameObject('/Pg'):page.indirect_reference,
                                NameObject('/MCID'):NumberObject(0)})
        formula.update({NameObject('/Type'):NameObject('/StructElem'),NameObject('/S'):NameObject('/Formula'),
            NameObject('/P'):pr,NameObject('/Alt'):TextStringObject('a b'),NameObject('/K'):ArrayObject([mcr])})
        values = {'writer':writer,'page':page,'catalog':catalog,'tree':tree,'parent':parent,
            'doc':doc,'para':para,'formula':formula,'mcr':mcr,'array':array,
            'tree_ref':tr,'parent_ref':pt,'doc_ref':dr,'para_ref':pr,'formula_ref':fr}
        if mutate:
            mutate(values)
        path = Path(directory)/'structure.pdf'
        writer.write(path)
        return path

    def test_complete_source_graph_and_inherited_language_bind_one_occurrence(self):
        with tempfile.TemporaryDirectory() as directory:
            result = verify_source_structure(self.pdf(directory),self.package())
            self.assertEqual((result['source_structure_nodes'],result['source_occurrences'],result['pages']),(3,1,1))
            self.assertEqual(result['source_mcr_bindings'][0]['node_id'],2)

    def test_source_language_override_is_checked_against_the_corresponding_parent(self):
        package = self.package()
        package['document']['blocks'][0]['language'] = 'fr'
        with tempfile.TemporaryDirectory() as directory:
            path = self.pdf(directory,lambda b:b['para'].__setitem__(NameObject('/Lang'),TextStringObject('fr')))
            self.assertEqual(verify_source_structure(path,package)['source_occurrences'],1)

    def test_parent_language_alternative_and_child_order_mutations_are_rejected(self):
        changes = [lambda b:b['formula'].__setitem__(NameObject('/P'),b['doc_ref']),
            lambda b:b['para'].__setitem__(NameObject('/Lang'),TextStringObject('fr')),
            lambda b:b['catalog'].__setitem__(NameObject('/Lang'),TextStringObject('fr')),
            lambda b:b['formula'].__setitem__(NameObject('/Alt'),TextStringObject('ab')),
            lambda b:b['formula'].__setitem__(NameObject('/S'),NameObject('/Figure')),
            lambda b:b['para'].__setitem__(NameObject('/K'),ArrayObject([b['formula_ref'],b['formula_ref']])),
            lambda b:b['para'].__setitem__(NameObject('/K'),ArrayObject([b['doc_ref']]))]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(ValueError):
                    verify_source_structure(self.pdf(directory,change),self.package())

    def test_mcr_parent_tree_ranges_types_and_duplicate_keys_are_rejected(self):
        changes = [lambda b:b['mcr'].__setitem__(NameObject('/Pg'),b['doc_ref']),
            lambda b:b['mcr'].__setitem__(NameObject('/MCID'),NumberObject(1)),
            lambda b:b['mcr'].__setitem__(NameObject('/MCID'),BooleanObject(False)),
            lambda b:b['array'].__setitem__(0,b['para_ref']),
            lambda b:b['page'].__setitem__(NameObject('/StructParents'),BooleanObject(False)),
            lambda b:b['parent'].__setitem__(NameObject('/Nums'),ArrayObject([
                NumberObject(0),b['array'],NumberObject(0),b['array']])),
            lambda b:b['parent'].__setitem__(NameObject('/Limits'),ArrayObject([NumberObject(0),NumberObject(1)]))]
        for change in changes:
            with self.subTest(change=repr(change)),tempfile.TemporaryDirectory() as directory:
                with self.assertRaises(ValueError):
                    verify_source_structure(self.pdf(directory,change),self.package())

    def test_unreachable_structures_and_unbound_parent_tree_slots_are_rejected(self):
        def orphan(bundle):
            bundle['writer']._add_object(DictionaryObject({NameObject('/Type'):NameObject('/StructElem'),
                NameObject('/S'):NameObject('/Span'),NameObject('/P'):bundle['doc_ref'],NameObject('/K'):ArrayObject()}))
        for change in (orphan,lambda b:b['array'].append(b['formula_ref'])):
            with tempfile.TemporaryDirectory() as directory,self.assertRaises(ValueError):
                verify_source_structure(self.pdf(directory,change),self.package())

    def test_catalog_boolean_and_role_map_are_typed(self):
        changes = [lambda b:b['catalog']['/MarkInfo'].__setitem__(NameObject('/Marked'),NumberObject(1)),
            lambda b:b['catalog']['/ViewerPreferences'].__setitem__(NameObject('/DisplayDocTitle'),NumberObject(1)),
            lambda b:b['tree']['/RoleMap'].__setitem__(NameObject('/Result'),NameObject('/Span'))]
        for change in changes:
            with tempfile.TemporaryDirectory() as directory,self.assertRaises(ValueError):
                verify_source_structure(self.pdf(directory,change),self.package())

    def test_body_and_note_source_cannot_be_reported_as_formula_only_verification(self):
        package = self.package()
        package['document']['blocks'][0]['children'][0]['kind'] = 'text'
        with self.assertRaisesRegex(ValueError,'does not cover source kind'):
            source_tree(package)
        package = self.package()
        package['document']['footnotes'] = [{'node_id':3}]
        with self.assertRaisesRegex(ValueError,'formula-only source structure scope'):
            source_tree(package)
        package = self.package()
        package['document']['blocks'][0]['node_id'] = False
        with self.assertRaisesRegex(ValueError,'typed source structure owner'):
            source_tree(package)

    def test_shared_paint_kernel_rejects_an_empty_source_or_output_inside_inputs(self):
        with tempfile.TemporaryDirectory() as directory:
            job = Path(directory)/'job'
            with self.assertRaisesRegex(ValueError,'complete source occurrence inventory'):
                verify_source_pdf(job,job/'missing.pdf',{}, {'placements':0},[])
            with self.assertRaisesRegex(ValueError,'outside the input package'):
                verify_source_pdf(job,job/'missing.pdf',{}, {},[],job/'evidence.json')

    def test_second_real_physical_size_requires_an_explicit_checked_size(self):
        corpus = ROOT/'samples/machine-package/staging/production-book-1/vmb-book/engine-v2'
        case = json.loads((corpus/'fixture-index.json').read_text())['cases'][1]
        source,derived = (corpus/case['source_svg']).read_bytes(),(corpus/case['derived_svg']).read_bytes()
        with self.assertRaisesRegex(ValueError,'physical font size'):
            original_svg(source,derived,case)
        _,counts = original_svg(source,derived,case,expected_font_size_raw=case['font_size_raw'])
        self.assertEqual(counts['segments'],case['segment_count'])


if __name__ == '__main__':
    unittest.main()
