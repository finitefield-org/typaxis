#!/usr/bin/env python3
"""Check variable-width PDF text against source, original font metrics and masters."""
import argparse
import copy
import hashlib
import json
import struct
from pathlib import Path

from pypdf import PdfReader
from verify_book_v2_nested_table_pdf import paragraphs
from verify_book_v2_pdf_assembly import selected_master

U = 65536
HARANO_SHA = '66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717'


def original_metrics(path):
    data = path.read_bytes()
    assert hashlib.sha256(data).hexdigest() == HARANO_SHA
    u16 = lambda p: struct.unpack_from('>H', data, p)[0]
    u32 = lambda p: struct.unpack_from('>I', data, p)[0]
    tables = {}
    for i in range(u16(4)):
        p = 12 + 16 * i
        tables[data[p:p+4]] = u32(p+8)
    cmap = tables[b'cmap']
    offsets = []
    for i in range(u16(cmap+2)):
        p = cmap+4+8*i
        if u16(p) == 0 or (u16(p) == 3 and u16(p+2) in (1, 10)):
            offsets.append(cmap+u32(p+4))

    def glyph(scalar):
        code = ord(scalar)
        for p in offsets:
            if u16(p) == 12:
                for i in range(u32(p+12)):
                    q = p+16+12*i
                    if u32(q) <= code <= u32(q+4):
                        return u32(q+8)+code-u32(q)
            elif u16(p) == 4 and code <= 65535:
                count = u16(p+6)//2
                for i in range(count):
                    end = u16(p+14+2*i)
                    start = u16(p+16+2*count+2*i)
                    if start <= code <= end:
                        delta = u16(p+16+4*count+2*i)
                        at = p+16+6*count+2*i
                        offset = u16(at)
                        g = u16(at+offset+2*(code-start)) if offset else code
                        return (g+delta) & 65535 if g else 0
        raise AssertionError(f'missing original glyph: {scalar}')

    upem = u16(tables[b'head']+18)
    hhea = tables[b'hhea']
    ascent, descent = struct.unpack_from('>hh', data, hhea+4)
    count = u16(hhea+34)

    def advance(text):
        return sum(round(u16(tables[b'hmtx']+4*min(glyph(c), count-1))*12*U/upem) for c in text)

    return advance, round(ascent*12*U/upem), round(-descent*12*U/upem)


def check(actual, wire, reader, metrics):
    text = wire['text_buffers'][0]['utf8']
    tag, = [r['style_id'] for r in wire['style_sheet']['rules'] if r['style_id'].startswith('variable-width-')]
    mode = tag.removeprefix('variable-width-')
    japanese = text == '左側右側左側右側左側右側'
    assert japanese or text == 'R R R R R R'
    advance, ascent, descent = metrics if japanese else (
        lambda value: sum(round((300 if c == ' ' else 600)*12*U/1000) for c in value),
        round(800*12*U/1000), round(200*12*U/1000))
    baseline = ascent+(20*U-ascent-descent)//2
    digit = advance('1')
    assert len(actual) == len(reader.pages)
    sources = [''] * (3 if mode == 'references' else 1)
    starts, labels, note_text = [], [], ''
    paragraph = 0
    note_markers = body_markers = 0
    for index, entries in enumerate(actual):
        master = selected_master(wire['page_masters'], index)
        body = master['body']
        assert body['width'] == ([24, 48, 72][0 if index == 0 else 1 if index % 2 else 2])*U
        body_entries = [e for e in entries if e[2]*U < 100*U]
        note_entries = [e for e in entries if e[2]*U >= 100*U]
        if body_entries:
            assert paragraph < len(sources)
            if not sources[paragraph]: starts.append(index)
            part = body_entries[0][0]
            assert text.startswith(sources[paragraph]+part), (text, sources, part)
            sources[paragraph] += part
            if len(body_entries) == 2:
                assert sources[paragraph] == text
                label = body_entries[1][0]
                assert label.isascii() and label.isdecimal()
                if mode == 'notes':
                    assert label == '1'
                    body_markers += 1
                else:
                    assert mode == 'references'
                    labels.append(int(label))
            else:
                assert len(body_entries) == 1
            used = sum(advance(e[0]) for e in body_entries)
            inset = 3 if mode == 'references' else 0
            available = body['width']-(9 if mode == 'references' else 0)
            assert used <= available
            x = body['x']+inset+(available-used)//2
            for word, left, y in body_entries:
                assert abs(left*U-x) < 2 and abs(y*U-body['y']-baseline) < 2
                x += advance(word)
            if sources[paragraph] == text:
                if mode == 'references': assert len(body_entries) == 2
                paragraph += 1
        if note_entries:
            assert mode == 'notes'
            note = master['footnote']
            assert note['width'] == (72 if index == 0 else 36 if index % 2 else 60)*U
            if note_entries[0][0] == '1':
                assert note_text == '' and body_markers == 1
                note_markers += 1
                marker = note_entries.pop(0)
                assert abs(marker[1]*U-note['x']) < 2
                assert note_entries and abs(marker[2]-note_entries[0][2])*U < 2
            assert 1 <= len(note_entries) <= 2
            top = note['y']+note['height']-len(note_entries)*20*U
            available = note['width']-digit-12*U
            for row, (word, x, y) in enumerate(note_entries):
                assert text.startswith(note_text+word)
                note_text += word
                used = advance(word)
                assert used <= available
                left = note['x']+digit+12*U+(available-used)//2
                assert abs(x*U-left) < 2 and abs(y*U-top-row*20*U-baseline) < 2
    assert sources == [text]*len(sources)
    if mode == 'notes': assert note_text == text and body_markers == note_markers == 1
    if mode == 'references':
        assert labels == [starts[2]+1, starts[0]+1, starts[1]+1]
        for name, page in zip(['first', 'middle', 'last'], starts):
            assert reader.get_destination_page_number(reader.named_destinations[name]) == page


