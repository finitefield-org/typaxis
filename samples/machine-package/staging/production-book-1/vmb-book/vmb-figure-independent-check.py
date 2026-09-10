# Single-page authored fixture check; requires bundled pypdf/Pillow.
# Arguments: staged input directory, retained run directory.
import io,json,sys,hashlib
from pathlib import Path
from decimal import Decimal
from pypdf import PdfReader
from pypdf.generic import ContentStream
from PIL import Image
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'tools'))
import verify_pdf_structure as structure
from verify_vmb_scale_pdf import verify_form_geometry
job,run=map(Path,sys.argv[1:]); package=json.loads((job/'document-package.json').read_bytes()); pdf=(run/'output.pdf').read_bytes()
report=structure.verify_production_pdf_structure(pdf,package,structure._production_source_ledger(package),expected_page_count=1)
reader=PdfReader(io.BytesIO(pdf)); assert len(reader.pages)==1
objects=reader.pages[0]['/Resources']['/XObject']; matched={}; resource_report=[]
for resource in package['resources']['images']:
 source=(job/resource['uri']).read_bytes(); assert hashlib.sha256(source).hexdigest()==resource['expected_sha256']
 candidates=[]
 for name,ref in objects.items():
  obj=ref.get_object()
  if resource['media_type']=='svg-safe-2':
   if obj['/Subtype']!='/Form':continue
   try: paths,commands=verify_form_geometry(source,obj,reader)
   except ValueError:continue
   candidates.append((name,dict(paths=paths,commands=commands)))
  else:
   if obj['/Subtype']!='/Image':continue
   original=Image.open(io.BytesIO(source)); original.load()
   if (obj['/Width'],obj['/Height'])!=original.size:continue
   assert obj['/ColorSpace']=='/DeviceRGB' and obj['/BitsPerComponent']==8
   assert all(k not in obj for k in ('/Decode','/Mask','/ImageMask','/OC'))
   if resource['media_type']=='png':
    if obj.get_data()!=original.convert('RGB').tobytes():continue
    alpha=original.convert('RGBA').getchannel('A').tobytes(); mask=obj['/SMask'].get_object()
    assert mask['/ColorSpace']=='/DeviceGray' and mask['/BitsPerComponent']==8
    assert (mask['/Width'],mask['/Height'])==original.size and mask.get_data()==alpha
    assert '/Decode' not in mask
   else:
    assert obj['/Filter']=='/DCTDecode' and '/SMask' not in obj
    actual=Image.open(io.BytesIO(obj.get_data()));actual.load()
    if actual.convert('RGB').tobytes()!=original.convert('RGB').tobytes():continue
   candidates.append((name,dict(pixels=original.width*original.height,width=original.width,height=original.height)))
 assert len(candidates)==1,(resource,candidates)
 name,detail=candidates[0];assert name not in matched;matched[name]=resource['image_id']
 resource_report.append(dict(image_id=resource['image_id'],xobject=str(name),**detail))
assert set(matched)==set(objects)
nodes=structure._production_source_nodes(package)
expected=[n for n in nodes if n['kind'] in ('figure','math_vector','math_vector_block')]
uses=[]; matrix=None; stack=[]
for args,op in ContentStream(reader.pages[0].get_contents(),reader).operations:
 if op==b'q':stack.append(matrix)
 elif op==b'Q':matrix=stack.pop()
 elif op==b'cm':matrix=list(map(Decimal,map(str,args)))
 elif op==b'Do':
  node=expected[len(uses)];assert matched[args[0]]==node['image_id']
  if node['kind']=='figure':
   rules=[r for r in package['style_sheet']['rules'] if r['selector']==f"figure.vmb-figure-{node['node_id']}"];assert len(rules)==1
   width=next(d['value']['value'] for d in rules[0]['declarations'] if d['name']=='width')
   dim=next(r for r in resource_report if r['image_id']==node['image_id'])
   want=Decimal(width)/65536;assert matrix[:4]==[want,0,0,-want*dim['height']/dim['width']]
  uses.append(dict(node_id=node['node_id'],image_id=node['image_id'],matrix=list(map(str,matrix))))
assert len(uses)==len(expected) and not stack
normalize=lambda s:''.join(s.split())
expected_text=normalize(''.join(structure._production_actual_text(package)))
for name in ['poppler-raw.txt','mupdf.txt']:
 assert normalize((run/name).read_text())==expected_text,name
report.update(resources=resource_report,uses=uses,extraction={'poppler_raw':'source-order exact ignoring whitespace','mupdf':'source-order exact ignoring whitespace','poppler_default':'spatial ordering differs; not used for source-order assertion'})
(run/'independent-figures.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n'); print(json.dumps(report,ensure_ascii=False))
