#!/usr/bin/env python3
"""Independently verify actual Column font and physical paint contributions.

Original display/source and resource primitives are reused from the frozen
checkers. Font streams, object fragments and text operators are derived here;
the image helper derives vector/image fragments. No Column PDF acceptance,
original CID-CFF positive gate or whole layout/shaping replay is implied.
"""
from __future__ import annotations

import argparse
import copy
import json
import struct
import tempfile
import zlib
from collections import Counter
from pathlib import Path

import verify_book_v2_column_display as display
import verify_book_v2_column_resources as resource

check = display.check
integer = display.integer
digest = display.digest
domain = display.domain
hash_bytes = display.hash_bytes
u = display.u
fields = resource.fields
strict_equal = resource.strict_equal

SCHEMA = "typaxis.book-2-column-paint-probe/1"


class PDFBudget:
    def __init__(self, values):
        check(type(values) is list and len(values) == 4, "PDF charge quartet")
        self.records, self.spool, self.output, self.work = [
            integer(n, 64, False) for n in values
        ]

    def step(self, n=1):
        self.work += n

    def charge(self, records=0, spool=0, output=0, work=0):
        self.records += records
        self.spool += spool
        self.output += output
        self.work += work

    def reserve(self, records, spool, output):
        self.charge(records, spool, output)

    def fold(self, previous, value):
        check(len(value) <= 96, "PDF closed fold capacity")
        self.step((32 + len(value) + 63) // 64)
        return display.fold(previous, value)

    def triple4(self):
        return [self.records, self.spool, self.output, self.work]

    def matches4(self, stage):
        expected = stage["charges"] if isinstance(stage, dict) else stage
        strict_equal(expected, self.triple4(), "exact PDF reservation/work quartet")


class Sink:
    def __init__(self, budget):
        self.budget = budget
        self.bytes = bytearray()

    def extend(self, value):
        self.budget.step((len(value) + 63) // 64 + 1)
        self.bytes.extend(value)

    def push(self, value):
        self.extend(bytes([value]))

    def unsigned(self, value):
        self.extend(str(integer(value, 64, False)).encode())

    def integer(self, value):
        self.extend(str(integer(value)).encode())

    def number(self, raw):
        self.decimal(integer(raw) * 152_587_890_625, 16)

    def decimal(self, coefficient, scale):
        if not coefficient:
            return self.push(ord("0"))
        digits = str(abs(coefficient)).rjust(scale + 1, "0")
        result = (
            ((digits[:-scale] + "." + digits[-scale:]).rstrip("0").rstrip("."))
            if scale
            else digits
        )
        self.extend(("-" if coefficient < 0 else "").encode() + result.encode())

    def name(self, value):
        self.push(ord("/"))
        for byte in value:
            if 33 <= byte <= 126 and byte not in b"()<>[]{}/%#":
                self.push(byte)
            else:
                self.extend(("#%02X" % byte).encode())

    def hex(self, value):
        self.push(ord("<"))
        for byte in value:
            for part in ("%02X" % byte).encode():
                self.push(part)
        self.push(ord(">"))

    def utf16(self, text, bom=False):
        self.push(ord("<"))
        if bom:
            self.extend(b"FEFF")
        for byte in text.encode("utf-16-be"):
            for part in ("%02X" % byte).encode():
                self.push(part)
        self.push(ord(">"))

    def reference(self, value):
        self.unsigned(value)
        self.extend(b" 0 R")


CMAP_HEADER = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> def\n/CMapName /Typaxis-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"
CMAP_END = b"endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"


def artifact(root, value):
    resource.verify_artifact_fields(value)
    data = resource.artifact(root, value["file"], value["sha256"])
    check(len(data) == value["length"], "actual retained artifact length")
    return data


def encode_streams(probe, budget):
    sink = Sink(budget)
    fonts, actual = [], []
    for index, font in enumerate(probe["cids"]["fonts"]):
        bindings = font["bindings"]
        budget.step(2 * len(bindings) + 1)
        start = len(sink.bytes)
        sink.extend(CMAP_HEADER)
        mappings = [binding for binding in bindings if binding["unicode"] is not None]
        for first in range(0, len(mappings), 100):
            chunk = mappings[first : first + 100]
            sink.unsigned(len(chunk))
            sink.extend(b" beginbfchar\n")
            for binding in chunk:
                sink.hex(u(binding["cid"], 2))
                sink.push(ord(" "))
                sink.utf16(chr(binding["unicode"]))
                sink.push(ord("\n"))
            sink.extend(b"endbfchar\n")
        sink.extend(CMAP_END)
        split = len(sink.bytes)
        check(
            probe["closures"]["fonts"][index]["kind"] == "TrueType",
            "original CID-CFF gate unavailable",
        )
        sink.extend(bytes(2))
        for binding in bindings:
            sink.extend(u(binding["subset"], 2))
        fonts.append(
            dict(
                font=index,
                auxiliary_kind="TrueTypeCidToGid",
                start=start,
                split=split,
                end=len(sink.bytes),
            )
        )
    for use in probe["cids"]["uses"]:
        budget.step()
        if use["actual_text"] is None:
            actual.append(None)
        else:
            start = len(sink.bytes)
            sink.utf16(use["actual_text"], True)
            actual.append([start, len(sink.bytes)])
    return bytes(sink.bytes), fonts, actual


def verify_font_streams(root, probe, budget):
    stage = fields(
        probe["font_streams"],
        ("fingerprint", "charges", "fonts", "actual_text"),
        "font streams",
    )
    data, fonts, actual = encode_streams(probe, budget)
    metadata = (
        len(fonts) * probe["size_of"]["font_stream"]
        + len(actual) * probe["size_of"]["option_range"]
    )
    budget.reserve(len(fonts) + len(actual) + 1, metadata + len(data), len(data))
    budget.step()
    copied, second_fonts, second_actual = encode_streams(probe, budget)
    check(
        (copied, second_fonts, second_actual) == (data, fonts, actual),
        "both stream passes agree",
    )
    check(
        len(stage["fonts"]) == len(fonts) and len(stage["actual_text"]) == len(actual),
        "complete font stream contributions",
    )
    for reported, expected in zip(stage["fonts"], fonts):
        fields(
            reported,
            ("font", "auxiliary_kind", "to_unicode", "auxiliary"),
            "font stream contribution",
        )
        strict_equal(reported["font"], expected["font"], "font stream original index")
        check(
            reported["auxiliary_kind"] == expected["auxiliary_kind"],
            "font auxiliary kind",
        )
        check(
            artifact(root, reported["to_unicode"])
            == data[expected["start"] : expected["split"]],
            "independent original Unicode CMap bytes",
        )
        check(
            artifact(root, reported["auxiliary"])
            == data[expected["split"] : expected["end"]],
            "exact original CID-to-subset-GID map",
        )
    for reported, expected in zip(stage["actual_text"], actual):
        check(
            (reported is None) == (expected is None),
            "actual occurrence replacement presence",
        )
        if expected is not None:
            check(
                artifact(root, reported) == data[expected[0] : expected[1]],
                "exact UTF16BE BOM occurrence token",
            )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        domain("typaxis.book-2-font-streams/1"),
        hash_bytes(probe["cids"]["fingerprint"]),
    )
    fp = budget.fold(fp, digest(data))
    for font in fonts:
        fp = budget.fold(
            fp,
            u(font["font"])
            + u(font["start"])
            + u(font["split"])
            + u(font["end"])
            + bytes([0]),
        )
    for value in actual:
        fp = budget.fold(
            fp,
            bytes([int(value is not None)])
            + u(value[0] if value else 0)
            + u(value[1] if value else 0),
        )
    check(fp == hash_bytes(stage["fingerprint"]), "font stream canonical fold")
    budget.matches4(stage)
    return data, fonts, actual


FONT_ROLES = ("Type0", "CidFont", "Descriptor", "Program", "ToUnicode", "Auxiliary")


def font_dictionary(sink, probe, index, role, ids):
    program = probe["programs"]["fonts"][index]
    metric = program["metrics"]
    name = program["postscript_name"]
    bindings = probe["cids"]["fonts"][index]["bindings"]
    if role == "Type0":
        fields_to_write = [
            ("BaseFont", "name", name),
            ("DescendantFonts", "descendant", ids[1]),
            ("Encoding", "name", "Identity-H"),
            ("Subtype", "name", "Type0"),
            ("ToUnicode", "ref", ids[4]),
            ("Type", "name", "Font"),
        ]
    elif role == "CidFont":
        fields_to_write = [
            ("BaseFont", "name", name),
            ("CIDSystemInfo", "system", None),
            ("CIDToGIDMap", "ref", ids[5]),
            ("DW", "int", 1000),
            ("FontDescriptor", "ref", ids[2]),
            ("Subtype", "name", "CIDFontType2"),
            ("Type", "name", "Font"),
        ]
        if bindings:
            fields_to_write.append(("W", "widths", bindings))
    else:
        fields_to_write = [
            ("Ascent", "int", metric["ascent"]),
            ("CapHeight", "int", metric["cap_height"]),
            ("Descent", "int", metric["descent"]),
            ("Flags", "int", metric["flags"]),
            ("FontBBox", "bbox", metric["bbox"]),
            ("FontFile2", "ref", ids[3]),
            ("FontName", "name", name),
            ("ItalicAngle", "angle", metric["italic_angle"]),
            ("StemV", "int", metric["stem_v"]),
            ("Type", "name", "FontDescriptor"),
        ]
    sink.extend(b"<<")
    for key, kind, value in fields_to_write:
        sink.push(ord(" "))
        sink.name(key.encode())
        sink.push(ord(" "))
        if kind == "name":
            sink.name(value.encode())
        elif kind == "ref":
            sink.reference(value)
        elif kind == "descendant":
            sink.push(ord("["))
            sink.reference(value)
            sink.push(ord("]"))
        elif kind == "int":
            sink.integer(value)
        elif kind == "angle":
            sink.decimal(resource.trunc_div(value * 1000, 65536), 3)
        elif kind == "bbox":
            sink.push(ord("["))
            for offset, number in enumerate(value):
                if offset:
                    sink.push(ord(" "))
                sink.integer(number)
            sink.push(ord("]"))
        elif kind == "system":
            sink.extend(
                b"<< /Ordering <4964656E74697479> /Registry <41646F6265> /Supplement 0 >>"
            )
        elif kind == "widths":
            sink.push(ord("["))
            for offset, binding in enumerate(value):
                if offset:
                    sink.push(ord(" "))
                sink.unsigned(binding["cid"])
                sink.extend(b" [")
                sink.unsigned(binding["width"])
                sink.push(ord("]"))
            sink.push(ord("]"))
    sink.extend(b" >>")


def encode_font_objects(root, probe, streams, budget):
    stream_bytes, stream_fonts, _ = streams
    sink, objects = Sink(budget), []
    first = integer(probe["font_objects"]["first"], 32, False)
    check(first > 0, "positive font object origin")
    for index, program in enumerate(probe["programs"]["fonts"]):
        budget.step(256)
        ids = list(range(first + 6 * index, first + 6 * index + 6))
        for id, role in zip(ids, FONT_ROLES):
            start = len(sink.bytes)
            sink.unsigned(id)
            sink.extend(b" 0 obj\n")
            if role in FONT_ROLES[:3]:
                font_dictionary(sink, probe, index, role, ids)
            else:
                font = stream_fonts[index]
                payload = (
                    artifact(root, program["program"])
                    if role == "Program"
                    else (
                        stream_bytes[font["start"] : font["split"]]
                        if role == "ToUnicode"
                        else stream_bytes[font["split"] : font["end"]]
                    )
                )
                sink.extend(b"<< /Length ")
                sink.unsigned(len(payload))
                if role == "Program":
                    sink.extend(b" /Length1 ")
                    sink.unsigned(len(payload))
                sink.extend(b" >>\nstream\n")
                sink.extend(payload)
                sink.extend(b"\nendstream")
            sink.extend(b"\nendobj\n")
            objects.append(
                dict(id=id, font=index, role=role, range=[start, len(sink.bytes)])
            )
    return bytes(sink.bytes), objects


def verify_font_objects(root, probe, streams, budget):
    stage = fields(
        probe["font_objects"],
        ("fingerprint", "charges", "first", "next", "bytes", "objects"),
        "font objects",
    )
    credit = sum(font["end"] - font["start"] for font in streams[1])
    budget.step(len(streams[1]))
    data, objects = encode_font_objects(root, probe, streams, budget)
    budget.reserve(
        len(objects) + 1,
        len(objects) * probe["size_of"]["font_object"] + len(data),
        len(data) - credit,
    )
    budget.step()
    copied, second = encode_font_objects(root, probe, streams, budget)
    check((copied, second) == (data, objects), "both object encoding passes agree")
    strict_equal(
        stage["objects"], objects, "all exact original font objects/roles/ranges"
    )
    check(
        stage["next"] == stage["first"] + len(objects), "exclusive font object cursor"
    )
    check(
        artifact(root, stage["bytes"]) == data,
        "exact font dictionaries and original embedded payloads",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        domain("typaxis.book-2-font-objects/1"),
        hash_bytes(probe["font_streams"]["fingerprint"]),
    )
    fp = budget.fold(fp, u(stage["first"], 4))
    fp = budget.fold(fp, digest(data))
    for obj in objects:
        fp = budget.fold(
            fp,
            u(obj["id"], 4)
            + u(obj["font"])
            + u(obj["range"][0])
            + u(obj["range"][1])
            + bytes([FONT_ROLES.index(obj["role"])]),
        )
    check(fp == hash_bytes(stage["fingerprint"]), "font object canonical fold")
    budget.matches4(stage)


def text_slots(probe):
    original = probe["display_probe"]
    result = []
    for paint, (tag, index) in enumerate(original["paints"]):
        slots = []
        if tag == "text":
            draw = original["text"][index]
            slots.append(
                dict(page=draw["page"], slot=0, glyphs=draw["glyphs"], rule=None)
            )
        elif tag in ("marker", "number"):
            draw = original["markers" if tag == "marker" else "numbers"][index]
            page = (
                original["original_fragments"][draw["fragment"]]["page"]
                if tag == "marker"
                else draw["page"]
            )
            slots = [
                dict(page=page, slot=offset, glyphs=cluster["glyphs"], rule=None)
                for offset, cluster in enumerate(draw["clusters"])
            ]
        elif tag == "math":
            draw = original["math"][index]
            if draw["paint"]["kind"] == "native":
                for offset, source in enumerate(draw["paint"]["paints"]):
                    slots.append(
                        dict(
                            page=draw["page"],
                            slot=offset,
                            glyphs=(
                                [[source["gid"], source["x"], source["y"]]]
                                if "gid" in source
                                else []
                            ),
                            rule=source.get("rule"),
                        )
                    )
        elif tag == "separator":
            draw = original["separators"][index]
            slots.append(
                dict(page=draw["page"], slot=None, glyphs=[], rule=draw["ink"])
            )
        result.append(slots)
    return result


def encode_text(probe, slots, streams, budget):
    sink, commands, ranges = Sink(budget), [], []
    uses = probe["fonts_selection"]["uses"]
    cids = probe["cids"]["uses"]
    cursor = 0
    for paint, positions in enumerate(slots):
        budget.step()
        before = len(commands)
        for source in positions:
            budget.step()
            start = len(sink.bytes)
            if source["rule"] is not None:
                sink.extend(b"0 g\n")
                for index, raw in enumerate(source["rule"]):
                    if index:
                        sink.push(ord(" "))
                    sink.number(raw)
                sink.extend(b" re f\n")
                usage = None
                actual = None
            else:
                usage = cursor
                use, cid_use = uses[cursor], cids[cursor]
                check(
                    (use["paint"], use["slot"]) == (paint, source["slot"]),
                    "all source glyph occurrences preserve CID usage order",
                )
                check(
                    [glyph[0] for glyph in source["glyphs"]] == use["glyphs"],
                    "actual original physical slot GIDs",
                )
                sink.extend(b"0 g\n")
                sink.extend(b"BT /PB")
                sink.unsigned(use["instance"])
                sink.push(ord(" "))
                sink.number(use["size"])
                sink.extend(b" Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n")
                for (_, x, y), cid in zip(source["glyphs"], cid_use["cids"]):
                    sink.extend(b"1 0 0 -1 ")
                    sink.number(x)
                    sink.push(ord(" "))
                    sink.number(y)
                    sink.extend(b" Tm ")
                    sink.hex(u(cid, 2))
                    sink.extend(b" Tj\n")
                sink.extend(b"ET\n")
                actual = streams[2][cursor]
                cursor += 1
            commands.append(
                dict(
                    paint=paint,
                    typed_paint=probe["display_probe"]["paints"][paint],
                    slot=source["slot"],
                    usage=usage,
                    page=source["page"],
                    range=[start, len(sink.bytes)],
                    actual_text=actual,
                )
            )
        ranges.append([before, len(commands)])
    check(cursor == len(uses), "each original font occurrence encoded exactly once")
    return bytes(sink.bytes), commands, ranges


def verify_text(root, probe, streams, budget):
    stage = fields(
        probe["text_commands"],
        ("fingerprint", "charges", "bytes", "commands"),
        "text commands",
    )
    slots = text_slots(probe)
    budget.step(len(slots))
    data, commands, ranges = encode_text(probe, slots, streams, budget)
    metadata = (
        len(commands) * probe["size_of"]["text_command"]
        + len(slots) * probe["size_of"]["range"]
    )
    budget.reserve(len(commands) + len(slots) + 1, metadata + len(data), len(data))
    budget.step()
    copied, second, second_ranges = encode_text(probe, slots, streams, budget)
    check(
        (copied, second, second_ranges) == (data, commands, ranges),
        "both text encoding passes agree",
    )
    check(len(stage["commands"]) == len(commands), "all glyph and actual rule commands")
    for reported, expected in zip(stage["commands"], commands):
        fields(
            reported,
            ("paint", "typed_paint", "slot", "usage", "page", "range", "actual_text"),
            "text command occurrence",
        )
        view = dict(reported)
        replacement = view.pop("actual_text")
        match = dict(expected)
        token_range = match.pop("actual_text")
        strict_equal(view, match, "typed original glyph/rule page/slot/range")
        check(
            (replacement is None) == (token_range is None),
            "source ActualText occurrence token",
        )
        if token_range is not None:
            check(
                artifact(root, replacement)
                == streams[0][token_range[0] : token_range[1]],
                "same original occurrence UTF16 replacement",
            )
    check(
        artifact(root, stage["bytes"]) == data,
        "exact physical Tf/Tm/Tj/rule bytes without added column origin",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        domain("typaxis.book-2-text-commands/1"),
        hash_bytes(probe["font_objects"]["fingerprint"]),
    )
    fp = budget.fold(fp, digest(data))
    for command in commands:
        fp = budget.fold(
            fp,
            u(command["paint"])
            + bytes([int(command["slot"] is not None)])
            + u(command["slot"] or 0)
            + bytes([int(command["usage"] is not None)])
            + u(command["usage"] or 0)
            + u(command["page"], 4)
            + u(command["range"][0])
            + u(command["range"][1]),
        )
    for start, end in ranges:
        fp = budget.fold(fp, u(start) + u(end))
    check(fp == hash_bytes(stage["fingerprint"]), "text command canonical fold")
    budget.matches4(stage)
    return len(commands), sum(
        source["rule"] is not None for paint in slots for source in paint
    )


PDF_SIZE_FIELDS = (
    "font_stream",
    "font_object",
    "text_command",
    "vector_slot",
    "vector_state",
    "image_object",
    "image_command",
    "range",
    "option_range",
)


def constructor(probe, index, budget):
    expected = budget.triple4()
    expected[0] += 1
    strict_equal(
        probe["observed"]["constructors"][index],
        expected,
        "accepted PDF constructor prefix",
    )
    budget.charge(records=1)


def verify(root, probe):
    fields(
        probe,
        (
            "wire",
            "case",
            "display_probe",
            "original_font_instances",
            "original_font_origins",
            "original_images",
            "fonts_selection",
            "closures",
            "programs",
            "cids",
            "images_selection",
            "rasters",
            "observed",
            "size_of",
            "font_streams",
            "font_objects",
            "text_commands",
            "vector_programs",
            "image_objects",
            "image_commands",
            "image_boundary",
        ),
        "column paint probe",
    )
    check(
        probe["wire"] == SCHEMA and probe["case"] == probe["display_probe"]["case"],
        "actual column paint schema/case",
    )
    display.verify(probe["display_probe"])
    source = display.decode_source(probe["display_probe"])
    resources = display.verify_resources(probe["display_probe"], source)
    resource.verify_original_images(root, probe, resources)
    instances = resource.verify_font_instances(probe, resources)
    fields(
        probe["size_of"],
        (*resource.SIZE_FIELDS, *PDF_SIZE_FIELDS),
        "actual paint reservation units",
    )
    for value in probe["size_of"].values():
        check(integer(value, 64, False) > 0, "positive actual reservation size")
    check(
        probe["size_of"]["usize"] in (4, 8) and probe["size_of"]["cid"] == 2,
        "platform pointer and CID reservation units",
    )
    observed = fields(
        probe["observed"],
        (
            "scope",
            "maximum_work",
            "constructors",
            "caller_before",
            "caller_after",
            "ancestor_callback",
        ),
        "paint observation",
    )
    scope = observed["scope"]
    check(
        scope in ("paint_driver", "source_callback", "retained_source_cut"),
        "explicit paint evidence scope",
    )
    check(len(observed["constructors"]) == 8, "every actual stage constructor")
    for values in observed["constructors"]:
        PDFBudget(values)
    source_observed = probe["display_probe"]["observed"]
    first = observed["constructors"][0]
    if scope == "paint_driver":
        before = PDFBudget(observed["caller_before"])
        check(
            source_observed["scope"] == "display_callback"
            and probe["display_probe"]["selection_mode"] == "automatic_driver",
            "natural column paint driver lineage",
        )
        strict_equal(
            first,
            [
                source_observed["display_records"] + 1,
                max(source_observed["source_spool"], before.spool),
                before.output,
                source_observed["display_work"],
            ],
            "actual font constructor caller/source floor",
        )
    else:
        check(
            observed["caller_before"] is None and observed["caller_after"] is None,
            "manual source chain has no paint driver claim",
        )
        check(
            scope == source_observed["scope"],
            "actual source callback or explicit retained cut",
        )
        strict_equal(
            first,
            [
                source_observed["display_records"] + 1,
                source_observed["source_spool"],
                0,
                source_observed["display_work"],
            ],
            "direct font constructor actual prefix",
        )
        check(
            (observed["ancestor_callback"] is None) == (scope == "retained_source_cut"),
            "only an actual source callback has ancestor observations",
        )
    font_budget = resource.Budget([first[0], first[1], first[3]])
    uses = resource.verify_font_selection(probe, resources, instances, font_budget)
    sources = resource.verify_closures(root, probe, font_budget)
    resource.verify_programs(root, probe, sources, font_budget)
    resource.verify_cids(probe, uses, sources, font_budget)
    budget = PDFBudget(
        [font_budget.records, font_budget.spool, first[2], font_budget.work]
    )
    constructor(probe, 1, budget)
    streams = verify_font_streams(root, probe, budget)
    constructor(probe, 2, budget)
    verify_font_objects(root, probe, streams, budget)
    constructor(probe, 3, budget)
    commands, rules = verify_text(root, probe, streams, budget)
    boundary = fields(
        probe["image_boundary"],
        ("text_prefix", "resource_prior", "vector_prior_output"),
        "actual encoded text before images",
    )
    strict_equal(
        boundary["text_prefix"], budget.triple4(), "complete actual encoded text prefix"
    )
    strict_equal(
        boundary["resource_prior"],
        [budget.records, budget.spool, budget.work],
        "image selection follows actual encoded text",
    )
    strict_equal(
        boundary["vector_prior_output"],
        budget.output,
        "vector output follows encoded text",
    )
    constructor(probe, 4, budget)
    image_budget = resource.Budget([budget.records, budget.spool, budget.work])
    image_uses = resource.verify_images(root, probe, resources, image_budget)
    rasters = resource.verify_rasters(root, probe, image_budget)
    budget.records, budget.spool, budget.work = image_budget.triple()
    from verify_book_v2_column_paint_images import verify_image_paints

    image_counts = verify_image_paints(root, probe, budget)
    base = resources["limits"]["base"]
    extension = resources["limits"]["extension"]
    maximum_work = integer(observed["maximum_work"], 64, False)
    check(maximum_work == source_observed["maximum_work"], "same cumulative work cap")
    for values in [
        *observed["constructors"],
        *[
            probe[name]["charges"]
            for name in (
                "font_streams",
                "font_objects",
                "text_commands",
                "vector_programs",
                "image_objects",
                "image_commands",
            )
        ],
    ]:
        r, s, o, w = PDFBudget(values).triple4()
        check(
            r <= base["max_fragments"]
            and s <= base["max_spool_bytes"]
            and o <= base["max_output_bytes"]
            and w <= maximum_work,
            "inclusive actual stage record/spool/output/work caps",
        )
    for name in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
    ):
        r, s, w = resource.Budget(probe[name]["charges"]).triple()
        check(
            r <= base["max_fragments"]
            and s <= base["max_spool_bytes"]
            and w <= maximum_work,
            "inclusive resource contribution caps",
        )
    for font, program in zip(
        probe["fonts_selection"]["fonts"], probe["programs"]["fonts"]
    ):
        check(
            len(font["selected"]) <= base["max_cids_per_font"]
            and program["program"]["length"] <= extension["max_font_subset_bytes"],
            "actual CID and subset byte caps",
        )
    for name in ("font_objects", "image_objects"):
        stage = probe[name]
        first_object = integer(stage["first"], 32, False)
        next_object = integer(stage["next"], 32, False)
        check(
            first_object > 0
            and next_object == first_object + len(stage["objects"])
            and next_object - 1 <= base["max_pdf_objects"],
            "bounded exclusive object cursor",
        )
    if scope == "paint_driver":
        strict_equal(
            observed["caller_after"],
            budget.triple4(),
            "caller retains complete accepted paint prefix",
        )
        strict_equal(
            observed["ancestor_callback"],
            budget.triple4(),
            "actual paint callback observation",
        )
        check(
            probe["font_objects"]["next"] == probe["image_objects"]["first"],
            "disjoint contiguous font/image object contributions",
        )
        check(
            all(a <= b for a, b in zip(before.triple4(), budget.triple4())),
            "caller counters retain their complete prior prefix",
        )
    elif scope == "retained_source_cut":
        check(
            observed["ancestor_callback"] is None,
            "retained cut cannot invent driver observations",
        )
    else:
        ancestor = PDFBudget(observed["ancestor_callback"])
        strict_equal(
            ancestor.triple4(),
            [
                source_observed["driver_records"],
                source_observed["driver_spool"],
                0,
                source_observed["driver_work"],
            ],
            "manual source callback preserves actual predecessor observations",
        )
        # A direct display builder with prior=(0,0) starts from the retained
        # source counters. It does not represent the driver's cumulative work
        # through every earlier candidate/retry. Only paint_driver claims that
        # complete caller prefix; manual evidence keeps both observations.
    return dict(
        pages=len(probe["display_probe"]["pages"]),
        paints=len(probe["display_probe"]["paints"]),
        fonts=len(sources),
        glyph_uses=len(uses),
        text_commands=commands,
        rules=rules,
        closed_glyphs=sum(len(source["closed"]) for source in sources),
        font_objects=len(probe["font_objects"]["objects"]),
        image_resources=len(probe["images_selection"]["images"]),
        image_uses=len(image_uses),
        png=rasters[0],
        jpeg=rasters[1],
        alpha=rasters[2],
        scope=scope,
        selection_mode=probe["display_probe"]["selection_mode"],
        **image_counts,
    )


