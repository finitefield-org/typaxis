#!/usr/bin/env python3
"""Check selected book-2 vector commands against admitted IR and original SVG.

PDF envelopes made here are inspection fixtures, not book-2 page/structure output.
Physical operators are parsed independently with pypdf. Optional rendering compares
MuPDF with librsvg at fixed 144/288 DPI (mean RGB error <= 2/255; >32 error <= 3%).
"""
import argparse
import hashlib
import io
import importlib.metadata
import json
import subprocess
import tempfile
from fractions import Fraction
from pathlib import Path
from PIL import Image, ImageChops, ImageStat
from pypdf import PdfWriter
from pypdf.generic import (ContentStream, DecodedStreamObject, DictionaryObject,
                          NameObject, NumberObject, FloatObject, ArrayObject,
                          read_object)

ONE = 65536


def mul(a, b):
    return round(Fraction(a * b, ONE))


def transform(left, right):
    return dict(a=mul(left['a'], right['a']), d=mul(left['d'], right['d']),
                e=mul(left['a'], right['e']) + left['e'],
                f=mul(left['d'], right['f']) + left['f'])


def expected_ops(ir):
    ops = []
    def add(op, *args):
        ops.append((list(args), op.encode('ascii')))
    def fixed(op, *raw):
        add(op, *(v / ONE for v in raw))
    def path(segments, matrix=None):
        current = start = None
        def point(p):
            if matrix is None:
                return p
            return [mul(matrix['a'], p[0]) + matrix['e'], mul(matrix['d'], p[1]) + matrix['f']]
        for s in segments:
            kind = s['kind']
            points = [point(p) for p in s.get('points', [])]
            if kind == 'close':
                add('h')
                current = start
            elif kind in ('move', 'line'):
                fixed('m' if kind == 'move' else 'l', *points[0])
                current = points[0]
                if kind == 'move':
                    start = current
            elif kind == 'quadratic':
                control, endpoint = points
                first = [round(Fraction(v + 2*c, 3)) for v, c in zip(current, control)]
                second = [round(Fraction(v + 2*c, 3)) for v, c in zip(endpoint, control)]
                fixed('c', *first, *second, *endpoint)
                current = endpoint
            else:
                assert kind == 'cubic'
                fixed('c', *(v for p in points for v in p))
                current = points[-1]
    v2 = ir['algorithm'].endswith('/2')
    pairs = [(d['fill']['alpha'], d['stroke']['alpha']) if v2 else (ONE, ONE) for d in ir['draws']]
    states = sorted(set(pairs))
    add('q')
    fixed('re', 0, 0, ir['intrinsic_width'], ir['intrinsic_height'])
    add('W'); add('n')
    scale = ir['root_scale']
    fixed('cm', scale, 0, 0, scale, -mul(scale, ir['view_box'][0]), -mul(scale, ir['view_box'][1]))
    for draw, pair in zip(ir['draws'], pairs):
        add('q')
        for use in draw['clips']:
            definition = ir['clips'][use['clip_id']]
            assert definition['clip_id'] == use['clip_id']
            path(definition['path'], transform(use['transform'], definition['transform']))
            add('W*' if definition['fill_rule'] == 'evenodd' else 'W'); add('n')
        t = draw['transform']
        fixed('cm', t['a'], 0, 0, t['d'], t['e'], t['f'])
        add('gs', '/GS' + str(states.index(pair)))
        if v2:
            fill, stroke = draw['fill']['paint'], draw['stroke']['paint']
        else:
            fill = {'kind': 'none'} if draw['fill'] is None else {'kind': 'fixed-rgb8', 'rgb': draw['fill']}
            stroke = {'kind': 'none'} if draw['stroke'] is None else {'kind': 'fixed-rgb8', 'rgb': draw['stroke']['color']}
        for paint, op in [(fill, 'rg'), (stroke, 'RG')]:
            if paint['kind'] == 'fixed-rgb8':
                fixed(op, *(round(Fraction(c * ONE, 255)) for c in paint['rgb']))
        if stroke['kind'] != 'none':
            s = draw['stroke']
            fixed('w', s['width'])
            add('J', {'butt': 0, 'round': 1, 'square': 2}[s['line_cap']])
            add('j', {'miter': 0, 'round': 1, 'bevel': 2}[s['line_join']])
            fixed('M', s['miter_limit'])
        if fill['kind'] != 'none':
            path(draw['path'])
            add('f*' if draw['fill_rule'] == 'evenodd' else 'f')
        if stroke['kind'] != 'none':
            path(draw['path'])
            add('S')
        add('Q')
    add('Q')
    return ops, states


def check_content(data, ir):
    stream = DecodedStreamObject(); stream.set_data(data)
    operations = ContentStream(stream, None).operations
    expected, states = expected_ops(ir)
    assert len(operations) == len(expected), (len(operations), len(expected))
    for i, ((actual, op), (args, wanted)) in enumerate(zip(operations, expected)):
        assert op == wanted, (i, op, wanted)
        normalized = [str(v) if isinstance(v, NameObject) else float(v) for v in actual]
        assert normalized == args, (i, op, normalized, args)
    return len(operations), states


