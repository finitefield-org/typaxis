#!/usr/bin/env python3
"""Parse actual book-2 image placement commands and their real XObject references.

Probes flatten commands onto a single inspection page; original page assembly,
marked content, reading order and final PDF/UA are not supplied by this fixture.
"""
import argparse
import io
import json
from collections import Counter
from pathlib import Path
from pypdf import PdfReader
from pypdf.generic import ContentStream


def verify(reader, expected):
    assert len(reader.pages) == 1
    page = reader.pages[0]
    resources = page['/Resources']['/XObject']
    assert set(resources) == {'/BI'+str(c['resource']) for c in expected}
    operations = iter(ContentStream(page['/Contents'], reader).operations)
    def take(op):
        operands, actual = next(operations)
        assert actual == op, (actual, op)
        return operands
    assert take(b'q') == []
    assert list(take(b'cm')) == [1,0,0,-1,0,792]
    raster = vector = 0
    for command in expected:
        assert take(b'q') == []
        if command['color'] is not None:
            color = [((v*65536+127)//255)/65536 for v in command['color']]
            assert [float(v) for v in take(b'rg')] == color
            assert [float(v) for v in take(b'RG')] == color
            vector += 1
        else:
            raster += 1
        assert [float(v) for v in take(b'cm')] == [v/65536 for v in command['matrix']]
        name = '/BI'+str(command['resource'])
        assert list(take(b'Do')) == [name]
        assert resources.raw_get(name).idnum == command['object']
        resource = resources[name]
        assert resource['/Subtype'] == ('/Image' if command['color'] is None else '/Form')
        assert not {'/MCID','/Alt','/ActualText','/Lang'} & set(resource)
        assert take(b'Q') == []
    assert take(b'Q') == []
    assert next(operations, None) is None
    return raster, vector


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    total = {'pdfs':0,'raster_placements':0,'vector_placements':0,'shared_resource_uses':0,'tamper_rejections':0}
    for path in sorted(args.directory.glob('*.pdf')):
        data = path.read_bytes(); expected = json.loads(path.with_suffix('.json').read_text())
        raster, vector = verify(PdfReader(io.BytesIO(data),strict=True), expected)
        total['pdfs'] += 1; total['raster_placements'] += raster; total['vector_placements'] += vector
        total['shared_resource_uses'] += sum(n-1 for n in Counter(c['resource'] for c in expected).values())
        if args.self_test:
            mutations = [(b'1 0 0 -1 0 792 cm', b'1 0 0  1 0 792 cm')]
            if expected:
                mutations.append((b'/BI0 Do',b'/ZZ0 Do'))
            for old, new in mutations:
                assert len(old)==len(new) and old in data
                altered = data.replace(old,new,1)
                try:
                    verify(PdfReader(io.BytesIO(altered),strict=True),expected)
                except (AssertionError,KeyError):
                    total['tamper_rejections'] += 1
                else:
                    raise AssertionError('tampered image command accepted')
    assert total['pdfs'] and total['raster_placements'] and total['vector_placements'] and total['shared_resource_uses']
    print(json.dumps(total))


if __name__ == '__main__':
    main()
