#!/usr/bin/env python3
"""Inspect actual marked body streams and extraction objects in test pages.

The envelope supplies a fixed inspection MediaBox and no StructTreeRoot. This
checks the actual selected-page body, not final book publication or PDF/UA.
"""
import argparse
import io
import json
import re
import subprocess
import tempfile
from pathlib import Path
from pypdf import PdfReader, PdfWriter
from pypdf.generic import ContentStream, DecodedStreamObject


def ops(data):
    stream = DecodedStreamObject()
    stream.set_data(data.encode() if isinstance(data, str) else data)
    return ContentStream(stream, None).operations


def verify(reader, expected):
    assert len(reader.pages) == expected['pages']
    counts = dict(pages=len(reader.pages), groups=0, glyphs=0, rules=0,
                  image_placements=0, anchors=0, replacements=0, artifacts=0)
    cursor = 0
    for page_index, page in enumerate(reader.pages):
        operations = iter(ContentStream(page['/Contents'], reader).operations)
        def take(op, wanted=None):
            args, actual = next(operations)
            assert actual == op, (actual, op)
            if wanted is not None:
                assert args == wanted, (op, args, wanted)
            return args
        take(b'q', [])
        take(b'cm', [1, 0, 0, -1, 0, 792])  # inspection envelope
        take(b'q', [])  # actual body
        next_mcid = 0
        for group in [g for g in expected['groups'] if g['page'] == page_index]:
            assert group['group'] == cursor
            cursor += 1
            counts['groups'] += 1
            take(b'q', [])
            opening = ops(group['begin'])
            assert len(opening) == (2 if group['parent_actual_text'] is not None else 1)
            assert opening[0][1] == b'BDC'
            args = take(b'BDC', opening[0][0])
            props = args[1]
            if group['artifact']:
                assert args[0] == '/Artifact'
                assert '/MCID' not in props and '/ActualText' not in props
                assert group['replacement'] is None and group['parent_actual_text'] is None
                counts['artifacts'] += 1
            else:
                assert props['/MCID'] == group['mcid'] == next_mcid
                next_mcid += 1
            if group['parent_actual_text'] is not None:
                assert '/ActualText' not in props
                assert opening[1][1] == b'BDC'
                args = take(b'BDC', opening[1][0])
                assert args[0] == '/Span' and set(args[1]) == {'/ActualText'}
                assert args[1]['/ActualText'] == group['parent_actual_text']
                counts['replacements'] += 1
            if group['replacement'] is not None:
                assert group['parent_actual_text'] is None
                args = take(b'BDC')
                assert args[0] == '/Span' and set(args[1]) == {'/ActualText'}
                assert args[1]['/ActualText'] == group['replacement']
                counts['replacements'] += 1
            if group['anchor_matrix'] is not None:
                assert group['parent_actual_text'] is not None and not group['artifact']
                take(b'BT', [])
                take(b'Tf', ['/BMA', 1])
                for op, value in [(b'Tr', 3), (b'Tc', 0), (b'Tw', 0), (b'Tz', 100), (b'TL', 0), (b'Ts', 0)]:
                    take(op, [value])
                assert [float(v) for v in take(b'Tm')] == [v / 65536 for v in group['anchor_matrix']]
                shown = take(b'Tj')
                value = shown[0].original_bytes if hasattr(shown[0], 'original_bytes') else bytes(shown[0])
                assert value == b'\x00'
                take(b'ET', [])
                counts['anchors'] += 1
                font = page['/Resources']['/Font']['/BMA']
                assert page['/Resources']['/Font'].raw_get('/BMA').idnum == expected['anchor_font']
                assert font['/Type'] == '/Font' and font['/Subtype'] == '/Type3'
                assert list(font['/FontBBox']) == [0, 0, 1000, 1000]
                assert [float(v) for v in font['/FontMatrix']] == [.001, 0, 0, .001, 0, 0]
                assert list(font['/Widths']) == [1000]
                assert font['/FirstChar'] == font['/LastChar'] == 0
                assert list(font['/Encoding']['/Differences']) == [0, '/anchor']
                assert not font['/Resources']
                assert ops(font['/CharProcs']['/anchor'].get_data()) == [([1000, 0, 0, 0, 1000, 1000], b'd1')]
                cmap = font['/ToUnicode'].get_data()
                assert re.search(rb'1\s+beginbfchar\s+<00>\s+<FFFC>\s+endbfchar', cmap)
            raw = ops(group['raw_commands'])
            assert all(op not in (b'BDC', b'BMC', b'EMC') for _, op in raw)
            for args, op in raw:
                take(op, args)
                counts['glyphs'] += op == b'Tj'
                counts['rules'] += op == b're'
                if op == b'Do':
                    counts['image_placements'] += 1
                    image = page['/Resources']['/XObject'][args[0]]
                    assert image['/Subtype'] in ('/Form', '/Image')
                    assert not {'/MCID', '/Alt', '/ActualText', '/Lang'} & set(image)
                if op == b'Tf':
                    assert page['/Resources']['/Font'][args[0]]['/Subtype'] == '/Type0'
            if group['replacement'] is not None:
                take(b'EMC', [])
            if group['parent_actual_text'] is not None:
                take(b'EMC', [])
            take(b'EMC', [])
            take(b'Q', [])
        take(b'Q', [])  # actual body
        take(b'Q', [])  # inspection envelope
        assert next(operations, None) is None
    assert cursor == len(expected['groups'])
    return counts


