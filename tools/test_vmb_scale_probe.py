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
from verify_vmb_scale_pdf import PagePaint, verify_placement, verify_inputs
from vmb_probe_io import contained_bytes, extracted_occurrences, same_json, strict_json
from pypdf.generic import ArrayObject, DictionaryObject, NameObject, NumberObject, DecodedStreamObject
from pypdf.generic import ContentStream


class ScaleProbeTests(unittest.TestCase):
    def test_json_rejects_nested_duplicate_and_nonfinite_members(self):
        self.assertEqual(strict_json(b'{"a":[1,2]}'), {'a': [1, 2]})
        for payload in [b'{"a":1,"a":2}', b'{"a":{"b":1,"b":2}}',
                        b'{"a":NaN}', b'{"a":Infinity}', b'{"a":1e9999}',
                        b'{"a":"\\ud800"}', b'["\\udfff"]']:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                strict_json(payload)

    def test_input_paths_reject_escape_absolute_and_symlink_reads(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'inside').write_bytes(b'ok')
            self.assertEqual(contained_bytes(root, 'inside'), b'ok')
            (root/'alias').symlink_to(root/'inside')
            for path in ['../outside', str(root/'inside'), './inside', 'x/../inside',
                         'x//inside', 'x\\inside', 'alias']:
                with self.subTest(path=path), self.assertRaises(ValueError):
                    contained_bytes(root, path)

    def test_extraction_preserves_source_spaces_and_unicode(self):
        expected = ['a b', 'c d']
        self.assertEqual(extracted_occurrences(b'\na b\n\f\nc d\n\f', expected, 'test'), 2)
        for payload in [b'a b c d', b'a b\r\nc d', b'a b \nc d', b'a b\tc d']:
            self.assertEqual(extracted_occurrences(payload, expected, 'test'), 2)
        for payload in [b'ab\nc d', b'a  b\nc d', b'a\tb\nc d',
                        b'a b\n c d', b'a b\nc d ', b'a b\nc d\nextra',
                        b'a b  c d', b'a bc d', 'a\u00a0b\nc d'.encode()]:
            with self.subTest(payload=payload), self.assertRaises(ValueError):
                extracted_occurrences(payload, expected, 'test')
        with self.assertRaises(ValueError):
            extracted_occurrences('e\u0301'.encode(), ['é'], 'test')
        with self.assertRaises(ValueError):
            extracted_occurrences(b'\n', [], 'test')

    def page_paint_fixture(self, content=None, baseline=10):
        stream = DecodedStreamObject()
        stream.set_data(content or b'q 1 0 0 -1 0 100 cm '
                        b'BT /F0 1 Tf 3 Tr 0 Tc 0 Tw 100 Tz 0 TL 0 Ts '
                        b'20 0 0 -10 10 40 Tm <00> Tj ET '
                        b'q 0 0 0 rg 1 0 0 1 10 30 cm /V0 Do Q Q')
        resources = DictionaryObject({NameObject('/Font'): DictionaryObject({
            NameObject('/F0'): DictionaryObject()})})
        paint = PagePaint(resources)
        anchors, images = [], []
        for args, op in ContentStream(stream, None).operations:
            event = paint.step(args, op)
            if event and event[0] == 'anchor':
                anchors.append(event[1])
            elif event:
                images.append(event)
        paint.finish()
        self.assertEqual(len(images), 1)
        want = {'kind': 'math_vector', 'metrics': {'viewport': {'width': 20*65536,
            'height': 10*65536}, 'baseline': baseline*65536}}
        body = {'x': 0, 'y': 0, 'width': 100*65536, 'height': 100*65536}
        return verify_placement(want, images[0], anchors, (0, 0, 100, 100), body)

    def test_page_paint_binds_unit_scale_and_source_baseline(self):
        self.assertEqual(self.page_paint_fixture(), tuple(v*65536 for v in [10, 60, 30, 70]))
        with self.assertRaisesRegex(ValueError, 'baseline anchor'):
            self.page_paint_fixture(baseline=9)

    def test_page_paint_rejects_invisible_clipped_moved_and_extra_ink(self):
        base = (b'q 1 0 0 -1 0 100 cm '
                b'BT /F0 1 Tf 3 Tr 0 Tc 0 Tw 100 Tz 0 TL 0 Ts '
                b'20 0 0 -10 10 40 Tm <00> Tj ET '
                b'q 0 0 0 rg 1 0 0 1 10 30 cm /V0 Do Q Q')
        variants = [
            base.replace(b'0 0 0 rg', b'1 1 1 rg'),
            base.replace(b'1 0 0 1 10 30 cm', b'0 0 0 0 10 30 cm'),
            base.replace(b'1 0 0 1 10 30 cm', b'2 0 0 2 10 30 cm'),
            base.replace(b'1 0 0 1 10 30 cm', b'1 0 0 1 200 30 cm'),
            base.replace(b'1 0 0 -1 0 100 cm', b'1 0 0 -1 0 100 cm 0 0 1 1 re W n'),
            base.replace(b'3 Tr', b'0 Tr'),
            base.replace(b'10 40 Tm', b'11 40 Tm'),
            base.replace(b'<00> Tj', b'<0000> Tj'),
            base.replace(b'<00> Tj', b'<00> Tj <00> Tj'),
            base.replace(b'/V0 Do', b'/V0 Do 0 0 2 2 re f'),
            base.replace(b'/V0 Do', b'/OC <<>> BDC /V0 Do EMC'),
            base+b' Q',
        ]
        for index, variant in enumerate(variants):
            with self.subTest(index=index), self.assertRaises(ValueError):
                self.page_paint_fixture(variant)

    def test_boolean_image_ids_are_not_integer_source_ids(self):
        self.assertTrue(same_json({'a': [0, 1]}, {'a': [0, 1]}))
        self.assertFalse(same_json({'a': [False, True]}, {'a': [0, 1]}))
        self.assertFalse(same_json({'a': 0.0}, {'a': 0}))
        images = [{'image_id': i} for i in range(5000)]
        images[1]['image_id'] = True
        with self.assertRaisesRegex(ValueError, 'dense image declarations'):
            verify_inputs(Path('.'), {}, {'case': 'distinct', 'placements': 5000,
                          'resources': 5000}, [], images)

    def test_page_graphics_state_rejects_transparency_and_blending(self):
        for key, value in [('/ca', NumberObject(0)), ('/BM', NameObject('/Multiply')),
                           ('/SMask', DictionaryObject()), ('/Unknown', NumberObject(1))]:
            resources = DictionaryObject({NameObject('/ExtGState'): DictionaryObject({
                NameObject('/Bad'): DictionaryObject({NameObject(key): value})})})
            paint = PagePaint(resources)
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, 'opaque normal'):
                paint.step([NameObject('/Bad')], b'gs')

    def test_placement_evidence_cannot_modify_inputs_or_overwrite_a_file(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            job = root/'job'
            job.mkdir()
            output = root/'observed.json'
            output.write_text('keep')
            with self.assertRaisesRegex(ValueError, 'outside the input package'):
                verify(job, root/'missing.pdf', job/'new-evidence.json')
            with self.assertRaisesRegex(ValueError, 'already exists'):
                verify(job, root/'missing.pdf', output)
            self.assertEqual(output.read_text(), 'keep')

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
