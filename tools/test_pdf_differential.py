#!/usr/bin/env python3

from __future__ import annotations

from pathlib import Path
import stat
import sys
import tempfile
import unittest

import verify_pdf_differential as differential


PNG = bytes(
    [
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82,
        0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0, 31, 21, 196, 137,
        0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 240,
        31, 0, 5, 0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69,
        78, 68, 174, 66, 96, 130,
    ]
)


def executable(path: Path, source: str) -> str:
    path.write_text(f"#!{sys.executable}\n{source}\n", encoding="utf-8")
    path.chmod(stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
    return str(path)


class PdfDifferentialTests(unittest.TestCase):
    def test_normalization_drops_unmapped_artifact_control_scalars(self) -> None:
        self.assertEqual(
            differential._normalized_text(b"Typaxis machine input\n\x01\n\f"),
            "Typaxis machine input",
        )
        self.assertEqual(
            differential._normalized_text(
                b"Basic document\ninternal external\nFirst item\n\n\fPNG caption\n"
            ),
            "Basic document internal external First item PNG caption",
        )

    def test_independent_results_must_match_across_inputs(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            first = temporary / "first.pdf"
            second = temporary / "second.pdf"
            first.write_bytes(b"first")
            second.write_bytes(b"second")
            renderer = executable(
                temporary / "mutool",
                "from pathlib import Path\n"
                f"payload = {PNG!r}\n"
                "import sys\n"
                "Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(payload)",
            )
            extractor = executable(
                temporary / "pdftotext",
                "print('Page 1')",
            )
            inspector = executable(
                temporary / "pdfinfo",
                "print('Pages: 1')",
            )
            result = differential.verify_pdf_differential(
                [first, second],
                expected_text="Page 1",
                expected_pages=1,
                mutool=renderer,
                pdftotext=extractor,
                pdfinfo=inspector,
            )
            self.assertEqual(result.page_count, 1)
            self.assertEqual(len(result.render_sha256), 64)
            self.assertEqual(len(result.extracted_text_sha256), 64)

    def test_text_or_page_differences_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            pdf = temporary / "input.pdf"
            pdf.write_bytes(b"pdf")
            renderer = executable(
                temporary / "mutool",
                "from pathlib import Path\n"
                f"payload = {PNG!r}\n"
                "import sys\n"
                "Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(payload)",
            )
            extractor = executable(temporary / "pdftotext", "print('wrong')")
            inspector = executable(temporary / "pdfinfo", "print('Pages: 2')")
            with self.assertRaises(differential.PdfDifferentialError):
                differential.verify_pdf_differential(
                    [pdf],
                    expected_text="expected",
                    expected_pages=1,
                    mutool=renderer,
                    pdftotext=extractor,
                    pdfinfo=inspector,
                )


if __name__ == "__main__":
    unittest.main()
