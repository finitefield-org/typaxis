#!/usr/bin/env python3
"""Check physical block/number alignment against declared varying page widths."""
import argparse
import copy
import json
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream
from verify_book_v2_font_objects import widths as cid_widths
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_pdf_assembly import selected_master
from verify_book_v2_variable_width_pdf import original_metrics

U = 65536


def placements(reader):
    images, formulas = [], []
    for page_index, page in enumerate(reader.pages):
        stack, group = [], None
        cm = tm = font = size = None
        fonts = page['/Resources']['/Font']
        for args, op in ContentStream(page.get_contents(), reader).operations:
            if op in (b'BDC', b'BMC'):
                stack.append(str(args[0]))
                if args[0] == '/Formula':
                    assert group is None
                    group = []
            elif op == b'EMC':
                if stack.pop() == '/Formula':
                    formulas.append((page_index, group))
                    group = None
            elif op == b'cm': cm = [float(v) for v in args]
            elif op == b'Tm': tm = [float(v) for v in args]
            elif op == b'Tf': font, size = args[0], float(args[1])
            elif op == b'Do': images.append([page_index, cm.copy()])
            elif op == b'Tj' and group is not None:
                if '/DescendantFonts' not in fonts[font]: continue
                raw = args[0].original_bytes
                descendant = fonts[font]['/DescendantFonts'][0].get_object()
                widths = cid_widths(descendant.get('/W', []))
                assert len(raw) == 2
                cid = int.from_bytes(raw, 'big')
                group.append([tm[4], tm[5], widths.get(cid, descendant.get('/DW', 1000))*size/1000])
        assert not stack
    return images, formulas


def check(images, formulas, text, wire, kind, notes, metrics, table_offset=0, table_fraction=False):
    harano = wire['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
    advance = metrics[0] if harano else lambda value: sum(round((300 if c == ' ' else 600)*12*U/1000) for c in value)
    digit = advance('1')

    def parent(page, in_note):
        master = selected_master(wire['page_masters'], page)
        region = master['footnote'] if in_note else master['body']
        left, width = region['x'], region['width']
        if in_note:
            left += digit+12*U
            width -= digit+12*U
        if table_fraction:
            # Authored 1:2 fractional columns, rounded to nearest raw unit;
            # denominator three has no half-way tie. The final column receives
            # the remaining width, independently of driver-observed geometry.
            first = (width+1)//3
            left += first
            width -= first
        indent, end = (100000, 200000) if kind == 'vector' else (4, 5)
        return left+indent+table_offset, width-indent-end-table_offset

    if kind == 'native':
        assert not images and len(formulas) == (3 if notes else 6)
        blocks = formulas[-1:] if notes else formulas[1::2]
        for page, glyphs in blocks:
            assert len(glyphs) == 3
            left, width = parent(page, notes)
            used = round((glyphs[-1][0]-glyphs[0][0])*U)+round(glyphs[-1][2]*U)
            assert 0 < used <= width
            wanted = left+(width-used)//2
            assert abs(glyphs[0][0]*U-wanted) < 3, (glyphs, wanted/U, used/U)
        return

    expected = (6 if notes else 12) if kind == 'vector' else (1 if notes else 3)
    assert len(images) == expected
    blocks = [(page, matrix) for index, (page, matrix) in enumerate(images)
              if kind != 'vector' or (index >= 4 if notes else index % 4 >= 2)]
    for page, matrix in blocks:
        left, width = parent(page, notes)
        if kind == 'vector':
            left += 2*U
            width -= 5*U
            assert width >= 30*U
            wanted = left+(width-30*U)//2
            assert matrix[:4] == [1, 0, 0, 1]
        else:
            wanted = left
            if kind == 'svg':
                assert abs(matrix[0]*U-34952) < 1 and matrix[:2] == [matrix[0], 0]
                assert matrix[2] == 0 and matrix[3] == matrix[0]
                assert width >= 1048560
            else:
                assert matrix[:4] == [32, 0, 0, -16]
                assert width >= 32*U
        assert abs(matrix[4]*U-wanted) < 2, (kind, notes, page, matrix, wanted/U)
    if kind == 'vector':
        numbers = [(page, x) for page, values in enumerate(text) for word, x, _ in values if word == '(1)']
        assert len(numbers) == (1 if notes else 3)
        for page, x in numbers:
            left, width = parent(page, notes)
            assert abs(x*U-(left+width-3*U-advance('(1)'))) < 2


def verify(directory, font, self_test, tables=False):
    metrics = original_metrics(font)
    drivers = {}
    for path in directory.glob('*.driver'):
        d = json.loads(path.read_text())
        drivers.setdefault(tuple(d['display']), []).append(d['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        tags = [r['style_id'] for r in wire['style_sheet']['rules'] if r['selector'] == 'paragraph' and r['style_id'].startswith('table-block-width-' if tables else 'block-width-')]
        if not tags: continue
        tag, = tags
        kind, mode = tag.removeprefix('table-block-width-' if tables else 'block-width-').split('-')
        notes = mode == 'notes'
        harano = wire['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
        if tables:
            blocks = wire['document']['footnotes'][0]['blocks'] if notes else wire['document']['blocks']
            for table in (b for b in blocks if b['kind'] == 'table'):
                assert table['columns'] == [{'kind':'fixed','width':20*U},{'kind':'fraction','weight':1},{'kind':'fraction','weight':2}]
                cell = table['body'][0]['cells'][1]
                assert cell['colspan'] == 2
                nested = cell['blocks'][0]
                assert nested['columns'] == [{'kind':'fixed','width':20*U},{'kind':'fraction','weight':1}]
        key = (harano, kind, notes)
        assert key not in seen
        seen.add(key)
        pdfs = [probe['relations']['navigation']['assembly']['pdf'], *drivers[tuple(probe['display'])]]
        assert len(pdfs) == 2
        for pdf in pdfs:
            reader = PdfReader(pdf, strict=True)
            images, formulas = placements(reader)
            text = paragraphs(reader)
            check(images, formulas, text, wire, kind, notes, metrics, 40*U if tables else 0)
            count += 1
            pages += len(reader.pages)
            if self_test:
                changed = copy.deepcopy(formulas if kind == 'native' else images)
                if kind == 'native': changed[-1][1][0][0] += 1
                else: changed[-1][1][4] += 1
                bad_images, bad_formulas = (images, changed) if kind == 'native' else (changed, formulas)
                try: check(bad_images, bad_formulas, text, wire, kind, notes, metrics, 40*U if tables else 0)
                except AssertionError: rejected += 1
                else: raise AssertionError('accepted displaced block')
                changed = (formulas if kind == 'native' else images)[:-1]
                bad_images, bad_formulas = (images, changed) if kind == 'native' else (changed, formulas)
                try: check(bad_images, bad_formulas, text, wire, kind, notes, metrics, 40*U if tables else 0)
                except AssertionError: rejected += 1
                else: raise AssertionError('accepted missing block')
    assert seen == {(font, kind, notes) for font in [False, True] for kind in ['vector', 'native', 'png', 'jpeg', 'svg'] for notes in [False, True] if not (font and kind == 'native')}
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--harano-font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--tables', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.harano_font, args.self_test, args.tables)
    print(f'PASS: {count} block-width PDFs / {pages} pages / {rejected} alterations rejected; vector/native/raster alignment, fixed sizes and equation numbers')
