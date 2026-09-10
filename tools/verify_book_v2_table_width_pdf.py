#!/usr/bin/env python3
"""Verify physical table columns against declared masters and original font metrics."""
import argparse
import copy
import json
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_pdf_assembly import selected_master
from verify_book_v2_variable_width_pdf import original_metrics

U = 65536


def paragraphs(reader):
    tree = reader.trailer['/Root']['/StructTreeRoot']['/ParentTree']['/Nums']
    parents = dict(zip(tree[::2], tree[1::2]))
    pages = []
    for page in reader.pages:
        stack, found = [], []
        matrix = None
        owners = parents[page['/StructParents']]
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                props = args[1] if op == b'BDC' else {}
                stack.append([props.get('/ActualText'), False, props.get('/MCID')])
            elif op == b'EMC': stack.pop()
            elif op == b'Tm': matrix = [float(v) for v in args]
            elif op == b'Tj':
                for entry in reversed(stack):
                    if entry[0] is None: continue
                    if not entry[1]:
                        mcid = next((item[2] for item in reversed(stack) if item[2] is not None), None)
                        owner = None
                        if mcid is not None:
                            node = owners[mcid].get_object()
                            while node.get('/S') != '/P' and node.get('/P'):
                                node = node['/P']
                            if node.get('/S') == '/P':
                                owner = bytes(node['/ID'].original_bytes)
                        found.append([entry[0], matrix[4], matrix[5], owner])
                        entry[1] = True
                    break
        assert not stack
        pages.append(found)
    return pages


def half_even(raw):
    q, r = divmod(raw, 2)
    return q + (r and q % 2)


def check(actual, wire, metrics):
    tag, = [r['style_id'] for r in wire['style_sheet']['rules']
            if r['style_id'].startswith('table-page-width-')]
    notes = tag.endswith('notes')
    continuation = tag.endswith('continuation')
    harano = wire['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
    advance = metrics[0] if harano else lambda text: sum(round((300 if c == ' ' else 600)*12*U/1000) for c in text)
    text = wire['text_buffers'][0]['utf8']
    assert text == ('表の本文を元の字形で組む' if harano else 'Pro Pro Pro')
    tables = [d['blocks'][0] for d in wire['document']['footnotes']] if notes else [b for b in wire['document']['blocks'] if b['kind'] == 'table']
    assert len(actual) == 3
    assert len(tables) == (2 if continuation else 3)
    for table in tables:
        assert table['columns'] == [{'kind': 'fixed', 'width': 40*U}, {'kind': 'fraction', 'weight': 1}, {'kind': 'fraction', 'weight': 1}]
        assert table['body'][0]['cells'][0]['rowspan'] == 2
        assert table['body'][0]['cells'][1]['colspan'] == 2
    for page_index, values in enumerate(actual):
        master = selected_master(wire['page_masters'], page_index)
        region = master['footnote' if notes else 'body']
        left, width = region['x'], region['width']
        if notes:
            marker = max(advance(str(i)) for i in range(1, 4))
            left += marker + 12*U
            width -= marker + 12*U
        content = width - 5*U
        remaining = content - 40*U
        nested = remaining - 5*U
        # Caption, first column, second column, third column, and nested
        # columns use authored table/paragraph indents of 2/3 and 1/2 pt.
        first = half_even(remaining)
        second = remaining-first
        inner_first = half_even(nested)
        # Match the source order of original P structures through the PDF
        # ParentTree, so captions cannot mask an overwide first-column line.
        frames = [(left+3*U, content-3*U),
                  (left+3*U, 40*U-3*U),
                  (left+43*U, first-3*U),
                  (left+43*U+first, second-3*U),
                  (left+3*U, 40*U-3*U),
                  (left+45*U, nested-3*U),
                  (left+45*U, inner_first-3*U),
                  (left+45*U+inner_first, nested-inner_first-3*U),
                  (left+43*U, first-3*U),
                  (left+43*U+first, second-3*U)]
        if continuation and page_index == 0:
            frames = frames[:1]
        table_lines, markers = [], []
        for word, x, y, owner in values:
            if notes and y*U < region['y']: continue
            if notes and word == str(page_index+1):
                assert abs(x*U-(region['x']+marker-advance(word))) < 2
                markers.append(word)
                continue
            assert owner is not None
            table_lines.append((word,x,y,owner))
        owners = sorted({line[3] for line in table_lines})
        assert len(owners) == len(frames)
        groups = {owner: '' for owner in owners}
        expected = dict(zip(owners,frames))
        for word,x,y,owner in table_lines:
            start,capacity = expected[owner]
            assert abs(x*U-start) < 2, (page_index,word,x,start/U)
            assert advance(word) <= capacity+2, (word,advance(word)/U,capacity/U)
            groups[owner] += word
        assert list(groups.values()) == [text]*len(frames), (page_index,groups)
        assert markers == ([str(page_index+1)] if notes else [])


def verify(directory, font, self_test):
    metrics = original_metrics(font)
    drivers = {}
    for path in directory.glob('*.driver'):
        probe = json.loads(path.read_text())
        drivers.setdefault(tuple(probe['display']), []).append(probe['assembly']['pdf'])
    count = pages = rejected = 0
    seen = set()
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        tags = [r['style_id'] for r in wire['style_sheet']['rules'] if r['style_id'].startswith('table-page-width-')]
        if not tags:
            continue
        key = (wire['resources']['font_faces'][0]['media_type'], tags[0])
        assert key not in seen
        seen.add(key)
        pdfs = [probe['relations']['navigation']['assembly']['pdf'], *drivers[tuple(probe['display'])]]
        assert len(pdfs) == 2
        for pdf in pdfs:
            reader = PdfReader(pdf, strict=True)
            actual = paragraphs(reader)
            check(actual, wire, metrics)
            count += 1
            pages += len(reader.pages)
            if self_test:
                shifted = copy.deepcopy(actual)
                shifted[-1][-1][1] += 0.25
                missing = copy.deepcopy(actual)
                missing[-1].pop()
                for changed in [shifted, missing]:
                    try:
                        check(changed, wire, metrics)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('altered table paragraph accepted')
    assert len(seen) == 6, seen
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--harano-font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.harano_font, args.self_test)
    print(f'PASS: {count} table-width PDFs / {pages} pages / {rejected} alterations rejected; physical columns, spans, nested tables, captions and original text')
