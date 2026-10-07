#!/usr/bin/env python3
"""Verify actual nested-table page text, original paragraph origins and artifacts."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_table_header_break_pdf import glyph_pages


def paragraphs(reader):
    pages = []
    for page in reader.pages:
        stack, found = [], []
        matrix = None
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                actual = args[1].get('/ActualText') if op == b'BDC' else None
                stack.append([actual, False])
            elif op == b'EMC':
                stack.pop()
            elif op == b'Tm':
                matrix = [float(v) for v in args]
            elif op == b'Tj':
                for entry in reversed(stack):
                    if entry[0] is not None:
                        if not entry[1]:
                            found.append([entry[0], matrix[4], matrix[5]])
                            entry[1] = True
                        break
        assert not stack
        pages.append(found)
    return pages


def classify(parent):
    child = parent['body'][0]['cells'][0]['blocks'][0]
    right = parent['body'][0]['cells'][1]['blocks']
    if not right: return 'deep'
    if right[0]['kind'] == 'table': return 'parallel-children'
    if len(parent['body'][0]['cells'][0]['blocks']) == 2: return 'two-children'
    if child.get('caption'): return 'caption'
    if child['head']:
        return 'child-header-break' if any(b['kind'] == 'page_break' for row in child['head'] for c in row['cells'] for b in c['blocks']) else 'headers'
    if child['body'][0]['cells'][1]['rowspan'] == 2: return 'child-span'
    if len(parent['body']) == 2: return 'following'
    if right[0]['kind'] == 'page_break': return 'earlier-parent'
    if len(right) == 3: return 'simultaneous'
    if child['classes']: return 'spacing'
    if len(child['body'][0]['cells'][0]['blocks']) == 4: return 'natural'
    return 'forced'


def expected(kind, left, right):
    both = left + right
    l, r, b = (left, 10), (right, 55), (both, 100)
    groups = [[l,b],[l,r],[r]]
    repeats = ['', '', '']
    text = [left + both, both, right]
    if kind == 'natural' or kind == 'spacing' or kind == 'deep':
        groups = [[l,l,r,b], [l] if kind == 'spacing' else [l,l,r]]
        text = [left * 2 + right + both, left if kind == 'spacing' else left * 2 + right]
        repeats = ['', '']
        if kind == 'deep':
            groups = [[(word, 58.75 if x == 55 else 107.5 if x == 100 else x) for word,x in page] for page in groups]
    elif kind == 'headers':
        groups, text, repeats = [[l,r,l,r,l,r,b],[l,r,l,r]], [both * 4, both * 3], ['', left[0] + right[0]]
    elif kind == 'child-header-break':
        groups = [[l,b],[l,r,r,l,r],[l,r],[l,r],[l,r]]
        text = [left + both, left + right * 2 + both] + [left * 2 + right * 2 + both] * 3
        repeats = ['', ''] + [left[0] * 2 + right[0] * 2] * 3
    elif kind == 'caption':
        groups = [[],[(both,10),b],[],[(both,10)],[l],[l,r],[r]]
        text, repeats = ['',both * 2,'',both,left,both,right], [''] * 7
    elif kind == 'simultaneous':
        groups[1].append(b); text[1] += both
    elif kind == 'earlier-parent':
        groups.insert(0,[]); text.insert(0,''); repeats.insert(0,'')
    elif kind == 'following':
        groups[2] += [l,(right,100)]; text[2] += both
    elif kind == 'child-span':
        groups[1].append(l); text[1] += left
    elif kind == 'two-children':
        groups = [[l,b],[l,r],[r,l],[l,r],[r]]
        text, repeats = [left+both,both,right+left,both,right], [''] * 5
    elif kind == 'parallel-children':
        groups = [[l,(left,100)],[l,r,(left,100),(right,145)],[r,(right,145)]]
        text, repeats = [left*2,both*2,right*2], [''] * 3
    return groups, text, repeats


def check_text(actual, expected):
    assert [''.join(v.split()) for v in actual] == expected, (actual, expected)


def check_origins(actual, expected, height):
    assert len(actual) == len(expected)
    for page, source in zip(actual, expected):
        assert len(page) == len(source), (page, source)
        for (text,x,y), (wanted,origin) in zip(page,source):
            assert text == wanted and abs(x-origin) <= 1/65536, (text,x,wanted,origin)
            assert 10 < y <= 10 + height, y


def check_artifacts(pages, expected, left, right):
    assert len(pages) == len(expected)
    for page, wanted in zip(pages, expected):
        repeated = [g for g in page if g[3]]
        assert ''.join(g[0] for g in repeated) == wanted
        for char,x,_,_ in repeated:
            assert abs(x-(10 if char==left[0] else 55)) <= 1/65536


def verify(directory, self_test):
    drivers = {}
    for path in directory.glob('*.driver'):
        d = json.loads(path.read_text()); drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set(); count=total_pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text()); wire=p['wire']; doc=wire['document']; text=wire['text_buffers'][0]['utf8']
        if text not in ('LeftRight','左側右側') or doc['footnotes'] or len(doc['blocks'])!=1: continue
        parent=doc['blocks'][0]
        if parent['kind']!='table' or not parent['body'] or not parent['body'][0]['cells'][0]['blocks']: continue
        if parent['body'][0]['cells'][0]['blocks'][0]['kind']!='table': continue
        if any(r['selector'] == 'table.keep' or r['style_id'] in ('nested-header-keep', 'nested-span-keep') for r in wire['style_sheet']['rules']): continue
        kind=classify(parent); key=text,kind; assert key not in seen,key; seen.add(key)
        left,right=('Left','Right') if text=='LeftRight' else ('左側','右側')
        groups,texts,repeats=expected(kind,left,right)
        height=wire['page_masters']['masters'][0]['body']['height']/65536
        actual=drivers[tuple(p['display'])]; assert len(actual)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*actual]:
            reader=PdfReader(pdf,strict=True); extracted=[p.extract_text() for p in reader.pages]
            check_text(extracted,texts)
            positions=paragraphs(reader); check_origins(positions,groups,height)
            glyphs=glyph_pages(reader,left,right); check_artifacts(glyphs,repeats,left,right)
            count+=1; total_pages+=len(reader.pages)
            if self_test:
                for bad in [extracted[1:],extracted+[''],[extracted[0]+left]+extracted[1:],extracted[:-1]+[extracted[-1]+left]]:
                    try: check_text(bad,texts)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted lost/duplicated nested source')
                page=next(i for i,p in enumerate(positions) if p)
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    bad=copy.deepcopy(positions);bad[page][0][field]=value
                    try: check_origins(bad,groups,height)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted incorrect nested original text or origin')
                page=next(i for i,p in enumerate(glyphs) if p)
                bad=copy.deepcopy(glyphs);bad[page][0][3]=not bad[page][0][3]
                try: check_artifacts(bad,repeats,left,right)
                except AssertionError: rejected+=1
                else: raise AssertionError('accepted source/artifact exchange')
    assert seen=={('LeftRight',k) for k in ['natural','forced','headers','caption','simultaneous','earlier-parent','following','spacing','deep','child-span','child-header-break','two-children','parallel-children']}|{('左側右側','forced')},seen
    return count,total_pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test)
    print(f'PASS: {count} nested PDFs / {pages} pages; physical text, original glyph origins and artifact columns; {rejected} alterations rejected')
