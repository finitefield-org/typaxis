#!/usr/bin/env python3
"""Check heterogeneous table continuation text against source columns and font metrics."""
import argparse
import copy
import json
from pathlib import Path

from pypdf import PdfReader
from verify_book_v2_pdf_assembly import selected_master
from verify_book_v2_source_structure import source_nodes
from verify_book_v2_table_width_pdf import paragraphs, half_even
from verify_book_v2_variable_width_pdf import original_metrics

U = 65536


def check(actual, wire, metrics):
    tag, = [r['style_id'] for r in wire['style_sheet']['rules']
            if r['style_id'].startswith('table-source-profile-')]
    mode, region_name = tag.removeprefix('table-source-profile-').split('-')
    notes = region_name == 'notes'
    assert mode in ('natural', 'split', 'forced')
    assert len(actual) == (5 if mode == 'split' else 3)
    harano = wire['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
    advance, ascent, descent = metrics if harano else (
        lambda text: sum(round((300 if c == ' ' else 600)*12*U/1000) for c in text),
        round(800*12*U/1000), round(200*12*U/1000))
    assert len(wire['text_buffers']) == 1
    source_text = wire['text_buffers'][0]['utf8']
    document = wire['document']
    # PDF /ID values enumerate semantic structures, including generated table
    # sections and note links. Reconstruct that mapping from the original wire;
    # they are not source node IDs and must not be inferred from painted order.
    paragraph_owners = {index: node['key'][0]
                        for index, node in enumerate(source_nodes(document))
                        if node['role'] == 'P'}
    if notes:
        assert len(document['blocks']) == len(document['footnotes']) == 1
        assert len(document['footnotes'][0]['blocks']) == 1
        table = document['footnotes'][0]['blocks'][0]
    else:
        assert len(document['blocks']) == 1 and not document['footnotes']
        table = document['blocks'][0]
    assert table['kind'] == 'table' and table['head'] == []
    assert table['columns'] == [{'kind': 'fraction', 'weight': 1}, {'kind': 'fraction', 'weight': 1}]
    assert len(table['body']) == 1 and len(table['body'][0]['cells']) == 2
    # These are declared fixture styles; unrelated old rules for absent source
    # containers do not influence the retained plain paragraph/table hierarchy.
    for rule in wire['style_sheet']['rules']:
        if rule['selector'] in ('paragraph', 'paragraph.left', 'paragraph.right', 'table'):
            for declaration in rule['declarations']:
                if declaration['name'] in ('start_indent', 'end_indent'):
                    assert declaration['value']['value'] == 0
                if declaration['name'] == 'font_size':
                    assert declaration['value']['value'] == 12*U
    descriptions = {}

    def paragraph(p, location):
        assert p['kind'] == 'paragraph'
        text = ''
        for child in p['children']:
            if child['kind'] == 'text':
                span = child['text_span']
                text += source_text.encode()[span['start_byte']:span['end_byte']].decode()
            else:
                assert notes and location == 'body' and child['kind'] == 'footnote_reference'
                text += '1'
        classes = p['classes']
        align = 'center' if 'left' in classes else 'end' if 'right' in classes else 'start'
        descriptions[p['node_id']] = (location, align, text)

    if notes:
        paragraph(document['blocks'][0], 'body')
    for column, cell in enumerate(table['body'][0]['cells']):
        assert cell['colspan'] == cell['rowspan'] == 1 and len(cell['blocks']) == 4
        for p in cell['blocks']:
            paragraph(p, column)
    caption = table.get('caption', [])
    if mode == 'forced':
        assert len(caption) == 3 and caption[1]['kind'] == 'page_break'
        paragraph(caption[0], 'caption')
        paragraph(caption[2], 'caption')
    else:
        assert not caption
    covered = dict.fromkeys(descriptions, '')
    marker_count = 0
    observed_widths = set()
    for index, entries in enumerate(actual):
        master = selected_master(wire['page_masters'], index)
        grouped = {}
        for word, x, y, raw_owner in entries:
            assert raw_owner is not None and len(raw_owner) == 4
            structure = int.from_bytes(raw_owner, 'big')
            assert structure in paragraph_owners
            owner = paragraph_owners[structure]
            assert owner in descriptions
            location, align, text = descriptions[owner]
            if notes and location == 'body' and word == '1' and y*U >= master['footnote']['y']:
                assert abs(x*U-master['footnote']['x']) < 2
                marker_count += 1
                continue
            grouped.setdefault((owner, y), []).append((word, x))
        for (owner, y), chunks in grouped.items():
            location, align, text = descriptions[owner]
            region = master['body' if location == 'body' or not notes else 'footnote']
            left, width = region['x'], region['width']
            if location != 'body':
                observed_widths.add(width)
                if notes:
                    left += advance('1')+12*U
                    width -= advance('1')+12*U
                if isinstance(location, int):
                    first = half_even(width)
                    if location == 0:
                        width = first
                    else:
                        left += first
                        width -= first
            used = sum(advance(word) for word, _ in chunks)
            assert used <= width+2
            slack = width-used
            offset = 0 if align == 'start' else slack if align == 'end' else slack//2
            expected_x = left+offset
            for word, x in chunks:
                assert abs(x*U-expected_x) < 2, (index, owner, word, x, expected_x/U)
                expected_x += advance(word)
                covered[owner] += word
            assert y*U >= region['y']+ascent-2
            assert y*U+descent <= region['y']+region['height']+2
    assert len(observed_widths) > 1
    assert marker_count == int(notes)
    assert covered == {owner: values[2] for owner, values in descriptions.items()}, covered


def verify(directory, font, self_test):
    metrics = original_metrics(font)
    drivers = {}
    for path in directory.glob('*.driver'):
        value = json.loads(path.read_text())
        drivers.setdefault(tuple(value['display']), []).append(value['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        tags = [r['style_id'] for r in wire['style_sheet']['rules'] if r['style_id'].startswith('table-source-profile-')]
        if not tags:
            continue
        key = (wire['resources']['font_faces'][0]['media_type'], tags[0])
        assert key not in seen
        seen.add(key)
        pdfs = [probe['relations']['navigation']['assembly']['pdf'], *drivers[tuple(probe['display'])]]
        assert len(pdfs) == 2
        for pdf in pdfs:
            actual = paragraphs(PdfReader(pdf, strict=True))
            check(actual, wire, metrics)
            count += 1
            pages += len(actual)
            if self_test:
                shifted, missing, changed, duplicated = [copy.deepcopy(actual) for _ in range(4)]
                shifted[-1][-1][1] += 0.25
                missing[-1].pop()
                changed[-1][-1][0] = 'lost'
                duplicated[-1].append(duplicated[-1][-1].copy())
                for altered in [shifted, missing, changed, duplicated, actual[1:], actual+[[]]]:
                    try:
                        check(altered, wire, metrics)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('altered table continuation accepted')
    assert len(seen) == 12, seen
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--harano-font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.harano_font, args.self_test)
    print(f'PASS: {count} table source-profile PDFs / {pages} pages / {rejected} alterations rejected; original source, physical columns, alignment and font metrics')