def verify(directory, font, self_test):
    metrics = original_metrics(font)
    drivers = {}
    for path in directory.glob('*.driver'):
        value = json.loads(path.read_text())
        drivers.setdefault(tuple(value['display']), []).append(value['assembly']['pdf'])
    seen = set()
    count = pages = rejected = 0
    for path in directory.glob('*.json'):
        value = json.loads(path.read_text())
        wire = value['wire']
        tags = [r['style_id'] for r in wire['style_sheet']['rules'] if r['style_id'].startswith('variable-width-')]
        if not tags: continue
        key = (wire['text_buffers'][0]['utf8'], tags[0])
        assert key not in seen
        seen.add(key)
        pdfs = [value['relations']['navigation']['assembly']['pdf'], *drivers[tuple(value['display'])]]
        assert len(pdfs) == 2
        for pdf in pdfs:
            reader = PdfReader(pdf, strict=True)
            actual = paragraphs(reader)
            check(copy.deepcopy(actual), wire, reader, metrics)
            count += 1
            pages += len(actual)
            if self_test:
                alterations = [actual[1:], actual+[[]]]
                for field, replacement in [(0, 'lost'), (1, 999), (2, 999)]:
                    altered = copy.deepcopy(actual)
                    altered[0][0][field] = replacement
                    alterations.append(altered)
                for page, entries in enumerate(actual):
                    if any(e[2] >= 100 for e in entries):
                        altered = copy.deepcopy(actual)
                        at = next(i for i, e in enumerate(entries) if e[2] >= 100 and e[0] != '1')
                        altered[page][at][1] += 1
                        alterations.append(altered)
                        break
                if tags == ['variable-width-references']:
                    altered = copy.deepcopy(actual)
                    page = next(i for i, entries in enumerate(actual) if len(entries) == 2)
                    altered[page][1][0] = '99'
                    alterations.append(altered)
                for altered in alterations:
                    try: check(altered, wire, reader, metrics)
                    except AssertionError: rejected += 1
                    else: raise AssertionError('accepted changed source/width/baseline/page count')
    assert seen == {(text, 'variable-width-'+mode) for text in ['R R R R R R', '左側右側左側右側左側右側'] for mode in ['body', 'notes', 'references']}
    return count, pages, rejected


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--harano-font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    count, pages, rejected = verify(args.directory, args.harano_font, args.self_test)
    print(f'PASS: {count} variable-width PDFs / {pages} pages / {rejected} alterations rejected; original source, font metrics, body/note widths, alignment and page labels')
