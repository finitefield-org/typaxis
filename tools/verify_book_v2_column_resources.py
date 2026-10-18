#!/usr/bin/env python3
"""Check actual Book /2 column resource diagnostics independently.

The enclosed display is checked by the existing column display verifier.
Original source glyphs determine font uses; original font/image bytes determine
closed glyphs, subset metrics, CID extraction and raster pixels. This is resource
evidence, without Column PDF acceptance or a replay of the whole layout engine.
"""
from __future__ import annotations

import argparse
import copy
import io
import json
import logging
import struct
import tempfile
import zlib
from collections import Counter
from pathlib import Path

from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.ttLib import TTFont
from PIL import Image

import verify_book_v2_column_display as display

logging.getLogger("fontTools.ttLib.tables._h_e_a_d").setLevel(logging.ERROR)


SCHEMA = "typaxis.book-2-column-resources-probe/1"
check = display.check
integer = display.integer
boolean = display.boolean
digest = display.digest
domain = display.domain
hash_bytes = display.hash_bytes
u = display.u
i = display.i
fold = display.fold


def fields(value, expected, label):
    check(type(value) is dict and set(value) == set(expected), label + " closed fields")
    return value


def strict_equal(actual, expected, label):
    check(type(actual) is type(expected), label + " primitive type")
    if isinstance(expected, dict):
        check(set(actual) == set(expected), label + " fields")
        for key in expected:
            strict_equal(actual[key], expected[key], label + "." + key)
    elif isinstance(expected, list):
        check(len(actual) == len(expected), label + " length")
        for index, (a, e) in enumerate(zip(actual, expected)):
            strict_equal(a, e, label + f"[{index}]")
    else:
        check(actual == expected, label + " value")


def artifact(root, name, expected_hash):
    check(type(name) is str and name and Path(name).name == name, "local artifact name")
    path = root / name
    check(
        path.is_file() and path.resolve().parent == root.resolve(),
        "artifact stays in probe directory",
    )
    data = path.read_bytes()
    check(
        digest(data) == hash_bytes(expected_hash), "original/program artifact SHA-256"
    )
    return data


def verify_artifact_fields(value):
    fields(value, ("file", "sha256", "length"), "artifact")
    hash_bytes(value["sha256"])
    integer(value["length"], 64, False)


def outline(font, gid):
    glyphs = font.getGlyphSet()
    pen = DecomposingRecordingPen(glyphs)
    glyphs[font.getGlyphOrder()[gid]].draw(pen)
    return pen.value


