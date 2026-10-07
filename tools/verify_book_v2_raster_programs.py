#!/usr/bin/env python3
"""Independently compare book-2 raster payloads with original PNG/JPEG pixels.

These are selected-raster program probes; no PDF object/page/structure success
is implied. Pillow decodes source media, and zlib decodes actual PDF payloads.
"""
import argparse
import hashlib
import io
import json
import zlib
from pathlib import Path
from PIL import Image


def verify(path):
    expected = json.loads(path.read_text())
    source = path.with_suffix('.source').read_bytes()
    color = path.with_suffix('.color').read_bytes()
    assert hashlib.sha256(source).hexdigest() == expected['source_sha256']
    assert hashlib.sha256(color).hexdigest() == expected['payload_sha256']
    original = Image.open(io.BytesIO(source))
    original.load()
    assert original.size == (expected['width'], expected['height'])
    if expected['encoding'] == 'Jpeg':
        assert not expected['alpha']
        normalized = Image.open(io.BytesIO(color))
        normalized.load()
        assert normalized.size == original.size
        assert normalized.convert('RGB').tobytes() == original.convert('RGB').tobytes()
        return 0, 1, 0
    assert expected['encoding'] == 'Flate'
    mode = {'Rgb': 'RGB', 'Gray': 'L'}[expected['color_space']]
    decoded = zlib.decompress(color)
    assert decoded == original.convert(mode).tobytes()
    alpha = original.convert('RGBA').getchannel('A').tobytes()
    assert expected['alpha'] == any(value != 255 for value in alpha)
    if expected['alpha']:
        assert zlib.decompress(path.with_suffix('.alpha').read_bytes()) == alpha
    else:
        assert not path.with_suffix('.alpha').exists()
    return 1, 0, int(expected['alpha'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.json'))
    assert paths, 'no raster probes'
    totals = [0, 0, 0]
    for path in paths:
        totals = [a + b for a, b in zip(totals, verify(path))]
    assert totals[0] and totals[1], 'both PNG and JPEG must be exercised'
    print(json.dumps({'probes': len(paths), 'png': totals[0], 'jpeg': totals[1], 'alpha': totals[2]}))


if __name__ == '__main__':
    main()
