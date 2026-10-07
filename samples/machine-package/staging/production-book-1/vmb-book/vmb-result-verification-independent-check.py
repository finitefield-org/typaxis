"""Authored result verification probe. Arguments: INPUT RUN."""
import hashlib
import io
import json
import runpy
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from pypdf import PdfReader

job,run=map(Path,sys.argv[1:])
package=json.loads((job/'document-package.json').read_bytes())
sidecar=json.loads((job/'typaxis-source-map.json').read_bytes())
assert len(sidecar['verification_results'])==1
record=sidecar['verification_results'][0]
source=json.loads(record['source_json'])
display=source['display'];style=display['linkStyle']
assert source['formal_result_id']=='formal.fraction'
assert display['status']=='verified' and display['badge']=='NPA VERIFIED'
assert display['certificateHash']=='sha256:'+'a'*64
assert display['kernelName']=='npa-kernel' and display['kernelVersion']=='1.2.3'
sys.path.insert(0,str(Path(__file__).resolve().parents[5]/'tools'))
import verify_pdf_structure as structure
from verify_vmb_scale_pdf import verify_form_geometry
nodes={n['node_id']:n for n in structure._production_source_nodes(package)}
projection={n['node_id']:n for n in sidecar['nodes']}
container=nodes[record['node_id']]
assert container['semantic_kind']=='result' and container['anchor_id']==record['vmb_id']=='result.test'
assert projection[record['node_id']]['origin']==record['origin']
texts={t['text_span']['text_id']:t['utf8'] for t in sidecar['texts']}
seen=[]
def descendants(node):
    if isinstance(node,list):
        for child in node: yield from descendants(child)
    elif isinstance(node,dict):
        if 'node_id' in node: yield node
        for child in node.values():
            if isinstance(child,(list,dict)): yield from descendants(child)
for index,node_id in enumerate(record['generated_nodes'],record['start_block']):
    node=nodes[node_id]
    assert container['blocks'][index]['node_id']==node_id
    assert projection[node_id]['parent_id']==record['node_id']
    for child in descendants(node):
        assert projection[child['node_id']]['origin']==dict(record['origin'],generated=True)
        if child['kind']=='text': seen.append(texts[child['text_span']['text_id']])
for text in (display['badge'],display['shortId'],display['certificateHash'],display['kernelName']+' '+display['kernelVersion']):assert text in seen
if style=='direct':
    targets=[n['target'] for n in descendants(container)if n.get('kind')=='link']
    assert {'kind':'uri','uri':display['url']} in targets
elif style=='qr-per-result':
    assert record['qr']['url']==display['url']
else:
    assert style=='qr-per-chapter'
    companion=sidecar['verification_companions'][0]
    page=json.loads(companion['page_json'])
    assert record['chapter_id']==page['chapterId']
    assert page['results']==[{'resultId':source['formal_result_id'],'status':display['status'],'shortId':display['shortId'],'url':'https://verify.example/result/fraction'}]
    html=(job/companion['html_path']).read_bytes()
    assert hashlib.sha256(html).hexdigest()==companion['html_sha256']
    root=ET.fromstring(html);ns={'h':'http://www.w3.org/1999/xhtml'}
    assert [s.text for s in root.findall('.//h:li/h:span',ns)]==[source['formal_result_id'],display['status'],display['shortId']]
    assert root.find('.//h:li/h:a',ns).attrib['href']==page['results'][0]['url']
if style!='direct':
    meta=record['qr'] if style=='qr-per-result' else sidecar['verification_companions'][0]['qr']
    meta_path=run/'result-qr.json';meta_path.write_text(json.dumps(meta)+'\n')
    sys.argv=['vmb-verification-qr-check.py',str(job),str(run),str(meta_path)]
    runpy.run_path(str(Path(__file__).with_name('vmb-verification-qr-check.py')),run_name='__main__')
    report=json.loads((run/'independent-qr.json').read_bytes())
    for name in ('source-decoded.json','page-decoded.json'):
        assert json.loads((run/name).read_bytes())==[meta['url']]
    report['independently_decoded']=['source PNG','full PDF page rendered at 300 dpi']
else:
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
    expected=''.join(''.join(structure._production_actual_text(package)).split())
    for name in ('poppler-raw.txt','mupdf.txt'):assert ''.join((run/name).read_text().split())==expected
    report.update(math_geometry=geometries,extraction='Poppler raw and MuPDF source order match ignoring whitespace')
report.update(verification_source=source,source_record=record)
(run/'independent-verification.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(report,ensure_ascii=False))
