#!/usr/bin/env python3
"""Verify public contract-1.4 CFF build failures and their admitted-font records."""
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path

from fontTools.ttLib import TTFont
from fontTools.pens.boundsPen import BoundsPen

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / 'samples/machine-package/staging/production-book-1/cff-media/typaxis-cff-fixture.otf.hex'
CASES = {
    'operations': ('R7133', 'budget_exceeded', 'resource-budget', 'CharstringOperationLimit', 5, 2, None),
    'segments': ('R7134', 'budget_exceeded', 'resource-budget', 'OutlineSegmentLimit', 5, 10, 6),
    'subset': ('R7135', 'budget_exceeded', 'resource-budget', 'SubsetByteLimit', 5, None, None),
    'reserved': ('R7100', 'reserved_cff_operator', 'malformed-or-invalid-input', 'InvalidCharstring', 1, 0, 0),
    'unsupported': ('R7100', 'unsupported_cff_operator', 'unsupported', 'InvalidCharstring', 1, 0, 0x0c0a),
    'end': ('R7100', 'invalid_charstring', 'malformed-or-invalid-input', 'InvalidCharstring', 1, 12, None),
    'bounds': ('R7100', 'invalid_table', 'malformed-or-invalid-input', 'InvalidSubset', 1, None, None),
}


def checksum(data):
    data = data + bytes((-len(data)) % 4)
    return sum(int.from_bytes(data[i:i+4], 'big') for i in range(0, len(data), 4)) & 0xffffffff


def expected_source(original, case, start):
    data = bytearray(original)
    if case == 'reserved':
        data[start] = 0
    elif case == 'unsupported':
        data[start:start+2] = bytes([12, 10])
    elif case == 'end':
        data[start:start+12] = bytes([139]) * 12
    elif case == 'bounds':
        data[start:start+12] = bytes([248, 236, 255, 127, 255, 128, 0, 22, 138, 139, 5, 14])
    else:
        return bytes(data)
    with TTFont(io.BytesIO(original)) as font:
        head = font.reader.tables['head'].offset
    data[head+8:head+12] = bytes(4)
    count = int.from_bytes(data[4:6], 'big')
    for pos in range(12, 12 + count * 16, 16):
        offset = int.from_bytes(data[pos+8:pos+12], 'big')
        length = int.from_bytes(data[pos+12:pos+16], 'big')
        data[pos+4:pos+8] = checksum(data[offset:offset+length]).to_bytes(4, 'big')
    data[head+8:head+12] = ((0xB1B0AFBA - checksum(data)) & 0xffffffff).to_bytes(4, 'big')
    return bytes(data)


def check(case, artifacts, data, original, start, cff_base, reference_size):
    code, reason, category, kind, exit_code, position, operator = CASES[case]
    assert data == expected_source(original, case, start)
    digest = hashlib.sha256(data).hexdigest()
    diagnostic, checked, manifest, outcome = artifacts
    assert diagnostic['contract'] == checked['contract'] == manifest['contract'] == 'typaxis.contract/1.4'
    assert checked['diagnostics'] == []
    assert len(diagnostic['diagnostics']) == 1
    d = diagnostic['diagnostics'][0]
    assert d['code'] == code and d['severity'] == 'error'
    assert d['message'] == f'cff1 {reason}'
    assert d['location'] == {'byte_offset': None, 'json_pointer': '/resources/font_faces/2',
                             'kind': 'package_json', 'uri': 'document-package.json'}
    fields = [f"phase={'subset' if case in ('subset', 'bounds') else 'charstring'}", f'reason={reason}',
              f'class={category}', 'requested_face_index=0']
    if position is not None:
        fields += ['table=CFF ', f"offset_kind={'program-end' if case == 'end' else 'field'}",
                   f'font_byte={start + position}', f'table_byte={start + position - cff_base}']
        if operator is not None:
            fields.append(f'cff_operator=0x{operator:04X}')
        fields.append('gid=0')
    if case == 'subset':
        fields += ['subset_stage=sfnt-size', 'limit=1', f'observed={reference_size}']
    if case == 'bounds':
        fields += ['gid=0', 'subset_stage=glyph-bounds']
        with TTFont(io.BytesIO(data)) as font:
            glyphs = font.getGlyphSet()
            pen = BoundsPen(glyphs)
            glyphs['.notdef'].draw(pen)
            assert pen.bounds == (32766.5, 0, 32767.5, 0)
    if case in ('operations', 'segments'):
        fields += ['limit=1', 'observed=2']
    fields += ['embedding=allowed', 'fs_type=0x0000']
    context = '; '.join(fields)
    assert len(d['notes']) == 3
    assert all(n['location'] is None for n in d['notes'])
    assert d['notes'][0]['message'] == 'resource=typaxis-cff-fixture.otf'
    assert d['notes'][1]['message'] == context
    assert 'standalone name-keyed CFF1' in d['notes'][2]['message']
    assert 'inspect-font FONT' in d['notes'][2]['message']
    assert manifest['status'] == 'failed' and manifest['output'] is None
    assert len(manifest['fonts']) == 3
    assert manifest['fonts'][2] == {
        'attested_media_kind': 'sfnt-cff1', 'bytes': len(data), 'face_index': 0, 'font_face_id': 2,
        'glyph_count': 4, 'media_declaration': {'kind': 'declared', 'media_type': 'sfnt-cff1'},
        'sha256': digest, 'units_per_em': 1000, 'uri': 'typaxis-cff-fixture.otf'}
    assert outcome == {'case': case, 'exit_code': exit_code, 'font_sha256': digest,
                       'message': f'{code}: {kind}; font_face_id=2; {context}',
                       'output_preserved': case == 'unsupported', 'resource_uri': 'typaxis-cff-fixture.otf'}