def envelope(content, ir, dictionaries, path):
    writer = PdfWriter()
    width, height = ir['intrinsic_width']/ONE, ir['intrinsic_height']/ONE
    page = writer.add_blank_page(width, height)
    states = DictionaryObject({NameObject('/GS'+str(i)): writer._add_object(d) for i, d in enumerate(dictionaries)})
    form = DecodedStreamObject(); form.set_data(content)
    form.update({NameObject('/Type'): NameObject('/XObject'), NameObject('/Subtype'): NameObject('/Form'),
                 NameObject('/FormType'): NumberObject(1), NameObject('/BBox'): ArrayObject([FloatObject(v) for v in [0, 0, width, height]]),
                 NameObject('/Resources'): DictionaryObject({NameObject('/ExtGState'): states})})
    page[NameObject('/Resources')] = DictionaryObject({NameObject('/XObject'): DictionaryObject({NameObject('/V'): writer._add_object(form)})})
    commands = DecodedStreamObject(); commands.set_data(f'q 1 0 0 -1 0 {height:.16f} cm 0 g 0 G /V Do Q'.encode('ascii'))
    page[NameObject('/Contents')] = writer._add_object(commands)
    with path.open('wb') as out:
        writer.write(out)


def render_check(source, pdf, directory, mutool, rsvg):
    observations = []
    for dpi in (144, 288):
        original = directory / f'svg-{dpi}.png'
        actual = directory / f'pdf-{dpi}.png'
        subprocess.run([rsvg, '--dpi-x', str(dpi), '--dpi-y', str(dpi), '-o', str(original), str(source)], check=True, capture_output=True)
        subprocess.run([mutool, 'draw', '-q', '-r', str(dpi), '-c', 'rgb', '-o', str(actual), str(pdf)], check=True, capture_output=True)
        def rgb(path):
            rgba = Image.open(path).convert('RGBA')
            white = Image.new('RGBA', rgba.size, (255, 255, 255, 255))
            return Image.alpha_composite(white, rgba).convert('RGB')
        a, b = rgb(original), rgb(actual)
        assert a.size == b.size, (a.size, b.size)
        diff = ImageChops.difference(a, b)
        mean = sum(ImageStat.Stat(diff).mean)/3
        fraction = sum(max(pixel)>32 for pixel in diff.get_flattened_data()) / (a.width*a.height)
        assert mean <= 2.0 and fraction <= 0.03, (source.name, dpi, mean, fraction)
        observations.append({'dpi': dpi, 'mean_rgb_error': mean, 'fraction_over_32': fraction})
    return observations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--mutool')
    parser.add_argument('--rsvg-convert')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    assert bool(args.mutool) == bool(args.rsvg_convert)
    paths = sorted(args.directory.glob('*.json'))
    assert paths, 'no vector probes'
    totals = {'probes': len(paths), 'operations': 0, 'draws': 0, 'states': 0, 'tamper_rejections': 0, 'renders': []}
    totals['parser_versions'] = {name: importlib.metadata.version(name) for name in ('pypdf', 'Pillow')}
    if args.mutool:
        totals['renderer_versions'] = {}
        for name, command, expected in [('mutool', [args.mutool, '-v'], 'mutool version 1.28.2'), ('rsvg-convert', [args.rsvg_convert, '--version'], 'rsvg-convert version 2.62.3')]:
            result = subprocess.run(command, check=True, capture_output=True, text=True)
            first = (result.stdout + result.stderr).strip().splitlines()[0]
            assert first == expected, (name, first, expected)
            totals['renderer_versions'][name] = first
    versions = set()
    for path in paths:
        expected = json.loads(path.read_text()); ir = expected['ir']; versions.add(ir['algorithm'])
        source = path.with_suffix('.svg'); content = path.with_suffix('.content').read_bytes()
        assert hashlib.sha256(source.read_bytes()).hexdigest() == expected['source_sha256']
        assert hashlib.sha256(content).hexdigest() == expected['content_sha256']
        count, states = check_content(content, ir)
        dictionaries = [read_object(io.BytesIO(s.encode('ascii')), None) for s in expected['states']]
        assert len(dictionaries) == len(states)
        for d, (fill, stroke) in zip(dictionaries, states):
            assert set(d) == {'/Type', '/ca', '/CA'} and d['/Type'] == '/ExtGState'
            assert float(d['/ca']) == fill/ONE and float(d['/CA']) == stroke/ONE
        totals['operations'] += count; totals['draws'] += len(ir['draws']); totals['states'] += len(states)
        if args.self_test:
            for old, new in [(b' re W n', b' re W* n'), (b'/GS0 gs', b'/GS999 gs')]:
                assert old in content
                try:
                    check_content(content.replace(old, new, 1), ir)
                except (AssertionError, ValueError):
                    totals['tamper_rejections'] += 1
                else:
                    raise AssertionError('tampered vector accepted')
        if args.mutool:
            with tempfile.TemporaryDirectory(prefix='typaxis-vector-') as tmp:
                directory = Path(tmp); pdf = directory/'probe.pdf'
                envelope(content, ir, dictionaries, pdf)
                totals['renders'].append({'probe': path.stem, 'comparisons': render_check(source, pdf, directory, args.mutool, args.rsvg_convert)})
    totals['ir_versions'] = sorted(versions)
    print(json.dumps(totals))


if __name__ == '__main__':
    main()
