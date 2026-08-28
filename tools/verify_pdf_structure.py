#!/usr/bin/env python3
"""Independently verify the private MI4-07 PDF metadata/navigation graph."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from dataclasses import dataclass
from datetime import datetime
from decimal import Decimal
from pathlib import Path
from typing import Any


class PdfValidationError(ValueError):
    """The serialized PDF disagrees with the closed MI4-07 expectation."""


@dataclass(frozen=True)
class PdfName:
    value: str


@dataclass(frozen=True)
class PdfRef:
    number: int


@dataclass(frozen=True)
class PdfString:
    value: bytes
    syntax: str


@dataclass(frozen=True)
class ParsedObject:
    number: int
    raw: bytes
    value: Any
    stream: bytes | None


_WHITESPACE = b"\x00\x09\x0a\x0c\x0d\x20"
_DELIMITERS = b"()<>[]{}/%"
_SCALE = 65_536
_JSON_SAFE_INTEGER_MAX = 9_007_199_254_740_991
_GRANDFATHERED_LANGUAGE_TAGS = {
    value.lower(): value
    for value in (
        "art-lojban", "cel-gaulish", "en-GB-oed", "i-ami", "i-bnn",
        "i-default", "i-enochian", "i-hak", "i-klingon", "i-lux",
        "i-mingo", "i-navajo", "i-pwn", "i-tao", "i-tay", "i-tsu",
        "no-bok", "no-nyn", "sgn-BE-FR", "sgn-BE-NL", "sgn-CH-DE",
        "zh-guoyu", "zh-hakka", "zh-min", "zh-min-nan", "zh-xiang",
    )
}


class PdfParser:
    """A strict parser for the deterministic PDF subset emitted by MI4-07."""

    def __init__(self, data: bytes):
        self.data = data
        self.pos = 0

    def skip_space(self) -> None:
        while self.pos < len(self.data):
            byte = self.data[self.pos]
            if byte in _WHITESPACE:
                self.pos += 1
            elif byte == ord("%"):
                end = self.data.find(b"\n", self.pos)
                self.pos = len(self.data) if end < 0 else end + 1
            else:
                break

    def parse(self) -> Any:
        self.skip_space()
        if self.data.startswith(b"<<", self.pos):
            return self.parse_dictionary()
        if self.data.startswith(b"[", self.pos):
            return self.parse_array()
        if self.data.startswith(b"(", self.pos):
            return PdfString(self.parse_literal_string(), "literal")
        if self.data.startswith(b"<", self.pos):
            return PdfString(self.parse_hex_string(), "hex")
        if self.data.startswith(b"/", self.pos):
            return PdfName(self.parse_name())
        for spelling, value in ((b"true", True), (b"false", False), (b"null", None)):
            if self._keyword(spelling):
                self.pos += len(spelling)
                return value
        if self.pos < len(self.data) and self.data[self.pos] in b"+-0123456789.":
            first = self.parse_number()
            if isinstance(first, int):
                after_first = self.pos
                self.skip_space()
                try:
                    second = self.parse_number()
                except PdfValidationError:
                    self.pos = after_first
                    return first
                if isinstance(second, int):
                    self.skip_space()
                    if second == 0 and self._keyword(b"R"):
                        self.pos += 1
                        if first <= 0:
                            raise PdfValidationError("invalid indirect object reference")
                        return PdfRef(first)
                self.pos = after_first
            return first
        raise PdfValidationError(f"unsupported PDF token at byte {self.pos}")

    def _keyword(self, value: bytes) -> bool:
        if not self.data.startswith(value, self.pos):
            return False
        end = self.pos + len(value)
        return end == len(self.data) or self.data[end] in _WHITESPACE + _DELIMITERS

    def parse_dictionary(self) -> dict[str, Any]:
        if not self.data.startswith(b"<<", self.pos):
            raise PdfValidationError("expected dictionary")
        self.pos += 2
        output: dict[str, Any] = {}
        while True:
            self.skip_space()
            if self.data.startswith(b">>", self.pos):
                self.pos += 2
                return output
            key = self.parse()
            if not isinstance(key, PdfName):
                raise PdfValidationError("dictionary key is not a PDF name")
            if key.value in output:
                raise PdfValidationError(f"duplicate dictionary key /{key.value}")
            output[key.value] = self.parse()

    def parse_array(self) -> list[Any]:
        self.pos += 1
        output: list[Any] = []
        while True:
            self.skip_space()
            if self.pos >= len(self.data):
                raise PdfValidationError("unterminated array")
            if self.data[self.pos] == ord("]"):
                self.pos += 1
                return output
            output.append(self.parse())

    def parse_name(self) -> str:
        self.pos += 1
        start = self.pos
        while self.pos < len(self.data) and self.data[self.pos] not in _WHITESPACE + _DELIMITERS:
            self.pos += 1
        raw = self.data[start : self.pos]
        if not raw:
            raise PdfValidationError("empty PDF name")
        try:
            return raw.decode("ascii")
        except UnicodeDecodeError as error:
            raise PdfValidationError("non-ASCII PDF name") from error

    def parse_literal_string(self) -> bytes:
        self.pos += 1
        depth = 1
        output = bytearray()
        while self.pos < len(self.data):
            byte = self.data[self.pos]
            self.pos += 1
            if byte == ord("\\"):
                if self.pos >= len(self.data):
                    raise PdfValidationError("unterminated literal escape")
                escaped = self.data[self.pos]
                self.pos += 1
                replacements = {
                    ord("n"): b"\n",
                    ord("r"): b"\r",
                    ord("t"): b"\t",
                    ord("b"): b"\x08",
                    ord("f"): b"\x0c",
                }
                if escaped in replacements:
                    output.extend(replacements[escaped])
                elif escaped in b"()\\":
                    output.append(escaped)
                elif escaped in b"\r\n":
                    if escaped == ord("\r") and self.pos < len(self.data) and self.data[self.pos] == ord("\n"):
                        self.pos += 1
                elif escaped in b"01234567":
                    digits = bytearray([escaped])
                    while len(digits) < 3 and self.pos < len(self.data) and self.data[self.pos] in b"01234567":
                        digits.append(self.data[self.pos])
                        self.pos += 1
                    value = int(digits.decode("ascii"), 8)
                    if value > 255:
                        raise PdfValidationError("literal octal escape exceeds one byte")
                    output.append(value)
                else:
                    output.append(escaped)
            elif byte == ord("("):
                depth += 1
                output.append(byte)
            elif byte == ord(")"):
                depth -= 1
                if depth == 0:
                    return bytes(output)
                output.append(byte)
            else:
                output.append(byte)
        raise PdfValidationError("unterminated literal string")

    def parse_hex_string(self) -> bytes:
        self.pos += 1
        digits = bytearray()
        while self.pos < len(self.data) and self.data[self.pos] != ord(">"):
            byte = self.data[self.pos]
            self.pos += 1
            if byte in _WHITESPACE:
                continue
            if byte not in b"0123456789abcdefABCDEF":
                raise PdfValidationError("invalid hexadecimal string")
            digits.append(byte)
        if self.pos >= len(self.data):
            raise PdfValidationError("unterminated hexadecimal string")
        self.pos += 1
        if len(digits) % 2:
            digits.append(ord("0"))
        return bytes.fromhex(digits.decode("ascii"))

    def parse_number(self) -> int | Decimal:
        start = self.pos
        while self.pos < len(self.data) and self.data[self.pos] in b"+-0123456789.":
            self.pos += 1
        raw = self.data[start : self.pos]
        if not raw or raw in {b"+", b"-", b".", b"+.", b"-."}:
            raise PdfValidationError("invalid number")
        try:
            text = raw.decode("ascii")
            return Decimal(text) if "." in text else int(text)
        except (ValueError, ArithmeticError) as error:
            raise PdfValidationError("invalid number") from error


def _exact_keys(value: Any, expected: set[str], label: str) -> None:
    if not isinstance(value, dict):
        raise PdfValidationError(f"{label} is not a dictionary")
    if set(value) != expected:
        raise PdfValidationError(
            f"{label} keys differ: missing={sorted(expected - set(value))}, "
            f"extra={sorted(set(value) - expected)}"
        )


def _name(value: Any, expected: str, label: str) -> None:
    if value != PdfName(expected):
        raise PdfValidationError(f"{label} is not /{expected}")


def _ref(value: Any, label: str) -> int:
    if not isinstance(value, PdfRef):
        raise PdfValidationError(f"{label} is not an indirect reference")
    return value.number


def _integer(value: Any, label: str) -> int:
    if not isinstance(value, int) or isinstance(value, bool):
        raise PdfValidationError(f"{label} is not an integer")
    return value


def _utf16_text(value: Any, label: str) -> str:
    if (
        not isinstance(value, PdfString)
        or value.syntax != "hex"
        or not value.value.startswith(b"\xfe\xff")
    ):
        raise PdfValidationError(f"{label} is not a UTF-16BE hexadecimal text string")
    try:
        return value.value[2:].decode("utf-16-be")
    except UnicodeDecodeError as error:
        raise PdfValidationError(f"{label} is not valid UTF-16BE text") from error


def _literal_ascii_text(value: Any, label: str) -> str:
    if not isinstance(value, PdfString) or value.syntax != "literal":
        raise PdfValidationError(f"{label} is not a literal text string")
    try:
        return value.value.decode("ascii")
    except UnicodeDecodeError as error:
        raise PdfValidationError(f"{label} is not ASCII text") from error


def _fixed(value: Any, label: str) -> int:
    if not isinstance(value, (int, Decimal)) or isinstance(value, bool):
        raise PdfValidationError(f"{label} is not numeric")
    scaled = Decimal(value) * _SCALE
    if scaled != scaled.to_integral_value():
        raise PdfValidationError(f"{label} is not an exact 1/65536 value")
    return int(scaled)


def _canonical_language(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > 255:
        raise PdfValidationError(f"{label} is not a bounded language tag")
    try:
        value.encode("ascii")
    except UnicodeEncodeError as error:
        raise PdfValidationError(f"{label} is not ASCII") from error
    parts = value.split("-")
    if any(
        not part
        or len(part) > 8
        or not all(character.isascii() and character.isalnum() for character in part)
        for part in parts
    ):
        raise PdfValidationError(f"{label} has an invalid subtag")
    grandfathered = _GRANDFATHERED_LANGUAGE_TAGS.get(value.lower())
    if grandfathered is not None:
        canonical = grandfathered
    elif parts[0].lower() == "x":
        if len(parts) < 2:
            raise PdfValidationError(f"{label} has empty private use")
        canonical = "-".join(part.lower() for part in parts)
    else:
        primary = parts[0]
        if not primary.isalpha() or not 2 <= len(primary) <= 8:
            raise PdfValidationError(f"{label} has an invalid primary language")
        index = 1
        output = [primary.lower()]
        if len(primary) <= 3:
            extlang_count = 0
            while (
                index < len(parts)
                and extlang_count < 3
                and len(parts[index]) == 3
                and parts[index].isalpha()
            ):
                output.append(parts[index].lower())
                index += 1
                extlang_count += 1
        if index < len(parts) and len(parts[index]) == 4 and parts[index].isalpha():
            output.append(parts[index].title())
            index += 1
        if index < len(parts) and (
            (len(parts[index]) == 2 and parts[index].isalpha())
            or (len(parts[index]) == 3 and parts[index].isdigit())
        ):
            output.append(parts[index].upper())
            index += 1
        variants: set[str] = set()
        while index < len(parts) and (
            5 <= len(parts[index]) <= 8
            or (len(parts[index]) == 4 and parts[index][0].isdigit())
        ):
            variant = parts[index].lower()
            if variant in variants:
                raise PdfValidationError(f"{label} has a duplicate variant")
            variants.add(variant)
            output.append(variant)
            index += 1
        extensions: list[tuple[str, list[str]]] = []
        singletons: set[str] = set()
        while index < len(parts) and len(parts[index]) == 1 and parts[index].lower() != "x":
            singleton = parts[index].lower()
            if singleton in singletons:
                raise PdfValidationError(f"{label} has a duplicate extension singleton")
            singletons.add(singleton)
            index += 1
            start = index
            extension: list[str] = []
            while index < len(parts) and 2 <= len(parts[index]) <= 8:
                extension.append(parts[index].lower())
                index += 1
            if index == start:
                raise PdfValidationError(f"{label} has an empty extension")
            extensions.append((singleton, extension))
        for singleton, extension in sorted(extensions):
            output.append(singleton)
            output.extend(extension)
        if index < len(parts) and parts[index].lower() == "x":
            index += 1
            if index == len(parts):
                raise PdfValidationError(f"{label} has empty private use")
            output.append("x")
            output.extend(part.lower() for part in parts[index:])
            index = len(parts)
        if index != len(parts):
            raise PdfValidationError(f"{label} has an invalid language tail")
        canonical = "-".join(output)
    if canonical != value:
        raise PdfValidationError(f"{label} is not canonical")
    return canonical


def _metadata_text(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise PdfValidationError(f"{label} is not a nonempty metadata string")
    whitespace = {
        0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x0020, 0x0085,
        0x00A0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000,
        *range(0x2000, 0x200B),
    }
    if all(ord(character) in whitespace for character in value) or any(
        ord(character) <= 0x1F
        or 0x7F <= ord(character) <= 0x9F
        or ord(character) in {0xFFFE, 0xFFFF}
        for character in value
    ):
        raise PdfValidationError(f"{label} is not a valid metadata string")
    return value


def _parse_xref(pdf: bytes) -> tuple[dict[int, ParsedObject], dict[str, Any]]:
    if not pdf.startswith(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n") or not pdf.endswith(b"%%EOF\n"):
        raise PdfValidationError("PDF header or EOF marker differs")
    marker = b"startxref\n"
    position = pdf.rfind(marker)
    if position < 0:
        raise PdfValidationError("missing startxref")
    line_end = pdf.find(b"\n", position + len(marker))
    if line_end < 0:
        raise PdfValidationError("unterminated startxref offset")
    raw_xref_offset = pdf[position + len(marker) : line_end]
    if (
        not raw_xref_offset.isdigit()
        or (len(raw_xref_offset) > 1 and raw_xref_offset.startswith(b"0"))
    ):
        raise PdfValidationError("invalid startxref offset")
    if pdf[line_end + 1 :] != b"%%EOF\n":
        raise PdfValidationError("bytes between startxref and EOF differ")
    xref_offset = int(raw_xref_offset)
    if not 0 <= xref_offset < position:
        raise PdfValidationError("startxref offset is outside the PDF body")
    if pdf[xref_offset : xref_offset + 5] != b"xref\n":
        raise PdfValidationError("startxref does not address a classic xref table")
    cursor = xref_offset + 5
    header_end = pdf.find(b"\n", cursor)
    if header_end < 0:
        raise PdfValidationError("unterminated xref subsection header")
    header = pdf[cursor:header_end].split(b" ")
    if len(header) != 2 or header[0] != b"0" or not header[1].isdigit():
        raise PdfValidationError("xref must contain one dense subsection from object zero")
    try:
        count = int(header[1])
    except ValueError as error:
        raise PdfValidationError("invalid xref size") from error
    if count < 2 or count > len(pdf) // 20:
        raise PdfValidationError("xref does not contain the required object range")
    cursor = header_end + 1
    offsets: list[int] = []
    for index in range(count):
        line_end = pdf.find(b"\n", cursor)
        if line_end < 0:
            raise PdfValidationError("unterminated xref row")
        line = pdf[cursor:line_end]
        cursor = line_end + 1
        if len(line) != 19:
            raise PdfValidationError("xref row has the wrong width")
        expected_tail = b"65535 f " if index == 0 else b"00000 n "
        if line[10:11] != b" " or line[11:] != expected_tail:
            raise PdfValidationError("xref row has the wrong generation/status")
        if not line[:10].isdigit():
            raise PdfValidationError("xref offset is not decimal")
        offsets.append(int(line[:10]))
    if offsets[0] != 0 or offsets[1:] != sorted(offsets[1:]) or len(set(offsets[1:])) != count - 1:
        raise PdfValidationError("xref object offsets are not dense and increasing")
    if not pdf.startswith(b"trailer\n", cursor):
        raise PdfValidationError("missing trailer after xref")
    trailer_parser = PdfParser(pdf[cursor + len(b"trailer\n") : position])
    trailer = trailer_parser.parse()
    trailer_parser.skip_space()
    if trailer_parser.pos != len(trailer_parser.data) or not isinstance(trailer, dict):
        raise PdfValidationError("invalid trailer dictionary")
    _exact_keys(trailer, {"Info", "Root", "Size"}, "trailer")
    if _integer(trailer["Size"], "trailer /Size") != count or _ref(
        trailer["Root"], "trailer /Root"
    ) != 1:
        raise PdfValidationError("trailer root/size mismatch")

    objects: dict[int, ParsedObject] = {}
    expected_offset = len(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    for number in range(1, count):
        offset = offsets[number]
        if offset != expected_offset:
            raise PdfValidationError(f"object {number} is not contiguous with its predecessor")
        prefix = f"{number} 0 obj\n".encode("ascii")
        if not pdf.startswith(prefix, offset):
            raise PdfValidationError(f"xref offset for object {number} is wrong")
        start = offset + len(prefix)
        end = pdf.find(b"\nendobj\n", start)
        if end < 0:
            raise PdfValidationError(f"object {number} is unterminated")
        raw = pdf[start:end]
        parser = PdfParser(raw)
        value = parser.parse()
        parser.skip_space()
        stream: bytes | None = None
        if parser.data.startswith(b"stream\n", parser.pos):
            if (
                not isinstance(value, dict)
                or not isinstance(value.get("Length"), int)
                or isinstance(value.get("Length"), bool)
                or value["Length"] < 0
            ):
                raise PdfValidationError(f"object {number} stream length is not direct")
            content_start = parser.pos + len(b"stream\n")
            content_end = content_start + value["Length"]
            stream = raw[content_start:content_end]
            if len(stream) != value["Length"] or raw[content_end:] != b"\nendstream":
                raise PdfValidationError(f"object {number} stream framing differs")
        elif parser.pos != len(raw):
            raise PdfValidationError(f"object {number} has trailing payload")
        objects[number] = ParsedObject(number, raw, value, stream)
        expected_offset = end + len(b"\nendobj\n")
    if expected_offset != xref_offset:
        raise PdfValidationError("xref is not contiguous with the final object")
    if _ref(trailer["Info"], "trailer /Info") not in objects:
        raise PdfValidationError("trailer /Info is missing")
    return objects, trailer


def _xml_text(value: str) -> str:
    return value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def _xml_attribute(value: str) -> str:
    return _xml_text(value).replace('"', "&quot;")


def _xmp_alt(property_name: str, value: str, language: str) -> str:
    result = (
        f"<{property_name}><rdf:Alt><rdf:li xml:lang=\"x-default\">"
        f"{_xml_text(value)}</rdf:li>"
    )
    if language != "x-default":
        result += (
            f"<rdf:li xml:lang=\"{_xml_attribute(language)}\">"
            f"{_xml_text(value)}</rdf:li>"
        )
    return result + f"</rdf:Alt></{property_name}>"


def expected_xmp(expectation: dict[str, Any]) -> bytes:
    metadata = expectation["metadata"]
    language = expectation["document_language"]
    producer = f'{expectation["engine"]["name"]} {expectation["engine"]["version"]}'
    properties = ""
    if metadata["title"] is not None:
        properties += _xmp_alt("dc:title", metadata["title"], language)
    if metadata["author"] is not None:
        properties += (
            "<dc:creator><rdf:Seq><rdf:li>"
            + _xml_text(metadata["author"])
            + "</rdf:li></rdf:Seq></dc:creator>"
        )
    if metadata["subject"] is not None:
        properties += _xmp_alt("dc:description", metadata["subject"], language)
    if metadata["keywords"]:
        properties += "<dc:subject><rdf:Bag>"
        properties += "".join(
            f"<rdf:li>{_xml_text(keyword)}</rdf:li>" for keyword in metadata["keywords"]
        )
        properties += "</rdf:Bag></dc:subject><pdf:Keywords>"
        properties += _xml_text("; ".join(metadata["keywords"])) + "</pdf:Keywords>"
    if metadata["identifier"] is not None:
        properties += f'<dc:identifier>{_xml_text(metadata["identifier"])}</dc:identifier>'
    if metadata["created"] is not None:
        properties += f'<xmp:CreateDate>{metadata["created"]}</xmp:CreateDate>'
    if metadata["modified"] is not None:
        properties += f'<xmp:ModifyDate>{metadata["modified"]}</xmp:ModifyDate>'
    properties += f'<dc:language><rdf:Bag><rdf:li>{_xml_text(language)}</rdf:li></rdf:Bag></dc:language>'
    properties += f'<pdf:Producer>{_xml_text(producer)}</pdf:Producer>'
    return (
        '<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF '
        'xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description '
        'rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" '
        'xmlns:pdf="http://ns.adobe.com/pdf/1.3/" '
        f'xmlns:xmp="http://ns.adobe.com/xap/1.0/">{properties}'
        '</rdf:Description></rdf:RDF></x:xmpmeta>'
    ).encode("utf-8")


def _pdf_date(value: str) -> str:
    return "D:" + value[0:4] + value[5:7] + value[8:10] + value[11:13] + value[14:16] + value[17:19] + "Z"


def _expectation_shape(expectation: dict[str, Any]) -> None:
    _exact_keys(
        expectation,
        {
            "destinations",
            "document_language",
            "engine",
            "language_paints",
            "links",
            "metadata",
            "outline",
            "pages",
        },
        "expectation",
    )

    def expect_dict(value: Any, label: str) -> dict[str, Any]:
        if not isinstance(value, dict):
            raise PdfValidationError(f"{label} is not an object")
        return value

    def expect_list(value: Any, label: str) -> list[Any]:
        if not isinstance(value, list):
            raise PdfValidationError(f"{label} is not an array")
        return value

    def expect_string(value: Any, label: str) -> str:
        if not isinstance(value, str) or not value:
            raise PdfValidationError(f"{label} is not a nonempty string")
        return value

    def expect_integer(value: Any, label: str, *, minimum: int = 0) -> int:
        if not isinstance(value, int) or isinstance(value, bool) or value < minimum:
            raise PdfValidationError(f"{label} is not an integer >= {minimum}")
        return value

    engine = expect_dict(expectation["engine"], "expectation engine")
    _exact_keys(engine, {"name", "version"}, "expectation engine")
    expect_string(engine["name"], "expectation engine name")
    expect_string(engine["version"], "expectation engine version")

    language = _canonical_language(
        expectation["document_language"], "expectation document language"
    )

    metadata = expect_dict(expectation["metadata"], "expectation metadata")
    _exact_keys(
        metadata,
        {"author", "created", "identifier", "keywords", "modified", "subject", "title"},
        "expectation metadata",
    )
    for key in ("author", "identifier", "subject", "title"):
        value = metadata[key]
        if value is not None:
            _metadata_text(value, f"expectation metadata {key}")
    keywords = expect_list(metadata["keywords"], "expectation metadata keywords")
    for index, keyword in enumerate(keywords):
        _metadata_text(keyword, f"expectation metadata keyword {index}")
    if keywords != sorted(set(keywords), key=lambda value: value.encode("utf-8")):
        raise PdfValidationError("expectation metadata keywords are not unique UTF-8 order")
    for key in ("created", "modified"):
        value = metadata[key]
        if value is None:
            continue
        expect_string(value, f"expectation metadata {key}")
        if re.fullmatch(r"[0-9]{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12][0-9]|3[01])T(?:[01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]Z", value) is None:
            raise PdfValidationError(f"expectation metadata {key} is not a UTC-second timestamp")
        try:
            datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
        except ValueError as error:
            raise PdfValidationError(
                f"expectation metadata {key} is not a calendar timestamp"
            ) from error
    if metadata["created"] is not None and metadata["modified"] is not None and metadata["modified"] < metadata["created"]:
        raise PdfValidationError("expectation metadata modification precedes creation")

    pages = expect_list(expectation["pages"], "expectation pages")
    if not pages:
        raise PdfValidationError("expectation pages are empty")
    for index, page_value in enumerate(pages):
        page = expect_dict(page_value, f"expectation page {index}")
        _exact_keys(page, {"height", "page_index", "width"}, f"expectation page {index}")
        if expect_integer(page["page_index"], f"expectation page {index} index") != index:
            raise PdfValidationError("expected page indexes are not dense")
        width = expect_integer(
            page["width"], f"expectation page {index} width", minimum=1
        )
        height = expect_integer(
            page["height"], f"expectation page {index} height", minimum=1
        )
        if width > _JSON_SAFE_INTEGER_MAX or height > _JSON_SAFE_INTEGER_MAX:
            raise PdfValidationError(
                f"expectation page {index} geometry is not JSON-safe"
            )

    destinations = expect_list(expectation["destinations"], "expectation destinations")
    names: list[bytes] = []
    destination_names: set[str] = set()
    anchor_pattern = re.compile(r"[A-Za-z_][A-Za-z0-9_.:-]*")
    for index, destination_value in enumerate(destinations):
        destination = expect_dict(destination_value, f"expectation destination {index}")
        _exact_keys(destination, {"name", "page_index", "view"}, f"expectation destination {index}")
        name = expect_string(destination["name"], f"expectation destination {index} name")
        if anchor_pattern.fullmatch(name) is None:
            raise PdfValidationError(f"expectation destination {index} name is not an anchor ID")
        try:
            encoded_name = name.encode("ascii")
        except UnicodeEncodeError as error:
            raise PdfValidationError(
                f"expectation destination {index} name is not ASCII"
            ) from error
        names.append(encoded_name)
        destination_names.add(name)
        page_index = expect_integer(
            destination["page_index"], f"expectation destination {index} page index"
        )
        if page_index >= len(pages):
            raise PdfValidationError(f"expectation destination {index} page is out of range")
        view = expect_dict(destination["view"], f"expectation destination {index} view")
        kind = expect_string(view.get("kind"), f"expectation destination {index} view kind")
        if kind == "xyz":
            _exact_keys(view, {"kind", "x", "y"}, f"expectation destination {index} view")
            x = expect_integer(view["x"], f"expectation destination {index} x")
            y = expect_integer(view["y"], f"expectation destination {index} y")
            if x > pages[page_index]["width"] or y > pages[page_index]["height"]:
                raise PdfValidationError(f"expectation destination {index} coordinate is out of bounds")
        elif kind == "fit_page":
            _exact_keys(view, {"kind"}, f"expectation destination {index} view")
        elif kind == "fit_width":
            _exact_keys(view, {"kind", "top"}, f"expectation destination {index} view")
            top = view["top"]
            if top is not None:
                expect_integer(top, f"expectation destination {index} top")
                if top > pages[page_index]["height"]:
                    raise PdfValidationError(f"expectation destination {index} top is out of bounds")
        else:
            raise PdfValidationError(f"expectation destination {index} view kind is unsupported")
    if names != sorted(set(names)):
        raise PdfValidationError("expected destinations are not unique UTF-8 order")

    paints = expect_list(expectation["language_paints"], "expectation language paints")
    previous_paint_page = -1
    for index, paint_value in enumerate(paints):
        paint = expect_dict(paint_value, f"expectation language paint {index}")
        _exact_keys(paint, {"actual_text", "language", "page_index"}, f"expectation language paint {index}")
        page_index = expect_integer(paint["page_index"], f"expectation language paint {index} page index")
        if page_index >= len(pages) or page_index < previous_paint_page:
            raise PdfValidationError("expectation language-paint page order is invalid")
        previous_paint_page = page_index
        _canonical_language(
            paint["language"], f"expectation language paint {index} language"
        )
        if paint["actual_text"] is not None:
            expect_string(paint["actual_text"], f"expectation language paint {index} actual text")

    links = expect_list(expectation["links"], "expectation links")
    previous_link_page = -1
    for index, link_value in enumerate(links):
        link = expect_dict(link_value, f"expectation link {index}")
        _exact_keys(link, {"destination", "page_index", "rect"}, f"expectation link {index}")
        destination = expect_string(link["destination"], f"expectation link {index} destination")
        if destination not in destination_names:
            raise PdfValidationError(f"expectation link {index} destination is unresolved")
        page_index = expect_integer(link["page_index"], f"expectation link {index} page index")
        if page_index >= len(pages) or page_index < previous_link_page:
            raise PdfValidationError("expectation link page order is invalid")
        previous_link_page = page_index
        rect = expect_list(link["rect"], f"expectation link {index} rect")
        if len(rect) != 4:
            raise PdfValidationError(f"expectation link {index} rect does not have four coordinates")
        coordinates = [
            expect_integer(value, f"expectation link {index} rect coordinate {coordinate}")
            for coordinate, value in enumerate(rect)
        ]
        if (
            coordinates[0] >= coordinates[2]
            or coordinates[1] >= coordinates[3]
            or coordinates[2] > pages[page_index]["width"]
            or coordinates[3] > pages[page_index]["height"]
        ):
            raise PdfValidationError(f"expectation link {index} rect is empty or out of bounds")

    outline = expect_list(expectation["outline"], "expectation outline")
    parent_stack: list[int] = []
    source_node_ids: set[int] = set()
    outline_destinations: set[str] = set()
    previous_source_node_id = -1
    for index, entry_value in enumerate(outline):
        entry = expect_dict(entry_value, f"expectation outline entry {index}")
        _exact_keys(
            entry,
            {"destination", "label", "level", "outline_id", "parent_outline_id", "source_node_id"},
            f"expectation outline entry {index}",
        )
        if expect_integer(entry["outline_id"], f"expectation outline entry {index} ID") != index:
            raise PdfValidationError("expected outline IDs are not dense")
        level = expect_integer(entry["level"], f"expectation outline entry {index} level", minimum=1)
        if level > 6 or level > len(parent_stack) + 1:
            raise PdfValidationError("expected outline level skips a parent")
        expected_parent = None if level == 1 else parent_stack[level - 2]
        parent = entry["parent_outline_id"]
        if parent is not None:
            expect_integer(parent, f"expectation outline entry {index} parent")
        if parent != expected_parent:
            raise PdfValidationError("expected outline parent does not match preorder")
        parent_stack[level - 1 :] = [index]
        destination = expect_string(entry["destination"], f"expectation outline entry {index} destination")
        if destination not in destination_names:
            raise PdfValidationError(f"expectation outline entry {index} destination is unresolved")
        if destination in outline_destinations:
            raise PdfValidationError("expectation outline destinations are not unique")
        outline_destinations.add(destination)
        _metadata_text(entry["label"], f"expectation outline entry {index} label")
        source_node_id = expect_integer(entry["source_node_id"], f"expectation outline entry {index} source node ID")
        if source_node_id in source_node_ids or source_node_id <= previous_source_node_id:
            raise PdfValidationError(
                "expectation outline source node IDs are not strict preorder"
            )
        source_node_ids.add(source_node_id)
        previous_source_node_id = source_node_id


def _parse_destination_view(value: list[Any], page_by_object: dict[int, int]) -> tuple[int, dict[str, Any]]:
    if len(value) < 2:
        raise PdfValidationError("named destination array is incomplete")
    page_index = page_by_object.get(_ref(value[0], "destination page"))
    if page_index is None or not isinstance(value[1], PdfName):
        raise PdfValidationError("named destination page/view is invalid")
    if value[1].value == "XYZ" and len(value) == 5 and value[4] is None:
        return page_index, {"kind": "xyz", "x": _fixed(value[2], "destination x"), "y": _fixed(value[3], "destination y")}
    if value[1].value == "Fit" and len(value) == 2:
        return page_index, {"kind": "fit_page"}
    if value[1].value == "FitH" and len(value) == 3:
        return page_index, {"kind": "fit_width", "top": None if value[2] is None else _fixed(value[2], "destination top")}
    raise PdfValidationError("unsupported named destination view")


def _outline_descendant_count(outline: list[dict[str, Any]], index: int) -> int:
    level = outline[index]["level"]
    count = 0
    for candidate in outline[index + 1 :]:
        if candidate["level"] <= level:
            break
        count += 1
    return count


def _validate_content_stream(content: bytes, expected: list[dict[str, Any]]) -> list[dict[str, Any]]:
    observations: list[dict[str, Any]] = []
    cursor = 0
    suffix = b" BDC\n0 0 m 0 0 l S\nEMC\n"
    for paint in expected:
        if not content.startswith(b"/Span ", cursor):
            raise PdfValidationError("missing /Span language marked content")
        parser = PdfParser(content[cursor + len(b"/Span ") :])
        dictionary = parser.parse()
        cursor += len(b"/Span ") + parser.pos
        if not content.startswith(suffix, cursor):
            raise PdfValidationError("language marked-content commands differ")
        cursor += len(suffix)
        required = {"Lang"} | ({"ActualText"} if paint["actual_text"] is not None else set())
        _exact_keys(dictionary, required, "marked-content property list")
        language = _utf16_text(dictionary["Lang"], "marked-content /Lang")
        actual_text = _utf16_text(dictionary["ActualText"], "marked-content /ActualText") if "ActualText" in dictionary else None
        observed = {"actual_text": actual_text, "language": language, "page_index": paint["page_index"]}
        if observed != paint:
            raise PdfValidationError("marked-content language observation differs")
        observations.append(observed)
    if cursor != len(content):
        raise PdfValidationError("extra page content outside language spans")
    return observations


def verify_pdf_structure(pdf: bytes, expectation: dict[str, Any]) -> dict[str, Any]:
    """Return a canonical observation or raise ``PdfValidationError``."""

    _expectation_shape(expectation)
    objects, trailer = _parse_xref(pdf)
    pages_expected = expectation["pages"]
    links_expected = expectation["links"]
    outline_expected = expectation["outline"]
    annotation_start = 4 + 2 * len(pages_expected)
    info_number = annotation_start + len(links_expected)
    metadata_number = info_number + 1
    outline_root = metadata_number + 1 if outline_expected else None
    object_count = metadata_number if outline_root is None else outline_root + len(outline_expected)
    if sorted(objects) != list(range(1, object_count + 1)):
        raise PdfValidationError("PDF object allocation does not match the MI4-07 role plan")
    if _ref(trailer["Info"], "trailer /Info") != info_number:
        raise PdfValidationError("trailer /Info role is out of order")

    catalog = objects[1].value
    catalog_keys = {"Lang", "Metadata", "Names", "Pages", "Type"} | ({"Outlines"} if outline_expected else set())
    _exact_keys(catalog, catalog_keys, "catalog")
    _name(catalog["Type"], "Catalog", "catalog /Type")
    if _ref(catalog["Pages"], "catalog /Pages") != 2 or _ref(catalog["Metadata"], "catalog /Metadata") != metadata_number:
        raise PdfValidationError("catalog role reference differs")
    document_language = _utf16_text(catalog["Lang"], "catalog /Lang")
    if document_language != expectation["document_language"]:
        raise PdfValidationError("catalog /Lang differs from source language")
    _exact_keys(catalog["Names"], {"Dests"}, "catalog /Names")
    if _ref(catalog["Names"]["Dests"], "catalog destination tree") != 3:
        raise PdfValidationError("catalog destination name-tree reference differs")
    if outline_expected:
        if _ref(catalog["Outlines"], "catalog /Outlines") != outline_root:
            raise PdfValidationError("catalog outline-root reference differs")

    pages = objects[2].value
    _exact_keys(pages, {"Count", "Kids", "Type"}, "pages root")
    _name(pages["Type"], "Pages", "pages /Type")
    page_objects = [5 + 2 * index for index in range(len(pages_expected))]
    if (
        _integer(pages["Count"], "pages /Count") != len(page_objects)
        or not isinstance(pages["Kids"], list)
        or [_ref(item, "pages /Kids") for item in pages["Kids"]] != page_objects
    ):
        raise PdfValidationError("page tree differs")
    page_by_object = {number: index for index, number in enumerate(page_objects)}

    name_tree = objects[3].value
    _exact_keys(name_tree, {"Names"}, "destination name tree")
    values = name_tree["Names"]
    if not isinstance(values, list) or len(values) % 2:
        raise PdfValidationError("destination name tree has an odd value count")
    destinations: list[dict[str, Any]] = []
    for index in range(0, len(values), 2):
        name = _literal_ascii_text(values[index], "destination name")
        if not isinstance(values[index + 1], list):
            raise PdfValidationError("destination value is not an array")
        page_index, view = _parse_destination_view(values[index + 1], page_by_object)
        destinations.append({"name": name, "page_index": page_index, "view": view})
    if destinations != expectation["destinations"]:
        raise PdfValidationError("named destination registry differs")

    page_annots: list[list[int]] = []
    paints_by_page: list[list[dict[str, Any]]] = []
    paint_observations: list[dict[str, Any]] = []
    for page_index, expected_page in enumerate(pages_expected):
        expected_paints = [paint for paint in expectation["language_paints"] if paint["page_index"] == page_index]
        paints_by_page.append(expected_paints)
        content_number = 4 + 2 * page_index
        page_number = content_number + 1
        content_object = objects[content_number]
        _exact_keys(content_object.value, {"Length"}, f"page {page_index} content stream")
        if content_object.stream is None:
            raise PdfValidationError("page content object is not a stream")
        paint_observations.extend(_validate_content_stream(content_object.stream, expected_paints))
        page = objects[page_number].value
        expected_keys = {"Contents", "MediaBox", "Parent", "Resources", "Type"}
        expected_page_links = [annotation_start + index for index, link in enumerate(links_expected) if link["page_index"] == page_index]
        if expected_page_links:
            expected_keys.add("Annots")
        _exact_keys(page, expected_keys, f"page {page_index}")
        _name(page["Type"], "Page", f"page {page_index} /Type")
        if _ref(page["Parent"], "page /Parent") != 2 or _ref(page["Contents"], "page /Contents") != content_number or page["Resources"] != {}:
            raise PdfValidationError(f"page {page_index} role references differ")
        media = page["MediaBox"]
        if not isinstance(media, list) or len(media) != 4 or [_fixed(item, "MediaBox") for item in media] != [0, 0, expected_page["width"], expected_page["height"]]:
            raise PdfValidationError(f"page {page_index} MediaBox differs")
        raw_annots = page.get("Annots", [])
        if not isinstance(raw_annots, list):
            raise PdfValidationError(f"page {page_index} /Annots is not an array")
        observed_annots = [_ref(item, "page /Annots") for item in raw_annots]
        if observed_annots != expected_page_links:
            raise PdfValidationError(f"page {page_index} annotation order differs")
        page_annots.append(observed_annots)

    link_observations: list[dict[str, Any]] = []
    destination_names = {item["name"] for item in destinations}
    for index, expected_link in enumerate(links_expected):
        number = annotation_start + index
        link = objects[number].value
        _exact_keys(link, {"Border", "Dest", "Rect", "Subtype", "Type"}, f"link {index}")
        _name(link["Type"], "Annot", "link /Type")
        _name(link["Subtype"], "Link", "link /Subtype")
        destination = _literal_ascii_text(link["Dest"], "link /Dest")
        if destination not in destination_names or destination != expected_link["destination"]:
            raise PdfValidationError("link destination does not resolve through the name tree")
        if (
            not isinstance(link["Border"], list)
            or len(link["Border"]) != 3
            or any(_integer(value, f"link {index} /Border") != 0 for value in link["Border"])
            or not isinstance(link["Rect"], list)
            or len(link["Rect"]) != 4
        ):
            raise PdfValidationError("link rectangle/border differs")
        rectangle = [
            _fixed(value, f"link {index} /Rect") for value in link["Rect"]
        ]
        if rectangle != expected_link["rect"]:
            raise PdfValidationError(f"link {index} rectangle differs")
        link_observations.append(
            {
                "destination": destination,
                "object_number": number,
                "page_index": expected_link["page_index"],
                "rect": rectangle,
            }
        )

    metadata = expectation["metadata"]
    info = objects[info_number].value
    expected_info: dict[str, str] = {
        "Producer": f'{expectation["engine"]["name"]} {expectation["engine"]["version"]}'
    }
    for source, target in (("author", "Author"), ("subject", "Subject"), ("title", "Title")):
        if metadata[source] is not None:
            expected_info[target] = metadata[source]
    if metadata["keywords"]:
        expected_info["Keywords"] = "; ".join(metadata["keywords"])
    if metadata["created"] is not None:
        expected_info["CreationDate"] = _pdf_date(metadata["created"])
    if metadata["modified"] is not None:
        expected_info["ModDate"] = _pdf_date(metadata["modified"])
    _exact_keys(info, set(expected_info), "Info dictionary")
    info_observation = {
        key: (
            _literal_ascii_text(info[key], f"Info /{key}")
            if key in {"CreationDate", "ModDate"}
            else _utf16_text(info[key], f"Info /{key}")
        )
        for key in sorted(info)
    }
    if info_observation != {key: expected_info[key] for key in sorted(expected_info)}:
        raise PdfValidationError("Info values differ from source metadata")

    metadata_object = objects[metadata_number]
    _exact_keys(metadata_object.value, {"Length", "Subtype", "Type"}, "Metadata stream")
    _name(metadata_object.value["Type"], "Metadata", "Metadata /Type")
    _name(metadata_object.value["Subtype"], "XML", "Metadata /Subtype")
    xmp = expected_xmp(expectation)
    if metadata_object.stream != xmp:
        raise PdfValidationError("decompressed XMP bytes differ from typaxis.book-xmp/1")

    outline_observations: list[dict[str, Any]] = []
    if outline_expected:
        root = objects[outline_root].value
        _exact_keys(root, {"Count", "First", "Last", "Type"}, "outline root")
        _name(root["Type"], "Outlines", "outline root /Type")
        children: dict[int | None, list[int]] = {}
        sibling_positions: dict[int, int] = {}
        for entry in outline_expected:
            siblings = children.setdefault(entry["parent_outline_id"], [])
            sibling_positions[entry["outline_id"]] = len(siblings)
            siblings.append(entry["outline_id"])
        top = children.get(None, [])
        item_start = outline_root + 1
        if not top or _integer(root["Count"], "outline root /Count") != len(outline_expected) or _ref(root["First"], "outline root /First") != item_start + top[0] or _ref(root["Last"], "outline root /Last") != item_start + top[-1]:
            raise PdfValidationError("outline root relationships differ")
        for index, expected_entry in enumerate(outline_expected):
            number = item_start + index
            item = objects[number].value
            siblings = children[expected_entry["parent_outline_id"]]
            sibling_index = sibling_positions[index]
            direct_children = children.get(index, [])
            expected_keys = {"Dest", "Parent", "Title"}
            if sibling_index:
                expected_keys.add("Prev")
            if sibling_index + 1 < len(siblings):
                expected_keys.add("Next")
            if direct_children:
                expected_keys |= {"Count", "First", "Last"}
            _exact_keys(item, expected_keys, f"outline item {index}")
            parent_number = outline_root if expected_entry["parent_outline_id"] is None else item_start + expected_entry["parent_outline_id"]
            if _ref(item["Parent"], "outline /Parent") != parent_number:
                raise PdfValidationError("outline parent differs")
            if sibling_index and _ref(item["Prev"], "outline /Prev") != item_start + siblings[sibling_index - 1]:
                raise PdfValidationError("outline previous sibling differs")
            if sibling_index + 1 < len(siblings) and _ref(item["Next"], "outline /Next") != item_start + siblings[sibling_index + 1]:
                raise PdfValidationError("outline next sibling differs")
            descendants = _outline_descendant_count(outline_expected, index)
            if direct_children and (
                _ref(item["First"], "outline /First") != item_start + direct_children[0]
                or _ref(item["Last"], "outline /Last") != item_start + direct_children[-1]
                or _integer(item["Count"], "outline /Count") != descendants
            ):
                raise PdfValidationError("outline child relationships differ")
            title = _utf16_text(item["Title"], "outline /Title")
            destination = _literal_ascii_text(item["Dest"], "outline /Dest")
            if title != expected_entry["label"] or destination != expected_entry["destination"] or destination not in destination_names:
                raise PdfValidationError("outline title/destination differs")
            outline_observations.append(
                {
                    "destination": destination,
                    "object_number": number,
                    "outline_id": index,
                    "parent_outline_id": expected_entry["parent_outline_id"],
                    "source_node_id": expected_entry["source_node_id"],
                    "title": title,
                }
            )

    roles: dict[int, str] = {1: "catalog", 2: "pages", 3: "destination_name_tree"}
    for index in range(len(pages_expected)):
        roles[4 + index * 2] = f"page_content:{index}"
        roles[5 + index * 2] = f"page:{index}"
    for index in range(len(links_expected)):
        roles[annotation_start + index] = f"link_annotation:{index}"
    roles[info_number] = "info"
    roles[metadata_number] = "metadata"
    if outline_root is not None:
        roles[outline_root] = "outline_root"
        for index in range(len(outline_expected)):
            roles[outline_root + 1 + index] = f"outline_item:{index}"
    observation = {
        "algorithm": "typaxis.pdf-structure-observation/1",
        "catalog_language": document_language,
        "destinations": destinations,
        "info": info_observation,
        "language_paints": paint_observations,
        "links": link_observations,
        "objects": [
            {
                "object_number": number,
                "role": roles[number],
                "sha256": hashlib.sha256(objects[number].raw).hexdigest(),
            }
            for number in sorted(objects)
        ],
        "outline": outline_observations,
        "pdf_sha256": hashlib.sha256(pdf).hexdigest(),
        "xmp_sha256": hashlib.sha256(xmp).hexdigest(),
    }
    return observation


def load_expectation(path: Path) -> dict[str, Any]:
    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        output: dict[str, Any] = {}
        for key, value in pairs:
            if key in output:
                raise PdfValidationError(f"duplicate expectation member {key!r}")
            output[key] = value
        return output

    try:
        raw = path.read_bytes()
        value = json.loads(raw.decode("utf-8"), object_pairs_hook=reject_duplicates)
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise PdfValidationError(f"cannot read expectation: {error}") from error
    if not isinstance(value, dict):
        raise PdfValidationError("expectation root is not an object")
    canonical = json.dumps(value, ensure_ascii=False, separators=(",", ":"), sort_keys=True).encode("utf-8")
    if raw.rstrip(b"\n") != canonical:
        raise PdfValidationError("expectation is not canonical JSON")
    return value


def main(argv: list[str] | None = None) -> int:
    arguments = sys.argv[1:] if argv is None else argv
    if len(arguments) != 2:
        print("usage: verify_pdf_structure.py PDF EXPECTATION.json", file=sys.stderr)
        return 2
    try:
        pdf = Path(arguments[0]).read_bytes()
        expectation = load_expectation(Path(arguments[1]))
        observation = verify_pdf_structure(pdf, expectation)
    except (OSError, PdfValidationError) as error:
        print(f"PDF structure validation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(observation, ensure_ascii=False, separators=(",", ":"), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