def compare_anchor_ink(path, expected, mutool):
    from PIL import Image, ImageChops
    selected = sorted({g['page'] for g in expected['groups'] if g['anchor_matrix'] is not None})
    if not selected:
        return 0
    with tempfile.TemporaryDirectory(prefix='typaxis-anchor-ink-') as temporary:
        root = Path(temporary)
        for removed in (False, True):
            reader = PdfReader(path, strict=True)
            writer = PdfWriter()
            for page_index in selected:
                page = writer.add_page(reader.pages[page_index])
                stream = ContentStream(page['/Contents'], writer)
                original = stream.operations
                filtered = []
                i = 0
                deleted = 0
                while i < len(original):
                    args, op = original[i]
                    if op == b'BT' and i + 1 < len(original) and original[i + 1][1] == b'Tf' and original[i + 1][0][0] == '/BMA':
                        end = i + 2
                        while original[end][1] != b'ET':
                            end += 1
                        if not removed:
                            filtered.extend(original[i:end + 1])
                        deleted += 1
                        i = end + 1
                    else:
                        filtered.append((args, op))
                        i += 1
                assert deleted == sum(g['page'] == page_index and g['anchor_matrix'] is not None for g in expected['groups'])
                # Apply the same parser serialization to both controls, so
                # float formatting cannot masquerade as extraction-glyph ink.
                stream.operations = filtered
                page.replace_contents(stream)
            label = 'without' if removed else 'with'
            output = root / (label + '.pdf')
            writer.write(output)
            subprocess.run([str(mutool), 'draw', '-q', '-r', '72', '-o', str(root / (label + '-%d.png')), str(output)], check=True, capture_output=True)
        for i in range(1, len(selected) + 1):
            with Image.open(root / f'with-{i}.png') as a, Image.open(root / f'without-{i}.png') as b:
                assert a.size == b.size
                assert ImageChops.difference(a.convert('RGB'), b.convert('RGB')).getbbox() is None, (path, selected[i - 1])
    return len(selected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    parser.add_argument('--pdftotext', type=Path)
    parser.add_argument('--mutool', type=Path)
    args = parser.parse_args()
    totals = dict(probes=0, tamper_rejections=0, gsub_extractions=0)
    if args.mutool:
        version = subprocess.run([str(args.mutool), '-v'], capture_output=True, check=True)
        assert '1.28.2' in (version.stdout + version.stderr).decode(), 'unpinned MuPDF version'
    paths = sorted(args.directory.glob('*.pdf'))
    assert paths, 'no marked-content probes'
    for path in paths:
        expected = json.loads(path.with_suffix('.json').read_text())
        data = path.read_bytes()
        for key, value in verify(PdfReader(io.BytesIO(data), strict=True), expected).items():
            totals[key] = totals.get(key, 0) + value
        totals['probes'] += 1
        if args.mutool:
            totals['anchor_ink_comparisons'] = totals.get('anchor_ink_comparisons', 0) + compare_anchor_ink(path, expected, args.mutool)
        if args.self_test:
            mutations = [(b'1 0 0 -1 0 792 cm', b'1 0 0  1 0 792 cm')]
            if b'/MCID 0 ' in data:
                mutations.append((b'/MCID 0 ', b'/MCID 1 '))
            if b'/BMA 1 Tf 3 Tr' in data:
                mutations.append((b'/BMA 1 Tf 3 Tr', b'/BMA 1 Tf 0 Tr'))
            if b'/ActualText <FEFF' in data:
                mutations.append((b'/ActualText <FEFF', b'/ActualText <FEFE'))
            for old, new in mutations:
                assert len(old) == len(new) and old in data
                changed = data.replace(old, new, 1)
                try:
                    verify(PdfReader(io.BytesIO(changed), strict=True), expected)
                except (AssertionError, KeyError):
                    totals['tamper_rejections'] += 1
                else:
                    raise AssertionError(f'accepted modified marked stream: {old!r}')
        if args.pdftotext:
            original = ''.join(g['replacement'] or g['parent_actual_text'] or '' for g in expected['groups'])
            if original == 'ABA fi X D E f i':
                result = subprocess.run([str(args.pdftotext), '-raw', str(path), '-'], capture_output=True, check=True)
                assert result.stdout.decode().rstrip('\n\f') == original, result.stdout
                totals['gsub_extractions'] += 1
    assert totals['anchors'] and totals['artifacts'] and totals['image_placements'] and totals['glyphs']
    if args.pdftotext:
        assert totals['gsub_extractions'], 'actual GSUB extraction fixture missing'
    print(json.dumps(totals, sort_keys=True))


if __name__ == '__main__':
    main()
