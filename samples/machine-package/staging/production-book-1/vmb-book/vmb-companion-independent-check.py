"""Independent authored one-chapter companion probe: INPUT RUN DECODE_DIR."""
import hashlib
import json
import runpy
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

job, run, decode = map(Path, sys.argv[1:])
package = json.loads((job / 'document-package.json').read_bytes())
sidecar = json.loads((job / 'typaxis-source-map.json').read_bytes())
assert len(sidecar['verification_companions']) == 1
record = sidecar['verification_companions'][0]
page = json.loads(record['page_json'])
assert record['source_index'] == 0
assert record['html_path'] == page['path'] == 'verification/' + page['chapterId'] + '.html'
assert record['qr']['url'] == page['url']
html = (job / record['html_path']).read_bytes()
assert hashlib.sha256(html).hexdigest() == record['html_sha256']
root = ET.fromstring(html)
ns = {'h': 'http://www.w3.org/1999/xhtml'}
assert root.tag == '{http://www.w3.org/1999/xhtml}html' and root.attrib['lang'] == 'ja'
assert root.find('h:head/h:title', ns).text == '検証情報'
assert not any(root.findall('.//h:' + name, ns) for name in ('script', 'link', 'iframe', 'object'))
csp = root.find("h:head/h:meta[@http-equiv='Content-Security-Policy']", ns)
assert csp is not None and csp.attrib['content'] == "default-src 'none'; img-src 'self' data:; style-src 'self'"
items = root.findall('h:body/h:main/h:ul/h:li', ns)
assert len(items) == len(page['results']) == 2
for item, result in zip(items, page['results']):
    assert [node.text for node in item.findall('h:span', ns)] == [result['resultId'], result['status'], result['shortId']]
    link = item.find('h:a', ns)
    assert link.attrib == {'href': result['url']} and link.text == '詳細'
blocks = package['document']['blocks']
assert blocks[0]['kind'] == 'heading' and blocks[0]['node_id'] == record['chapter_node_id']
assert blocks[0]['anchor_id'] == page['chapterId']
assert blocks[1]['kind'] == 'figure' and blocks[1]['node_id'] == record['figure_node_id']
assert blocks[2]['kind'] == 'paragraph'
assert blocks[1]['caption'][0]['children'][1]['target'] == {'kind': 'uri', 'uri': page['url']}
chapter = next(n for n in sidecar['nodes'] if n['node_id'] == record['chapter_node_id'])
assert chapter['origin'] == record['origin'] and chapter['parent_id'] == 0
figure = next(n for n in sidecar['nodes'] if n['node_id'] == record['figure_node_id'])
assert figure['origin'] == dict(record['origin'], generated=True)
assert figure['package_pointer'] == record['package_pointer'] == '/document/blocks/1'
for node in sidecar['nodes']:
    if node['package_pointer'] == record['package_pointer'] or node['package_pointer'].startswith(record['package_pointer'] + '/'):
        assert node['origin'] == dict(record['origin'], generated=True)
for name in ('source-decoded.json', 'page-decoded.json'):
    assert json.loads((decode / name).read_bytes()) == [page['url']]
metadata = run / 'companion-qr.json'
metadata.write_text(json.dumps(record['qr']) + '\n')
sys.argv = ['vmb-verification-qr-check.py', str(job), str(run), str(metadata)]
runpy.run_path(str(Path(__file__).with_name('vmb-verification-qr-check.py')), run_name='__main__')
report = json.loads((run / 'independent-qr.json').read_bytes())
report.update(companion=record, html_bytes=len(html), html_result_count=len(items),
              chapter_placement='immediately after actual chapter heading',
              independently_decoded=['source PNG', 'full PDF page rendered at 300 dpi'])
(run / 'independent-companion.json').write_text(json.dumps(report, ensure_ascii=False, indent=2) + '\n')
print(json.dumps(report, ensure_ascii=False))
