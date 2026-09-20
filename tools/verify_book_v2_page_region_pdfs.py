#!/usr/bin/env python3
"""Check running-region probe PDFs against original source and actual resources.

Requires pypdf. FontTools verification is delegated to --font-python. This checks
controlled plain-text fixtures, not arbitrary books or PDF/UA conformance.
"""
import argparse
from collections import Counter
import copy
import hashlib
import io
import json
from pathlib import Path
import subprocess

from pypdf import PdfReader, PdfWriter
from pypdf.generic import (ContentStream, FloatObject,
                           NameObject, NumberObject)
from verify_book_v2_header_pdfs import verify as verify_fonts, mutated as mutate_font
from verify_book_v2_pdf_assembly import original, refid, selected_master, verify_xref


def source_text(region, buffer):
    return ''.join(buffer[item['text_span']['start_byte']:item['text_span']['end_byte']]
                   .decode('utf-8') for block in region['blocks']
                   for item in block['children'] if item['kind'] == 'text')


def verify(path, proof, resources, reader=None):
    if reader is None:
        verify_xref(path.read_bytes())
        reader = PdfReader(path, strict=True)
    verify_fonts(path, resources, reader)
    assert len(reader.pages) == proof['pages']
    assert len(proof['regions']) == 2 * proof['pages']
    assert all(f['source_sha256'] == bytes(proof['font_hash']).hex()
               for f in resources['fonts'])
    font_hashes = [f['subset_sha256'] for f in resources['fonts']]
    body_refs, counts = [], Counter()
    page_ids = {refid(p): i for i, p in enumerate(reader.pages)}

    def structure(value):
        value = value.get_object()
        if isinstance(value, list):
            for child in value:
                structure(child)
        elif isinstance(value, dict):
            if '/S' in value:
                counts[value['/S']] += 1
            if value.get('/Type') == '/MCR':
                body_refs.append((page_ids[refid(value['/Pg'])], int(value['/MCID'])))
            if '/K' in value:
                structure(value['/K'])
    structure(reader.trailer['/Root']['/StructTreeRoot'])
    # This fixture is a section containing plain paragraphs. Running heading
    # and empty-region paragraphs must not introduce extra logical nodes.
    blocks = proof['wire']['document']['blocks']
    assert len(blocks) == 1 and all(b['kind'] == 'paragraph' for b in blocks[0]['blocks'])
    paragraphs = blocks[0]['blocks']
    assert all(len(p['children']) == 1 and p['children'][0]['kind'] == 'text' for p in paragraphs)
    assert counts == Counter({'/Document': 1, '/Sect': 1, '/P': len(paragraphs), '/Span': len(paragraphs)})
    seen_body, paints, roles = [], [], Counter()
    for page_index, page in enumerate(reader.pages):
        master = selected_master(proof['wire']['page_masters'], page_index)
        assert float(page.mediabox.width) == master['width'] / 65536
        assert float(page.mediabox.height) == master['height'] / 65536
        fonts = {}
        for name, ref in page['/Resources']['/Font'].items():
            desc = ref.get_object()['/DescendantFonts'][0].get_object()['/FontDescriptor']
            program = desc['/FontFile3' if '/FontFile3' in desc else '/FontFile2'].get_data()
            fonts[name] = font_hashes.index(hashlib.sha256(program).hexdigest())
        scopes, matrix, font, size = [], None, None, None
        transforms = []
        for args, op in ContentStream(page.get_contents(), reader).operations:
            assert op not in (b'W', b'W*'), 'unexpected clipping'
            if op == b'cm':
                transforms.append(list(map(float, args)))
            elif op in (b'BMC', b'BDC'):
                prop = args[1] if op == b'BDC' else {}
                role = None
                if args[0] == '/Artifact':
                    assert prop.get('/Type') == '/Pagination'
                    assert prop.get('/Subtype') in ('/Header', '/Footer')
                    assert '/MCID' not in prop and '/ActualText' not in prop
                    role = str(prop['/Subtype'])[1:].lower()
                    roles[page_index, role] += 1
                elif '/MCID' in prop:
                    assert not any(scopes)
                    seen_body.append((page_index, int(prop['/MCID'])))
                scopes.append(role)
            elif op == b'EMC':
                assert scopes
                scopes.pop()
            elif op == b'Tf':
                font, size = fonts[args[0]], float(args[1])
            elif op == b'Tm':
                assert list(map(float, args[:4])) == [1, 0, 0, -1]
                matrix = list(map(float, args[4:]))
            elif op == b'Tj':
                raw = original(args[0])
                assert len(raw) == 2 and matrix is not None and scopes
                role = next((r for r in scopes if r), None)
                paints.append((page_index, role, font, int.from_bytes(raw, 'big'), size, matrix))
            else:
                assert op not in (b'TJ', b"'", b'"'), 'unexpected text operator'
        assert not scopes
        assert transforms == [[1, 0, 0, -1, 0, master['height'] / 65536]]
        for role in ('header', 'footer'):
            region = next(r for r in proof['regions'] if r['page'] == page_index and r['role'] == role)
            authored = master[role + '_content']
            assert region['owner'] == authored['node_id']
            assert region['text'] == source_text(authored, proof['source_text'].encode())
            assert bool(roles[page_index, role]) == bool(region['glyphs'])
            expected_glyphs = [(gid, *xy) for u in resources['uses']
                               if (u['page'], u['role']) == (page_index, role)
                               for gid, xy in zip(u['gids'], u['positions'])]
            assert expected_glyphs == [(g['gid'], g['x'], g['y']) for g in region['glyphs']]
            assert ''.join(u['text'] for u in resources['uses']
                           if (u['page'], u['role']) == (page_index, role)) == region['text']
    expected = [(u['page'], u['role'], u['font'], cid, u['size'] / 65536,
                 [v / 65536 for v in xy]) for u in resources['uses']
                for cid, xy in zip(u['cids'], u['positions'])]
    assert len(expected) == len(paints)
    for actual, wanted in zip(paints, expected):
        assert actual[:4] == wanted[:4], ('page/role/font/CID', actual, wanted)
        assert abs(actual[4] - wanted[4]) < 1e-8
        assert all(abs(a - b) < 1e-8 for a, b in zip(actual[5], wanted[5])), 'glyph position'
    assert Counter(seen_body) == Counter(body_refs)
    assert len(seen_body) == len(paragraphs) and len(set(seen_body)) == len(seen_body)
    region_gids = {g for u in resources['uses'] if u['role'] for g in u['gids']}
    body_gids = {g for u in resources['uses'] if not u['role'] for g in u['gids']}
    assert not region_gids or region_gids - body_gids, 'fixture needs a running-only glyph'
    return len(paints)


