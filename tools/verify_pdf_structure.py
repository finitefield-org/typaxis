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


def _json_integer(value: Any, label: str, minimum: int = 0) -> int:
    if not isinstance(value, int) or isinstance(value, bool) or value < minimum:
        raise PdfValidationError(f"{label} is not an integer >= {minimum}")
    return value


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


def _tagged_manifest_shape(manifest: dict[str, Any]) -> None:
    _exact_keys(
        manifest,
        {
            "accessibility_profile", "algorithm", "contract", "destinations", "document_language",
            "engine", "fingerprints", "marked_content", "metadata", "outline", "pdf",
            "profile_id", "structure", "validators",
        },
        "tagged manifest",
    )
    if manifest["algorithm"] != "typaxis.tagged-pdf-manifest/1":
        raise PdfValidationError("tagged manifest algorithm differs")
    if manifest["contract"] != "typaxis.contract/1.4":
        raise PdfValidationError("tagged manifest contract differs")
    if manifest["accessibility_profile"] != "typaxis.pdfua1-profile/1":
        raise PdfValidationError("tagged accessibility profile differs")
    if manifest["profile_id"] != "typaxis.machine-pdf/production-book-1":
        raise PdfValidationError("tagged machine profile differs")
    language = _canonical_language(manifest["document_language"], "tagged document language")
    _exact_keys(manifest["engine"], {"name", "version"}, "tagged engine")
    if any(
        not isinstance(manifest["engine"][key], str) or not manifest["engine"][key]
        for key in ("name", "version")
    ):
        raise PdfValidationError("tagged engine identity differs")
    fingerprints = manifest["fingerprints"]
    _exact_keys(
        fingerprints,
        {
            "book_navigation_sha256", "destination_registry_sha256", "language_sha256", "limits_sha256",
            "marked_content_sha256", "metadata_sha256", "outline_sha256",
            "package_sha256", "pdf_observation_sha256", "pdf_sha256",
            "profile_sha256", "selected_binding_sha256", "semantic_sha256",
            "structure_registry_sha256", "xmp_sha256",
        },
        "tagged fingerprints",
    )
    if any(
        not isinstance(value, str) or re.fullmatch(r"[0-9a-f]{64}", value) is None
        for value in fingerprints.values()
    ):
        raise PdfValidationError("tagged fingerprint differs")
    metadata = manifest["metadata"]
    _exact_keys(
        metadata,
        {"author", "created", "identifier", "keywords", "modified", "subject", "title"},
        "tagged metadata",
    )
    for key in ("author", "identifier", "subject", "title"):
        value = metadata[key]
        if value is not None and (not isinstance(value, str) or not value.strip()):
            raise PdfValidationError("tagged metadata string differs")
    if metadata["title"] is None:
        raise PdfValidationError("tagged metadata title is missing")
    timestamp_pattern = r"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"
    for key in ("created", "modified"):
        value = metadata[key]
        if value is not None and (
            not isinstance(value, str) or re.fullmatch(timestamp_pattern, value) is None
        ):
            raise PdfValidationError("tagged metadata timestamp differs")
        if value is not None:
            try:
                datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
            except ValueError as error:
                raise PdfValidationError("tagged metadata timestamp differs") from error
    if (
        metadata["created"] is not None
        and metadata["modified"] is not None
        and metadata["modified"] < metadata["created"]
    ):
        raise PdfValidationError("tagged metadata modification precedes creation")
    keywords = metadata["keywords"]
    if (
        not isinstance(keywords, list)
        or any(not isinstance(value, str) or not value.strip() for value in keywords)
        or keywords != sorted(set(keywords), key=lambda value: value.encode("utf-8"))
    ):
        raise PdfValidationError("tagged metadata keywords differ")
    if manifest["validators"] != [
        "typaxis.tagged-pdf-validator/1",
        "verapdf-greenfield/1.30.2:ua1",
        "typaxis.matterhorn-assessment/1",
    ]:
        raise PdfValidationError("tagged validator registry differs")

    structure = manifest["structure"]
    if not isinstance(structure, list) or not structure:
        raise PdfValidationError("tagged structure registry is empty")
    allowed_roles = {
        "Caption", "Document", "Em", "Exercise", "Figure", "Formula",
        "H1", "H2", "H3", "H4", "H5", "H6", "L", "LBody", "LI",
        "Lbl", "Link", "Note", "P", "Proof", "Reference", "Result",
        "Span", "Strong", "TBody", "TD", "TH", "THead", "TR", "Table",
    }
    for index, node in enumerate(structure):
        _exact_keys(
            node,
            {
                "accessible_name", "actual_text", "alternative", "children", "language",
                "list_numbering", "marker", "outline_ids", "owner", "paint_required", "parent",
                "related_nodes", "role", "source_span", "structure_id", "structure_node_id",
                "table",
            },
            f"structure node {index}",
        )
        if (
            _json_integer(node["structure_node_id"], f"structure node {index} ID") != index
            or not isinstance(node["role"], str)
            or node["role"] not in allowed_roles
            or not isinstance(node["paint_required"], bool)
        ):
            raise PdfValidationError("structure node ID/role registry differs")
        if _canonical_language(node["language"], f"structure node {index} language") != node["language"]:
            raise PdfValidationError("structure language differs")
        if node["role"] == "L":
            if not isinstance(node["list_numbering"], str) or node["list_numbering"] not in {"decimal", "disc"}:
                raise PdfValidationError("List numbering differs")
        elif node["list_numbering"] is not None:
            raise PdfValidationError("non-List carries List numbering")
        owner = node["owner"]
        owner_kind = owner.get("kind") if isinstance(owner, dict) else None
        if (
            not isinstance(owner_kind, str)
            or owner_kind not in {"source", "generated"}
        ):
            raise PdfValidationError("structure owner kind differs")
        if owner_kind == "source":
            _exact_keys(owner, {"kind", "node_id"}, f"structure node {index} source owner")
            if _json_integer(owner["node_id"], f"structure node {index} source owner") < 0:
                raise PdfValidationError("source structure owner differs")
        else:
            _exact_keys(
                owner,
                {"kind", "ordinal", "owner_node_id", "slot"},
                f"structure node {index} generated owner",
            )
            if (
                _json_integer(owner["ordinal"], f"structure node {index} generated ordinal") != 0
                or _json_integer(
                    owner["owner_node_id"], f"structure node {index} generated owner"
                ) < 0
                or not isinstance(owner["slot"], str)
                or owner["slot"] not in {
                    "figure_caption", "footnote_label", "list_body", "list_label",
                    "table_body", "table_head",
                }
            ):
                raise PdfValidationError("generated structure owner differs")
        source_span = node["source_span"]
        if source_span is not None:
            _exact_keys(source_span, {"end_byte", "source_id", "start_byte"}, f"structure node {index} source span")
            if (
                _json_integer(source_span["source_id"], f"structure node {index} source ID") < 0
                or _json_integer(source_span["start_byte"], f"structure node {index} span start") < 0
                or _json_integer(source_span["end_byte"], f"structure node {index} span end")
                < source_span["start_byte"]
            ):
                raise PdfValidationError("structure source span differs")
        parent = node["parent"]
        if index == 0:
            if parent is not None or node["role"] != "Document":
                raise PdfValidationError("structure root is not /Document")
        elif _json_integer(parent, f"structure node {index} parent") >= index:
            raise PdfValidationError("structure parent is not prior preorder")
        children = node["children"]
        if not isinstance(children, list) or any(
            not isinstance(child, int)
            or isinstance(child, bool)
            or child <= index
            or child >= len(structure)
            for child in children
        ) or len(children) != len(set(children)):
            raise PdfValidationError("structure child order differs")
        for child in children:
            if structure[child]["parent"] != index:
                raise PdfValidationError("structure parent/child closure differs")
        alternative = node["alternative"]
        if node["role"] in {"Figure", "Formula"}:
            if not isinstance(alternative, str) or not alternative.strip():
                raise PdfValidationError("Figure/Formula alternative is missing")
        elif alternative is not None:
            raise PdfValidationError("unexpected structure alternative")
        if node["role"] == "Formula":
            if node["actual_text"] != alternative:
                raise PdfValidationError("Formula structure ActualText differs")
        elif node["actual_text"] is not None:
            raise PdfValidationError("unexpected structure ActualText")
        if node["role"] == "Link" and (
            not isinstance(node["accessible_name"], str)
            or not node["accessible_name"].strip()
        ):
            raise PdfValidationError("Link accessible name is missing")
        if node["role"] != "Link" and node["accessible_name"] is not None:
            raise PdfValidationError("unexpected structure accessible name")
        if node["role"] in {"Note", "TH"}:
            expected_id = f"typaxis-se-{index:08x}"
            if node["structure_id"] != expected_id:
                raise PdfValidationError("Note/TH structure ID differs")
        elif node["structure_id"] is not None:
            raise PdfValidationError("unexpected structure ID")
        table = node["table"]
        if node["role"] in {"TH", "TD"}:
            if not isinstance(table, dict):
                raise PdfValidationError("table cell attributes are missing")
            _exact_keys(
                table,
                {"colspan", "column_ordinal", "header_ids", "row_ordinal", "rowspan", "section"},
                f"structure node {index} table",
            )
            if (
                any(
                    not isinstance(table[key], int) or isinstance(table[key], bool)
                    for key in ("colspan", "column_ordinal", "row_ordinal", "rowspan")
                )
                or table["colspan"] < 1
                or table["rowspan"] < 1
                or table["column_ordinal"] < 0
                or table["row_ordinal"] < 0
                or not isinstance(table["header_ids"], list)
                or any(
                    not isinstance(header, str)
                    or re.fullmatch(r"typaxis-se-[0-9a-f]{8}", header) is None
                    for header in table["header_ids"]
                )
            ):
                raise PdfValidationError("table span is not positive")
            expected_section = "head" if node["role"] == "TH" else "body"
            if table["section"] != expected_section:
                raise PdfValidationError("table section/role differs")
            if node["role"] == "TD" and not table["header_ids"]:
                raise PdfValidationError("TD header association is empty")
        elif table is not None:
            raise PdfValidationError("non-cell has table attributes")
        related = node["related_nodes"]
        if (
            not isinstance(related, list)
            or any(
                not isinstance(target, int)
                or isinstance(target, bool)
                or not 0 <= target < len(structure)
                for target in related
            )
            or len(related) != len(set(related))
        ):
            raise PdfValidationError("structure relation registry differs")
        outline_ids = node["outline_ids"]
        if (
            not isinstance(outline_ids, list)
            or any(
                not isinstance(outline_id, int)
                or isinstance(outline_id, bool)
                or outline_id < 0
                for outline_id in outline_ids
            )
            or len(outline_ids) != len(set(outline_ids))
        ):
            raise PdfValidationError("structure outline relation differs")

    source_nodes = {
        node["owner"]["node_id"]: node
        for node in structure if node["owner"]["kind"] == "source"
    }
    if len(source_nodes) != sum(node["owner"]["kind"] == "source" for node in structure):
        raise PdfValidationError("source structure owner is duplicated")
    generated_keys: set[tuple[int, str, int]] = set()
    for index, node in enumerate(structure):
        owner = node["owner"]
        if owner["kind"] == "source":
            if index == 0:
                if owner["node_id"] != 0 or node["source_span"] is not None:
                    raise PdfValidationError("Document owner/source span differs")
            elif node["source_span"] is None:
                raise PdfValidationError("source structure span is missing")
        else:
            key = (owner["owner_node_id"], owner["slot"], owner["ordinal"])
            source = source_nodes.get(owner["owner_node_id"])
            if key in generated_keys or source is None or node["source_span"] != source["source_span"]:
                raise PdfValidationError("generated structure owner/span closure differs")
            generated_keys.add(key)
        if index and index not in structure[node["parent"]]["children"]:
            raise PdfValidationError("structure child/parent closure differs")
        related = node["related_nodes"]
        if node["role"] == "Note":
            if not related or any(
                structure[target]["role"] != "Reference"
                or structure[target]["related_nodes"] != [index]
                for target in related
            ):
                raise PdfValidationError("footnote Note/reference relation differs")
        elif related:
            if node["role"] != "Reference" or len(related) != 1:
                raise PdfValidationError("unexpected structure relation")
            note = related[0]
            if structure[note]["role"] != "Note" or index not in structure[note]["related_nodes"]:
                raise PdfValidationError("footnote reference/Note relation differs")

    block_roles = {
        "Exercise", "Figure", "Formula", "H1", "H2", "H3", "H4", "H5", "H6",
        "L", "P", "Proof", "Result", "Table",
    }
    inline_roles = {"Em", "Formula", "Link", "Reference", "Span", "Strong"}
    generated_expectations = {
        "figure_caption": ("Figure", "Caption"),
        "list_body": ("LI", "LBody"),
        "list_label": ("LI", "Lbl"),
        "table_body": ("Table", "TBody"),
        "table_head": ("Table", "THead"),
    }
    for index, node in enumerate(structure):
        role = node["role"]
        child_roles = [structure[child]["role"] for child in node["children"]]
        owner = node["owner"]
        if owner["kind"] == "generated":
            parent = structure[node["parent"]]
            slot = owner["slot"]
            if slot == "footnote_label":
                expected_parent_roles = {"Note", "Reference"}
                expected_role = "Lbl"
            else:
                expected_parent, expected_role = generated_expectations[slot]
                expected_parent_roles = {expected_parent}
            if (
                parent["role"] not in expected_parent_roles
                or role != expected_role
                or parent["owner"]["kind"] != "source"
                or parent["owner"]["node_id"] != owner["owner_node_id"]
            ):
                raise PdfValidationError("generated wrapper role/owner differs")
        if role == "Document":
            valid_children = all(child in block_roles for child in child_roles)
        elif role in {"Exercise", "Proof", "Result", "Caption", "LBody", "TH", "TD"}:
            valid_children = all(child in block_roles for child in child_roles)
        elif role in {"P", "H1", "H2", "H3", "H4", "H5", "H6"}:
            valid_children = all(child in inline_roles | {"Note"} for child in child_roles)
            if role != "P" and "Note" in child_roles:
                valid_children = False
        elif role in {"Em", "Link", "Strong"}:
            valid_children = all(child in inline_roles for child in child_roles)
        elif role == "L":
            valid_children = bool(child_roles) and all(child == "LI" for child in child_roles)
        elif role == "LI":
            valid_children = child_roles == ["Lbl", "LBody"]
        elif role == "Table":
            valid_children = child_roles == ["THead", "TBody"]
        elif role in {"THead", "TBody"}:
            valid_children = bool(child_roles) and all(child == "TR" for child in child_roles)
        elif role == "TR":
            section_role = structure[node["parent"]]["role"]
            expected_cell = "TH" if section_role == "THead" else "TD"
            valid_children = bool(child_roles) and all(child == expected_cell for child in child_roles)
        elif role == "Figure":
            valid_children = child_roles in ([], ["Caption"])
        elif role == "Note":
            valid_children = bool(child_roles) and child_roles[0] == "Lbl" and all(
                child in block_roles for child in child_roles[1:]
            )
        elif role == "Reference" and node["related_nodes"]:
            valid_children = child_roles == ["Lbl"]
        else:
            valid_children = not child_roles
        if not valid_children:
            raise PdfValidationError(f"structure role/child mapping differs at node {index}")
        marker = node["marker"]
        expected_paint = role in {"Figure", "Formula", "Lbl", "Span"} or (
            role == "Reference" and not node["related_nodes"]
        )
        if node["paint_required"] is not expected_paint:
            raise PdfValidationError("structure paint requirement differs")
        if role == "Lbl":
            if not isinstance(marker, str) or not marker:
                raise PdfValidationError("generated Label marker is missing")
        elif role == "Reference" and node["paint_required"]:
            if not isinstance(marker, str) or not marker:
                raise PdfValidationError("generated Reference label is missing")
        elif marker is not None:
            raise PdfValidationError("unexpected structure marker")
        if role == "Note":
            note_marker = structure[node["children"][0]]["marker"]
            if (
                not isinstance(note_marker, str)
                or re.fullmatch(r"[1-9][0-9]*", note_marker) is None
            ):
                raise PdfValidationError("footnote marker is not canonical decimal")
            for reference_id in node["related_nodes"]:
                reference = structure[reference_id]
                if structure[reference["children"][0]]["marker"] != note_marker:
                    raise PdfValidationError("footnote marker relation differs")
        if role == "L":
            markers = [
                structure[structure[item_id]["children"][0]]["marker"]
                for item_id in node["children"]
            ]
            if node["list_numbering"] == "disc":
                if any(marker != "•" for marker in markers):
                    raise PdfValidationError("unordered List marker differs")
            else:
                values = []
                for marker in markers:
                    if not isinstance(marker, str) or re.fullmatch(r"[1-9][0-9]*\.", marker) is None:
                        raise PdfValidationError("ordered List marker differs")
                    values.append(int(marker[:-1]))
                if values != list(range(values[0], values[0] + len(values))):
                    raise PdfValidationError("ordered List markers are not consecutive")
        if role == "Table":
            head_wrapper = structure[node["children"][0]]
            body_wrapper = structure[node["children"][1]]
            headers: list[dict[str, Any]] = []
            for row_ordinal, row_id in enumerate(head_wrapper["children"]):
                for cell_id in structure[row_id]["children"]:
                    cell = structure[cell_id]
                    if (
                        cell["table"]["row_ordinal"] != row_ordinal
                        or cell["table"]["section"] != "head"
                        or cell["table"]["header_ids"]
                    ):
                        raise PdfValidationError("Table head cell attributes differ")
                    headers.append(cell)
            for row_ordinal, row_id in enumerate(body_wrapper["children"]):
                for cell_id in structure[row_id]["children"]:
                    cell = structure[cell_id]
                    table = cell["table"]
                    if table["row_ordinal"] != row_ordinal or table["section"] != "body":
                        raise PdfValidationError("Table body cell attributes differ")
                    start = table["column_ordinal"]
                    end = start + table["colspan"]
                    expected_headers = [
                        header["structure_id"]
                        for header in headers
                        if header["table"]["column_ordinal"] < end
                        and start
                        < header["table"]["column_ordinal"] + header["table"]["colspan"]
                    ]
                    if table["header_ids"] != expected_headers:
                        raise PdfValidationError("Table header association differs")

    outline = manifest["outline"]
    if not isinstance(outline, list):
        raise PdfValidationError("tagged outline expectation differs")
    outline_stack: list[int] = []
    for index, entry in enumerate(outline):
        _exact_keys(
            entry,
            {
                "destination", "label", "level", "outline_id", "parent_outline_id",
                "source_node_id", "structure_node_id",
            },
            f"tagged outline {index}",
        )
        level = entry["level"]
        node_id = entry["structure_node_id"]
        if (
            not isinstance(level, int)
            or isinstance(level, bool)
            or not 1 <= level <= 6
            or level > len(outline_stack) + 1
        ):
            raise PdfValidationError("tagged outline level differs")
        expected_parent = None if level == 1 else outline_stack[level - 2]
        if len(outline_stack) >= level:
            del outline_stack[level - 1:]
        outline_stack.append(index)
        node = (
            structure[node_id]
            if isinstance(node_id, int)
            and not isinstance(node_id, bool)
            and 0 <= node_id < len(structure)
            else None
        )
        if (
            _json_integer(entry["outline_id"], f"tagged outline {index} ID") != index
            or _json_integer(entry["source_node_id"], f"tagged outline {index} source") < 0
            or (
                entry["parent_outline_id"] is not None
                and _json_integer(
                    entry["parent_outline_id"], f"tagged outline {index} parent"
                ) < 0
            )
            or entry["parent_outline_id"] != expected_parent
            or node is None
            or node["owner"]["kind"] != "source"
            or node["owner"]["node_id"] != entry["source_node_id"]
            or node["role"] not in {
                "Exercise", "H1", "H2", "H3", "H4", "H5", "H6", "Proof", "Result"
            }
            or node["language"] != language
            or index not in node["outline_ids"]
            or not isinstance(entry["destination"], str)
            or not isinstance(entry["label"], str)
            or not entry["label"].strip()
        ):
            raise PdfValidationError("tagged outline source/structure closure differs")
    observed_outline_ids = [
        outline_id for node in structure for outline_id in node["outline_ids"]
    ]
    expected_outline_ids = list(range(len(outline)))
    if sorted(observed_outline_ids) != expected_outline_ids:
        raise PdfValidationError("tagged outline ID closure differs")

    marked = manifest["marked_content"]
    _exact_keys(
        marked,
        {"annotations", "pages", "parent_tree", "records", "selected_layout_fragment_count"},
        "marked content",
    )
    if not isinstance(marked["annotations"], list):
        raise PdfValidationError("marked-content annotations are not an array")
    if not isinstance(marked["parent_tree"], list):
        raise PdfValidationError("marked-content ParentTree is not an array")
    pages = marked["pages"]
    if not isinstance(pages, list) or not pages:
        raise PdfValidationError("marked-content pages are empty")
    page_records: dict[int, list[dict[str, Any]]] = {index: [] for index in range(len(pages))}
    page_ordinals = [0] * len(pages)
    page_mcids = [0] * len(pages)
    required: set[int] = set()
    artifact_occurrences: dict[str, set[int]] = {}
    semantic_fragments: dict[int, set[int]] = {}
    next_selected_paint_id = 0
    previous_group: tuple[Any, ...] | None = None
    if not isinstance(marked["records"], list):
        raise PdfValidationError("marked-content records are not an array")
    for index, record in enumerate(marked["records"]):
        _exact_keys(
            record,
            {
                "actual_text", "language", "owner", "page_index", "paint_ordinal_start",
                "selected_paint_ids", "semantic_fragment_ordinal",
            },
            f"marked-content record {index}",
        )
        selected_paint_ids = record["selected_paint_ids"]
        if (
            not isinstance(selected_paint_ids, list)
            or not selected_paint_ids
            or any(not isinstance(value, int) or isinstance(value, bool) for value in selected_paint_ids)
            or selected_paint_ids
            != list(range(next_selected_paint_id, next_selected_paint_id + len(selected_paint_ids)))
        ):
            raise PdfValidationError("selected paint IDs are not dense within nonempty groups")
        next_selected_paint_id += len(selected_paint_ids)
        page = record["page_index"]
        if not isinstance(page, int) or isinstance(page, bool) or not 0 <= page < len(pages):
            raise PdfValidationError("marked-content page is out of range")
        if (
            _json_integer(record["paint_ordinal_start"], "marked paint ordinal")
            != page_ordinals[page]
        ):
            raise PdfValidationError("page paint ordinals are not dense")
        page_ordinals[page] += len(selected_paint_ids)
        owner = record["owner"]
        kind = owner.get("kind") if isinstance(owner, dict) else None
        if kind == "structure":
            _exact_keys(owner, {"kind", "mcid", "role", "structure_node_id"}, "marked structure owner")
            node_id = owner["structure_node_id"]
            if (
                not isinstance(node_id, int)
                or isinstance(node_id, bool)
                or not 0 <= node_id < len(structure)
                or not structure[node_id]["paint_required"]
            ):
                raise PdfValidationError("marked content has a wrong structure owner")
            if (
                owner["role"] != structure[node_id]["role"]
                or _json_integer(owner["mcid"], "marked-content MCID") != page_mcids[page]
            ):
                raise PdfValidationError("marked-content role/MCID differs")
            fragment = record["semantic_fragment_ordinal"]
            if not isinstance(fragment, int) or isinstance(fragment, bool) or fragment < 0:
                raise PdfValidationError("semantic fragment ordinal is invalid")
            fragments = semantic_fragments.setdefault(node_id, set())
            if fragment in fragments:
                raise PdfValidationError("semantic fragment is split across marked groups")
            fragments.add(fragment)
            page_mcids[page] += 1
            required.add(node_id)
            expected_language = (
                structure[node_id]["language"]
                if structure[node_id]["language"] != language
                else None
            )
            if record["language"] != expected_language:
                raise PdfValidationError("marked-content computed language differs")
            if record["actual_text"] is not None:
                if owner["role"] != "Formula" or record["actual_text"] != structure[node_id]["alternative"]:
                    raise PdfValidationError("Formula ActualText differs from alternative")
            group_key = (
                "structure", node_id, fragment, record["actual_text"], record["language"]
            )
        elif kind == "artifact":
            _exact_keys(owner, {"class", "kind", "occurrence"}, "marked artifact owner")
            if (
                not isinstance(owner["class"], str)
                or owner["class"]
                not in {"layout", "pagination", "pagination_footer", "pagination_header"}
            ):
                raise PdfValidationError("artifact class is outside the closed registry")
            occurrence = owner["occurrence"]
            if not isinstance(occurrence, int) or isinstance(occurrence, bool) or occurrence < 0:
                raise PdfValidationError("artifact occurrence is invalid")
            occurrences = artifact_occurrences.setdefault(owner["class"], set())
            if occurrence in occurrences:
                raise PdfValidationError("artifact occurrence is duplicated")
            occurrences.add(occurrence)
            if record["semantic_fragment_ordinal"] != 0 or record["actual_text"] is not None or record["language"] is not None:
                raise PdfValidationError("artifact carries semantic text/language")
            group_key = ("artifact", owner["class"], occurrence)
        else:
            raise PdfValidationError("marked-content owner kind differs")
        if previous_group is not None and previous_group[0] > page:
            raise PdfValidationError("marked-content records are not in page/paint order")
        if previous_group == (page, *group_key):
            raise PdfValidationError("adjacent identical marked-content groups are not maximal")
        previous_group = (page, *group_key)
        page_records[page].append(record)
    for fragments in semantic_fragments.values():
        if fragments != set(range(len(fragments))):
            raise PdfValidationError("semantic fragment ordinals are not dense")
    for occurrences in artifact_occurrences.values():
        if occurrences != set(range(len(occurrences))):
            raise PdfValidationError("artifact occurrences are not dense")
    expected_required = {
        node["structure_node_id"] for node in structure if node["paint_required"]
    }
    if required != expected_required:
        raise PdfValidationError("visual/structure paint closure differs")
    next_page_parent_key = 0
    expected_parent_tree: list[dict[str, Any]] = []
    for index, page in enumerate(pages):
        _exact_keys(
            page,
            {
                "artifact_count", "height_raw", "marked_content_count", "page_index",
                "structure_parent_key", "width_raw",
            },
            f"marked page {index}",
        )
        expected_parent_key = None
        if page_mcids[index]:
            expected_parent_key = next_page_parent_key
            next_page_parent_key += 1
        if (
            _json_integer(page["page_index"], f"marked page {index} ID") != index
            or page["structure_parent_key"] != expected_parent_key
            or not isinstance(page["width_raw"], int)
            or isinstance(page["width_raw"], bool)
            or not isinstance(page["height_raw"], int)
            or isinstance(page["height_raw"], bool)
            or page["width_raw"] <= 0
            or page["height_raw"] <= 0
            or _json_integer(page["artifact_count"], "page artifact count") < 0
            or _json_integer(page["marked_content_count"], "page MCID count") < 0
        ):
            raise PdfValidationError("page StructParents keys are not dense")
        if page["marked_content_count"] != page_mcids[index]:
            raise PdfValidationError("page marked-content count differs")
        if expected_parent_key is not None:
            expected_parent_tree.append(
                {
                    "key": expected_parent_key,
                    "kind": "page",
                    "structure_node_ids": [
                        record["owner"]["structure_node_id"]
                        for record in page_records[index]
                        if record["owner"]["kind"] == "structure"
                    ],
                }
            )
        artifacts = sum(record["owner"]["kind"] == "artifact" for record in page_records[index])
        if page["artifact_count"] != artifacts:
            raise PdfValidationError("page artifact count differs")

    destinations = manifest["destinations"]
    if not isinstance(destinations, list):
        raise PdfValidationError("tagged destination registry is not an array")
    destination_names: list[str] = []
    for index, destination in enumerate(destinations):
        _exact_keys(
            destination,
            {"anchor_id", "frame_id", "page_index", "source_node_id", "view"},
            f"tagged destination {index}",
        )
        name = destination["anchor_id"]
        page_index = destination["page_index"]
        if (
            not isinstance(name, str)
            or re.fullmatch(r"[A-Za-z_][A-Za-z0-9_.:-]*", name) is None
            or not isinstance(page_index, int)
            or isinstance(page_index, bool)
            or not 0 <= page_index < len(pages)
            or not isinstance(destination["frame_id"], int)
            or isinstance(destination["frame_id"], bool)
            or destination["frame_id"] < 0
            or not isinstance(destination["source_node_id"], int)
            or isinstance(destination["source_node_id"], bool)
            or destination["source_node_id"] < 0
        ):
            raise PdfValidationError("tagged destination owner/page differs")
        view = destination["view"]
        view_kind = view.get("kind") if isinstance(view, dict) else None
        if (
            not isinstance(view_kind, str)
            or view_kind not in {"fit_page", "fit_width", "xyz"}
        ):
            raise PdfValidationError("tagged destination view differs")
        if view_kind == "fit_page":
            _exact_keys(view, {"kind"}, f"tagged destination {index} view")
        elif view_kind == "fit_width":
            _exact_keys(view, {"kind", "top"}, f"tagged destination {index} view")
            if view["top"] is not None and (
                not isinstance(view["top"], int) or isinstance(view["top"], bool)
            ):
                raise PdfValidationError("tagged FitWidth top differs")
        else:
            _exact_keys(view, {"kind", "x", "y"}, f"tagged destination {index} view")
            if (
                not isinstance(view["x"], int)
                or isinstance(view["x"], bool)
                or not isinstance(view["y"], int)
                or isinstance(view["y"], bool)
            ):
                raise PdfValidationError("tagged XYZ point differs")
        destination_names.append(name)
    if destination_names != sorted(set(destination_names), key=lambda value: value.encode("utf-8")):
        raise PdfValidationError("tagged destination names are not unique/sorted")
    annotations = marked["annotations"]
    linked_nodes: set[int] = set()
    previous_annotation_page = -1
    for index, annotation in enumerate(annotations):
        _exact_keys(
            annotation,
            {
                "accessible_name", "annotation_id", "destination", "page_index", "rect",
                "structure_node_id", "structure_parent_key",
            },
            f"marked annotation {index}",
        )
        page = annotation["page_index"]
        node_id = annotation["structure_node_id"]
        if (
            _json_integer(annotation["annotation_id"], f"marked annotation {index} ID")
            != index
            or not isinstance(page, int)
            or isinstance(page, bool)
            or not 0 <= page < len(pages)
            or page < previous_annotation_page
            or not isinstance(node_id, int)
            or isinstance(node_id, bool)
            or not 0 <= node_id < len(structure)
            or structure[node_id]["role"] != "Link"
            or annotation["accessible_name"] != structure[node_id]["accessible_name"]
            or _json_integer(
                annotation["structure_parent_key"],
                f"marked annotation {index} structure parent key",
            )
            != next_page_parent_key + index
            or not isinstance(annotation["destination"], str)
            or not isinstance(annotation["rect"], list)
            or len(annotation["rect"]) != 4
            or any(
                not isinstance(coordinate, int) or isinstance(coordinate, bool)
                for coordinate in annotation["rect"]
            )
        ):
            raise PdfValidationError("marked Link annotation closure differs")
        left, bottom, right, top = annotation["rect"]
        if not (
            0 <= left < right <= pages[page]["width_raw"]
            and 0 <= bottom < top <= pages[page]["height_raw"]
        ):
            raise PdfValidationError("marked Link annotation rectangle differs")
        previous_annotation_page = page
        linked_nodes.add(node_id)
        expected_parent_tree.append(
            {
                "key": next_page_parent_key + index,
                "kind": "annotation",
                "structure_node_id": node_id,
            }
        )
    used_destinations = {entry["destination"] for entry in outline} | {
        annotation["destination"] for annotation in annotations
    }
    if not used_destinations.issubset(set(destination_names)):
        raise PdfValidationError("outline/Link destination is missing")
    expected_links = {
        node["structure_node_id"] for node in structure if node["role"] == "Link"
    }
    if linked_nodes != expected_links:
        raise PdfValidationError("Link annotation coverage differs")
    for index, entry in enumerate(marked["parent_tree"]):
        entry_kind = entry.get("kind") if isinstance(entry, dict) else None
        if not isinstance(entry_kind, str) or entry_kind not in {"annotation", "page"}:
            raise PdfValidationError("marked-content ParentTree entry differs")
        if entry_kind == "page":
            _exact_keys(
                entry,
                {"key", "kind", "structure_node_ids"},
                f"marked-content ParentTree entry {index}",
            )
            values = entry["structure_node_ids"]
            if not isinstance(values, list) or any(
                not isinstance(value, int)
                or isinstance(value, bool)
                or not 0 <= value < len(structure)
                for value in values
            ):
                raise PdfValidationError("marked-content ParentTree page value differs")
        else:
            _exact_keys(
                entry,
                {"key", "kind", "structure_node_id"},
                f"marked-content ParentTree entry {index}",
            )
            _json_integer(
                entry["structure_node_id"],
                f"marked-content ParentTree entry {index} structure node",
            )
        if _json_integer(entry["key"], f"marked-content ParentTree entry {index} key") != index:
            raise PdfValidationError("marked-content ParentTree keys are not dense")
    if marked["parent_tree"] != expected_parent_tree:
        raise PdfValidationError("marked-content ParentTree expectation differs")
    if (
        not isinstance(marked["selected_layout_fragment_count"], int)
        or isinstance(marked["selected_layout_fragment_count"], bool)
        or marked["selected_layout_fragment_count"] < len(marked["records"])
    ):
        raise PdfValidationError("selected layout fragment count differs")

    pdf_fact = manifest["pdf"]
    _exact_keys(
        pdf_fact,
        {
            "artifact_count", "byte_length", "id_tree_object", "link_annotation_count",
            "marked_content_count", "objects", "parent_tree_object",
            "structure_element_count", "structure_tree_root_object",
        },
        "tagged PDF observation",
    )
    for key in (
        "artifact_count", "byte_length", "link_annotation_count", "marked_content_count",
        "parent_tree_object", "structure_element_count", "structure_tree_root_object",
    ):
        minimum = 1 if key in {
            "byte_length", "parent_tree_object", "structure_element_count",
            "structure_tree_root_object",
        } else 0
        _json_integer(pdf_fact[key], f"tagged PDF {key}", minimum)
    if pdf_fact["id_tree_object"] is not None:
        _json_integer(pdf_fact["id_tree_object"], "tagged PDF IDTree object", 1)
    if not isinstance(pdf_fact["objects"], list) or not pdf_fact["objects"]:
        raise PdfValidationError("tagged PDF object observations are empty")
    if (
        pdf_fact["structure_element_count"] != len(structure)
        or pdf_fact["marked_content_count"] != sum(page_mcids)
        or pdf_fact["artifact_count"]
        != sum(page["artifact_count"] for page in pages)
        or pdf_fact["link_annotation_count"] != len(annotations)
    ):
        raise PdfValidationError("tagged PDF observation counts differ")


