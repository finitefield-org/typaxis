#!/usr/bin/env python3
"""Check unequal natural table-cell continuation against declared 17/23 pt lines."""
import argparse
import copy
import json
from pathlib import Path
from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_table_header_break_pdf import glyph_pages
from verify_book_v2_definition_table_pdf import check_positions

U = 65536


def expected(mode):
    ascent = round(800 * 12 * U / 1000)
    descent = round(200 * 12 * U / 1000)
    baseline = lambda height: ascent + (height * U - ascent - descent) // 2
    right_x = 55 if mode == 'nested' else 100
    def body(left_count, right_count, top=0):
        return ([('Left', 10*U, (10+top+i*17)*U+baseline(17)) for i in range(left_count)]
                + [('Right', right_x*U, (10+top+i*23)*U+baseline(23)) for i in range(right_count)])
    repeats = []
    if mode == 'header':
        header = [('Left',10*U,10*U+baseline(17)),('Right',100*U,10*U+baseline(17))]
        source = [header+body(1,1,17)] + [body(1,1,17) for _ in range(3)]
        repeats = [[]] + [[('L',10*U,10*U+baseline(17)),('R',100*U,10*U+baseline(17))] for _ in range(3)]
    elif mode == 'caption':
        source = [[('Left',10*U,10*U+baseline(17))]+body(1,1,17),body(2,2),body(1,1)]
    else:
        source = [body(2,2),body(2,2)]
        if mode == 'span':
            # Source-order deficit allocation puts the first spanning cell's
            # 4*17 pt in row 2 before the next cell puts 4*23 pt in row 1.
            # Row 2 therefore retains 68 pt: its 23 pt text is followed by
            # 45 pt of authored-table band geometry across two physical pages.
            source.extend([body(0,1), []])
        if mode == 'nested': source[0].append(('LeftRight',100*U,10*U+baseline(16)))
    return source, repeats or [[] for _ in source]


def verify(directory, self_test):
    drivers = {}
    for path in directory.glob('*.driver'):
        value = json.loads(path.read_text())
        drivers.setdefault(tuple(value['display']), []).append(value['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        value = json.loads(path.read_text())
        wire = value['wire']
        if not any(r['style_id'] == 'unequal' for r in wire['style_sheet']['rules']): continue
        assert wire['text_buffers'][0]['utf8'] == 'LeftRight'
        assert wire['page_masters']['masters'][0]['body'] == {'x':10*U,'y':10*U,'width':180*U,'height':48*U}
        block = wire['document']['blocks'][0]
        mode = ('nested' if block['body'][0]['cells'][0]['blocks'][0]['kind']=='table'
                else 'header' if block['head'] else 'caption' if block.get('caption')
                else 'span' if len(block['body'])==2 else 'flat')
        assert mode not in seen
        seen.add(mode)
        wanted, headers = expected(mode)
        matched = drivers[tuple(value['display'])]
        assert len(matched) == 1
        for pdf in [value['relations']['navigation']['assembly']['pdf'], *matched]:
            reader = PdfReader(pdf, strict=True)
            actual = paragraphs(reader)
            check_positions(actual, wanted)
            repeated = [[[c,x,y] for c,x,y,repeat in page if repeat]
                        for page in glyph_pages(reader,'Left','Right',allow_layout=True)]
            check_positions(repeated, headers)
            assert not any(page.get('/Annots') for page in reader.pages)
            count += 1
            pages += len(reader.pages)
            if self_test:
                alterations = [actual[1:], actual+[[]]]
                for field, replacement in [(0,'lost'),(1,999),(2,999)]:
                    changed = copy.deepcopy(actual)
                    changed[0][0][field] = replacement
                    alterations.append(changed)
                changed = copy.deepcopy(actual)
                changed[1].insert(0, changed[0][0])
                alterations.append(changed)
                for changed in alterations:
                    try: check_positions(changed, wanted)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted altered natural-cell source/geometry')
                if mode == 'header':
                    try: check_positions([[] for _ in headers], headers)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted missing repeated headers')
    assert seen == {'flat','nested','header','caption','span'}, seen
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path)
    parser.add_argument('--self-test',action='store_true')
    args = parser.parse_args()
    count,pages,rejected = verify(args.directory,args.self_test)
    print(f'PASS: {count} natural-cell PDFs / {pages} pages / {rejected} alterations rejected; declared line metrics, source once, columns and header repetitions')
