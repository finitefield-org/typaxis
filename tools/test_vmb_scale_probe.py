"""Fail-closed checks for the real-engine scale probe harness."""
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from PIL import Image
from prepare_vmb_distinct_probe import png, prepare
from verify_vmb_scale_pdf import verify


class ScaleProbeTests(unittest.TestCase):
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
