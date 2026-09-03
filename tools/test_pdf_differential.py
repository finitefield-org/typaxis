#!/usr/bin/env python3

from __future__ import annotations

import hashlib
from pathlib import Path
import stat
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parent))
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
            legacy_render = hashlib.sha256()
            legacy_render.update((1).to_bytes(4, "big"))
            legacy_render.update((1).to_bytes(4, "big"))
            legacy_render.update((1).to_bytes(4, "big"))
            legacy_render.update((1).to_bytes(4, "big"))
            legacy_render.update(len(PNG).to_bytes(8, "big"))
            legacy_render.update(PNG)
            self.assertEqual(result.render_sha256, legacy_render.hexdigest())

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

    def test_raster_differential_visits_every_reported_page(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            pdf = temporary / "input.pdf"
            pdf.write_bytes(b"pdf")
            visited = temporary / "visited.txt"
            renderer = executable(
                temporary / "mutool",
                "from pathlib import Path\n"
                f"payload = {PNG!r}\n"
                "import sys\n"
                "Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(payload)\n"
                f"with Path({str(visited)!r}).open('a', encoding='utf-8') as output:\n"
                "    output.write(sys.argv[-1] + '\\n')",
            )
            extractor = executable(temporary / "pdftotext", "print('all pages')")
            inspector = executable(temporary / "pdfinfo", "print('Pages: 3')")
            result = differential.verify_pdf_differential(
                [pdf],
                expected_text="all pages",
                expected_pages=3,
                mutool=renderer,
                pdftotext=extractor,
                pdfinfo=inspector,
            )
            self.assertEqual(result.page_count, 3)
            self.assertEqual(visited.read_text("utf-8").splitlines(), ["1", "2", "3"])

    def test_vector_render_gate_visits_200_and_800_percent_equivalent_dpis(self) -> None:
        with tempfile.TemporaryDirectory() as raw:
            temporary = Path(raw)
            pdf = temporary / "input.pdf"
            pdf.write_bytes(b"pdf")
            visited = temporary / "visited.txt"
            renderer = executable(
                temporary / "mutool",
                "from pathlib import Path\n"
                f"payload = {PNG!r}\n"
                "import sys\n"
                "Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(payload)\n"
                f"with Path({str(visited)!r}).open('a', encoding='utf-8') as output:\n"
                "    output.write(sys.argv[sys.argv.index('-r') + 1] + ':' + sys.argv[-1] + '\\n')",
            )
            extractor = executable(temporary / "pdftotext", "print('vector')")
            inspector = executable(temporary / "pdfinfo", "print('Pages: 1')")
            result = differential.verify_pdf_differential(
                [pdf],
                expected_text="vector",
                expected_pages=1,
                mutool=renderer,
                pdftotext=extractor,
                pdfinfo=inspector,
                render_dpis=(144, 576),
            )
            self.assertEqual(result.render_dpis, (144, 576))
            self.assertEqual(
                visited.read_text("utf-8").splitlines(), ["144:1", "576:1"]
            )

            with self.assertRaises(differential.PdfDifferentialError):
                differential.verify_pdf_differential(
                    [pdf],
                    expected_text="vector",
                    expected_pages=1,
                    mutool=renderer,
                    pdftotext=extractor,
                    pdfinfo=inspector,
                    render_dpis=(0x1_0000_0000,),
                )

    def test_independent_vector_structure_parser_is_fail_closed(self) -> None:
        form_stream = (
            b"q\n0 0 30 12 re W n\n1 0 0 1 0 0 cm\n"
            b"q\n/GS0 gs\n0 0 m\n1 1 l\n1 w\n0 J\n0 j\n10 M\nS\nQ\nQ"
        )
        page_stream = (
            b"q\n1 0 0 -1 0 140 cm\nq\n0 0 0 rg\n0 0 0 RG\n"
            b"1 0 0 1 0 0 cm\n/V0 Do\nQ\nQ"
        )

        def pdf(
            form: bytes,
            ext: bytes,
            *,
            page_target: bytes = b"5",
            form_ext_target: bytes = b"6",
            content: bytes = page_stream,
        ) -> bytes:
            return (
                b"%PDF-1.7\n"
                b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n"
                b"2 0 obj\n<< /Type /Pages /Count 1 /Kids [3 0 R] >>\nendobj\n"
                b"3 0 obj\n<< /Type /Page /Parent 2 0 R "
                b"/Resources << /XObject << /V0 "
                + page_target
                + b" 0 R >> >> /Contents 4 0 R >>\nendobj\n"
                + b"4 0 obj\n<< /Length 1 >>\nstream\n"
                + content
                + b"\nendstream\nendobj\n"
                + b"5 0 obj\n<< /Type /XObject /Subtype /Form /FormType 1 "
                + b"/BBox [0 0 30 12] /Resources << /ExtGState << /GS0 "
                + form_ext_target
                + b" 0 R >> >> "
                + b"/Length 1 >>\nstream\n"
                + form
                + b"\nendstream\nendobj\n"
                + b"6 0 obj\n"
                + ext
                + b"\nendobj\nxref\ntrailer\n%%EOF\n"
            )

        expected = differential.VectorPdfExpectations(1, 1, 1, 1)
        valid = pdf(form_stream, b"<< /Type /ExtGState /ca 1 /CA 1 >>")
        result = differential.inspect_vector_pdf_structure(valid, expected)
        self.assertEqual(result.form_count, 1)
        self.assertEqual(result.ext_g_state_count, 1)
        self.assertEqual(result.do_count, 1)
        self.assertEqual(result.page_root_y_flip_count, 1)

        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream + b" /Subtype /Image",
                    b"<< /Type /ExtGState /ca 1 /CA 1 >>",
                ),
                expected,
            )
        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream,
                    b"<< /Type /ExtGState /ca NaN /CA 1 >>",
                ),
                expected,
            )
        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream,
                    b"<< /Type /ExtGState /ca 1 /CA 1 >>",
                    page_target=b"6",
                ),
                expected,
            )
        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream,
                    b"<< /Type /ExtGState /ca 1 /CA 1 >>",
                    form_ext_target=b"5",
                ),
                expected,
            )
        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream,
                    b"<< /Type /ExtGState /ca 1 /CA 1 >>",
                    content=(
                        b"q\n1 0 0 -1 0 140 cm\nq\n0 0 0 rg\n0 0 0 RG\n"
                        b"/V0 Do\nQ\nQ"
                    ),
                ),
                expected,
            )
        with self.assertRaises(differential.PdfDifferentialError):
            differential.inspect_vector_pdf_structure(
                pdf(
                    form_stream,
                    b"<< /Type /ExtGState /ca 1 /CA 1 /BM /Normal >>",
                ),
                expected,
            )


if __name__ == "__main__":
    unittest.main()
