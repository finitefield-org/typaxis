#!/usr/bin/env python3
"""Verify serial nested captions, body transition and original header artifacts."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs, check_origins, check_text
from verify_book_v2_table_header_break_pdf import glyph_pages


def classify(doc):
    parent=doc['blocks'][0];cap=parent['caption'];body=parent['body'][0]['cells'][0]['blocks']
    if doc['footnotes']: return 'notes'
    if cap[0]['kind']=='paragraph': return 'ordinary'
    if cap[0]['kind']=='page_break': return 'caption-break'
    child=cap[0]
    if not body:
        if len(cap)>1: return 'caption-only-break'
        return 'empty-kept-body' if child['classes']==['keep'] else 'caption-only'
    if body[0]['kind']=='page_break': return 'body-forced'
    if child['head']: return 'child-header'
    if child.get('caption'):
        return 'deep' if child['caption'][0]['kind']=='table' else 'child-caption'
    if child['classes']==['child']: return 'spacing'
    if child['classes']==['keep']: return 'child-keep'
    if len(cap)>1: return 'multiple' if cap[1]['kind']=='table' else 'keep'
    if any(b['kind']=='page_break' for b in child['body'][0]['cells'][0]['blocks']): return 'forced-child'
    return 'caption-child'


def expected(mode,left,right):
    l,r,b=(left,10),(right,100),(left+right,10)
    half=[l,l,r];body=[l,r]
    if mode=='ordinary': return [[b,l,l,(right,55),r],[l,l,(right,55)]]
    if mode=='caption-child': return [half,half+body]
    if mode=='forced-child': return [[l],[l,r],[r]+body] if left=='Left' else [[l],[l,r],[r],body]
    if mode=='caption-break': return [[],[l],[l,r],[r],[b]+body]
    if mode=='multiple': return [half,half,half,half+body]
    if mode=='keep': return [half,half,[l]+body]
    if mode=='child-keep': return [half,[l]+body]
    if mode in ('caption-only','empty-kept-body'): return [half,half]
    if mode=='caption-only-break': return [half,half,[]]
    if mode=='body-forced': return [half,half,body]
    if mode=='child-header': return [body*3,body*2,body]
    if mode=='child-caption': return [[],[b],[],[b],[l],body,[r]+body]
    if mode=='spacing': return [half,[l]+body]
    if mode=='deep': return [half,half+body,body]
    if mode=='notes':
        note=[(left,29.199996948242188)]*2
        return [half,half,[l,('1',38.79998779296875)]+body+[('1',10)]+note,note,note,note]
    raise AssertionError(mode)


def check_repeats(glyphs,mode,left,right):
    repeats=['']*len(glyphs)
    if mode=='child-header': repeats[1]=left[0]+right[0]
    for page,wanted in zip(glyphs,repeats):
        copies=[g for g in page if g[3]]
        assert ''.join(g[0] for g in copies)==wanted
        for char,x,y,_ in copies:
            assert abs(x-(10 if char==left[0] else 100))<=1/65536
            assert abs(y-21.600006103515625)<=1/65536


def verify(directory,self_test):
    drivers={}
    for path in directory.glob('*.driver'):
        d=json.loads(path.read_text());drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text());w=p['wire']
        if not any(r['style_id'].startswith('cap-keep-') for r in w['style_sheet']['rules']): continue
        text=w['text_buffers'][0]['utf8'];assert text in ('LeftRight','左側右側')
        left,right=('Left','Right') if text=='LeftRight' else ('左側','右側')
        mode=classify(w['document']);key=text,mode;assert key not in seen,key;seen.add(key)
        groups=expected(mode,left,right)
        texts=[''.join(word for word,_ in page) for page in groups]
        if mode=='child-header': texts[1]=left+right+texts[1]
        height=118 if mode=='notes' else w['page_masters']['masters'][0]['body']['height']/65536
        actual=drivers[tuple(p['display'])];assert len(actual)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*actual]:
            reader=PdfReader(pdf,strict=True);extracted=[page.extract_text() for page in reader.pages]
            check_text(extracted,texts)
            positions=paragraphs(reader);check_origins(positions,groups,height)
            glyphs=glyph_pages(reader,left,right,allow_layout=mode=='notes');check_repeats(glyphs,mode,left,right)
            annotations=[len(page.get('/Annots',[])) for page in reader.pages]
            assert annotations==([0,0,2,0,0,0] if mode=='notes' else [0]*len(groups)),annotations
            if mode=='notes': assert all(p[2]>80 for p in positions[2][-3:])
            count+=1;pages+=len(reader.pages)
            if self_test:
                for bad in [extracted[1:],extracted+[''],[extracted[0]+left]+extracted[1:],extracted[:-1]+[extracted[-1]+right]]:
                    try: check_text(bad,texts)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted lost/duplicated nested-caption source')
                page=next(i for i,p in enumerate(positions) if p)
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    bad=copy.deepcopy(positions);bad[page][0][field]=value
                    try: check_origins(bad,groups,height)
                    except AssertionError: rejected+=1
                    else: raise AssertionError('accepted altered original paragraph')
                page=next(i for i,p in enumerate(glyphs) if p)
                bad=copy.deepcopy(glyphs);bad[page][0][3]=not bad[page][0][3]
                try: check_repeats(bad,mode,left,right)
                except AssertionError: rejected+=1
                else: raise AssertionError('accepted exchanged header/source ownership')
    assert seen=={('LeftRight',mode) for mode in ['ordinary','caption-child','forced-child','caption-break','multiple','keep','child-keep','caption-only','caption-only-break','empty-kept-body','body-forced','child-header','child-caption','spacing','deep','notes']}|{('左側右側','forced-child')},seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test)
    print(f'PASS: {count} nested caption PDFs / {pages} pages; physical text, original columns, header artifacts and note rollback; {rejected} alterations rejected')
