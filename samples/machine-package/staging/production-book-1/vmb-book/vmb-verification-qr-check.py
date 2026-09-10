# Authored QR raster/PDF probe. Arguments: INPUT RUN QR_METADATA.
import io,json,sys,hashlib
from decimal import Decimal
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream
from PIL import Image
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'tools'))
import verify_pdf_structure as structure
from verify_vmb_scale_pdf import verify_form_geometry
job,run,meta_path=map(Path,sys.argv[1:]);package=json.loads((job/'document-package.json').read_bytes());meta=json.loads(meta_path.read_bytes());pdf=(run/'output.pdf').read_bytes()
report=structure.verify_production_pdf_structure(pdf,package,structure._production_source_ledger(package),expected_page_count=1)
reader=PdfReader(io.BytesIO(pdf));objects=reader.pages[0]['/Resources']['/XObject'];raster=[r for r in package['resources']['images']if r['media_type']=='png'];assert len(raster)==1
source=(job/raster[0]['uri']).read_bytes();assert hashlib.sha256(source).hexdigest()==meta['png_sha256']==raster[0]['expected_sha256']
original=Image.open(io.BytesIO(source));original.load();n=meta['modules']*meta['pixels_per_module'];assert original.size==(n,n) and original.mode=='RGB'
images=[(name,ref.get_object())for name,ref in objects.items()if ref.get_object()['/Subtype']=='/Image'];assert len(images)==1
name,image=images[0];assert image['/Width']==image['/Height']==n and image['/ColorSpace']=='/DeviceRGB' and image['/BitsPerComponent']==8
assert all(k not in image for k in ['/Decode','/Mask','/SMask','/ImageMask','/OC']) and image.get_data()==original.tobytes()
uses=[];matrix=None;stack=[]
for args,op in ContentStream(reader.pages[0].get_contents(),reader).operations:
 if op==b'q':stack.append(matrix)
 elif op==b'Q':matrix=stack.pop()
 elif op==b'cm':matrix=list(map(Decimal,map(str,args)))
 elif op==b'Do' and args[0]==name:uses.append(matrix)
assert not stack and len(uses)==1
figure=next(n for n in structure._production_source_nodes(package)if n['kind']=='figure');rule=next(r for r in package['style_sheet']['rules']if r['selector']==f"figure.vmb-figure-{figure['node_id']}")
width=next(d['value']['value']for d in rule['declarations']if d['name']=='width');physical=Decimal(width)/65536
assert abs(uses[0][0]-physical)<Decimal('0.000001') and uses[0][1:3]==[0,0] and abs(uses[0][3]+physical)<Decimal('0.000001')
forms=[ref.get_object()for ref in objects.values()if ref.get_object()['/Subtype']=='/Form'];geometries=[]
for resource in package['resources']['images']:
 if resource['media_type']!='svg-safe-2':continue
 svg=(job/resource['uri']).read_bytes();matches=[]
 for form in forms:
  try:matches.append(verify_form_geometry(svg,form,reader))
  except ValueError:pass
 assert len(matches)==1;geometries.append(matches[0])
normalize=lambda s:''.join(s.split());expected=normalize(''.join(structure._production_actual_text(package)))
for file in ['poppler-raw.txt','mupdf.txt']:assert normalize((run/file).read_text())==expected
report.update(qr_metadata=meta,qr_pixels=n*n,qr_width_raw=width,qr_use_matrix=list(map(str,uses[0])),math_geometry=geometries,extraction='Poppler raw and MuPDF source order match ignoring whitespace')
(run/'independent-qr.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n');print(json.dumps(report,ensure_ascii=False))
