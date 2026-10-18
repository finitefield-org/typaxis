"""Independent geometry and source-coordinate controls for SVG Form references."""
import tempfile
import unittest
import json
from decimal import Decimal
from pathlib import Path
from xml.etree import ElementTree as ET

from pypdf import PdfReader, PdfWriter
from pypdf.generic import (ContentStream, DecodedStreamObject, DictionaryObject,
                           FloatObject, NameObject, NumberObject)
from prepare_vmb_distinct_probe import png
from verify_vmb_scale_pdf import fixed, verify_form_geometry, verify_raster_pixels
from verify_vmb_scale_render import original_svg, precise_matrices
from vmb_svg_reference import local_form, reference_page, source_form


class SourceViewportTests(unittest.TestCase):
    def test_wide_real_source_uses_an_integer_canvas_and_the_exact_uniform_transform(self):
        corpus = Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book/engine-v2'
        cases = json.loads((corpus/'fixture-index.json').read_bytes())['cases']
        case = next(case for case in cases if case['case_id'] == 'long' and case['display'] == 'inline'
                    and case['font_size_raw'] == 786431)
        svg,_ = original_svg((corpus/case['source_svg']).read_bytes(),(corpus/case['derived_svg']).read_bytes(),
                           case,expected_font_size_raw=786431)
        before = ET.tostring(svg)
        scale = Decimal(786431)*4096/(65536*65536)
        width,height = (Decimal(case['metrics']['viewport'][key])/65536 for key in ('width','height'))
        captured = []
        def renderer(argv):
            document = ET.fromstring(Path(argv[-1]).read_bytes())
            captured.append(document)
            canvas_width,canvas_height = (Decimal(document.attrib[key][:-2]) for key in ('width','height'))
            self.assertEqual(canvas_width,canvas_width.to_integral_value())
            self.assertEqual(canvas_height,canvas_height.to_integral_value())
            writer = PdfWriter()
            page = writer.add_blank_page(width=int(canvas_width),height=int(canvas_height))
            stream = DecodedStreamObject()
            stream.set_data(f'1 0 0 -1 0 {canvas_height} cm q Q'.encode())
            page[NameObject('/Contents')] = writer._add_object(stream)
            writer.write(argv[argv.index('-o')+1])
        with tempfile.TemporaryDirectory() as directory:
            form = source_form(svg,width,height,Path(directory),'wide','synthetic-render-callback',renderer,
                               source_to_reference_scale=scale)
            self.assertEqual(ET.tostring(svg),before)
            self.assertGreater(Decimal(captured[0].attrib['width'][:-2]),1_130_052)
            self.assertEqual(captured[0][0].attrib['transform'],f'matrix({scale} 0 0 {scale} 0 0)')
            self.assertEqual([element.attrib for element in captured[0][0].iter() if element.tag.endswith('path')],
                             [element.attrib for element in svg.iter() if element.tag.endswith('path')])
            self.assertEqual(tuple(form['/BBox']), (0,0,width,height))
            self.assertIn(f'0 0 {width} {height} re W n'.encode(),form.get_data())

    def test_wrong_uniform_scale_or_quantized_integer_canvas_cannot_pass(self):
        svg = ET.fromstring('<svg width="10pt" height="20pt" viewBox="0 0 10 20"/>')
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError,'uniform source reference transform'):
                source_form(svg,Decimal(10),Decimal(20),Path(directory),'bad','unused',lambda _:None,
                            source_to_reference_scale=Decimal(4095))
            reader = self.reader(Path(directory))
            with self.assertRaisesRegex(ValueError,'integer source reference canvas dimensions'):
                local_form(reader,Decimal(10),Decimal(20),canvas_size=(Decimal(40961),Decimal(81920)))

    def reader(self, root, content=None, opacity=1):
        writer = PdfWriter()
        page = writer.add_blank_page(width=40960, height=81920)
        state = DictionaryObject({NameObject('/Type'): NameObject('/ExtGState'),
            NameObject('/ca'): FloatObject(opacity), NameObject('/CA'): NumberObject(1)})
        page[NameObject('/Resources')] = DictionaryObject({NameObject('/ExtGState'):
            DictionaryObject({NameObject('/A'): writer._add_object(state)})})
        stream = DecodedStreamObject()
        stream.set_data(content or b'1 0 0 -1 0 81920 cm q 0 0 0 rg /A gs '
                        b'4096 8192 m 16384 8192 l 16384 20480 l 4096 20480 l h '
                        b'4096 8192 m f 1024 3072 2048 4096 re f Q')
        page[NameObject('/Contents')] = writer._add_object(stream)
        path = root/'cairo-control.pdf'
        writer.write(path)
        return PdfReader(path)

    def test_local_reference_recovers_svg_paths_and_viewport(self):
        with tempfile.TemporaryDirectory() as temporary:
            reader = self.reader(Path(temporary))
            form = local_form(reader, Decimal(10), Decimal(20))
            svg = (b'<svg width="10pt" height="20pt" viewBox="0 0 10 20">'
                   b'<g fill="currentColor"><path d="M 1 2 L 4 2 L 4 5 L 1 5 Z"/>'
                   b'<path d="M 0.25 0.75 L 0.75 0.75 L 0.75 1.75 L 0.25 1.75 Z"/></g></svg>')
            self.assertEqual(verify_form_geometry(svg, form, reader), (2, 10))
            self.assertEqual(list(form['/BBox']), [0, 0, 10, 20])

    def test_wrong_page_frame_and_transparency_cannot_become_references(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'opaque normal'):
                local_form(self.reader(root, opacity=0.5), Decimal(10), Decimal(20))
            with self.assertRaisesRegex(ValueError, 'coordinate frame'):
                local_form(self.reader(root, b'1 0 0 -1 0 77824 cm q Q'), Decimal(10), Decimal(20))
            with self.assertRaisesRegex(ValueError, 'opaque black'):
                local_form(self.reader(root, b'1 0 0 -1 0 81920 cm q 1 g Q'), Decimal(10), Decimal(20))

    def test_page_retains_exact_root_and_view_transform(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            form = local_form(self.reader(root), Decimal(10), Decimal(20))
            output = root/'reference.pdf'
            matrix = tuple(map(Decimal, ['1', '0', '0', '-1', '11', '90.0000152587890625']))
            reference_page(output, Decimal(100), Decimal(100),
                           [{'image_id': 0, 'kind': 'math_vector'}], [matrix], lambda _: form, lambda _: None)
            reader = PdfReader(output)
            self.assertIn(b'9.9999847412109375 cm', reader.pages[0].get_contents().get_data())
            observed = precise_matrices(output)[0][0]
            self.assertEqual(tuple(map(fixed, observed)), tuple(map(fixed, matrix)))

    def test_png_reference_keeps_source_pixels_and_upright_transform(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            from PIL import Image
            import io
            data = png(3)
            with Image.open(io.BytesIO(data)) as image:
                image.load()
                stream = DecodedStreamObject()
                stream.set_data(image.tobytes())
            stream.update({NameObject('/Type'): NameObject('/XObject'), NameObject('/Subtype'): NameObject('/Image'),
                NameObject('/Width'): NumberObject(2), NameObject('/Height'): NumberObject(2),
                NameObject('/ColorSpace'): NameObject('/DeviceRGB'), NameObject('/BitsPerComponent'): NumberObject(8)})
            output = root/'raster-reference.pdf'
            matrix = tuple(map(Decimal, (32, 0, 0, 32, 11, 50)))
            reference_page(output, Decimal(100), Decimal(100), [{'kind': 'figure', 'image_id': 3}],
                           [matrix], lambda _: None, lambda _: stream.flate_encode())
            reader = PdfReader(output)
            resource = reader.pages[0]['/Resources']['/XObject']['/R3'].get_object()
            self.assertEqual(verify_raster_pixels(data, resource), 4)
            self.assertEqual(precise_matrices(output)[0][0], matrix)


if __name__ == '__main__':
    unittest.main()