def _role_objects(manifest: dict[str, Any]) -> dict[str, int]:
    output: dict[str, int] = {}
    objects = manifest["pdf"]["objects"]
    for index, item in enumerate(objects, 1):
        _exact_keys(item, {"object_number", "role", "sha256"}, f"PDF object role {index}")
        if (
            _json_integer(item["object_number"], f"PDF object role {index} number", 1)
            != index
            or not isinstance(item["role"], str)
            or not item["role"]
            or item["role"] in output
        ):
            raise PdfValidationError("PDF object roles are not dense and unique")
        if (
            not isinstance(item["sha256"], str)
            or re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is None
        ):
            raise PdfValidationError("PDF object hash is not lowercase SHA-256")
        output[item["role"]] = index
    return output


def _as_items(value: Any) -> list[Any]:
    return value if isinstance(value, list) else [value]


def _artifact_class(properties: dict[str, Any]) -> str:
    keys = set(properties)
    if keys == {"Type"} and properties["Type"] == PdfName("Layout"):
        return "layout"
    if keys == {"Type"} and properties["Type"] == PdfName("Pagination"):
        return "pagination"
    if keys == {"Subtype", "Type"} and properties["Type"] == PdfName("Pagination"):
        if properties["Subtype"] == PdfName("Header"):
            return "pagination_header"
        if properties["Subtype"] == PdfName("Footer"):
            return "pagination_footer"
    raise PdfValidationError("artifact property list differs")


