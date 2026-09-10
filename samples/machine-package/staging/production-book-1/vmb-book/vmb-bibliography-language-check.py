# Authored bibliography fixture language check. Arguments: INPUT RUN.
import json,sys,collections
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'tools'));import verify_pdf_structure as v
job,run=map(Path,sys.argv[1:]);j=json.loads((job/'document-package.json').read_text());r=PdfReader(run/'output.pdf');cat=r.trailer['/Root'];root=cat['/StructTreeRoot'];nums=root['/ParentTree']['/Nums'];assert nums[0]==0;parents=nums[1]
buffers={t['text_id']:t['utf8'].encode() for t in j['text_buffers']}
expected=collections.Counter(v._production_text_span(n,buffers,'authored English') for n in v._production_source_nodes(j) if n['kind']=='text' and n.get('language')=='en')
observed=collections.Counter();ui_labels=[];stack=[];mcid=None
for args,op in ContentStream(r.pages[0].get_contents(),r).operations:
 if op==b'EMC':mcid=stack.pop();continue
 if op==b'BMC':stack.append(mcid);continue
 if op!=b'BDC':continue
 stack.append(mcid);props=args[1];mcid=props.get('/MCID',mcid)
 if '/ActualText' not in props:continue
 assert mcid is not None
 owner=parents[int(mcid)].get_object();lang=cat['/Lang']
 while True:
  if '/Lang' in owner:lang=owner['/Lang'];break
  if '/P' not in owner:break
  owner=owner['/P'].get_object()
 text=str(props['/ActualText'])
 if lang=='en':observed[text]+=1
 if text=='出典':ui_labels.append(str(lang))
assert not stack and mcid is None
assert observed==expected,(observed,expected)
assert ui_labels==['ja','ja'],ui_labels
normalize=lambda s:''.join(s.split());default_differs=normalize((run/'poppler.txt').read_text())!=normalize((run/'poppler-raw.txt').read_text());assert default_differs
report={'authored_english_text_spans':sum(expected.values()),'source_english_texts':dict(expected),'generated_source_labels_languages':ui_labels,'poppler_default_order_differs':default_differs};(run/'independent-languages.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(report)