def reseal_font_paints(root, probe):
    """Reissue reported output hashes for adversarial probes, not source facts."""
    stage = probe["font_streams"]
    joined, fonts, actual = bytearray(), [], []
    for font in stage["fonts"]:
        start = len(joined)
        joined.extend(artifact(root, font["to_unicode"]))
        split = len(joined)
        joined.extend(artifact(root, font["auxiliary"]))
        fonts.append((font["font"], start, split, len(joined)))
    for token in stage["actual_text"]:
        if token is None:
            actual.append(None)
        else:
            start = len(joined)
            joined.extend(artifact(root, token))
            actual.append([start, len(joined)])
    fp = display.fold(
        domain("typaxis.book-2-font-streams/1"),
        hash_bytes(probe["cids"]["fingerprint"]),
    )
    fp = display.fold(fp, digest(joined))
    for index, start, split, end in fonts:
        fp = display.fold(fp, u(index) + u(start) + u(split) + u(end) + bytes([0]))
    for value in actual:
        fp = display.fold(
            fp,
            bytes([int(value is not None)])
            + u(value[0] if value else 0)
            + u(value[1] if value else 0),
        )
    stage["fingerprint"] = fp.hex()
    stage = probe["font_objects"]
    fp = display.fold(
        domain("typaxis.book-2-font-objects/1"),
        hash_bytes(probe["font_streams"]["fingerprint"]),
    )
    fp = display.fold(fp, u(stage["first"], 4))
    fp = display.fold(fp, digest(artifact(root, stage["bytes"])))
    for obj in stage["objects"]:
        fp = display.fold(
            fp,
            u(obj["id"], 4)
            + u(obj["font"])
            + u(obj["range"][0])
            + u(obj["range"][1])
            + bytes([FONT_ROLES.index(obj["role"])]),
        )
    stage["fingerprint"] = fp.hex()
    stage = probe["text_commands"]
    fp = display.fold(
        domain("typaxis.book-2-text-commands/1"),
        hash_bytes(probe["font_objects"]["fingerprint"]),
    )
    fp = display.fold(fp, digest(artifact(root, stage["bytes"])))
    for command in stage["commands"]:
        fp = display.fold(
            fp,
            u(command["paint"])
            + bytes([int(command["slot"] is not None)])
            + u(command["slot"] or 0)
            + bytes([int(command["usage"] is not None)])
            + u(command["usage"] or 0)
            + u(command["page"], 4)
            + u(command["range"][0])
            + u(command["range"][1]),
        )
    cursor = 0
    for paint in range(len(probe["display_probe"]["paints"])):
        start = cursor
        while (
            cursor < len(stage["commands"])
            and stage["commands"][cursor]["paint"] == paint
        ):
            cursor += 1
        fp = display.fold(fp, u(start) + u(cursor))
    stage["fingerprint"] = fp.hex()


