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
        self.assertEqual(observation["actual_text"], ["x squared", "x plus one"])
        self.assertEqual(
            [
                record["actual_text"]
                for page in observation["marked_pages"]
                for record in page
                if record.get("actual_text") is not None
            ],
            ["x plus one", "x squared"],
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


if __name__ == "__main__":
    unittest.main()
