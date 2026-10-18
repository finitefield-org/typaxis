"""Original visible body glyphs, exact spaces, source scope and job controls."""
import copy
import struct
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from pypdf.generic import NumberObject
from test_vmb_book_font import number_font
from vmb_book_font import TrueType,verify_number_font,verify_text_font
from vmb_book_inputs import canonical_bytes
from vmb_book_mixed import CASES,FONT_FILE,layout_model,prepare,source_inputs,source_wire,verify_extraction,verify_job
from vmb_book_pdf import source_tree

FIXTURE=Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book'


class MixedBodyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=source_inputs(FIXTURE)
        cls.font=TrueType((FIXTURE/FONT_FILE).read_bytes())
        cls.characters=' AB本文前後'

    def test_original_japanese_characters_have_visible_outlines_and_original_metrics(self):
        self.assertEqual((self.font.units,self.font.ascender,self.font.descender),(2048,1802,-246))
        for char in '本文前後':
            gid=self.font.codepoint(ord(char))
            self.assertEqual(self.font.metrics(gid)[0],2048)
            self.assertGreater(len(self.font.glyph(gid)['points']),0)
        self.assertEqual(self.font.glyph_bytes(self.font.codepoint(32)),b'')

    def test_independently_built_subset_accepts_only_the_authored_nonpainting_space(self):
        font=number_font(self.font,self.characters)
        result=verify_text_font(font,self.font,self.characters)
        self.assertEqual(result['source_glyphs'],7)
        with self.assertRaises(ValueError):verify_number_font(font,self.font,self.characters)

    def test_fractional_pdf_width_rounds_up_and_floor_is_rejected(self):
        font=number_font(self.font,self.characters)
        result=verify_text_font(font,self.font,self.characters)
        cid=next(row['cid'] for row in result['cids'] if row['unicode']=='B')
        widths=font['/DescendantFonts'][0]['/W']
        offset=next(index for index in range(0,len(widths),2) if int(widths[index])==cid)
        self.assertEqual(int(widths[offset+1][0]),709)
        widths[offset+1][0]=NumberObject(708)
        with self.assertRaisesRegex(ValueError,'PDF CID width'):verify_text_font(font,self.font,self.characters)

    def test_a_visible_glyph_cannot_replace_the_authored_space(self):
        font=number_font(self.font,self.characters)
        count=len(self.characters)+1;mapping=list(range(count));mapping[1],mapping[2]=mapping[2],mapping[1]
        font['/DescendantFonts'][0]['/CIDToGIDMap'].set_data(struct.pack('>'+str(count)+'H',*mapping))
        with self.assertRaisesRegex(ValueError,'space has no'):verify_text_font(font,self.font,self.characters)

    def test_tounicode_cannot_relabel_visible_body_glyphs(self):
        font=number_font(self.font,self.characters);stream=font['/ToUnicode']
        raw=stream.get_data().replace(b'<0002> <0041>',b'<0002> <0042>').replace(b'<0003> <0042>',b'<0003> <0041>')
        stream.set_data(raw)
        with self.assertRaisesRegex(ValueError,'original chapter number outline'):verify_text_font(font,self.font,self.characters)

    def test_default_formula_structure_scope_still_rejects_body_text(self):
        package,_,_=source_wire(FIXTURE,self.facts,'ascii-before')
        with self.assertRaises(ValueError):source_tree(package)
        self.assertEqual(source_tree(package,body_text=True)['role'],'/Document')
        with self.assertRaises(ValueError):source_tree(package,body_text=1)

    def test_source_page_break_does_not_invent_a_structure_node(self):
        package,_,_=source_wire(FIXTURE,self.facts,'block-page')
        tree=source_tree(package,body_text=True)
        self.assertEqual([row['role'] for row in tree['children'][0]['children']],['/P','/Formula','/P'])

    def test_all_authored_payloads_preserve_spaces_in_both_extractors(self):
        for name in CASES:
            _,_,events=source_wire(FIXTURE,self.facts,name)
            groups=[]
            for event in events:
                if not groups or groups[-1][0]!=event['group']:groups.append([event['group'],''])
                groups[-1][1]+=event['text']
            raw=('\n'.join(text for _,text in groups)+'\f').encode()
            for extractor in ('MuPDF','Poppler'):
                with self.subTest(name=name,extractor=extractor):self.assertTrue(verify_extraction(raw,events,extractor)['all_authored_spaces_preserved'])

    def test_added_deleted_or_collapsed_authored_spaces_are_rejected(self):
        _,_,events=source_wire(FIXTURE,self.facts,'ascii-double')
        expected=''.join(row['text'] for row in events)
        values=[expected.replace('A  ','A ',1),expected.replace('A  ','A   ',1),expected.replace('  B',' B'),
                ' '+expected,expected+' ',expected+'\t',expected+'EXTRA']
        for value in values:
            for extractor in ('MuPDF','Poppler'):
                with self.subTest(value=value,extractor=extractor),self.assertRaises(ValueError):
                    verify_extraction((value+'\f').encode(),events,extractor)

    def test_flow_boundaries_cannot_be_deleted_or_changed_to_authored_spaces(self):
        _,_,events=source_wire(FIXTURE,self.facts,'block-page')
        for separator in ('',' '):
            groups=[]
            for row in events:
                if not groups or groups[-1][0]!=row['group']:groups.append([row['group'],''])
                groups[-1][1]+=row['text']
            with self.subTest(separator=separator),self.assertRaises(ValueError):
                verify_extraction((separator.join(text for _,text in groups)+'\f').encode(),events,'Poppler')

    def test_source_layout_does_not_open_a_pdf_or_trace(self):
        with patch('pypdf.PdfReader',side_effect=AssertionError('source model read a PDF')):
            for name in CASES:
                with self.subTest(name=name):self.assertEqual(layout_model(FIXTURE,name)['pages'],2 if name=='block-page' else 1)

    def test_mutated_job_text_or_font_and_existing_output_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            job=Path(directory)/'job';prepare(FIXTURE,job,'ascii-before')
            verify_job(FIXTURE,job,'ascii-before')
            with self.assertRaises(FileExistsError):prepare(FIXTURE,job,'ascii-before')
            package_path=job/'document-package.json';raw=package_path.read_bytes()
            package,_,_=source_wire(FIXTURE,self.facts,'ascii-before')
            package=copy.deepcopy(package);package['text_buffers'][0]['utf8']='A  '
            package_path.write_bytes(canonical_bytes(package))
            with self.assertRaises(ValueError):verify_job(FIXTURE,job,'ascii-before')
            package_path.write_bytes(raw)
            (job/'fonts/ipaexm.ttf').write_bytes(b'')
            with self.assertRaises(ValueError):verify_job(FIXTURE,job,'ascii-before')


if __name__=='__main__':unittest.main()