def _xml_text(value: str) -> str:
    return value.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def _xmp_alt(property_name: str, value: str, language: str) -> str:
    output = (
        f"<{property_name}><rdf:Alt><rdf:li xml:lang=\"x-default\">"
        f"{_xml_text(value)}</rdf:li>"
    )
    if language != "x-default":
        output += f'<rdf:li xml:lang="{language}">{_xml_text(value)}</rdf:li>'
    return output + f"</rdf:Alt></{property_name}>"


def _expected_tagged_xmp(manifest: dict[str, Any]) -> bytes:
    metadata = manifest["metadata"]
    language = manifest["document_language"]
    properties = _xmp_alt("dc:title", metadata["title"], language)
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
        joined = _xml_text("; ".join(metadata["keywords"]))
        properties += f"</rdf:Bag></dc:subject><pdf:Keywords>{joined}</pdf:Keywords>"
    if metadata["identifier"] is not None:
        properties += f"<dc:identifier>{_xml_text(metadata['identifier'])}</dc:identifier>"
    if metadata["created"] is not None:
        properties += f"<xmp:CreateDate>{metadata['created']}</xmp:CreateDate>"
    if metadata["modified"] is not None:
        properties += f"<xmp:ModifyDate>{metadata['modified']}</xmp:ModifyDate>"
    properties += (
        f"<dc:language><rdf:Bag><rdf:li>{_xml_text(language)}</rdf:li></rdf:Bag></dc:language>"
        f"<pdf:Producer>{_xml_text(manifest['engine']['name'] + ' ' + manifest['engine']['version'])}</pdf:Producer>"
        "<pdfuaid:part>1</pdfuaid:part>"
    )
    return (
        '<x:xmpmeta xmlns:x="adobe:ns:meta/"><rdf:RDF '
        'xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"><rdf:Description '
        'rdf:about="" xmlns:dc="http://purl.org/dc/elements/1.1/" '
        'xmlns:pdf="http://ns.adobe.com/pdf/1.3/" '
        'xmlns:xmp="http://ns.adobe.com/xap/1.0/" '
        f'xmlns:pdfuaid="http://www.aiim.org/pdfua/ns/id/">{properties}'
        '</rdf:Description></rdf:RDF></x:xmpmeta>'
    ).encode("utf-8")


def _verify_tagged_content(
    content: bytes,
    expected: list[dict[str, Any]],
) -> list[dict[str, Any]]:
    cursor = 0
    observations: list[dict[str, Any]] = []
    for record in expected:
        paint_commands = b"0 0 m 0 0 l S\n" * len(record["selected_paint_ids"])
        simple_suffix = b"BDC\n" + paint_commands + b"EMC\n"
        parser = PdfParser(content[cursor:])
        tag = parser.parse()
        properties = parser.parse()
        parser.skip_space()
        if not isinstance(tag, PdfName) or not isinstance(properties, dict):
            raise PdfValidationError("marked-content operator prefix differs")
        cursor += parser.pos
        owner = record["owner"]
        if owner["kind"] == "artifact":
            if tag != PdfName("Artifact") or _artifact_class(properties) != owner["class"]:
                raise PdfValidationError("artifact class/tag differs")
            if not content.startswith(simple_suffix, cursor):
                raise PdfValidationError("artifact paint commands differ")
            cursor += len(simple_suffix)
            observations.append({"class": owner["class"], "kind": "artifact"})
            continue
        actual_text = record["actual_text"]
        language = record["language"]
        is_span = owner["role"] == "Span"
        required = {"MCID"}
        if is_span and actual_text is not None:
            required.add("ActualText")
        if is_span and language is not None:
            required.add("Lang")
        _exact_keys(properties, required, "structure marked-content property list")
        if tag.value != owner["role"] or _integer(properties["MCID"], "marked MCID") != owner["mcid"]:
            raise PdfValidationError("marked-content tag/MCID differs")
        outer_language = (
            _utf16_text(properties["Lang"], "marked-content /Lang")
            if "Lang" in properties else None
        )
        outer_actual = (
            _utf16_text(properties["ActualText"], "marked-content /ActualText")
            if "ActualText" in properties else None
        )
        nested = not is_span and (actual_text is not None or language is not None)
        if not nested:
            if not content.startswith(simple_suffix, cursor):
                raise PdfValidationError("structure paint commands differ")
            cursor += len(simple_suffix)
            if outer_language != language or outer_actual != actual_text:
                raise PdfValidationError("marked-content property values differ")
        else:
            prefix = b"BDC\n/Span "
            if not content.startswith(prefix, cursor):
                raise PdfValidationError("nested Span marked content differs")
            nested = PdfParser(content[cursor + len(prefix):])
            nested_properties = nested.parse()
            nested.skip_space()
            nested_keys = ({"ActualText"} if actual_text is not None else set()) | (
                {"Lang"} if language is not None else set()
            )
            _exact_keys(nested_properties, nested_keys, "nested Span property list")
            observed_actual = (
                _utf16_text(nested_properties["ActualText"], "nested Span /ActualText")
                if "ActualText" in nested_properties else None
            )
            observed_language = (
                _utf16_text(nested_properties["Lang"], "nested Span /Lang")
                if "Lang" in nested_properties else None
            )
            suffix = b"BDC\n" + paint_commands + b"EMC\nEMC\n"
            cursor += len(prefix) + nested.pos
            if (
                not content.startswith(suffix, cursor)
                or observed_actual != actual_text
                or observed_language != language
            ):
                raise PdfValidationError("nested Span properties/paint differ")
            cursor += len(suffix)
        observations.append(
            {
                "actual_text": actual_text,
                "kind": "structure",
                "language": language,
                "mcid": owner["mcid"],
                "role": owner["role"],
                "structure_node_id": owner["structure_node_id"],
            }
        )
    if cursor != len(content):
        raise PdfValidationError("extra or missing page marked content")
    return observations


