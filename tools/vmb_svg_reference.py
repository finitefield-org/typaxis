"""Build source SVG references with local viewports and exact PDF coordinates.

The source vocabulary is the opaque path-only engine scale corpus. Cairo is an
independent SVG interpreter. Keep each source viewport separate so PDF renderers
see the same physical coordinate context, without any pixel alignment search.
"""
import copy
from decimal import Decimal, ROUND_CEILING, ROUND_HALF_EVEN
from xml.etree import ElementTree as ET

from pypdf import PdfReader, PdfWriter
from pypdf.generic import (ArrayObject, ContentStream, DecodedStreamObject,
                           DictionaryObject, FloatObject, NameObject, NumberObject)
from verify_vmb_scale_pdf import fixed, multiply, require


REFERENCE_SCALE = Decimal(4096)


class ExactNumber(FloatObject):
    """Serialize the fixed physical grid without FloatObject's short format."""
    def __new__(cls, value):
        result = super().__new__(cls, str(value))
        result.exact = Decimal(value)
        return result

    def write_to_stream(self, stream, encryption_key=None):
        stream.write(format(self.exact, 'f').encode('ascii'))


def pdf_decimal(value):
    return Decimal(repr(float(value))) if isinstance(value, FloatObject) else Decimal(int(value))


def physical(value):
    return ExactNumber((value/REFERENCE_SCALE*65536).to_integral_value(
        rounding=ROUND_HALF_EVEN)/65536)


def exact_raw(value):
    return Decimal(fixed(value))/65536


def local_form(reader, width, height, *, canvas_size=None):
    """Keep SVG's local downward axis; the reference page supplies its transform."""
    require(len(reader.pages) == 1, 'one original-SVG reference page')
    page = reader.pages[0]
    if canvas_size is None:
        require(tuple(fixed(pdf_decimal(v)/REFERENCE_SCALE) for v in page.mediabox)
                == (0, 0, fixed(width), fixed(height)), 'source viewport reference dimensions')
        canvas_height = height*REFERENCE_SCALE
    else:
        require(tuple(map(pdf_decimal,page.mediabox)) == (0,0,*canvas_size),
                'integer source reference canvas dimensions')
        canvas_height = canvas_size[1]
    content = ContentStream(page.get_contents(), reader)
    first_args, first_op = content.operations[0]
    require(first_op == b'cm' and list(map(pdf_decimal, first_args[:5])) == [1, 0, 0, -1, 0]
            and len(first_args) == 6 and pdf_decimal(first_args[5]) == canvas_height,
            'Cairo reference page coordinate frame')
    matrix = tuple(map(Decimal, (1, 0, 0, 1, 0, 0)))
    stack, operations = [], []

    def point(x, y):
        a, b, c, d, e, f = matrix
        return [physical(a*x+c*y+e), physical(b*x+d*y+f)]

    for args, op in content.operations[1:]:
        if op == b'cm':
            require(len(args) == 6, 'reference affine matrix')
            matrix = multiply(matrix, tuple(map(pdf_decimal, args)))
            continue
        if op == b'q':
            stack.append(matrix)
        elif op == b'Q':
            require(bool(stack), 'reference graphics-state restore')
            matrix = stack.pop()
        elif op in (b'rg', b'g'):
            require(all(pdf_decimal(v) == 0 for v in args), 'opaque black source reference')
            continue
        elif op == b'gs':
            require(len(args) == 1, 'reference graphics-state resource')
            state = page['/Resources']['/ExtGState'][args[0]].get_object()
            require(set(state) <= {'/Type', '/ca', '/CA', '/BM', '/SMask'}
                    and state.get('/ca', 1) == state.get('/CA', 1) == 1
                    and state.get('/BM', '/Normal') == '/Normal'
                    and state.get('/SMask', '/None') == '/None', 'opaque normal reference state')
        elif op in (b'm', b'l', b'c'):
            values = list(map(pdf_decimal, args))
            require(len(values) == (6 if op == b'c' else 2), 'reference path operands')
            args = [v for i in range(0, len(values), 2) for v in point(values[i], values[i+1])]
        elif op == b're':
            require(len(args) == 4, 'reference rectangle operands')
            x, y, w, h = map(pdf_decimal, args)
            corners = [point(x, y), point(x+w, y), point(x+w, y+h), point(x, y+h)]
            # The source SVG uses explicit paths. Undo Cairo's rectangle shortcut
            # and retain the source's closed polygon semantics in its local frame.
            operations.extend([(corners[0], b'm'), (corners[1], b'l'), (corners[2], b'l'),
                               (corners[3], b'l'), ([], b'h')])
            continue
        elif op in (b'BDC', b'BMC', b'EMC'):
            continue  # Reference metadata is not an accessibility oracle.
        else:
            require(op in (b'h', b'f', b'f*', b'n', b'W', b'W*'), 'reference fill vocabulary')
        if (op in (b'f', b'f*') and len(operations) > 1 and operations[-1][1] == b'm'
                and operations[-2][1] == b'h'):
            operations.pop()  # An empty trailing subpath adds no SVG paint.
        operations.append((args, op))
    require(not stack, 'balanced source reference graphics state')
    content.operations = operations
    form = DecodedStreamObject()
    form.set_data(f'q\n0 0 {width} {height} re W n\n'.encode('ascii')+content.get_data()+b'\nQ\n')
    form.update({NameObject('/Type'): NameObject('/XObject'), NameObject('/Subtype'): NameObject('/Form'),
                 NameObject('/FormType'): NumberObject(1),
                 NameObject('/BBox'): ArrayObject([NumberObject(0), NumberObject(0),
                                                   ExactNumber(width), ExactNumber(height)]),
                 NameObject('/Resources'): page['/Resources']})
    return form.flate_encode()


