#!/usr/bin/env python3
"""Verify parent rowspans, nested source placement and full repeated header paint."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs, check_text
from verify_book_v2_table_header_break_pdf import glyph_pages


def classify(doc):
 t=doc['blocks'][0]
 if doc['footnotes']:return 'notes'
 if t['head']:
  if not t['body']:return 'empty-body'
  if t['head'][0]['cells'][0]['blocks'][0].get('caption'):return 'header-caption'
  blocks=t['head'][1]['cells'][0]['blocks'];k=[b['kind'] for b in blocks]
  if k==['paragraph','page_break','paragraph']:return 'header-forced'
  if k==['page_break','paragraph']:return 'header-leading'
  if k==['paragraph','page_break']:return 'header-trailing'
  if k==['page_break','page_break','paragraph']:return 'header-consecutive'
  return 'header'
 if t.get('caption'):return 'caption-keep'
 if t['body'][0]['cells'][1]['rowspan']==2:return 'right-span'
 if len(t['body'])==3:return 'three-rows'
 if t['body'][1]['cells'][0]['blocks'][0]['kind']=='page_break':return 'later-break'
 if len(t['body'][0]['cells'][1]['blocks'])==3:return 'simultaneous'
 child=t['body'][0]['cells'][0]['blocks'][0]
 return 'natural' if len(child['body'][0]['cells'][0]['blocks'])==4 else 'forced'


def expected(mode,left,right):
    # Source fixtures use 16 pt normal lines, 32 pt tall lines and 90/45 pt columns.
    # Entries are original word, x origin and baseline offset from the first normal line.
    l=(left,10,0); c=(right,55,8); b=(left+right,100,0)
    shift=lambda items,dy:[(w,x,y+dy) for w,x,y in items]
    tail=[l,(left,10,16),c]
    head=[l,(left,55,0),b,(left,100,16)]
    row=[l,b]
    rows=lambda n,top:sum((shift(row,top+i*16) for i in range(n)),[])
    if mode in ('natural','three-rows'):return [tail+[b,(left+right,100,16)],tail],[[],[]]
    if mode in ('forced','notes'):return [[l,b],[l,c,b],[c]],[[],[],[]]
    if mode=='right-span':return [[l,b],[l,c],[c,(left+right,10,32)]],[[],[],[]]
    if mode=='later-break':return [[b],tail+[b],tail],[[],[],[]]
    if mode=='simultaneous':return [[l,b],[l,c,b,(left+right,100,16)],[c]],[[],[],[]]
    if mode=='caption-keep':return [[(left+right,10,0),(left+right,10,16),(left+right,100,32)],tail+[b],tail],[[],[],[]]
    if mode=='header':return [head+rows(3,32),rows(2,32)],[[],head]
    if mode=='empty-body':return [head],[[]]
    if mode=='header-trailing':return [head,rows(3,32),rows(2,32)],[[],head,head]
    if mode=='header-caption':
        head=[(left+right,10,0),(left,10,16),(left,55,16),b,(left,100,16)]
        return [head+rows(2,48),rows(2,48),rows(1,48)],[[],head,head]
    if mode=='header-forced':return [head,[(left,100,0)]+rows(4,16),rows(1,48)],[[],[],head+[(left,100,32)]]
    if mode in ('header-leading','header-consecutive'):
        originals=[head[:-1],[(left,100,0)]+rows(4,16),rows(1,32)]
        repeats=[[],[],head]
        if mode=='header-consecutive': originals.insert(1,[]);repeats.insert(1,[])
        return originals,repeats
    raise AssertionError(mode)


def check_positions(actual,expected,baseline):
    assert len(actual)==len(expected),(len(actual),len(expected))
    for page,wanted in zip(actual,expected):
        assert len(page)==len(wanted),(page,wanted)
        for (word,x,y),(text,left,offset) in zip(page,wanted):
            assert word==text and abs(x-left)<1/65536 and abs(y-baseline-offset)<2/65536,(word,x,y,text,left,baseline+offset)


def check_repeated(actual,expected,left,right,baseline):
    found=[[[c,x,y] for c,x,y,repeated in page if repeated] for page in actual]
    wanted=[]
    for page in expected:
        glyphs=[]
        for word,x,y in page:
            if word==left+right:
                glyphs.extend([(left[0],x,y),(right[0],x+(28.79998779296875 if left=='Left' else 24),y)])
            else:glyphs.append((word[0],x,y))
        wanted.append(glyphs)
    check_positions(found,wanted,baseline)


def verify(directory,self_test):
    drivers={}
    for path in directory.glob('*.driver'):
        d=json.loads(path.read_text());drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        p=json.loads(path.read_text());w=p['wire'];doc=w['document']
        if not any(r['style_id']=='nested-span-keep' for r in w['style_sheet']['rules']):continue
        mode=classify(doc);text=w['text_buffers'][0]['utf8'];key=text,mode
        assert key not in seen,key;seen.add(key)
        left,right=('Left','Right') if text=='LeftRight' else ('左側','右側')
        original,repeat=expected(mode,left,right)
        baseline=21.6
        if text=='左側右側':
            # Unchanged Harano hhea: 1151/-286 units, UPEM 1000, size 12 pt.
            ascent=round(1151*12*65536/1000)
            height=ascent+round(286*12*65536/1000)
            baseline=10+ascent/65536
            tall_leading=((32*65536-height)//2)/65536
            original=[[(left+right,100,0)],
                      [(left,10,i*height/65536) for i in range(4)]+
                      [(right,55,tall_leading+i*32) for i in range(2)]+[(left+right,100,0)]]
            repeat=[[],[]]
        if mode=='notes':
            original[0].insert(1,('1',38.79998779296875,0))
            original[0].append(('1',10,86))
            original.append([]);repeat.append([])
            for page in original:page.extend([(left,29.199996948242188,86),(left,29.199996948242188,102)])
        actual=drivers[tuple(p['display'])];assert len(actual)==1
        for pdf in [p['relations']['navigation']['assembly']['pdf'],*actual]:
            reader=PdfReader(pdf,strict=True)
            positions=paragraphs(reader)
            check_positions(positions,original,baseline)
            glyphs=glyph_pages(reader,left,right,allow_layout=mode=='notes')
            check_repeated(glyphs,repeat,left,right,baseline)
            texts=[page.extract_text() for page in reader.pages]
            wanted=[''.join(w for w,_,_ in head+source) for head,source in zip(repeat,original)]
            check_text(texts,wanted)
            annotations=[len(page.get('/Annots',[])) for page in reader.pages]
            assert annotations==([2,0,0,0] if mode=='notes' else [0]*len(original)),annotations
            count+=1;pages+=len(reader.pages)
            if self_test:
                for changed in [texts[1:],texts+[''],[texts[0]+left]+texts[1:]]:
                    try:check_text(changed,wanted)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted missing or duplicated source page')
                index=next(i for i,page in enumerate(positions) if page)
                for field,value in [(0,'changed'),(1,999),(2,999)]:
                    bad=copy.deepcopy(positions);bad[index][0][field]=value
                    try:check_positions(bad,original,baseline)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted shifted rowspan source')
                index=next(i for i,page in enumerate(glyphs) if page)
                bad=copy.deepcopy(glyphs);bad[index][0][3]=not bad[index][0][3]
                try:check_repeated(bad,repeat,left,right,baseline)
                except AssertionError:rejected+=1
                else:raise AssertionError('accepted source/repeated artifact exchange')
    assert seen=={('LeftRight',mode) for mode in ['natural','forced','right-span','later-break','simultaneous','three-rows','caption-keep','header','header-forced','header-leading','header-trailing','header-consecutive','header-caption','empty-body','notes']}|{('左側右側','later-break')},seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test)
    print(f'PASS: {count} nested rowspan PDFs / {pages} pages; source baselines, column origins, repeated header roles and note annotations; {rejected} alterations rejected')