def verify_tagged_pdf_structure(pdf: bytes, manifest: dict[str, Any]) -> dict[str, Any]:
    """Independently decode and close the MI4-09 tagged-PDF graph."""

    _tagged_manifest_shape(manifest)
    objects, trailer = _parse_xref(pdf)
    roles = _role_objects(manifest)
    expected_role_order = ["catalog", "pages", "destination_name_tree"]
    for index in range(len(manifest["marked_content"]["pages"])):
        expected_role_order.extend((f"page_content:{index}", f"page:{index}"))
    expected_role_order.extend(
        f"link_annotation:{index}"
        for index in range(len(manifest["marked_content"]["annotations"]))
    )
    expected_role_order.extend(("info", "metadata"))
    if manifest["outline"]:
        expected_role_order.append("outline_root")
        expected_role_order.extend(
            f"outline_item:{index}" for index in range(len(manifest["outline"]))
        )
    expected_role_order.extend(("structure_tree_root", "parent_tree"))
    if manifest["pdf"]["id_tree_object"] is not None:
        expected_role_order.append("structure_id_tree")
    expected_role_order.extend(
        f"structure_element:{index}" for index in range(len(manifest["structure"]))
    )
    observed_role_order = [item["role"] for item in manifest["pdf"]["objects"]]
    if observed_role_order != expected_role_order:
        raise PdfValidationError("tagged PDF object role plan differs")
    if (
        roles["structure_tree_root"] != manifest["pdf"]["structure_tree_root_object"]
        or roles["parent_tree"] != manifest["pdf"]["parent_tree_object"]
        or roles.get("structure_id_tree") != manifest["pdf"]["id_tree_object"]
    ):
        raise PdfValidationError("tagged PDF structure object observation differs")
    page_roles = {
        index: roles[f"page:{index}"]
        for index in range(len(manifest["marked_content"]["pages"]))
    }
    expected_objects = manifest["pdf"]["objects"]
    if len(objects) != len(expected_objects) or sorted(objects) != list(range(1, len(objects) + 1)):
        raise PdfValidationError("tagged PDF object allocation differs")
    if _ref(trailer["Info"], "trailer /Info") != roles["info"]:
        raise PdfValidationError("tagged PDF trailer /Info differs")
    if manifest["pdf"]["byte_length"] != len(pdf):
        raise PdfValidationError("tagged PDF byte length differs")
    pdf_hash = hashlib.sha256(pdf).hexdigest()
    if manifest["fingerprints"]["pdf_sha256"] != pdf_hash:
        raise PdfValidationError("tagged PDF hash differs")
    for item in expected_objects:
        if hashlib.sha256(objects[item["object_number"]].raw).hexdigest() != item["sha256"]:
            raise PdfValidationError(f"tagged PDF object hash differs for {item['role']}")

    catalog = objects[roles["catalog"]].value
    catalog_keys = {
        "Lang", "MarkInfo", "Metadata", "Names", "Pages", "StructTreeRoot",
        "Type", "ViewerPreferences",
    } | ({"Outlines"} if manifest["outline"] else set())
    _exact_keys(catalog, catalog_keys, "tagged catalog")
    _name(catalog["Type"], "Catalog", "tagged catalog /Type")
    if (
        _ref(catalog["Pages"], "tagged catalog /Pages") != roles["pages"]
        or _ref(catalog["Metadata"], "tagged catalog /Metadata") != roles["metadata"]
        or catalog["Names"] != {"Dests": PdfRef(roles["destination_name_tree"])}
    ):
        raise PdfValidationError("tagged catalog graph differs")
    language = _utf16_text(catalog["Lang"], "tagged catalog /Lang")
    if language != manifest["document_language"]:
        raise PdfValidationError("tagged catalog language differs")
    if catalog["MarkInfo"] != {"Marked": True}:
        raise PdfValidationError("catalog /MarkInfo differs")
    if catalog["ViewerPreferences"] != {"DisplayDocTitle": True}:
        raise PdfValidationError("catalog /ViewerPreferences differs")
    if manifest["outline"]:
        if _ref(catalog["Outlines"], "tagged catalog /Outlines") != roles["outline_root"]:
            raise PdfValidationError("tagged catalog outline root differs")

    metadata_expectation = manifest["metadata"]
    info = objects[roles["info"]].value
    info_keys = {"Producer"}
    for manifest_key, pdf_key in (
        ("author", "Author"),
        ("created", "CreationDate"),
        ("modified", "ModDate"),
        ("subject", "Subject"),
        ("title", "Title"),
    ):
        if metadata_expectation[manifest_key] is not None:
            info_keys.add(pdf_key)
    if metadata_expectation["keywords"]:
        info_keys.add("Keywords")
    _exact_keys(info, info_keys, "tagged PDF Info")
    for manifest_key, pdf_key in (
        ("author", "Author"),
        ("subject", "Subject"),
        ("title", "Title"),
    ):
        if metadata_expectation[manifest_key] is not None and _utf16_text(
            info[pdf_key], f"Info /{pdf_key}"
        ) != metadata_expectation[manifest_key]:
            raise PdfValidationError("tagged PDF Info text differs")
    if metadata_expectation["keywords"] and _utf16_text(
        info["Keywords"], "Info /Keywords"
    ) != "; ".join(metadata_expectation["keywords"]):
        raise PdfValidationError("tagged PDF Info keywords differ")
    for manifest_key, pdf_key in (("created", "CreationDate"), ("modified", "ModDate")):
        timestamp = metadata_expectation[manifest_key]
        if timestamp is not None:
            expected_date = (
                "D:" + timestamp[0:4] + timestamp[5:7] + timestamp[8:10]
                + timestamp[11:13] + timestamp[14:16] + timestamp[17:19] + "Z"
            )
            if _literal_ascii_text(info[pdf_key], f"Info /{pdf_key}") != expected_date:
                raise PdfValidationError("tagged PDF Info date differs")
    expected_producer = manifest["engine"]["name"] + " " + manifest["engine"]["version"]
    if _utf16_text(info["Producer"], "Info /Producer") != expected_producer:
        raise PdfValidationError("tagged PDF Info producer differs")

    pages_tree = objects[roles["pages"]].value
    _exact_keys(pages_tree, {"Count", "Kids", "Type"}, "tagged pages tree")
    _name(pages_tree["Type"], "Pages", "tagged pages tree /Type")
    expected_page_refs = [PdfRef(page_roles[index]) for index in range(len(page_roles))]
    if pages_tree["Count"] != len(page_roles) or pages_tree["Kids"] != expected_page_refs:
        raise PdfValidationError("tagged pages tree differs")

    destination_tree = objects[roles["destination_name_tree"]].value
    _exact_keys(destination_tree, {"Names"}, "tagged destination name tree")
    names = destination_tree["Names"]
    if not isinstance(names, list) or len(names) != 2 * len(manifest["destinations"]):
        raise PdfValidationError("tagged destination name tree length differs")
    page_by_object = {value: key for key, value in page_roles.items()}
    observed_destinations = []
    for offset in range(0, len(names), 2):
        name = _literal_ascii_text(names[offset], "tagged destination name")
        page_index, pdf_view = _parse_destination_view(names[offset + 1], page_by_object)
        observed_destinations.append((name, page_index, pdf_view))
    expected_destinations = []
    for destination in manifest["destinations"]:
        page_index = destination["page_index"]
        selected_view = destination["view"]
        page_height = manifest["marked_content"]["pages"][page_index]["height_raw"]
        if selected_view["kind"] == "xyz":
            pdf_view = {
                "kind": "xyz",
                "x": selected_view["x"],
                "y": page_height - selected_view["y"],
            }
        elif selected_view["kind"] == "fit_width":
            pdf_view = {
                "kind": "fit_width",
                "top": None if selected_view["top"] is None else page_height - selected_view["top"],
            }
        else:
            pdf_view = {"kind": "fit_page"}
        expected_destinations.append((destination["anchor_id"], page_index, pdf_view))
    if observed_destinations != expected_destinations:
        raise PdfValidationError("tagged destination name tree differs")

    structure_root_number = roles["structure_tree_root"]
    if _ref(catalog["StructTreeRoot"], "catalog /StructTreeRoot") != structure_root_number:
        raise PdfValidationError("catalog structure-root reference differs")
    structure_root = objects[structure_root_number].value
    root_keys = {"K", "ParentTree", "ParentTreeNextKey", "RoleMap", "Type"}
    id_tree_role = roles.get("structure_id_tree")
    if id_tree_role is not None:
        root_keys.add("IDTree")
    _exact_keys(structure_root, root_keys, "structure tree root")
    _name(structure_root["Type"], "StructTreeRoot", "structure root /Type")
    role_map = structure_root["RoleMap"]
    expected_role_map = {
        "Em": PdfName("Span"), "Exercise": PdfName("Div"),
        "Proof": PdfName("Div"), "Result": PdfName("Div"),
        "Strong": PdfName("Span"),
    }
    if role_map != expected_role_map:
        raise PdfValidationError("structure /RoleMap differs")

    structure = manifest["structure"]
    structure_objects = {
        index: roles[f"structure_element:{index}"] for index in range(len(structure))
    }
    object_to_structure = {value: key for key, value in structure_objects.items()}
    root_k = structure_root["K"]
    if not isinstance(root_k, list) or root_k != [PdfRef(structure_objects[0])]:
        raise PdfValidationError("structure root /K differs")
    observed_mcr: dict[int, list[tuple[int, int]]] = {index: [] for index in range(len(structure))}
    observed_objr: dict[int, list[int]] = {index: [] for index in range(len(structure))}
    expected_mcr_with_ordinals: dict[int, list[tuple[int, int, int]]] = {
        index: [] for index in range(len(structure))
    }
    for record in manifest["marked_content"]["records"]:
        if record["owner"]["kind"] == "structure":
            expected_mcr_with_ordinals[record["owner"]["structure_node_id"]].append(
                (
                    record["semantic_fragment_ordinal"],
                    record["page_index"],
                    record["owner"]["mcid"],
                )
            )
    expected_mcr = {
        owner: [(page, mcid) for _, page, mcid in sorted(values)]
        for owner, values in expected_mcr_with_ordinals.items()
    }
    expected_objr: dict[int, list[int]] = {index: [] for index in range(len(structure))}
    for annotation in manifest["marked_content"]["annotations"]:
        expected_objr[annotation["structure_node_id"]].append(
            roles[f"link_annotation:{annotation['annotation_id']}"]
        )
    for index, expected in enumerate(structure):
        dictionary = objects[structure_objects[index]].value
        required = {"P", "S", "Type"}
        if expected["children"] or expected["paint_required"] or expected["role"] == "Link":
            required.add("K")
        parent_language = manifest["document_language"] if expected["parent"] is None else structure[expected["parent"]]["language"]
        if expected["language"] != parent_language:
            required.add("Lang")
        if expected["alternative"] is not None:
            required.add("Alt")
        if expected["structure_id"] is not None:
            required.add("ID")
        if expected["table"] is not None or expected["list_numbering"] is not None:
            required.add("A")
        _exact_keys(dictionary, required, f"structure element {index}")
        _name(dictionary["Type"], "StructElem", f"structure element {index} /Type")
        _name(dictionary["S"], expected["role"], f"structure element {index} /S")
        expected_parent = structure_root_number if expected["parent"] is None else structure_objects[expected["parent"]]
        if _ref(dictionary["P"], f"structure element {index} /P") != expected_parent:
            raise PdfValidationError("structure parent object differs")
        if "Lang" in dictionary and _utf16_text(dictionary["Lang"], f"structure {index} /Lang") != expected["language"]:
            raise PdfValidationError("structure language projection differs")
        if "Alt" in dictionary and _utf16_text(dictionary["Alt"], f"structure {index} /Alt") != expected["alternative"]:
            raise PdfValidationError("structure alternative differs")
        if "ID" in dictionary and _literal_ascii_text(dictionary["ID"], f"structure {index} /ID") != expected["structure_id"]:
            raise PdfValidationError("structure ID differs")
        if "A" in dictionary:
            if expected["list_numbering"] is not None:
                _exact_keys(dictionary["A"], {"ListNumbering", "O"}, f"structure {index} List attributes")
                _name(dictionary["A"]["O"], "List", f"structure {index} List owner")
                _name(
                    dictionary["A"]["ListNumbering"],
                    "Decimal" if expected["list_numbering"] == "decimal" else "Disc",
                    f"structure {index} List numbering",
                )
            else:
                table = expected["table"]
                table_keys = {"O"}
                if expected["role"] == "TH":
                    table_keys.add("Scope")
                if table["rowspan"] > 1:
                    table_keys.add("RowSpan")
                if table["colspan"] > 1:
                    table_keys.add("ColSpan")
                if table["header_ids"]:
                    table_keys.add("Headers")
                _exact_keys(dictionary["A"], table_keys, f"structure {index} table attributes")
                _name(dictionary["A"]["O"], "Table", f"structure {index} table owner")
                if "Scope" in dictionary["A"]:
                    _name(dictionary["A"]["Scope"], "Column", f"structure {index} table scope")
                if dictionary["A"].get("RowSpan", 1) != table["rowspan"] or dictionary["A"].get("ColSpan", 1) != table["colspan"]:
                    raise PdfValidationError("table span attributes differ")
                headers = [
                    _literal_ascii_text(value, f"structure {index} /Headers")
                    for value in dictionary["A"].get("Headers", [])
                ]
                if headers != table["header_ids"]:
                    raise PdfValidationError("table header associations differ")
        child_ids: list[int] = []
        observed_k_order: list[tuple[Any, ...]] = []
        raw_k = dictionary.get("K", [])
        if "K" in dictionary and not isinstance(raw_k, list):
            raise PdfValidationError("structure /K is not an array")
        for kid in raw_k:
            if isinstance(kid, PdfRef) and kid.number in object_to_structure:
                child_id = object_to_structure[kid.number]
                child_ids.append(child_id)
                observed_k_order.append(("child", child_id))
                continue
            if not isinstance(kid, dict):
                raise PdfValidationError("structure /K member type differs")
            kind = kid.get("Type")
            if kind == PdfName("MCR"):
                _exact_keys(kid, {"MCID", "Pg", "Type"}, f"structure {index} MCR")
                page = page_by_object.get(_ref(kid["Pg"], "MCR /Pg"))
                if page is None:
                    raise PdfValidationError("MCR page differs")
                observed_mcr[index].append((page, _integer(kid["MCID"], "MCR /MCID")))
                observed_k_order.append(("mcr", page, _integer(kid["MCID"], "MCR /MCID")))
            elif kind == PdfName("OBJR"):
                _exact_keys(kid, {"Obj", "Pg", "Type"}, f"structure {index} OBJR")
                page = page_by_object.get(_ref(kid["Pg"], "OBJR /Pg"))
                if page is None:
                    raise PdfValidationError("OBJR page differs")
                annotation_object = _ref(kid["Obj"], "OBJR /Obj")
                observed_objr[index].append(annotation_object)
                observed_k_order.append(("objr", annotation_object))
            else:
                raise PdfValidationError("unknown structure /K dictionary")
        if child_ids != expected["children"]:
            raise PdfValidationError("structure logical child order differs")
        child_order = [("child", child) for child in expected["children"]]
        mcr_order = [("mcr", page, mcid) for page, mcid in expected_mcr[index]]
        expected_k_order = (
            mcr_order + child_order
            if expected["role"] in {"Figure", "Formula"}
            else child_order + mcr_order
        )
        expected_k_order.extend(("objr", number) for number in expected_objr[index])
        if observed_k_order != expected_k_order:
            raise PdfValidationError("structure /K logical order differs")

    if observed_mcr != expected_mcr:
        raise PdfValidationError("MCR/marked-content closure differs")
    if observed_objr != expected_objr:
        raise PdfValidationError("OBJR/annotation closure differs")

    parent_tree_number = roles["parent_tree"]
    if _ref(structure_root["ParentTree"], "structure /ParentTree") != parent_tree_number:
        raise PdfValidationError("structure ParentTree reference differs")
    parent_tree = objects[parent_tree_number].value
    _exact_keys(parent_tree, {"Nums"}, "ParentTree")
    nums = parent_tree["Nums"]
    if not isinstance(nums, list) or len(nums) % 2:
        raise PdfValidationError("ParentTree /Nums differs")
    observed_parent: list[dict[str, Any]] = []
    for offset in range(0, len(nums), 2):
        key = _integer(nums[offset], "ParentTree key")
        value = nums[offset + 1]
        if isinstance(value, list):
            observed_parent.append(
                {"key": key, "kind": "page", "structure_node_ids": [
                    object_to_structure[_ref(item, "ParentTree page member")] for item in value
                ]}
            )
        else:
            observed_parent.append(
                {"key": key, "kind": "annotation", "structure_node_id": object_to_structure[_ref(value, "ParentTree annotation member")]}
            )
    if observed_parent != manifest["marked_content"]["parent_tree"]:
        raise PdfValidationError("ParentTree values differ")
    if structure_root["ParentTreeNextKey"] != len(observed_parent):
        raise PdfValidationError("ParentTreeNextKey differs")

    expected_ids = sorted(
        (node["structure_id"], node["structure_node_id"])
        for node in structure if node["structure_id"] is not None
    )
    if expected_ids:
        if id_tree_role is None or _ref(structure_root["IDTree"], "structure /IDTree") != id_tree_role:
            raise PdfValidationError("IDTree reference is missing")
        id_tree = objects[id_tree_role].value
        _exact_keys(id_tree, {"Names"}, "IDTree")
        names = id_tree["Names"]
        observed_ids = [
            (
                _literal_ascii_text(names[offset], "IDTree name"),
                object_to_structure[_ref(names[offset + 1], "IDTree value")],
            )
            for offset in range(0, len(names), 2)
        ]
        if observed_ids != expected_ids:
            raise PdfValidationError("IDTree values differ")
    elif id_tree_role is not None or "IDTree" in structure_root:
        raise PdfValidationError("empty IDTree was emitted")

    pages_observed: list[list[dict[str, Any]]] = []
    marked = manifest["marked_content"]
    annotations = marked["annotations"]
    for page_index, page_expectation in enumerate(marked["pages"]):
        page = objects[page_roles[page_index]].value
        page_annotations = [
            annotation for annotation in annotations
            if annotation["page_index"] == page_index
        ]
        page_keys = {"Contents", "MediaBox", "Parent", "Resources", "Type"}
        if page_expectation["structure_parent_key"] is not None:
            page_keys.add("StructParents")
        if page_annotations:
            page_keys.update({"Annots", "Tabs"})
        _exact_keys(page, page_keys, f"tagged page {page_index}")
        _name(page["Type"], "Page", f"tagged page {page_index} /Type")
        media_box = page["MediaBox"]
        if (
            _ref(page["Parent"], f"tagged page {page_index} /Parent") != roles["pages"]
            or page["Resources"] != {}
            or not isinstance(media_box, list)
            or len(media_box) != 4
            or [_fixed(value, "tagged page MediaBox") for value in media_box]
            != [0, 0, page_expectation["width_raw"], page_expectation["height_raw"]]
        ):
            raise PdfValidationError("tagged page geometry/resources differ")
        if page.get("StructParents") != page_expectation["structure_parent_key"]:
            raise PdfValidationError("page StructParents differs")
        if page_annotations:
            if page["Tabs"] != PdfName("S"):
                raise PdfValidationError("annotated page /Tabs differs")
            expected_annots = [
                PdfRef(roles[f"link_annotation:{annotation['annotation_id']}"])
                for annotation in page_annotations
            ]
            if page["Annots"] != expected_annots:
                raise PdfValidationError("page /Annots order differs")
        content_number = _ref(page["Contents"], "page /Contents")
        if content_number != roles[f"page_content:{page_index}"]:
            raise PdfValidationError("tagged page content reference differs")
        expected_records = [record for record in marked["records"] if record["page_index"] == page_index]
        content_object = objects[content_number]
        _exact_keys(
            content_object.value,
            {"Length"},
            f"tagged page {page_index} content stream",
        )
        stream = content_object.stream
        if stream is None:
            raise PdfValidationError("page content stream is missing")
        pages_observed.append(_verify_tagged_content(stream, expected_records))

    annotation_objects: dict[int, int] = {}
    for expected in annotations:
        number = roles[f"link_annotation:{expected['annotation_id']}"]
        annotation_objects[expected["annotation_id"]] = number
        dictionary = objects[number].value
        _exact_keys(
            dictionary,
            {"Border", "Contents", "Dest", "P", "Rect", "StructParent", "Subtype", "Type"},
            f"Link annotation {expected['annotation_id']}",
        )
        _name(dictionary["Type"], "Annot", "Link annotation /Type")
        _name(dictionary["Subtype"], "Link", "Link annotation /Subtype")
        if (
            not isinstance(dictionary["Border"], list)
            or len(dictionary["Border"]) != 3
            or any(
                _integer(value, "Link annotation /Border") != 0
                for value in dictionary["Border"]
            )
        ):
            raise PdfValidationError("Link annotation border differs")
        if _utf16_text(dictionary["Contents"], "Link annotation /Contents") != expected["accessible_name"]:
            raise PdfValidationError("Link accessible name differs")
        if _literal_ascii_text(dictionary["Dest"], "Link annotation /Dest") != expected["destination"]:
            raise PdfValidationError("Link destination differs")
        if dictionary["StructParent"] != expected["structure_parent_key"]:
            raise PdfValidationError("annotation StructParent differs")
        if _ref(dictionary["P"], "Link annotation /P") != page_roles[expected["page_index"]]:
            raise PdfValidationError("Link annotation page differs")
        rect = [_fixed(value, "Link annotation rectangle") for value in dictionary["Rect"]]
        if rect != expected["rect"]:
            raise PdfValidationError("Link annotation rectangle differs")
        owner_objrs = observed_objr[expected["structure_node_id"]]
        if number not in owner_objrs:
            raise PdfValidationError("Link OBJR owner differs")
    all_objr = [number for values in observed_objr.values() for number in values]
    if sorted(all_objr) != sorted(annotation_objects.values()):
        raise PdfValidationError("Link annotation/OBJR closure differs")

    outline = manifest["outline"]
    if outline:
        outline_root_number = roles["outline_root"]
        outline_objects = {
            entry["outline_id"]: roles[f"outline_item:{entry['outline_id']}"]
            for entry in outline
        }
        children: dict[int | None, list[int]] = {}
        for entry in outline:
            children.setdefault(entry["parent_outline_id"], []).append(entry["outline_id"])
        top = children[None]
        outline_root = objects[outline_root_number].value
        _exact_keys(outline_root, {"Count", "First", "Last", "Type"}, "tagged outline root")
        _name(outline_root["Type"], "Outlines", "tagged outline root /Type")
        if (
            outline_root["Count"] != len(outline)
            or _ref(outline_root["First"], "outline root /First") != outline_objects[top[0]]
            or _ref(outline_root["Last"], "outline root /Last") != outline_objects[top[-1]]
        ):
            raise PdfValidationError("tagged outline root differs")
        for expected in outline:
            outline_id = expected["outline_id"]
            dictionary = objects[outline_objects[outline_id]].value
            siblings = children[expected["parent_outline_id"]]
            position = siblings.index(outline_id)
            direct_children = children.get(outline_id, [])
            keys = {"Dest", "Parent", "SE", "Title"}
            if position:
                keys.add("Prev")
            if position + 1 < len(siblings):
                keys.add("Next")
            if direct_children:
                keys.update({"Count", "First", "Last"})
            _exact_keys(dictionary, keys, f"tagged outline item {outline_id}")
            expected_parent_object = (
                outline_root_number
                if expected["parent_outline_id"] is None
                else outline_objects[expected["parent_outline_id"]]
            )
            if _ref(dictionary["Parent"], "outline /Parent") != expected_parent_object:
                raise PdfValidationError("outline parent differs")
            if position and _ref(dictionary["Prev"], "outline /Prev") != outline_objects[siblings[position - 1]]:
                raise PdfValidationError("outline previous sibling differs")
            if position + 1 < len(siblings) and _ref(dictionary["Next"], "outline /Next") != outline_objects[siblings[position + 1]]:
                raise PdfValidationError("outline next sibling differs")
            if direct_children:
                descendants = 0
                for candidate in outline[outline_id + 1:]:
                    if candidate["level"] <= expected["level"]:
                        break
                    descendants += 1
                if (
                    _ref(dictionary["First"], "outline /First") != outline_objects[direct_children[0]]
                    or _ref(dictionary["Last"], "outline /Last") != outline_objects[direct_children[-1]]
                    or dictionary["Count"] != descendants
                ):
                    raise PdfValidationError("outline child/count closure differs")
            if _ref(dictionary.get("SE"), "outline /SE") != structure_objects[expected["structure_node_id"]]:
                raise PdfValidationError("outline /SE differs")
            if _utf16_text(dictionary["Title"], "outline /Title") != expected["label"]:
                raise PdfValidationError("outline title differs")
            if _literal_ascii_text(dictionary["Dest"], "outline /Dest") != expected["destination"]:
                raise PdfValidationError("outline destination differs")

    metadata_object = objects[roles["metadata"]]
    _exact_keys(metadata_object.value, {"Length", "Subtype", "Type"}, "tagged Metadata stream")
    _name(metadata_object.value["Type"], "Metadata", "tagged Metadata /Type")
    _name(metadata_object.value["Subtype"], "XML", "tagged Metadata /Subtype")
    metadata = metadata_object.stream
    if metadata is None or metadata != _expected_tagged_xmp(manifest):
        raise PdfValidationError("PDF/UA metadata/XMP differs")
    if hashlib.sha256(metadata).hexdigest() != manifest["fingerprints"]["xmp_sha256"]:
        raise PdfValidationError("tagged XMP hash differs")
    return {
        "actual_text": [
            record["actual_text"]
            for node in structure
            for record in sorted(
                (
                    candidate
                    for candidate in marked["records"]
                    if candidate["owner"]["kind"] == "structure"
                    and candidate["owner"]["structure_node_id"] == node["structure_node_id"]
                    and candidate["actual_text"] is not None
                ),
                key=lambda candidate: candidate["semantic_fragment_ordinal"],
            )
        ],
        "alternatives": [
            {"role": node["role"], "text": node["alternative"]}
            for node in structure if node["alternative"] is not None
        ],
        "artifact_count": sum(page["artifact_count"] for page in marked["pages"]),
        "catalog_language": language,
        "marked_pages": pages_observed,
        "outline_structure": [entry["structure_node_id"] for entry in manifest["outline"]],
        "pdf_sha256": pdf_hash,
        "reading_order": [node["structure_node_id"] for node in structure],
        "roles": [node["role"] for node in structure],
        "structure_count": len(structure),
        "xmp_sha256": hashlib.sha256(metadata).hexdigest(),
    }


