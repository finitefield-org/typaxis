#!/usr/bin/env python3
"""Verify legacy CFF finalization budget probes against the synthetic source font."""
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path
from fontTools.ttLib import TTFont

FIXTURE = Path(__file__).resolve().parents[1] / 'samples/machine-package/staging/production-book-1/cff-media/typaxis-cff-fixture.otf.hex'
SOURCE_HASH = '16da5a41c10414b308aa62db39066e0540e460f3e2d0f9c2811f0b32907d94f0'


def check(probe, axis, font, data):
    assert probe['axis'] == axis
    root = font['CFF '].cff.topDictIndex[0].CharStrings['.notdef'].bytecode
    # Independent FontTools program lookup and unique source-byte match. These
    # are the second token and first line of this fixed synthetic program.
    assert root == bytes.fromhex('f8ecbd16f888f950fc88060e')
    assert data.count(root) == 1
    position = data.index(root) + (2 if axis == 'operations' else 10)
    table_position = position - font.reader.tables['CFF '].offset
    operator = None if axis == 'operations' else 6
    assert probe['file_offset'] == position
    assert probe['table_offset'] == table_position
    assert probe['operator'] == operator
    assert probe['gid'] == 0 and probe['fd'] is None
    assert probe['limit'] == 1 and probe['observed'] == 2
    assert font['OS/2'].fsType == 0
    fields = ['phase=charstring', 'reason=budget_exceeded', 'class=resource-budget',
              'requested_face_index=0', 'table=CFF ', 'offset_kind=field',
              f'font_byte={position}', f'table_byte={table_position}']
    if operator is not None:
        fields.append('cff_operator=0x0006')
    fields += ['gid=0', 'limit=1', 'observed=2', 'embedding=allowed', 'fs_type=0x0000']
    note = '; '.join(fields)
    assert probe['note'] == note
    code, kind = ('R7133', 'CharstringOperationLimit') if axis == 'operations' else ('R7134', 'OutlineSegmentLimit')
    assert probe['message'] == f'{code}: {kind}; font_face_id=0; {note}'
    assert probe['exit_code'] == 5


def verify(directory, self_test):
    data = bytes.fromhex(FIXTURE.read_text())
    assert hashlib.sha256(data).hexdigest() == SOURCE_HASH
    rejected = 0
    with TTFont(io.BytesIO(data)) as font:
        for axis in ['operations', 'segments']:
            probe = json.loads((directory / f'{axis}.json').read_text())
            check(probe, axis, font, data)
            if self_test:
                for field, value in [('axis', 'wrong'), ('file_offset', 0), ('table_offset', 0),
                                     ('operator', 0), ('gid', 1), ('fd', 0), ('limit', 2),
                                     ('observed', 1), ('note', 'lost'), ('message', 'lost'), ('exit_code', 4)]:
                    altered = copy.deepcopy(probe)
                    altered[field] = value
                    try:
                        check(altered, axis, font, data)
                    except AssertionError:
                        rejected += 1
                    else:
                        raise AssertionError(f'accepted altered {field}')
    print(f'PASS: 2 legacy-CFF finalization diagnostics / {rejected} alterations rejected; source positions, GID, absent FD, budgets, CLI messages and exit codes verified')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    verify(args.directory, args.self_test)
