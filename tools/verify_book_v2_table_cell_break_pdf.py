#!/usr/bin/env python3
"""Independently extract controlled parallel-cell break PDFs, including Harano.

This complements the source/object/geometry verifier with exact physical page
text. Dedicated oracles cover rowspan/header breaks; full-book output is separate.
"""
import argparse
import json
from pathlib import Path
from pypdf import PdfReader


def check(texts, expected):
    assert [''.join(text.split()) for text in texts] == expected, (texts, expected)


def verify(directory, self_test):
    drivers = {}
    for path in directory.glob('*.driver'):
        driver = json.loads(path.read_text())
        drivers.setdefault(tuple(driver['display']), []).append(driver['assembly']['pdf'])
    count = pages = rejected = 0
    seen = set()
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        text = wire['text_buffers'][0]['utf8']
        if text not in ('LeftRight', '左側右側') or wire['document']['footnotes']:
            continue
        table, = wire['document']['blocks']
        if table['kind'] != 'table' or len(table['body']) not in (1, 2):
            continue
        if any(cell['rowspan'] != 1 for row in table['body'] for cell in row['cells']):
            continue
        left, right = table['body'][0]['cells']
        if not any(b['kind'] == 'page_break' for b in left['blocks']):
            continue
        l, r = ('Left', 'Right') if text == 'LeftRight' else ('左側', '右側')
        expected = [l, l + r, r]
        kind = 'asymmetric'
        if len(table['body']) == 2:
            colspan = len(table['body'][1]['cells']) == 1
            expected = [l, l + r, r + l] + ([] if colspan else [r])
            kind = 'colspan' if colspan else 'following-row'
        elif not right['blocks']:
            expected, kind = ['', '', ''], 'empty'
        elif any(b['kind'] == 'page_break' for b in right['blocks']):
            expected, kind = [l + r, l + r], 'simultaneous'
        elif table.get('caption'):
            if any(b['kind'] == 'page_break' for b in table['caption']):
                expected, kind = ['', text, '', text] + expected, 'caption-breaks'
            elif len(table['caption']) == 3:
                expected, kind = [text * 2, text + l, l + r, r], 'caption-keep-long'
            else:
                kind = 'caption-keep' if table['caption'][0]['classes'] else 'caption-prefix'
                expected = [text + l, l + r, r]
        elif table['head']:
            expected, kind = [l + r + value for value in expected], 'headers'
        key = text, kind
        assert key not in seen, key
        seen.add(key)
        actual = drivers[tuple(probe['display'])]
        assert len(actual) == 1
        for pdf in [probe['relations']['navigation']['assembly']['pdf'], *actual]:
            texts = [page.extract_text() for page in PdfReader(pdf, strict=True).pages]
            check(texts, expected)
            count += 1
            pages += len(texts)
            if self_test:
                for changed in [texts[1:], texts + [''], [r] + texts[1:], texts[:-1] + [texts[-1] + l]]:
                    try:
                        check(changed, expected)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('accepted missing/extra/misplaced/duplicate cell text')
    assert seen == {('LeftRight', kind) for kind in ['asymmetric', 'empty', 'simultaneous', 'caption-breaks', 'caption-prefix', 'caption-keep', 'caption-keep-long', 'headers', 'colspan', 'following-row']} | {('左側右側', 'asymmetric')}, seen
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.self_test)
    print(f'PASS: {count} PDFs / {pages} pages; exact parallel-cell page text; {rejected} alterations rejected')