def _tagged_v2_expectation_shape(expectation: dict[str, Any]) -> None:
    _exact_keys(
        expectation,
        {
            "algorithm", "document_language", "equation_numbers", "form_count",
            "object_budget_charge_count", "observation_algorithm", "page_count",
            "pdf", "vectors", "xmp_sha256",
        },
        "tagged-PDF /2 expectation",
    )
    if expectation["algorithm"] != "typaxis.tagged-pdf-validator/2":
        raise PdfValidationError("tagged-PDF validator algorithm is not /2")
    if expectation["observation_algorithm"] != "typaxis.tagged-pdf-observation/2":
        raise PdfValidationError("tagged-PDF observation algorithm is not /2")
    _canonical_language(expectation["document_language"], "tagged-PDF document language")
    page_count = _json_integer(expectation["page_count"], "tagged-PDF page count", 1)
    form_count = _json_integer(expectation["form_count"], "tagged-PDF Form count", 1)
    if expectation["object_budget_charge_count"] != 1:
        raise PdfValidationError("tagged-PDF object budget must be charged exactly once")
    if (
        not isinstance(expectation["xmp_sha256"], str)
        or re.fullmatch(r"[0-9a-f]{64}", expectation["xmp_sha256"]) is None
    ):
        raise PdfValidationError("tagged-PDF XMP hash is not lowercase SHA-256")

    pdf = expectation["pdf"]
    _exact_keys(pdf, {"byte_length", "object_count", "objects", "sha256"}, "tagged-PDF /2 closure")
    _json_integer(pdf["byte_length"], "tagged-PDF byte length", 1)
    object_count = _json_integer(pdf["object_count"], "tagged-PDF object count", 1)
    if not isinstance(pdf["sha256"], str) or re.fullmatch(r"[0-9a-f]{64}", pdf["sha256"]) is None:
        raise PdfValidationError("tagged-PDF hash is not lowercase SHA-256")
    if not isinstance(pdf["objects"], list) or len(pdf["objects"]) != object_count:
        raise PdfValidationError("tagged-PDF object closure count differs")
    roles: set[str] = set()
    singleton_roles = {
        "catalog", "pages", "destinations", "info",
        "metadata", "outline_root", "structure_tree_root", "structure_parent_tree",
        "structure_id_tree",
    }
    indexed_roles: dict[str, list[int]] = {
        prefix: []
        for prefix in (
            "page_content", "page", "link_annotation", "outline_item", "vector_form",
            "vector_ext_g_state", "structure_element", "equation_font_type0",
            "equation_font_cid", "equation_font_descriptor", "equation_font_program",
            "equation_font_to_unicode", "equation_font_cid_to_gid",
        )
    }
    for index, item in enumerate(pdf["objects"], 1):
        _exact_keys(item, {"object_number", "role", "sha256"}, f"tagged-PDF /2 object {index}")
        if item["object_number"] != index or not isinstance(item["role"], str) or not item["role"]:
            raise PdfValidationError("tagged-PDF object closure is not dense")
        if item["role"] in roles:
            raise PdfValidationError("tagged-PDF object closure has a duplicate role")
        roles.add(item["role"])
        role = item["role"]
        if role not in singleton_roles:
            prefix, separator, suffix = role.partition(":")
            if (
                not separator
                or prefix not in indexed_roles
                or re.fullmatch(r"0|[1-9][0-9]*", suffix) is None
                or len(suffix) > 10
            ):
                raise PdfValidationError("tagged-PDF object role is not canonical")
            role_index = int(suffix)
            if role_index > 0xFFFFFFFF:
                raise PdfValidationError("tagged-PDF object role index is outside u32")
            indexed_roles[prefix].append(role_index)
        if not isinstance(item["sha256"], str) or re.fullmatch(r"[0-9a-f]{64}", item["sha256"]) is None:
            raise PdfValidationError("tagged-PDF object hash is not lowercase SHA-256")

    required_singletons = {
        "catalog", "pages", "destinations", "info", "metadata",
        "structure_tree_root", "structure_parent_tree",
    }
    if not required_singletons.issubset(roles) or pdf["objects"][0]["role"] != "catalog":
        raise PdfValidationError("tagged-PDF required singleton role plan differs")
    for prefix in ("page_content", "page"):
        if sorted(indexed_roles[prefix]) != list(range(page_count)):
            raise PdfValidationError(f"tagged-PDF {prefix} role plan differs")
    for prefix in ("link_annotation", "outline_item", "structure_element"):
        values = sorted(indexed_roles[prefix])
        if values != list(range(len(values))):
            raise PdfValidationError(f"tagged-PDF {prefix} role plan is not dense")
    relative_vectors = sorted(indexed_roles["vector_form"] + indexed_roles["vector_ext_g_state"])
    if relative_vectors != list(range(len(relative_vectors))):
        raise PdfValidationError("tagged-PDF vector object role plan is not dense")
    if len(indexed_roles["vector_form"]) != form_count:
        raise PdfValidationError("tagged-PDF vector Form role count differs")
    if ("outline_root" in roles) != bool(indexed_roles["outline_item"]):
        raise PdfValidationError("tagged-PDF outline role plan differs")

    vectors = expectation["vectors"]
    numbers = expectation["equation_numbers"]
    if not isinstance(vectors, list) or not vectors or not isinstance(numbers, list):
        raise PdfValidationError("tagged-PDF vector/number expectations are not arrays")
    kinds = {
        "inline_vector": "Figure",
        "math_vector": "Formula",
        "vector_figure": "Figure",
        "math_vector_block": "Formula",
    }
    structure_nodes: set[int] = set()
    form_indices: set[int] = set()
    page_mcids: dict[int, list[int]] = {page: [] for page in range(page_count)}
    vector_by_node: dict[int, dict[str, Any]] = {}
    for index, vector in enumerate(vectors):
        _exact_keys(
            vector,
            {
                "actual_text", "alternative", "form_index", "kind", "mcid",
                "page_index", "paint_language", "structure_language", "structure_node_id",
            },
            f"tagged vector {index}",
        )
        if vector["kind"] not in kinds:
            raise PdfValidationError("tagged vector kind is not closed")
        page = _json_integer(vector["page_index"], "tagged vector page")
        mcid = _json_integer(vector["mcid"], "tagged vector MCID")
        node = _json_integer(vector["structure_node_id"], "tagged vector structure node", 1)
        form_index = _json_integer(vector["form_index"], "tagged vector Form index")
        if page >= page_count or form_index >= form_count or node in structure_nodes:
            raise PdfValidationError("tagged vector page/Form/node is outside the closed domain")
        structure_nodes.add(node)
        form_indices.add(form_index)
        page_mcids[page].append(mcid)
        vector_by_node[node] = vector
        if not isinstance(vector["alternative"], str) or not vector["alternative"].strip():
            raise PdfValidationError("tagged vector alternative is empty")
        actual = vector["actual_text"]
        if actual is not None and (not isinstance(actual, str) or not actual.strip()):
            raise PdfValidationError("tagged vector ActualText is invalid")
        if vector["kind"] in {"math_vector", "math_vector_block"} and actual is None:
            raise PdfValidationError("tagged math vector requires resolved ActualText")
        if vector["kind"] == "vector_figure" and actual is not None:
            raise PdfValidationError("vector Figure cannot carry paint ActualText")
        for key in ("paint_language", "structure_language"):
            if vector[key] is not None:
                _canonical_language(vector[key], f"tagged vector {key}")
    if form_indices != set(range(form_count)):
        raise PdfValidationError("tagged vector Forms are not all used")

    for index, number in enumerate(numbers):
        _exact_keys(
            number,
            {
                "exact_text", "font_index", "mcid", "page_index", "paint_language",
                "parent_structure_node_id", "structure_language", "structure_node_id",
            },
            f"equation number {index}",
        )
        font_index = _json_integer(number["font_index"], "equation-number font index")
        page = _json_integer(number["page_index"], "equation-number page")
        mcid = _json_integer(number["mcid"], "equation-number MCID")
        node = _json_integer(number["structure_node_id"], "equation-number structure node", 1)
        parent = _json_integer(number["parent_structure_node_id"], "equation-number parent", 1)
        if page >= page_count or node in structure_nodes:
            raise PdfValidationError("equation-number page/node is outside the closed domain")
        structure_nodes.add(node)
        page_mcids[page].append(mcid)
        parent_vector = vector_by_node.get(parent)
        if (
            parent_vector is None
            or parent_vector["kind"] != "math_vector_block"
            or parent_vector["page_index"] != page
            or parent_vector["mcid"] + 1 != mcid
        ):
            raise PdfValidationError("equation-number Formula relationship/order differs")
        text = number["exact_text"]
        if (
            not isinstance(text, str)
            or not text.strip()
            or any(ord(character) < 0x20 or 0x7F <= ord(character) <= 0x9F for character in text)
        ):
            raise PdfValidationError("equation-number text is not supported normal text")
        for key in ("paint_language", "structure_language"):
            if number[key] is not None:
                _canonical_language(number[key], f"equation number {key}")
    font_indices = sorted({number["font_index"] for number in numbers})
    if font_indices != list(range(len(font_indices))):
        raise PdfValidationError("tagged-PDF equation font indices are not dense")
    for prefix in (
        "equation_font_type0", "equation_font_cid", "equation_font_descriptor",
        "equation_font_program", "equation_font_to_unicode", "equation_font_cid_to_gid",
    ):
        if sorted(indexed_roles[prefix]) != font_indices:
            raise PdfValidationError(f"tagged-PDF {prefix} role plan differs")
    for page, mcids in page_mcids.items():
        if sorted(mcids) != list(range(len(mcids))):
            raise PdfValidationError(f"tagged page {page} MCIDs are not dense")


