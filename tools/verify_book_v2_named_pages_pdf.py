#!/usr/bin/env python3
"""Independently check named scopes, forced pages, tables and note tails."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_definition_table_pdf import check_positions
from verify_book_v2_pdf_assembly import selected_master
U=65536


def expected(wire):
    blocks=wire['document']['blocks']
    notes=wire['document']['footnotes']
    text=wire['text_buffers'][0]['utf8']
    japanese=text=='左側右側'
    assert japanese or text=='Result'
    explicit=any(r['style_id']=='explicit-break' for r in wire['style_sheet']['rules'])
    mode=('standalone' if explicit and blocks[0]['kind']=='page_break' else 'explicit' if explicit else 'notes' if notes else 'table' if blocks[0]['kind']=='table'
          else 'references' if blocks[0].get('anchor_id')=='first'
          else 'forced' if blocks[0]['blocks'][0]['kind']=='page_break' else 'nested')
    names={'references':[None,'appendix',None], 'nested':['appendix','short','appendix',None],
           'standalone':['short',None,'short',None,None,'short','short'],
           'explicit':['short','appendix','short','appendix','appendix','short','short'],
           'forced':['appendix']*5,'table':['appendix','appendix',None],'notes':['appendix']*4}[mode]
    if mode in ('explicit','standalone'):
        original=blocks if mode=='standalone' else blocks[0]['blocks']
        assert [b['kind'] for b in original]==['page_break','paragraph','page_break','page_break','paragraph','page_break']
        assert [b['classes'] for b in original if b['kind']=='page_break']==[['short'],['short'],[],['short']]
    ascent=round((1151 if japanese else 800)*12*U/1000)
    descent=round((286 if japanese else 200)*12*U/1000)
    line=max(16*U,ascent+descent)
    baseline=ascent+(line-ascent-descent)//2
    digit=round((471 if japanese else 600)*12*U/1000)
    advance=4*12*U if japanese else 6*digit
    source=[];boxes=[];annots=[]
    for page,name in enumerate(names):
        master=selected_master(wire['page_masters'],page,name)
        b=master['body'];x,y=b['x'],b['y']
        # The original semantic-base rule uses a four-fixed-unit indent;
        # nested scopes retain it independently of their physical page names.
        container=next(r for r in wire['style_sheet']['rules'] if r['style_id']=='semantic-base')
        indent=next(d['value']['value'] for d in container['declarations'] if d['name']=='start_indent')
        depth=1 if mode in ('references','forced','explicit') else [1,2,1,0][page] if mode=='nested' else 0
        x+=depth*indent
        boxes.append([0,0,master['width']/U,master['height']/U])
        if mode=='references':
            found=[(text,x,y+baseline),(str([3,1,2][page]),x+advance,y+baseline)]
        elif mode in ('explicit','standalone'):found=[(text,x,y+baseline)] if page in (1,4) else []
        elif mode=='forced':found=[] if page in (0,2,4) else [(text,x,y+baseline)]
        elif mode=='table' and page<2:
            found=[(text,x+column*b['width']//2,y+i*line+baseline) for column in range(2) for i in range(2)]
        elif mode=='notes':
            found=[(text,x,y+baseline),('1',x+advance,y+baseline)] if page==0 else []
            note=master['footnote'];count=[2,2,2,1][page]
            top=note['y']+note['height']-count*line
            if page==0:found.append(('1',note['x'],top+baseline))
            found += [(text,note['x']+digit+12*U,top+i*line+baseline) for i in range(count)]
        else:found=[(text,x,y+baseline)]
        source.append(found)
        annots.append(1 if mode=='references' else 2 if mode=='notes' and page==0 else 0)
    return mode,names,source,boxes,annots


def verify(directory,self_test,require_harano,require_explicit=False):
    drivers={}
    for path in directory.glob('*.driver'):
        d=json.loads(path.read_text());drivers.setdefault(tuple(d['display']),[]).append(d['assembly']['pdf'])
    seen=set();count=pages=rejected=0
    for path in directory.glob('*.json'):
        value=json.loads(path.read_text());wire=value['wire']
        if any(r['style_id'].startswith('horizontal-') for r in wire['style_sheet']['rules']):continue
        if not any(r['style_id']=='named-scope' for r in wire['style_sheet']['rules']):continue
        mode,names,wanted,boxes,annots=expected(wire)
        key=(wire['text_buffers'][0]['utf8'],mode)
        assert key not in seen,key
        seen.add(key)
        assert value['relations']['navigation']['assembly']['page_names']==names
        matching=drivers[tuple(value['display'])];assert len(matching)==1
        for pdf in [value['relations']['navigation']['assembly']['pdf'],*matching]:
            reader=PdfReader(pdf,strict=True);actual=paragraphs(reader)
            check_positions(actual,wanted)
            assert [list(p['/MediaBox']) for p in reader.pages]==boxes
            assert [len(p.get('/Annots',[])) for p in reader.pages]==annots
            count+=1;pages+=len(reader.pages)
            if self_test:
                changed=[actual[1:],actual+[[]]]
                target=next(i for i,p in enumerate(actual) if p)
                for field,replacement in [(0,'lost'),(1,999),(2,999)]:
                    altered=copy.deepcopy(actual);altered[target][0][field]=replacement;changed.append(altered)
                altered=copy.deepcopy(actual);altered[target].append(altered[target][0]);changed.append(altered)
                for altered in changed:
                    try:check_positions(altered,wanted)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted altered named-page source or geometry')
                # Independent source expectations must distinguish a default
                # master from the actual named first/even/ordinary choice.
                wrong=[list(p['/MediaBox']) for p in reader.pages]
                page=next(i for i,n in enumerate(names) if n is not None)
                default=selected_master(wire['page_masters'],page)
                wrong[page]=[0,0,default['width']/U,default['height']/U]
                assert wrong!=boxes;rejected+=1
    required={('Result',m) for m in ['references','nested','forced','table','notes']}
    if require_harano:required|={('左側右側',m) for m in ['references','nested']}
    if require_explicit:
        required|={('Result','explicit'),('Result','standalone')}
        if require_harano:required|={('左側右側','explicit'),('左側右側','standalone')}
    assert required<=seen,seen
    return count,pages,rejected


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true');parser.add_argument('--require-harano',action='store_true')
    parser.add_argument('--require-explicit',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test,args.require_harano,args.require_explicit)
    print(f'PASS: {count} named-page PDFs / {pages} pages / {rejected} alterations rejected; nested scopes, physical masters, forced pages, table continuation, note tails and reference labels')
