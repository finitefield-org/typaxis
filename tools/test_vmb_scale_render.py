"""Original-SVG and fixed raster gate controls, independent of generated PDFs."""
import copy
import json
import tempfile
import unittest
from subprocess import CompletedProcess
from unittest.mock import patch
from decimal import Decimal
from pathlib import Path
from xml.etree import ElementTree as ET

from PIL import Image
from pypdf import PdfReader, PdfWriter
from pypdf.generic import DecodedStreamObject, NameObject
from verify_vmb_scale_render import compare_masks, crop_box, ink, original_svg, render_pages
from verify_vmb_scale_render import normalize_reference, precise_matrices, REFERENCE_SCALE
from verify_vmb_scale_render import command


class RawSvgRenderTests(unittest.TestCase):
    def mask(self, points, size=(50, 40)):
        result = Image.new('L', size)
        for point in points:
            result.putpixel(point, 255)
        return result

    def test_threshold_is_strict_and_transparency_composites_to_white(self):
        image = Image.new('RGBA', (3, 1))
        image.putdata([(127, 127, 127, 255), (128, 128, 128, 255), (0, 0, 0, 0)])
        self.assertEqual(ink(image).tobytes(), b'\xff\0\0')

    def test_one_pixel_distance_allows_orthogonal_but_not_diagonal_or_two_pixels(self):
        reference = self.mask([(20, 20)])
        self.assertTrue(compare_masks(reference, self.mask([(21, 20)]), (0, 0, 50, 40))['passed'])
        for point in [(21, 21), (22, 20), (49, 20)]:
            self.assertFalse(compare_masks(reference, self.mask([point]), (0, 0, 50, 40))['passed'])
        self.assertFalse(compare_masks(self.mask([(0, 20)]), self.mask([(49, 20)]),
                                       (0, 0, 50, 40))['passed'])

    def test_small_area_tolerance_is_four_pixels_inclusive(self):
        points = {(x, y) for x in range(10, 15) for y in range(10, 15)}
        reference = self.mask(points)
        self.assertTrue(compare_masks(reference, self.mask(points-{(10, y) for y in range(10, 14)}),
                                      (0, 0, 50, 40))['passed'])
        result = compare_masks(reference, self.mask(points-{(10, y) for y in range(10, 15)}),
                               (0, 0, 50, 40))
        self.assertFalse(result['passed'])
        self.assertEqual(result['missing_over_one_pixel'], 0)
        self.assertEqual(result['area_difference'], 5)

    def test_one_percent_area_tolerance_is_inclusive_and_not_rounded_up(self):
        points = {(x, y) for x in range(5, 45) for y in range(5, 30)}
        reference = self.mask(points)
        for removed, want in [(10, True), (11, False)]:
            observed = self.mask(points-{(x, 5) for x in range(5, 5+removed)})
            self.assertEqual(compare_masks(reference, observed, (0, 0, 50, 40))['passed'], want)

    def test_outer_ring_extra_ink_and_interior_holes_are_not_discarded(self):
        points = {(x, y) for x in range(10, 17) for y in range(10, 17)}
        reference = self.mask(points)
        extra = compare_masks(reference, self.mask(points|{(8, 8)}), (10, 10, 17, 17))
        self.assertFalse(extra['passed'])
        self.assertEqual(extra['extra_over_one_pixel'], 1)
        hole = compare_masks(reference, self.mask(points-{(x, y) for x in range(12, 15)
                                                         for y in range(12, 15)}), (10, 10, 17, 17))
        self.assertFalse(hole['passed'])
        self.assertGreater(hole['missing_over_one_pixel'], 0)
        self.assertFalse(compare_masks(reference, self.mask([]), (10, 10, 17, 17))['passed'])

    def test_mask_dimensions_cannot_be_resized_to_match(self):
        with self.assertRaisesRegex(ValueError, 'unaltered equal mask dimensions'):
            compare_masks(self.mask([(1, 1)]), self.mask([(1, 1)], (49, 40)), (0, 0, 10, 10))

    def test_empty_subpixel_masks_preserve_the_separate_geometry_requirement(self):
        self.assertTrue(compare_masks(self.mask([]), self.mask([]), (0, 0, 50, 40))['passed'])
        self.assertFalse(compare_masks(self.mask([]), self.mask([(10, 10)]), (0, 0, 50, 40))['passed'])
        self.assertFalse(compare_masks(self.mask([(10, 10)]), self.mask([]), (0, 0, 50, 40))['passed'])

    def test_renderer_syntax_warnings_fail_even_with_zero_exit(self):
        for tool in ['/usr/bin/mutool', '/usr/bin/pdftoppm', '/usr/bin/rsvg-convert']:
            with patch('verify_vmb_scale_render.subprocess.run', return_value=CompletedProcess(
                    [tool], 0, stdout=b'', stderr=b'syntax error: invalid page\n')):
                with self.subTest(tool=tool), self.assertRaisesRegex(ValueError, 'unexpected renderer output'):
                    command([tool], [])
        with patch('verify_vmb_scale_render.subprocess.run', return_value=CompletedProcess(
                ['mutool'], 0, stdout=b'', stderr=b'warning: ICC support is not available\n')):
            self.assertEqual(command(['/usr/bin/mutool'], []), b'')

    def test_crop_uses_absolute_page_grid_without_position_search(self):
        bounds = [int(Decimal(v)*65536) for v in ['10.25', '60.25', '30.25', '80.25']]
        self.assertEqual(crop_box(bounds, Decimal(100), 144), (20, 39, 61, 80))

    def original_fixture(self):
        root = Path(__file__).resolve().parents[1]/'samples/machine-package/staging/production-book-1/vmb-book/engine-v2'
        case = json.loads((root/'fixture-index.json').read_bytes())['cases'][0]
        return (root/case['source_svg']).read_bytes(), (root/case['derived_svg']).read_bytes(), case

    def test_original_geometry_and_metrics_are_checked_before_rendering(self):
        source, derived, case = self.original_fixture()
        _, counts = original_svg(source, derived, case)
        self.assertEqual(counts, {'paths': case['path_count'], 'segments': case['segment_count']})
        raw = ET.fromstring(source)
        raw.attrib['data-math-baseline'] = str(int(raw.attrib['data-math-baseline'])+1)
        with self.assertRaisesRegex(ValueError, 'metrics conversion'):
            original_svg(ET.tostring(raw), derived, case)
        converted = ET.fromstring(derived)
        path = next(node for node in converted.iter() if node.tag.rsplit('}', 1)[-1] == 'path')
        tokens = path.attrib['d'].split()
        tokens[1] = str(Decimal(tokens[1])+1)
        path.attrib['d'] = ' '.join(tokens)
        with self.assertRaisesRegex(ValueError, 'path conversion'):
            original_svg(source, ET.tostring(converted), case)
        wrong = copy.deepcopy(case)
        wrong['speech'] += ' extra'
        with self.assertRaisesRegex(ValueError, 'semantics'):
            original_svg(source, derived, wrong)

    def test_render_output_cannot_modify_input_directories(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(ValueError, 'outside inputs'):
                render_pages(root/'job', root/'corpus', root/'missing.pdf', {}, root/'job/new')
            self.assertFalse((root/'job').exists())

    def test_vector_normalization_keeps_source_content_and_restores_physical_page(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, output = root/'internal.pdf', root/'physical.pdf'
            writer = PdfWriter()
            page = writer.add_blank_page(width=10*REFERENCE_SCALE, height=20*REFERENCE_SCALE)
            content = b'0 0 0 rg 16 32 m 4096 8192 l 10240 20480 l h f'
            stream = DecodedStreamObject()
            stream.set_data(content)
            page[NameObject('/Contents')] = writer._add_object(stream)
            writer.write(source)
            original = source.read_bytes()
            normalize_reference(source, output, Decimal(10), Decimal(20))
            result = PdfReader(output)
            self.assertEqual(list(result.pages[0].mediabox), [0, 0, 10, 20])
            self.assertEqual(result.pages[0].get_contents().get_data(),
                             b'q\n0.000244140625 0 0 0.000244140625 0 0 cm\n'+content+b'\nQ\n')
            self.assertEqual(source.read_bytes(), original)
            with self.assertRaisesRegex(ValueError, 'preserves its source'):
                normalize_reference(source, source, Decimal(10), Decimal(20))

    def test_pdf_matrices_keep_unit_fraction_precision_across_saved_states(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary)/'positions.pdf'
            writer = PdfWriter()
            page = writer.add_blank_page(width=500, height=400)
            stream = DecodedStreamObject()
            stream.set_data(b'q 1 0 0 -1 0 400 cm '
                            b'q 1 0 0 1 212.7307891845703125 26.6322021484375 cm /V0 Do Q '
                            b'q 1 0 0 1 11 10 cm /V1 Do Q Q')
            page[NameObject('/Contents')] = writer._add_object(stream)
            writer.write(path)
            first, second = precise_matrices(path)[0]
            self.assertLess(abs(first[4]-Decimal('212.7307891845703125')), Decimal('0.000000000001'))
            self.assertLess(abs(first[5]-Decimal('373.3677978515625')), Decimal('0.000000000001'))
            self.assertEqual(second, tuple(map(Decimal, (1, 0, 0, -1, 11, 390))))


if __name__ == '__main__':
    unittest.main()
