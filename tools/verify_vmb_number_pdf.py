#!/usr/bin/env python3
"""Check the saved VMB numbering fixture against source, sidecar and Poppler text.

This is a fixture oracle, not a full-book text normalizer or PDF/UA validator.
Ordinary raster images have no extracted text; their source captions do. Decimal
list markers, the positive equation-number gap and the two-column table fixture's
cell separation use Poppler's raw-text space.
Every other non-line/page-boundary character must match the source exactly.
"""
import argparse
import copy
import hashlib
import json
import subprocess
from pathlib import Path


def source_text(package, sidecar, source, figure_titles=False, table_captions=False):
    assert not (figure_titles and table_captions)
    assert package['contract'] == 'typaxis.contract/1.5'
    assert hashlib.sha256(source).hexdigest() == sidecar['source_sha256']
    assert len(package['sources']) == 1
    declared = package['sources'][0]
    assert declared['source_id'] == 0 and declared['uri'] == sidecar['source_uri']
    assert declared['sha256'] == sidecar['source_sha256']
    assert declared['utf8_byte_length'] == len(source)
    buffers = {b['text_id']: b['utf8'].encode('utf-8') for b in package['text_buffers']}
    for buffer in package['text_buffers']:
        for mapping in buffer['mappings']:
            assert mapping['kind'] == 'identity'
            a, b = mapping['text_range'], mapping['source_span']
            assert b['source_id'] == 0
            assert buffers[buffer['text_id']][a['start_byte']:a['end_byte']] == source[b['start_byte']:b['end_byte']]
    raw, parents = {}, {}

    def walk(value, parent=None):
        if isinstance(value, list):
            for child in value:
                walk(child, parent)
        elif isinstance(value, dict):
            if 'node_id' in value:
                owner = value['node_id']
                assert owner not in raw
                raw[owner], parents[owner] = value, parent
                parent = owner
            for field in ('blocks', 'children', 'items', 'caption', 'head', 'body', 'cells', 'equation_number', 'footnotes'):
                if field in value:
                    walk(value[field], parent)

    walk(package['document'])

    def descendant(child, owner):
        while child is not None:
            if child == owner:
                return True
            child = parents[child]
        return False

    def text(span):
        return buffers[span['text_id']][span['start_byte']:span['end_byte']].decode('utf-8')

    bindings = package['document']['number_bindings']
    records = sidecar['number_bindings']
    assert len(bindings) == len(records) == (1 if table_captions else 4)
    numbers = {}
    projected = {n['node_id']: n for n in sidecar['nodes']}
    for binding, record in zip(bindings, records):
        assert set(binding) == {'anchor_id', 'owner_node_id', 'label_node_id', 'text_span'}
        assert all(record[k] == v for k, v in binding.items())
        anchor, owner, label = (binding[k] for k in ('anchor_id', 'owner_node_id', 'label_node_id'))
        assert anchor and anchor not in numbers and descendant(label, owner)
        node, leaf = raw[owner], raw[label]
        expected_kind = {'chapter': 'heading', 'theorem': 'semantic_container',
                         'figure': 'figure', 'table': 'table', 'equation': 'math_vector_block'}[record['source_kind']]
        assert node['kind'] == expected_kind
        if expected_kind == 'semantic_container':
            assert node['semantic_kind'] == 'result'
        assert projected[owner]['package_pointer'] == record['package_pointer']
        assert projected[owner]['origin'] == record['origin']
        if expected_kind == 'math_vector_block':
            assert node['equation_number'] is leaf
        else:
            assert leaf['kind'] == 'text'
        original, selected = leaf['text_span'], binding['text_span']
        assert original['text_id'] == selected['text_id']
        assert original['start_byte'] <= selected['start_byte'] < selected['end_byte'] <= original['end_byte']
        value = text(selected)
        assert value.strip() and all(ord(c) >= 32 and not 127 <= ord(c) <= 159 for c in value)
        assert hashlib.sha256(value.encode()).hexdigest() == record['display_sha256']
        assert record['number'] and isinstance(record['number'], dict)
        original_anchor = next((i for i, n in raw.items() if n.get('anchor_id') == anchor), None)
        if original_anchor is not None:
            assert descendant(original_anchor, owner)
        else:
            assert expected_kind == 'math_vector_block'
        numbers[anchor] = value

    counts = dict(references=0, math=0, figures=0, lists=0, tables=0)
    figure_paragraphs = []
    table_paragraphs = []

    def formula(node):
        counts['math'] += 1
        value = node.get('actual_text') or node['alt']
        assert value and value != '数式'
        return value

    def inlines(values):
        out = []
        for node in values:
            kind = node['kind']
            if kind == 'text':
                out.append(text(node['text_span']))
            elif kind in ('strong', 'emphasis', 'link'):
                out.append(inlines(node['children']))
            elif kind == 'reference':
                assert node['format'] == 'number'
                assert node['span']['start_byte'] == node['span']['end_byte']
                counts['references'] += 1
                out.append(numbers[node['target']])
            elif kind == 'math_vector':
                out.append(formula(node))
            elif kind in ('soft_break', 'hard_break'):
                out.append('\n')
            else:
                assert kind == 'anchor'
        return ''.join(out)

    def blocks(values):
        out = []
        for node in values:
            kind = node['kind']
            if kind in ('paragraph', 'heading'):
                out.append(inlines(node['children']))
            elif kind == 'semantic_container':
                out.append(blocks(node['blocks']))
            elif kind == 'figure':
                counts['figures'] += 1
                assert node['alt'] and node['placement'] == 'block'
                figure_paragraphs.append([p['classes'] for p in node['caption']])
                out.append(blocks(node['caption']))
            elif kind == 'list':
                counts['lists'] += 1
                assert node['ordered'] and node['start'] > 0
                for i, item in enumerate(node['items'], node['start']):
                    out.append(f'{i}. ' + blocks(item['blocks']))
            elif kind == 'table':
                counts['tables'] += 1
                table_paragraphs.append([p['classes'] for p in node['caption']])
                out.append(blocks(node['caption']))
                for section in ('head', 'body'):
                    for row in node[section]:
                        assert len(row['cells']) == 2
                        assert all(cell['rowspan'] == cell['colspan'] == 1 for cell in row['cells'])
                        out.append(' '.join(blocks(cell['blocks']) for cell in row['cells']))
            elif kind == 'math_vector_block':
                number = node['equation_number']
                assert number['minimum_gap'] > 0
                out.append(formula(node) + ' ' + text(number['text_span']))
            else:
                raise AssertionError(f'unsupported numbering fixture block: {kind}')
        return '\n'.join(out)

    assert not package['document']['footnotes']
    expected = blocks(package['document']['blocks'])
    wanted = (dict(references=1, math=3, figures=0, lists=0, tables=1) if table_captions else
              dict(references=5, math=7 if figure_titles else 6, figures=3, lists=1, tables=0))
    assert counts == wanted, counts
    if table_captions:
        assert table_paragraphs == [[['vmb-table-title'], ['vmb-table-caption']]]
    if figure_titles:
        assert figure_paragraphs == [[['vmb-figure-title'], ['vmb-figure-caption']],
                                     [['vmb-figure-title']], [['vmb-figure-caption']]]
    return expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('job', type=Path)
    parser.add_argument('pdf', type=Path)
    parser.add_argument('--pdftotext', required=True)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--figure-titles', action='store_true',
                        help='check the separate fixture with two rich figure titles and seven math occurrences')
    parser.add_argument('--table-captions', action='store_true',
                        help='check the table title/caption fixture with one number binding and three math occurrences')
    args = parser.parse_args()
    package = json.loads((args.job / 'document-package.json').read_text())
    sidecar = json.loads((args.job / 'typaxis-source-map.json').read_text())
    relative = Path(sidecar['source_uri'])
    assert not relative.is_absolute() and '..' not in relative.parts
    source_path = (args.job / relative).resolve()
    assert source_path.is_relative_to(args.job.resolve())
    source = source_path.read_bytes()
    expected = source_text(package, sidecar, source, args.figure_titles, args.table_captions)
    output = subprocess.run([args.pdftotext, '-raw', '-enc', 'UTF-8', str(args.pdf), '-'],
                            check=True, capture_output=True, text=True)
    assert not output.stderr, output.stderr
    normalize = lambda v: v.replace('\r', '').replace('\n', '').replace('\f', '')
    assert normalize(output.stdout) == normalize(expected), (output.stdout, expected)
    assert '\ufffc' not in output.stdout
    actions = [lambda p, s: p['document']['number_bindings'][0].update(owner_node_id=0),
               lambda p, s: p['document']['number_bindings'][0]['text_span'].update(start_byte=1),
               lambda p, s: p['document']['number_bindings'].append(p['document']['number_bindings'][0]),
               lambda p, s: s['number_bindings'][0].update(display_sha256='0' * 64),
               lambda p, s: s['number_bindings'][0].update(source_kind='figure'),
               lambda p, s: s['number_bindings'][0].update(package_pointer='/wrong-owner')]
    if args.figure_titles:
        def first_figure(p):
            return next(n for n in p['document']['blocks'] if n['kind'] == 'figure')

        actions += [lambda p, s: first_figure(p)['caption'][0].update(classes=['vmb-figure-caption']),
                    lambda p, s: first_figure(p)['caption'].pop(1),
                    lambda p, s: first_figure(p)['caption'].reverse()]
    if args.table_captions:
        def first_table(p):
            return next(n for n in p['document']['blocks'] if n['kind'] == 'table')

        actions += [lambda p, s: first_table(p)['caption'][0].update(classes=['vmb-table-caption']),
                    lambda p, s: first_table(p)['caption'].pop(1),
                    lambda p, s: first_table(p)['caption'].reverse()]
    if args.self_test:
        for mutate in actions:
            p, s = copy.deepcopy(package), copy.deepcopy(sidecar)
            mutate(p, s)
            try:
                source_text(p, s, source, args.figure_titles, args.table_captions)
            except (AssertionError, KeyError, IndexError, ValueError, TypeError):
                continue
            raise AssertionError('tampered numbering source accepted')
    if args.table_captions:
        detail = '1 number binding, 1 reference, 3 math occurrences, 1 table title and caption'
    else:
        detail = (f'4 number bindings, 5 references, {7 if args.figure_titles else 6} math occurrences, '
                  f'3 figures, {2 if args.figure_titles else 0} titles, 1 decimal list')
    print(f'PASS: exact source text, {detail}; {len(actions) if args.self_test else 0} tamper rejections')


if __name__ == '__main__':
    main()
