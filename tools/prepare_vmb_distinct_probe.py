#!/usr/bin/env python3
"""Verify real-engine template fixtures and prepare diagnostic scale packages.

This authoring harness is not the formal VMB RenderBook exporter. It preserves
all 5,000 generated expression identities and never substitutes color variants.
"""
import argparse
import copy
import hashlib
import json
import shutil
import struct
import zlib
from pathlib import Path
from decimal import Decimal, ROUND_HALF_EVEN
from xml.etree import ElementTree as ET


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify(root):
    index_bytes = (root / 'fixture-index.json').read_bytes()
    index = json.loads(index_bytes)
    require(index['algorithm'] == 'vmb.typaxis-engine-fixtures/1', 'index algorithm')
    require(index['rules_version'] == 'vmb.typaxis-safe-svg/1', 'adapter rules')
    require(len(index['cases']) == 5000, 'exact expression count')
    source_hashes, derived_hashes, geometry_hashes, identities = set(), set(), set(), set()
    for i, case in enumerate(index['cases'], 1):
        mode = 'inline' if i % 2 else 'block'
        stem = f'distinct-{i:04d}-{mode}'
        require(case['case_id'] == f'distinct-{i:04d}' and case['display'] == mode, 'case identity')
        require(case['tex'] == rf'x_{{{i}}}=\frac{{{i}}}{{{i+1}}}', f'template {i}')
        require(case['speech'] == f'x subscript {i} equals {i} over {i+1}', f'speech {i}')
        require(case['font_size_raw'] == 720896, '11pt font size')
        require(case['source_svg'] == stem+'.source.svg' and case['derived_svg'] == stem+'-720896.svg', 'file names')
        source = (root / case['source_svg']).read_bytes()
        derived = (root / case['derived_svg']).read_bytes()
        require(digest(source) == case['source_sha256'] and digest(derived) == case['derived_sha256'], 'file hashes')
        artifact = case['engine_artifact']
        require(artifact['SVG_SHA256'] == 'sha256:'+digest(source), 'engine raw SVG binding')
        require(artifact['EngineID'] == 'vmb-go-math-svg' and artifact['EngineVersion'] == '2.0.0', 'real engine identity')
        identities.add(tuple(sorted((k, v) for k, v in artifact.items() if k not in ['RequestSHA256', 'SVG_SHA256'])))
        svg = ET.fromstring(derived)
        # Ignore XML whitespace and paint: distinctness must come from path geometry.
        paths = [n.attrib['d'].split() for n in svg.iter() if n.tag.rsplit('}', 1)[-1] == 'path']
        require(len(paths) == case['path_count'] and bool(paths), 'path count')
        geometry_hashes.add(digest(json.dumps(paths, separators=(',', ':')).encode()))
        source_hashes.add(digest(source))
        derived_hashes.add(digest(derived))
        require(svg.attrib['width'].endswith('pt') and svg.attrib['height'].endswith('pt'), 'physical SVG dimensions')
        for dimension in ['width', 'height']:
            require(Decimal(svg.attrib[dimension][:-2]) == (Decimal(case['metrics']['viewport'][dimension])/65536).quantize(Decimal('0.000001'), rounding=ROUND_HALF_EVEN), 'decimal6 physical SVG metrics')
        require(case['metrics']['viewport']['height'] == case['metrics']['ascent'] + case['metrics']['descent'], 'vertical metrics')
    require(len(source_hashes) == len(derived_hashes) == len(geometry_hashes) == 5000, '5000 distinct raw, derived and path geometry hashes')
    require(len(identities) == 1, 'one fixed engine artifact identity')
    return index, {'index_sha256': digest(index_bytes), 'expressions': 5000, 'distinct_path_geometry': 5000,
                   'total_paths': sum(c['path_count'] for c in index['cases']),
                   'total_source_segments': sum(c['segment_count'] for c in index['cases']),
                   'public_build': False, 'full_book': False}


def png(index):
    def chunk(tag, data):
        return struct.pack('>I', len(data))+tag+data+struct.pack('>I', zlib.crc32(tag+data))
    pixels = bytes([0, index*5, 255-index*5, index*3, 255, 0, index, 0, index, 255, 0, 0, 255-index, index])
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR', struct.pack('>IIBBBBB', 2, 2, 8, 2, 0, 0, 0))+chunk(b'IDAT', zlib.compress(pixels))+chunk(b'IEND', b'')


