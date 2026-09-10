#!/usr/bin/env python3
"""Independently check blocks continued in a source table's fractional columns."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_block_width_pdf import placements, check
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_variable_width_pdf import original_metrics


def verify(directory, font, self_test):
    metrics = original_metrics(font)
    drivers = {}
    for path in directory.glob('*.driver'):
        probe = json.loads(path.read_text())
        drivers.setdefault(tuple(probe['display']), []).append(probe['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        tags = [r['style_id'] for r in wire['style_sheet']['rules'] if r['selector'] == 'paragraph' and r['style_id'].startswith('table-block-source-')]
        if not tags:
            continue
        tag, = tags
        kind, mode = tag.removeprefix('table-block-source-').split('-')
        notes = mode == 'notes'
        harano = wire['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
        source = wire['document']['footnotes'][0]['blocks'] if notes else wire['document']['blocks']
        assert len(source) == 1
        table, = source
        assert table['kind'] == 'table'
        assert table['columns'] == [{'kind': 'fraction', 'weight': 1}, {'kind': 'fraction', 'weight': 2}]
        assert table['head'] == [] and len(table['body']) == 1
        assert [p['kind'] for p in table['caption']] == ['paragraph', 'page_break', 'paragraph']
        cells = table['body'][0]['cells']
        assert len(cells) == 2 and cells[0]['blocks'] == []
        assert all(c['colspan'] == c['rowspan'] == 1 for c in cells)
        assert len(cells[1]['blocks']) == (1 if notes else 5)
        key = (harano, kind, notes)
        assert key not in seen
        seen.add(key)
        pdfs = [probe['relations']['navigation']['assembly']['pdf'], *drivers[tuple(probe['display'])]]
        assert len(pdfs) == 2
        for pdf in pdfs:
            reader = PdfReader(pdf, strict=True)
            assert len(reader.pages) == (2 if notes else 4)
            images, formulas = placements(reader)
            text = paragraphs(reader)
            check(images, formulas, text, wire, kind, notes, metrics, table_fraction=True)
            count += 1
            pages += len(reader.pages)
            if self_test:
                shifted = copy.deepcopy(formulas if kind == 'native' else images)
                if kind == 'native':
                    shifted[-1][1][0][0] += 0.25
                else:
                    shifted[-1][1][4] += 0.25
                original = formulas if kind == 'native' else images
                for changed in [shifted, original[:-1], original+[original[-1]]]:
                    bad_images, bad_formulas = (images, changed) if kind == 'native' else (changed, formulas)
                    try:
                        check(bad_images, bad_formulas, text, wire, kind, notes, metrics, table_fraction=True)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('altered block continuation accepted')
    expected = {(font, kind, notes) for font in [False, True] for kind in ['vector', 'native', 'png', 'jpeg', 'svg'] for notes in [False, True] if not (font and kind == 'native')}
    assert seen == expected
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--harano-font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.harano_font, args.self_test)
    print(f'PASS: {count} table block source PDFs / {pages} pages / {rejected} alterations rejected; source fractional columns, vector/native/raster alignment, fixed sizes and equation numbers')
