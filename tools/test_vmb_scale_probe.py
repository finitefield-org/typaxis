"""Fail-closed checks for the real-engine scale probe harness."""
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from PIL import Image
from prepare_vmb_distinct_probe import png, prepare
from verify_vmb_scale_pdf import verify, verify_form_geometry, verify_raster_pixels
from pypdf.generic import ArrayObject, DictionaryObject, NameObject, NumberObject, DecodedStreamObject


class ScaleProbeTests(unittest.TestCase):
    def geometry_fixture(self, content=None):
        svg = b'<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1"><g fill="currentColor"><path d="M 0.25 0.5 L 1 1 Z"/><path d="M 0 0 C 0 1 1 0 1 1 Z"/></g></svg>'
        form = DecodedStreamObject()
        form[NameObject('/BBox')] = ArrayObject([NumberObject(n) for n in [0, 0, 1, 1]])
        gs = DictionaryObject({NameObject('/Type'): NameObject('/ExtGState'),
                               NameObject('/ca'): NumberObject(1), NameObject('/CA'): NumberObject(1)})
        form[NameObject('/Resources')] = DictionaryObject({NameObject('/ExtGState'):
            DictionaryObject({NameObject('/GS0'): gs})})
        form.set_data(content or b'q\n0 0 1 1 re W n\n1 0 0 1 0 0 cm\nq\n/GS0 gs\n0.25 0.5 m\n1 1 l\nh\nf\nQ\nq\n/GS0 gs\n0 0 m\n0 1 1 0 1 1 c\nh\nf\nQ\nQ')
        return svg, form

    def test_all_path_geometry_and_separate_paint_operations_are_verified(self):
        svg, form = self.geometry_fixture()
        self.assertEqual(verify_form_geometry(svg, form, None), (2, 6))
        content = form.get_data()
        for old, new in [(b'0.25 0.5 m', b'0.5 0.5 m'),
                         (b'0 1 1 0 1 1 c', b'0 1 0 0 1 1 c'),
                         (b'h\nf\nQ\nq\n/GS0 gs', b'h'),
                         (b'1 0 0 1 0 0 cm', b'1 0 0 1 1 0 cm'),
                         (b'0 0 1 1 re W n', b'0 0 0.5 1 re W n'),
                         (b'\nf\n', b'\nf*\n')]:
            with self.subTest(change=(old, new)):
                _, mutated = self.geometry_fixture(content.replace(old, new, 1))
                with self.assertRaises(ValueError):
                    verify_form_geometry(svg, mutated, None)
        form['/Resources']['/ExtGState']['/GS0'][NameObject('/ca')] = NumberObject(0)
        with self.assertRaisesRegex(ValueError, 'opaque paint'):
            verify_form_geometry(svg, form, None)

    def test_form_dictionary_transform_and_visibility_are_verified(self):
        for key, value in [('/Matrix', ArrayObject([NumberObject(n) for n in [1, 0, 0, 1, 1, 0]])),
                           ('/Group', DictionaryObject()), ('/OC', DictionaryObject())]:
            svg, form = self.geometry_fixture()
            form[NameObject(key)] = value
            with self.assertRaises(ValueError):
                verify_form_geometry(svg, form, None)

    def test_pdf_raster_pixels_and_interpretation_are_verified(self):
        source = png(3)
        with Image.open(io.BytesIO(source)) as original:
            pixels = original.tobytes()
        def fixture():
            image = DecodedStreamObject()
            image.update({NameObject('/Width'): NumberObject(2), NameObject('/Height'): NumberObject(2),
                          NameObject('/ColorSpace'): NameObject('/DeviceRGB'),
                          NameObject('/BitsPerComponent'): NumberObject(8)})
            image.set_data(pixels)
            return image
        self.assertEqual(verify_raster_pixels(source, fixture()), 4)
        image = fixture()
        image.set_data(bytes([pixels[0] ^ 1]) + pixels[1:])
        with self.assertRaisesRegex(ValueError, 'pixel mismatch'):
            verify_raster_pixels(source, image)
        for key, value in [('/ColorSpace', NameObject('/DeviceGray')),
                           ('/BitsPerComponent', NumberObject(4)),
                           ('/Decode', ArrayObject()), ('/SMask', DictionaryObject())]:
            image = fixture()
            image[NameObject(key)] = value
            with self.assertRaises(ValueError):
                verify_raster_pixels(source, image)

    def test_unused_declarations_cannot_pass_the_pdf_gate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = {'document': {'blocks': [{'kind': 'math_vector_block', 'image_id': 0}]},
                       'resources': {'images': [{} for _ in range(5000)]}}
            payload = json.dumps(package).encode()
            (root/'document-package.json').write_bytes(payload)
            (root/'probe-inventory.json').write_text(json.dumps({
                'package_sha256': hashlib.sha256(payload).hexdigest(), 'placements': 1}))
            with self.assertRaisesRegex(ValueError, 'every declaration must be placed'):
                verify(root, root/'nonexistent.pdf')

    def test_pngs_have_distinct_decoded_pixels(self):
        pixels = set()
        for index in range(48):
            with Image.open(io.BytesIO(png(index))) as image:
                image.load()
                self.assertEqual(image.size, (2, 2))
                self.assertEqual(image.mode, 'RGB')
                pixels.add(image.tobytes())
        self.assertEqual(len(pixels), 48)

    def test_existing_output_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            marker = root/'sentinel'
            marker.write_text('keep')
            with self.assertRaises(FileExistsError):
                prepare(root, root, 'distinct', {}, {})
            self.assertEqual(marker.read_text(), 'keep')


if __name__ == '__main__':
    unittest.main()
