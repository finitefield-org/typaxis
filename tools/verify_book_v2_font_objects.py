#!/usr/bin/env python3
"""Independently parse test-only book-2 font object probes with pypdf.

These probes contain an empty, untagged page. They prove font-object bytes and
references only; they are not full-book, page-layout or PDF/UA evidence.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import tempfile
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import IndirectObject


def widths(value):
    result = {}
    cursor = 0
    while cursor < len(value):
        first = int(value[cursor])
        following = value[cursor + 1]
        if isinstance(following, list):
            for offset, width in enumerate(following):
                assert first + offset not in result
                result[first + offset] = int(width)
            cursor += 2
        else:
            last = int(following)
            assert first <= last
            for cid in range(first, last + 1):
                assert cid not in result
                result[cid] = int(value[cursor + 2])
            cursor += 3
    return result


def verify(path: Path):
    expected = json.loads(path.with_suffix('.json').read_text())
    pdf = PdfReader(path, strict=True)
    assert len(pdf.pages) == 1
    assert pdf.trailer['/Size'] == 4 + 6 * len(expected)
    counts = [0, 0]
    for font in expected:
        first = font['id']
        ref = lambda number: IndirectObject(number, 0, pdf)
        top = ref(first).get_object()
        assert top['/Type'] == '/Font'
        assert top['/Subtype'] == '/Type0'
        assert top['/BaseFont'] == '/' + font['name']
        assert top['/Encoding'] == '/Identity-H'
        descendants = top['/DescendantFonts']
        assert len(descendants) == 1 and descendants[0].idnum == first + 1
        assert top.raw_get('/ToUnicode').idnum == first + 4
        cid = descendants[0].get_object()
        cff = font['cff']
        counts[int(cff)] += 1
        assert cid['/Subtype'] == ('/CIDFontType0' if cff else '/CIDFontType2')
        assert cid['/Type'] == '/Font'
        assert cid['/BaseFont'] == top['/BaseFont']
        info = cid['/CIDSystemInfo']
        assert info['/Registry'] == 'Adobe'
        assert info['/Ordering'] == 'Identity'
        assert info['/Supplement'] == 0
        assert cid['/DW'] == 1000
        expected_widths = {int(c): int(w) for c, w in font['widths']}
        if cff:
            expected_widths[0] = font['notdef_width']
        assert widths(cid['/W']) == expected_widths
        assert cid.raw_get('/FontDescriptor').idnum == first + 2
        descriptor = cid['/FontDescriptor']
        assert descriptor['/Type'] == '/FontDescriptor'
        assert descriptor['/FontName'] == top['/BaseFont']
        assert list(descriptor['/FontBBox']) == font['bbox']
        for key, expected_key in [('Ascent', 'ascent'), ('Descent', 'descent'), ('Flags', 'flags')]:
            assert descriptor['/' + key] == font[expected_key]
        program_key = '/FontFile3' if cff else '/FontFile2'
        assert descriptor.raw_get(program_key).idnum == first + 3
        assert ('/FontFile2' if cff else '/FontFile3') not in descriptor
        program = descriptor[program_key]
        data = program.get_data()
        assert hashlib.sha256(data).hexdigest() == font['program_sha256']
        if cff:
            assert program['/Subtype'] == '/OpenType'
            assert data.startswith(b'OTTO')
            assert '/Length1' not in program
            assert '/CIDToGIDMap' not in cid
            assert descriptor.raw_get('/CIDSet').idnum == first + 5
            auxiliary = descriptor['/CIDSet']
        else:
            assert program['/Length1'] == len(data)
            assert data.startswith(b'\0\1\0\0')
            assert '/Subtype' not in program
            assert '/CIDSet' not in descriptor
            assert cid.raw_get('/CIDToGIDMap').idnum == first + 5
            auxiliary = cid['/CIDToGIDMap']
        assert auxiliary.get_data() == bytes(font['auxiliary'])
        assert top['/ToUnicode'].get_data() == bytes(font['to_unicode'])
    return counts


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.pdf'))
    assert paths, 'no font probes'
    counts = [0, 0]
    for path in paths:
        result = verify(path)
        counts = [a + b for a, b in zip(counts, result)]
    assert all(counts), 'both original-format font kinds must be exercised'
    rejected = 0
    if args.self_test:
        for cff in (False, True):
            path = next(p for p in paths if any(f['cff'] == cff for f in json.loads(p.with_suffix('.json').read_text())))
            expected = json.loads(path.with_suffix('.json').read_text())
            font = next(f for f in expected if f['cff'] == cff)
            data = path.read_bytes()
            source = PdfReader(path, strict=True)
            start = source.xref[0][font['id'] + 3]
            payload = data.index(b'\nstream\n', start) + 8
            corrupted = bytearray(data)
            corrupted[payload + 20] ^= 1
            marker = b'/Subtype /CIDFontType0' if cff else b'/Subtype /CIDFontType2'
            wrong_kind = data.replace(marker, marker[:-1] + (b'2' if cff else b'0'), 1)
            assert wrong_kind != data
            for changed in (corrupted, wrong_kind):
                with tempfile.TemporaryDirectory(prefix='typaxis-font-object-negative-') as directory:
                    probe = Path(directory) / 'negative.pdf'
                    probe.write_bytes(changed)
                    probe.with_suffix('.json').write_text(json.dumps(expected))
                    try:
                        verify(probe)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError('tampered font object was accepted')
        assert rejected == 4
    print(json.dumps({'font_object_probes': len(paths), 'truetype_fonts': counts[0], 'cff_fonts': counts[1], 'tamper_rejections': rejected}))


if __name__ == '__main__':
    main()
