#!/usr/bin/env python3
"""Independently extract blank/caption/header pages around authored caption breaks.

The controlled fixtures use three body rows or a single empty cell. They prove
source-break pagination, not original Japanese typography or full-book success.
"""
import argparse
import json
from pathlib import Path
from pypdf import PdfReader


def check(texts, expected, label):
    assert len(texts) == len(expected), (len(texts), len(expected))
    for text, count in zip(texts, expected):
        assert ''.join(text.split()) == label * count, (text, count)


def verify(directory, self_test, harano=False):
    drivers = {}
    for path in directory.glob('*.driver'):
        driver = json.loads(path.read_text())
        drivers.setdefault(tuple(driver['display']), []).append(driver['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        label = probe['wire']['text_buffers'][0]['utf8']
        if label not in ('Result', '表の説明') or label == '表の説明' and not harano:
            continue
        doc = probe['wire']['document']
        if doc['footnotes'] or len(doc['blocks']) != 1:
            continue
        table = doc['blocks'][0]
        caption = table.get('caption', [])
        if table['kind'] != 'table' or not any(b['kind'] == 'page_break' for b in caption):
            continue
        kinds = [b['kind'] for b in caption]
        headers = bool(table['head'])
        if kinds == ['page_break'] * 2:
            assert not headers and len(table['body']) == 1
            assert table['body'][0]['cells'][0]['blocks'] == []
            expected = [0, 0, 0]
            key = 'empty'
        else:
            assert kinds == ['page_break', 'paragraph', 'page_break', 'page_break', 'paragraph', 'page_break']
            assert len(table['body']) == 3
            # The original Japanese font has taller metrics than the minimal font.
            rows = [2, 2, 2] if label == '表の説明' else [3, 2]
            expected = [0, 1, 0, 1] + (rows if headers else [3])
            key = 'headers' if headers else 'body'
        key = (label, key)
        assert key not in seen
        seen.add(key)
        actual = drivers[tuple(probe['display'])]
        assert len(actual) == 1
        for path in [probe['relations']['navigation']['assembly']['pdf'], *actual]:
            reader = PdfReader(path, strict=True)
            texts = [page.extract_text() for page in reader.pages]
            check(texts, expected, label)
            count += 1
            pages += len(texts)
            if self_test:
                for bad in [texts[1:], texts + [''], [label] + texts[1:]]:
                    try:
                        check(bad, expected, label)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('accepted missing/extra/nonblank forced page')
    wanted = {('Result', key) for key in ['empty', 'headers', 'body']}
    if harano:
        wanted.add(('表の説明', 'headers'))
    assert seen == wanted, seen
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--harano', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.self_test, args.harano)
    print(f'PASS: {count} PDFs / {pages} pages; exact forced blank/caption/header text; {rejected} page alterations rejected')
