#!/usr/bin/env python3
"""Compare real PDFs against source-declared independent body/note X translations."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_definition_table_pdf import check_positions
from verify_book_v2_pdf_assembly import selected_master

U=65536

def page_master(probe,page):
    names=probe['relations']['navigation']['assembly']['page_names']
    return selected_master(probe['wire']['page_masters'],page,names[page])

def translation(before,after,y):
    # These authored fixtures separate body paint above their note region.
    # Use source declarations, independently of retained placement rectangles.
    note=before['footnote']
    region='footnote' if note is not None and y>=note['y'] else 'body'
    return after[region]['x']-before[region]['x']

def compare(old,new,before,after):
    a,b=paragraphs(old),paragraphs(new)
    assert len(a)==len(b)
    wanted=[]
    for page,points in enumerate(a):
        first,last=page_master(before,page),page_master(after,page)
        wanted.append([(text,x*U+translation(first,last,round(y*U)),y*U) for text,x,y in points])
        assert list(old.pages[page]['/MediaBox'])==list(new.pages[page]['/MediaBox'])
        links_a=old.pages[page].get('/Annots',[]);links_b=new.pages[page].get('/Annots',[])
        assert len(links_a)==len(links_b)
        for left,right in zip(links_a,links_b):
            ra=list(map(float,left.get_object()['/Rect']));rb=list(map(float,right.get_object()['/Rect']))
            y=first['height']-round(ra[3]*U)
            delta=translation(first,last,y)/U
            expected=[ra[0]+delta,ra[1],ra[2]+delta,ra[3]]
            assert all(abs(v-w)*U<2 for v,w in zip(rb,expected)),(rb,expected)
    check_positions(b,wanted)
    assert old.named_destinations.keys()==new.named_destinations.keys()
    for name,dest in old.named_destinations.items():
        other=new.named_destinations[name]
        page=old.get_destination_page_number(dest)
        assert new.get_destination_page_number(other)==page
        first,last=page_master(before,page),page_master(after,page)
        delta=translation(first,last,first['height']-round(float(dest.top)*U))/U
        assert abs(float(other.left)-float(dest.left)-delta)*U<2
        assert abs(float(other.top)-float(dest.top))*U<2
    return b,wanted

def verify(directory,self_test,require_harano):
    drivers={};pairs={}
    for path in directory.glob('*.driver'):
        value=json.loads(path.read_text())
        drivers.setdefault(tuple(value['display']),[]).append(value['assembly']['pdf'])
    for path in directory.glob('*.json'):
        value=json.loads(path.read_text());wire=value['wire']
        tags=[r['style_id'] for r in wire['style_sheet']['rules'] if r['style_id'].startswith('horizontal-')]
        if not tags:continue
        assert len(tags)==1
        normalized=copy.deepcopy(wire)
        for master in normalized['page_masters']['masters']:
            master['body']['x']=0
            if master['footnote'] is not None:master['footnote']['x']=0
        key=json.dumps(normalized,sort_keys=True,separators=(',',':'))
        pairs.setdefault(key,[]).append(value)
    seen=set();count=pages=rejected=0
    for values in pairs.values():
        assert len(values)==2
        values.sort(key=lambda p:p['wire']['page_masters']['masters'][0]['body']['x'])
        before,after=values
        tag=next(r['style_id'] for r in before['wire']['style_sheet']['rules'] if r['style_id'].startswith('horizontal-'))
        mode=tag.removeprefix('horizontal-')
        text=before['wire']['text_buffers'][0]['utf8']
        seen.add(('harano' if text=='左側右側' else 'controlled',mode))
        for i,master in enumerate(after['wire']['page_masters']['masters']):
            assert master['body']['x']==[40,15,25,55][i]*U
            if master['footnote'] is not None:assert master['footnote']['x']==[10,45,20,60][i]*U
        first=[before['relations']['navigation']['assembly']['pdf'],*drivers[tuple(before['display'])]]
        last=[after['relations']['navigation']['assembly']['pdf'],*drivers[tuple(after['display'])]]
        assert len(first)==len(last)==2
        for old,new in zip(first,last):
            a,b=PdfReader(old,strict=True),PdfReader(new,strict=True)
            actual,wanted=compare(a,b,before,after)
            count+=2;pages+=len(a.pages)+len(b.pages)
            if self_test:
                changes=[paragraphs(a),actual[1:],actual+[[]]]
                target=next(i for i,p in enumerate(actual) if p)
                for field,value in [(0,'lost'),(1,999),(2,999)]:
                    altered=copy.deepcopy(actual);altered[target][0][field]=value;changes.append(altered)
                for altered in changes:
                    try:check_positions(altered,wanted)
                    except AssertionError:rejected+=1
                    else:raise AssertionError('accepted altered horizontal source or geometry')
    required={('controlled',m) for m in ['body','notes','table','note-table','references','nested','list','vectors']}
    if require_harano:required|={('harano','notes'),('harano','references')}
    assert required<=seen,seen
    return count,pages,rejected

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path);parser.add_argument('--self-test',action='store_true');parser.add_argument('--require-harano',action='store_true')
    args=parser.parse_args();count,pages,rejected=verify(args.directory,args.self_test,args.require_harano)
    print(f'PASS: {count} horizontal-origin PDFs / {pages} pages / {rejected} alterations rejected; body/note origins, text, links and destinations')