def verify(directory, self_test):
    original = bytes.fromhex(FIXTURE.read_text())
    assert hashlib.sha256(original).hexdigest() == '16da5a41c10414b308aa62db39066e0540e460f3e2d0f9c2811f0b32907d94f0'
    with TTFont(io.BytesIO(original)) as font:
        program = font['CFF '].cff.topDictIndex[0].CharStrings['.notdef'].bytecode
        assert program == bytes.fromhex('f8ecbd16f888f950fc88060e')
        assert original.count(program) == 1
        start = original.index(program)
        cff_base = font.reader.tables['CFF '].offset
    # Use the successful public build's real embedded OpenType program as the
    # independent byte-size oracle. Recompute the padded SFNT length and checksum.
    reference = (directory / 'subset' / 'reference-subset.otf').read_bytes()
    pdf = (directory / 'subset' / 'reference.pdf').read_bytes()
    assert pdf.count(reference) == 1
    assert b'stream\n' + reference + b'\nendstream' in pdf
    assert checksum(reference) == 0xB1B0AFBA
    with TTFont(io.BytesIO(reference)) as font:
        assert set(font.reader.tables) == {'CFF ', 'OS/2', 'cmap', 'head', 'hhea', 'hmtx', 'maxp', 'name', 'post'}
        reference_size = 12 + 16 * len(font.reader.tables) + sum((t.length + 3) & ~3 for t in font.reader.tables.values())
        assert reference_size == len(reference)
        assert font['CFF '].cff.topDictIndex[0].ROS == ('Adobe', 'Identity', 0)
        assert font.getBestCmap()  # actual selected text, not an empty font
    rejected = 0
    for case in CASES:
        folder = directory / case
        data = (folder / 'font.otf').read_bytes()
        artifacts = [json.loads((folder / name).read_text()) for name in
                     ['diagnostics.json', 'check-diagnostics.json', 'manifest.json', 'outcome.json']]
        check(case, artifacts, data, original, start, cff_base, reference_size)
        if self_test:
            for change in range(12):
                altered = copy.deepcopy(artifacts)
                d = altered[0]['diagnostics'][0]
                if change == 0: d['code'] = 'I9190'
                elif change == 1: d['location']['json_pointer'] = '/resources/font_faces/0'
                elif change == 2: d['location']['byte_offset'] = start
                elif change == 3: d['notes'][0]['message'] = 'resource=other.otf'
                elif change == 4: d['notes'][1]['message'] += '; fd=0'
                elif change == 5: altered[1]['diagnostics'] = [d]
                elif change == 6: altered[2]['status'] = 'built'
                elif change == 7: altered[2]['fonts'][2]['sha256'] = '0' * 64
                elif change == 8: altered[3]['exit_code'] = 4
                elif change == 9: altered[0]['diagnostics'].append(d)
                elif change == 10: d['notes'][1]['message'] += '; subset_stage=sfnt-write'
                else: d['notes'][1]['message'] += '; observed=0'
                try:
                    check(case, altered, data, original, start, cff_base, reference_size)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError(f'accepted change {change} for {case}')
    print(f'PASS: {len(CASES)} public CFF failures / {rejected} alterations rejected; check/build distinction, resource pointers, source bytes, notes, failed manifests and exit codes verified')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    verify(args.directory, args.self_test)
