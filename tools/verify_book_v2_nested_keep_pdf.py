#!/usr/bin/env python3
"""Check nested parent keeps from original source shapes and physical PDF pages."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs, check_text, check_origins


def classify(doc):
    if len(doc['blocks']) == 2: return 'whole'
    parent = doc['blocks'][0]
    contents = parent['body'][0]['cells'][0]['blocks']
    right = parent['body'][0]['cells'][1]['blocks']
    if doc['footnotes']: return 'notes'
    if len(right) == 2: return 'forced'
    if len(contents) == 1: return 'last'
    if contents[0]['kind'] == 'paragraph':
        return 'preceding' if len(contents) == 2 else 'prefix-backtrack'
    if not contents[0]['classes']: return 'suffix'
    if len(contents) == 3: return 'chain'
    return 'child-shorten' if contents[1]['classes'] == ['tall'] else 'child-kept'


def expected(mode, left, right):
    # The original child columns begin at 10/55pt inside the outer 90pt cell.
    l, r, b, after = (left, 10), (right, 55), (left + right, 100), (right, 10)
    start, end = [l, l, r, b], [l, l, r]
    if mode == 'last': return [start, end]
    if mode == 'preceding': return [[l,l,l,r,b], end]
    if mode == 'prefix-backtrack': return [[l,b], [l,l,l,r], end]
    if mode == 'suffix': return [start, end, [l,after]]
    if mode == 'child-kept': return [start, end + [l]]
    if mode == 'child-shorten': return [start, [l,after]]
    if mode == 'chain': return [start, [l,l,after]]
    if mode == 'forced': return [[b], [l,l,l,r], end]
    if mode == 'whole': return [[(left+right,10)], [l,r,b]]
    if mode == 'notes':
        marker=('1',38.79998779296875)
        note=[(left,29.199996948242188)]*2
        return [[b], [l,marker,l,l,r,('1',10)]+note, end+note, note, note]
    raise AssertionError(mode)


def check(reader, mode, groups, height):
    texts = [page.extract_text() for page in reader.pages]
    check_text(texts, [''.join(word for word,_ in page) for page in groups])
    positions = paragraphs(reader)
    check_origins(positions, groups, 118 if mode == 'notes' else height)
    annotations = [len(page.get('/Annots', [])) for page in reader.pages]
    assert annotations == ([0,2,0,0,0] if mode == 'notes' else [0]*len(groups)), annotations
    if mode == 'notes':
        # A rejected kept reference must not allocate a note or annotation on page 1.
        assert len(positions[0]) == 1 and positions[1][1][0] == '1'
        assert all(p[2] > 80 for p in positions[1][-3:])
    return texts, positions


def verify(directory, self_test):
    drivers={}
    for path in directory.glob('*.driver'):
        d=json.loads(path.read_text());drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text());w=p['wire']
        if not any(r['selector']=='table.keep' for r in w['style_sheet']['rules']): continue
        if any(r['style_id'].startswith('cap-keep-') for r in w['style_sheet']['rules']): continue
        text=w['text_buffers'][0]['utf8']; assert text in ('LeftRight','左側右側')
        mode=classify(w['document']);key=text,mode;assert key not in seen,key;seen.add(key)
        left,right=('Left','Right') if text=='LeftRight' else ('左側','右側')
        groups=expected(mode,left,right)
        height=w['page_masters']['masters'][0]['body']['height']/65536
        actual=drivers[tuple(p['display'])];assert len(actual)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*actual]:
            reader=PdfReader(pdf,strict=True);texts,positions=check(reader,mode,groups,height)
            count+=1;pages+=len(reader.pages)
            if self_test:
                wanted=[''.join(word for word,_ in page) for page in groups]
                for bad in [texts[1:],texts+[''],[texts[0]+left]+texts[1:],texts[:-1]+[texts[-1]+left]]:
                    try: check_text(bad,wanted)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted missing/duplicated kept source')
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    bad=copy.deepcopy(positions);bad[0][0][field]=value
                    try: check_origins(bad,groups,118 if mode=='notes' else height)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted altered original paragraph')
                bad=copy.deepcopy(positions);bad[0].append(bad[1].pop(0))
                try: check_origins(bad,groups,118 if mode=='notes' else height)
                except AssertionError: rejected+=1
                else: raise AssertionError('accepted keep moved across page boundary')
    assert seen=={('LeftRight',mode) for mode in ['last','preceding','prefix-backtrack','suffix','child-kept','child-shorten','chain','forced','notes','whole']}|{('左側右側','child-shorten')},seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test)
    print(f'PASS: {count} nested keep PDFs / {pages} pages; physical page text, original column origins and note rollback; {rejected} alterations rejected')