REJECTION_ERRORS = (
    AssertionError,
    KeyError,
    IndexError,
    ValueError,
    TypeError,
    struct.error,
)


def altered_subset_glyph(data):
    """Alter a real glyph while retaining valid sfnt and whole checksums."""
    changed = bytearray(data)
    entries = [
        struct.unpack(">4sIII", changed[at : at + 16])
        for at in range(12, 12 + 16 * int.from_bytes(changed[4:6], "big"), 16)
    ]
    glyf = next(entry for entry in entries if entry[0] == b"glyf")
    check(glyf[3] > 0, "actual positive TrueType glyf payload")
    changed[glyf[2] + min(10, glyf[3] - 1)] ^= 1
    head_at = next(entry[2] for entry in entries if entry[0] == b"head")
    changed[head_at + 8 : head_at + 12] = bytes(4)
    for index, (_, _, offset, length) in enumerate(entries):
        changed[16 + 16 * index : 20 + 16 * index] = u(
            resource.checksum(bytes(changed[offset : offset + length])), 4
        )
    changed[head_at + 8 : head_at + 12] = u(
        (0xB1B0AFBA - resource.checksum(bytes(changed))) & 0xFFFFFFFF, 4
    )
    check(
        resource.checksum(bytes(changed)) == 0xB1B0AFBA,
        "negative glyph program has valid whole checksum",
    )
    return bytes(changed)


