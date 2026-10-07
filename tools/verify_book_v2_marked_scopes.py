#!/usr/bin/env python3
"""Parse actual book-2 marked-boundary bytes; this is not final PDF/UA proof."""
import argparse
import json
from pathlib import Path

from pypdf.generic import ContentStream, DecodedStreamObject


def boundary_bytes(group):
    closing = 'EMC\nEMC\n' if group['actual_text'] is not None else 'EMC\n'
    return (group['begin'] + closing).encode()


def check(group, data=None):
    stream = DecodedStreamObject()
    stream.set_data(boundary_bytes(group) if data is None else data)
    operations = ContentStream(stream, None).operations
    actual = group['actual_text']
    assert len(operations) == (4 if actual is not None else 2), operations
    (args, op), (endargs, endop) = operations[0], operations[-1]
    assert op == b'BDC' and endop == b'EMC' and not endargs
    assert len(args) == 2 and str(args[0]) == '/' + group['tag']
    props = args[1]
    if group['tag'] == 'Artifact':
        assert group['mcid'] is None and group['actual_text'] is None
        keys = {'/Type'} | ({'/Subtype'} if group['subtype'] else set())
        assert set(props) == keys
        assert str(props['/Type']) == '/' + group['type']
        if group['subtype']:
            assert str(props['/Subtype']) == '/' + group['subtype']
    else:
        assert set(props) == {'/MCID'}
        assert int(props['/MCID']) == group['mcid']
        if actual is not None:
            (replacement, beginop), (closing, closeop) = operations[1:3]
            assert beginop == b'BDC' and closeop == b'EMC' and not closing
            assert replacement[0] == '/Span' and set(replacement[1]) == {'/ActualText'}
            assert str(replacement[1]['/ActualText']) == actual


def reject(group, data):
    try:
        check(group, data)
    except (AssertionError, ValueError):
        return
    raise AssertionError('accepted changed marked-content boundary')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    counts = dict(probes=0, groups=0, semantic=0, repeated_headers=0,
                  separators=0, actual_text=0, covered_paints=0, tamper_rejections=0)
    files = sorted(args.directory.glob('*.json'))
    assert files, 'no marked-scope probes'
    for file in files:
        probe = json.loads(file.read_text())
        counts['probes'] += 1
        next_mcid = [0] * probe['pages']
        cursor = 0
        page_before = 0
        for group in probe['groups']:
            assert group['paint_start'] == cursor < group['paint_end']
            cursor = group['paint_end']
            page = group['page']
            assert page_before <= page < probe['pages']
            page_before = page
            check(group)
            counts['groups'] += 1
            if group['tag'] == 'Artifact':
                counts['repeated_headers' if group['subtype'] == 'Header' else 'separators'] += 1
            else:
                assert group['mcid'] == next_mcid[page]
                next_mcid[page] += 1
                counts['semantic'] += 1
            counts['actual_text'] += group['actual_text'] is not None
            if args.self_test:
                data = boundary_bytes(group)
                if group['mcid'] is not None:
                    old = f"/MCID {group['mcid']}".encode()
                    changed = f"/MCID {group['mcid'] + 1}".encode()
                    reject(group, data.replace(old, changed, 1))
                else:
                    reject(group, data.replace(b'/Artifact', b'/Span', 1))
                counts['tamper_rejections'] += 1
        counts['covered_paints'] += cursor
    print(json.dumps(counts, sort_keys=True))


if __name__ == '__main__':
    main()
