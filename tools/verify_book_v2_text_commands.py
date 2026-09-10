#!/usr/bin/env python3
"""Independently parse test-only book-2 glyph/rule PDF command probes.

The probe envelope puts commands on one test page and omits images and semantic
structure. This verifies physical operators, not whole-book page assembly/UA.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import tempfile
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ContentStream


def raw_bytes(value):
    return value.original_bytes if hasattr(value, 'original_bytes') else bytes(value)


def verify(path):
    expected = json.loads(path.with_suffix('.json').read_text())
    pdf = PdfReader(path, strict=True)
    assert len(pdf.pages) == 1
    page = pdf.pages[0]
    operations = iter(ContentStream(page['/Contents'], pdf).operations)

    def take(operator):
        operands, actual = next(operations)
        assert actual == operator, (actual, operator)
        return operands

    assert take(b'q') == []
    assert list(take(b'cm')) == [1, 0, 0, -1, 0, 792]
    glyphs = rules = replacements = 0
    for command in expected:
        actual = command.get('actual_text')
        if actual is not None:
            operands = take(b'BDC')
            assert operands[0] == '/Span'
            encoded = bytes(actual).decode('ascii')
            assert encoded.startswith('<FEFF') and encoded.endswith('>')
            decoded = bytes.fromhex(encoded[5:-1]).decode('utf-16-be')
            assert operands[1]['/ActualText'] == decoded == command['source_text']
            replacements += 1
        assert list(take(b'g')) == [0]
        if 'rect' in command:
            assert [float(v) for v in take(b're')] == [v / 65536 for v in command['rect']]
            assert take(b'f') == []
            rules += 1
        else:
            assert take(b'BT') == []
            name, size = take(b'Tf')
            assert name == '/PB' + str(command['font'])
            assert page['/Resources']['/Font'].raw_get(name).idnum == command['font_object']
            assert float(size) == command['size'] / 65536
            assert list(take(b'Tr')) == [0]
            for operator, value in [(b'Tc', 0), (b'Tw', 0), (b'Tz', 100), (b'TL', 0), (b'Ts', 0)]:
                assert list(take(operator)) == [value]
            assert len(command['positions']) == len(command['cids'])
            for (x, y), cid in zip(command['positions'], command['cids']):
                matrix = take(b'Tm')
                assert list(matrix[:4]) == [1, 0, 0, -1]
                assert [float(v) for v in matrix[4:]] == [x / 65536, y / 65536]
                shown = take(b'Tj')
                assert len(shown) == 1 and raw_bytes(shown[0]) == cid.to_bytes(2, 'big')
                glyphs += 1
            assert take(b'ET') == []
        if actual is not None:
            assert take(b'EMC') == []
    assert take(b'Q') == []
    assert next(operations, None) is None
    return glyphs, rules, replacements


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--pdftotext', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.pdf'))
    assert paths, 'no command probes'
    totals = [0, 0, 0]
    for path in paths:
        totals = [a + b for a, b in zip(totals, verify(path))]
    assert all(totals), 'glyphs, rules and occurrence ActualText must all be exercised'
    extraction = 0
    if args.pdftotext:
        for path in paths:
            expected = json.loads(path.with_suffix('.json').read_text())
            source = ''.join(c.get('source_text', '') for c in expected)
            if source == 'ABA fi X D E f i':
                output = subprocess.run([str(args.pdftotext), '-raw', str(path), '-'], check=True, capture_output=True).stdout.decode('utf-8')
                assert output.rstrip('\n\f') == source, repr(output)
                extraction += 1
        assert extraction > 0, 'real GSUB ambiguity/ligature/multiple-substitution probe missing'
    rejected = 0
    if args.self_test:
        path = next(p for p in paths if any('positions' in c for c in json.loads(p.with_suffix('.json').read_text())))
        data = path.read_bytes()
        for original, changed in [(b'1 0 0 -1 ', b'2 0 0 -1 '), (b'0 Tc 0 Tw 100 Tz', b'0 Tc 0 Tw 200 Tz')]:
            # Same-size edits preserve xref and stream-length validity, so the
            # verifier must detect the actual operator-value change.
            content_start = data.index(b' cm\n') + 4
            offset = data.index(original, content_start)
            mutated = data[:offset] + changed + data[offset + len(original):]
            assert mutated != data
            with tempfile.TemporaryDirectory(prefix='typaxis-text-command-negative-') as directory:
                probe = Path(directory) / 'negative.pdf'
                probe.write_bytes(mutated)
                probe.with_suffix('.json').write_bytes(path.with_suffix('.json').read_bytes())
                try:
                    verify(probe)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError('changed physical text operator accepted')
        assert rejected == 2
    print(json.dumps({'probes': len(paths), 'glyphs': totals[0], 'rules': totals[1], 'actual_text_scopes': totals[2], 'gsub_extraction_probes': extraction, 'tamper_rejections': rejected}))


if __name__ == '__main__':
    main()