def altered_raster_pixel(data):
    pixels = bytearray(zlib.decompress(data))
    check(pixels, "actual nonempty raster pixels")
    pixels[0] ^= 1
    return zlib.compress(bytes(pixels), 6)


def tamper_checks(root, probe, missing_seen):
    rejected = 0

    def reject(bad, label, at=root):
        nonlocal rejected
        try:
            verify(at, bad)
        except REJECTION_ERRORS:
            rejected += 1
        else:
            raise AssertionError("column paint tamper accepted: " + label)

    for path in resource.schema_paths(probe):
        normalized = tuple(0 if type(part) is int else part for part in path)
        if normalized in missing_seen:
            continue
        missing_seen.add(normalized)
        bad = copy.deepcopy(probe)
        container, key = resource.location(bad, path)
        del container[key]
        reject(bad, "missing " + ".".join(map(str, path)))
    for stage in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
        "font_streams",
        "font_objects",
        "text_commands",
        "vector_programs",
        "image_objects",
        "image_commands",
    ):
        bad = copy.deepcopy(probe)
        bad[stage]["fingerprint"] = bytes(32).hex()
        reject(bad, stage + " fingerprint")
        for counter, value in enumerate(probe[stage]["charges"]):
            if value:
                bad = copy.deepcopy(probe)
                bad[stage]["charges"][counter] -= 1
                reject(bad, stage + " accepted charge " + str(counter))
    for index, values in enumerate(probe["observed"]["constructors"]):
        for counter, value in enumerate(values):
            if value:
                bad = copy.deepcopy(probe)
                bad["observed"]["constructors"][index][counter] -= 1
                reject(
                    bad,
                    "accepted constructor " + str(index) + " counter " + str(counter),
                )
    for key in ("caller_after", "ancestor_callback"):
        values = probe["observed"][key]
        if values is not None:
            for axis in range(4):
                bad = copy.deepcopy(probe)
                bad["observed"][key][axis] += 1
                reject(bad, "actual observation " + key + " axis " + str(axis))
    for key in ("text_prefix", "resource_prior"):
        for axis in range(len(probe["image_boundary"][key])):
            bad = copy.deepcopy(probe)
            bad["image_boundary"][key][axis] += 1
            reject(bad, "actual text before images " + key + " axis " + str(axis))
    bad = copy.deepcopy(probe)
    bad["image_boundary"]["vector_prior_output"] += 1
    reject(bad, "actual vector output prefix after text")
    mutations = []
    if probe["fonts_selection"]["uses"]:
        mutations.extend(
            [
                (("fonts_selection", "uses", 0, "glyphs", 0), 1),
                (("fonts_selection", "uses", 0, "instance"), 1),
                (("fonts_selection", "uses", 0, "size"), 1),
                (("fonts_selection", "uses", 0, "text"), "x"),
            ]
        )
    if probe["closures"]["fonts"]:
        mutations.append((("closures", "fonts", 0, "mapping", 0, 1), 1))
    if probe["programs"]["fonts"]:
        mutations.extend(
            [
                (("programs", "fonts", 0, "metrics", "ascent"), 1),
                (("programs", "fonts", 0, "advances", 0, 1), 1),
            ]
        )
    if probe["cids"]["fonts"]:
        mutations.extend(
            [
                (("cids", "fonts", 0, "bindings", 0, "width"), 1),
                (("cids", "fonts", 0, "bindings", 0, "subset"), 1),
            ]
        )
    if probe["cids"]["uses"]:
        mutations.extend(
            [
                (("cids", "uses", 0, "cids", 0), 1),
                (("cids", "uses", 0, "requires_actual_text"), None),
            ]
        )
    if probe["font_objects"]["objects"]:
        for key in ("id", "font"):
            mutations.append((("font_objects", "objects", 0, key), 1))
    if probe["text_commands"]["commands"]:
        for key in ("paint", "page"):
            mutations.append((("text_commands", "commands", 0, key), 1))
        mutations.append((("text_commands", "commands", 0, "range", 0), 1))
    for path, increment in mutations:
        bad = copy.deepcopy(probe)
        container, key = resource.location(bad, path)
        container[key] = (
            not container[key] if increment is None else container[key] + increment
        )
        resource.reseal_resources(bad)
        reseal_font_paints(root, bad)
        reject(bad, "resealed " + ".".join(map(str, path)))
    from verify_book_v2_column_paint_images import (
        image_tamper_cases,
        image_artifact_tamper_cases,
        reseal_image_paints,
    )

    for label, bad in image_tamper_cases(probe):
        if "missing fingerprint" not in label:
            reseal_image_paints(root, bad)
        reject(bad, label)
    artifact_cases = []
    if probe["font_streams"]["fonts"]:
        artifact_cases.extend(
            [
                (
                    "resealed wrong Unicode CMap",
                    ("font_streams", "fonts", 0, "to_unicode"),
                    lambda b: (
                        b.replace(b"<0066>", b"<0078>", 1)
                        if b"<0066>" in b
                        else b[:-1] + bytes([b[-1] ^ 1])
                    ),
                ),
                (
                    "resealed wrong CID-to-GID",
                    ("font_streams", "fonts", 0, "auxiliary"),
                    lambda b: b[:2] + bytes([b[2] ^ 1]) + b[3:],
                ),
                (
                    "resealed wrong font dictionary",
                    ("font_objects", "bytes"),
                    lambda b: b.replace(b"/Identity-H", b"/Identity-V", 1),
                ),
            ]
        )
    if probe["text_commands"]["commands"]:
        artifact_cases.extend(
            [
                (
                    "resealed wrong physical glyph position",
                    ("text_commands", "bytes"),
                    lambda b: b.replace(b"1 0 0 -1 ", b"1 0 1 -1 ", 1),
                ),
                (
                    "resealed wrong CID command",
                    ("text_commands", "bytes"),
                    lambda b: (
                        b.replace(b"<0001>", b"<0000>", 1)
                        if b"<0001>" in b
                        else b.replace(b" Tj", b" TJ", 1)
                    ),
                ),
            ]
        )
        if any(
            command["usage"] is None for command in probe["text_commands"]["commands"]
        ):
            artifact_cases.append(
                (
                    "resealed wrong native/separator rule",
                    ("text_commands", "bytes"),
                    lambda b: b.replace(b" re f", b" re S", 1),
                )
            )
    for index, token in enumerate(probe["font_streams"]["actual_text"]):
        if token is not None:
            artifact_cases.append(
                (
                    "resealed wrong ActualText UTF16",
                    ("font_streams", "actual_text", index),
                    lambda b: b[:1] + bytes([b[1] ^ 1]) + b[2:],
                )
            )
            break
    for index, program in enumerate(probe["programs"]["fonts"]):
        actual_tables = resource.font_tables(artifact(root, program["program"]), 0)
        if actual_tables.get(b"glyf"):
            artifact_cases.append(
                (
                    "checksum-valid resealed original glyph corruption",
                    ("programs", "fonts", index, "program"),
                    altered_subset_glyph,
                )
            )
            break
    for index, program in enumerate(probe["rasters"]["programs"]):
        if program is not None and program["encoding"] == "Flate":
            artifact_cases.append(
                (
                    "resealed raster source-pixel corruption",
                    ("rasters", "programs", index, "program"),
                    altered_raster_pixel,
                )
            )
            if program["alpha"] is not None:
                artifact_cases.append(
                    (
                        "resealed original alpha-pixel corruption",
                        ("rasters", "programs", index, "alpha", "program"),
                        altered_raster_pixel,
                    )
                )
    artifact_cases.extend(image_artifact_tamper_cases(probe))
    for label, path, mutate in artifact_cases:
        bad = copy.deepcopy(probe)
        container, key = resource.location(bad, path)
        descriptor = container[key]
        original = artifact(root, descriptor)
        altered = mutate(original)
        check(
            altered != original, "negative mutation changes actual artifact: " + label
        )
        with tempfile.TemporaryDirectory(
            prefix="typaxis-column-paint-negative-"
        ) as directory:
            isolated = Path(directory)
            for name in root.glob("*.bin"):
                (isolated / name.name).symlink_to(name.resolve())
            # Immutable originals remain untouched; only replacement bytes are copied.
            filename = "negative-" + digest(altered).hex() + ".bin"
            (isolated / filename).write_bytes(altered)
            container[key] = dict(
                file=filename, sha256=digest(altered).hex(), length=len(altered)
            )
            if path[0] == "programs":
                container["sha256"] = digest(altered).hex()
            elif path[0] == "rasters":
                container["sha256"] = digest(altered).hex()
            # Artifact locality requires real files in the isolated directory.
            for name in list(isolated.iterdir()):
                if name.is_symlink():
                    data = name.read_bytes()
                    name.unlink()
                    name.write_bytes(data)
            resource.reseal_resources(bad)
            reseal_font_paints(isolated, bad)
            reseal_image_paints(isolated, bad)
            reject(bad, label, isolated)
    rejected += display.tamper_checks(probe["display_probe"])
    return rejected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.directory.resolve()
    paths = sorted(root.glob("*.json"))
    check(paths, "actual column paint JSON probes required")
    totals, scopes, modes, missing_seen = Counter(), Counter(), Counter(), set()
    for path in paths:
        raw = path.read_bytes()
        probe = json.loads(raw)
        check(
            path.name == probe["case"] + "-" + digest(raw).hex() + ".json",
            "actual paint filename SHA",
        )
        try:
            result = verify(root, probe)
            if args.self_test:
                totals["tamper_rejections"] += tamper_checks(root, probe, missing_seen)
        except REJECTION_ERRORS as error:
            raise AssertionError(f"{path.name}: {error}") from error
        scopes[result.pop("scope")] += 1
        modes[result.pop("selection_mode")] += 1
        totals.update(result)
    print(
        json.dumps(
            dict(
                probes=len(paths),
                **totals,
                observation_scopes=dict(scopes),
                selection_modes=dict(modes),
                budget_evidence="exact font/text/vector/image reservations and work; raster deflater write suffix bounded",
                scope="typed source and physical paint contributions; no Column PDF acceptance, full SVG/shaping/layout replay or original CID-CFF positive gate",
                manual_scope="source_callback and retained_source_cut builders use reported source floors, without full paint-driver convergence or cumulative predecessor work",
            ),
            sort_keys=True,
        )
    )


if __name__ == "__main__":
    main()
