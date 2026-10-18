#!/usr/bin/env python3
"""Compare diagnostic scale PDFs with original engine SVGs at fixed DPI.

Observed PDF placements align the rendering comparison; they are not independent
layout goldens. The public source/paint/extraction verifier runs before rendering.
"""
import argparse
import copy
import hashlib
import io
import json
import subprocess
import tempfile
from decimal import Decimal, ROUND_CEILING, ROUND_FLOOR
from pathlib import Path
from xml.etree import ElementTree as ET

from PIL import Image, ImageChops
from pypdf import PdfReader, PdfWriter
from pypdf.generic import ArrayObject, ContentStream, DecodedStreamObject, FloatObject, NameObject, NumberObject
from prepare_vmb_distinct_probe import verify as verify_corpus
from verify_vmb_scale_pdf import fixed, multiply, require, verify
from vmb_probe_io import contained_bytes, same_json, strict_json
from vmb_svg_reference import reference_page, source_form


DPIS = (72, 144, 288)
REFERENCE_SCALE = 4096
POLICY = {'algorithm': 'typaxis.vmb-scale-raw-svg-render/5',
          'render_dpis': list(DPIS), 'ink_luminance_below': 128,
          'maximum_distance_pixels': 1, 'distance_metric': 'euclidean',
          'maximum_area_difference': 'max(4 pixels, reference ink / 100)',
          'outer_ring_pixels': 2, 'automatic_alignment_or_resizing': False,
          'reference_internal_vector_scale': REFERENCE_SCALE,
          'reference_canvas': 'integer point dimensions, exact uniform source transform, source viewport clip',
          'reference_physical_point_grid': 'pdf_point_1_65536, nearest ties-to-even',
          'reference_pipeline': 'original SVG -> integer high-precision librsvg canvas with uniform source transform -> local source Form -> original page/view transforms -> same PDF renderer as actual PDF'}
SVG = 'http://www.w3.org/2000/svg'


def local(node):
    return node.tag.rsplit('}', 1)[-1]


def paths(root):
    result = []
    arities = {'M': 2, 'L': 2, 'C': 6, 'Z': 0}
    for child in root:
        if local(child) == 'metadata':
            continue
        require(local(child) == 'g' and child.attrib == {'fill': 'currentColor'},
                'original engine SVG group vocabulary')
        for node in child:
            require(local(node) == 'path' and set(node.attrib) == {'d'} and not len(node),
                    'original engine SVG path vocabulary')
            tokens = node.attrib['d'].split()
            parsed, offset = [], 0
            while offset < len(tokens):
                command = tokens[offset]
                require(command in arities, 'original engine explicit path command')
                count = arities[command]
                operands = tokens[offset+1:offset+count+1]
                require(len(operands) == count, 'original engine complete operands')
                values = tuple(Decimal(value) for value in operands)
                require(all(v.is_finite() for v in values), 'original engine finite coordinates')
                parsed.append((command, values))
                offset += count+1
            require(bool(parsed) and parsed[0][0] == 'M', 'original engine nonempty path')
            result.append(parsed)
    require(bool(result), 'original engine nonempty SVG')
    return result


