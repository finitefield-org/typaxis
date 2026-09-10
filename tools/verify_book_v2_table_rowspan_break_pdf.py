#!/usr/bin/env python3
"""Check actual page text and column/row origins for controlled rowspan breaks."""
import argparse
import copy
import json
import re
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream


def text_check(texts, expected):
    assert [''.join(t.split()) for t in texts] == expected, (texts, expected)


def origins(reader, left, right):
    result = []
    for page in reader.pages:
        found = []
        cmap = matrix = None
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op == b'Tf':
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
                    found.append([char, matrix[4], matrix[5]])
            elif op == b'TJ':
                raise AssertionError('controlled fixture requires explicit glyph matrices')
        result.append(found)
    return result


def geometry_check(pages, left, right, later):
    expected = [[right[0]], [left[0], right[0]], [left[0]]] if later else [[left[0]], [left[0], right[0], left[0]], [right[0]]]
    assert [[c[0] for c in p] for p in pages] == expected
    for page in pages:
        for char, x, y in page:
            assert abs(x - (10 if char == left[0] else 100)) <= 1 / 65536, (char, x)
            assert 10 < y < 58, y
    if not later:
        # The following row starts within the tall spanning line's 32pt band,
        # in its own column; it is neither overprinted nor deferred below it.
        distance = pages[1][2][2] - pages[1][0][2]
        assert 12 <= distance < 32, distance


def verify(directory, self_test):
    drivers = {}
    for path in directory.glob('*.driver'):
        d = json.loads(path.read_text())
        drivers.setdefault(tuple(d['display']), []).append(d['assembly']['pdf'])
    count = total_pages = rejected = geometry = 0
    seen = set()
    for path in directory.glob('*.json'):
        p = json.loads(path.read_text())
        wire = p['wire']; text = wire['text_buffers'][0]['utf8']; doc = wire['document']
        if text not in ('LeftRight', '左側右側') or doc['footnotes'] or len(doc['blocks']) != 1:
            continue
        if any(r['style_id']=='nested-span-keep' for r in wire['style_sheet']['rules']): continue
        table = doc['blocks'][0]
        if table['kind'] != 'table' or not any(c['rowspan'] > 1 for row in table['body'] for c in row['cells']):
            continue
        if any(b['kind'] == 'page_break' for row in table['head'] for c in row['cells'] for b in c['blocks']):
            continue  # Dedicated original-header/repetition oracle covers these.
        left, right = ('Left', 'Right') if text == 'LeftRight' else ('左側', '右側')
        expected = [left, left + right + left, right]; kind = 'right-span'
        if len(table.get('caption', [])) == 2:
            expected, kind = [text, text + left, left], 'caption-band-keep'
        elif not table['body'][0]['cells'][1]['blocks']:
            expected, kind = ['', '', ''], 'blank'
        elif len(table['body']) == 3:
            expected, kind = [left + right, left + right], 'three-rows'
        elif table['body'][0]['cells'][0]['rowspan'] == 2:
            if table['body'][1]['cells'][0]['blocks'][0]['kind'] == 'page_break':
                expected, kind = [right, left + right, left], 'later-break'
            else:
                expected, kind = [left + right, left + right + right], 'simultaneous'
        elif table['head']:
            expected, kind = [left + right + left, left + right + left + right + right + left], 'header'
        elif table.get('caption'):
            expected, kind = ['', text, '', text] + expected, 'caption'
        key = text, kind
        assert key not in seen, key
        seen.add(key)
        actual = drivers[tuple(p['display'])]; assert len(actual) == 1
        for pdf in [p['relations']['navigation']['assembly']['pdf'], *actual]:
            reader = PdfReader(pdf, strict=True)
            texts = [page.extract_text() for page in reader.pages]
            text_check(texts, expected); count += 1; total_pages += len(texts)
            if self_test:
                for bad in [texts[1:], texts + [''], [texts[0] + left] + texts[1:], texts[:-1] + [texts[-1] + left]]:
                    try: text_check(bad, expected)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted incorrect physical-page source text')
            if kind in ('right-span', 'later-break'):
                positions = origins(reader, left, right)
                geometry_check(positions, left, right, kind == 'later-break'); geometry += 1
                if self_test:
                    bad = copy.deepcopy(positions); bad[1][1][1] = 10
                    try: geometry_check(bad, left, right, kind == 'later-break')
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted spanning-cell column swap')
                    if kind == 'right-span':
                        for delta in [0, 32]:
                            bad = copy.deepcopy(positions); bad[1][2][2] = bad[1][0][2] + delta
                            try: geometry_check(bad, left, right, False)
                            except AssertionError: rejected += 1
                            else: raise AssertionError('accepted overlapping/delayed following row')
    assert seen == {('LeftRight', k) for k in ['right-span', 'three-rows', 'later-break', 'simultaneous', 'header', 'caption', 'blank', 'caption-band-keep']} | {('左側右側', 'right-span')}, seen
    return count, total_pages, geometry, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path); parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, geometry, rejected = verify(args.directory, args.self_test)
    print(f'PASS: {count} PDFs / {pages} pages / {geometry} actual-matrix geometry checks / {rejected} alterations rejected')
