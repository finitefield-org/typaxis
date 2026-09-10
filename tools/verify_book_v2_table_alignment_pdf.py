#!/usr/bin/env python3
"""Independently verify the three-column alignment fixture in emitted PDF bytes.

Read actual text matrices and embedded CID widths, not the renderer's expected
positions. Check unequal measured columns, caption isolation and repeated header
rows. This fixture oracle is separate from full-book and PDF/UA acceptance.
"""
import argparse
import copy
import json
import re
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream


def character_width(font, character, size):
    cmap = font['/ToUnicode'].get_data()
    pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>', cmap.split(b'beginbfchar', 1)[1])
    codes = {bytes.fromhex(b.decode()).decode('utf-16-be'): int(a, 16) for a, b in pairs}
    cid = codes[character]
    descendant = font['/DescendantFonts'][0].get_object()
    widths = descendant['/W']
    i = 0
    while i < len(widths):
        start, value = widths[i:i+2]
        if isinstance(value, list):
            if start <= cid < start + len(value):
                return float(value[cid-start]) * size / 1000
            i += 2
        else:
            if start <= cid <= value:
                return float(widths[i+2]) * size / 1000
            i += 3
    return float(descendant.get('/DW', 1000)) * size / 1000


def words(reader):
    pages = []
    for page in reader.pages:
        chars = []
        font = size = matrix = None
        for operands, op in ContentStream(page.get_contents(), reader).operations:
            if op == b'Tf':
                font = page['/Resources']['/Font'][operands[0]].get_object()
                size = float(operands[1])
            elif op == b'Tm':
                matrix = [float(v) for v in operands]
            elif op == b'Tj':
                assert font is not None and matrix is not None
                assert matrix[:4] == [1, 0, 0, -1]
                pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>', font['/ToUnicode'].get_data().split(b'beginbfchar', 1)[1])
                cmap = {int(a, 16): bytes.fromhex(b.decode()).decode('utf-16-be') for a, b in pairs}
                encoded = operands[0].original_bytes if hasattr(operands[0], 'original_bytes') else bytes(operands[0])
                assert len(encoded) == 2
                text = cmap[int.from_bytes(encoded, 'big')]
                assert len(text) == 1 and text in 'Result'
                chars.append((text, matrix[4], matrix[5], character_width(font, text, size)))
            elif op == b'TJ':
                raise AssertionError('fixture oracle requires explicit glyph matrices')
        assert len(chars) % 6 == 0
        groups = []
        for i in range(0, len(chars), 6):
            group = chars[i:i+6]
            assert ''.join(c[0] for c in group) == 'Result'
            assert len({c[2] for c in group}) == 1
            # Six advances round individually in the fixed-point renderer.
            for a, b in zip(group, group[1:]):
                assert abs(a[1] + a[3] - b[1]) <= 1 / 65536
            groups.append((group[0][1], group[0][2], sum(c[3] for c in group)))
        pages.append(groups)
    return pages


def check(pages, aligned):
    assert [len(p) for p in pages] == [10, 9]
    caption = pages[0][0]
    assert abs(caption[0] - 10) <= 1 / 65536
    for page_no, page in enumerate(pages):
        rows = page[1:] if page_no == 0 else page
        assert len(rows) == 9
        for i, (x, y, width) in enumerate(rows):
            column = i % 3
            start = [10, 70, 130][column]
            available = [60, 60, 120][column]
            offset = 0
            if aligned and column == 1:
                offset = available - width
            elif aligned and column == 2:
                offset = (available - width) / 2
            assert abs(x - start - offset) <= 8 / 65536, (page_no, i, x, start + offset)
            assert abs(y - rows[i-column][1]) <= 1 / 65536
    return True


def verify(directory, self_test=False):
    drivers = {}
    for path in directory.glob('*.driver'):
        driver = json.loads(path.read_text())
        drivers.setdefault(tuple(driver['display']), []).append(driver['assembly']['pdf'])
    seen = set()
    count = rejected = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        blocks = probe['wire']['document']['blocks']
        if len(blocks) != 1 or blocks[0].get('kind') != 'table' or len(blocks[0]['columns']) != 3:
            continue
        table = blocks[0]
        classes = [cell.get('classes') for cell in table['head'][0]['cells']]
        if classes not in [[['start'], ['start'], ['start']], [['start'], ['end'], ['center']]]:
            continue
        aligned = classes[1] == ['end']
        assert aligned not in seen
        seen.add(aligned)
        actual = drivers[tuple(probe['display'])]
        assert len(actual) == 1
        for pdf in [probe['relations']['navigation']['assembly']['pdf'], *actual]:
            pages = words(PdfReader(pdf, strict=True))
            check(pages, aligned)
            count += 1
            if self_test:
                for page, index in [(0, 0), (0, 2), (1, 1), (1, 2)]:
                    bad = copy.deepcopy(pages)
                    x, y, width = bad[page][index]
                    bad[page][index] = (x + 1, y, width)
                    try:
                        check(bad, aligned)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('accepted displaced caption/cell/header')
    assert seen == {False, True}, 'missing alignment fixture'
    return count, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, rejected = verify(args.directory, args.self_test)
    print(f'PASS: {count} PDFs; exact cell alignment, caption and repeated headers; {rejected} displaced records rejected')