def prepare(root, output, kind, index, result):
    output.mkdir(mode=0o700)  # Exclusive: never overwrite a previous run.
    base = Path(__file__).resolve().parents[1] / 'samples/machine-package/profiles/production-book-1/combined/job'
    package = json.loads((base / 'document-package.json').read_bytes())
    for font in package['resources']['font_faces']:
        shutil.copyfile(base/font['uri'], output/font['uri'])
    package['outline']['entries'] = []
    package['document']['footnotes'] = []
    package['document']['blocks'] = []
    package['text_buffers'] = []
    package['page_masters']['masters'][0]['footnote'] = None
    package['metadata']['title'] = 'VMB actual-engine scale probe: '+kind
    package['resources']['images'] = []
    mapping = []
    for i in range(5000):
        if kind == 'mixed' and i >= 4952:
            data, uri, media, case = png(i-4952), f'raster-{i-4952:02d}.png', 'png', None
        else:
            case = index['cases'][i % 100 if kind == 'alias' else i]
            data, uri, media = (root/case['derived_svg']).read_bytes(), f'math-{i:04d}.svg', 'svg-safe-2'
        (output/uri).write_bytes(data)
        image = {'image_id': i, 'uri': uri, 'media_type': media, 'expected_sha256': digest(data)}
        if case is not None:
            artifact = case['engine_artifact']
            image['vector_provenance'] = {'engine_id': artifact['EngineID'], 'engine_version': artifact['EngineVersion'], 'rules_version': index['rules_version']}
        package['resources']['images'].append(image)
        mapping.append(case)
    source, offset, next_node = [], 0, 1
    total = 8000 if kind == 'alias' else 5000
    chapter = None
    def span(start, end): return {'source_id': 0, 'start_byte': start, 'end_byte': end}
    for occurrence in range(total):
        if occurrence % 1000 == 0:
            chapter = {'kind': 'semantic_container', 'node_id': next_node, 'span': span(offset, offset),
                       'classes': [], 'semantic_kind': 'result', 'anchor_id': None, 'blocks': []}
            next_node += 1
            package['document']['blocks'].append(chapter)
        image_id = occurrence % 5000
        case = mapping[image_id]
        text = case['tex'] if case else f'[PNG {image_id-4952}]'
        end = offset+len(text.encode())
        node_span = span(offset, end)
        source.append(text+'\n')
        if case:
            text_id = len(package['text_buffers'])
            package['text_buffers'].append({'text_id': text_id, 'utf8': text, 'mappings': [
                {'kind': 'identity', 'source_span': node_span, 'text_range': {'start_byte': 0, 'end_byte': len(text.encode())}}]})
            formula = {'kind': 'math_vector' if case['display'] == 'inline' else 'math_vector_block',
                       'node_id': next_node, 'span': node_span, 'image_id': image_id, 'metrics': copy.deepcopy(case['metrics']),
                       'source_tex': {'text_span': {'text_id': text_id, 'start_byte': 0, 'end_byte': len(text.encode())}},
                       'alt': case['speech'], 'actual_text': None}
            if case['display'] == 'inline':
                formula['node_id'] += 1
                formula['spacing'] = {'before': 0, 'after': 0}
                node = {'kind': 'paragraph', 'node_id': next_node, 'span': node_span, 'classes': [], 'children': [formula]}
                next_node += 2
            else:
                formula.update(classes=[], equation_number=None)
                node = formula
                next_node += 1
        else:
            node = {'kind': 'figure', 'node_id': next_node, 'span': node_span, 'classes': [],
                    'image_id': image_id, 'alt': text, 'caption': [], 'placement': 'block'}
            next_node += 1
        chapter['blocks'].append(node)
        chapter['span']['end_byte'] = end
        offset = end+1
    raw = ''.join(source).encode()
    package['sources'] = [{'source_id': 0, 'uri': 'input.tsf', 'utf8_byte_length': len(raw), 'sha256': digest(raw)}]
    (output/'input.tsf').write_bytes(raw)
    payload = json.dumps(package, separators=(',', ':'), ensure_ascii=False).encode()+b'\n'
    (output/'document-package.json').write_bytes(payload)
    result.update(case=kind, package_sha256=digest(payload), resources=5000, placements=total,
                  distinct_resources=len({r['expected_sha256'] for r in package['resources']['images']}))
    (output/'probe-inventory.json').write_text(json.dumps(result, indent=2)+'\n')
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('corpus', type=Path)
    parser.add_argument('--output', type=Path)
    parser.add_argument('--case', choices=['distinct', 'mixed', 'alias'], default='distinct')
    args = parser.parse_args()
    inventory, result = verify(args.corpus)
    if args.output:
        result = prepare(args.corpus, args.output, args.case, inventory, result)
    print(json.dumps(result, indent=2))