def trunc_div(numerator, denominator):
    check(denominator > 0, "positive font units")
    return (abs(numerator) // denominator) * (-1 if numerator < 0 else 1)


def font_tables(data, face_index):
    integer(face_index, 32, False)
    offset = 0
    if data[:4] == b"ttcf":
        count = int.from_bytes(data[8:12], "big")
        check(face_index < count, "font collection face index")
        offset = int.from_bytes(data[12 + 4 * face_index : 16 + 4 * face_index], "big")
    else:
        check(face_index == 0, "standalone face index")
    check(offset + 12 <= len(data), "SFNT header length")
    count = int.from_bytes(data[offset + 4 : offset + 6], "big")
    tables = {}
    for at in range(offset + 12, offset + 12 + count * 16, 16):
        check(at + 16 <= len(data), "SFNT directory length")
        tag, checksum, start, length = struct.unpack(">4sIII", data[at : at + 16])
        check(
            tag not in tables and start + length <= len(data),
            "SFNT unique bounded table",
        )
        tables[tag] = data[start : start + length]
    return tables


def glyph_components(glyph):
    if not glyph:
        return []
    check(len(glyph) >= 10, "TrueType glyph header")
    if int.from_bytes(glyph[:2], "big", signed=True) >= 0:
        return []
    result, at = [], 10
    while True:
        check(at + 4 <= len(glyph), "composite component header")
        flags, gid = struct.unpack(">HH", glyph[at : at + 4])
        result.append(gid)
        at += 4 + (4 if flags & 1 else 2)
        at += 2 if flags & 8 else 4 if flags & 64 else 8 if flags & 128 else 0
        check(at <= len(glyph), "composite component arguments")
        if not flags & 32:
            return result


def truetype_data(data, face_index):
    tables = font_tables(data, face_index)
    for name in (b"head", b"hhea", b"maxp", b"hmtx", b"loca", b"glyf"):
        check(name in tables, "required TrueType table")
    count = int.from_bytes(tables[b"maxp"][4:6], "big")
    short = int.from_bytes(tables[b"head"][50:52], "big", signed=True) == 0
    width = 2 if short else 4
    loca = tables[b"loca"]
    check(len(loca) >= (count + 1) * width, "original loca coverage")
    offsets = [
        int.from_bytes(loca[n * width : (n + 1) * width], "big") * (2 if short else 1)
        for n in range(count + 1)
    ]
    check(
        offsets == sorted(offsets) and offsets[-1] <= len(tables[b"glyf"]),
        "original glyph offsets",
    )
    glyphs = [tables[b"glyf"][a:b] for a, b in zip(offsets, offsets[1:])]
    components = [glyph_components(glyph) for glyph in glyphs]
    check(
        all(g < count for group in components for g in group),
        "composite original glyph coverage",
    )
    return tables, glyphs, components


def closed_glyphs(selected, components):
    closed, pending = set(selected) | {0}, sorted(set(selected) | {0})
    check(all(0 <= g < len(components) for g in closed), "selected original GID bounds")
    while pending:
        gid = pending.pop()
        for child in components[gid]:
            if child not in closed:
                closed.add(child)
                pending.append(child)
    return sorted(closed)


def metrics(tables):
    head, hhea = tables[b"head"], tables[b"hhea"]

    def s(data, at):
        return int.from_bytes(data[at : at + 2], "big", signed=True)

    units = int.from_bytes(head[18:20], "big")
    scale = lambda value: trunc_div(value * 1000, units)
    bbox = [scale(s(head, at)) for at in (36, 38, 40, 42)]
    if bbox[0] >= bbox[2] or bbox[1] >= bbox[3]:
        bbox = [0, -200, 1000, 800]
    ascent = scale(s(hhea, 4))
    os2, post = tables.get(b"OS/2", b""), tables.get(b"post", b"")
    cap = (
        scale(s(os2, 88))
        if len(os2) >= 90 and int.from_bytes(os2[:2], "big") >= 2
        else ascent
    )
    return dict(
        ascent_1000=ascent,
        descent_1000=scale(s(hhea, 6)),
        cap_height_1000=cap,
        stem_v_1000=80,
        italic_angle_fixed_16_16=(
            int.from_bytes(post[4:8], "big", signed=True) if len(post) >= 8 else 0
        ),
        flags=32,
        bbox_1000=bbox,
    )


class Budget:
    """Recompute charged successful prefixes for the bounded resource kernels."""

    def __init__(self, values):
        check(type(values) is list and len(values) == 3, "charge triple")
        self.records, self.spool, self.work = [integer(n, 64, False) for n in values]

    def charge(self, records=0, spool=0, work=0):
        self.records += records
        self.spool += spool
        self.work += work

    def step(self):
        self.work += 1

    def fold(self, previous, value):
        self.work += (32 + len(value) + 63) // 64
        return fold(previous, value)

    def triple(self):
        return [self.records, self.spool, self.work]

    def matches(self, stage):
        check(
            self.triple() == stage["charges"],
            f"exact resource reservation/work prefix: expected {self.triple()}, actual {stage['charges']}",
        )


def charged_sort(values, budget, key_work, key):
    values = list(values)

    def sift(root, end):
        while 2 * root + 1 < end:
            child = 2 * root + 1
            budget.step()
            if child + 1 < end:
                budget.charge(work=key_work)
                if key(values[child]) < key(values[child + 1]):
                    child += 1
            budget.charge(work=key_work)
            if key(values[root]) >= key(values[child]):
                break
            budget.step()
            values[root], values[child] = values[child], values[root]
            root = child

    for start in reversed(range(len(values) // 2)):
        sift(start, len(values))
    for end in reversed(range(1, len(values))):
        budget.step()
        values[0], values[end] = values[end], values[0]
        sift(0, end)
    return values


def derive_font_uses(probe):
    source = probe["display_probe"]
    origins = probe["original_font_origins"]
    expected_origin_paints = [
        n
        for n, (tag, index) in enumerate(source["paints"])
        if tag in ("text", "marker", "number")
        or tag == "math"
        and source["math"][index]["original"]["kind"] == "native"
    ]
    check(
        [v["paint"] for v in origins] == expected_origin_paints,
        "complete ordered original font origins",
    )
    origins = {value["paint"]: value for value in origins}
    uses, slots = [], []
    for paint, (tag, index) in enumerate(source["paints"]):
        if paint in origins:
            origin = fields(
                origins[paint], ("paint", "draw", "instance"), "original font origin"
            )
            check(origin["draw"] == [tag, index], "original font origin draw identity")
            instance = integer(origin["instance"], 32, False)
        else:
            instance = None
        values = []
        if tag == "text":
            draw = source["text"][index]
            line = source["original_fragments"][draw["fragment"]]["line"]
            original = next(
                v
                for v in line["items"]
                if v["kind"] == "text"
                and (v["run"], v["cluster"]) == (draw["run"], draw["cluster"])
            )
            values = [
                (
                    original["text"],
                    [g["gid"] for g in original["glyphs"]],
                    {"kind": "text", "span": original["source"]["span"]},
                    line["font"]["size"],
                )
            ]
        elif tag in ("marker", "number"):
            draw = source["markers" if tag == "marker" else "numbers"][index]
            original = draw["original"]
            for cluster in draw["clusters"]:
                # The strict display checker independently validates each local
                # run/cluster projection against the original glyph ranges.
                values.append(
                    (
                        cluster["text"],
                        [g[0] for g in cluster["glyphs"]],
                        {"kind": "text", "span": cluster["span"]},
                        original["font"]["size"],
                    )
                )
        elif tag == "math":
            draw = source["math"][index]
            original = draw["original"]
            if original["kind"] == "native":
                for slot, native in enumerate(original["paints"]):
                    if "gid" not in native:
                        values.append(None)
                        continue
                    values.append(
                        (
                            chr(native["unicode"]),
                            [native["gid"]],
                            dict(
                                kind="native",
                                owner=original["owner"],
                                receipt=original["fingerprint"],
                                computation=original["computation_fingerprint"],
                                paint_index=slot,
                                logical_ordinal=native["ordinal"],
                            ),
                            native["font_size"],
                        )
                    )
        slots.append(len(values))
        for slot, value in enumerate(values):
            if value is None:
                continue
            text, glyphs, origin_source, size = value
            check(
                glyphs and all(integer(g, 16, False) > 0 for g in glyphs),
                "real original nonzero glyph occurrence",
            )
            uses.append(
                dict(
                    paint=paint,
                    typed_paint=[tag, index],
                    slot=slot,
                    instance=instance,
                    source=origin_source,
                    text=text,
                    glyphs=glyphs,
                    size=size,
                )
            )
    return uses, slots


def verify_font_instances(probe, resources):
    original = fields(
        probe["original_font_instances"],
        ("fingerprint", "count", "instances"),
        "original font instance table",
    )
    instances = original["instances"]
    check(
        integer(original["count"], 64, False) == len(instances),
        "actual original table count",
    )
    faces = []
    for index, instance in enumerate(instances):
        fields(
            instance,
            ("id", "face_id", "face_index", "sha256", "table_fingerprint"),
            "original instance",
        )
        check(
            integer(instance["id"], 32, False) == index, "dense original instance IDs"
        )
        integer(instance["face_id"], 32, False)
        integer(instance["face_index"], 32, False)
        face = resources["fonts"][instance["face_id"]]
        check(
            instance["face_index"] == face["face_index"]
            and instance["sha256"] == face["sha256"],
            "original instance admitted face",
        )
        check(
            instance["table_fingerprint"] == original["fingerprint"],
            "original instance table binding",
        )
        faces.append(instance["face_id"])
    check(faces == sorted(set(faces)), "original table actual face ordering")
    canonical = display.canonical(
        dict(
            algorithm="typaxis.production-font-instances/3",
            faces=faces,
            ledger_fingerprint=probe["display_probe"]["admitted_fingerprint"],
        )
    )
    check(
        digest(canonical.encode()) == hash_bytes(original["fingerprint"]),
        "independent original font table canonical SHA",
    )
    return instances


def verify_font_selection(probe, resources, instances, budget):
    stage = fields(
        probe["fonts_selection"],
        ("fingerprint", "charges", "fonts", "uses"),
        "font selection",
    )
    uses, slots = derive_font_uses(probe)
    for origin in probe["original_font_origins"]:
        tag, index = origin["draw"]
        source = probe["display_probe"]
        if tag == "text":
            draw = source["text"][index]
            font = source["original_fragments"][draw["fragment"]]["line"]["font"]
        else:
            font = source[
                {"marker": "markers", "number": "numbers", "math": "math"}[tag]
            ][index]["original"]
            font = font if tag == "math" else font["font"]
        instance = instances[origin["instance"]]
        check(
            instance["face_id"] == font["face"]
            and instance["sha256"] == font["font_sha256" if tag == "math" else "sha256"]
            and instance["face_index"]
            == font["face_index" if tag == "math" else "index"],
            "original font origin exact source face binding",
        )
    strict_equal(
        stage["uses"], uses, "all font uses from original actual display source"
    )
    glyphs = [(use["instance"], gid) for use in uses for gid in use["glyphs"]]
    capacity = min(len(glyphs), len(instances))
    sizes = probe["size_of"]
    budget.charge(work=len(slots) + sum(slots))
    budget.charge(
        len(glyphs) + capacity + len(uses) + 1,
        len(glyphs) * sizes["font_glyph"]
        + capacity * sizes["selected_font"]
        + len(uses) * sizes["selected_font_use"],
    )
    budget.step()
    fp = budget.fold(
        domain("typaxis.book-2-font-selection/1"),
        hash_bytes(probe["display_probe"]["fingerprint"]),
    )
    fp = budget.fold(fp, hash_bytes(probe["display_probe"]["admitted_fingerprint"]))
    budget.charge(work=len(slots) + sum(slots))
    for use in uses:
        budget.charge(work=len(use["glyphs"]))
        fp = budget.fold(fp, u(use["paint"]) + u(use["slot"]) + u(use["instance"], 4))
    ordered = charged_sort(glyphs, budget, 1, lambda value: value)
    expected = {}
    for instance, gid in ordered:
        budget.step()
        if gid in expected.get(instance, []):
            continue
        if instance not in expected:
            original = instances[instance]
            fp = budget.fold(fp, hash_bytes(original["sha256"]))
            fp = budget.fold(fp, hash_bytes(original["table_fingerprint"]))
            fp = budget.fold(fp, u(instance, 4))
            expected[instance] = []
        expected[instance].append(gid)
        fp = budget.fold(fp, u(gid, 2))
    check(
        len(stage["fonts"]) == len(expected), "complete unique selected font instances"
    )
    for font, (instance, selected) in zip(stage["fonts"], expected.items()):
        fields(
            font,
            (
                "instance",
                "table_fingerprint",
                "face_id",
                "face_index",
                "source",
                "selected",
                "units_per_em",
                "glyph_count",
            ),
            "selected font",
        )
        original = instances[instance]
        check(
            integer(font["instance"], 32, False) == instance,
            "selected original instance",
        )
        strict_equal(font["selected"], selected, "unique sorted original GID union")
        for name in ("table_fingerprint", "face_id", "face_index"):
            check(
                font[name] == original[name], "selected font original instance identity"
            )
        check(
            font["source"]["sha256"] == original["sha256"],
            "selected original font byte identity",
        )
    check(fp == hash_bytes(stage["fingerprint"]), "font selection canonical fold")
    budget.matches(stage)
    return uses


def closure_charge(budget, selected, tables, components, usize_size):
    count, initial = len(components), len(selected) + 1
    budget.charge(len(tables), len(tables) * 128 + 512, len(tables) * 128)
    budget.charge(count + 1, (count + 1) * usize_size, count + 1)
    budget.charge(initial, initial * 128 + 512, initial * 128)
    closed = set(selected) | {0}
    pending = sorted(closed)
    capacity = len(pending)
    budget.charge(len(closed), len(closed) * 2, len(closed))
    while pending:
        gid = pending.pop()
        budget.step()
        for child in components[gid]:
            budget.charge(work=128)
            if child not in closed:
                budget.charge(1, 128, 128)
                if len(pending) == capacity:
                    next_capacity = min(2 * capacity + 1, count)
                    budget.charge(
                        next_capacity - capacity, 2 * (next_capacity - capacity), 1
                    )
                    capacity = next_capacity
                closed.add(child)
                pending.append(child)
    budget.charge(len(closed), len(closed) * 128 + 512, len(closed) * 128)


def verify_closures(root, probe, budget):
    stage = fields(
        probe["closures"], ("fingerprint", "charges", "fonts"), "font closures"
    )
    selection = probe["fonts_selection"]
    sizes = probe["size_of"]
    count = len(selection["fonts"])
    check(len(stage["fonts"]) == count, "one closure for every selected font")
    budget.charge(count + 1, count * sizes["closed_font"], 1)
    fp = budget.fold(
        domain("typaxis.book-2-font-closures/1"), hash_bytes(selection["fingerprint"])
    )
    sources = []
    for selected, closure in zip(selection["fonts"], stage["fonts"]):
        fields(closure, ("fingerprint", "kind", "mapping"), "closed font")
        # CID-keyed CFF positive evidence requires an admitted original fixture;
        # name-keyed diagnostic fixtures cannot establish that acceptance gate.
        check(
            closure["kind"] == "TrueType", "unsupported original CID-CFF positive gate"
        )
        data = artifact(root, selected["source"]["file"], selected["source"]["sha256"])
        check(
            len(data) == selected["source"]["length"], "original font artifact length"
        )
        tables, glyphs, components = truetype_data(data, selected["face_index"])
        check(
            selected["glyph_count"] == len(glyphs)
            and selected["units_per_em"]
            == int.from_bytes(tables[b"head"][18:20], "big"),
            "admitted source font metrics",
        )
        closed = closed_glyphs(selected["selected"], components)
        expected_map = [[gid, dense] for dense, gid in enumerate(closed)]
        strict_equal(
            closure["mapping"],
            expected_map,
            "original composite closure dense subset mapping",
        )
        n = len(selected["selected"])
        budget.charge(n + 1, n * 128 + 512, n * 128 + 1)
        closure_charge(budget, selected["selected"], tables, components, sizes["usize"])
        current = budget.fold(
            hash_bytes(selection["fingerprint"]),
            hash_bytes(selected["source"]["sha256"]),
        )
        current = budget.fold(current, u(selected["instance"], 4))
        current = budget.fold(current, bytes([0]))
        for gid, dense in expected_map:
            budget.charge(work=129)
            current = budget.fold(current, u(gid, 2) + u(dense, 2))
        check(
            current == hash_bytes(closure["fingerprint"]), "closed font canonical fold"
        )
        fp = budget.fold(fp, current)
        sources.append(
            dict(
                data=data,
                tables=tables,
                glyphs=glyphs,
                components=components,
                selected=selected,
                closed=closed,
            )
        )
    check(
        fp == hash_bytes(stage["fingerprint"]), "font closures aggregate canonical fold"
    )
    budget.matches(stage)
    return sources


def lookup_work(count):
    return count.bit_length() + 1


def verify_cids(probe, uses, sources, budget):
    stage = fields(
        probe["cids"], ("fingerprint", "charges", "fonts", "uses"), "CID plans"
    )
    nfonts, nuses = len(sources), len(uses)
    unique_glyphs = sum(len(source["selected"]["selected"]) for source in sources)
    occurrences = sum(len(use["glyphs"]) for use in uses)
    sizes = probe["size_of"]
    budget.charge(work=nfonts + nuses)
    budget.charge(
        nfonts + unique_glyphs + nuses + occurrences + 1,
        nfonts * sizes["cid_font"]
        + unique_glyphs * sizes["binding"]
        + nuses * sizes["cid_use"]
        + occurrences * sizes["cid"],
        1,
    )
    claims = {}
    instance_indexes = {
        source["selected"]["instance"]: index for index, source in enumerate(sources)
    }
    for use in uses:
        budget.charge(work=9)
        if len(use["glyphs"]) != 1 or len(use["text"]) != 1:
            continue
        font = instance_indexes[use["instance"]]
        budget.charge(
            work=lookup_work(nfonts)
            + lookup_work(len(sources[font]["selected"]["selected"]))
        )
        claims.setdefault((font, use["glyphs"][0]), set()).add(ord(use["text"]))
    expected_fonts = []
    budget.charge(work=256 * unique_glyphs)
    fp = budget.fold(
        domain("typaxis.book-2-cids/1"), hash_bytes(probe["programs"]["fingerprint"])
    )
    for index, source in enumerate(sources):
        selected = source["selected"]
        bindings = []
        mapping = {gid: dense for dense, gid in enumerate(source["closed"])}
        fp = budget.fold(fp, u(selected["instance"], 4))
        for offset, gid in enumerate(selected["selected"]):
            choices = claims.get((index, gid), set())
            unicode = next(iter(choices)) if len(choices) == 1 else None
            width = (
                source["advances"][gid] * 1000 + selected["units_per_em"] // 2
            ) // selected["units_per_em"]
            binding = dict(
                gid=gid,
                cid=offset + 1,
                subset=mapping[gid],
                width=width,
                unicode=unicode,
            )
            bindings.append(binding)
            encoded = u(gid, 2) + u(offset + 1, 2) + u(mapping[gid], 2) + u(width, 4)
            encoded += bytes([int(unicode is not None)]) + u(unicode or 0, 4)
            fp = budget.fold(fp, encoded)
        expected_fonts.append(dict(instance=selected["instance"], bindings=bindings))
    strict_equal(
        stage["fonts"],
        expected_fonts,
        "exact unique CID bindings widths and original Unicode observations",
    )
    expected_uses = []
    for index, use in enumerate(uses):
        font = instance_indexes[use["instance"]]
        bindings = {value["gid"]: value for value in expected_fonts[font]["bindings"]}
        budget.charge(work=lookup_work(nfonts))
        selected_cids = []
        extracted = ""
        for gid in use["glyphs"]:
            budget.charge(work=lookup_work(len(bindings)) + 5)
            binding = bindings[gid]
            selected_cids.append(binding["cid"])
            if binding["unicode"] is not None:
                extracted += chr(binding["unicode"])
            fp = budget.fold(fp, u(binding["cid"], 2))
        budget.charge(work=4)
        requires = extracted != use["text"]
        fp = budget.fold(
            fp, u(index) + u(font) + u(len(selected_cids)) + bytes([int(requires)])
        )
        expected_uses.append(
            dict(
                paint=use["paint"],
                slot=use["slot"],
                font=font,
                cids=selected_cids,
                requires_actual_text=requires,
                actual_text=use["text"] if requires else None,
            )
        )
    strict_equal(
        stage["uses"],
        expected_uses,
        "every original occurrence CID sequence and exact ActualText",
    )
    check(fp == hash_bytes(stage["fingerprint"]), "CID plan canonical fold")
    budget.matches(stage)


MEDIA_KIND = {"png": 0, "jpeg-baseline": 1, "svg-safe": 2, "svg-safe-2": 3}


def image_identity(original, resources, vector):
    image = resources["images"][original["image"]]
    media, source_hash = image["media"], image["sha256"]
    check(
        source_hash == original["source_sha256"], "original image ledger SHA identity"
    )
    kind = MEDIA_KIND[media]
    if kind < 2:
        key = [kind, source_hash, "", "", bytes(32).hex()]
    else:
        parser = "typaxis.safe-svg-parser/" + ("2" if kind == 3 else "1")
        ir = "typaxis.safe-vector-ir/" + ("2" if kind == 3 else "1")
        fingerprint = vector["ir_fingerprint"]
        if "binding" in original:
            bound = original["binding"]["resource"]
            check(
                (bound["parser_id"], bound["ir_id"], bound["ir_fingerprint"])
                == (parser, ir, fingerprint),
                "original admitted vector parser identity",
            )
        key = [kind, source_hash, parser, ir, fingerprint]
    return key


def verify_original_images(root, probe, resources):
    values = probe["original_images"]
    check(
        len(values) == len(resources["images"]),
        "complete actual original admitted image views",
    )
    identities = {}
    for index, image in enumerate(values):
        fields(
            image,
            (
                "id",
                "media",
                "sha256",
                "source",
                "width",
                "height",
                "decoded_bytes",
                "key",
            ),
            "original admitted image",
        )
        check(
            integer(image["id"], 32, False) == index,
            "dense original admitted image IDs",
        )
        admitted = resources["images"][index]
        check(
            image["media"] == admitted["media"]
            and image["sha256"] == admitted["sha256"],
            "original image sealed admitted identity",
        )
        verify_artifact_fields(image["source"])
        check(
            image["source"]["sha256"] == image["sha256"],
            "original image payload SHA identity",
        )
        source = artifact(root, image["source"]["file"], image["sha256"])
        check(len(source) == image["source"]["length"], "original image payload length")
        check(
            all(
                integer(image[name], 64, False) > 0
                for name in ("width", "height", "decoded_bytes")
            ),
            "positive original admitted image metadata",
        )
        kind = MEDIA_KIND[image["media"]]
        key = image["key"]
        check(
            type(key) is list
            and len(key) == 5
            and integer(key[0], 8, False) == kind
            and key[1] == image["sha256"],
            "original image key media/source binding",
        )
        hash_bytes(key[4])
        if kind < 2:
            strict_equal(
                key[2:], ["", "", bytes(32).hex()], "original raster identity key"
            )
            with Image.open(io.BytesIO(source)) as actual:
                actual.load()
                strict_equal(
                    [image["width"], image["height"]],
                    list(actual.size),
                    "original raster byte dimensions",
                )
                if kind == 0:
                    check(source[:8] == b"\x89PNG\r\n\x1a\n", "original PNG signature")
                    depth, color = source[24], source[25]
                    channels = 2 if color in (0, 4) else 4
                    estimate = (
                        image["width"]
                        * image["height"]
                        * channels
                        * (2 if depth == 16 else 1)
                    )
                else:
                    estimate = (
                        image["width"]
                        * image["height"]
                        * (1 if actual.mode == "L" else 3)
                    )
                check(
                    image["decoded_bytes"] == estimate,
                    "original raster decoded buffer estimate",
                )
        else:
            version = "2" if kind == 3 else "1"
            check(
                key[2] == "typaxis.safe-svg-parser/" + version
                and key[3] == "typaxis.safe-vector-ir/" + version,
                "original admitted vector algorithm identities",
            )
            # Safe vector resources use a fixed image carrier dimension. Their
            # IR allocation charge is an actual admission input diagnostic;
            # independently replaying the SVG parser is outside this checker.
            strict_equal(
                [image["width"], image["height"]],
                [1, 1],
                "original vector carrier dimensions",
            )
        metadata = (image["width"], image["height"], image["decoded_bytes"])
        key_tuple = tuple(image["key"])
        if key_tuple in identities:
            check(
                metadata == identities[key_tuple],
                "equal original payload aliases have equal admitted metadata",
            )
        identities[key_tuple] = metadata
    resources["original_images"] = values


def image_key_order(key):
    return (key[0], hash_bytes(key[1]), key[2], key[3], hash_bytes(key[4]))


def derive_image_uses(probe, resources):
    source = probe["display_probe"]
    uses = []
    for paint, (tag, index) in enumerate(source["paints"]):
        if tag == "image":
            draw = source["images"][index]
            original, vector = draw["original"], draw["vector"]
            page = source["original_fragments"][draw["fragment"]]["page"]
            viewport = draw["viewport"]
        elif tag == "math" and source["math"][index]["original"]["kind"] == "vector":
            draw = source["math"][index]
            original, vector = draw["original"], draw["paint"]
            page, viewport = draw["page"], vector["viewport"]
        else:
            continue
        key = image_identity(original, resources, vector)
        if vector is None:
            geometry = dict(kind="raster", viewport=viewport)
        else:
            geometry = dict(
                kind="vector",
                viewport=viewport,
                scale=vector["scale"],
                matrix=vector["matrix"],
                color=vector["color"],
                key=dict(
                    source_sha256=key[1],
                    parser=key[2],
                    ir=key[3],
                    ir_fingerprint=key[4],
                ),
            )
        uses.append(
            dict(
                paint=paint,
                typed_paint=[tag, index],
                resource=None,
                image_id=original["image"],
                owner=original["owner"],
                page=page,
                geometry=geometry,
                key=key,
            )
        )
    return uses


def verify_images(root, probe, resources, budget):
    stage = fields(
        probe["images_selection"],
        ("fingerprint", "charges", "prior", "images", "uses"),
        "image selection",
    )
    expected_uses = derive_image_uses(probe, resources)
    count = len(expected_uses)
    check(
        stage["prior"] == [budget.records - 1, budget.spool, budget.work],
        "image selection actual prior constructor",
    )
    budget.charge(work=len(probe["display_probe"]["paints"]))
    budget.charge(
        2 * count + 1,
        count
        * (probe["size_of"]["selected_image"] + probe["size_of"]["selected_image_use"]),
        1,
    )
    budget.charge(work=len(probe["display_probe"]["paints"]))
    ordered = charged_sort(
        expected_uses,
        budget,
        4,
        lambda value: (image_key_order(value["key"]), value["image_id"]),
    )
    source_lengths = {
        tuple(image["key"]): len(
            artifact(root, image["source"]["file"], image["source"]["sha256"])
        )
        for image in stage["images"]
    }
    expected_images = []
    for use in ordered:
        budget.step()
        if expected_images and expected_images[-1]["key"] == use["key"]:
            budget.charge(work=(source_lengths[tuple(use["key"])] + 63) // 64 + 1)
            continue
        expected_images.append(dict(id=use["image_id"], key=use["key"]))
    check(
        len(stage["images"]) == len(expected_images),
        "deduplicate payloads without deduplicating original occurrences",
    )
    fp = budget.fold(
        domain("typaxis.book-2-image-selection/1"),
        hash_bytes(probe["display_probe"]["fingerprint"]),
    )
    fp = budget.fold(fp, hash_bytes(probe["display_probe"]["admitted_fingerprint"]))
    for expected, image in zip(expected_images, stage["images"]):
        fields(
            image,
            ("id", "media", "key", "source", "width", "height", "decoded_bytes"),
            "selected image",
        )
        check(
            (image["id"], image["key"]) == (expected["id"], expected["key"]),
            "canonical payload representative/key",
        )
        admitted = resources["images"][expected["id"]]
        check(
            image["media"] == admitted["media"]
            and image["source"]["sha256"] == admitted["sha256"],
            "selected original admitted image",
        )
        original = resources["original_images"][expected["id"]]
        for name in ("width", "height", "decoded_bytes", "key"):
            strict_equal(
                image[name],
                original[name],
                "selected image original admitted metadata." + name,
            )
        data = artifact(root, image["source"]["file"], image["source"]["sha256"])
        check(len(data) == image["source"]["length"], "actual image source byte length")
        integer(image["width"], 32, False)
        integer(image["height"], 32, False)
        integer(image["decoded_bytes"], 64, False)
        if MEDIA_KIND[image["media"]] < 2:
            with Image.open(io.BytesIO(data)) as original:
                original.load()
                check(
                    original.size == (image["width"], image["height"]),
                    "original raster dimensions",
                )
        budget.step()
        key = expected["key"]
        for value in (
            bytes([key[0]]),
            u(image["id"], 4),
            hash_bytes(key[1]),
            key[2].encode(),
            key[3].encode(),
            hash_bytes(key[4]),
        ):
            fp = budget.fold(fp, value)
    indexes = {
        tuple(image["key"]): index for index, image in enumerate(expected_images)
    }
    for use in expected_uses:
        use["resource"] = indexes[tuple(use.pop("key"))]
        budget.charge(work=len(expected_images).bit_length() + 1)
        fp = budget.fold(
            fp, u(use["paint"]) + u(use["resource"]) + u(use["image_id"], 4)
        )
    strict_equal(
        stage["uses"],
        expected_uses,
        "every typed original image occurrence owner/page/geometry",
    )
    check(fp == hash_bytes(stage["fingerprint"]), "image selection canonical fold")
    budget.matches(stage)
    return expected_uses


def verify_rasters(root, probe, budget):
    stage = fields(
        probe["rasters"], ("fingerprint", "charges", "programs"), "raster programs"
    )
    images = probe["images_selection"]["images"]
    check(
        len(stage["programs"]) == len(images), "raster program option for each payload"
    )
    budget.charge(len(images) + 1, len(images) * probe["size_of"]["raster_slot"], 1)
    fp = budget.fold(
        domain("typaxis.book-2-raster-programs/1"),
        hash_bytes(probe["images_selection"]["fingerprint"]),
    )
    png = jpeg = alpha_count = 0
    output_lengths = []
    for index, (image, program) in enumerate(zip(images, stage["programs"])):
        budget.step()
        if MEDIA_KIND[image["media"]] >= 2:
            check(program is None, "vector resource has no raster program")
            continue
        fields(
            program,
            (
                "fingerprint",
                "program",
                "sha256",
                "width",
                "height",
                "color",
                "encoding",
                "alpha",
                "jpeg",
            ),
            "raster program",
        )
        data = artifact(root, program["program"]["file"], program["program"]["sha256"])
        source = artifact(root, image["source"]["file"], image["source"]["sha256"])
        check(
            len(data) == program["program"]["length"]
            and program["sha256"] == program["program"]["sha256"],
            "actual raster program bytes/SHA",
        )
        check(
            (program["width"], program["height"]) == (image["width"], image["height"]),
            "raster original dimensions",
        )
        with Image.open(io.BytesIO(source)) as original:
            original.load()
            if image["media"] == "jpeg-baseline":
                jpeg += 1
                check(
                    program["encoding"] == "Jpeg" and program["alpha"] is None,
                    "actual JPEG raster encoding",
                )
                attestation = fields(
                    program["jpeg"],
                    (
                        "source_sha256",
                        "normalized_sha256",
                        "width",
                        "height",
                        "decoded_bytes",
                        "color",
                    ),
                    "JPEG attestation",
                )
                check(
                    attestation["source_sha256"] == image["source"]["sha256"]
                    and attestation["normalized_sha256"] == program["sha256"]
                    and (
                        attestation["width"],
                        attestation["height"],
                        attestation["decoded_bytes"],
                    )
                    == (image["width"], image["height"], image["decoded_bytes"]),
                    "JPEG original/normalized attestation",
                )
                check(
                    program["color"]
                    == ("Gray" if attestation["color"] == "Grayscale" else "Rgb")
                    and attestation["color"] in ("Grayscale", "YCbCr"),
                    "JPEG actual color model",
                )
                with Image.open(io.BytesIO(data)) as normalized:
                    normalized.load()
                    check(
                        normalized.size == original.size
                        and normalized.convert("RGB").tobytes()
                        == original.convert("RGB").tobytes(),
                        "JPEG normalized original pixels",
                    )
                budget.charge(1, 0, (len(data) + 63) // 64 + 1)
            else:
                png += 1
                check(
                    program["encoding"] == "Flate" and program["jpeg"] is None,
                    "actual PNG PDF encoding",
                )
                check(program["color"] in ("Gray", "Rgb"), "PNG admitted color model")
                mode = "L" if program["color"] == "Gray" else "RGB"
                pixels = original.convert(mode).tobytes()
                check(
                    zlib.decompress(data) == pixels, "PNG original color pixel payload"
                )
                opacity = original.convert("RGBA").getchannel("A").tobytes()
                has_alpha = any(value != 255 for value in opacity)
                check(
                    (program["alpha"] is not None) == has_alpha,
                    "PNG original alpha presence",
                )
                decoded = image["decoded_bytes"]
                decoder = decoded + len(source) + 65536
                budget.charge(8, decoded * 2 + decoder, decoded + len(source) + 65536)
                budget.charge(1, 1048576, len(pixels) * 128 + 65536)
                output_lengths.append(len(data))
                if has_alpha:
                    alpha_count += 1
                    alpha = fields(
                        program["alpha"],
                        ("program", "width", "height", "encoding"),
                        "alpha program",
                    )
                    check(
                        (alpha["width"], alpha["height"], alpha["encoding"])
                        == (image["width"], image["height"], "Flate"),
                        "original alpha dimensions/encoding",
                    )
                    alpha_data = artifact(
                        root, alpha["program"]["file"], alpha["program"]["sha256"]
                    )
                    check(
                        len(alpha_data) == alpha["program"]["length"]
                        and zlib.decompress(alpha_data) == opacity,
                        "PNG original alpha mask bytes",
                    )
                    budget.charge(1, 1048576, len(opacity) * 128 + 65536)
                    output_lengths.append(len(alpha_data))
        budget.charge(work=(len(data) + 63) // 64 + 1)
        current = budget.fold(
            domain("typaxis.book-2-raster-programs/1"),
            hash_bytes(probe["images_selection"]["fingerprint"]),
        )
        current = budget.fold(current, u(index))
        current = budget.fold(current, digest(data))
        if program["alpha"] is not None:
            budget.charge(work=(program["alpha"]["program"]["length"] + 63) // 64 + 1)
            current = budget.fold(
                current, hash_bytes(program["alpha"]["program"]["sha256"])
            )
        check(
            current == hash_bytes(program["fingerprint"]),
            "actual raster payload canonical fold",
        )
        fp = budget.fold(fp, current)
    check(
        fp == hash_bytes(stage["fingerprint"]),
        "raster programs aggregate canonical fold",
    )
    # The output sink allocates once per backend write. Its chunk boundaries
    # are not retained by the receipt: check exact bytes and the mathematically
    # bounded record/work suffix, without inventing a codec replay certificate.
    reported = Budget(stage["charges"])
    check(
        reported.spool == budget.spool + sum(output_lengths),
        "exact raster output spool reservation",
    )
    writes = reported.records - budget.records
    output_work = reported.work - budget.work
    if output_lengths:
        check(
            len(output_lengths) <= writes <= sum(output_lengths),
            "bounded actual deflater allocation records",
        )
        low = sum((length + 63) // 64 for length in output_lengths) + writes
        high = low + writes - len(output_lengths)
        check(low <= output_work <= high, "bounded actual deflater output work")
    else:
        check(writes == output_work == 0, "zero raster output writes")
    budget.records, budget.spool, budget.work = reported.triple()
    return png, jpeg, alpha_count


SIZE_FIELDS = (
    "font_glyph",
    "sfnt_rewrite_table",
    "usize",
    "selected_font",
    "selected_font_use",
    "closed_font",
    "font_program",
    "cid_font",
    "binding",
    "cid_use",
    "cid",
    "selected_image",
    "selected_image_use",
    "raster_slot",
)


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
        ),
        "resource probe",
    )
    check(
        probe["wire"] == SCHEMA and probe["case"] == probe["display_probe"]["case"],
        "resource schema and actual display case",
    )
    display.verify(probe["display_probe"])
    source = display.decode_source(probe["display_probe"])
    resources = display.verify_resources(probe["display_probe"], source)
    fields(probe["size_of"], SIZE_FIELDS, "actual reservation units")
    for value in probe["size_of"].values():
        check(
            integer(value, 64, False) > 0, "positive actual resource reservation unit"
        )
    check(
        probe["size_of"]["usize"] in (4, 8) and probe["size_of"]["cid"] == 2,
        "platform pointer and CID reservation units",
    )
    for stage_name in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
    ):
        stage = probe[stage_name]
        hash_bytes(stage["fingerprint"])
        Budget(stage["charges"])
    # Artifact paths are a closed, local content source. No absolute fixture
    # path or an externally supplied font can silently override these bytes.
    for font in probe["fonts_selection"]["fonts"]:
        verify_artifact_fields(font["source"])
    for font in probe["programs"]["fonts"]:
        verify_artifact_fields(font["program"])
    for image in probe["images_selection"]["images"]:
        verify_artifact_fields(image["source"])
    for program in probe["rasters"]["programs"]:
        if program is not None:
            verify_artifact_fields(program["program"])
            if program["alpha"] is not None:
                verify_artifact_fields(program["alpha"]["program"])
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
        "resource observation",
    )
    scope = observed["scope"]
    check(
        scope in ("resource_driver", "source_callback", "retained_source_cut"),
        "explicit resource observation scope",
    )
    maximum = integer(observed["maximum_work"], 64, False)
    check(
        maximum == probe["display_probe"]["observed"]["maximum_work"],
        "same cumulative work cap",
    )
    constructors = observed["constructors"]
    check(
        type(constructors) is list and len(constructors) == 2,
        "both actual resource constructor prefixes",
    )
    for constructor in constructors:
        Budget(constructor)
    source_observed = probe["display_probe"]["observed"]
    if scope == "resource_driver":
        check(
            probe["display_probe"]["selection_mode"] == "automatic_driver"
            and source_observed["scope"] == "display_callback",
            "natural resource driver display lineage",
        )
        before, after = Budget(observed["caller_before"]), Budget(
            observed["caller_after"]
        )
        check(
            all(a <= b for a, b in zip(before.triple(), after.triple())),
            "caller cumulative prefix",
        )
        check(
            observed["ancestor_callback"] == observed["caller_after"],
            "resource callback actual observation",
        )
        # Source spool includes accepted prior source workspaces; the resource
        # constructor keeps the complete display and all accepted caller floors.
        check(
            constructors[0][0] == source_observed["display_records"] + 1
            and constructors[0][1] == max(source_observed["source_spool"], before.spool)
            and constructors[0][2] == source_observed["display_work"],
            "actual resource driver constructor floors",
        )
    else:
        check(
            observed["caller_before"] is None and observed["caller_after"] is None,
            "manual resource chain cannot invent resource driver observations",
        )
        check(
            scope == source_observed["scope"],
            "actual source callback or retained-cut scope",
        )
        check(
            (observed["ancestor_callback"] is None) == (scope == "retained_source_cut"),
            "retained cut has no invented ancestor callback",
        )
        check(
            constructors[0]
            == [
                source_observed["display_records"] + 1,
                source_observed["source_spool"],
                source_observed["display_work"],
            ],
            "direct builder exact constructor prefix",
        )
    instances = verify_font_instances(probe, resources)
    verify_original_images(root, probe, resources)
    budget = Budget(constructors[0])
    uses = verify_font_selection(probe, resources, instances, budget)
    sources = verify_closures(root, probe, budget)
    verify_programs(root, probe, sources, budget)
    verify_cids(probe, uses, sources, budget)
    budget.charge(records=1)
    strict_equal(
        constructors[1], budget.triple(), "next resource constructor accepted prefix"
    )
    image_uses = verify_images(root, probe, resources, budget)
    rasters = verify_rasters(root, probe, budget)
    base = resources["limits"]["base"]
    extension = resources["limits"]["extension"]
    for triple in [
        *constructors,
        *[
            probe[name]["charges"]
            for name in (
                "fonts_selection",
                "closures",
                "programs",
                "cids",
                "images_selection",
                "rasters",
            )
        ],
    ]:
        records, spool, work = Budget(triple).triple()
        check(
            records <= base["max_fragments"]
            and spool <= base["max_spool_bytes"]
            and work <= maximum,
            "inclusive resource stage caps",
        )
    for font, program in zip(
        probe["fonts_selection"]["fonts"], probe["programs"]["fonts"]
    ):
        check(
            len(font["selected"]) <= base["max_cids_per_font"]
            and program["program"]["length"] <= extension["max_font_subset_bytes"],
            "actual CID/subset byte caps",
        )
    if scope == "resource_driver":
        strict_equal(
            observed["caller_after"],
            budget.triple(),
            "caller retains complete resource callback prefix",
        )
    return dict(
        pages=len(probe["display_probe"]["pages"]),
        paints=len(probe["display_probe"]["paints"]),
        fonts=len(sources),
        closed_glyphs=sum(len(s["closed"]) for s in sources),
        font_uses=len(uses),
        image_resources=len(probe["images_selection"]["images"]),
        image_uses=len(image_uses),
        png=rasters[0],
        jpeg=rasters[1],
        alpha=rasters[2],
        scope=scope,
        selection_mode=probe["display_probe"]["selection_mode"],
    )


def reseal_resources(probe):
    """Reseal output folds for negative checks; never replace original bytes."""
    selection = probe["fonts_selection"]
    fp = fold(
        domain("typaxis.book-2-font-selection/1"),
        hash_bytes(probe["display_probe"]["fingerprint"]),
    )
    fp = fold(fp, hash_bytes(probe["display_probe"]["admitted_fingerprint"]))
    for use in selection["uses"]:
        fp = fold(fp, u(use["paint"]) + u(use["slot"]) + u(use["instance"], 4))
    for font in selection["fonts"]:
        for value in (
            hash_bytes(font["source"]["sha256"]),
            hash_bytes(font["table_fingerprint"]),
            u(font["instance"], 4),
        ):
            fp = fold(fp, value)
        for gid in font["selected"]:
            fp = fold(fp, u(gid, 2))
    selection["fingerprint"] = fp.hex()
    fp = fold(domain("typaxis.book-2-font-closures/1"), fp)
    for selected, closure in zip(selection["fonts"], probe["closures"]["fonts"]):
        leaf = fold(
            hash_bytes(selection["fingerprint"]),
            hash_bytes(selected["source"]["sha256"]),
        )
        leaf = fold(leaf, u(selected["instance"], 4))
        leaf = fold(leaf, bytes([int(closure["kind"] != "TrueType")]))
        for gid, dense in closure["mapping"]:
            leaf = fold(leaf, u(gid, 2) + u(dense, 2))
        closure["fingerprint"] = leaf.hex()
        fp = fold(fp, leaf)
    probe["closures"]["fingerprint"] = fp.hex()
    fp = fold(domain("typaxis.book-2-font-programs/1"), fp)
    for closure, program in zip(probe["closures"]["fonts"], probe["programs"]["fonts"]):
        leaf = fold(
            domain("typaxis.book-2-truetype-subset/1"),
            hash_bytes(closure["fingerprint"]),
        )
        leaf = fold(leaf, hash_bytes(program["sha256"]))
        leaf = fold(leaf, u(program["program"]["length"]))
        leaf = fold(leaf, program["postscript_name"].encode())
        program["fingerprint"] = leaf.hex()
        fp = fold(fp, leaf)
    probe["programs"]["fingerprint"] = fp.hex()
    fp = fold(domain("typaxis.book-2-cids/1"), fp)
    for font in probe["cids"]["fonts"]:
        fp = fold(fp, u(font["instance"], 4))
        for binding in font["bindings"]:
            value = binding["unicode"]
            encoded = (
                u(binding["gid"], 2)
                + u(binding["cid"], 2)
                + u(binding["subset"], 2)
                + u(binding["width"], 4)
            )
            fp = fold(fp, encoded + bytes([int(value is not None)]) + u(value or 0, 4))
    for index, use in enumerate(probe["cids"]["uses"]):
        for cid in use["cids"]:
            fp = fold(fp, u(cid, 2))
        fp = fold(
            fp,
            u(index)
            + u(use["font"])
            + u(len(use["cids"]))
            + bytes([int(use["requires_actual_text"])]),
        )
    probe["cids"]["fingerprint"] = fp.hex()
    images = probe["images_selection"]
    fp = fold(
        domain("typaxis.book-2-image-selection/1"),
        hash_bytes(probe["display_probe"]["fingerprint"]),
    )
    fp = fold(fp, hash_bytes(probe["display_probe"]["admitted_fingerprint"]))
    for image in images["images"]:
        key = image["key"]
        for value in (
            bytes([key[0]]),
            u(image["id"], 4),
            hash_bytes(key[1]),
            key[2].encode(),
            key[3].encode(),
            hash_bytes(key[4]),
        ):
            fp = fold(fp, value)
    for use in images["uses"]:
        fp = fold(fp, u(use["paint"]) + u(use["resource"]) + u(use["image_id"], 4))
    images["fingerprint"] = fp.hex()
    fp = fold(domain("typaxis.book-2-raster-programs/1"), fp)
    for index, program in enumerate(probe["rasters"]["programs"]):
        if program is None:
            continue
        leaf = fold(
            domain("typaxis.book-2-raster-programs/1"),
            hash_bytes(images["fingerprint"]),
        )
        leaf = fold(leaf, u(index))
        leaf = fold(leaf, hash_bytes(program["sha256"]))
        if program["alpha"] is not None:
            leaf = fold(leaf, hash_bytes(program["alpha"]["program"]["sha256"]))
        program["fingerprint"] = leaf.hex()
        fp = fold(fp, leaf)
    probe["rasters"]["fingerprint"] = fp.hex()


def schema_paths(value, prefix=()):
    """One representative of every concrete nested object schema."""
    if isinstance(value, dict):
        for key, item in value.items():
            if prefix == () and key == "display_probe":
                # Its exact field/type/owner negatives remain in the imported
                # original display checker; missing enclosure is still tested.
                yield prefix + (key,)
                continue
            yield prefix + (key,)
            yield from schema_paths(item, prefix + (key,))
    elif isinstance(value, list) and value:
        seen = set()
        for index, item in enumerate(value):
            if isinstance(item, (dict, list)):
                signature = schema_signature(item)
                if signature in seen:
                    continue
                seen.add(signature)
                yield from schema_paths(item, prefix + (index,))


def schema_signature(value):
    if isinstance(value, dict):
        return tuple(
            (key, schema_signature(item)) for key, item in sorted(value.items())
        )
    if isinstance(value, list):
        return ("list", tuple(sorted({repr(schema_signature(item)) for item in value})))
    return type(value).__name__


def location(probe, path):
    value = probe
    for key in path[:-1]:
        value = value[key]
    return value, path[-1]


def tamper_checks(root, probe, missing_seen):
    rejected = 0

    def reject(bad, mode):
        nonlocal rejected
        try:
            verify(root, bad)
        except (
            AssertionError,
            KeyError,
            IndexError,
            ValueError,
            TypeError,
            struct.error,
        ):
            rejected += 1
        else:
            raise AssertionError("resource tamper accepted: " + mode)

    for path in schema_paths(probe):
        normalized = tuple(0 if type(part) is int else part for part in path)
        if normalized in missing_seen:
            continue
        missing_seen.add(normalized)
        bad = copy.deepcopy(probe)
        value, key = location(bad, path)
        del value[key]
        reject(bad, "missing " + ".".join(map(str, path)))
    for stage in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
    ):
        bad = copy.deepcopy(probe)
        bad[stage]["fingerprint"] = bytes(32).hex()
        reject(bad, stage + " SHA")
        for counter in range(3):
            if probe[stage]["charges"][counter]:
                bad = copy.deepcopy(probe)
                bad[stage]["charges"][counter] -= 1
                reject(bad, stage + " charge " + str(counter))
    bad = copy.deepcopy(probe)
    for constructor in bad["observed"]["constructors"]:
        constructor[1] += 1
    for stage in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
    ):
        bad[stage]["charges"][1] += 1
    bad["images_selection"]["prior"][1] += 1
    for key in ("caller_after", "ancestor_callback"):
        if (
            bad["observed"]["scope"] == "resource_driver"
            and bad["observed"][key] is not None
        ):
            bad["observed"][key][1] += 1
    reject(
        bad, "coupled resource constructor spool floor and all downstream reservations"
    )
    mutations = []
    if probe["fonts_selection"]["uses"]:
        mutations.extend(
            [
                (("fonts_selection", "uses", 0, "glyphs", 0), "increment"),
                (("fonts_selection", "uses", 0, "instance"), "increment"),
                (("fonts_selection", "uses", 0, "text"), "append"),
                (("fonts_selection", "uses", 0, "size"), "increment"),
            ]
        )
    if probe["closures"]["fonts"]:
        mutations.append((("closures", "fonts", 0, "mapping", 0, 1), "increment"))
    if probe["programs"]["fonts"]:
        mutations.extend(
            [
                (("programs", "fonts", 0, "metrics", "ascent"), "increment"),
                (("programs", "fonts", 0, "advances", 0, 1), "increment"),
            ]
        )
    if probe["cids"]["fonts"]:
        mutations.extend(
            [
                (("cids", "fonts", 0, "bindings", 0, "width"), "increment"),
                (("cids", "fonts", 0, "bindings", 0, "subset"), "increment"),
            ]
        )
    if probe["cids"]["uses"]:
        mutations.append((("cids", "uses", 0, "requires_actual_text"), "toggle"))
    if probe["images_selection"]["uses"]:
        mutations.extend(
            [
                (("images_selection", "uses", 0, "owner"), "increment"),
                (("images_selection", "uses", 0, "page"), "increment"),
                (("images_selection", "uses", 0, "resource"), "increment"),
                (
                    ("images_selection", "uses", 0, "geometry", "viewport", 0),
                    "increment",
                ),
            ]
        )
    if probe["images_selection"]["images"]:
        mutations.extend(
            [
                (("images_selection", "images", 0, "width"), "increment"),
                (("images_selection", "images", 0, "height"), "increment"),
                (("images_selection", "images", 0, "decoded_bytes"), "increment"),
            ]
        )
    for path, mode in mutations:
        bad = copy.deepcopy(probe)
        value, key = location(bad, path)
        value[key] = (
            not value[key]
            if mode == "toggle"
            else value[key] + ("x" if mode == "append" else 1)
        )
        reseal_resources(bad)
        reject(bad, "resealed " + ".".join(map(str, path)))
    if probe["programs"]["fonts"]:
        bad = copy.deepcopy(probe)
        program = bad["programs"]["fonts"][0]
        data = bytearray(
            artifact(root, program["program"]["file"], program["program"]["sha256"])
        )
        # Preserve valid directory/whole checksums, then reissue all reported
        # output hashes. The original admitted glyph bytes remain unchanged.
        entries = [
            struct.unpack(">4sIII", data[at : at + 16])
            for at in range(12, 12 + 16 * int.from_bytes(data[4:6], "big"), 16)
        ]
        glyf = next(entry for entry in entries if entry[0] == b"glyf")
        if glyf[3]:
            data[glyf[2] + min(10, glyf[3] - 1)] ^= 1
            head_at = next(entry[2] for entry in entries if entry[0] == b"head")
            data[head_at + 8 : head_at + 12] = bytes(4)
            for index, (_, _, offset, length) in enumerate(entries):
                data[16 + 16 * index : 20 + 16 * index] = u(
                    checksum(bytes(data[offset : offset + length])), 4
                )
            data[head_at + 8 : head_at + 12] = u(
                (0xB1B0AFBA - checksum(bytes(data))) & 0xFFFFFFFF, 4
            )
            with tempfile.TemporaryDirectory(
                prefix="typaxis-column-resource-font-negative-"
            ) as directory:
                isolated = Path(directory)
                for path in root.glob("*.bin"):
                    (isolated / path.name).write_bytes(path.read_bytes())
                actual_hash = digest(bytes(data)).hex()
                name = "altered-font-" + actual_hash + ".bin"
                (isolated / name).write_bytes(data)
                program["program"] = dict(
                    file=name, sha256=actual_hash, length=len(data)
                )
                program["sha256"] = actual_hash
                reseal_resources(bad)
                try:
                    verify(isolated, bad)
                except (
                    AssertionError,
                    KeyError,
                    IndexError,
                    ValueError,
                    TypeError,
                    struct.error,
                ):
                    rejected += 1
                else:
                    raise AssertionError(
                        "checksum-valid resealed altered glyph program accepted"
                    )
    for index, program in enumerate(probe["rasters"]["programs"]):
        if program is None or program["encoding"] != "Flate":
            continue
        bad = copy.deepcopy(probe)
        altered = bad["rasters"]["programs"][index]
        data = artifact(root, altered["program"]["file"], altered["sha256"])
        pixels = bytearray(zlib.decompress(data))
        pixels[0] ^= 1
        output = zlib.compress(bytes(pixels), 6)
        with tempfile.TemporaryDirectory(
            prefix="typaxis-column-resource-pixel-negative-"
        ) as directory:
            isolated = Path(directory)
            for path in root.glob("*.bin"):
                (isolated / path.name).write_bytes(path.read_bytes())
            actual_hash = digest(output).hex()
            name = "altered-raster-" + actual_hash + ".bin"
            (isolated / name).write_bytes(output)
            altered["program"] = dict(file=name, sha256=actual_hash, length=len(output))
            altered["sha256"] = actual_hash
            reseal_resources(bad)
            try:
                verify(isolated, bad)
            except (
                AssertionError,
                KeyError,
                IndexError,
                ValueError,
                TypeError,
                struct.error,
            ):
                rejected += 1
            else:
                raise AssertionError("resealed altered original raster pixels accepted")
    # The enclosed source/display checker has its own full field/owner reseal
    # negative suite. Reuse it without changing the frozen implementation.
    rejected += display.tamper_checks(probe["display_probe"])
    return rejected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    root = args.directory.resolve()
    paths = sorted(root.glob("*.json"))
    check(paths, "actual resource JSON probes required")
    totals, scopes, modes, missing_seen = Counter(), Counter(), Counter(), set()
    for path in paths:
        raw = path.read_bytes()
        probe = json.loads(raw)
        check(
            path.name == probe["case"] + "-" + digest(raw).hex() + ".json",
            "actual artifact filename SHA",
        )
        try:
            result = verify(root, probe)
        except (AssertionError, KeyError, ValueError) as error:
            raise AssertionError(f"{path.name}: {error}") from error
        scopes[result.pop("scope")] += 1
        modes[result.pop("selection_mode")] += 1
        totals.update(result)
        if args.self_test:
            totals["tamper_rejections"] += tamper_checks(root, probe, missing_seen)
    print(
        json.dumps(
            dict(
                probes=len(paths),
                **totals,
                observation_scopes=dict(scopes),
                selection_modes=dict(modes),
                budget_evidence="exact staged reservations/work except bounded deflater write suffix",
                scope="display/source/resource evidence; no Column PDF acceptance or whole engine replay",
            ),
            sort_keys=True,
        )
    )


def subset_name(instance):
    check(instance < 26**6, "canonical font instance name bound")
    value, result = instance, []
    for _ in range(6):
        result.append(chr(ord("A") + value % 26))
        value //= 26
    return "".join(reversed(result)) + "+Typaxis"


def checksum(data):
    data += bytes((-len(data)) % 4)
    return sum(struct.unpack(">" + "I" * (len(data) // 4), data)) & 0xFFFFFFFF


def remap_glyph(glyph, mapping):
    result = bytearray(glyph)
    if len(result) < 10 or int.from_bytes(result[:2], "big", signed=True) >= 0:
        return bytes(result)
    at = 10
    while True:
        flags, gid = struct.unpack(">HH", result[at : at + 4])
        result[at + 2 : at + 4] = u(mapping[gid], 2)
        at += 4 + (4 if flags & 1 else 2)
        at += 2 if flags & 8 else 4 if flags & 64 else 8 if flags & 128 else 0
        if not flags & 32:
            return bytes(result)


def verify_subset_bytes(data, source, name):
    tables = font_tables(data, 0)
    original, closed = source["tables"], source["closed"]
    expected_tags = {
        b"glyf",
        b"head",
        b"hhea",
        b"hmtx",
        b"loca",
        b"maxp",
        b"post",
        b"name",
    }
    if b"OS/2" in original:
        expected_tags.add(b"OS/2")
    check(set(tables) == expected_tags, "exact subset SFNT table set")
    check(
        data[:4] == bytes.fromhex("00010000") and checksum(data) == 0xB1B0AFBA,
        "actual subset SFNT whole checksum",
    )
    table_count = len(tables)
    selector = table_count.bit_length() - 1
    check(
        data[4:12]
        == u(table_count, 2)
        + u(16 << selector, 2)
        + u(selector, 2)
        + u(table_count * 16 - (16 << selector), 2),
        "canonical SFNT directory search fields",
    )
    previous, end = b"", 12 + 16 * table_count
    for at in range(12, 12 + 16 * table_count, 16):
        tag, claimed, offset, length = struct.unpack(">4sIII", data[at : at + 16])
        check(
            tag > previous and offset == end and offset % 4 == 0,
            "canonical nonoverlapping ordered SFNT tables",
        )
        payload = tables[tag]
        if tag == b"head":
            payload = payload[:8] + bytes(4) + payload[12:]
        check(checksum(payload) == claimed, "actual SFNT table checksum")
        end = (offset + length + 3) & ~3
        check(not any(data[offset + length : end]), "zero SFNT table padding")
        previous = tag
    check(end == len(data), "no unbound SFNT trailing bytes")
    mapping = {gid: dense for dense, gid in enumerate(closed)}
    glyph_data, offsets, horizontal = bytearray(), [], bytearray()
    hmetrics = int.from_bytes(original[b"hhea"][34:36], "big")
    for gid in closed:
        offsets.append(len(glyph_data))
        glyph = remap_glyph(source["glyphs"][gid], mapping)
        glyph_data.extend(glyph)
        glyph_data.extend(bytes((-len(glyph_data)) % 4))
        if gid < hmetrics:
            horizontal.extend(original[b"hmtx"][4 * gid : 4 * gid + 4])
        else:
            horizontal.extend(
                original[b"hmtx"][4 * (hmetrics - 1) : 4 * (hmetrics - 1) + 2]
            )
            bearing_at = 4 * hmetrics + 2 * (gid - hmetrics)
            horizontal.extend(original[b"hmtx"][bearing_at : bearing_at + 2])
    offsets.append(len(glyph_data))
    check(
        tables[b"glyf"] == bytes(glyph_data),
        "all original glyph bytes and exact composite remapping",
    )
    check(
        tables[b"loca"] == b"".join(u(offset, 4) for offset in offsets),
        "exact dense subset loca",
    )
    check(tables[b"hmtx"] == bytes(horizontal), "exact original hmtx bytes")
    head = bytearray(original[b"head"])
    head[8:12] = tables[b"head"][8:12]
    head[50:52] = u(1, 2)
    hhea = bytearray(original[b"hhea"])
    hhea[34:36] = u(len(closed), 2)
    maxp = bytearray(original[b"maxp"])
    maxp[4:6] = u(len(closed), 2)
    post = bytearray(32)
    post[:4] = bytes.fromhex("00030000")
    if len(original.get(b"post", b"")) >= 16:
        post[4:16] = original[b"post"][4:16]
    check(
        (tables[b"head"], tables[b"hhea"], tables[b"maxp"], tables[b"post"])
        == (bytes(head), bytes(hhea), bytes(maxp), bytes(post)),
        "subset exact source metrics tables",
    )
    encoded = name.encode("utf-16-be")
    name_table = (
        struct.pack(">HHHHHHHHH", 0, 1, 18, 3, 1, 0x409, 6, len(encoded), 0) + encoded
    )
    check(tables[b"name"] == name_table, "canonical subset name table bytes")
    if b"OS/2" in original:
        check(tables[b"OS/2"] == original[b"OS/2"], "preserved original OS/2 bytes")


def writer_charge(budget, source, output_length, sizes):
    tables, closed = source["tables"], source["closed"]
    n = len(closed)
    budget.charge(1, 14, 1)
    budget.charge(work=128 * 16)
    budget.charge(work=n + 128 * sum(len(source["components"][gid]) for gid in closed))
    pad = lambda length: (length + 3) & ~3
    lengths = [
        sum(pad(len(source["glyphs"][gid])) for gid in closed),
        len(tables[b"head"]),
        len(tables[b"hhea"]),
        n * 4,
        (n + 1) * 4,
        len(tables[b"maxp"]),
        32,
        46,
        len(tables.get(b"OS/2", b"")),
    ]
    table_count = 8 + int(b"OS/2" in tables)
    expected_length = 12 + table_count * 16 + sum(pad(length) for length in lengths)
    check(output_length == expected_length, "exact TrueType subset output reservation")
    budget.charge(
        n * 3 + table_count + 5,
        sum(lengths)
        + output_length
        + 512
        + n * 128
        + table_count * sizes["sfnt_rewrite_table"]
        + 14,
        output_length * 4 + n * 128 + table_count * 128,
    )
    budget.charge(work=128 * sum(len(source["components"][gid]) for gid in closed))
    budget.charge(work=(output_length + 63) // 64 + 1)


def verify_programs(root, probe, sources, budget):
    stage = fields(
        probe["programs"], ("fingerprint", "charges", "fonts"), "font programs"
    )
    closures = probe["closures"]
    n = len(sources)
    check(len(stage["fonts"]) == n, "complete written font programs")
    # The one aggregate CFF session checks all instances even when all selected
    # original fonts are TrueType. Both complete visits are charged.
    budget.charge(work=2 * n)
    budget.charge(n + 1, n * probe["size_of"]["font_program"], 1)
    fp = budget.fold(
        domain("typaxis.book-2-font-programs/1"), hash_bytes(closures["fingerprint"])
    )
    for source, closure, program in zip(sources, closures["fonts"], stage["fonts"]):
        fields(
            program,
            (
                "fingerprint",
                "program",
                "sha256",
                "postscript_name",
                "advances",
                "metrics",
            ),
            "font program",
        )
        budget.step()
        data = artifact(root, program["program"]["file"], program["program"]["sha256"])
        check(
            len(data) == program["program"]["length"]
            and program["sha256"] == program["program"]["sha256"],
            "actual subset bytes length/SHA",
        )
        name = subset_name(source["selected"]["instance"])
        check(program["postscript_name"] == name, "canonical subset PostScript name")
        verify_subset_bytes(data, source, name)
        writer_charge(budget, source, len(data), probe["size_of"])
        current = budget.fold(
            domain("typaxis.book-2-truetype-subset/1"),
            hash_bytes(closure["fingerprint"]),
        )
        current = budget.fold(current, digest(data))
        current = budget.fold(current, u(len(data)))
        current = budget.fold(current, name.encode())
        check(
            current == hash_bytes(program["fingerprint"]),
            "TrueType program canonical fold",
        )
        fp = budget.fold(fp, current)
        original = TTFont(
            io.BytesIO(source["data"]), fontNumber=source["selected"]["face_index"]
        )
        subset = TTFont(io.BytesIO(data))
        try:
            check(
                "glyf" in subset and "CFF " not in subset, "actual TrueType subset kind"
            )
            check(
                subset["maxp"].numGlyphs == len(source["closed"]),
                "subset closed glyph count",
            )
            check(
                original["head"].unitsPerEm == subset["head"].unitsPerEm,
                "subset UPM preserved",
            )
            check(subset["name"].getDebugName(6) == name, "actual subset name bytes")
            check("cmap" not in subset, "TrueType resource subset no unintended cmap")
            advances = []
            for dense, gid in enumerate(source["closed"]):
                source_name, dest_name = (
                    original.getGlyphOrder()[gid],
                    subset.getGlyphOrder()[dense],
                )
                check(
                    outline(original, gid) == outline(subset, dense),
                    "subset original glyph outline",
                )
                check(
                    original["hmtx"].metrics[source_name]
                    == subset["hmtx"].metrics[dest_name],
                    "subset original advance and bearing",
                )
                advances.append([gid, original["hmtx"].metrics[source_name][0]])
            strict_equal(
                program["advances"], advances, "original closed-glyph advances"
            )
            expected = metrics(source["tables"])
            reported = program["metrics"]
            strict_equal(
                reported,
                dict(
                    ascent=expected["ascent_1000"],
                    descent=expected["descent_1000"],
                    cap_height=expected["cap_height_1000"],
                    stem_v=expected["stem_v_1000"],
                    italic_angle=expected["italic_angle_fixed_16_16"],
                    flags=expected["flags"],
                    bbox=expected["bbox_1000"],
                ),
                "original byte-derived PDF font metrics",
            )
            source["advances"] = dict(advances)
        finally:
            original.close()
            subset.close()
    check(
        fp == hash_bytes(stage["fingerprint"]), "font programs aggregate canonical fold"
    )
    budget.matches(stage)


if __name__ == "__main__":
    main()
