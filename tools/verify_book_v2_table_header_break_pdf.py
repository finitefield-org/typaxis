#!/usr/bin/env python3
"""Verify controlled header breaks using actual PDF text, glyphs and artifacts."""
import argparse
import copy
import json
import re
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream


def glyph_pages(reader, left, right, *, allow_layout=False):
    pages = []
    for page in reader.pages:
        stack, glyphs = [], []
        matrix = cmap = None
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                artifact = args[0] == '/Artifact'
                if artifact:
                    if allow_layout and op == b'BDC' and args[1] == {'/Type': '/Layout'}:
                        artifact = False  # Footnote separator, never a repeated header.
                    else:
                        assert op == b'BDC' and args[1] == {'/Type': '/Pagination', '/Subtype': '/Header'}
                stack.append(artifact)
            elif op == b'EMC':
                stack.pop()
            elif op == b'Tf':
                font = page['/Resources']['/Font'][args[0]].get_object()
                pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>', font['/ToUnicode'].get_data().split(b'beginbfchar', 1)[1])
                cmap = {int(a, 16): bytes.fromhex(b.decode()).decode('utf-16-be') for a, b in pairs}
            elif op == b'Tm':
                matrix = [float(v) for v in args]
            elif op == b'Tj':
                raw = args[0].original_bytes if hasattr(args[0], 'original_bytes') else bytes(args[0])
                assert len(raw) == 2 and matrix[:4] == [1, 0, 0, -1]
                char = cmap[int.from_bytes(raw, 'big')]
                if char in (left[0], right[0]):
                    glyphs.append([char, matrix[4], matrix[5], any(stack)])
            elif op == b'TJ':
                raise AssertionError('controlled fixture requires explicit glyph matrices')
        assert not stack
        pages.append(glyphs)
    return pages


def check_text(texts, expected):
    assert [''.join(t.split()) for t in texts] == expected, (texts, expected)


def check_glyphs(pages, repeated, height, left, right):
    assert len(pages) == len(repeated)
    for glyphs, repeat in zip(pages, repeated):
        assert ''.join(g[0] for g in glyphs if g[3]) == repeat
        for char, x, y, _ in glyphs:
            assert abs(x - (10 if char == left[0] else 100)) <= 1 / 65536, (char, x)
            assert 10 < y <= 10 + height, y
    # A header repeat is paint only. It never consumes the first original
    # header on the page where that source resumes after its authored break.


def classify(table, left, right):
    lr = left + right
    a, b = table['head'][0]['cells']
    kinds = [v['kind'] for v in a['blocks']]
    if not b['blocks']:
        return 'blank', ['', '', ''], ['', '', '']
    if not table['body']:
        return 'empty-body', [lr, lr], ['', '']
    if kinds[0] == 'page_break':
        count = kinds.count('page_break')
        return ('leading' if count == 1 else 'consecutive', [''] * count + [lr * 3, lr * 2], [''] * (count + 1) + [left[0] + right[0]])
    if kinds[-1] == 'page_break':
        return 'trailing', [lr, lr * 3, lr * 2], ['', left[0] + right[0], left[0] + right[0]]
    if any(v['kind'] == 'page_break' for v in b['blocks']):
        return 'simultaneous', [lr, lr * 4], ['', '']
    repeat = left * 2 + right * 2
    marks = left[0] * 2 + right[0] * 2
    if len(table['head']) == 2:
        return 'head-span', [left, left + right * 2 + left + lr] + [repeat + left + lr] * 3, ['', ''] + [marks + left[0]] * 3
    if any(c['rowspan'] > 1 for row in table['body'] for c in row['cells']):
        return 'body-span', [left, left + right * 2 + lr + left] + [repeat + lr] * 2, ['', ''] + [marks] * 2
    if any(v['kind'] == 'page_break' for row in table['body'] for c in row['cells'] for v in c['blocks']):
        return 'body-break', [left, left + right * 2 + lr, repeat + left] + [repeat + lr] * 3, ['', ''] + [marks] * 4
    expected = [left, left + right * 2 + lr] + [repeat + lr] * 3
    repeated = ['', ''] + [marks] * 3
    if table.get('caption'):
        return 'caption', ['', lr, '', lr] + expected, [''] * 4 + repeated
    return 'asymmetric', expected, repeated


def verify(directory, self_test):
    drivers = {}
    for path in directory.glob('*.driver'):
        d = json.loads(path.read_text())
        drivers.setdefault(tuple(d['display']), []).append(d['assembly']['pdf'])
    count = total_pages = rejected = geometry = 0
    seen = set()
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text()); wire = probe['wire']; doc = wire['document']
        text = wire['text_buffers'][0]['utf8']
        if text not in ('LeftRight', '左側右側') or doc['footnotes'] or len(doc['blocks']) != 1:
            continue
        if any(r['style_id'] in ('nested-header-keep', 'nested-span-keep') for r in wire['style_sheet']['rules']): continue
        table = doc['blocks'][0]
        if table['kind'] != 'table' or not any(b['kind'] == 'page_break' for row in table['head'] for c in row['cells'] for b in c['blocks']):
            continue
        left, right = ('Left', 'Right') if text == 'LeftRight' else ('左側', '右側')
        kind, expected, repeated = classify(table, left, right)
        key = text, kind
        assert key not in seen, key
        seen.add(key)
        actual = drivers[tuple(probe['display'])]; assert len(actual) == 1
        height = wire['page_masters']['masters'][0]['body']['height'] / 65536
        for pdf in [probe['relations']['navigation']['assembly']['pdf'], *actual]:
            reader = PdfReader(pdf, strict=True)
            texts = [p.extract_text() for p in reader.pages]
            check_text(texts, expected); count += 1; total_pages += len(texts)
            if self_test:
                for bad in [texts[1:], texts + [''], [texts[0] + left] + texts[1:], texts[:-1] + [texts[-1] + left]]:
                    try: check_text(bad, expected)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted incorrect original/repeated header text')
            if kind == 'caption':
                continue  # Caption glyphs use a different text span and origin.
            pages = glyph_pages(reader, left, right)
            check_glyphs(pages, repeated, height, left, right); geometry += 1
            if self_test and any(pages):
                page = next(i for i, p in enumerate(pages) if p)
                for field, value in [(1, 999), (2, 999), (3, not pages[page][0][3])]:
                    bad = copy.deepcopy(pages); bad[page][0][field] = value
                    try: check_glyphs(bad, repeated, height, left, right)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted shifted/misclassified header glyph')
    assert seen == {('LeftRight', k) for k in ['asymmetric', 'leading', 'consecutive', 'trailing', 'simultaneous', 'caption', 'head-span', 'body-span', 'body-break', 'empty-body', 'blank']} | {('左側右側', 'asymmetric')}, seen
    return count, total_pages, geometry, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path); parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, geometry, rejected = verify(args.directory, args.self_test)
    print(f'PASS: {count} PDFs / {pages} pages / {geometry} glyph/artifact checks; {rejected} alterations rejected')