def original_svg(source, derived, case, *, expected_font_size_raw=11*65536):
    raw, converted = ET.fromstring(source), ET.fromstring(derived)
    require(local(raw) == 'svg', 'original engine SVG root')
    require(set(raw.attrib) == {'role', 'aria-label', 'lang', 'dir', 'data-math-display',
            'data-math-baseline', 'data-math-origin-x', 'data-math-advance',
            'viewBox', 'width', 'height'}, 'original engine root vocabulary')
    require(raw.attrib['role'] == 'img' and raw.attrib['lang'] == 'en'
            and raw.attrib['dir'] == 'ltr' and raw.attrib['aria-label'] == case['speech']
            and raw.attrib['data-math-display'] == case['display'], 'original engine semantics')
    metadata = [n for n in raw if local(n) == 'metadata']
    require(len(metadata) == 1 and metadata[0].attrib == {'data-format': 'vmb-math-metadata-v2'}
            and metadata[0].text == case['tex'] and not len(metadata[0]), 'original engine TeX')
    box = tuple(map(Decimal, raw.attrib['viewBox'].split()))
    require(len(box) == 4 and box[:2] == (0, 0) and min(box[2:]) > 0,
            'original engine zero-origin viewport')
    for key, dimension in zip(['width', 'height'], box[2:]):
        value = raw.attrib[key]
        require(value.endswith('em') and Decimal(value[:-2])*65536 == dimension,
                'original engine em dimensions')
    require(type(expected_font_size_raw) is int and expected_font_size_raw > 0
            and type(case['font_size_raw']) is int and case['font_size_raw'] == expected_font_size_raw,
            'original engine expected physical font size')
    scale = Decimal(case['font_size_raw'])/(65536*65536)
    width, height = box[2]*scale, box[3]*scale
    baseline = Decimal(raw.attrib['data-math-baseline'])*scale
    metrics = {'advance': fixed(Decimal(raw.attrib['data-math-advance'])*scale),
               'ascent': fixed(baseline), 'descent': fixed(height-baseline),
               'origin_x': fixed(-Decimal(raw.attrib['data-math-origin-x'])*scale),
               'baseline': fixed(baseline),
               'viewport': {'width': fixed(width), 'height': fixed(height)}}
    require(same_json(metrics, case['metrics']), 'original engine metrics conversion')
    before, after = paths(raw), paths(converted)
    require(len(before) == len(after) == case['path_count'], 'original engine path inventory')
    segments = 0
    for source_path, derived_path in zip(before, after):
        require(len(source_path) == len(derived_path), 'original engine segment inventory')
        for (command, operands), (other_command, other_operands) in zip(source_path, derived_path):
            require(command == other_command and len(operands) == len(other_operands)
                    and all(abs(fixed(value*scale)-fixed(other)) <= 1
                            for value, other in zip(operands, other_operands)),
                    'original engine path conversion within one raw unit')
            segments += 1
    require(segments == case['segment_count'], 'original engine source segment count')
    reference = ET.Element('{'+SVG+'}svg', {'viewBox': raw.attrib['viewBox'],
        'width': str(width), 'height': str(height), 'color': 'black', 'overflow': 'hidden'})
    for node in raw:
        if local(node) != 'metadata':
            reference.append(copy.deepcopy(node))
    return reference, {'paths': len(before), 'segments': segments}


def ink(image):
    require(image.mode in ('L', 'RGB', 'RGBA'), 'known raster color format')
    if image.mode == 'RGBA':
        background = Image.new('RGBA', image.size, 'white')
        background.alpha_composite(image)
        image = background.convert('RGB')
    return image.convert('L').point(lambda value: 255 if value < 128 else 0)


def dilate_one(mask):
    """Euclidean distance <= 1, without wraparound at image boundaries."""
    result = mask.copy()
    for offset in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
        shifted = Image.new('L', mask.size)
        shifted.paste(mask, offset)
        result = ImageChops.lighter(result, shifted)
    return result


def compare_masks(reference, observed, inner):
    require(reference.size == observed.size and reference.mode == observed.mode == 'L',
            'unaltered equal mask dimensions')
    missing = ImageChops.subtract(reference, dilate_one(observed))
    extra = ImageChops.subtract(observed, dilate_one(reference))
    reference_ink = reference.crop(inner).histogram()[255]
    observed_ink = observed.crop(inner).histogram()[255]
    area_difference = abs(reference_ink-observed_ink)
    result = {'reference_ink': reference_ink, 'observed_ink': observed_ink,
              'area_difference': area_difference,
              'missing_over_one_pixel': missing.histogram()[255],
              'extra_over_one_pixel': extra.histogram()[255]}
    result['passed'] = mask_decision(result)
    return result