def _tagged_v2_roles(expectation: dict[str, Any]) -> dict[str, int]:
    return {item["role"]: item["object_number"] for item in expectation["pdf"]["objects"]}


def _tagged_v2_equation_fonts(
    objects: dict[int, ParsedObject],
    roles: dict[str, int],
    equation_numbers: list[dict[str, Any]],
) -> dict[int, tuple[int, frozenset[int]]]:
    facts: dict[int, tuple[int, frozenset[int]]] = {}
    for index in sorted({number["font_index"] for number in equation_numbers}):
        type0_number = roles[f"equation_font_type0:{index}"]
        cid_number = roles[f"equation_font_cid:{index}"]
        descriptor_number = roles[f"equation_font_descriptor:{index}"]
        program_number = roles[f"equation_font_program:{index}"]
        cmap_number = roles[f"equation_font_to_unicode:{index}"]
        gid_map_number = roles[f"equation_font_cid_to_gid:{index}"]

        type0 = objects[type0_number].value
        _exact_keys(
            type0,
            {"BaseFont", "DescendantFonts", "Encoding", "Subtype", "ToUnicode", "Type"},
            f"equation font {index} Type0",
        )
        _name(type0["Type"], "Font", f"equation font {index} /Type")
        _name(type0["Subtype"], "Type0", f"equation font {index} /Subtype")
        _name(type0["Encoding"], "Identity-H", f"equation font {index} /Encoding")
        if (
            not isinstance(type0["BaseFont"], PdfName)
            or type0["DescendantFonts"] != [PdfRef(cid_number)]
            or _ref(type0["ToUnicode"], "equation Type0 /ToUnicode") != cmap_number
        ):
            raise PdfValidationError("equation Type0 font closure differs")

        cid = objects[cid_number].value
        _exact_keys(
            cid,
            {
                "BaseFont", "CIDSystemInfo", "CIDToGIDMap", "DW", "FontDescriptor",
                "Subtype", "Type", "W",
            },
            f"equation font {index} CIDFont",
        )
        _name(cid["Type"], "Font", f"equation font {index} CID /Type")
        _name(cid["Subtype"], "CIDFontType2", f"equation font {index} CID /Subtype")
        system_info = cid["CIDSystemInfo"]
        _exact_keys(system_info, {"Ordering", "Registry", "Supplement"}, "CIDSystemInfo")
        if (
            cid["BaseFont"] != type0["BaseFont"]
            or _literal_ascii_text(system_info["Registry"], "CID registry") != "Adobe"
            or _literal_ascii_text(system_info["Ordering"], "CID ordering") != "Identity"
            or _integer(system_info["Supplement"], "CID supplement") != 0
            or _integer(cid["DW"], "CID default width") != 1000
            or _ref(cid["FontDescriptor"], "CID /FontDescriptor") != descriptor_number
            or _ref(cid["CIDToGIDMap"], "CID /CIDToGIDMap") != gid_map_number
        ):
            raise PdfValidationError("equation CID font closure differs")

        descriptor = objects[descriptor_number].value
        _exact_keys(
            descriptor,
            {
                "Ascent", "CapHeight", "Descent", "Flags", "FontBBox", "FontFile2",
                "FontName", "ItalicAngle", "StemV", "Type",
            },
            f"equation font {index} descriptor",
        )
        _name(descriptor["Type"], "FontDescriptor", "equation font descriptor /Type")
        if (
            descriptor["FontName"] != type0["BaseFont"]
            or _ref(descriptor["FontFile2"], "font descriptor /FontFile2") != program_number
            or not isinstance(descriptor["FontBBox"], list)
            or len(descriptor["FontBBox"]) != 4
        ):
            raise PdfValidationError("equation font descriptor closure differs")

        program = objects[program_number]
        _exact_keys(program.value, {"Length", "Length1"}, "equation font program")
        if (
            program.stream is None
            or program.value["Length1"] != len(program.stream)
            or not program.stream.startswith(b"\x00\x01\x00\x00")
        ):
            raise PdfValidationError("equation font program differs")

        gid_map = objects[gid_map_number]
        _exact_keys(gid_map.value, {"Length"}, "equation CIDToGIDMap")
        if (
            gid_map.stream is None
            or len(gid_map.stream) < 4
            or len(gid_map.stream) % 2
            or gid_map.stream[:2] != b"\x00\x00"
        ):
            raise PdfValidationError("equation CIDToGIDMap differs")
        cid_count = len(gid_map.stream) // 2 - 1
        valid_cids = frozenset(range(1, cid_count + 1))

        widths = cid["W"]
        if not isinstance(widths, list) or len(widths) != cid_count * 2:
            raise PdfValidationError("equation CID widths differ")
        for expected_cid, offset in enumerate(range(0, len(widths), 2), 1):
            width = widths[offset + 1]
            if (
                widths[offset] != expected_cid
                or not isinstance(width, list)
                or len(width) != 1
                or not isinstance(width[0], int)
                or isinstance(width[0], bool)
            ):
                raise PdfValidationError("equation CID widths are not canonical")

        cmap = objects[cmap_number]
        _exact_keys(cmap.value, {"Length"}, "equation ToUnicode CMap")
        prefix = (
            b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
            b"/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n"
            b"/CMapName /Typaxis-Identity-UCS def\n/CMapType 2 def\n"
            b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
        )
        suffix = b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
        if cmap.stream is None or not cmap.stream.startswith(prefix) or not cmap.stream.endswith(suffix):
            raise PdfValidationError("equation ToUnicode CMap differs")
        mappings = cmap.stream[len(prefix):-len(suffix)]
        mapped_cids: set[int] = set()
        # All number glyphs are independently required to be inside an exact
        # ActualText scope. A font used solely by such clusters may have no
        # per-CID mappings; unknown operators must still fail closed.
        cursor = 0
        previous_cid = 0
        while cursor < len(mappings):
            chunk = re.match(
                rb"([1-9][0-9]*) beginbfchar\n(.*?)endbfchar\n",
                mappings[cursor:],
                re.DOTALL,
            )
            if chunk is None or len(chunk[1]) > 3:
                raise PdfValidationError("equation ToUnicode bfchar syntax differs")
            count = int(chunk[1])
            lines = chunk[2].splitlines()
            if count > 100 or count != len(lines):
                raise PdfValidationError("equation ToUnicode bfchar count differs")
            for line in lines:
                match = re.fullmatch(rb"<([0-9A-F]{4})> <((?:[0-9A-F]{4})+)>", line)
                if match is None:
                    raise PdfValidationError("equation ToUnicode bfchar syntax differs")
                source = int(match.group(1), 16)
                if source <= previous_cid or source not in valid_cids:
                    raise PdfValidationError("equation ToUnicode source CID differs")
                try:
                    bytes.fromhex(match.group(2).decode("ascii")).decode("utf-16-be")
                except (UnicodeError, ValueError) as error:
                    raise PdfValidationError("equation ToUnicode destination is invalid") from error
                mapped_cids.add(source)
                previous_cid = source
            cursor += chunk.end()
        facts[index] = (type0_number, valid_cids)
    return facts


def _tagged_v2_parse_outer(content: bytes, cursor: int) -> tuple[PdfName, dict[str, Any], int]:
    parser = PdfParser(content[cursor:])
    tag = parser.parse()
    properties = parser.parse()
    parser.skip_space()
    if not isinstance(tag, PdfName) or not isinstance(properties, dict):
        raise PdfValidationError("tagged-PDF /2 marked-content prefix differs")
    cursor += parser.pos
    if not content.startswith(b"BDC\n", cursor):
        raise PdfValidationError("tagged-PDF /2 BDC is missing")
    return tag, properties, cursor + len(b"BDC\n")


