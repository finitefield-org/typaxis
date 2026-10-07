#!/usr/bin/env python3
"""Independently read actual book-2 Image/SMask/Form/ExtGState fragments.

The Rust fixture supplies only an empty inspection page. Optional renders attach
one actual parsed XObject to a new inspection page, not a production book page.
Raster pixel comparisons disable MuPDF antialiasing (-A 0) to avoid resampling;
vector comparisons retain their established antialiasing and fixed thresholds.
"""
import argparse
import hashlib
import io
import importlib.metadata
import json
import subprocess
import tempfile
from pathlib import Path
from PIL import Image, ImageChops, ImageStat
from pypdf import PdfReader, PdfWriter
from pypdf.generic import DecodedStreamObject, DictionaryObject, IndirectObject, NameObject
from verify_book_v2_vector_programs import check_content, render_check


def digest(data):
    return hashlib.sha256(data).hexdigest()


def check_group(reader, metadata, source):
    data = metadata['data']; objects = metadata['objects']
    assert digest(source) == metadata['source_sha256']
    values = []
    for expected in objects:
        obj = reader.get_object(IndirectObject(expected['id'], 0, reader))
        assert not {'/Alt', '/ActualText', '/MCID', '/Lang', '/StructParent'} & set(obj)
        if expected['role'] != 'ExtGState':
            assert digest(obj._data) == expected['payload_sha256']
            assert obj['/Type'] == '/XObject'
        values.append(obj)
    primary = values[0]
    if data['kind'] == 'raster':
        original = Image.open(io.BytesIO(source)); original.load()
        assert original.size == (data['width'], data['height'])
        for i, obj in enumerate(values):
            assert obj['/Subtype'] == '/Image'
            assert (int(obj['/Width']), int(obj['/Height'])) == original.size
            assert int(obj['/BitsPerComponent']) == 8
            mode = 'L' if i or data['color'] == 'Gray' else 'RGB'
            assert obj['/ColorSpace'] == ('/DeviceGray' if mode == 'L' else '/DeviceRGB')
            if i == 0 and data['encoding'] == 'Jpeg':
                assert obj['/Filter'] == '/DCTDecode' and not data['alpha']
                assert int(obj['/DecodeParms']['/ColorTransform']) == int(mode == 'RGB')
                decoded = Image.open(io.BytesIO(obj._data)); decoded.load()
                assert decoded.convert(mode).tobytes() == original.convert(mode).tobytes()
            else:
                assert obj['/Filter'] == '/FlateDecode' and '/DecodeParms' not in obj
                pixels = original.convert('RGBA').getchannel('A').tobytes() if i else original.convert(mode).tobytes()
                assert obj.get_data() == pixels
        assert len(values) == 1 + data['alpha']
        if data['alpha']:
            assert primary.raw_get('/SMask').idnum == objects[1]['id']
            assert '/SMask' not in values[1]
        else:
            assert '/SMask' not in primary
        return primary, (data['width'], data['height'])
    assert data['kind'] == 'vector'
    assert primary['/Subtype'] == '/Form' and int(primary['/FormType']) == 1
    assert [float(v) for v in primary['/BBox']] == [v/65536 for v in data['bbox']]
    assert '/Filter' not in primary
    operations, states = check_content(primary.get_data(), data['ir'])
    assert states == [tuple(s) for s in data['states']]
    assert set(primary['/Resources']) == {'/ExtGState'}
    resources = primary['/Resources']['/ExtGState']
    assert set(resources) == {'/GS'+str(i) for i in range(len(states))}
    assert len(values) == len(states)+1
    for i, ((fill, stroke), obj, expected) in enumerate(zip(states, values[1:], objects[1:])):
        assert expected['role'] == 'ExtGState' and expected['state'] == i
        assert resources.raw_get('/GS'+str(i)).idnum == expected['id']
        assert set(obj) == {'/Type', '/ca', '/CA'} and obj['/Type'] == '/ExtGState'
        assert float(obj['/ca']) == fill/65536 and float(obj['/CA']) == stroke/65536
    return primary, (data['bbox'][2]/65536, data['bbox'][3]/65536)


def write_page(obj, size, vector, path):
    writer = PdfWriter(); width, height = size
    page = writer.add_blank_page(width, height)
    clone = obj.clone(writer)
    page[NameObject('/Resources')] = DictionaryObject({NameObject('/XObject'): DictionaryObject({NameObject('/I'): writer._add_object(clone)})})
    content = DecodedStreamObject()
    matrix = f'1 0 0 -1 0 {height:.16f}' if vector else f'{width:.16f} 0 0 {height:.16f} 0 0'
    content.set_data(f'q {matrix} cm 0 g 0 G /I Do Q'.encode('ascii'))
    page[NameObject('/Contents')] = writer._add_object(content)
    with path.open('wb') as out:
        writer.write(out)


