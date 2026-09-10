#!/usr/bin/env python3
"""Independently check body/definition table columns, note baselines and repeats."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_table_header_break_pdf import glyph_pages

U = 65536

def expected(text, body_height=80):
    japanese = text == '左側右側'
    left, right = ('左側', '右側') if japanese else ('Left', 'Right')
    # Original Harano hhea (1151/-286), UPEM 1000 and digit-one hmtx 471;
    # the controlled TrueType fixture uses 800/-200 and advance 600.
    ascent = round((1151 if japanese else 800) * 12 * U / 1000)
    descent = round((286 if japanese else 200) * 12 * U / 1000)
    normal = max(16 * U, ascent + descent)
    baseline = ascent + (normal - ascent - descent) // 2
    digit = round((471 if japanese else 600) * 12 * U / 1000)
    note_left = 10 * U + digit + 12 * U
    parent_half = round((180 * U - digit - 12 * U) / 2)
    inner_half = round(parent_half / 2)
    parent_right = note_left + parent_half
    note_right = note_left + inner_half
    rows = 1 if japanese else 2
    count = 4 // rows
    note_height = (1 + rows) * normal
    # Declared footnote rectangle starts at 150 pt and is 48 pt + 1 pt separator.
    note_baseline = 199 * U - note_height + baseline
    source, repeats = [], []
    for page in range(count):
        current = []
        left_per_page = body_height * U // normal
        right_per_page = body_height // 32
        if japanese:
            current += [(left,10*U,10*U+baseline+i*normal) for i in range(min(left_per_page,max(0,4-page*left_per_page)))]
            tall_leading = (32*U-ascent-descent)//2
            current += [(right,55*U,10*U+ascent+tall_leading+i*32*U) for i in range(min(right_per_page,max(0,2-page*right_per_page)))]
        if not japanese:
            current += [(left,10*U,10*U+baseline+i*normal) for i in range(2)]
            current += [(right,55*U,10*U+baseline+8*U)]
        if page == 0:
            advance = 48*U if japanese else 9*digit
            current += [(left+right,100*U,10*U+baseline),('1',100*U+advance,10*U+baseline)]
            current += [('1',10*U,note_baseline),(left,note_left,note_baseline),(right,note_right,note_baseline)]
        for row in range(rows):
            current += [(left,note_left,note_baseline+(row+1)*normal),(right,note_right,note_baseline+(row+1)*normal)]
        if page == 0:
            current += [(left+right,parent_right,note_baseline)]
        source.append(current)
        repeats.append([] if page == 0 else [(left[0],note_left,note_baseline),(right[0],note_right,note_baseline)])
    return source,repeats,left,right

def check_positions(actual, wanted):
    assert len(actual)==len(wanted),(len(actual),len(wanted))
    for found, expected_page in zip(actual,wanted):
        assert len(found)==len(expected_page),(found,expected_page)
        for (text,x,y),(word,left,baseline) in zip(found,expected_page):
            assert text==word and abs(x*U-left)<2 and abs(y*U-baseline)<2,(text,x,y,word,left,baseline)

def check(reader, text, body_height):
    source,repeats,left,right=expected(text, body_height)
    actual=paragraphs(reader)
    check_positions(actual,source)
    glyphs=glyph_pages(reader,left,right,allow_layout=True)
    repeated=[[[c,x,y] for c,x,y,repeat in page if repeat] for page in glyphs]
    check_positions(repeated,repeats)
    # Exactly one original body occurrence and one definition marker; no repeated labels.
    assert [sum(word=='1' for word,_,_ in page) for page in actual]==[2]+[0]*(len(source)-1)
    assert [len(page.get('/Annots',[])) for page in reader.pages]==[2]+[0]*(len(source)-1)
    return actual,repeated,source,repeats

def verify(directory,self_test,require_narrow=False):
    drivers={}
    for path in directory.glob('*.driver'):
        p=json.loads(path.read_text());drivers.setdefault(tuple(p['display']),[]).append(p['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text());doc=p['wire']['document']
        if len(doc['blocks'])!=1 or doc['blocks'][0]['kind']!='table' or len(doc['footnotes'])!=1:continue
        note=doc['footnotes'][0]['blocks']
        if len(note)!=1 or note[0]['kind']!='table':continue
        try:head=note[0]['body'][0]['cells'][0]['blocks'][0]['head']
        except (KeyError,IndexError):continue
        if not head:continue
        text=p['wire']['text_buffers'][0]['utf8']
        body_raw=p['wire']['page_masters']['masters'][0]['body']['height']
        key=(text,body_raw);assert key not in seen,key;seen.add(key)
        matched=drivers[tuple(p['display'])];assert len(matched)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*matched]:
            reader=PdfReader(pdf,strict=True)
            assert body_raw in [48*U,80*U],body_raw
            actual,repeated,source,repeats=check(reader,text,body_raw//U)
            count+=1;pages+=len(reader.pages)
            if self_test:
                alterations=[]
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    changed=copy.deepcopy(actual);changed[0][-1][field]=value;alterations.append(changed)
                changed=copy.deepcopy(actual);changed[1].insert(0,['1',10,150]);alterations.append(changed)
                alterations += [actual[1:],actual+[[]]]
                for changed in alterations:
                    try:check_positions(changed,source)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted altered note source, position or marker')
                for changed in [[[] for _ in repeated],repeated+[[]]]:
                    try:check_positions(changed,repeats)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted altered header repetitions')
    required={('LeftRight',48*U),('左側右側',80*U)}
    if require_narrow:required.add(('左側右側',48*U))
    assert required<=seen<=required|{('左側右側',48*U)},seen
    return count,pages,rejected

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    parser.add_argument('--require-narrow',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test,args.require_narrow)
    print(f'PASS: {count} combined body/definition table PDFs / {pages} pages; original columns, independently computed baselines, marker/annotation once and repeated headers; {rejected} alterations rejected')