def _tagged_v2_content(
    content: bytes,
    page: int,
    records: list[tuple[str, dict[str, Any]]],
    page_dictionary: dict[str, Any],
    form_objects: list[int],
    equation_fonts: dict[int, tuple[int, frozenset[int]]],
) -> tuple[int, list[str]]:
    if not content.startswith(b"q\n1 0 0 -1 0 ") or not content.endswith(b"Q"):
        raise PdfValidationError("tagged-PDF /2 page-root transform differs")
    first_end = content.find(b"\n")
    transform_end = content.find(b"\n", first_end + 1)
    if first_end != 1 or transform_end < 0:
        raise PdfValidationError("tagged-PDF /2 page-root transform is unterminated")
    transform = content[first_end + 1:transform_end]
    match = re.fullmatch(rb"1 0 0 -1 0 ([0-9]+(?:\.[0-9]+)?) cm", transform)
    media_box = page_dictionary.get("MediaBox")
    if (
        match is None
        or not isinstance(media_box, list)
        or len(media_box) != 4
        or _fixed(media_box[0], f"tagged page {page} MediaBox x") != 0
        or _fixed(media_box[1], f"tagged page {page} MediaBox y") != 0
        or _fixed(Decimal(match.group(1).decode("ascii")), f"tagged page {page} transform")
        != _fixed(media_box[3], f"tagged page {page} MediaBox height")
    ):
        raise PdfValidationError("tagged-PDF /2 page-root transform height differs")
    cursor = transform_end + 1
    xobjects = page_dictionary.get("Resources", {}).get("XObject", {})
    if not isinstance(xobjects, dict):
        raise PdfValidationError("tagged-PDF /2 page XObject resources differ")
    do_count = 0
    extracted: list[str] = []
    for record_kind, record in records:
        tag, outer, cursor = _tagged_v2_parse_outer(content, cursor)
        if record_kind == "vector":
            role = "Formula" if record["kind"] in {"math_vector", "math_vector_block"} else "Figure"
            _name(tag, role, "tagged vector outer role")
            _exact_keys(outer, {"MCID"}, "tagged vector outer properties")
            if _integer(outer["MCID"], "tagged vector MCID") != record["mcid"]:
                raise PdfValidationError("tagged vector MCID differs")
            nested = record["actual_text"] is not None or record["paint_language"] is not None
            if nested:
                inner_tag, inner, cursor = _tagged_v2_parse_outer(content, cursor)
                _name(inner_tag, "Span", "tagged vector inner role")
                keys = ({"ActualText"} if record["actual_text"] is not None else set()) | (
                    {"Lang"} if record["paint_language"] is not None else set()
                )
                _exact_keys(inner, keys, "tagged vector inner properties")
                if (
                    (record["actual_text"] is not None and _utf16_text(inner["ActualText"], "vector ActualText") != record["actual_text"])
                    or (record["paint_language"] is not None and _utf16_text(inner["Lang"], "vector Lang") != record["paint_language"])
                ):
                    raise PdfValidationError("tagged vector inner properties differ")
            paint_end = content.find(b"EMC\n", cursor)
            if paint_end < 0:
                raise PdfValidationError("tagged vector marked content is unclosed")
            paint = content[cursor:paint_end]
            do_lines = [line for line in paint.splitlines() if line.endswith(b" Do")]
            if len(do_lines) != 1 or any(
                token in paint
                for token in (
                    b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b"BDC", b"BMC",
                    b"EMC", b"BT ",
                )
            ):
                raise PdfValidationError("tagged vector Do/nesting differs")
            resource_name = do_lines[0][:-len(b" Do")]
            if not resource_name.startswith(b"/V"):
                raise PdfValidationError("tagged vector resource name differs")
            try:
                name = resource_name[1:].decode("ascii")
            except UnicodeDecodeError as error:
                raise PdfValidationError("tagged vector resource name is not ASCII") from error
            target = xobjects.get(name)
            if not isinstance(target, PdfRef) or target.number != form_objects[record["form_index"]]:
                raise PdfValidationError("tagged vector Form binding differs")
            cursor = paint_end + len(b"EMC\n")
            if nested:
                if not content.startswith(b"EMC\n", cursor):
                    raise PdfValidationError("tagged vector outer EMC is missing")
                cursor += len(b"EMC\n")
            if record["actual_text"] is not None:
                extracted.append(record["actual_text"])
            do_count += 1
        else:
            _name(tag, "Span", "equation-number outer role")
            keys = {"MCID"} | ({"Lang"} if record["paint_language"] is not None else set())
            _exact_keys(outer, keys, "equation-number outer properties")
            if _integer(outer["MCID"], "equation-number MCID") != record["mcid"]:
                raise PdfValidationError("equation-number MCID differs")
            if record["paint_language"] is not None and _utf16_text(outer["Lang"], "equation-number Lang") != record["paint_language"]:
                raise PdfValidationError("equation-number language differs")
            inner_tag, inner, cursor = _tagged_v2_parse_outer(content, cursor)
            _name(inner_tag, "Span", "equation-number extraction role")
            _exact_keys(inner, {"ActualText"}, "equation-number extraction properties")
            if _utf16_text(inner["ActualText"], "equation-number ActualText") != record["exact_text"]:
                raise PdfValidationError("equation-number ActualText differs")
            paint_end = content.find(b"EMC\n", cursor)
            if paint_end < 0:
                raise PdfValidationError("equation-number marked content is unclosed")
            paint = content[cursor:paint_end]
            font_index = record["font_index"]
            type0_object, valid_cids = equation_fonts[font_index]
            fonts = page_dictionary.get("Resources", {}).get("Font", {})
            font_name = f"F{font_index}"
            if (
                not isinstance(fonts, dict)
                or fonts.get(font_name) != PdfRef(type0_object)
            ):
                raise PdfValidationError("equation-number page font binding differs")
            number = rb"[+-]?[0-9]+(?:\.[0-9]+)?"
            glyph_line = (
                rb"1 0 0 -1 " + number + rb" " + number
                + rb" Tm <([0-9A-F]{4})> Tj\n"
            )
            header = (
                b"0 g\nBT /"
                + font_name.encode("ascii")
                + b" "
                + number
                + b" Tf 0 Tr\n"
            )
            match = re.fullmatch(
                header + rb"(?:" + glyph_line + rb")+ET\n",
                paint,
            )
            shown_cids = {
                int(value, 16)
                for value in re.findall(rb"<([0-9A-F]{4})> Tj\n", paint)
            }
            if (
                match is None
                or not shown_cids
                or not shown_cids.issubset(valid_cids)
                or any(
                    token in paint
                    for token in (
                        b" Do\n", b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b"BDC",
                        b"BMC", b"EMC",
                    )
                )
            ):
                raise PdfValidationError("equation-number normal text/extraction differs")
            extracted.append(record["exact_text"])
            cursor = paint_end + len(b"EMC\n")
            if not content.startswith(b"EMC\n", cursor):
                raise PdfValidationError("equation-number outer EMC is missing")
            cursor += len(b"EMC\n")
    if content[cursor:] != b"Q" or content.count(b"/MCID ") != len(records):
        raise PdfValidationError("extra or missing tagged-PDF /2 page content")
    return do_count, extracted


def _tagged_v2_mcr(value: Any, page_object: int, mcid: int, label: str) -> None:
    _exact_keys(value, {"MCID", "Pg", "Type"}, label)
    _name(value["Type"], "MCR", f"{label} /Type")
    if _ref(value["Pg"], f"{label} /Pg") != page_object or _integer(value["MCID"], f"{label} /MCID") != mcid:
        raise PdfValidationError(f"{label} page/MCID differs")


def _tagged_v2_outline_graph(
    objects: dict[int, ParsedObject],
    roles: dict[str, int],
    page_objects: dict[int, int],
) -> None:
    name_tree = objects[roles["destinations"]].value
    _exact_keys(name_tree, {"Names"}, "tagged-PDF /2 destination name tree")
    names = name_tree["Names"]
    if not isinstance(names, list) or len(names) % 2:
        raise PdfValidationError("tagged-PDF /2 destination name/value pairs differ")
    destination_names: list[str] = []
    pages_by_object = {number: page for page, number in page_objects.items()}
    for offset in range(0, len(names), 2):
        destination_names.append(_literal_ascii_text(names[offset], "tagged destination name"))
        if not isinstance(names[offset + 1], list):
            raise PdfValidationError("tagged-PDF /2 destination value is not an array")
        _parse_destination_view(names[offset + 1], pages_by_object)
    if destination_names != sorted(set(destination_names)):
        raise PdfValidationError("tagged-PDF /2 destination names are not unique/sorted")

    item_numbers = {
        number for role, number in roles.items() if role.startswith("outline_item:")
    }
    root_number = roles.get("outline_root")
    if root_number is None:
        if item_numbers:
            raise PdfValidationError("tagged-PDF /2 outline root is absent")
        return
    root = objects[root_number].value
    _exact_keys(root, {"Count", "First", "Last", "Type"}, "tagged-PDF /2 outline root")
    _name(root["Type"], "Outlines", "tagged-PDF /2 outline root /Type")
    if _integer(root["Count"], "tagged outline root /Count") != len(item_numbers):
        raise PdfValidationError("tagged-PDF /2 outline count differs")
    structure_numbers = {
        number for role, number in roles.items() if role.startswith("structure_element:")
    }
    parents: dict[int, int] = {}
    children: dict[int, set[int]] = {}
    for number in sorted(item_numbers):
        item = objects[number].value
        if not isinstance(item, dict):
            raise PdfValidationError("tagged-PDF /2 outline item is not a dictionary")
        required = {"Dest", "Parent", "SE", "Title"}
        optional = {"Count", "First", "Last", "Next", "Prev"}
        if not required.issubset(item) or not set(item).issubset(required | optional):
            raise PdfValidationError("tagged-PDF /2 outline item keys differ")
        parent = _ref(item["Parent"], "tagged outline /Parent")
        if (
            parent not in item_numbers | {root_number}
            or _ref(item["SE"], "tagged outline /SE") not in structure_numbers
            or not _utf16_text(item["Title"], "tagged outline /Title")
            or _literal_ascii_text(item["Dest"], "tagged outline /Dest") not in destination_names
        ):
            raise PdfValidationError("tagged-PDF /2 outline structure/destination closure differs")
        parents[number] = parent
        children.setdefault(parent, set()).add(number)

    descendant_counts: dict[int, int] = {}
    for number in item_numbers:
        parent = parents[number]
        visited: set[int] = set()
        while parent != root_number:
            if parent in visited or parent not in parents:
                raise PdfValidationError("tagged-PDF /2 outline parent cycle differs")
            visited.add(parent)
            descendant_counts[parent] = descendant_counts.get(parent, 0) + 1
            parent = parents[parent]

    for parent_number in item_numbers | {root_number}:
        parent = objects[parent_number].value
        direct = children.get(parent_number, set())
        if not direct:
            if any(key in parent for key in ("First", "Last", "Count")):
                raise PdfValidationError("tagged-PDF /2 leaf outline has child fields")
            continue
        first = _ref(parent.get("First"), "tagged outline /First")
        last = _ref(parent.get("Last"), "tagged outline /Last")
        current: int | None = first
        previous: int | None = None
        visited = set()
        while current is not None:
            if current not in direct or current in visited:
                raise PdfValidationError("tagged-PDF /2 outline sibling closure differs")
            visited.add(current)
            item = objects[current].value
            observed_previous = _ref(item["Prev"], "tagged outline /Prev") if "Prev" in item else None
            if observed_previous != previous:
                raise PdfValidationError("tagged-PDF /2 outline previous sibling differs")
            previous = current
            current = _ref(item["Next"], "tagged outline /Next") if "Next" in item else None
        if previous != last or visited != direct:
            raise PdfValidationError("tagged-PDF /2 outline sibling coverage differs")
        count = len(item_numbers) if parent_number == root_number else descendant_counts[parent_number]
        if _integer(parent.get("Count"), "tagged outline /Count") != count:
            raise PdfValidationError("tagged-PDF /2 outline descendant count differs")


