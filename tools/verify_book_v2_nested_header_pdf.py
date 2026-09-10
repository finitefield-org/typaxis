#!/usr/bin/env python3
"""Check original nested headers and detached repeated captions in actual PDFs."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs,check_text,check_origins
from verify_book_v2_table_header_break_pdf import glyph_pages


def classify(doc):
    t=doc['blocks'][0];h=t['head'][0]['cells'][0]['blocks'];cap=t.get('caption',[])
    if doc['footnotes']:return 'notes'
    if len(cap)==2:return 'caption-backtrack'
    if cap:return 'caption-header'
    if h[0]['kind']=='paragraph':return 'forced-header' if len(h)>1 else 'body-child'
    child=h[0]
    if not t['body']:return 'empty-body'
    if child.get('caption'):
        c=child['caption'][0]
        if c['kind']=='table':return 'header-deep'
        if c['kind']=='figure':return 'image-caption'
        if c['children'][0]['kind']=='anchor':return 'empty-caption' if len(c['children'])==1 else 'anchor-caption'
        return 'header-caption'
    return 'forced-child-header' if any(b['kind']=='page_break' for b in child['body'][0]['cells'][0]['blocks']) else 'header-child'


def expected(mode,left,right):
    l,r,c,b,outer=(left,10),(right,100),(right,55),(left+right,10),(left+right,100)
    row=[l,r];tail=[l,l,c];head=[b,l,c,r]
    if mode in ('header-caption','anchor-caption'):return [head+row*2,row],[[],head]
    if mode in ('image-caption','empty-caption'):return [[l,c,r]+row*2,row],[[],[l,c,r]]
    if mode=='empty-body':return [head],[[]]
    if mode=='header-deep':
        head=[b,l,c,l,c,r];return [head+row*2,row],[[],head]
    if mode=='body-child':return [row+tail+[outer],tail],[[],row]
    if mode=='caption-header':return [[b]+row+tail+[outer],tail],[[],row]
    if mode=='forced-header':return [row,[l]+tail+[outer],tail],[[],[],[l,l,r]]
    if mode=='header-child':
        head=[l]*4+[c,c,r];return [head+row*2,row],[[],head]
    if mode=='forced-child-header':
        head=[l,l,c,c,r];return [row,[l,c,c]+row*2,row*2,row],[[],[],head,head]
    if mode=='caption-backtrack':return [[l],[l]+head+row,row*2],[[],[],head]
    if mode=='notes':
        note=[(left,29.199996948242188)]*2
        marked=[b,('1',74.79997253417969),l,c,r]
        return [marked+row*2+[('1',10)]+note,row+note,note,note],[[],marked,[],[]]
    raise AssertionError(mode)


def repeated_glyphs(pages,heads,left,right):
    for page,head in zip(pages,heads):
        wanted=[]
        for word,x in head:
            if word==left+right:wanted.extend([(left[0],x),(right[0],x+(28.79998779296875 if left=='Left' else 24))])
            elif word in (left,right):wanted.append((word[0],x))
        actual=[g for g in page if g[3]]
        assert len(actual)==len(wanted),(actual,wanted)
        for (char,x,y,_),(word,origin) in zip(actual,wanted):
            assert char==word and abs(x-origin)<=1/65536,(char,x,word,origin)
            assert 10<y<115,y


def verify(directory,self_test):
    drivers={}
    for p in directory.glob('*.driver'):
        d=json.loads(p.read_text());drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text());w=p['wire']
        if not any(r['style_id']=='nested-header-keep' for r in w['style_sheet']['rules']):continue
        text=w['text_buffers'][0]['utf8'];assert text in ('LeftRight','左側右側')
        left,right=('Left','Right') if text=='LeftRight' else ('左側','右側')
        mode=classify(w['document']);key=text,mode;assert key not in seen,key;seen.add(key)
        groups,heads=expected(mode,left,right)
        texts=[''.join(word for word,_ in header+group) for group,header in zip(groups,heads)]
        height=118 if mode=='notes' else w['page_masters']['masters'][0]['body']['height']/65536
        actual=drivers[tuple(p['display'])];assert len(actual)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*actual]:
            reader=PdfReader(pdf,strict=True);extracted=[p.extract_text() for p in reader.pages];check_text(extracted,texts)
            positions=paragraphs(reader);check_origins(positions,groups,height)
            glyphs=glyph_pages(reader,left,right,allow_layout=mode=='notes');repeated_glyphs(glyphs,heads,left,right)
            annotations=[len(p.get('/Annots',[])) for p in reader.pages]
            assert annotations==([2,0,0,0] if mode=='notes' else [0]*len(groups)),annotations
            # Parent header height must remain reserved under each physical repeat.
            header_height={'body-child':16,'caption-header':16,'forced-header':32,'header-caption':32,'anchor-caption':32,'empty-caption':32,'header-deep':48,'header-child':64,'forced-child-header':64,'caption-backtrack':32,'notes':32}.get(mode)
            if left=='Left' and header_height is not None:
                repeated_page=next(i for i,h in enumerate(heads) if h)
                assert abs(positions[repeated_page][0][2]-(10+header_height+11.600006103515625))<=1/65536
            count+=1;pages+=len(reader.pages)
            if self_test:
                for bad in [extracted[1:],extracted+[''],[extracted[0]+left]+extracted[1:],extracted[:-1]+[extracted[-1]+right]]:
                    try:check_text(bad,texts)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted missing/duplicate original or repeated text')
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    bad=copy.deepcopy(positions);bad[0][0][field]=value
                    try:check_origins(bad,groups,height)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted altered source text or column')
                bad=copy.deepcopy(glyphs);bad[0][0][3]=not bad[0][0][3]
                try:repeated_glyphs(bad,heads,left,right)
                except AssertionError:rejected+=1
                else:raise AssertionError('accepted source/artifact exchange')
                if any(heads):
                    idx=next(i for i,h in enumerate(heads) if h);bad=copy.deepcopy(glyphs)
                    next(g for g in bad[idx] if g[3])[1]=999
                    try:repeated_glyphs(bad,heads,left,right)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted displaced repeated caption/header')
    modes=['body-child','header-child','header-caption','header-deep','forced-header','forced-child-header','caption-header','caption-backtrack','notes','empty-body','image-caption','anchor-caption','empty-caption']
    assert seen=={('LeftRight',k) for k in modes}|{('左側右側','header-caption')},seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test)
    print(f'PASS: {count} nested header PDFs / {pages} pages; original source, repeated caption/header glyphs, reserved height and single note demand; {rejected} alterations rejected')
