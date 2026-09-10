#!/usr/bin/env python3
"""Check varying body/note frames against authored line and font metrics."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_definition_table_pdf import check_positions
from verify_book_v2_pdf_assembly import selected_master

U = 65536


def expected(wire):
    text = wire['text_buffers'][0]['utf8']
    japanese = text == '左側右側'
    assert japanese or text == 'Result'
    table = wire['document']['blocks'][0]['kind'] == 'table'
    notes = wire['document']['footnotes']
    note_table = bool(notes and notes[0]['blocks'][0]['kind'] == 'table')
    assert not (japanese and (table or note_table))
    ascent = round((1151 if japanese else 800) * 12 * U / 1000)
    descent = round((286 if japanese else 200) * 12 * U / 1000)
    digit = round((471 if japanese else 600) * 12 * U / 1000)
    text_width = 4*12*U if japanese else 6*digit
    base_height = 20 if japanese else 16
    baseline = lambda h: ascent + (h*U - ascent - descent)//2
    if table or note_table:
        counts = [1,2,1]
    else:
        counts = [1,2,3,2,3]
    result, annotations = [], []
    for page, count in enumerate(counts):
        master = selected_master(wire['page_masters'],page)
        body = master['body']
        x, y = body['x'], body['y']
        assert x == 500_000 and y == [10,20,30,20,30][page]*U
        found = []
        if not (table or note_table):
            assert body['height'] == count*base_height*U
            for i in range(count):
                top = y + i*base_height*U + baseline(base_height)
                found.append((text,x,top))
                if notes: found.append(('1',x+text_width,top))
        elif note_table and page == 0:
            found += [(text,x,y+baseline(16)),('1',x+text_width,y+baseline(16))]
        if table or note_table:
            if table:
                left, top, width = x, y, body['width']
            else:
                note = master['footnote']
                top = note['y']+note['height']-count*23*U
                left = note['x']+digit+12*U
                width = note['width']-digit-12*U
                if page == 0: found.append(('1',note['x'],top+baseline(17)))
            found.extend((text,left,top+i*17*U+baseline(17)) for i in range(count))
            found.extend((text,left+round(width/2),top+i*23*U+baseline(23)) for i in range(count))
        elif notes:
            remaining = [1,2,3,1,0][page]
            note = master['footnote']
            top = note['y']+note['height']-remaining*base_height*U
            if page == 0: found.append(('1',note['x'],top+baseline(base_height)))
            found.extend((text,note['x']+digit+12*U,top+i*base_height*U+baseline(base_height)) for i in range(remaining))
        result.append(found)
        annotations.append((2 if page==0 else 0) if note_table else (count+int(page==0) if notes else 0))
    return result,annotations,(japanese,table,bool(notes),note_table)


def verify(directory,self_test,require_harano):
    drivers = {}
    for path in directory.glob('*.driver'):
        value=json.loads(path.read_text())
        drivers.setdefault(tuple(value['display']),[]).append(value['assembly']['pdf'])
    count=pages=rejected=0
    seen=set()
    for path in directory.glob('*.json'):
        value=json.loads(path.read_text())
        wire=value['wire']
        if any(r['style_id'].startswith('horizontal-') for r in wire['style_sheet']['rules']):continue
        if not any(r['style_id']=='varying' for r in wire['style_sheet']['rules']): continue
        wanted,annotations,key=expected(wire)
        assert key not in seen,key
        seen.add(key)
        matching=drivers[tuple(value['display'])]
        assert len(matching)==1
        for pdf in [value['relations']['navigation']['assembly']['pdf'],*matching]:
            reader=PdfReader(pdf,strict=True)
            actual=paragraphs(reader)
            check_positions(actual,wanted)
            assert [len(p.get('/Annots',[])) for p in reader.pages]==annotations
            count+=1;pages+=len(reader.pages)
            if self_test:
                altered=[actual[1:],actual+[[]]]
                for field,replacement in [(0,'lost'),(1,999),(2,999)]:
                    changed=copy.deepcopy(actual);changed[0][0][field]=replacement;altered.append(changed)
                changed=copy.deepcopy(actual);changed[1].insert(0,changed[0][0]);altered.append(changed)
                changed=copy.deepcopy(actual)
                # A fixed first-page origin must not replace the selected origin.
                for row in changed[1]: row[2]-=10
                altered.append(changed)
                if key[2]:
                    changed=copy.deepcopy(actual);changed[0].pop();altered.append(changed)
                for changed in altered:
                    try: check_positions(changed,wanted)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted changed source or page geometry')
    required={(False,False,False,False),(False,False,True,False),(False,True,False,False),(False,False,True,True)}
    if require_harano: required.add((True,False,True,False))
    assert required <= seen,seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path)
    parser.add_argument('--self-test',action='store_true')
    parser.add_argument('--require-harano',action='store_true')
    args=parser.parse_args()
    count,pages,rejected=verify(args.directory,args.self_test,args.require_harano)
    print(f'PASS: {count} varying-frame PDFs / {pages} pages / {rejected} alterations rejected; authored line heights, table source, note markers, physical origins and region bottoms')
