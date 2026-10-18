"""Original font/subset joins and malformed-font/CID controls for numbers."""
import copy
import hashlib
import math
import struct
import unittest
from fractions import Fraction
from pathlib import Path

from pypdf.generic import ArrayObject,DecodedStreamObject,DictionaryObject,NameObject,NumberObject
from vmb_book_font import CMAP_HEAD,CMAP_END,TrueType,checksum,unicode_map,verify_number_font


FONT=Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book/chapter-layout/DejaVuSans.ttf'
CHARS='()0123456789'


def sfnt(tables):
    tables={tag:bytes(value) for tag,value in tables.items()}
    head=bytearray(tables[b'head']);head[8:12]=b'\0'*4;tables[b'head']=bytes(head)
    count=len(tables);power=2**int(math.log2(count));offset=12+16*count
    header=struct.pack('>IHHHH',0x10000,count,power*16,int(math.log2(power)),count*16-power*16)
    entries=[];payload=bytearray();head_offset=None
    for tag,data in sorted(tables.items()):
        entries.append(struct.pack('>4sIII',tag,checksum(data),offset,len(data)))
        if tag==b'head':head_offset=offset
        aligned=data+b'\0'*((-len(data))%4);payload.extend(aligned);offset+=len(aligned)
    result=bytearray(header+b''.join(entries)+payload)
    struct.pack_into('>I',result,head_offset+8,(0xb1b0afba-checksum(result))&0xffffffff)
    return bytes(result)


def number_font(source, characters=CHARS):
    glyphs=[0]+[source.codepoint(ord(char)) for char in characters]
    programs=[];offsets=[0];metrics=[]
    for gid in glyphs:
        raw=source.glyph_bytes(gid);raw+=b'\0'*((-len(raw))%4)
        programs.append(raw);offsets.append(offsets[-1]+len(raw));metrics.append(struct.pack('>Hh',*source.metrics(gid)))
    head=bytearray(source.tables[b'head']);struct.pack_into('>h',head,50,1)
    hhea=bytearray(source.tables[b'hhea']);struct.pack_into('>H',hhea,34,len(glyphs))
    maxp=bytearray(source.tables[b'maxp']);struct.pack_into('>H',maxp,4,len(glyphs))
    binary=sfnt({b'head':head,b'hhea':hhea,b'maxp':maxp,b'glyf':b''.join(programs),
        b'loca':struct.pack('>'+str(len(offsets))+'I',*offsets),b'hmtx':b''.join(metrics)})
    program=DecodedStreamObject();program.set_data(binary)
    cmap=DecodedStreamObject();cmap.set_data(CMAP_HEAD+str(len(characters)).encode()+b' beginbfchar\n'+b''.join(
        f'<{cid:04X}> <{ord(char):04X}>\n'.encode() for cid,char in enumerate(characters,1))+b'endbfchar\n'+CMAP_END)
    mapping=DecodedStreamObject();mapping.set_data(struct.pack('>'+str(len(glyphs))+'H',*range(len(glyphs))))
    scaled=lambda value:NumberObject(int(Fraction(value*1000,source.units)))
    descriptor=DictionaryObject({NameObject('/FontFile2'):program,
        NameObject('/FontBBox'):ArrayObject([scaled(value) for value in source.bbox]),
        NameObject('/Ascent'):scaled(source.ascender),NameObject('/Descent'):scaled(source.descender)})
    widths=ArrayObject()
    for cid,char in enumerate(characters,1):
        width=Fraction(source.metrics(source.codepoint(ord(char)))[0]*1000,source.units)
        # Independent half-up conversion, including fractional advances > .5.
        rounded=(2*width.numerator+width.denominator)//(2*width.denominator)
        widths.extend([NumberObject(cid),ArrayObject([NumberObject(rounded)])])
    cid=DictionaryObject({NameObject('/Type'):NameObject('/Font'),NameObject('/Subtype'):NameObject('/CIDFontType2'),
        NameObject('/FontDescriptor'):descriptor,NameObject('/W'):widths,NameObject('/CIDToGIDMap'):mapping})
    font=DictionaryObject({NameObject('/Type'):NameObject('/Font'),NameObject('/Subtype'):NameObject('/Type0'),
        NameObject('/Encoding'):NameObject('/Identity-H'),NameObject('/ToUnicode'):cmap,
        NameObject('/DescendantFonts'):ArrayObject([cid])})
    return font