def mutated(path, mode):
    if mode in ('program', 'cid'):
        return mutate_font(path, mode)
    writer = PdfWriter()
    writer.clone_document_from_reader(PdfReader(path, strict=True))
    changed = False
    if mode == 'structure':
        writer.root_object['/StructTreeRoot'][NameObject('/K')] = NumberObject(0)
        changed = True
    else:
        for page in writer.pages:
            stream = ContentStream(page.get_contents(), writer)
            ops = stream.operations
            for args, op in ops:
                if changed:
                    break
                if mode in ('role', 'mcid') and op == b'BDC' and args[0] == '/Artifact':
                    if mode == 'role':
                        args[1][NameObject('/Subtype')] = NameObject('/Footer' if args[1]['/Subtype'] == '/Header' else '/Header')
                    else:
                        args[1][NameObject('/MCID')] = NumberObject(999)
                    changed = True
                elif mode == 'position' and op == b'Tm':
                    args[4] = FloatObject(float(args[4]) + 1)
                    changed = True
            stream.operations = ops
            page[NameObject('/Contents')] = stream
    assert changed
    data = io.BytesIO()
    writer.write(data)
    return PdfReader(io.BytesIO(data.getvalue()), strict=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('probes', type=Path)
    parser.add_argument('--font-python', default='python3')
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    root = args.probes.resolve()
    paths = sorted(root.glob('*.json'))
    assert paths, 'no page-region probes'
    command = [args.font_python, str(Path(__file__).with_name('verify_book_v2_header_resources.py')),
               str(root / 'resources')]
    if args.self_test:
        command.append('--self-test')
    subprocess.run(command, check=True)
    pages = glyphs = rejected = 0
    for path in paths:
        proof = json.loads(path.read_text())
        resources = json.loads((root / 'resources' / path.name).read_text())
        pdf = root / (path.stem + '.pdf')
        glyphs += verify(pdf, proof, resources)
        pages += proof['pages']
        if args.self_test:
            modes = ['program', 'cid', 'position', 'structure']
            if any(r['glyphs'] for r in proof['regions']):
                modes += ['role', 'mcid']
            for mode in modes:
                reader = mutated(pdf, mode)
                try:
                    verify(pdf, proof, resources, reader)
                except (AssertionError, KeyError, ValueError):
                    rejected += 1
                else:
                    raise AssertionError((path.name, mode, 'modified PDF accepted'))
            for field in ('owner', 'text'):
                bad = copy.deepcopy(proof)
                bad['regions'][0][field] = -1 if field == 'owner' else 'incorrect'
                try:
                    verify(pdf, bad, resources)
                except AssertionError:
                    rejected += 1
                else:
                    raise AssertionError((path.name, field, 'modified source binding accepted'))
    print(f'PASS: {len(paths)} running-region PDFs / {pages} pages / {glyphs} actual glyph paints; '
          f'{rejected} modified PDF/source rejections; original fonts, geometry, Artifact and body structure verified')


if __name__ == '__main__':
    main()
