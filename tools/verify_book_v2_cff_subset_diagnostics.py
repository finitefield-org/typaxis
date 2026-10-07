#!/usr/bin/env python3
"""Check /2 driver subset failures against original Harano and real output bytes.

Requires FontTools. This verifies private-driver evidence, not public Book /2
availability or whole-book acceptance.
"""
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.ttLib import TTFont

HARANO_SHA256 = '66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717'


def checksum(data):
    data += bytes((-len(data)) % 4)
    return sum(int.from_bytes(data[i:i+4], 'big') for i in range(0, len(data), 4)) & 0xffffffff


def reference(directory, original):
    data = (directory / 'reference.otf').read_bytes()
    pdf = (directory / 'reference.pdf').read_bytes()
    assert pdf.count(data) == 1
    assert b'stream\n' + data + b'\nendstream' in pdf
    assert checksum(data) == 0xB1B0AFBA
    source = json.loads((directory / 'source.json').read_text())
    assert source['resources']['font_faces'][0]['expected_sha256'] == HARANO_SHA256
    assert source['resources']['font_faces'][0]['media_type'] == 'sfnt-cff1'
    with TTFont(io.BytesIO(data)) as subset, TTFont(io.BytesIO(original)) as font:
        assert font['OS/2'].fsType == 0
        tables = subset.reader.tables
        assert set(tables) == {'CFF ', 'OS/2', 'cmap', 'head', 'hhea', 'hmtx', 'maxp', 'name', 'post'}
        size = 12 + 16 * len(tables) + sum((t.length + 3) & ~3 for t in tables.values())
        assert size == len(data)
        top = subset['CFF '].cff.topDictIndex[0]
        original_top = font['CFF '].cff.topDictIndex[0]
        assert top.ROS == ('Adobe', 'Identity', 0)
        assert len(top.charset) == 3
        prefix = len(top.CharStrings['.notdef'].bytecode)
        fd = original_top.FDSelect[0]
        assert prefix > 1 and prefix < size - 1
        original_glyphs, subset_glyphs = font.getGlyphSet(), subset.getGlyphSet()
        names = [('.notdef', '.notdef')]
        for char in '本文':
            names.append((font.getBestCmap()[ord(char)], subset.getBestCmap()[ord(char)]))
        assert {new for _, new in names} == set(top.charset)
        for old, new in names:
            a, b = BoundsPen(original_glyphs), BoundsPen(subset_glyphs)
            original_glyphs[old].draw(a)
            subset_glyphs[new].draw(b)
            assert a.bounds == b.bounds
            assert font['hmtx'].metrics[old][0] == subset['hmtx'].metrics[new][0]
    return size, prefix, fd


def check(case, fact, size, prefix, fd):
    early = case == 'charstrings'
    limit, observed = (1, prefix) if early else (size - 1, size)
    stage = 'charstring-size' if early else 'sfnt-size'
    fields = ['phase=subset', 'reason=budget_exceeded', 'class=resource-budget', 'requested_face_index=0']
    if early:
        fields += ['gid=0', f'fd={fd}']
    fields += [f'subset_stage={stage}', f'limit={limit}', f'observed={observed}', 'embedding=allowed', 'fs_type=0x0000']
    assert set(fact) == {'font_sha256', 'font_face_id', 'case', 'note', 'code', 'gid', 'fd', 'limit', 'observed', 'source_chain_depth'}
    assert fact['font_sha256'] == HARANO_SHA256
    assert fact['font_face_id'] == 0 and fact['case'] == case
    assert fact['code'] == 'R7135'
    assert (fact['limit'], fact['observed']) == (limit, observed)
    assert (fact['gid'], fact['fd']) == ((0, fd) if early else (None, None))
    assert 3 <= fact['source_chain_depth'] <= 8
    assert fact['note'] == '; '.join(fields)


def verify(directory, font_path, self_test):
    original = font_path.read_bytes()
    assert hashlib.sha256(original).hexdigest() == HARANO_SHA256
    size, prefix, fd = reference(directory, original)
    rejected = 0
    for case in ['charstrings', 'sfnt']:
        fact = json.loads((directory / f'{case}.json').read_text())
        check(case, fact, size, prefix, fd)
        if self_test:
            for change in range(12):
                altered = copy.deepcopy(fact)
                if change == 0: altered['font_sha256'] = '0' * 64
                elif change == 1: altered['font_face_id'] = 1
                elif change == 2: altered['case'] = 'other'
                elif change == 3: altered['code'] = 'I9190'
                elif change == 4: altered['gid'] = 1
                elif change == 5: altered['fd'] = 0
                elif change == 6: altered['limit'] += 1
                elif change == 7: altered['observed'] += 1
                elif change == 8: altered['source_chain_depth'] = 0
                elif change == 9: altered['note'] += '; font_byte=0'
                elif change == 10: altered['note'] = altered['note'].replace('subset_stage=', 'table=')
                else: altered['note'] = altered['note'].replace('embedding=allowed', 'embedding=denied')
                try:
                    check(case, altered, size, prefix, fd)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError(f'accepted change {change} in {case}')
    print(f'PASS: 2 original-Harano subset diagnostics / {rejected} alterations rejected; resource identity, GID/FD, original outlines, actual charstring bytes ({prefix}), SFNT bytes ({size}) and budget notes verified')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--font', type=Path, required=True)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    verify(args.directory, args.font, args.self_test)