class NumberFontTests(unittest.TestCase):
    def setUp(self):self.source=TrueType(FONT.read_bytes())

    def test_original_source_characters_are_visible_and_have_exact_metrics(self):
        self.assertEqual(hashlib.sha256(self.source.raw).hexdigest(),'b4c632e3cdf9acc7f28758fb5a323c8524d7fc6660d46904d9b6cbe2809c419c')
        self.assertEqual((self.source.units,self.source.ascender,self.source.descender),(2048,1901,-483))
        self.assertEqual([self.source.codepoint(ord(char)) for char in CHARS],[11,12,*range(19,29)])
        for char in CHARS:
            gid=self.source.codepoint(ord(char));glyph=self.source.glyph(gid)
            self.assertGreater(len(glyph['points']),0)
            self.assertEqual(self.source.metrics(gid)[0],799 if char in '()' else 1303)

    def test_bool_outside_or_missing_unicode_glyphs_do_not_fall_back(self):
        self.source.glyph(0)
        for gid in (True,False,-1,self.source.glyph_count):
            with self.subTest(gid=gid),self.assertRaises(ValueError):self.source.glyph(gid)
        for point in (True,-1,0x110000,0x10ffff):
            with self.subTest(point=point),self.assertRaises(ValueError):self.source.codepoint(point)
        with self.assertRaisesRegex(ValueError,'visible simple'):self.source.glyph(self.source.codepoint(32))

    def test_checksums_directory_offsets_and_metric_ranges_are_not_trusted(self):
        raw=bytearray(self.source.raw);raw[-1]^=1
        with self.assertRaises(ValueError):TrueType(bytes(raw))
        raw=bytearray(self.source.raw);struct.pack_into('>I',raw,20,0)
        with self.assertRaises(ValueError):TrueType(bytes(raw))
        for field,value in [('loca',2),('metrics',0)]:
            tables=copy.deepcopy(self.source.tables)
            if field=='loca':
                data=bytearray(tables[b'head']);struct.pack_into('>h',data,50,value);tables[b'head']=data
            else:
                data=bytearray(tables[b'hhea']);struct.pack_into('>H',data,34,value);tables[b'hhea']=data
            with self.assertRaises(ValueError):TrueType(sfnt(tables))

    def test_independently_built_subset_binds_every_unicode_outline_and_advance(self):
        result=verify_number_font(number_font(self.source),self.source,CHARS)
        self.assertEqual((result['source_glyphs'],result['subset_glyphs']),(12,13))
        self.assertEqual([row['unicode'] for row in result['cids']],list(CHARS))

    def test_cid_glyph_swap_is_rejected_with_unchanged_semantic_text(self):
        font=number_font(self.source);mapping=font['/DescendantFonts'][0]['/CIDToGIDMap']
        mapping.set_data(struct.pack('>13H',0,2,1,*range(3,13)))
        with self.assertRaisesRegex(ValueError,'original chapter number outline'):
            verify_number_font(font,self.source,CHARS)

    def test_tounicode_swap_is_rejected_even_when_all_required_characters_remain(self):
        font=number_font(self.source);cmap=font['/ToUnicode'];raw=cmap.get_data()
        raw=raw.replace(b'<0001> <0028>',b'<0001> <0029>').replace(b'<0002> <0029>',b'<0002> <0028>')
        cmap.set_data(raw)
        with self.assertRaisesRegex(ValueError,'original chapter number outline'):
            verify_number_font(font,self.source,CHARS)

    def test_wrong_pdf_width_font_descriptor_or_incomplete_gid_map_fails(self):
        for kind in ('width','ascent','gidmap'):
            font=number_font(self.source);cid=font['/DescendantFonts'][0]
            if kind=='width':cid['/W'][1][0]=NumberObject(391)
            elif kind=='ascent':cid['/FontDescriptor'][NameObject('/Ascent')]=NumberObject(0)
            else:cid['/CIDToGIDMap'].set_data(b'\0')
            with self.subTest(kind=kind),self.assertRaises(ValueError):verify_number_font(font,self.source,CHARS)

    def test_cmap_duplicate_count_and_unowned_tokens_cannot_pass(self):
        body=b'2 beginbfchar\n<0001> <0028>\n<0002> <0029>\nendbfchar\n'
        self.assertEqual(unicode_map(CMAP_HEAD+body+CMAP_END),{1:'(',2:')'})
        variants=[body.replace(b'<0002>',b'<0001>'),body.replace(b'2 begin',b'3 begin'),body+b'unknown\n']
        for raw in variants:
            with self.subTest(raw=raw),self.assertRaises(ValueError):unicode_map(CMAP_HEAD+raw+CMAP_END)


if __name__=='__main__':unittest.main()