def _tagged_v2_structure(
    objects: dict[int, ParsedObject],
    roles: dict[str, int],
    expectation: dict[str, Any],
    page_objects: dict[int, int],
) -> None:
    root = objects[roles["structure_tree_root"]].value
    if not isinstance(root, dict) or root.get("Type") != PdfName("StructTreeRoot"):
        raise PdfValidationError("tagged-PDF /2 StructTreeRoot differs")
    if _ref(root.get("ParentTree"), "StructTreeRoot /ParentTree") != roles["structure_parent_tree"]:
        raise PdfValidationError("tagged-PDF /2 ParentTree reference differs")
    if root.get("RoleMap") != {
        "Em": PdfName("Span"),
        "Exercise": PdfName("Div"),
        "Proof": PdfName("Div"),
        "Result": PdfName("Div"),
        "Strong": PdfName("Span"),
    }:
        raise PdfValidationError("tagged-PDF /2 RoleMap differs")
    if ("IDTree" in root) != ("structure_id_tree" in roles):
        raise PdfValidationError("tagged-PDF /2 IDTree role/reference presence differs")
    if "structure_id_tree" in roles and _ref(root["IDTree"], "StructTreeRoot /IDTree") != roles["structure_id_tree"]:
        raise PdfValidationError("tagged-PDF /2 IDTree reference differs")
    structure_ids: dict[str, int] = {}
    for role, number in roles.items():
        if not role.startswith("structure_element:"):
            continue
        dictionary = objects[number].value
        if not isinstance(dictionary, dict) or dictionary.get("Type") != PdfName("StructElem"):
            raise PdfValidationError("tagged-PDF /2 structure object type differs")
        if "ID" in dictionary:
            identity = _literal_ascii_text(dictionary["ID"], "tagged StructElem /ID")
            if not identity or identity in structure_ids:
                raise PdfValidationError("tagged-PDF /2 structure IDs are empty/duplicate")
            structure_ids[identity] = number
    if bool(structure_ids) != ("structure_id_tree" in roles):
        raise PdfValidationError("tagged-PDF /2 structure IDs and IDTree presence differ")
    if "structure_id_tree" in roles:
        id_tree = objects[roles["structure_id_tree"]].value
        _exact_keys(id_tree, {"Names"}, "tagged-PDF /2 IDTree")
        names = id_tree["Names"]
        if not isinstance(names, list) or len(names) != 2 * len(structure_ids):
            raise PdfValidationError("tagged-PDF /2 IDTree pair count differs")
        observed_ids = [
            (_literal_ascii_text(names[offset], "IDTree name"), _ref(names[offset + 1], "IDTree target"))
            for offset in range(0, len(names), 2)
        ]
        if observed_ids != sorted(structure_ids.items()):
            raise PdfValidationError("tagged-PDF /2 IDTree/StructElem correspondence differs")

    vectors = {item["structure_node_id"]: item for item in expectation["vectors"]}
    numbers = {item["structure_node_id"]: item for item in expectation["equation_numbers"]}
    for node, vector in vectors.items():
        dictionary = objects[roles[f"structure_element:{node}"]].value
        if not isinstance(dictionary, dict) or dictionary.get("Type") != PdfName("StructElem"):
            raise PdfValidationError("tagged vector StructElem differs")
        role = "Formula" if vector["kind"] in {"math_vector", "math_vector_block"} else "Figure"
        _name(dictionary.get("S"), role, "tagged vector StructElem /S")
        if _utf16_text(dictionary.get("Alt"), "tagged vector StructElem /Alt") != vector["alternative"]:
            raise PdfValidationError("tagged vector alternative differs")
        if "ActualText" in dictionary:
            raise PdfValidationError("ActualText must not be stored on vector StructElem")
        language = _utf16_text(dictionary["Lang"], "tagged vector StructElem /Lang") if "Lang" in dictionary else None
        if language != vector["structure_language"]:
            raise PdfValidationError("tagged vector structure language differs")
        kids = dictionary.get("K")
        if not isinstance(kids, list):
            raise PdfValidationError("tagged vector /K is not an array")
        _tagged_v2_mcr(kids[0] if kids else None, page_objects[vector["page_index"]], vector["mcid"], "tagged vector MCR")
        number = next((item for item in expectation["equation_numbers"] if item["parent_structure_node_id"] == node), None)
        if number is None:
            if len(kids) != 1:
                raise PdfValidationError("unnumbered vector has extra structure kids")
        elif len(kids) != 2 or _ref(kids[1], "Formula equation child") != roles[f"structure_element:{number['structure_node_id']}"]:
            raise PdfValidationError("Formula vector/number child order differs")

    for node, number in numbers.items():
        dictionary = objects[roles[f"structure_element:{node}"]].value
        if not isinstance(dictionary, dict) or dictionary.get("Type") != PdfName("StructElem"):
            raise PdfValidationError("equation-number StructElem differs")
        _name(dictionary.get("S"), "Span", "equation-number StructElem /S")
        if _ref(dictionary.get("P"), "equation-number /P") != roles[f"structure_element:{number['parent_structure_node_id']}"]:
            raise PdfValidationError("equation-number parent differs")
        if "Alt" in dictionary or "ActualText" in dictionary:
            raise PdfValidationError("equation-number StructElem duplicates replacement text")
        language = _utf16_text(dictionary["Lang"], "equation-number StructElem /Lang") if "Lang" in dictionary else None
        if language != number["structure_language"]:
            raise PdfValidationError("equation-number structure language differs")
        kids = dictionary.get("K")
        if not isinstance(kids, list) or len(kids) != 1:
            raise PdfValidationError("equation-number /K differs")
        _tagged_v2_mcr(kids[0], page_objects[number["page_index"]], number["mcid"], "equation-number MCR")

    parent = objects[roles["structure_parent_tree"]].value
    _exact_keys(parent, {"Nums"}, "tagged-PDF /2 ParentTree")
    nums = parent["Nums"]
    if not isinstance(nums, list) or len(nums) != 2 * expectation["page_count"]:
        raise PdfValidationError("tagged-PDF /2 ParentTree number tree differs")
    for page in range(expectation["page_count"]):
        if nums[2 * page] != page or not isinstance(nums[2 * page + 1], list):
            raise PdfValidationError("tagged-PDF /2 ParentTree key/value differs")
        records = [
            item for item in expectation["vectors"] + expectation["equation_numbers"]
            if item["page_index"] == page
        ]
        records.sort(key=lambda item: item["mcid"])
        observed = [_ref(item, "ParentTree value") for item in nums[2 * page + 1]]
        expected_objects = [roles[f"structure_element:{item['structure_node_id']}"] for item in records]
        if observed != expected_objects:
            raise PdfValidationError("tagged-PDF /2 ParentTree order differs")


def verify_tagged_pdf_structure_v2(pdf: bytes, expectation: dict[str, Any]) -> dict[str, Any]:
    """Independently decode the precomposed-vector tagged-PDF `/2` graph."""

    _tagged_v2_expectation_shape(expectation)
    objects, trailer = _parse_xref(pdf)
    roles = _tagged_v2_roles(expectation)
    pdf_fact = expectation["pdf"]
    if len(pdf) != pdf_fact["byte_length"] or hashlib.sha256(pdf).hexdigest() != pdf_fact["sha256"]:
        raise PdfValidationError("tagged-PDF /2 final byte closure differs")
    if sorted(objects) != list(range(1, pdf_fact["object_count"] + 1)):
        raise PdfValidationError("tagged-PDF /2 object allocation differs")
    for item in pdf_fact["objects"]:
        if hashlib.sha256(objects[item["object_number"]].raw).hexdigest() != item["sha256"]:
            raise PdfValidationError(f"tagged-PDF /2 object hash differs for {item['role']}")
    required_roles = {"catalog", "pages", "destinations", "info", "metadata", "structure_tree_root", "structure_parent_tree"}
    required_roles.update(f"page:{page}" for page in range(expectation["page_count"]))
    required_roles.update(f"page_content:{page}" for page in range(expectation["page_count"]))
    required_roles.update(f"structure_element:{item['structure_node_id']}" for item in expectation["vectors"] + expectation["equation_numbers"])
    if not required_roles.issubset(roles):
        raise PdfValidationError("tagged-PDF /2 required object roles are missing")
    if (
        roles["catalog"] != 1
        or _ref(trailer["Root"], "tagged-PDF /2 trailer /Root") != roles["catalog"]
        or _ref(trailer["Info"], "tagged-PDF /2 trailer /Info") != roles["info"]
    ):
        raise PdfValidationError("tagged-PDF /2 trailer/catalog role differs")

    catalog = objects[roles["catalog"]].value
    if not isinstance(catalog, dict) or catalog.get("Type") != PdfName("Catalog"):
        raise PdfValidationError("tagged-PDF /2 catalog differs")
    catalog_keys = {
        "Lang", "MarkInfo", "Metadata", "Names", "Pages", "StructTreeRoot", "Type",
        "ViewerPreferences",
    } | ({"Outlines"} if "outline_root" in roles else set())
    _exact_keys(catalog, catalog_keys, "tagged-PDF /2 catalog")
    names = catalog.get("Names")
    if not isinstance(names, dict):
        raise PdfValidationError("tagged-PDF /2 catalog Names differs")
    _exact_keys(names, {"Dests"}, "tagged-PDF /2 catalog Names")
    if (
        _utf16_text(catalog.get("Lang"), "tagged catalog /Lang") != expectation["document_language"]
        or catalog.get("MarkInfo") != {"Marked": True}
        or catalog.get("ViewerPreferences") != {"DisplayDocTitle": True}
        or _ref(catalog.get("Pages"), "catalog /Pages") != roles["pages"]
        or _ref(names.get("Dests"), "catalog /Names /Dests") != roles["destinations"]
        or _ref(catalog.get("Metadata"), "catalog /Metadata") != roles["metadata"]
        or _ref(catalog.get("StructTreeRoot"), "catalog /StructTreeRoot") != roles["structure_tree_root"]
    ):
        raise PdfValidationError("tagged-PDF /2 catalog accessibility state differs")
    if "outline_root" in roles and _ref(catalog.get("Outlines"), "catalog /Outlines") != roles["outline_root"]:
        raise PdfValidationError("tagged-PDF /2 catalog outline reference differs")

    destinations = objects[roles["destinations"]].value
    if not isinstance(destinations, dict):
        raise PdfValidationError("tagged-PDF /2 destination name tree differs")
    _exact_keys(destinations, {"Names"}, "tagged-PDF /2 destination name tree")
    if not isinstance(destinations["Names"], list):
        raise PdfValidationError("tagged-PDF /2 destination names are not an array")

    metadata = objects[roles["metadata"]]
    if not isinstance(metadata.value, dict) or metadata.stream is None:
        raise PdfValidationError("tagged-PDF /2 XMP metadata stream differs")
    _exact_keys(metadata.value, {"Length", "Subtype", "Type"}, "tagged-PDF /2 XMP metadata")
    if (
        metadata.value["Type"] != PdfName("Metadata")
        or metadata.value["Subtype"] != PdfName("XML")
        or hashlib.sha256(metadata.stream).hexdigest() != expectation["xmp_sha256"]
    ):
        raise PdfValidationError("tagged-PDF /2 XMP metadata hash/type differs")
    if not isinstance(objects[roles["info"]].value, dict):
        raise PdfValidationError("tagged-PDF /2 Info object differs")

    pages_root = objects[roles["pages"]].value
    if not isinstance(pages_root, dict) or pages_root.get("Type") != PdfName("Pages"):
        raise PdfValidationError("tagged-PDF /2 page tree differs")
    _exact_keys(pages_root, {"Count", "Kids", "Type"}, "tagged-PDF /2 page tree")
    page_objects = {page: roles[f"page:{page}"] for page in range(expectation["page_count"])}
    if pages_root.get("Count") != expectation["page_count"] or pages_root.get("Kids") != [PdfRef(page_objects[page]) for page in range(expectation["page_count"])]:
        raise PdfValidationError("tagged-PDF /2 page tree order differs")
    _tagged_v2_outline_graph(objects, roles, page_objects)

    form_role_items = [
        item for item in pdf_fact["objects"] if item["role"].startswith("vector_form:")
    ]
    form_role_items.sort(key=lambda item: int(item["role"].split(":", 1)[1]))
    form_objects = [item["object_number"] for item in form_role_items]
    if len(form_objects) != expectation["form_count"]:
        raise PdfValidationError("tagged-PDF /2 Form count differs")
    for number in form_objects:
        form = objects[number]
        if not isinstance(form.value, dict) or form.value.get("Subtype") != PdfName("Form") or form.value.get("Type") != PdfName("XObject") or form.stream is None:
            raise PdfValidationError("tagged-PDF /2 vector object is not a Form")
        if any(token in form.stream for token in (b"/MCID", b"/Alt", b"/ActualText", b"/Lang", b"BDC", b"BMC", b"EMC")):
            raise PdfValidationError("tagged-PDF /2 reusable Form contains semantic state")
    if any(isinstance(item.value, dict) and item.value.get("Subtype") == PdfName("Image") for item in objects.values()):
        raise PdfValidationError("tagged-PDF /2 vector closure contains raster image content")
    equation_fonts = _tagged_v2_equation_fonts(
        objects,
        roles,
        expectation["equation_numbers"],
    )

    total_do = 0
    extracted: list[str] = []
    for page in range(expectation["page_count"]):
        dictionary = objects[page_objects[page]].value
        if not isinstance(dictionary, dict) or dictionary.get("Type") != PdfName("Page"):
            raise PdfValidationError("tagged-PDF /2 page dictionary differs")
        if (
            _ref(dictionary.get("Parent"), "page /Parent") != roles["pages"]
            or dictionary.get("StructParents") != page
            or _ref(dictionary.get("Contents"), "page /Contents") != roles[f"page_content:{page}"]
        ):
            raise PdfValidationError("tagged-PDF /2 page StructParents/content differs")
        resources = dictionary.get("Resources")
        if not isinstance(resources, dict):
            raise PdfValidationError("tagged-PDF /2 page resources differ")
        expected_font_indices = sorted(
            {
                number["font_index"]
                for number in expectation["equation_numbers"]
                if number["page_index"] == page
            }
        )
        observed_fonts = resources.get("Font", {})
        if not isinstance(observed_fonts, dict) or set(observed_fonts) != {
            f"F{index}" for index in expected_font_indices
        }:
            raise PdfValidationError("tagged-PDF /2 page equation font resources differ")
        for index in expected_font_indices:
            if observed_fonts[f"F{index}"] != PdfRef(equation_fonts[index][0]):
                raise PdfValidationError("tagged-PDF /2 page equation font reference differs")
        stream = objects[roles[f"page_content:{page}"]].stream
        if stream is None:
            raise PdfValidationError("tagged-PDF /2 page content is not a stream")
        records = [
            ("vector", item) for item in expectation["vectors"] if item["page_index"] == page
        ] + [
            ("equation_number", item) for item in expectation["equation_numbers"] if item["page_index"] == page
        ]
        records.sort(key=lambda item: item[1]["mcid"])
        do_count, page_text = _tagged_v2_content(
            stream,
            page,
            records,
            dictionary,
            form_objects,
            equation_fonts,
        )
        total_do += do_count
        extracted.extend(page_text)
    if total_do != len(expectation["vectors"]):
        raise PdfValidationError("tagged-PDF /2 shared Form usage count differs")
    _tagged_v2_structure(objects, roles, expectation, page_objects)
    expected_text = [
        item["actual_text"] if kind == "vector" else item["exact_text"]
        for page in range(expectation["page_count"])
        for kind, item in sorted(
            [
                ("vector", value) for value in expectation["vectors"]
                if value["page_index"] == page and value["actual_text"] is not None
            ] + [
                ("equation_number", value) for value in expectation["equation_numbers"]
                if value["page_index"] == page
            ],
            key=lambda value: value[1]["mcid"],
        )
    ]
    if extracted != expected_text:
        raise PdfValidationError("tagged-PDF /2 extraction order differs")
    return {
        "algorithm": "typaxis.tagged-pdf-validator/2",
        "document_language": expectation["document_language"],
        "equation_number_count": len(expectation["equation_numbers"]),
        "extracted_text": extracted,
        "form_count": len(form_objects),
        "form_do_count": total_do,
        "object_count": len(objects),
        "page_count": expectation["page_count"],
        "pdf_sha256": hashlib.sha256(pdf).hexdigest(),
        "vector_count": len(expectation["vectors"]),
    }


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
        if expectation.get("algorithm") == "typaxis.tagged-pdf-validator/2":
            observation = verify_tagged_pdf_structure_v2(pdf, expectation)
        elif expectation.get("algorithm") == "typaxis.tagged-pdf-manifest/1":
            observation = verify_tagged_pdf_structure(pdf, expectation)
        else:
            observation = verify_pdf_structure(pdf, expectation)
    except (OSError, PdfValidationError) as error:
        print(f"PDF structure validation failed: {error}", file=sys.stderr)
        return 1
    print(json.dumps(observation, ensure_ascii=False, separators=(",", ":"), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