def raster_render(source, pdf, path, mutool):
    original = Image.open(io.BytesIO(source)).convert('RGBA')
    for dpi in (144, 288):
        png = path/f'raster-{dpi}.png'
        subprocess.run([mutool, 'draw', '-q', '-A', '0', '-r', str(dpi), '-c', 'rgb', '-o', str(png), str(pdf)], check=True, capture_output=True)
        actual = Image.open(png).convert('RGB')
        white = Image.new('RGBA', original.size, (255,255,255,255))
        expected = Image.alpha_composite(white, original).convert('RGB').resize(actual.size, Image.Resampling.NEAREST)
        diff = ImageChops.difference(actual, expected)
        mean = sum(ImageStat.Stat(diff).mean)/3
        fraction = sum(max(p)>32 for p in diff.get_flattened_data())/(diff.width*diff.height)
        assert mean <= 2 and fraction <= 0.03, (mean, fraction)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--mutool'); parser.add_argument('--rsvg-convert')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args(); assert bool(args.mutool) == bool(args.rsvg_convert)
    totals = {'pdfs': 0, 'raster': 0, 'png': 0, 'jpeg': 0, 'vector': 0, 'soft_masks': 0, 'alpha_states': 0, 'renders': 0, 'tamper_rejections': 0}
    totals['parser_versions'] = {name: importlib.metadata.version(name) for name in ('pypdf', 'Pillow')}
    if args.mutool:
        totals['renderer_versions'] = {}
        for name, command, version in [('mutool',[args.mutool,'-v'],'mutool version 1.28.2'),('rsvg-convert',[args.rsvg_convert,'--version'],'rsvg-convert version 2.62.3')]:
            p = subprocess.run(command,check=True,capture_output=True,text=True)
            actual = (p.stdout+p.stderr).strip().splitlines()[0]; assert actual == version
            totals['renderer_versions'][name] = actual
    for pdf_path in sorted(args.directory.glob('*.pdf')):
        reader = PdfReader(pdf_path, strict=True); totals['pdfs'] += 1
        assert len(reader.pages) == 1 and '/Contents' not in reader.pages[0]
        seen = {1,2,3}
        for path in sorted(args.directory.glob(pdf_path.stem+'.*.json')):
            metadata = json.loads(path.read_text()); source = path.with_suffix('.source').read_bytes()
            obj, size = check_group(reader, metadata, source)
            vector = metadata['data']['kind'] == 'vector'
            totals['vector' if vector else 'raster'] += 1
            if not vector:
                totals['jpeg' if metadata['data']['encoding'] == 'Jpeg' else 'png'] += 1
            totals['alpha_states' if vector else 'soft_masks'] += len(metadata['objects'])-1
            for item in metadata['objects']:
                assert item['id'] not in seen; seen.add(item['id'])
            if args.self_test:
                key = '/BBox' if vector else '/Width'
                original = obj[key]
                from pypdf.generic import NumberObject
                obj[NameObject(key)] = NumberObject(0)
                try:
                    check_group(reader, metadata, source)
                except (AssertionError, TypeError):
                    totals['tamper_rejections'] += 1
                else:
                    raise AssertionError('tampered image geometry accepted')
                finally:
                    obj[NameObject(key)] = original
                original = obj._data
                assert original
                obj._data = bytes([original[0] ^ 1]) + original[1:]
                try:
                    check_group(reader, metadata, source)
                except AssertionError:
                    totals['tamper_rejections'] += 1
                else:
                    raise AssertionError('tampered image payload accepted')
                finally:
                    obj._data = original
            if args.mutool:
                with tempfile.TemporaryDirectory(prefix='typaxis-image-object-') as tmp:
                    directory = Path(tmp); pdf = directory/'probe.pdf'; write_page(obj,size,vector,pdf)
                    if vector:
                        svg = directory/'source.svg'; svg.write_bytes(source)
                        render_check(svg,pdf,directory,args.mutool,args.rsvg_convert)
                    else:
                        raster_render(source,pdf,directory,args.mutool)
                    totals['renders'] += 2
        assert set(reader.xref[0]) == seen
    assert totals['pdfs'] and totals['png'] and totals['jpeg'] and totals['vector'] and totals['soft_masks'] and totals['alpha_states']
    print(json.dumps(totals))


if __name__ == '__main__':
    main()
