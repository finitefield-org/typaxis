#!/usr/bin/env python3
"""Compare real Harano table-cell PDF geometry to the unchanged left-aligned fixture.

The two saved VMB packages must differ only in cell classes and their style
rules. Inspect actual glyph matrices and embedded widths, including math's
accessibility anchor box, to verify the common line is moved intact.
"""
import argparse
import copy
import json
import re
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_table_alignment_pdf import character_width


def glyphs(reader):
    assert len(reader.pages) == 1
    page = reader.pages[0]
    result = []
    font = matrix = size = None
    for args, op in ContentStream(page.get_contents(), reader).operations:
        if op == b'Tf':
            font = page['/Resources']['/Font'][args[0]].get_object()
            size = float(args[1])
        elif op == b'Tm':
            matrix = [float(v) for v in args]
        elif op == b'Tj':
            raw = args[0].original_bytes if hasattr(args[0], 'original_bytes') else bytes(args[0])
            pairs = re.findall(rb'<([0-9A-F]+)> <([0-9A-F]+)>', font['/ToUnicode'].get_data().split(b'beginbfchar', 1)[1])
            cmap = {int(a, 16): bytes.fromhex(b.decode()).decode('utf-16-be') for a, b in pairs}
            char = cmap[int.from_bytes(raw, 'big')]
            if font['/Subtype'] == '/Type3':
                assert char == '￼' and size == 1
                width = matrix[0]
            else:
                assert matrix[:4] == [1, 0, 0, -1]
                width = character_width(font, char, size)
            result.append((char, matrix[4], matrix[5], width))
    return result


def check(before, after, left, widths):
    assert len(before) == len(after)
    assert [(g[0], g[2], g[3]) for g in before] == [(g[0], g[2], g[3]) for g in after]
    # Caption and preceding heading/reference remain at their source positions.
    header = next(i for i, g in enumerate(before) if g[0] == '見')
    assert before[:header] == after[:header]
    rows = {}
    for index, g in enumerate(before[header:], header):
        column = 0 if g[1] < left + widths[0] - 1/65536 else 1
        rows.setdefault((g[2], column), []).append(index)
    assert len(rows) == 4
    for (y, column), indices in rows.items():
        first, last = before[indices[0]], before[indices[-1]]
        extent = last[1] + last[3] - first[1]
        offset = (widths[column] - extent) / (1 if column == 0 else 2)
        assert offset > 0
        for i in indices:
            assert abs(after[i][1] - before[i][1] - offset) < 0.001, (y, column, i)
    return len(rows)


def verify(before_job, after_job, before_pdf, after_pdf, self_test):
    old = json.loads((before_job/'document-package.json').read_text())
    new = json.loads((after_job/'document-package.json').read_text())
    stripped = copy.deepcopy(new)
    table = next(b for b in stripped['document']['blocks'] if b['kind']=='table')
    cells = []
    for section in ['head','body']:
        for row in table[section]:
            for col, cell in enumerate(row['cells']):
                assert cell.pop('classes') == ['vmb-table-cell-' + ['end','center'][col]]
                cells.append(cell['node_id'])
    stripped['style_sheet']['rules'] = [r for r in stripped['style_sheet']['rules'] if not r['selector'].startswith('table_cell.')]
    assert stripped == old
    records = json.loads((after_job/'typaxis-source-map.json').read_text())['table_cell_styles']
    assert [r['node_id'] for r in records] == cells
    master = new['page_masters']['masters'][0]['body']
    left = master['x']/65536
    fixed = table['columns'][0]['width']/65536
    widths = [fixed, master['width']/65536-fixed]
    before, after = glyphs(PdfReader(before_pdf, strict=True)), glyphs(PdfReader(after_pdf, strict=True))
    count = check(before, after, left, widths)
    rejected = 0
    if self_test:
        for index in [0, next(i for i,g in enumerate(after) if g[0]=='見'), len(after)-1]:
            bad = after.copy()
            char,x,y,width = bad[index]
            bad[index] = char,x+1,y,width
            try:
                check(before,bad,left,widths)
            except AssertionError:
                rejected += 1
            else:
                raise AssertionError('accepted displaced source text')
    return count,rejected


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    for arg in ['before_job','after_job','before_pdf','after_pdf']:
        parser.add_argument(arg,type=Path)
    parser.add_argument('--self-test',action='store_true')
    args=parser.parse_args()
    count,rejected=verify(args.before_job,args.after_job,args.before_pdf,args.after_pdf,args.self_test)
    print(f'PASS: {count} original Harano header/body cells aligned; captions unchanged; {rejected} displacements rejected')