def source_form(svg, width, height, scratch, stem, rsvg, run, *, source_to_reference_scale=None):
    box = tuple(map(Decimal,svg.attrib['viewBox'].split()))
    require(len(box) == 4 and box[:2] == (0,0) and min(box[2:]) > 0, 'source reference zero-origin viewport')
    source_width = Decimal(svg.attrib['width'].removesuffix('pt'))
    source_height = Decimal(svg.attrib['height'].removesuffix('pt'))
    require(fixed(source_width) == fixed(width) and fixed(source_height) == fixed(height),
            'source reference physical viewport')
    scale = source_to_reference_scale
    if scale is None:
        scale = source_width*REFERENCE_SCALE/box[2]
    require(scale.is_finite() and scale > 0
            and abs(fixed(box[2]*scale/REFERENCE_SCALE)-fixed(source_width)) <= 1
            and abs(fixed(box[3]*scale/REFERENCE_SCALE)-fixed(source_height)) <= 1,
            'exact uniform source reference transform')
    canvas = tuple(max(v*scale,w*REFERENCE_SCALE).to_integral_value(rounding=ROUND_CEILING)
                   for v,w in zip(box[2:],(width,height)))
    require(min(canvas) > 0 and max(canvas) <= 2**24, 'bounded exactly representable integer reference canvas')
    reference = ET.Element('{http://www.w3.org/2000/svg}svg',{
        'width':str(canvas[0])+'pt','height':str(canvas[1])+'pt',
        'viewBox':f'0 0 {canvas[0]} {canvas[1]}','color':'black','overflow':'hidden'})
    group = ET.SubElement(reference,'{http://www.w3.org/2000/svg}g',
                          {'transform':f'matrix({scale} 0 0 {scale} 0 0)'})
    for child in svg:
        if child.tag.rsplit('}',1)[-1] != 'metadata':
            group.append(copy.deepcopy(child))
    svg_path, pdf_path = scratch/(stem+'.svg'), scratch/(stem+'.pdf')
    svg_path.write_bytes(ET.tostring(reference, encoding='utf-8'))
    run([str(rsvg), '-f', 'pdf', '-d', '72', '-p', '72', '-o', str(pdf_path), str(svg_path)])
    reader = PdfReader(pdf_path, strict=True)
    form = local_form(reader, width, height,canvas_size=canvas)
    svg_path.unlink()
    pdf_path.unlink()
    return form


def reference_page(path, width, height, rows, matrices, form_for, image_for):
    require(len(rows) == len(matrices), 'one reference matrix per source occurrence')
    writer = PdfWriter()
    writer.pdf_header = b'%PDF-1.7'
    page = writer.add_blank_page(width=float(width), height=float(height))
    page[NameObject('/MediaBox')] = ArrayObject([NumberObject(0), NumberObject(0),
                                                ExactNumber(width), ExactNumber(height)])
    objects = DictionaryObject()
    data = [f'q\n1 0 0 -1 0 {height} cm\n0 0 0 rg\n']
    for row, matrix in zip(rows, matrices):
        a, b, c, d, x, y = matrix
        require(b == c == 0, 'source reference axis-aligned placement')
        x, top = exact_raw(x), height-exact_raw(y)
        name = '/R'+str(row['image_id'])
        if row['kind'] == 'figure':
            require(a > 0 and d > 0, 'source reference upright image')
            obj = image_for(row['image_id'])
            values = (exact_raw(a), 0, 0, -exact_raw(d), x, top)
        else:
            require(a == 1 and d == -1, 'source reference unit vector scale')
            obj = form_for(row['image_id'])
            values = (1, 0, 0, 1, x, top)
        if NameObject(name) not in objects:
            objects[NameObject(name)] = writer._add_object(obj.clone(writer))
        data.append('q\n'+' '.join(str(v) for v in values)+' cm\n'+name+' Do\nQ\n')
    data.append('Q\n')
    stream = DecodedStreamObject()
    stream.set_data(''.join(data).encode('ascii'))
    page[NameObject('/Contents')] = writer._add_object(stream.flate_encode())
    page[NameObject('/Resources')] = DictionaryObject({NameObject('/XObject'): objects})
    with path.open('wb') as output:
        writer.write(output)
