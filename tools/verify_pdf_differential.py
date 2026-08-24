#!/usr/bin/env python3
"""Run an independent MuPDF-render/Poppler-extract PDF differential gate."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
from typing import Sequence


class PdfDifferentialError(Exception):
    pass


@dataclass(frozen=True)
class PdfDifferentialResult:
    page_count: int
    render_sha256: str
    extracted_text_sha256: str


def _tool(name: str, override: str | None) -> str:
    candidate = override or shutil.which(name)
    if candidate is None:
        raise PdfDifferentialError(f"required independent PDF tool is unavailable: {name}")
    return candidate


def _run(command: Sequence[str]) -> bytes:
    try:
        completed = subprocess.run(
            command,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )
    except OSError as error:
        raise PdfDifferentialError(f"cannot execute {command[0]!r}: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise PdfDifferentialError(
            f"{' '.join(command)} failed: {detail or completed.returncode}"
        )
    return completed.stdout


def _png_dimensions(payload: bytes) -> tuple[int, int]:
    if (
        payload[:8] != b"\x89PNG\r\n\x1a\n"
        or payload[12:16] != b"IHDR"
        or len(payload) < 33
    ):
        raise PdfDifferentialError("renderer did not produce a canonical PNG header")
    width, height = struct.unpack(">II", payload[16:24])
    if width == 0 or height == 0 or b"IDAT" not in payload or payload[-12:] != b"\0\0\0\0IEND\xaeB`\x82":
        raise PdfDifferentialError("renderer produced an incomplete PNG")
    return width, height


def _normalized_text(payload: bytes) -> str:
    try:
        text = payload.decode("utf-8")
    except UnicodeDecodeError as error:
        raise PdfDifferentialError("extractor output is not UTF-8") from error
    return text.replace("\r\n", "\n").replace("\r", "\n").rstrip("\n\f")


def verify_pdf_differential(
    pdfs: Sequence[Path],
    *,
    expected_text: str,
    expected_pages: int,
    mutool: str | None = None,
    pdftotext: str | None = None,
    pdfinfo: str | None = None,
) -> PdfDifferentialResult:
    if not pdfs or expected_pages <= 0:
        raise PdfDifferentialError("at least one PDF and a positive page count are required")
    renderer = _tool("mutool", mutool)
    extractor = _tool("pdftotext", pdftotext)
    inspector = _tool("pdfinfo", pdfinfo)
    render_hashes: list[str] = []
    text_hashes: list[str] = []
    with tempfile.TemporaryDirectory(prefix="typaxis-pdf-differential-") as raw:
        temporary = Path(raw)
        for index, raw_pdf in enumerate(pdfs):
            pdf = raw_pdf.resolve(strict=True)
            info = _run([inspector, str(pdf)]).decode("utf-8", "replace")
            page_lines = [line for line in info.splitlines() if line.startswith("Pages:")]
            if len(page_lines) != 1:
                raise PdfDifferentialError("pdfinfo did not report exactly one page count")
            try:
                page_count = int(page_lines[0].split(":", 1)[1].strip())
            except ValueError as error:
                raise PdfDifferentialError("pdfinfo page count is not an integer") from error
            if page_count != expected_pages:
                raise PdfDifferentialError(
                    f"page count differs: expected {expected_pages}, observed {page_count}"
                )

            output = temporary / f"render-{index}.png"
            _run(
                [
                    renderer,
                    "draw",
                    "-q",
                    "-F",
                    "png",
                    "-r",
                    "72",
                    "-o",
                    str(output),
                    str(pdf),
                    "1",
                ]
            )
            rendered = output.read_bytes()
            _png_dimensions(rendered)
            render_hashes.append(hashlib.sha256(rendered).hexdigest())

            extracted = _run([extractor, "-enc", "UTF-8", str(pdf), "-"])
            normalized = _normalized_text(extracted)
            if normalized != expected_text:
                raise PdfDifferentialError(
                    f"extracted text differs: expected {expected_text!r}, observed {normalized!r}"
                )
            text_hashes.append(hashlib.sha256(normalized.encode("utf-8")).hexdigest())

    if len(set(render_hashes)) != 1 or len(set(text_hashes)) != 1:
        raise PdfDifferentialError("independent renderer/extractor results differ across PDFs")
    return PdfDifferentialResult(expected_pages, render_hashes[0], text_hashes[0])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf", action="append", required=True, type=Path)
    parser.add_argument("--expected-text", required=True)
    parser.add_argument("--expected-pages", required=True, type=int)
    parser.add_argument("--mutool")
    parser.add_argument("--pdftotext")
    parser.add_argument("--pdfinfo")
    arguments = parser.parse_args()
    try:
        result = verify_pdf_differential(
            arguments.pdf,
            expected_text=arguments.expected_text,
            expected_pages=arguments.expected_pages,
            mutool=arguments.mutool,
            pdftotext=arguments.pdftotext,
            pdfinfo=arguments.pdfinfo,
        )
    except (OSError, PdfDifferentialError) as error:
        parser.error(str(error))
    print(
        f"pages={result.page_count} render_sha256={result.render_sha256} "
        f"text_sha256={result.extracted_text_sha256}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
