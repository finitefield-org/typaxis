"""Authored FormalStatement + actual declared NPA source probe: INPUT RUN ORIGINAL_BOOK."""
import hashlib
import io
import json
import sys
from pathlib import Path
from pypdf import PdfReader
job,run,original=map(Path,sys.argv[1:])
package=json.loads((job/'document-package.json').read_bytes())
sidecar=json.loads((job/'typaxis-source-map.json').read_bytes())
assert len(sidecar['formal_statements'])==1
record=sidecar['formal_statements'][0]
statement=json.loads(record['statement_json'])
model=json.loads((original/'formal/results/de-morgan.json').read_bytes())
assert statement['resultId']==model['id'] and statement['formalName']==model['formalName']
assert statement['statementUnitIds']==model['statementUnitIds']
assert statement['sourceResourceId']==record['source_resource']['id']
assert record['source_resource']['path']==model['source']['path']
source=(original/model['source']['path']).read_bytes()
assert (job/record['source_uri']).read_bytes()==source
assert record['source_bytes']==len(source)
assert record['source_resource']['sha256']==model['source']['sha256']=='sha256:'+hashlib.sha256(source).hexdigest()
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'tools'))
import verify_pdf_structure as structure
from verify_vmb_scale_pdf import verify_form_geometry
nodes={n['node_id']:n for n in structure._production_source_nodes(package)}
projection={n['node_id']:n for n in sidecar['nodes']}
texts={t['text_span']['text_id']:t['utf8'] for t in sidecar['texts']}
header=nodes[record['heading_node_id']]
code=nodes[record['code_node_id']]
assert header['kind']==code['kind']=='paragraph'
assert header['children'][0]['anchor_id']==record['anchor_id']
assert texts[header['children'][-1]['text_span']['text_id']]==model['formalName']
assert projection[header['children'][-1]['node_id']]['origin']==record['formal_origin']
assert record['formal_origin']['path']=='formal/results/de-morgan.json'
code_origin=projection[record['code_node_id']]['origin']
assert code_origin['path']==model['source']['path'] and code_origin['vmb_id']==model['id']
assert code_origin['original_span']=={'source_id':0,'start_byte':0,'end_byte':len(source)}
rendered=[]
for child in code['children']:
    origin=projection[child['node_id']]['origin']
    assert origin['path']==model['source']['path'] and origin['vmb_id']==model['id']
    span=origin['original_span'];piece=source[span['start_byte']:span['end_byte']]
    if child['kind']=='text':
        text=texts[child['text_span']['text_id']]
        assert text==piece.decode().expandtabs(record['tab_width'])
        assert bool(origin.get('generated'))==(b'\t' in piece)
        rendered.append(text)
    else:
        assert child['kind']=='hard_break' and piece in (b'\n',b'\r',b'\r\n')
        rendered.append('\n')
expected=source.decode().replace('\r\n','\n').replace('\r','\n').expandtabs(record['tab_width'])
if expected.endswith('\n'):expected=expected[:-1]
assert ''.join(rendered)==expected
if statement['mode']=='appendix':
    assert record['parent_id']==0 and record['link_node_id']!=0
    assert len(sidecar['formal_appendices'])==1
    appendix=sidecar['formal_appendices'][0]
    assert nodes[appendix['node_id']]['kind']=='heading'
    link=nodes[record['link_node_id']]['children'][0]
    assert link['target']=={'kind':'internal','anchor_id':record['anchor_id']}
    assert projection[record['code_node_id']]['parent_id']==0
else:
    assert statement['mode'] in ('inline','collapsible') and record['parent_id']==record['owner_node_id']
    assert projection[record['code_node_id']]['parent_id']==record['owner_node_id']
    assert 'formal_appendices' not in sidecar
pdf=(run/'output.pdf').read_bytes()
report=structure.verify_production_pdf_structure(pdf,package,structure._production_source_ledger(package),expected_page_count=1)
reader=PdfReader(io.BytesIO(pdf))
forms=[r.get_object()for r in reader.pages[0]['/Resources']['/XObject'].values()if r.get_object()['/Subtype']=='/Form']
geometries=[]
for resource in package['resources']['images']:
    assert resource['media_type']=='svg-safe-2'
    matches=[]
    for form in forms:
        try:matches.append(verify_form_geometry((job/resource['uri']).read_bytes(),form,reader))
        except ValueError:pass
    assert len(matches)==1;geometries.append(matches[0])
expected_text=''.join(''.join(structure._production_actual_text(package)).split())
for name in ('poppler-raw.txt','mupdf.txt'):assert ''.join((run/name).read_text().split())==expected_text
report.update(formal_record=record,original_npa_bytes=len(source),original_npa_sha256=hashlib.sha256(source).hexdigest(),literal_code_lines=expected.split('\n'),math_geometry=geometries,extraction='Poppler raw and MuPDF source order match ignoring whitespace')
(run/'independent-formal.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False))
