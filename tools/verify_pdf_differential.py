#!/usr/bin/env python3
"""Run an independent MuPDF-render/Poppler-extract PDF differential gate."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
from typing import Sequence


class PdfDifferentialError(Exception):
    pass


EXTERNAL_COMMAND_TIMEOUT_SECONDS = 120


@dataclass(frozen=True)
class PdfDifferentialResult:
    page_count: int
    render_sha256: str
    extracted_text_sha256: str
    render_dpis: tuple[int, ...] = (72,)


@dataclass(frozen=True)
class VectorPdfExpectations:
    form_count: int
    ext_g_state_count: int
    do_count: int
    page_root_y_flip_count: int


@dataclass(frozen=True)
class VectorPdfStructureResult:
    form_count: int
    ext_g_state_count: int
    do_count: int
    page_root_y_flip_count: int
    structure_sha256: str


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
            timeout=EXTERNAL_COMMAND_TIMEOUT_SECONDS,
        )
    except subprocess.TimeoutExpired as error:
        raise PdfDifferentialError(
            f"{command[0]!r} exceeded {EXTERNAL_COMMAND_TIMEOUT_SECONDS} seconds"
        ) from error
    except OSError as error:
        raise PdfDifferentialError(f"cannot execute {command[0]!r}: {error}") from error
    if completed.returncode != 0:
        detail = completed.stderr.decode("utf-8", "replace").strip()
        raise PdfDifferentialError(
            f"{' '.join(command)} failed: {detail or completed.returncode}"
        )
    if completed.stderr:
        raise PdfDifferentialError(
            f"{' '.join(command)} emitted a warning: "
            + completed.stderr.decode("utf-8", "replace").strip()
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
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    # Some extractors expose an unmapped CID from Artifact marked content as
    # a C0 control scalar. Drop those scalars, then normalize extractor-owned
    # line, page, and tab separators to one space. Page topology is checked
    # independently, so layout wrapping cannot change logical text evidence.
    output: list[str] = []
    pending_separator = False
    for character in text:
        if character in "\t\n\f":
            pending_separator = True
        elif ord(character) >= 0x20:
            if pending_separator and output and output[-1] != " ":
                output.append(" ")
            output.append(character)
            pending_separator = False
    return "".join(output)


_OBJECT = re.compile(rb"(?m)^(\d+) 0 obj\n(.*?)\nendobj\n", re.DOTALL)


def _stream(body: bytes) -> bytes | None:
    marker = b"stream\n"
    start = body.find(marker)
    if start < 0:
        return None
    start += len(marker)
    end = body.find(b"\nendstream", start)
    if end < 0:
        raise PdfDifferentialError("vector PDF object has an unterminated stream")
    return body[start:end]


def _tokens(payload: bytes) -> list[bytes]:
    try:
        payload.decode("ascii")
    except UnicodeDecodeError as error:
        raise PdfDifferentialError("vector PDF operators are not ASCII") from error
    return payload.split()


def _canonical_alpha(token: bytes) -> bool:
    if token in (b"0", b"1"):
        return True
    match = re.fullmatch(rb"0\.([0-9]{1,16})", token)
    if match is None or match.group(1).endswith(b"0"):
        return False
    fraction = match.group(1)
    denominator = 10 ** len(fraction)
    scaled = int(fraction) * 65_536
    if scaled % denominator != 0:
        return False
    raw = scaled // denominator
    if raw > 65_536:
        return False
    integer, remainder = divmod(raw, 65_536)
    if remainder == 0:
        canonical = str(integer)
    else:
        decimal = f"{remainder * 152_587_890_625:016d}".rstrip("0")
        canonical = f"{integer}.{decimal}"
    return token == canonical.encode("ascii")


def inspect_vector_pdf_structure(
    payload: bytes, expected: VectorPdfExpectations
) -> VectorPdfStructureResult:
    if min(
        expected.form_count,
        expected.ext_g_state_count,
        expected.do_count,
        expected.page_root_y_flip_count,
    ) < 0:
        raise PdfDifferentialError("vector PDF expectations cannot be negative")
    if not payload.startswith(b"%PDF-") or b"xref\n" not in payload or b"trailer\n" not in payload:
        raise PdfDifferentialError("vector structure input is not a complete PDF")
    if b"/Subtype /Image" in payload or b"/ImageMask" in payload:
        raise PdfDifferentialError("vector PDF contains a raster image XObject")
    matches = list(_OBJECT.finditer(payload))
    if not matches:
        raise PdfDifferentialError("vector PDF has no indirect objects")
    numbers = [int(match.group(1)) for match in matches]
    if numbers != list(range(1, len(numbers) + 1)):
        raise PdfDifferentialError("vector PDF object table is duplicate or non-canonical")
    objects = [(int(match.group(1)), match.group(2)) for match in matches]
    forms = [(number, body) for number, body in objects if b"/Subtype /Form" in body]
    ext_g_states = [
        (number, body) for number, body in objects if b"/Type /ExtGState" in body
    ]
    if len(forms) != expected.form_count:
        raise PdfDifferentialError(
            f"vector Form count differs: expected {expected.form_count}, observed {len(forms)}"
        )
    if len(ext_g_states) != expected.ext_g_state_count:
        raise PdfDifferentialError(
            "vector ExtGState count differs: "
            f"expected {expected.ext_g_state_count}, observed {len(ext_g_states)}"
        )

    structure = hashlib.sha256()
    ext_object_numbers = {number for number, _ in ext_g_states}
    bound_ext_objects: set[int] = set()
    for _, body in forms:
        if (
            b"/Type /XObject" not in body
            or b"/FormType 1" not in body
            or b"/BBox [" not in body
            or b"/Resources << /ExtGState <<" not in body
        ):
            raise PdfDifferentialError("vector Form dictionary is incomplete")
        stream = _stream(body)
        if stream is None:
            raise PdfDifferentialError("vector Form has no content stream")
        dictionary = body[: body.index(b"\nstream\n")]
        binding_rows = re.findall(rb"(/GS[0-9]+) ([1-9][0-9]*) 0 R", dictionary)
        declared_names = re.findall(rb"/GS[0-9]+", dictionary)
        bindings = {name: int(target) for name, target in binding_rows}
        if (
            not bindings
            or len(binding_rows) != len(declared_names)
            or len(bindings) != len(binding_rows)
            or [name for name, _ in binding_rows]
            != [f"/GS{index}".encode("ascii") for index in range(len(binding_rows))]
            or any(target not in ext_object_numbers for target in bindings.values())
            or any(target in bound_ext_objects for target in bindings.values())
        ):
            raise PdfDifferentialError("vector Form ExtGState resources are invalid")
        if any(
            forbidden in stream
            for forbidden in (b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b" BDC", b" BMC")
        ):
            raise PdfDifferentialError("reusable vector Form contains semantic marked content")
        tokens = _tokens(stream)
        paint_count = sum(tokens.count(operator) for operator in (b"S", b"f", b"f*", b"B", b"B*"))
        stroked_paint_count = sum(tokens.count(operator) for operator in (b"S", b"B", b"B*"))
        used_ext_names = {
            tokens[index - 1]
            for index, token in enumerate(tokens)
            if token == b"gs" and index > 0
        }
        invalid_gs_name = any(
            token == b"gs" and (index == 0 or tokens[index - 1] not in bindings)
            for index, token in enumerate(tokens)
        )
        if (
            tokens.count(b"q") != tokens.count(b"Q")
            or tokens.count(b"q") < 2
            or tokens.count(b"gs") != paint_count
            or paint_count == 0
            or b"re" not in tokens
            or not ({b"W", b"W*"} & set(tokens))
            or b"m" not in tokens
            or b"cm" not in tokens
            or used_ext_names != set(bindings)
            or invalid_gs_name
            or (
                stroked_paint_count > 0
                and any(
                    tokens.count(operator) != stroked_paint_count
                    for operator in (b"w", b"J", b"j", b"M")
                )
            )
        ):
            raise PdfDifferentialError("vector Form operator closure is invalid")
        bound_ext_objects.update(bindings.values())
        structure.update(hashlib.sha256(body).digest())

    if bound_ext_objects != ext_object_numbers:
        raise PdfDifferentialError("vector ExtGState object is unbound or shared")

    for _, body in ext_g_states:
        tokens = _tokens(body)
        if (
            len(tokens) != 8
            or tokens[:4] != [b"<<", b"/Type", b"/ExtGState", b"/ca"]
            or tokens[5] != b"/CA"
            or tokens[7] != b">>"
        ):
            raise PdfDifferentialError("vector ExtGState has extra, missing, or reordered keys")
        if not _canonical_alpha(tokens[4]) or not _canonical_alpha(tokens[6]):
            raise PdfDifferentialError("vector ExtGState alpha is not canonical unsigned 16.16")
        structure.update(hashlib.sha256(body).digest())

    expected_form_targets = {
        f"/V{index}".encode("ascii"): number for index, (number, _) in enumerate(forms)
    }
    object_by_number = dict(objects)
    pages = [
        body
        for _, body in objects
        if b"/Type /Page" in body and b"/Type /Pages" not in body
    ]
    if len(pages) != expected.page_root_y_flip_count:
        raise PdfDifferentialError("vector page count differs from root-transform expectation")
    do_count = 0
    page_root_y_flip_count = 0
    for body in pages:
        if b"/Resources << /XObject <<" not in body:
            raise PdfDifferentialError("vector page resource dictionary is missing")
        binding_rows = re.findall(rb"(/V[0-9]+) ([1-9][0-9]*) 0 R", body)
        declared_names = re.findall(rb"/V[0-9]+", body)
        bindings = {name: int(target) for name, target in binding_rows}
        if (
            len(binding_rows) != len(declared_names)
            or len(bindings) != len(binding_rows)
            or any(
                expected_form_targets.get(name) != target
                for name, target in bindings.items()
            )
        ):
            raise PdfDifferentialError("vector page resource target differs")
        content_matches = re.findall(rb"/Contents ([1-9][0-9]*) 0 R", body)
        if len(content_matches) != 1:
            raise PdfDifferentialError("vector page content reference is not singular")
        content_body = object_by_number.get(int(content_matches[0]))
        if content_body is None:
            raise PdfDifferentialError("vector page content object is missing")
        stream = _stream(content_body)
        if stream is None:
            raise PdfDifferentialError("vector page content stream is missing")
        tokens = _tokens(stream)
        stream_do_count = tokens.count(b"Do")
        root_flip_count = stream.count(b"1 0 0 -1 0 ")
        used_names = {
            tokens[index - 1]
            for index, token in enumerate(tokens)
            if token == b"Do" and index > 0
        }
        invalid_do_name = any(
            token == b"Do" and (index == 0 or tokens[index - 1] not in bindings)
            for index, token in enumerate(tokens)
        )
        if (
            root_flip_count != 1
            or tokens.count(b"q") != stream_do_count + 1
            or tokens.count(b"Q") != stream_do_count + 1
            or tokens.count(b"cm") != stream_do_count + 1
            or tokens.count(b"rg") != stream_do_count
            or tokens.count(b"RG") != stream_do_count
            or used_names != set(bindings)
            or invalid_do_name
        ):
            raise PdfDifferentialError("vector page placement closure is invalid")
        do_count += stream_do_count
        page_root_y_flip_count += root_flip_count
        structure.update(hashlib.sha256(body).digest())
        structure.update(hashlib.sha256(stream).digest())
    if do_count != expected.do_count:
        raise PdfDifferentialError(
            f"vector Do count differs: expected {expected.do_count}, observed {do_count}"
        )
    if page_root_y_flip_count != expected.page_root_y_flip_count:
        raise PdfDifferentialError(
            "page-root Y-flip count differs: "
            f"expected {expected.page_root_y_flip_count}, observed {page_root_y_flip_count}"
        )
    return VectorPdfStructureResult(
        len(forms),
        len(ext_g_states),
        do_count,
        page_root_y_flip_count,
        structure.hexdigest(),
    )


def verify_pdf_differential(
    pdfs: Sequence[Path],
    *,
    expected_text: str,
    expected_pages: int,
    mutool: str | None = None,
    pdftotext: str | None = None,
    pdfinfo: str | None = None,
    render_dpis: Sequence[int] = (72,),
    vector_expectations: VectorPdfExpectations | None = None,
) -> PdfDifferentialResult:
    if not pdfs or expected_pages <= 0:
        raise PdfDifferentialError("at least one PDF and a positive page count are required")
    normalized_dpis = tuple(render_dpis)
    if (
        not normalized_dpis
        or any(
            not isinstance(dpi, int)
            or isinstance(dpi, bool)
            or dpi <= 0
            or dpi > 0xFFFF_FFFF
            for dpi in normalized_dpis
        )
        or tuple(sorted(set(normalized_dpis))) != normalized_dpis
    ):
        raise PdfDifferentialError(
            "render DPIs must be unsigned 32-bit, positive, unique, and ascending"
        )
    renderer = _tool("mutool", mutool)
    extractor = _tool("pdftotext", pdftotext)
    inspector = _tool("pdfinfo", pdfinfo)
    render_hashes: list[str] = []
    text_hashes: list[str] = []
    vector_hashes: list[str] = []
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

            rendered_document = hashlib.sha256()
            rendered_document.update(page_count.to_bytes(4, "big"))
            extended_dpi_domain = normalized_dpis != (72,)
            if extended_dpi_domain:
                rendered_document.update(b"typaxis.render-dpi-set/1\0")
                rendered_document.update(len(normalized_dpis).to_bytes(4, "big"))
            for dpi in normalized_dpis:
                if extended_dpi_domain:
                    rendered_document.update(dpi.to_bytes(4, "big"))
                for page_number in range(1, page_count + 1):
                    output = temporary / f"render-{index}-{dpi}-{page_number}.png"
                    _run(
                        [
                            renderer,
                            "draw",
                            "-q",
                            "-F",
                            "png",
                            "-r",
                            str(dpi),
                            "-o",
                            str(output),
                            str(pdf),
                            str(page_number),
                        ]
                    )
                    rendered = output.read_bytes()
                    width, height = _png_dimensions(rendered)
                    rendered_document.update(page_number.to_bytes(4, "big"))
                    rendered_document.update(width.to_bytes(4, "big"))
                    rendered_document.update(height.to_bytes(4, "big"))
                    rendered_document.update(len(rendered).to_bytes(8, "big"))
                    rendered_document.update(rendered)
            render_hashes.append(rendered_document.hexdigest())

            if vector_expectations is not None:
                vector_hashes.append(
                    inspect_vector_pdf_structure(
                        pdf.read_bytes(), vector_expectations
                    ).structure_sha256
                )

            extracted = _run([extractor, "-enc", "UTF-8", str(pdf), "-"])
            normalized = _normalized_text(extracted)
            if normalized != expected_text:
                raise PdfDifferentialError(
                    f"extracted text differs: expected {expected_text!r}, observed {normalized!r}"
                )
            text_hashes.append(hashlib.sha256(normalized.encode("utf-8")).hexdigest())

    if (
        len(set(render_hashes)) != 1
        or len(set(text_hashes)) != 1
        or (vector_hashes and len(set(vector_hashes)) != 1)
    ):
        raise PdfDifferentialError("independent renderer/extractor results differ across PDFs")
    return PdfDifferentialResult(
        expected_pages, render_hashes[0], text_hashes[0], normalized_dpis
    )


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pdf", action="append", required=True, type=Path)
    parser.add_argument("--expected-text", required=True)
    parser.add_argument("--expected-pages", required=True, type=int)
    parser.add_argument("--mutool")
    parser.add_argument("--pdftotext")
    parser.add_argument("--pdfinfo")
    parser.add_argument("--render-dpi", action="append", type=int)
    parser.add_argument("--expected-vector-forms", type=int)
    parser.add_argument("--expected-vector-ext-gstates", type=int)
    parser.add_argument("--expected-vector-dos", type=int)
    arguments = parser.parse_args()
    try:
        vector_values = (
            arguments.expected_vector_forms,
            arguments.expected_vector_ext_gstates,
            arguments.expected_vector_dos,
        )
        if any(value is not None for value in vector_values) and not all(
            value is not None for value in vector_values
        ):
            raise PdfDifferentialError(
                "all expected vector structure counts must be supplied together"
            )
        vector_expectations = (
            VectorPdfExpectations(
                arguments.expected_vector_forms,
                arguments.expected_vector_ext_gstates,
                arguments.expected_vector_dos,
                arguments.expected_pages,
            )
            if all(value is not None for value in vector_values)
            else None
        )
        result = verify_pdf_differential(
            arguments.pdf,
            expected_text=arguments.expected_text,
            expected_pages=arguments.expected_pages,
            mutool=arguments.mutool,
            pdftotext=arguments.pdftotext,
            pdfinfo=arguments.pdfinfo,
            render_dpis=tuple(arguments.render_dpi or (72,)),
            vector_expectations=vector_expectations,
        )
    except (OSError, PdfDifferentialError) as error:
        parser.error(str(error))
    print(
        f"pages={result.page_count} render_dpis={','.join(map(str, result.render_dpis))} "
        f"render_sha256={result.render_sha256} "
        f"text_sha256={result.extracted_text_sha256}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
