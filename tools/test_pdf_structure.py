#!/usr/bin/env python3
"""Tests for the independent MI4-07 PDF structure validator."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import io
import json
import tempfile
import unittest
from collections.abc import Callable
from pathlib import Path

from tools import verify_pdf_structure as verifier


ROOT = Path(__file__).resolve().parents[1]
EXPECTATION_PATH = (
    ROOT
    / "samples"
    / "machine-package"
    / "staging"
    / "production-book-1"
    / "book-navigation"
    / "pdf-expectation.json"
)
TAGGED_MANIFEST_PATH = (
    ROOT
    / "samples"
    / "machine-package"
    / "staging"
    / "production-book-1"
    / "accessibility"
    / "manifest.json"
)
TAGGED_PDF_PATH = TAGGED_MANIFEST_PATH.with_name("output.pdf")
PRODUCTION_FIXTURE_PATH = (
    ROOT
    / "samples"
    / "machine-package"
    / "profiles"
    / "production-book-1"
    / "combined"
)


def utf16_hex(value: str) -> str:
    return (b"\xfe\xff" + value.encode("utf-16-be")).hex().upper()


def literal(value: str) -> str:
    return "(" + value.replace("\\", "\\\\").replace("(", "\\(").replace(")", "\\)") + ")"


def stream(prefix: str, content: bytes) -> bytes:
    return (
        f"<< {prefix}/Length {len(content)} >>\nstream\n".encode("ascii")
        + content
        + b"\nendstream"
    )


def pdf_number(raw: int) -> str:
    if raw % 65_536:
        return str(raw / 65_536)
    return str(raw // 65_536)


def build_objects(expectation: dict) -> tuple[dict[int, bytes], int]:
    pages = expectation["pages"]
    links = expectation["links"]
    outline = expectation["outline"]
    annotation_start = 4 + 2 * len(pages)
    info_object = annotation_start + len(links)
    metadata_object = info_object + 1
    outline_root = metadata_object + 1 if outline else None
    item_start = outline_root + 1 if outline_root is not None else None
    objects: dict[int, bytes] = {}

    catalog = (
        f"<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> "
        f"/Lang <{utf16_hex(expectation['document_language'])}> "
        f"/Metadata {metadata_object} 0 R"
    )
    if outline_root is not None:
        catalog += f" /Outlines {outline_root} 0 R"
    objects[1] = (catalog + " >>").encode("ascii")
    page_objects = [5 + 2 * index for index in range(len(pages))]
    objects[2] = (
        f"<< /Type /Pages /Count {len(pages)} /Kids ["
        + "".join(f"{number} 0 R " for number in page_objects)
        + "] >>"
    ).encode("ascii")
    names = "<< /Names ["
    for destination in expectation["destinations"]:
        page_object = page_objects[destination["page_index"]]
        view = destination["view"]
        if view["kind"] == "xyz":
            projection = f"/XYZ {pdf_number(view['x'])} {pdf_number(view['y'])} null"
        elif view["kind"] == "fit_page":
            projection = "/Fit"
        else:
            top = "null" if view["top"] is None else pdf_number(view["top"])
            projection = f"/FitH {top}"
        names += f"{literal(destination['name'])} [{page_object} 0 R {projection}] "
    objects[3] = (names + "] >>").encode("ascii")

    for page in pages:
        page_index = page["page_index"]
        content_object = 4 + page_index * 2
        page_object = content_object + 1
        content = bytearray()
        for paint in expectation["language_paints"]:
            if paint["page_index"] != page_index:
                continue
            properties = "<<"
            if paint["actual_text"] is not None:
                properties += f" /ActualText <{utf16_hex(paint['actual_text'])}>"
            properties += f" /Lang <{utf16_hex(paint['language'])}> >>"
            content.extend(f"/Span {properties} BDC\n0 0 m 0 0 l S\nEMC\n".encode("ascii"))
        objects[content_object] = stream("", bytes(content))
        page_links = [
            annotation_start + index
            for index, link in enumerate(links)
            if link["page_index"] == page_index
        ]
        value = (
            f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 "
            f"{pdf_number(page['width'])} {pdf_number(page['height'])}] "
            f"/Resources << >> /Contents {content_object} 0 R"
        )
        if page_links:
            value += " /Annots [" + "".join(f"{number} 0 R " for number in page_links) + "]"
        objects[page_object] = (value + " >>").encode("ascii")

    for index, link in enumerate(links):
        rectangle = " ".join(pdf_number(value) for value in link["rect"])
        objects[annotation_start + index] = (
            f"<< /Type /Annot /Subtype /Link /Rect [{rectangle}] "
            f"/Border [0 0 0] /Dest {literal(link['destination'])} >>"
        ).encode("ascii")

    metadata = expectation["metadata"]
    fields: list[str] = []
    if metadata["author"] is not None:
        fields.append(f"/Author <{utf16_hex(metadata['author'])}>")
    if metadata["created"] is not None:
        fields.append(f"/CreationDate ({verifier._pdf_date(metadata['created'])})")
    if metadata["keywords"]:
        fields.append(f"/Keywords <{utf16_hex('; '.join(metadata['keywords']))}>")
    if metadata["modified"] is not None:
        fields.append(f"/ModDate ({verifier._pdf_date(metadata['modified'])})")
    producer = f"{expectation['engine']['name']} {expectation['engine']['version']}"
    fields.append(f"/Producer <{utf16_hex(producer)}>")
    if metadata["subject"] is not None:
        fields.append(f"/Subject <{utf16_hex(metadata['subject'])}>")
    if metadata["title"] is not None:
        fields.append(f"/Title <{utf16_hex(metadata['title'])}>")
    objects[info_object] = ("<< " + " ".join(fields) + " >>").encode("ascii")
    objects[metadata_object] = stream(
        "/Type /Metadata /Subtype /XML ", verifier.expected_xmp(expectation)
    )

    if outline_root is not None and item_start is not None:
        children: dict[int | None, list[int]] = {}
        sibling_positions: dict[int, int] = {}
        for entry in outline:
            siblings = children.setdefault(entry["parent_outline_id"], [])
            sibling_positions[entry["outline_id"]] = len(siblings)
            siblings.append(entry["outline_id"])
        top = children[None]
        objects[outline_root] = (
            f"<< /Type /Outlines /First {item_start + top[0]} 0 R "
            f"/Last {item_start + top[-1]} 0 R /Count {len(outline)} >>"
        ).encode("ascii")
        for index, entry in enumerate(outline):
            siblings = children[entry["parent_outline_id"]]
            sibling_index = sibling_positions[index]
            direct_children = children.get(index, [])
            parent = outline_root if entry["parent_outline_id"] is None else item_start + entry["parent_outline_id"]
            value = (
                f"<< /Title <{utf16_hex(entry['label'])}> /Parent {parent} 0 R "
                f"/Dest {literal(entry['destination'])}"
            )
            if sibling_index:
                value += f" /Prev {item_start + siblings[sibling_index - 1]} 0 R"
            if sibling_index + 1 < len(siblings):
                value += f" /Next {item_start + siblings[sibling_index + 1]} 0 R"
            if direct_children:
                descendants = verifier._outline_descendant_count(outline, index)
                value += (
                    f" /First {item_start + direct_children[0]} 0 R"
                    f" /Last {item_start + direct_children[-1]} 0 R /Count {descendants}"
                )
            objects[item_start + index] = (value + " >>").encode("ascii")
    return objects, info_object


def serialize(objects: dict[int, bytes], info_object: int) -> bytes:
    output = bytearray(b"%PDF-1.7\n%\xe2\xe3\xcf\xd3\n")
    offsets = [0]
    for number in range(1, len(objects) + 1):
        offsets.append(len(output))
        output.extend(f"{number} 0 obj\n".encode("ascii"))
        output.extend(objects[number])
        output.extend(b"\nendobj\n")
    xref = len(output)
    output.extend(f"xref\n0 {len(objects) + 1}\n".encode("ascii"))
    output.extend(b"0000000000 65535 f \n")
    for offset in offsets[1:]:
        output.extend(f"{offset:010} 00000 n \n".encode("ascii"))
    output.extend(
        (
            f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R /Info {info_object} 0 R >>\n"
            f"startxref\n{xref}\n%%EOF\n"
        ).encode("ascii")
    )
    return bytes(output)


def build_tagged_v2() -> tuple[dict[int, bytes], dict, bytes]:
    ja = utf16_hex("ja")
    en = utf16_hex("en-US")
    alt_inline = utf16_hex("丸括弧で囲んだ二項目")
    alt_math = utf16_hex("xたすy")
    alt_figure = utf16_hex("配置図")
    alt_block = utf16_hex("xたすy、式1")
    number_text = utf16_hex("(1)")
    font_program = b"\x00\x01\x00\x00typaxis-test-font"
    to_unicode = (
        b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n"
        b"/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n"
        b"/CMapName /Typaxis-Identity-UCS def\n/CMapType 2 def\n"
        b"1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
        b"3 beginbfchar\n<0001> <0028>\n<0002> <0031>\n<0003> <0029>\n"
        b"endbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
    )
    objects: dict[int, bytes] = {
        1: (
            f"<< /Type /Catalog /Pages 2 0 R /Names << /Dests 3 0 R >> /Lang <{ja}> "
            "/Metadata 15 0 R /MarkInfo << /Marked true >> "
            "/ViewerPreferences << /DisplayDocTitle true >> /StructTreeRoot 18 0 R >>"
        ).encode("ascii"),
        2: b"<< /Type /Pages /Count 2 /Kids [5 0 R 7 0 R ] >>",
        3: b"<< /Names [] >>",
        4: stream(
            "",
            (
                "q\n1 0 0 -1 0 800 cm\n"
                f"/Figure << /MCID 0 >> BDC\n/Span << /Lang <{en}> >> BDC\n"
                "q\n0 0 0 rg\n0 0 0 RG\n1 0 0 1 10 10 cm\n/V0 Do\nQ\nEMC\nEMC\n"
                f"/Formula << /MCID 1 >> BDC\n/Span << /ActualText <{alt_math}> /Lang <{en}> >> BDC\n"
                "q\n0 0 0 rg\n0 0 0 RG\n1 0 0 1 40 10 cm\n/V0 Do\nQ\nEMC\nEMC\nQ"
            ).encode("ascii"),
        ),
        5: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1000 800] /Resources << /XObject << /V0 16 0 R >> >> /Contents 4 0 R /StructParents 0 >>",
        6: stream(
            "",
            (
                "q\n1 0 0 -1 0 800 cm\n"
                f"/Figure << /MCID 0 >> BDC\n/Span << /Lang <{en}> >> BDC\n"
                "q\n0 0 0 rg\n0 0 0 RG\n1 0 0 1 10 10 cm\n/V0 Do\nQ\nEMC\nEMC\n"
                f"/Formula << /MCID 1 >> BDC\n/Span << /ActualText <{alt_block}> /Lang <{en}> >> BDC\n"
                "q\n0 0 0 rg\n0 0 0 RG\n1 0 0 1 40 10 cm\n/V0 Do\nQ\nEMC\nEMC\n"
                f"/Span << /MCID 2 /Lang <{en}> >> BDC\n"
                f"/Span << /ActualText <{number_text}> >> BDC\n"
                "0 g\nBT /F0 8 Tf 0 Tr\n"
                "1 0 0 -1 90 22 Tm <0001> Tj\n"
                "1 0 0 -1 94 22 Tm <0002> Tj\n"
                "1 0 0 -1 98 22 Tm <0003> Tj\n"
                "ET\nEMC\nEMC\nQ"
            ).encode("ascii"),
        ),
        7: b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 1000 800] /Resources << /XObject << /V0 16 0 R >> /Font << /F0 8 0 R >> >> /Contents 6 0 R /StructParents 1 >>",
        8: b"<< /Type /Font /Subtype /Type0 /BaseFont /ABCDEF+TestEquation /Encoding /Identity-H /DescendantFonts [9 0 R] /ToUnicode 12 0 R >>",
        9: b"<< /Type /Font /Subtype /CIDFontType2 /BaseFont /ABCDEF+TestEquation /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor 10 0 R /DW 1000 /W [1 [500] 2 [500] 3 [500] ] /CIDToGIDMap 13 0 R >>",
        10: b"<< /Type /FontDescriptor /FontName /ABCDEF+TestEquation /Flags 32 /FontBBox [0 -200 1000 800] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /FontFile2 11 0 R >>",
        11: stream(f"/Length1 {len(font_program)} ", font_program),
        12: stream("", to_unicode),
        13: stream("", b"\x00\x00\x00\x01\x00\x02\x00\x03"),
        14: b"<< /Producer <FEFF0054007900700061007800690073> >>",
        15: stream("/Type /Metadata /Subtype /XML ", b"<x:xmpmeta/>"),
        16: stream(
            "/Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 30 12] "
            "/Resources << /ExtGState << /GS0 17 0 R >> >> ",
            b"q\n0 0 30 12 re W n\nq\n/GS0 gs\n2 2 m\n10 10 l\n1 w\n0 J\n0 j\n10 M\nS\nQ\nQ",
        ),
        17: b"<< /Type /ExtGState /ca 1 /CA 1 >>",
        18: b"<< /Type /StructTreeRoot /RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >> /ParentTree 19 0 R /ParentTreeNextKey 2 /K [20 0 R ] >>",
        19: b"<< /Nums [0 [23 0 R 24 0 R ] 1 [25 0 R 26 0 R 27 0 R ] ] >>",
        20: b"<< /Type /StructElem /S /Document /P 18 0 R /K [21 0 R ] >>",
        21: b"<< /Type /StructElem /S /Result /P 20 0 R /K [22 0 R 25 0 R 26 0 R ] >>",
        22: b"<< /Type /StructElem /S /P /P 21 0 R /K [23 0 R 24 0 R ] >>",
        23: f"<< /Type /StructElem /S /Figure /P 22 0 R /Lang <{en}> /Alt <{alt_inline}> /K [<< /Type /MCR /Pg 5 0 R /MCID 0 >> ] >>".encode("ascii"),
        24: f"<< /Type /StructElem /S /Formula /P 22 0 R /Lang <{en}> /Alt <{alt_math}> /K [<< /Type /MCR /Pg 5 0 R /MCID 1 >> ] >>".encode("ascii"),
        25: f"<< /Type /StructElem /S /Figure /P 21 0 R /Lang <{en}> /Alt <{alt_figure}> /K [<< /Type /MCR /Pg 7 0 R /MCID 0 >> ] >>".encode("ascii"),
        26: f"<< /Type /StructElem /S /Formula /P 21 0 R /Lang <{en}> /Alt <{alt_block}> /K [<< /Type /MCR /Pg 7 0 R /MCID 1 >> 27 0 R ] >>".encode("ascii"),
        27: b"<< /Type /StructElem /S /Span /P 26 0 R /K [<< /Type /MCR /Pg 7 0 R /MCID 2 >> ] >>",
    }
    roles = [
        "catalog", "pages", "destinations", "page_content:0", "page:0",
        "page_content:1", "page:1", "equation_font_type0:0", "equation_font_cid:0",
        "equation_font_descriptor:0", "equation_font_program:0",
        "equation_font_to_unicode:0", "equation_font_cid_to_gid:0", "info", "metadata",
        "vector_form:0", "vector_ext_g_state:1", "structure_tree_root",
        "structure_parent_tree", "structure_element:0", "structure_element:1",
        "structure_element:2", "structure_element:3", "structure_element:4",
        "structure_element:5", "structure_element:6", "structure_element:7",
    ]
    pdf = serialize(objects, 14)
    expectation = {
        "algorithm": "typaxis.tagged-pdf-validator/2",
        "document_language": "ja",
        "equation_numbers": [
            {
                "exact_text": "(1)",
                "font_index": 0,
                "mcid": 2,
                "page_index": 1,
                "paint_language": "en-US",
                "parent_structure_node_id": 6,
                "structure_language": None,
                "structure_node_id": 7,
            }
        ],
        "form_count": 1,
        "object_budget_charge_count": 1,
        "observation_algorithm": "typaxis.tagged-pdf-observation/2",
        "page_count": 2,
        "xmp_sha256": hashlib.sha256(b"<x:xmpmeta/>").hexdigest(),
        "pdf": {
            "byte_length": len(pdf),
            "object_count": len(objects),
            "objects": [
                {
                    "object_number": number,
                    "role": roles[number - 1],
                    "sha256": hashlib.sha256(objects[number]).hexdigest(),
                }
                for number in range(1, len(objects) + 1)
            ],
            "sha256": hashlib.sha256(pdf).hexdigest(),
        },
        "vectors": [
            {
                "actual_text": None,
                "alternative": "丸括弧で囲んだ二項目",
                "form_index": 0,
                "kind": "inline_vector",
                "mcid": 0,
                "page_index": 0,
                "paint_language": "en-US",
                "structure_language": "en-US",
                "structure_node_id": 3,
            },
            {
                "actual_text": "xたすy",
                "alternative": "xたすy",
                "form_index": 0,
                "kind": "math_vector",
                "mcid": 1,
                "page_index": 0,
                "paint_language": "en-US",
                "structure_language": "en-US",
                "structure_node_id": 4,
            },
            {
                "actual_text": None,
                "alternative": "配置図",
                "form_index": 0,
                "kind": "vector_figure",
                "mcid": 0,
                "page_index": 1,
                "paint_language": "en-US",
                "structure_language": "en-US",
                "structure_node_id": 5,
            },
            {
                "actual_text": "xたすy、式1",
                "alternative": "xたすy、式1",
                "form_index": 0,
                "kind": "math_vector_block",
                "mcid": 1,
                "page_index": 1,
                "paint_language": "en-US",
                "structure_language": "en-US",
                "structure_node_id": 6,
            },
        ],
    }
    return objects, expectation, pdf


class PdfStructureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.expectation = verifier.load_expectation(EXPECTATION_PATH)

    def build(self) -> tuple[dict[int, bytes], int, bytes]:
        objects, info = build_objects(self.expectation)
        return objects, info, serialize(objects, info)

    def test_valid_metadata_language_outline_and_link(self) -> None:
        _, _, pdf = self.build()
        observation = verifier.verify_pdf_structure(pdf, self.expectation)
        self.assertEqual(observation["catalog_language"], "en-US")
        self.assertEqual(len(observation["outline"]), 3)
        self.assertEqual(observation["links"][0]["destination"], "chapter-1")
        self.assertEqual(
            observation["links"][0]["rect"],
            self.expectation["links"][0]["rect"],
        )
        self.assertEqual(
            observation["xmp_sha256"],
            verifier.hashlib.sha256(verifier.expected_xmp(self.expectation)).hexdigest(),
        )

    def test_empty_outline_omits_outline_objects_and_catalog_entry(self) -> None:
        expectation = copy.deepcopy(self.expectation)
        expectation["outline"] = []
        objects, info = build_objects(expectation)
        observation = verifier.verify_pdf_structure(
            serialize(objects, info), expectation
        )
        self.assertEqual(observation["outline"], [])
        self.assertNotIn(b"/Outlines", objects[1])

    def test_descendant_count_stops_at_the_next_root(self) -> None:
        expectation = copy.deepcopy(self.expectation)
        expectation["destinations"].append(
            {
                "name": "root-2-child",
                "page_index": 1,
                "view": {"kind": "fit_page"},
            }
        )
        expectation["destinations"].sort(key=lambda item: item["name"].encode("ascii"))
        expectation["outline"] = [
            {
                "destination": "part-1",
                "label": "Root 1",
                "level": 1,
                "outline_id": 0,
                "parent_outline_id": None,
                "source_node_id": 1,
            },
            {
                "destination": "chapter-1",
                "label": "Root 1 child",
                "level": 2,
                "outline_id": 1,
                "parent_outline_id": 0,
                "source_node_id": 2,
            },
            {
                "destination": "exercise-1",
                "label": "Root 2",
                "level": 1,
                "outline_id": 2,
                "parent_outline_id": None,
                "source_node_id": 7,
            },
            {
                "destination": "root-2-child",
                "label": "Root 2 child",
                "level": 2,
                "outline_id": 3,
                "parent_outline_id": 2,
                "source_node_id": 8,
            },
        ]
        objects, info = build_objects(expectation)
        outline_root = info + 2
        first_root = outline_root + 1
        self.assertIn(b"/Count 1", objects[first_root])
        verifier.verify_pdf_structure(serialize(objects, info), expectation)

        objects[first_root] = objects[first_root].replace(b"/Count 1", b"/Count 2")
        with self.assertRaisesRegex(verifier.PdfValidationError, "outline child"):
            verifier.verify_pdf_structure(serialize(objects, info), expectation)

    def test_catalog_language_tamper_is_rejected(self) -> None:
        _, _, pdf = self.build()
        original = utf16_hex("en-US").encode("ascii")
        tampered = utf16_hex("ja-JP").encode("ascii")
        pdf = pdf.replace(original, tampered, 1)
        with self.assertRaisesRegex(verifier.PdfValidationError, "catalog /Lang"):
            verifier.verify_pdf_structure(pdf, self.expectation)

        objects, info = build_objects(self.expectation)
        objects[1] = objects[1].replace(
            f"<{utf16_hex('en-US')}>".encode("ascii"), b"(en-US)", 1
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "UTF-16BE"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

    def test_link_action_or_missing_destination_is_rejected(self) -> None:
        objects, info, _ = self.build()
        annotation = 4 + 2 * len(self.expectation["pages"])
        objects[annotation] = objects[annotation].replace(
            b"/Dest (chapter-1)", b"/Dest (missing-1)"
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "link destination"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

        objects, info = build_objects(self.expectation)
        objects[annotation] = objects[annotation].replace(
            b"/Dest (chapter-1)", b"/A << /S /GoTo /D (chapter-1) >>"
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "link 0 keys"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

        objects, info = build_objects(self.expectation)
        objects[annotation] = objects[annotation].replace(
            b"/Rect [100 130 160 150]", b"/Rect [101 130 160 150]"
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "link 0 rectangle"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

    def test_outline_hierarchy_and_xmp_tamper_are_rejected(self) -> None:
        objects, info, _ = self.build()
        outline_root = info + 2
        second_item = outline_root + 2
        objects[second_item] = objects[second_item].replace(
            f"/Parent {outline_root + 1} 0 R".encode("ascii"),
            f"/Parent {outline_root} 0 R".encode("ascii"),
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "outline parent"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

        objects, info = build_objects(self.expectation)
        metadata_object = info + 1
        objects[metadata_object] = objects[metadata_object].replace(
            b"Typaxis Book", b"Typaxis B00k", 1
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "XMP bytes"):
            verifier.verify_pdf_structure(serialize(objects, info), self.expectation)

    def test_command_line_writes_canonical_observation(self) -> None:
        _, _, pdf = self.build()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "book.pdf"
            path.write_bytes(pdf)
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = verifier.main([str(path), str(EXPECTATION_PATH)])
        self.assertEqual(status, 0)
        decoded = json.loads(output.getvalue())
        self.assertEqual(
            output.getvalue().rstrip("\n"),
            json.dumps(decoded, ensure_ascii=False, separators=(",", ":"), sort_keys=True),
        )

    def test_malformed_expectation_and_footer_are_rejected(self) -> None:
        expectation = copy.deepcopy(self.expectation)
        expectation["links"][0]["rect"] = [100, 200, 100, 300]
        with self.assertRaisesRegex(verifier.PdfValidationError, "rect is empty"):
            verifier.verify_pdf_structure(self.build()[2], expectation)

        _, _, pdf = self.build()
        pdf = pdf.replace(b"\n%%EOF\n", b"\nignored\n%%EOF\n")
        with self.assertRaisesRegex(verifier.PdfValidationError, "between startxref"):
            verifier.verify_pdf_structure(pdf, self.expectation)

    def test_expectation_requires_canonical_language_and_outline_preorder(self) -> None:
        _, _, pdf = self.build()
        noncanonical_language = copy.deepcopy(self.expectation)
        noncanonical_language["document_language"] = "EN-us"
        with self.assertRaisesRegex(verifier.PdfValidationError, "not canonical"):
            verifier.verify_pdf_structure(pdf, noncanonical_language)

        wrong_source_order = copy.deepcopy(self.expectation)
        wrong_source_order["outline"][1]["source_node_id"] = 8
        wrong_source_order["outline"][2]["source_node_id"] = 7
        with self.assertRaisesRegex(verifier.PdfValidationError, "strict preorder"):
            verifier.verify_pdf_structure(pdf, wrong_source_order)

        duplicate_destination = copy.deepcopy(self.expectation)
        duplicate_destination["outline"][2]["destination"] = "chapter-1"
        with self.assertRaisesRegex(verifier.PdfValidationError, "not unique"):
            verifier.verify_pdf_structure(pdf, duplicate_destination)

        whitespace_title = copy.deepcopy(self.expectation)
        whitespace_title["metadata"]["title"] = "\u3000"
        with self.assertRaisesRegex(verifier.PdfValidationError, "metadata string"):
            verifier.verify_pdf_structure(pdf, whitespace_title)


class TaggedPdfStructureTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.manifest = verifier.load_expectation(TAGGED_MANIFEST_PATH)
        cls.pdf = TAGGED_PDF_PATH.read_bytes()

    @staticmethod
    def rebind_pdf_hashes(pdf: bytes, source: dict) -> dict:
        manifest = copy.deepcopy(source)
        objects, _ = verifier._parse_xref(pdf)
        manifest["fingerprints"]["pdf_sha256"] = hashlib.sha256(pdf).hexdigest()
        manifest["pdf"]["byte_length"] = len(pdf)
        for item in manifest["pdf"]["objects"]:
            item["sha256"] = hashlib.sha256(objects[item["object_number"]].raw).hexdigest()
        return manifest

    def test_tagged_structure_reading_order_language_alt_and_actual_text(self) -> None:
        observation = verifier.verify_tagged_pdf_structure(self.pdf, self.manifest)
        self.assertEqual(observation["catalog_language"], "en-US")
        self.assertEqual(observation["reading_order"], list(range(87)))
        self.assertEqual(
            set(observation["roles"]),
            {
                "Caption", "Document", "Em", "Exercise", "Figure", "Formula",
                "H1", "H2", "H3", "H4", "H5", "H6", "L", "LBody", "LI",
                "Lbl", "Link", "Note", "P", "Proof", "Reference", "Result",
                "Span", "Strong", "TBody", "TD", "TH", "THead", "TR", "Table",
            },
        )
        self.assertIn({"role": "Figure", "text": "PNG image"}, observation["alternatives"])
        self.assertEqual(
            observation["actual_text"],
            [
                "Basic document", "Basic document", "page top", "internal ", "1", "1",
                "Accessible footnote", "1.", "First item", "2.", "Second entry",
                "PNG caption", "Header A", "Header B", "alpha", "beta", "gamma", "delta",
                "x squared", "x plus one", "Heading level 2", "Heading level 3",
                "Heading level 4", "Heading level 5", "Heading level 6", "emphasized",
                "strong", "•", "Unordered item", "Accessible result", "Accessible proof",
                "Accessible exercise",
            ],
        )
        self.assertEqual(
            [
                record["actual_text"]
                for page in observation["marked_pages"]
                for record in page
                if record.get("actual_text") is not None
            ],
            [
                "Basic document", "page top", "1.", "Second entry", "PNG caption",
                "Header A", "alpha", "x plus one", "Heading level 2", "Heading level 3",
                "Heading level 4", "Heading level 5", "Heading level 6", "•",
                "Accessible result", "Accessible exercise", "Basic document", "internal ",
                "1", "1", "Accessible footnote", "First item", "2.", "Header B", "beta",
                "gamma", "delta", "x squared", "emphasized", "strong", "Unordered item",
                "Accessible proof",
            ],
        )
        self.assertEqual(observation["artifact_count"], 4)
        self.assertEqual(observation["outline_structure"], [1, 84])

    def test_tagged_missing_extra_owner_order_page_and_mcid_tamper(self) -> None:
        mutations: list[tuple[str, Callable[[dict], object]]] = [
            (
                "missing",
                lambda value: value["marked_content"]["records"].pop(),
            ),
            (
                "extra",
                lambda value: value["marked_content"]["records"].append(
                    {
                        **copy.deepcopy(value["marked_content"]["records"][-1]),
                        "selected_paint_ids": [
                            sum(
                                len(record["selected_paint_ids"])
                                for record in value["marked_content"]["records"]
                            )
                        ],
                        "paint_ordinal_start": sum(
                            len(record["selected_paint_ids"])
                            for record in value["marked_content"]["records"]
                            if record["page_index"] == 1
                        ),
                    }
                ),
            ),
            (
                "owner",
                lambda value: value["marked_content"]["records"][0]["owner"].update(
                    {"structure_node_id": 1, "role": "H1"}
                ),
            ),
            (
                "order",
                lambda value: value["marked_content"]["records"].__setitem__(
                    slice(0, 2), list(reversed(value["marked_content"]["records"][0:2]))
                ),
            ),
            (
                "page",
                lambda value: value["marked_content"]["records"][0].update(
                    {"page_index": 1}
                ),
            ),
            (
                "MCID",
                lambda value: value["marked_content"]["records"][0]["owner"].update(
                    {"mcid": 9}
                ),
            ),
        ]
        for label, mutate in mutations:
            with self.subTest(label=label):
                manifest = copy.deepcopy(self.manifest)
                mutate(manifest)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure(self.pdf, manifest)

    def test_tagged_role_alternative_and_language_tamper(self) -> None:
        role = copy.deepcopy(self.manifest)
        role["structure"][1]["role"] = "H2"
        with self.assertRaisesRegex(verifier.PdfValidationError, "structure element 1 /S"):
            verifier.verify_tagged_pdf_structure(self.pdf, role)

        alternative = copy.deepcopy(self.manifest)
        alternative["structure"][25]["alternative"] = "wrong image"
        with self.assertRaisesRegex(verifier.PdfValidationError, "alternative"):
            verifier.verify_tagged_pdf_structure(self.pdf, alternative)

        language = copy.deepcopy(self.manifest)
        language["structure"][55]["language"] = "ja-JP"
        with self.assertRaises(verifier.PdfValidationError):
            verifier.verify_tagged_pdf_structure(self.pdf, language)

    def test_tagged_pdf_bytes_parent_tree_and_objr_tamper(self) -> None:
        for original, replacement in [
            (b"/ParentTree 15 0 R", b"/ParentTree 16 0 R"),
            (b"/Type /OBJR", b"/Type /MCR "),
            (b"/StructParent 2", b"/StructParent 9"),
        ]:
            with self.subTest(original=original):
                tampered = self.pdf.replace(original, replacement, 1)
                manifest = self.rebind_pdf_hashes(tampered, self.manifest)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure(tampered, manifest)

    def test_tagged_header_relation_wrapper_outline_and_xmp_tamper(self) -> None:
        header = copy.deepcopy(self.manifest)
        data_cell = next(node for node in header["structure"] if node["role"] == "TD")
        data_cell["table"]["header_ids"] = []
        with self.assertRaisesRegex(verifier.PdfValidationError, "header"):
            verifier.verify_tagged_pdf_structure(self.pdf, header)

        relation = copy.deepcopy(self.manifest)
        note = next(node for node in relation["structure"] if node["role"] == "Note")
        note["related_nodes"] = []
        with self.assertRaisesRegex(verifier.PdfValidationError, "relation"):
            verifier.verify_tagged_pdf_structure(self.pdf, relation)

        wrapper = copy.deepcopy(self.manifest)
        generated = next(
            node for node in wrapper["structure"] if node["owner"]["kind"] == "generated"
        )
        generated["source_span"]["source_id"] += 1
        with self.assertRaisesRegex(verifier.PdfValidationError, "owner/span"):
            verifier.verify_tagged_pdf_structure(self.pdf, wrapper)

        numbering = copy.deepcopy(self.manifest)
        list_node = next(node for node in numbering["structure"] if node["role"] == "L")
        list_node["list_numbering"] = (
            "disc" if list_node["list_numbering"] == "decimal" else "decimal"
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "List"):
            verifier.verify_tagged_pdf_structure(self.pdf, numbering)

        outline = copy.deepcopy(self.manifest)
        outline["outline"][1]["parent_outline_id"] = None
        with self.assertRaisesRegex(verifier.PdfValidationError, "outline"):
            verifier.verify_tagged_pdf_structure(self.pdf, outline)

        tampered_xmp = self.pdf.replace(
            b"xmlns:pdfuaid=", b"xmlns:pdfuaix=", 1
        )
        manifest = self.rebind_pdf_hashes(tampered_xmp, self.manifest)
        with self.assertRaises(verifier.PdfValidationError):
            verifier.verify_tagged_pdf_structure(tampered_xmp, manifest)

    def test_tagged_malformed_json_types_fail_closed(self) -> None:
        mutations: list[tuple[str, Callable[[dict], object]]] = [
            (
                "generated owner slot",
                lambda value: next(
                    node for node in value["structure"]
                    if node["owner"]["kind"] == "generated"
                )["owner"].update({"slot": []}),
            ),
            (
                "related node",
                lambda value: value["structure"][0].update({"related_nodes": [{}]}),
            ),
            (
                "outline relation",
                lambda value: value["structure"][0].update({"outline_ids": [{}]}),
            ),
            (
                "annotation destination",
                lambda value: value["marked_content"]["annotations"][0].update(
                    {"destination": []}
                ),
            ),
        ]
        for label, mutate in mutations:
            with self.subTest(label=label):
                manifest = copy.deepcopy(self.manifest)
                mutate(manifest)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure(self.pdf, manifest)

        manifest = copy.deepcopy(self.manifest)
        manifest["marked_content"]["annotations"][0]["annotation_id"] = False
        with tempfile.TemporaryDirectory() as directory:
            expectation_path = Path(directory) / "manifest.json"
            expectation_path.write_text(
                json.dumps(
                    manifest,
                    ensure_ascii=False,
                    separators=(",", ":"),
                    sort_keys=True,
                )
                + "\n",
                encoding="utf-8",
            )
            error = io.StringIO()
            with contextlib.redirect_stderr(error):
                status = verifier.main([str(TAGGED_PDF_PATH), str(expectation_path)])
        self.assertEqual(status, 1)
        self.assertIn("PDF structure validation failed:", error.getvalue())

    def test_tagged_pdf_object_role_plan_is_exact(self) -> None:
        manifest = copy.deepcopy(self.manifest)
        manifest["pdf"]["objects"][0]["role"] = "unexpected"
        with self.assertRaisesRegex(verifier.PdfValidationError, "role plan"):
            verifier.verify_tagged_pdf_structure(self.pdf, manifest)

    def test_tagged_command_line_dispatches_to_accessibility_validator(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = verifier.main([str(TAGGED_PDF_PATH), str(TAGGED_MANIFEST_PATH)])
        self.assertEqual(status, 0)
        decoded = json.loads(output.getvalue())
        self.assertEqual(decoded["structure_count"], 87)


class TaggedPdfStructureV2Tests(unittest.TestCase):
    def build(self) -> tuple[dict[int, bytes], dict, bytes]:
        return build_tagged_v2()

    @staticmethod
    def rebind(objects: dict[int, bytes], expectation: dict) -> bytes:
        info_object = next(
            item["object_number"]
            for item in expectation["pdf"]["objects"]
            if item["role"] == "info"
        )
        pdf = serialize(objects, info_object)
        expectation["pdf"]["byte_length"] = len(pdf)
        expectation["pdf"]["sha256"] = hashlib.sha256(pdf).hexdigest()
        for item in expectation["pdf"]["objects"]:
            item["sha256"] = hashlib.sha256(objects[item["object_number"]]).hexdigest()
        return pdf

    def build_outline_and_id_tree(self) -> tuple[dict[int, bytes], dict, bytes]:
        objects, expectation, _ = self.build()
        objects[1] = objects[1][:-3] + b" /Outlines 28 0 R >>"
        objects[3] = b"<< /Names [(vector-result) [5 0 R /XYZ 0 0 null] ] >>"
        objects[18] = objects[18][:-3] + b" /IDTree 30 0 R >>"
        objects[21] = objects[21][:-3] + b" /ID (typaxis-se-00000001) >>"
        objects[28] = b"<< /Type /Outlines /First 29 0 R /Last 29 0 R /Count 1 >>"
        objects[29] = (
            f"<< /Title <{utf16_hex('Vector result')}> /Parent 28 0 R "
            "/Dest (vector-result) /SE 21 0 R >>"
        ).encode("ascii")
        objects[30] = b"<< /Names [(typaxis-se-00000001) 21 0 R ] >>"
        for number, role in (
            (28, "outline_root"),
            (29, "outline_item:0"),
            (30, "structure_id_tree"),
        ):
            expectation["pdf"]["objects"].append(
                {
                    "object_number": number,
                    "role": role,
                    "sha256": hashlib.sha256(objects[number]).hexdigest(),
                }
            )
        expectation["pdf"]["object_count"] = len(objects)
        pdf = self.rebind(objects, expectation)
        return objects, expectation, pdf

    def test_tagged_v2_accepts_formula_figure_inner_spans_number_and_shared_form(self) -> None:
        _, expectation, pdf = self.build()
        observation = verifier.verify_tagged_pdf_structure_v2(pdf, expectation)
        self.assertEqual(observation["algorithm"], "typaxis.tagged-pdf-validator/2")
        self.assertEqual(observation["vector_count"], 4)
        self.assertEqual(observation["form_count"], 1)
        self.assertEqual(observation["form_do_count"], 4)
        self.assertEqual(observation["equation_number_count"], 1)
        self.assertEqual(observation["extracted_text"], ["xたすy", "xたすy、式1", "(1)"])

    def test_tagged_v2_rejects_alt_actual_text_language_role_and_mcid_tamper(self) -> None:
        mutations: list[tuple[str, Callable[[dict[int, bytes]], None]]] = [
            (
                "Alt",
                lambda objects: objects.__setitem__(
                    25,
                    objects[25].replace(
                        f"/Alt <{utf16_hex('配置図')}>".encode("ascii"),
                        f"/Alt <{utf16_hex('誤配置')}>".encode("ascii"),
                    ),
                ),
            ),
            (
                "ActualText",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(
                        f"/ActualText <{utf16_hex('xたすy')}>".encode("ascii"),
                        f"/ActualText <{utf16_hex('xひくy')}>".encode("ascii"),
                    ),
                ),
            ),
            (
                "Lang",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(
                        f"/Lang <{utf16_hex('en-US')}>".encode("ascii"),
                        f"/Lang <{utf16_hex('fr-FR')}>".encode("ascii"),
                        1,
                    ),
                ),
            ),
            (
                "role",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(b"/Figure << /MCID 0", b"/Formul << /MCID 0", 1),
                ),
            ),
            (
                "MCID",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(b"/Figure << /MCID 0", b"/Figure << /MCID 9", 1),
                ),
            ),
            (
                "missing Alt",
                lambda objects: objects.__setitem__(
                    25,
                    objects[25].replace(b"/Alt ", b"/Xlt ", 1),
                ),
            ),
            (
                "missing ActualText",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(b"/ActualText ", b"/XctualText ", 1),
                ),
            ),
            (
                "missing Lang",
                lambda objects: objects.__setitem__(
                    4,
                    objects[4].replace(b"/Lang ", b"/Xang ", 1),
                ),
            ),
        ]
        for label, mutate in mutations:
            with self.subTest(label=label):
                objects, expectation, _ = self.build()
                mutate(objects)
                pdf = self.rebind(objects, expectation)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

    def test_tagged_v2_rejects_page_parent_tree_and_formula_child_order(self) -> None:
        mutations: list[tuple[str, Callable[[dict[int, bytes]], None]]] = [
            (
                "page",
                lambda objects: objects.__setitem__(
                    26,
                    objects[26].replace(b"/Pg 7 0 R /MCID 1", b"/Pg 5 0 R /MCID 1"),
                ),
            ),
            (
                "ParentTree",
                lambda objects: objects.__setitem__(
                    19,
                    objects[19].replace(b"0 [23 0 R 24 0 R", b"0 [24 0 R 23 0 R"),
                ),
            ),
            (
                "Formula order",
                lambda objects: objects.__setitem__(
                    26,
                    objects[26].replace(
                        b"/K [<< /Type /MCR /Pg 7 0 R /MCID 1 >> 27 0 R ",
                        b"/K [27 0 R << /Type /MCR /Pg 7 0 R /MCID 1 >> ",
                    ),
                ),
            ),
            (
                "equation leaf role",
                lambda objects: objects.__setitem__(
                    27,
                    objects[27].replace(b"/S /Span", b"/S /Div ", 1),
                ),
            ),
        ]
        for label, mutate in mutations:
            with self.subTest(label=label):
                objects, expectation, _ = self.build()
                mutate(objects)
                pdf = self.rebind(objects, expectation)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

    def test_tagged_v2_rejects_form_mcid_and_same_length_stream_tamper(self) -> None:
        objects, expectation, _ = self.build()
        objects[16] = objects[16].replace(b"2 2 m", b"BMC  ")
        pdf = self.rebind(objects, expectation)
        with self.assertRaisesRegex(verifier.PdfValidationError, "Form contains semantic"):
            verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

        objects, expectation, pdf = self.build()
        tampered = pdf.replace(b"2 2 m", b"9 9 m", 1)
        self.assertEqual(len(tampered), len(pdf))
        with self.assertRaisesRegex(verifier.PdfValidationError, "byte closure"):
            verifier.verify_tagged_pdf_structure_v2(tampered, expectation)

    def test_tagged_v2_rejects_equation_font_cmap_and_cid_tamper(self) -> None:
        mutations: list[tuple[str, int, bytes, bytes]] = [
            ("Type0 encoding", 8, b"/Encoding /Identity-H", b"/Encoding /Identity-X"),
            ("ToUnicode", 12, b"beginbfchar", b"beginxfchar"),
            ("ToUnicode count", 12, b"3 beginbfchar", b"2 beginbfchar"),
            ("ToUnicode source", 12, b"<0001>", b"<FFFF>"),
            ("shown CID", 6, b"<0001> Tj", b"<FFFF> Tj"),
        ]
        for label, number, original, replacement in mutations:
            with self.subTest(label=label):
                objects, expectation, _ = self.build()
                objects[number] = objects[number].replace(original, replacement, 1)
                pdf = self.rebind(objects, expectation)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

    def test_tagged_v2_extracts_actual_text_only_equation_font(self) -> None:
        objects, expectation, _ = self.build()
        content = objects[12].split(b"stream\n", 1)[1].rsplit(b"\nendstream", 1)[0]
        begin = content.index(b"3 beginbfchar\n")
        end = content.index(b"endbfchar\n", begin) + len(b"endbfchar\n")
        objects[12] = stream("", content[:begin] + content[end:])
        pdf = self.rebind(objects, expectation)
        observation = verifier.verify_tagged_pdf_structure_v2(pdf, expectation)
        self.assertEqual(observation["extracted_text"], ["xたすy", "xたすy、式1", "(1)"])

    def test_tagged_v2_extracts_unicode_equation_number_after_japanese_math(self) -> None:
        objects, expectation, _ = self.build()
        objects[6] = objects[6].replace(
            f"/ActualText <{utf16_hex('(1)')}>".encode("ascii"),
            f"/ActualText <{utf16_hex('第1式')}>".encode("ascii"),
        )
        objects[12] = objects[12].replace(b"<0001> <0028>", b"<0001> <7B2C>")
        objects[12] = objects[12].replace(b"<0003> <0029>", b"<0003> <5F0F>")
        expectation["equation_numbers"][0]["exact_text"] = "第1式"
        pdf = self.rebind(objects, expectation)
        observation = verifier.verify_tagged_pdf_structure_v2(pdf, expectation)
        self.assertEqual(observation["extracted_text"], ["xたすy", "xたすy、式1", "第1式"])

    def test_tagged_v2_rejects_legacy_observation_and_object_budget_charge_swaps(self) -> None:
        _, expectation, pdf = self.build()
        legacy = copy.deepcopy(expectation)
        legacy["observation_algorithm"] = "typaxis.tagged-pdf-observation/1"
        with self.assertRaisesRegex(verifier.PdfValidationError, "observation algorithm"):
            verifier.verify_tagged_pdf_structure_v2(pdf, legacy)
        for count in (0, 2):
            with self.subTest(count=count):
                charged = copy.deepcopy(expectation)
                charged["object_budget_charge_count"] = count
                with self.assertRaisesRegex(verifier.PdfValidationError, "exactly once"):
                    verifier.verify_tagged_pdf_structure_v2(pdf, charged)

        malformed_role = copy.deepcopy(expectation)
        form = next(
            item
            for item in malformed_role["pdf"]["objects"]
            if item["role"].startswith("vector_form:")
        )
        form["role"] = "vector_form:" + "9" * 100
        with self.assertRaisesRegex(verifier.PdfValidationError, "role is not canonical"):
            verifier.verify_tagged_pdf_structure_v2(pdf, malformed_role)

    def test_tagged_v2_closes_catalog_page_parent_and_xmp(self) -> None:
        mutations: list[tuple[str, int, bytes, bytes]] = [
            ("catalog", 1, b"/Pages 2 0 R", b"/Pages 3 0 R"),
            ("page parent", 5, b"/Parent 2 0 R", b"/Parent 3 0 R"),
        ]
        for label, number, original, replacement in mutations:
            with self.subTest(label=label):
                objects, expectation, _ = self.build()
                objects[number] = objects[number].replace(original, replacement, 1)
                pdf = self.rebind(objects, expectation)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

        objects, expectation, _ = self.build()
        objects[15] = objects[15].replace(b"<x:xmpmeta/>", b"<y:xmpmeta/>", 1)
        pdf = self.rebind(objects, expectation)
        with self.assertRaisesRegex(verifier.PdfValidationError, "XMP metadata"):
            verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

    def test_tagged_v2_closes_outline_and_id_tree_graph(self) -> None:
        _, expectation, pdf = self.build_outline_and_id_tree()
        observation = verifier.verify_tagged_pdf_structure_v2(pdf, expectation)
        self.assertEqual(observation["object_count"], 30)
        mutations = [
            (29, b"/SE 21 0 R", b"/SE 16 0 R"),
            (29, b"/Parent 28 0 R", b"/Parent 29 0 R"),
            (28, b"/Count 1", b"/Count 2"),
            (18, b"/IDTree 30 0 R", b"/IDTree 19 0 R"),
            (30, b"typaxis-se-00000001", b"typaxis-se-00000002"),
            (30, b"21 0 R", b"22 0 R"),
        ]
        for number, original, replacement in mutations:
            with self.subTest(object=number, original=original):
                objects, expectation, _ = self.build_outline_and_id_tree()
                objects[number] = objects[number].replace(original, replacement, 1)
                pdf = self.rebind(objects, expectation)
                with self.assertRaises(verifier.PdfValidationError):
                    verifier.verify_tagged_pdf_structure_v2(pdf, expectation)

    def test_tagged_v2_command_line_dispatches_by_validator_identity(self) -> None:
        _, expectation, pdf = self.build()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pdf_path = root / "output.pdf"
            expectation_path = root / "expectation.json"
            pdf_path.write_bytes(pdf)
            expectation_path.write_text(
                json.dumps(expectation, ensure_ascii=False, separators=(",", ":"), sort_keys=True)
                + "\n",
                encoding="utf-8",
            )
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                status = verifier.main([str(pdf_path), str(expectation_path)])
        self.assertEqual(status, 0)
        self.assertEqual(json.loads(output.getvalue())["algorithm"], "typaxis.tagged-pdf-validator/2")


class ProductionBookStructureDerivationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.package = json.loads(
            (PRODUCTION_FIXTURE_PATH / "job/document-package.json").read_text("utf-8")
        )
        self.ledger = json.loads(
            (PRODUCTION_FIXTURE_PATH / "ledger.json").read_text("utf-8")
        )

    def test_complete_source_ledger_roles_text_and_alternatives_are_independent(self) -> None:
        nodes = verifier._production_source_nodes(self.package)
        self.assertEqual(len(nodes), 95)
        self.assertEqual(
            [node.get("node_id") for node in nodes], self.ledger["reading_order"]
        )
        self.assertEqual(verifier._production_source_ledger(self.package), self.ledger)
        roles = verifier._production_structure_roles(self.package)
        self.assertEqual(len(roles), 104)
        self.assertEqual(
            hashlib.sha256("\0".join(roles).encode("utf-8")).hexdigest(),
            "013b0a074d681d226a0c55a516221bb6a734504229a25d450b457fa4aae9ba3c",
        )
        actual_text = verifier._production_actual_text(self.package)
        self.assertEqual(len(actual_text), 38)
        self.assertEqual(actual_text[-3:], ["x plus y, equation AB", "AB", "Accessible footnote"])
        self.assertEqual(
            verifier._production_alternatives(self.package),
            [
                "PNG image",
                "x squared",
                "x plus one",
                "SafeVector 1 inline",
                "SafeVector 2 inline",
                "x plus y",
                "SafeVector 1 figure",
                "JPEG figure",
                "SafeVector 1 vector figure",
                "SafeVector 2 vector figure",
                "x plus y, equation AB",
            ],
        )

    def test_source_ledger_and_typed_source_mutations_fail_closed(self) -> None:
        wrong_ledger = copy.deepcopy(self.ledger)
        wrong_ledger["reading_order"][1:3] = reversed(
            wrong_ledger["reading_order"][1:3]
        )
        with self.assertRaisesRegex(verifier.PdfValidationError, "source ledger closure"):
            verifier.verify_production_pdf_structure(
                b"not a PDF", self.package, wrong_ledger, expected_page_count=2
            )

        wrong_span = copy.deepcopy(self.package)
        inline_math = next(
            node
            for node in verifier._production_source_nodes(wrong_span)
            if node["kind"] == "inline_math"
        )
        inline_math["math_source"]["text_span"]["end_byte"] = 10_000
        # Native-math extraction is speech-bound, so exercise the shared text
        # span validator through the producer equation number as well.
        equation = next(
            node
            for node in verifier._production_source_nodes(wrong_span)
            if node["kind"] == "equation_number"
        )
        equation["text_span"]["end_byte"] = 10_000
        with self.assertRaisesRegex(verifier.PdfValidationError, "source ledger closure"):
            verifier.verify_production_pdf_structure(
                b"not a PDF", wrong_span, self.ledger, expected_page_count=2
            )
        with self.assertRaisesRegex(verifier.PdfValidationError, "outside its buffer"):
            verifier._production_actual_text(wrong_span)

        wrong_kind = copy.deepcopy(self.package)
        paragraph = next(
            node
            for node in verifier._production_source_nodes(wrong_kind)
            if node["kind"] == "paragraph" and node.get("children")
        )
        paragraph["children"][0]["kind"] = "flattened_math"
        with self.assertRaisesRegex(verifier.PdfValidationError, "unsupported production inline"):
            verifier._production_structure_roles(wrong_kind)


if __name__ == "__main__":
    unittest.main()