def mask_decision(counts):
    """Apply the fixed rule to captured counts without regenerating pixels.

    Matching empty masks can occur for subpixel strokes. The required source
    path/paint checks still run; a nonempty reference with empty PDF ink fails.
    """
    return ((counts['reference_ink'] == 0 or counts['observed_ink'] > 0)
            and 100*counts['area_difference'] <= max(400, counts['reference_ink'])
            and not counts['missing_over_one_pixel'] and not counts['extra_over_one_pixel'])


def crop_box(bounds, page_height, dpi):
    x0, y0, x1, y1 = (Decimal(v)/65536 for v in bounds)
    scale = Decimal(dpi)/72
    rect = (x0*scale, (page_height-y1)*scale, x1*scale, (page_height-y0)*scale)
    return tuple(int(v.to_integral_value(rounding=ROUND_FLOOR if i < 2 else ROUND_CEILING))
                 for i, v in enumerate(rect))


def command(argv, log):
    process = subprocess.run(argv, capture_output=True)
    record = {'argv': [str(a) for a in argv], 'exit_code': process.returncode,
              'stdout_bytes': len(process.stdout),
              'stdout_sha256': hashlib.sha256(process.stdout).hexdigest(),
              'stderr_bytes': len(process.stderr),
              'stderr_sha256': hashlib.sha256(process.stderr).hexdigest()}
    log.append(record)
    require(process.returncode == 0, 'renderer failed: '+str(argv[0])+': '
            +process.stderr.decode('utf-8', errors='replace')[-2000:])
    allowed = {b'', b'warning: ICC support is not available\n'} if Path(argv[0]).name == 'mutool' else {b''}
    require(process.stderr in allowed and not process.stdout, 'unexpected renderer output: '
            +process.stderr.decode('utf-8', errors='replace')[-2000:])
    return process.stdout


def normalize_reference(source, output, width, height):
    """Undo a fixed vector scale without resampling or moving raster pixels.

    Cairo quantizes unscaled paths more coarsely than 1/65536 pt. A 4096x
    vector canvas represents this corpus's 11/65536-pt engine coordinates on
    Cairo's grid. The exact inverse CTM restores the original physical page.
    Content bytes and painting order remain unchanged inside that CTM.
    """
    require(source.resolve() != output.resolve(), 'reference normalization preserves its source')
    reader = PdfReader(source, strict=True)
    require(len(reader.pages) == 1, 'one vector reference page')
    page = reader.pages[0]
    require(tuple(fixed(Decimal(str(v))/REFERENCE_SCALE) for v in page.mediabox)
            == (0, 0, fixed(width), fixed(height)),
            'reference internal vector canvas')
    require(all(k not in page for k in ('/CropBox', '/BleedBox', '/TrimBox', '/ArtBox',
                                       '/Rotate', '/UserUnit', '/Annots')),
            'plain vector reference page')
    stream = DecodedStreamObject()
    stream.set_data(b'q\n0.000244140625 0 0 0.000244140625 0 0 cm\n'
                    +page.get_contents().get_data()+b'\nQ\n')
    page[NameObject('/Contents')] = stream
    page[NameObject('/MediaBox')] = ArrayObject([NumberObject(0), NumberObject(0),
                                                FloatObject(str(width)), FloatObject(str(height))])
    writer = PdfWriter()
    writer.pdf_header = reader.pdf_header
    writer.add_page(page)
    with output.open('wb') as handle:
        writer.write(handle)


def precise_matrices(pdf):
    """Read actual unit-fraction PDF matrices without FloatObject's short str()."""
    reader = PdfReader(pdf, strict=True)
    result = []
    for page in reader.pages:
        matrix = tuple(map(Decimal, (1, 0, 0, 1, 0, 0)))
        stack, rows = [], []
        for args, operator in ContentStream(page.get_contents(), reader).operations:
            if operator == b'q':
                stack.append(matrix)
            elif operator == b'Q':
                require(bool(stack), 'precise PDF matrix stack')
                matrix = stack.pop()
            elif operator == b'cm':
                values = tuple(Decimal(repr(float(v))) if isinstance(v, FloatObject)
                               else Decimal(int(v)) for v in args)
                matrix = multiply(matrix, values)
            elif operator == b'Do':
                rows.append(matrix)
        require(not stack, 'balanced precise PDF matrix stack')
        result.append(rows)
    return result


def render_pages(job, corpus, pdf, placements, output, selected_pages=None, tool_paths=None):
    """Render all selected source views at their already checked PDF positions."""
    require(not output.resolve().is_relative_to(job.resolve())
            and not output.resolve().is_relative_to(corpus.resolve()), 'render output outside inputs')
    inventory = strict_json(contained_bytes(job, 'probe-inventory.json'))
    index, facts = verify_corpus(corpus)
    originals, original_counts = [], []
    for case in index['cases']:
        reference, counts = original_svg(contained_bytes(corpus, case['source_svg']),
                                         contained_bytes(corpus, case['derived_svg']), case)
        originals.append(reference)
        original_counts.append(counts)
    return render_source_pages(job, pdf, placements, output, index, inventory, originals,
                               original_counts, facts['index_sha256'], selected_pages, tool_paths)


def render_source_pages(job, pdf, placements, output, index, inventory, originals,
                        original_counts, corpus_sha256, selected_pages=None, tool_paths=None):
    """Shared fixed-policy raster kernel after independent source validation."""
    require(not output.resolve().is_relative_to(job.resolve()), 'render output outside inputs')
    require(len(index['cases']) == len(originals) == len(original_counts) and bool(originals),
            'complete original-SVG reference inventory')
    output.mkdir()  # Exclusive; never modify earlier evidence.
    tools = {'mutool': '/usr/bin/mutool', 'pdftoppm': '/usr/bin/pdftoppm',
             'rsvg-convert': '/usr/bin/rsvg-convert'}
    require(tool_paths is None or set(tool_paths) <= set(tools), 'known rendering tool roles')
    tools.update(tool_paths or {})
    tool_facts = {}
    for name, value in tools.items():
        path = Path(value).resolve(strict=True)
        require(path.is_file(), 'renderer executable file')
        version = subprocess.run([str(path), '--version' if name == 'rsvg-convert' else '-v'],
                                 capture_output=True, check=True)
        tool_facts[name] = {'path': str(path), 'bytes': path.stat().st_size,
                           'sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
                           'version_stdout': version.stdout.decode('utf-8'),
                           'version_stderr': version.stderr.decode('utf-8')}
        tools[name] = str(path)
    package_bytes = contained_bytes(job, 'document-package.json')
    package = strict_json(package_bytes)
    require(placements['algorithm'] == 'typaxis.vmb-scale-observed-placements/1'
            and placements['source_package_sha256'] == hashlib.sha256(package_bytes).hexdigest()
            and placements['pdf_sha256'] == hashlib.sha256(pdf.read_bytes()).hexdigest(),
            'raster source/package/PDF binding')
    page_count = len(placements['pages'])
    pages = list(range(page_count)) if selected_pages is None else list(selected_pages)
    require(bool(pages) and len(set(pages)) == len(pages)
            and all(type(p) is int and 0 <= p < page_count for p in pages), 'nonempty page selection')
    grouped = [[] for _ in range(page_count)]
    for row in placements['placements']:
        grouped[row['page_index']].append(row)
    require(len(placements['placements']) == inventory['placements'], 'full observed occurrence inventory')
    exact = precise_matrices(pdf)
    require(len(exact) == page_count, 'precise PDF page count')
    for page_index, rows in enumerate(grouped):
        require(len(rows) == len(exact[page_index]), 'precise PDF placement count')
        for row, matrix in zip(rows, exact[page_index]):
            require(all(abs(fixed(v)-fixed(Decimal(w))) <= 1 for v, w in zip(matrix, row['matrix'])),
                    'precise PDF matrices match checked placement evidence')
    master = package['page_masters']['masters'][0]
    images = package['resources']['images']
    logs, results, failures = [], [], []
    form_cache, image_cache, page_cache = {}, {}, {}
    captured_failures = 0
    with tempfile.TemporaryDirectory(prefix='render-', dir=output) as temporary:
        scratch = Path(temporary)
        def form_for(image_id):
            case_index = image_id % 100 if inventory['case'] == 'alias' else image_id
            if case_index not in form_cache:
                viewport = index['cases'][case_index]['metrics']['viewport']
                form_cache[case_index] = source_form(originals[case_index],
                    Decimal(viewport['width'])/65536, Decimal(viewport['height'])/65536,
                    scratch, 'source-'+str(case_index), tools['rsvg-convert'], lambda argv: command(argv, logs),
                    source_to_reference_scale=Decimal(index['cases'][case_index]['font_size_raw'])*REFERENCE_SCALE/(65536*65536))
            return form_cache[case_index]

        def image_for(image_id):
            if image_id not in image_cache:
                data = contained_bytes(job, images[image_id]['uri'])
                with Image.open(io.BytesIO(data)) as image:
                    image.load()
                    require(image.format == 'PNG' and image.mode == 'RGB' and image.size == (2, 2),
                            'opaque source PNG reference')
                    stream = DecodedStreamObject()
                    stream.set_data(image.tobytes())
                stream.update({NameObject('/Type'): NameObject('/XObject'),
                    NameObject('/Subtype'): NameObject('/Image'),
                    NameObject('/Width'): NumberObject(2), NameObject('/Height'): NumberObject(2),
                    NameObject('/ColorSpace'): NameObject('/DeviceRGB'),
                    NameObject('/BitsPerComponent'): NumberObject(8)})
                image_cache[image_id] = stream.flate_encode()
            return image_cache[image_id]

        for dpi in DPIS:
            for page_index in pages:
                page = placements['pages'][page_index]
                width, height = Decimal(master['width'])/65536, Decimal(master['height'])/65536
                require(tuple(map(fixed, map(Decimal, page['media_box'])))
                        == (0, 0, master['width'], master['height']), 'render source page geometry')
                if page_index not in page_cache:
                    page_cache[page_index] = scratch/('reference-'+str(page_index)+'.pdf')
                    reference_page(page_cache[page_index], width, height, grouped[page_index],
                                   exact[page_index], form_for, image_for)
                reference_pdf = page_cache[page_index]
                reference_reader = PdfReader(reference_pdf, strict=True)
                require(len(reference_reader.pages) == 1
                        and tuple(map(fixed, reference_reader.pages[0].mediabox))
                        == (0, 0, fixed(width), fixed(height)), 'reference physical page geometry')
                for renderer in ['MuPDF', 'Poppler']:
                    path = scratch/'observed.png'
                    reference_path = scratch/'reference.png'
                    if renderer == 'MuPDF':
                        argv = [tools['mutool'], 'draw', '-q', '-r', str(dpi), '-c', 'rgb',
                                '-o', str(path), str(pdf), str(page_index+1)]
                        reference_argv = [tools['mutool'], 'draw', '-q', '-r', str(dpi), '-c', 'rgb',
                                          '-o', str(reference_path), str(reference_pdf), '1']
                    else:
                        argv = [tools['pdftoppm'], '-f', str(page_index+1), '-l', str(page_index+1),
                                '-r', str(dpi), '-singlefile', '-png', str(pdf), str(path.with_suffix(''))]
                        reference_argv = [tools['pdftoppm'], '-f', '1', '-l', '1', '-r', str(dpi),
                                          '-singlefile', '-png', str(reference_pdf),
                                          str(reference_path.with_suffix(''))]
                    command(reference_argv, logs)
                    with Image.open(reference_path) as image:
                        ref_mask = ink(image)
                    command(argv, logs)
                    with Image.open(path) as image:
                        observed = ink(image)
                    require(observed.size == ref_mask.size, 'renderer page pixels must match without resizing')
                    for row in grouped[page_index]:
                        if row['kind'] == 'figure':
                            continue  # This gate owns formula masks; decoded PNGs have a separate gate.
                        inner = crop_box(row['bounds_raw'], height, dpi)
                        box = (inner[0]-2, inner[1]-2, inner[2]+2, inner[3]+2)
                        require(box[0] >= 0 and box[1] >= 0 and box[2] <= ref_mask.width
                                and box[3] <= ref_mask.height, 'complete outer ring inside page')
                        local_inner = (2, 2, box[2]-box[0]-2, box[3]-box[1]-2)
                        ref_crop, observed_crop = ref_mask.crop(box), observed.crop(box)
                        comparison = compare_masks(ref_crop, observed_crop, local_inner)
                        result = {'ordinal': row['ordinal'], 'page_index': page_index,
                                  'renderer': renderer, 'dpi': dpi, 'crop_pixels': box,
                                  'reference_mask_sha256': hashlib.sha256(ref_crop.tobytes()).hexdigest(),
                                  'observed_mask_sha256': hashlib.sha256(observed_crop.tobytes()).hexdigest(),
                                  **comparison}
                        results.append(result)
                        if not comparison['passed']:
                            failures.append(result)
                            if captured_failures < 12:
                                stem = f'failure-{dpi}-{renderer}-{row["ordinal"]}'
                                ref_crop.save(output/(stem+'-reference.png'))
                                observed_crop.save(output/(stem+'-observed.png'))
                                captured_failures += 1
                    path.unlink()
                    reference_path.unlink()
    summary = {'algorithm': POLICY['algorithm'], 'scope': 'Original SVG raster at observed placements, not independent layout.',
               'case': inventory['case'], 'policy': POLICY, 'pdf_sha256': placements['pdf_sha256'],
               'source_package_sha256': placements['source_package_sha256'],
               'corpus_sha256': corpus_sha256,
               'selected_pages': pages, 'all_case_pages': pages == list(range(page_count)),
               'original_geometry_cases': len(originals), 'original_geometry_counts': original_counts,
               'source_reference_forms': len(form_cache), 'source_reference_images': len(image_cache),
               'tools': tool_facts,
               'comparisons': len(results), 'failed_comparisons': len(failures),
               'passed': not failures, 'commands': logs}
    (output/'comparisons.json').write_text(json.dumps(results, separators=(',', ':'))+'\n')
    (output/'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
    require(not failures, f'{len(failures)} original-SVG formula raster comparisons failed')
    return {k: v for k, v in summary.items() if k not in ('commands', 'original_geometry_counts', 'selected_pages')}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('job', type=Path)
    parser.add_argument('corpus', type=Path)
    parser.add_argument('pdf', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--mutool', type=Path, default=Path('/usr/bin/mutool'))
    parser.add_argument('--pdftoppm', type=Path, default=Path('/usr/bin/pdftoppm'))
    parser.add_argument('--pdftotext', type=Path, default=Path('/usr/bin/pdftotext'))
    parser.add_argument('--rsvg-convert', type=Path, default=Path('/usr/bin/rsvg-convert'))
    args = parser.parse_args()
    require(not args.output.resolve().is_relative_to(args.job.resolve())
            and not args.output.resolve().is_relative_to(args.corpus.resolve()), 'render output outside inputs')
    with tempfile.TemporaryDirectory(prefix='vmb-render-placements-') as temporary:
        evidence = Path(temporary)/'placements.json'
        verify(args.job, args.pdf, evidence,
               extraction_tools={'mutool': str(args.mutool), 'pdftotext': str(args.pdftotext)})
        placements = strict_json(evidence.read_bytes())
        print(json.dumps(render_pages(args.job, args.corpus, args.pdf, placements, args.output,
              tool_paths={'mutool': str(args.mutool), 'pdftoppm': str(args.pdftoppm),
                          'rsvg-convert': str(args.rsvg_convert)}), indent=2))
