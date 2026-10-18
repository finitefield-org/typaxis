#!/usr/bin/env python3
"""Verify actual Book /2 column display callback diagnostics independently.

This checks sealed source bytes, resource identities, original selected source
data, and physical paint projections. It neither replays the whole layout or
font shaping engines nor authorizes balanced PDF output.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import struct
from pathlib import Path


ZERO = bytes(32)
STREAMS = ("separator", "marker", "text", "math", "image", "number")
SCHEMA = "typaxis.book-2-column-display-probe/1"


def check(condition, reason):
    if not condition:
        raise AssertionError(reason)


def integer(value, bits=64, signed=True):
    check(type(value) is int, f"integer required: {value!r}")
    low = -(1 << (bits - 1)) if signed else 0
    high = (1 << (bits - int(signed))) - 1
    check(low <= value <= high, f"integer range: {value}")
    return value


def boolean(value):
    check(type(value) is bool, f"boolean required: {value!r}")
    return value


def digest(value):
    return hashlib.sha256(value).digest()


def domain(name):
    return digest(name.encode())


def hash_bytes(value):
    check(type(value) is str and len(value) == 64, "SHA-256 hex width")
    check(value == value.lower(), "SHA-256 lowercase hex")
    try:
        result = bytes.fromhex(value)
    except ValueError as error:
        raise AssertionError("SHA-256 hex digits") from error
    check(len(result) == 32, "SHA-256 byte width")
    return result


def blob(value):
    check(type(value) is list, "canonical byte array")
    return bytes(integer(v, 8, False) for v in value)


def u(value, width=8):
    return integer(value, width * 8, False).to_bytes(width, "big")


def i(value, width=8):
    return integer(value, width * 8, True).to_bytes(width, "big", signed=True)


def b(value):
    return bytes([int(boolean(value))])


def fold(previous, value):
    check(len(previous) == 32 and len(value) <= 104, "closed fold width")
    return digest(previous + value)


def rect(value):
    check(type(value) is list and len(value) == 4, "rectangle width")
    result = [integer(v) for v in value]
    check(result[2] > 0 and result[3] > 0, "positive rectangle dimensions")
    return result


def rect_bytes(value):
    return b"".join(i(v) for v in rect(value))


def contains(outer, inner):
    ox, oy, ow, oh = rect(outer)
    x, y, w, h = rect(inner)
    return ox <= x and oy <= y and x + w <= ox + ow and y + h <= oy + oh


def canonical(value):
    # Diagnostic schemas use fixed ASCII keys and integer geometry, so sorted
    # JSON produces their JCS bytes without float or UTF-16 key-order ambiguity.
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def sealed_jcs(raw, fingerprint, algorithm=None):
    check(type(raw) is str, "canonical JCS string")
    value = json.loads(raw)
    check(canonical(value) == raw, "canonical JCS encoding")
    check(digest(raw.encode()) == hash_bytes(fingerprint), "canonical JCS digest")
    if algorithm is not None:
        check(value.get("algorithm") == algorithm, "canonical JCS algorithm")
    return value


class Reader:
    def __init__(self, data):
        self.data = data
        self.offset = 0

    def read(self, count):
        check(count >= 0 and self.offset + count <= len(self.data), "truncated canonical source")
        result = self.data[self.offset:self.offset + count]
        self.offset += count
        return result

    def num(self, width=8, signed=False):
        return int.from_bytes(self.read(width), "big", signed=signed)

    def flag(self):
        value = self.num(1)
        check(value in (0, 1), "canonical boolean")
        return bool(value)

    def optional(self, width):
        present = self.flag()
        value = self.num(width)
        check(present or value == 0, "absent optional integer must be zero")
        return value if present else None

    def rectangle(self):
        return rect([self.num(8, True) for _ in range(4)])

    def optional_rect(self):
        present = self.flag()
        data = self.read(32)
        if not present:
            check(data == bytes(32), "absent rectangle must be zero")
            return None
        return rect(list(struct.unpack(">qqqq", data)))

    def finish(self):
        check(self.offset == len(self.data), "unexpected canonical suffix")


def decode_source(probe):
    data = blob(probe["source_canonical"])
    check(digest(data) == hash_bytes(probe["source_fingerprint"]), "source terminal canonical SHA")
    reader = Reader(data)
    names = ("algorithm", "limits", "lines", "blocks", "measurements", "vectors", "native", "plan", "widths")
    hashes = {name: reader.read(32) for name in names}
    check(hashes["algorithm"] == domain("typaxis.book-2-column-math-terminals/1"), "column terminal algorithm")
    check(hashes["limits"] == hash_bytes(probe["limits_fingerprint"]), "source limits identity")
    check(hashes["measurements"] == hash_bytes(probe["source_measurements_fingerprint"]), "source measurement owner identity")
    check(hashes["plan"] == digest(domain("typaxis.book-2-column-source-plan/1") + hash_bytes(probe["source_package_fingerprint"])), "source authored column plan identity")
    counts = [reader.num() for _ in range(6)]
    page_count, semantic_math, repeated_math, unreferenced, number_count, fragment_count = counts
    check(page_count == len(probe["pages"]), "source physical page count")
    check(fragment_count == len(probe["original_fragments"]), "source physical fragment count")
    terminals = []
    for _ in range(semantic_math + repeated_math):
        kind = reader.num(1)
        check(kind in (0, 1), "source math kind")
        terminal = dict(kind="vector" if kind == 0 else "native", owner=reader.num(4), page=reader.num(4), fragment=reader.num())
        terminal["definition"] = reader.optional(8)
        terminal["item"] = reader.num()
        terminal["inline"] = reader.optional(4)
        terminal["cell"] = reader.optional(4)
        terminal["repeated"] = reader.flag()
        terminal["origin_x"] = reader.num(8, True)
        terminal["baseline"] = reader.num(8, True)
        terminal["viewport"] = reader.optional_rect()
        terminal["source_fingerprint"] = reader.read(32).hex()
        terminal["column"] = reader.optional(2)
        terminals.append(terminal)
    check(sum(not t["repeated"] for t in terminals) == semantic_math, "source semantic math count")
    check(sum(t["repeated"] for t in terminals) == repeated_math, "source repeated math count")
    coverage = probe["source_coverage"]
    check(coverage["semantic_math"] == semantic_math and coverage["repeated_math"] == repeated_math, "source closure math counts")
    check(coverage["unreferenced_definitions"] == unreferenced, "original unreferenced definition count")
    check(coverage["semantic_fragments"] == sum(not f["repeated"] for f in probe["original_fragments"]) and coverage["repeated_fragments"] == sum(f["repeated"] for f in probe["original_fragments"]), "exact original semantic/repeated source visits")
    numbers = []
    for _ in range(number_count):
        number = dict(owner=reader.num(4), parent=reader.num(4), fragment=reader.num(4), page=reader.num(4))
        number["bounds"] = reader.rectangle()
        number["shape_fingerprint"] = reader.read(32).hex()
        number["repeated"] = reader.flag()
        numbers.append(number)
    variants = []
    header_domain = domain("typaxis.book-2-math-terminal-header-owners/1")
    if reader.data[reader.offset:reader.offset + 32] == header_domain:
        reader.read(32)
        count = reader.num()
        check(count > 0, "nonempty header graph extension")
        for _ in range(count):
            variant = dict(page=reader.num(4), local_fragment=reader.num(), global_item=reader.num())
            variant.update({name: reader.read(32).hex() for name in ("header", "lines", "blocks", "vectors", "native")})
            variants.append(variant)
    check(coverage["header_variant_fragments"] == len(variants), "all original header graph associations")
    pages = []
    for _ in range(page_count):
        page = dict(page=reader.num(4), named_index=reader.optional(8), body=reader.rectangle())
        page["footnote"] = reader.optional_rect()
        page["placed_footnote"] = reader.optional_rect()
        columns = reader.num()
        page["separator"] = reader.optional_rect()
        page["columns"] = []
        for _ in range(columns):
            bounds = reader.rectangle()
            interval = [reader.num(), reader.num()]
            check(interval[0] <= interval[1] <= fragment_count, "source column fragment interval")
            page["columns"].append(dict(bounds=bounds, fragments=interval))
        check(columns > 0, "physical page retains empty columns")
        pages.append(page)
    balances = []
    if reader.offset < len(data):
        check(reader.read(32) == domain("typaxis.book-2-column-math-balance/1"), "source balance extension algorithm")
        count = reader.num()
        check(count > 0, "nonempty source balance extension")
        for _ in range(count):
            balances.append(dict(page=reader.num(4), fingerprint=reader.read(32).hex(), selected_height=reader.num(8, True)))
    reader.finish()
    check(len(probe["math"]) == len(terminals), "all original math terminals projected")
    check(len(probe["numbers"]) == len(numbers), "all original equation numbers projected")
    return dict(hashes=hashes, terminals=terminals, numbers=numbers, pages=pages, variants=variants, balances=balances, unreferenced=unreferenced)


def font_identity(font, resources):
    face = integer(font["face"], 32, False)
    check(face in resources, "admitted font face")
    admitted = resources[face]
    check(font["sha256"] == admitted["sha256"] and font["index"] == admitted["face_index"], "font content and face identity")
    integer(font["size"])
    check(font["size"] > 0, "positive font size")
    for name in ("ascent", "descent", "line_gap"):
        integer(font[name])


def chunks_hash(fingerprint, text):
    check(type(text) is str, "exact UTF-8 text")
    data = text.encode()
    for at in range(0, len(data), 104):
        fingerprint = fold(fingerprint, data[at:at + 104])
    return fingerprint


def glyph_hash(fingerprint, glyphs):
    for glyph in glyphs:
        check(type(glyph) is list and len(glyph) == 3, "glyph record width")
        fingerprint = fold(fingerprint, u(glyph[0], 2) + i(glyph[1]) + i(glyph[2]))
    return fingerprint


def text_hash(probe):
    fingerprint = fold(fold(domain("typaxis.book-2-text-display/1"), hash_bytes(probe["source_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    fragments = probe["original_fragments"]
    for draw in probe["text"]:
        font = fragments[draw["fragment"]]["line"]["font"]
        span = draw["span"]
        data = u(draw["owner"], 4) + u(draw["page"], 4) + u(draw["fragment"])
        data += b(draw["definition"] is not None) + u(draw["definition"] or 0) + u(draw["item"]) + u(draw["inline"], 4)
        data += b(draw["cell"] is not None) + u(draw["cell"] or 0, 4) + b(draw["repeated"])
        data += u(draw["run"], 4) + u(draw["cluster"], 4) + b(draw["generated"])
        data += b"".join(u(v, 4) for v in span) + u(len(draw["text"].encode())) + u(len(draw["glyphs"]))
        data += u(font["face"], 4) + b(draw["bounds"] is not None)
        check(len(data) == 85, "text canonical record width")
        fingerprint = fold(fingerprint, data)
        fingerprint = fold(fingerprint, hash_bytes(font["sha256"]) + u(font["index"], 4) + i(font["size"]))
        if draw["bounds"] is not None:
            fingerprint = fold(fingerprint, rect_bytes(draw["bounds"]))
        fingerprint = chunks_hash(fingerprint, draw["text"])
        fingerprint = glyph_hash(fingerprint, draw["glyphs"])
    return fingerprint


def math_hash(probe):
    fingerprint = fold(fold(domain("typaxis.book-2-math-display/1"), hash_bytes(probe["source_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    for draw in probe["math"]:
        fingerprint = fold(fingerprint, u(draw["owner"], 4))
        paint = draw["paint"]
        if paint["kind"] == "vector":
            data = hash_bytes(paint["source_sha256"]) + hash_bytes(paint["ir_fingerprint"]) + rect_bytes(paint["viewport"])
            data += i(paint["scale"], 4) + bytes(integer(v, 8, False) for v in paint["color"]) + bytes(1)
            check(len(data) == 104, "vector math canonical record width")
            fingerprint = fold(fingerprint, data)
        else:
            check(paint["kind"] == "native", "display math paint kind")
            fingerprint = fold(fingerprint, rect_bytes(paint["bounds"]) + bytes([1]))
            for item in paint["paints"]:
                if "rule" in item:
                    check(set(item) == {"rule"}, "native rule schema")
                    data = bytes([1]) + rect_bytes(item["rule"]) + bytes(2)
                else:
                    check(set(item) == {"gid", "unicode", "ordinal", "x", "y", "font_size"}, "native glyph schema")
                    data = bytes(1) + u(item["gid"], 2) + u(item["unicode"], 4) + u(item["ordinal"], 4) + i(item["x"]) + i(item["y"]) + i(item["font_size"])
                check(len(data) == 35, "native paint canonical width")
                fingerprint = fold(fingerprint, data)
    return fingerprint


def merge_hash(probe, component_hashes):
    fingerprint = domain("typaxis.book-2-column-display/1")
    for value in [probe["source_fingerprint"], probe["admitted_fingerprint"], *[component_hashes[name] for name in ("math", "text", "numbers", "markers", "images", "anchors")]]:
        fingerprint = fold(fingerprint, hash_bytes(value))
    arrays = [probe[name] for name in ("separators", "markers", "text", "math", "images", "numbers")]
    keys = []
    for stream, draws in enumerate(arrays):
        for index, draw in enumerate(draws):
            fragment = draw["before_fragment"] if stream == 0 else draw["fragment"]
            layer = 0 if stream == 0 else 1 if stream == 1 else 3 if stream == 5 else 2
            inline = draw.get("inline") or 0 if layer == 2 else 0
            keys.append(((fragment, layer, inline), stream, index))
    keys.sort()
    expected = [[STREAMS[stream], index] for _, stream, index in keys]
    check(probe["paints"] == expected, "exact six-stream authored paint order")
    inline_keys = [key for key, _, _ in keys if key[1] == 2]
    check(len(set(inline_keys)) == len(inline_keys), "unique original inline paint slot")
    fingerprint = fold(fingerprint, u(len(keys)))
    for key, stream, index in keys:
        fingerprint = fold(fingerprint, u(stream, 1) + u(index) + u(key[0]) + u(key[1], 1) + u(key[2], 4))
    for fragment in probe["original_fragments"]:
        column = fragment["column"]
        fingerprint = fold(fingerprint, b(column is not None) + u(column or 0, 2))
    return fingerprint


def graph_hash(graph, name):
    value = graph[name]
    return ZERO if value is None else hash_bytes(value)


def verify_resources(probe, source):
    limits = sealed_jcs(probe["limits_canonical_jcs"], probe["limits_fingerprint"], "typaxis.m4-effective-resource-limits/1")
    ledger = sealed_jcs(probe["admitted_canonical_jcs"], probe["admitted_fingerprint"], "typaxis.admitted-production-resources/3")
    check(ledger["limits_fingerprint"] == probe["limits_fingerprint"], "admitted limits identity")
    check(ledger["resource_set"] == "typaxis.production-book-resource-set/3", "admitted resource set")
    fonts = {integer(f["font_face_id"], 32, False): f for f in ledger["fonts"]}
    images = {integer(v["image_id"], 32, False): v for v in ledger["images"]}
    check(list(fonts) == list(range(len(fonts))), "dense admitted font IDs")
    check(list(images) == list(range(len(images))), "dense admitted image IDs")
    for value in [*fonts.values(), *images.values()]:
        hash_bytes(value["sha256"])
    epoch = fold(fold(domain("typaxis.book-2-vector-epoch/1"), hash_bytes(ledger["profile_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    original = probe["original_graph"]
    for field, sealed in (("line_fingerprint", "lines"), ("block_fingerprint", "blocks"), ("vector_fingerprint", "vectors"), ("native_fingerprint", "native")):
        check(graph_hash(original, field) == source["hashes"][sealed], f"original graph {field} source identity")
    for graph in [original, *[f["graph"] for f in probe["original_fragments"]]]:
        check(hash_bytes(graph["binding_epoch"]) == epoch, "source graph binding epoch")
        for name in ("line_fingerprint", "prepared_fingerprint", "shape_fingerprint", "block_fingerprint", "vector_fingerprint", "native_fingerprint"):
            graph_hash(graph, name)
    variants = {}
    for variant in source["variants"]:
        key = (variant["page"], variant["local_fragment"])
        check(key not in variants, "unique actual header association")
        variants[key] = variant
    for fragment in probe["original_fragments"]:
        key = (fragment["page"], fragment["local_fragment"])
        variant = variants.pop(key, None)
        header = fragment["header"]
        if variant is None:
            check(header is None, "no unsealed header graph")
            check(fragment["graph"] == original, "ordinary occurrence original graph")
        else:
            check(header is not None and fragment["repeated"], "actual repeated header graph association")
            check(header["fragment"] == variant["local_fragment"] and header["global_item"] == variant["global_item"], "header actual graph indexes")
            check(header["fingerprint"] == variant["header"], "header variant source receipt")
            for field, sealed in (("line_fingerprint", "lines"), ("block_fingerprint", "blocks"), ("vector_fingerprint", "vectors"), ("native_fingerprint", "native")):
                check(graph_hash(fragment["graph"], field).hex() == variant[sealed], "header actual graph source identity")
    check(not variants, "every sealed header association consumed")
    return dict(limits=limits, ledger=ledger, ledger_canonical=probe["admitted_canonical_jcs"], fonts=fonts, images=images, epoch=epoch)


def verify_geometry(probe, source):
    fragments = probe["original_fragments"]
    check([f["fragment"] for f in fragments] == list(range(len(fragments))), "contiguous original fragment indexes")
    balances = {v["page"]: v for v in source["balances"]}
    check(len(balances) == len(source["balances"]), "unique balanced physical page")
    offset = 0
    for page, sealed in zip(probe["pages"], source["pages"]):
        check(page["page"] == sealed["page"], "physical page source identity")
        for key in ("body", "footnote", "placed_footnote", "named_index"):
            check(page[key] == sealed[key], f"actual {key} source identity")
        check(page["separator_ink"] == sealed["separator"], "actual separator source identity")
        check(page["columns"] == sealed["columns"], "all actual columns including empty ones")
        start, end = page["fragments"]
        check(start == offset and start <= end <= len(fragments), "physical page contiguous fragment range")
        offset = end
        last = 0
        for index, column in enumerate(page["columns"]):
            first, stop = column["fragments"]
            check(first == last and first <= stop <= end - start, "actual column contiguous fragment range")
            last = stop
            for local in range(first, stop):
                fragment = fragments[start + local]
                # Column ranges are page-local, unlike the global callback index.
                check(fragment is not None, "page-local column fragment interval")
                check(fragment["column"] == index and fragment["definition"] is None, "body fragment exact column membership")
                check(contains(column["bounds"], fragment["bounds"]), "body fragment contained in actual column")
        for local, fragment in enumerate(fragments[start:end]):
            check(fragment["page"] == page["page"] and fragment["local_fragment"] == local, "original physical page/local index")
            check((fragment["column"] is None) == (fragment["definition"] is not None), "shared note role separate from body columns")
            rect(fragment["bounds"])
            integer(fragment["owner"], 32, False)
            boolean(fragment["repeated"])
            if fragment["column"] is None:
                check(page["placed_footnote"] is not None and contains(page["placed_footnote"], fragment["bounds"]), "shared note physical origin")
            line = fragment["line"]
            if line is not None:
                check(fragment["owner"] == line["owner"], "original paragraph source owner")
                check(fragment["source"] == {"kind": "paragraph", "paragraph": line["paragraph"], "line": line["line"]}, "paragraph actual graph indexes")
                check(fragment["baseline"] == fragment["bounds"][1] + line["baseline"], "source selected line physical baseline")
            else:
                check(fragment["owner"] == fragment["source"]["owner"], "original block/figure source owner")
        balance = page["balance"]
        sealed_balance = balances.pop(page["page"], None)
        check((balance is None) == (sealed_balance is None), "balanced page source extension completeness")
        if balance is not None:
            data = blob(balance["canonical"])
            check(digest(data) == hash_bytes(balance["fingerprint"]), "sealed balance receipt SHA")
            check(len(data) >= 80 and data[:32] == domain("typaxis.book-2-column-balance/1"), "sealed balance algorithm")
            selected = int.from_bytes(data[64:72], "big", signed=True)
            check(selected == balance["selected_height"] == sealed_balance["selected_height"] > 0, "sealed balance selected height")
            check(balance["fingerprint"] == sealed_balance["fingerprint"], "actual balanced source receipt")
            check(all(c["bounds"][3] == selected for c in page["columns"]), "effective selected balanced column height")
    check(offset == len(fragments) and not balances, "all physical pages and balance extensions covered")


def verify_text(probe, resources):
    expected = []
    for fragment in probe["original_fragments"]:
        line = fragment["line"]
        if line is None:
            continue
        for inline, item in enumerate(line["items"]):
            check(item["inline"] == inline, "original authored inline indexes")
            if item["kind"] != "text":
                check(item["kind"] in ("math", "image", "break"), "original successor inline kind")
                continue
            check(line["font"] is not None, "selected text has original font")
            font_identity(line["font"], resources["fonts"])
            width = sum(integer(g["advance_x"]) for g in item["glyphs"])
            height = line["font"]["ascent"] - line["font"]["descent"]
            check(width >= 0 and height >= 0, "nonnegative selected cluster metrics")
            bounds = [fragment["bounds"][0] + item["pen_x"], fragment["baseline"] - line["font"]["ascent"], width, height] if width and height else None
            glyphs = [[integer(g["gid"], 16, False), fragment["bounds"][0] + g["x"], fragment["bounds"][1] + g["y"]] for g in item["glyphs"]]
            expected.append(dict(owner=item["owner"], fragment=fragment["fragment"], page=fragment["page"], definition=fragment["definition"], item=fragment["item"], inline=inline, cell=fragment["cell"], repeated=fragment["repeated"], run=item["run"], cluster=item["cluster"], text=item["text"], span=item["source"]["span"], generated=item["source"]["generated"], bounds=bounds, glyphs=glyphs))
            span = item["source"]["span"]
            check(len(span) == 3 and span[1] <= span[2] and span[2] - span[1] == len(item["text"].encode()), "exact selected cluster UTF-8 span")
            check(span[0] >= probe["source_text_buffers"] if item["source"]["generated"] else span[0] < probe["source_text_buffers"], "parsed/generated buffer namespace")
    check(probe["text"] == expected, "exact original text/GID/span/physical projection")


def verify_vector(original, resources):
    check(original["kind"] == "vector", "original vector receipt kind")
    binding = original["binding"]
    check(binding["algorithm"] == "typaxis.book-2-vector-binding/1", "original vector binding algorithm")
    check(digest(canonical(binding).encode()) == hash_bytes(original["fingerprint"]), "original vector binding canonical SHA")
    check(hash_bytes(binding["epoch"]) == resources["epoch"] and binding["node_id"] == original["owner"], "original vector epoch and owner")
    resource = binding["resource"]
    image = resources["images"].get(original["image"])
    check(image is not None and resource["image_id"] == original["image"], "original vector admitted image ID")
    check(original["source_sha256"] == resource["source_sha256"] == image["sha256"], "original vector admitted source SHA")
    check(original["ir_fingerprint"] == resource["ir_fingerprint"], "original safe vector IR identity")
    check(resource["limits_fingerprint"] == resources["ledger"]["limits_fingerprint"] and resource["profile_fingerprint"] == resources["ledger"]["profile_fingerprint"], "vector resource limits/profile identity")
    placement = binding["placement"]
    check(placement["scale"] == original["scale"] > 0, "original vector scale")
    check(original["color"] == [placement["paint"][c] for c in ("red", "green", "blue")], "original vector color")
    check(len(original["color"]) == 3, "RGB channel count")
    for value in original["color"]:
        integer(value, 8, False)


def verify_native(original, resources):
    check(original["kind"] == "native", "original native receipt kind")
    face = resources["fonts"].get(original["face"])
    check(face is not None and face["sha256"] == original["font_sha256"] and face["face_index"] == original["face_index"], "native admitted font identity")
    computation = sealed_jcs(original["computation_canonical_jcs"], original["computation_fingerprint"], "typaxis.math-layout/1")
    check(computation["dimensions"]["bbox"] == original["bbox"], "original native computed box")
    vector = {"algorithm": "typaxis.math-vector-ir/1", "paint": []}
    for paint in original["paints"]:
        if "rule" in paint:
            check(set(paint) == {"rule"}, "original native rule schema")
            x, y, width, height = rect(paint["rule"])
            vector["paint"].append(dict(kind="rule", x=x, y=y, width=width, height=height))
        else:
            check(set(paint) == {"gid", "unicode", "ordinal", "x", "y", "font_size", "advance"}, "original native glyph schema")
            integer(paint["gid"], 16, False)
            check(paint["font_size"] > 0 and paint["advance"] > 0, "original native glyph metrics")
            scalar = integer(paint["unicode"], 32, False)
            check(scalar <= 0x10FFFF and not 0xD800 <= scalar <= 0xDFFF, "native Unicode scalar")
            vector["paint"].append(dict(kind="glyph", x=paint["x"], y=paint["y"], original_gid=paint["gid"], unicode=chr(scalar), logical_ordinal=paint["ordinal"], font_size=paint["font_size"], advance=paint["advance"]))
    check(digest(canonical(vector).encode()) == hash_bytes(computation["vector_fingerprint"]), "native original vector paint SHA")
    instances = dict(algorithm="typaxis.production-font-instances/3", faces=list(resources["fonts"]), ledger_fingerprint=digest(resources["ledger_canonical"].encode()).hex())
    key = fold(domain("typaxis.book-2-native-math/1"), resources["epoch"])
    key = fold(key, digest(u(original["owner"], 4)))
    for value in (digest(canonical(instances).encode()), hash_bytes(original["font_sha256"]), digest(canonical(original["style"]).encode()), hash_bytes(original["computation_fingerprint"])):
        key = fold(key, value)
    check(key == hash_bytes(original["fingerprint"]), "native original binding SHA")


def verify_math(probe, source, resources):
    expected_slots = []
    for fragment in probe["original_fragments"]:
        if fragment["line"] is not None:
            expected_slots.extend((fragment["fragment"], item["inline"], item["owner"], item["fingerprint"]) for item in fragment["line"]["items"] if item["kind"] == "math")
        elif fragment["source"]["kind"] in ("native_block", "vector_block"):
            if fragment["source"]["kind"] == "native_block" or fragment["source"]["vector"]["binding"]["placement"]["kind"] == "math_vector_block":
                expected_slots.append((fragment["fragment"], None, fragment["owner"], None))
    check(len(expected_slots) == len(probe["math"]), "every original inline/block math occurrence projected once")
    for draw, terminal, slot in zip(probe["math"], source["terminals"], expected_slots):
        for key, value in terminal.items():
            if key in ("kind", "viewport"):
                continue
            check(draw[key] == value, f"exact sealed math terminal {key}")
        check((draw["fragment"], draw["inline"], draw["owner"]) == slot[:3], "original authored math occurrence identity")
        if slot[3] is not None:
            check(draw["source_fingerprint"] == slot[3], "original selected inline math receipt")
        fragment = probe["original_fragments"][draw["fragment"]]
        for key in ("page", "column", "definition", "item", "cell", "repeated"):
            check(draw[key] == fragment[key], f"math original physical role {key}")
        original = draw["original"]
        check(original["kind"] == terminal["kind"] and original["owner"] == draw["owner"] and original["fingerprint"] == draw["source_fingerprint"], "original math receipt ownership")
        check(draw["alternative"] == original["alternative"] and draw["actual_text"] == original["actual_text"], "original per-occurrence math semantics")
        if draw["inline"] is not None:
            item = fragment["line"]["items"][draw["inline"]]
            check(draw["origin_x"] == fragment["bounds"][0] + item["pen_x"] and draw["baseline"] == fragment["bounds"][1] + item["baseline"], "original inline math physical origin")
        elif terminal["kind"] == "native":
            check(draw["origin_x"] == fragment["viewport"][0] - fragment["source"]["left"] and draw["baseline"] == fragment["baseline"], "original native block physical origin")
        else:
            check(draw["origin_x"] == fragment["viewport"][0] - fragment["source"]["origin_x"] and draw["baseline"] == fragment["baseline"], "original vector block physical origin")
        paint = draw["paint"]
        if terminal["kind"] == "vector":
            verify_vector(original, resources)
            check(paint["kind"] == "vector" and paint["viewport"] == terminal["viewport"], "actual vector math viewport")
            for key in ("source_sha256", "ir_fingerprint", "scale", "color"):
                check(paint[key] == original[key], f"vector math original {key}")
            check(paint["matrix"] == [original["scale"], 0, 0, original["scale"], paint["viewport"][0], paint["viewport"][1]], "exact vector placement matrix")
            if draw["inline"] is not None:
                local = fragment["line"]["items"][draw["inline"]]["viewport"]
                check(paint["viewport"] == [fragment["bounds"][0] + local[0], fragment["bounds"][1] + local[1], local[2], local[3]], "inline vector actual viewport translation")
            else:
                check(paint["viewport"] == fragment["viewport"], "block vector actual viewport")
        else:
            verify_native(original, resources)
            left, top, right, bottom = original["bbox"]
            check(paint["kind"] == "native" and paint["bounds"] == [draw["origin_x"] + left, draw["baseline"] + top, right - left, bottom - top], "native computed actual bounds")
            expected = []
            for local in original["paints"]:
                if "rule" in local:
                    x, y, width, height = local["rule"]
                    expected.append(dict(rule=[draw["origin_x"] + x, draw["baseline"] + y, width, height]))
                else:
                    glyph = {key: value for key, value in local.items() if key != "advance"}
                    glyph["x"] += draw["origin_x"]
                    glyph["y"] += draw["baseline"]
                    expected.append(glyph)
            check(paint["paints"] == expected, "native original GIDs/rules/ordinals/physical paints")


def utf8_slice(text, start, end):
    check(0 <= start <= end <= len(text.encode()), "original UTF-8 slice bounds")
    try:
        return text.encode()[start:end].decode()
    except UnicodeDecodeError as error:
        raise AssertionError("original UTF-8 scalar boundary") from error


def verify_numbers(probe, source, resources):
    actual_placements = [dict(n, page=p["page"]) for p in probe["pages"] for n in p["equation_numbers"]]
    check(actual_placements == source["numbers"], "all actual independent equation number placements")
    for draw, placement in zip(probe["numbers"], source["numbers"]):
        for key, value in placement.items():
            check(draw[key] == value, f"equation number sealed placement {key}")
        fragment = probe["original_fragments"][draw["fragment"]]
        check(fragment["owner"] == draw["parent"] and fragment["page"] == draw["page"], "equation number original parent occurrence")
        check(fragment["repeated"] == draw["repeated"], "equation number repeated source role")
        original = draw["original"]
        font = original["font"]
        font_identity(font, resources["fonts"])
        check(draw["bounds"][2:] == [original["width"], original["height"]], "equation number original measured dimensions")
        runs = original["runs"]
        widths = [sum(integer(g["advance_x"]) for g in r["glyphs"]) for r in runs]
        check(all(w > 0 for w in widths), "positive original number run advances")
        levels = [integer(r["bidi_level"], 8, False) for r in runs]
        order = list(range(len(runs)))
        odd = [level for level in levels if level % 2]
        if odd:
            for level in range(max(levels), min(odd) - 1, -1):
                at = 0
                while at < len(order):
                    if levels[order[at]] < level:
                        at += 1
                        continue
                    start = at
                    while at < len(order) and levels[order[at]] >= level:
                        at += 1
                    order[start:at] = reversed(order[start:at])
        origins = [0] * len(runs)
        width = 0
        for run in order:
            origins[run] = width
            width += widths[run]
        check(width == draw["bounds"][2], "independent number bidi run width")
        leading = draw["bounds"][3] - font["ascent"] + font["descent"]
        half = leading // 2 if leading >= 0 else -((-leading) // 2)
        baseline = draw["bounds"][1] + half + font["ascent"]
        expected = []
        for run_index, run in enumerate(runs):
            pen = draw["bounds"][0] + origins[run_index]
            positioned = []
            for glyph in run["glyphs"]:
                check(glyph["advance_y"] == 0, "horizontal independent number glyphs")
                positioned.append(([glyph["gid"], pen + glyph["offset_x"], baseline - glyph["offset_y"]], pen))
                pen += glyph["advance_x"]
            for cluster_index, cluster in enumerate(run["clusters"]):
                check(not cluster["source"]["generated"], "independently authored number parsed text")
                span = cluster["source"]["span"]
                start, end = cluster["glyph_range"]
                check(0 <= start <= end <= len(positioned), "number original glyph cluster range")
                advance = sum(g["advance_x"] for g in run["glyphs"][start:end])
                bounds = [positioned[start][1], draw["bounds"][1], advance, draw["bounds"][3]] if start < end and advance > 0 else None
                expected.append(dict(run=run_index, cluster=cluster_index, text=utf8_slice(draw["text"], span[1] - original["text_start"], span[2] - original["text_start"]), span=span, bounds=bounds, glyphs=[p[0] for p in positioned[start:end]]))
        check(draw["clusters"] == expected, "independent number source clusters/GIDs/bidi/geometry")


def number_hash(probe):
    fingerprint = fold(fold(domain("typaxis.book-2-number-display/1"), hash_bytes(probe["source_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    for draw in probe["numbers"]:
        data = b"".join(u(draw[name], 4) for name in ("owner", "parent", "page", "fragment"))
        data += rect_bytes(draw["bounds"]) + b(draw["repeated"]) + u(draw["original"]["font_instance"], 4)
        fingerprint = fold(fingerprint, data)
        fingerprint = fold(fingerprint, hash_bytes(draw["shape_fingerprint"]))
        font = draw["original"]["font"]
        fingerprint = fold(fingerprint, b"".join(i(font[name]) for name in ("ascent", "descent", "line_gap")))
        for cluster in draw["clusters"]:
            data = u(cluster["run"], 4) + u(cluster["cluster"], 4) + b"".join(u(v, 4) for v in cluster["span"])
            data += u(len(cluster["text"].encode())) + u(len(cluster["glyphs"])) + b(cluster["bounds"] is not None)
            check(len(data) == 37, "number cluster canonical width")
            fingerprint = fold(fingerprint, data)
            if cluster["bounds"] is not None:
                fingerprint = fold(fingerprint, rect_bytes(cluster["bounds"]))
            fingerprint = chunks_hash(fingerprint, cluster["text"])
            fingerprint = glyph_hash(fingerprint, cluster["glyphs"])
    return fingerprint


def verify_markers(probe, resources):
    expected = []
    separators = []
    for page in probe["pages"]:
        start, end = page["fragments"]
        placements = [(m["fragment"], 1, dict(m, kind="list")) for m in page["list_markers"]]
        placements += [(m["fragment"], 0, dict(m, kind="footnote")) for m in page["footnote_markers"]]
        placements.sort(key=lambda v: v[:2])
        for _, _, marker in placements:
            expected.append((start + marker["fragment"], marker))
        if page["separator_ink"] is not None:
            first = next((f["fragment"] for f in probe["original_fragments"][start:end] if f["definition"] is not None), None)
            check(first is not None, "actual separator has selected note source")
            separators.append(dict(page=page["page"], before_fragment=first, ink=page["separator_ink"]))
    check(probe["separators"] == separators, "every actual separator projected before original note")
    check(len(expected) == len(probe["markers"]), "every original marker placement projected")
    for draw, (fragment_index, placement) in zip(probe["markers"], expected):
        check(draw["fragment"] == fragment_index and draw["kind"] == placement["kind"] and draw["bounds"] == placement["bounds"] and draw["baseline"] == placement["baseline"], "actual original marker placement")
        fragment = probe["original_fragments"][fragment_index]
        check(draw["cell"] == fragment["cell"] and draw["repeated"] == fragment["repeated"], "marker original physical role")
        if draw["kind"] == "footnote":
            check(placement["definition"] == fragment["definition"], "original footnote marker definition")
        elif fragment["baseline"] is not None:
            check(draw["baseline"] == fragment["baseline"], "original list marker baseline")
        original = draw["original"]
        check(original["owner"] == draw["owner"], "original marker shape owner")
        font_identity(original["font"], resources["fonts"])
        hash_bytes(original["fingerprint"])
        check(draw["bounds"][2] == original["advance"], "original marker advance")
        expected_clusters = []
        pen = draw["bounds"][0]
        pen_y = 0
        glyph_cursor = 0
        text_cursor = original["full_start"]
        for index, cluster in enumerate(original["clusters"]):
            check(cluster["source"]["generated"], "marker generated provenance")
            span = cluster["source"]["span"]
            start, end = cluster["glyph_range"]
            check(start == glyph_cursor and 0 <= start <= end <= len(original["glyphs"]), "complete original marker glyph coverage")
            check(span[0] >= probe["source_text_buffers"] and span[1] == text_cursor, "complete marker generated text coverage")
            left = pen
            glyphs = []
            for glyph in original["glyphs"][start:end]:
                glyphs.append([glyph["gid"], pen + glyph["offset_x"], draw["baseline"] - pen_y - glyph["offset_y"]])
                pen += glyph["advance_x"]
                pen_y += glyph["advance_y"]
            width = pen - left
            check(width >= 0, "nonnegative generated marker cluster advance")
            bounds = [left, draw["bounds"][1], width, draw["bounds"][3]] if width else None
            text = utf8_slice(draw["text"], span[1] - original["full_start"], span[2] - original["full_start"])
            expected_clusters.append(dict(cluster=index, text=text, span=span, bounds=bounds, glyphs=glyphs))
            glyph_cursor = end
            text_cursor = span[2]
        check(pen_y == 0 and pen - draw["bounds"][0] == original["advance"], "marker original total advance")
        check(glyph_cursor == len(original["glyphs"]) and text_cursor - original["full_start"] == len(draw["text"].encode()), "all original marker source glyphs/text covered")
        check(draw["clusters"] == expected_clusters, "exact original marker GIDs/spans/physical clusters")


def marker_hash(probe):
    fingerprint = fold(fold(domain("typaxis.book-2-marker-display/1"), hash_bytes(probe["source_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    cursor = 0
    for page in probe["pages"]:
        while cursor < len(probe["markers"]) and probe["markers"][cursor]["fragment"] < page["fragments"][1]:
            draw = probe["markers"][cursor]
            fragment = probe["original_fragments"][draw["fragment"]]
            data = b(draw["kind"] == "footnote") + u(draw["owner"], 4) + u(fragment["page"], 4) + u(draw["fragment"])
            data += b(fragment["definition"] is not None) + u(fragment["definition"] or 0) + u(fragment["item"])
            data += b(draw["cell"] is not None) + u(draw["cell"] or 0, 4) + b(draw["repeated"]) + rect_bytes(draw["bounds"]) + i(draw["baseline"])
            check(len(data) == 80, "marker canonical placement width")
            fingerprint = fold(fingerprint, data)
            fingerprint = fold(fingerprint, hash_bytes(draw["original"]["fingerprint"]))
            for cluster in draw["clusters"]:
                data = u(cluster["cluster"], 4) + b"".join(u(v, 4) for v in cluster["span"])
                data += u(len(cluster["text"].encode())) + u(len(cluster["glyphs"])) + b(cluster["bounds"] is not None)
                check(len(data) == 33, "marker cluster canonical width")
                fingerprint = fold(fingerprint, data)
                if cluster["bounds"] is not None:
                    fingerprint = fold(fingerprint, rect_bytes(cluster["bounds"]))
                fingerprint = chunks_hash(fingerprint, cluster["text"])
                fingerprint = glyph_hash(fingerprint, cluster["glyphs"])
            cursor += 1
        for separator in probe["separators"]:
            if separator["page"] == page["page"]:
                fingerprint = fold(fingerprint, u(separator["page"], 4) + u(separator["before_fragment"]) + rect_bytes(separator["ink"]))
    check(cursor == len(probe["markers"]), "all marker canonical records consumed")
    return fingerprint


def verify_images(probe, resources):
    expected = []
    for fragment in probe["original_fragments"]:
        if fragment["line"] is not None:
            for item in fragment["line"]["items"]:
                if item["kind"] == "image":
                    viewport = item["viewport"]
                    expected.append((fragment, item["inline"], item["owner"], item["fingerprint"], [fragment["bounds"][0] + viewport[0], fragment["bounds"][1] + viewport[1], viewport[2], viewport[3]]))
        elif fragment["source"]["kind"] == "figure":
            expected.append((fragment, None, fragment["owner"], None, fragment["viewport"]))
        elif fragment["source"]["kind"] == "vector_block" and fragment["source"]["vector"]["binding"]["placement"]["kind"] == "vector_figure":
            expected.append((fragment, None, fragment["owner"], fragment["source"]["source_fingerprint"], fragment["viewport"]))
    check(len(expected) == len(probe["images"]), "all original ordinary images/vectors projected")
    for draw, (fragment, inline, owner, binding_fp, viewport) in zip(probe["images"], expected):
        check(draw["owner"] == owner and draw["inline"] == inline and draw["fragment"] == fragment["fragment"], "original ordinary image occurrence")
        check(draw["cell"] == fragment["cell"] and draw["repeated"] == fragment["repeated"], "ordinary image original physical role")
        check(draw["viewport"] == viewport, "ordinary image actual viewport translation")
        original = draw["original"]
        check(original["owner"] == owner and original["alternative"] == draw["alternative"], "ordinary image source owner/alternative")
        image = resources["images"].get(original["image"])
        check(image is not None and image["sha256"] == draw["image_sha256"], "ordinary image admitted source SHA")
        if original["kind"] == "vector":
            verify_vector(original, resources)
            check(binding_fp == original["fingerprint"], "ordinary vector original binding receipt")
            check(draw["vector"] is not None, "ordinary vector actual vector paint")
            for key in ("source_sha256", "ir_fingerprint", "scale", "color"):
                check(draw["vector"][key] == original[key], f"ordinary vector original {key}")
            if inline is None:
                check(original == fragment["source"]["vector"], "block vector actual original source graph")
            else:
                dimensions = original["binding"]["placement"]["metrics"]["viewport"]
                check(draw["viewport"][2:] == [dimensions["width"], dimensions["height"]], "inline vector original measured viewport")
        else:
            check(original["kind"] == "figure" and fragment["source"]["kind"] == "figure", "original ordinary figure source kind")
            for key in ("owner", "image", "alternative", "width", "height"):
                check(original[key] == fragment["source"][key], f"original ordinary figure {key}")
            check(draw["viewport"][2:] == [original["width"], original["height"]], "original ordinary figure measured viewport")
            media = original["media"]
            check(type(media) is dict, "structured original figure media")
            if media["kind"] == "raster":
                check(draw["vector"] is None, "raster figure has raster paint")
            else:
                check(media["kind"] == "svg" and draw["vector"] is not None, "safe SVG figure has vector paint")
                check(draw["vector"]["scale"] == media["scale"] and draw["vector"]["color"] == [0, 0, 0], "ordinary SVG original scale and paint")
                check(draw["vector"]["source_sha256"] == media["source_sha256"] == draw["image_sha256"], "ordinary SVG original source SHA")
                check(draw["vector"]["ir_fingerprint"] == media["ir_fingerprint"], "ordinary SVG original safe IR")
        if draw["vector"] is not None:
            vector = draw["vector"]
            check(vector["matrix"] == [vector["scale"], 0, 0, vector["scale"], viewport[0], viewport[1]], "ordinary vector physical placement matrix")


def image_hash(probe):
    fingerprint = fold(fold(domain("typaxis.book-2-image-display/1"), hash_bytes(probe["source_fingerprint"])), hash_bytes(probe["admitted_fingerprint"]))
    for draw in probe["images"]:
        fragment = probe["original_fragments"][draw["fragment"]]
        original = draw["original"]
        data = u(draw["owner"], 4) + u(fragment["page"], 4) + u(draw["fragment"]) + u(fragment["item"]) + u(fragment["definition"] or 0)
        data += b(fragment["definition"] is not None) + u(draw["inline"] or 0, 4) + b(draw["inline"] is not None)
        data += u(draw["cell"] or 0, 4) + b(draw["cell"] is not None) + b(draw["repeated"]) + u(original["image"], 4) + b(original["kind"] == "vector")
        check(len(data) == 49, "ordinary image canonical metadata width")
        fingerprint = fold(fingerprint, data)
        fingerprint = fold(fingerprint, hash_bytes(draw["image_sha256"]))
        if original["kind"] == "vector":
            fingerprint = fold(fingerprint, hash_bytes(original["fingerprint"]))
        fingerprint = fold(fingerprint, u(len(draw["alternative"].encode())))
        fingerprint = chunks_hash(fingerprint, draw["alternative"])
        fingerprint = fold(fingerprint, rect_bytes(draw["viewport"]) + b(draw["vector"] is not None))
        if draw["vector"] is not None:
            vector = draw["vector"]
            fingerprint = fold(fingerprint, hash_bytes(vector["source_sha256"]) + hash_bytes(vector["ir_fingerprint"]) + i(vector["scale"], 4) + bytes(integer(v, 8, False) for v in vector["color"]))
    return fingerprint


def anchor_bytes(source):
    check(len(source["source_span"]) == 3, "anchor original source span width")
    check(source["source_span"][1] <= source["source_span"][2], "anchor original source byte range")
    return u(source["owner"], 4) + b"".join(u(v, 4) for v in source["source_span"]) + u(source["boundary_unit"], 4)


def verify_anchors(probe):
    expected = []
    nonpainting = []
    sources = []
    for fragment in probe["original_fragments"]:
        line = fragment["line"]
        if line is None:
            continue
        for source in line["anchors"]:
            anchor_bytes(source)
            position = source["position"]
            check(position is not None and position["line"] == line["line"], "anchor exact original source line")
            baseline = fragment["bounds"][1] + position["baseline"]
            check(baseline == fragment["baseline"], "anchor original physical line baseline")
            expected.append(dict(owner=source["owner"], fragment=fragment["fragment"], cell=fragment["cell"], repeated=fragment["repeated"], x=fragment["bounds"][0] + position["x"], baseline=baseline))
            sources.append(source)
        if all(item["kind"] == "break" for item in line["items"]):
            nonpainting.append(dict(fragment=fragment["fragment"], cell=fragment["cell"], repeated=fragment["repeated"]))
    check(probe["anchors"] == expected, "every original source anchor actual position")
    check(probe["nonpainting"] == nonpainting, "every selected original nonpainting line")
    check(probe["unpositioned"] == probe["original_unpositioned"], "all original unresolved anchors remain unpositioned")
    for anchor in probe["unpositioned"]:
        check(anchor["position"] is None, "unresolved original anchor cannot invent physical destination")
        anchor_bytes(anchor)
    return sources


def anchor_hash(probe, positioned_sources):
    fingerprint = fold(domain("typaxis.book-2-anchor-display/1"), hash_bytes(probe["source_fingerprint"]))
    fingerprint = fold(fingerprint, u(len(probe["anchors"])))
    for draw, source in zip(probe["anchors"], positioned_sources):
        fingerprint = fold(fingerprint, anchor_bytes(source))
        fragment = probe["original_fragments"][draw["fragment"]]
        data = u(fragment["page"], 4) + u(draw["fragment"]) + i(draw["x"]) + i(draw["baseline"]) + u(fragment["item"]) + u(fragment["definition"] or 0)
        data += b(fragment["definition"] is not None) + u(draw["cell"] or 0, 4) + b(draw["cell"] is not None) + b(draw["repeated"])
        check(len(data) == 51, "positioned anchor canonical width")
        fingerprint = fold(fingerprint, data)
    fingerprint = fold(fingerprint, u(len(probe["unpositioned"])))
    for source in probe["unpositioned"]:
        fingerprint = fold(fingerprint, anchor_bytes(source))
    fingerprint = fold(fingerprint, u(len(probe["nonpainting"])))
    for draw in probe["nonpainting"]:
        fragment = probe["original_fragments"][draw["fragment"]]
        data = u(fragment["owner"], 4) + u(fragment["page"], 4) + u(draw["fragment"]) + rect_bytes(fragment["bounds"])
        data += u(draw["cell"] or 0, 4) + b(draw["cell"] is not None) + b(draw["repeated"])
        check(len(data) == 54, "nonpainting line canonical width")
        fingerprint = fold(fingerprint, data)
    return fingerprint


def reservation_delta(probe):
    math = 1 + len(probe["math"]) + sum(len(d["paint"]["paints"]) for d in probe["math"] if d["paint"]["kind"] == "native")
    text = 1 + sum(1 + len(d["glyphs"]) for d in probe["text"])
    numbers = 1
    for draw in probe["numbers"]:
        original = draw["original"]
        numbers += 1 + 3 * len(original["runs"]) + sum(len(r["glyphs"]) for r in original["runs"])
        numbers += sum(1 + len(c["glyphs"]) for c in draw["clusters"])
    markers = 1 + len(probe["separators"]) + sum(1 + sum(1 + len(c["glyphs"]) for c in d["clusters"]) for d in probe["markers"])
    images = 1 + len(probe["images"])
    anchors = 1 + len(probe["anchors"]) + len(probe["unpositioned"]) + len(probe["nonpainting"])
    merge = 1 + len(probe["paints"])
    membership = 1 + len(probe["original_fragments"])
    return 1 + math + text + numbers + markers + images + anchors + merge + membership


def verify_charges(probe, resources):
    observed = probe["observed"]
    base = resources["limits"]["base"]
    records = integer(observed["display_records"], 64, False)
    work = integer(observed["display_work"], 64, False)
    prefix_records = integer(observed["source_records"], 64, False)
    prefix_work = integer(observed["source_work"], 64, False)
    prior = observed["builder_prior"]
    if prior is not None:
        check(type(prior) is list and len(prior) == 2, "actual constructor prefix observations")
        prefix_records = max(prefix_records, integer(prior[0], 64, False))
        prefix_work = max(prefix_work, integer(prior[1], 64, False))
    check(records == prefix_records + reservation_delta(probe), "exact retained display record reservations")
    check(prefix_work < work <= observed["maximum_work"], "display accepted work prefix and inclusive cap")
    check(records <= base["max_fragments"], "display inclusive record cap")
    source_spool = integer(observed["source_spool"], 64, False)
    check(len(probe["source_canonical"]) <= source_spool <= base["max_spool_bytes"], "retained source canonical spool and inclusive cap")
    scope = observed["scope"]
    check(scope in ("source_callback", "display_callback", "retained_source_cut"), "explicit driver observation scope")
    if scope == "retained_source_cut":
        check(probe["selection_mode"] == "retained_source_cut", "explicit retained source-cut selection evidence")
        check(all(observed[key] is None for key in ("driver_records", "driver_work", "driver_spool", "balance_candidates")), "retained source-cut proof cannot invent driver observations")
        return
    check(probe["selection_mode"] == "automatic_driver", "automatic source pipeline evidence")
    for key in ("driver_records", "driver_work", "driver_spool", "balance_candidates"):
        integer(observed[key], 64, False)
    check(observed["driver_records"] <= base["max_fragments"] and observed["driver_spool"] <= base["max_spool_bytes"], "driver inclusive retained caps")
    check(source_spool <= observed["driver_spool"], "caller retains accepted source terminal spool")
    check(observed["balance_candidates"] <= base["max_column_balance_candidates"], "caller cumulative balance cap")
    lower_records = records if scope == "display_callback" else prefix_records
    lower_work = work if scope == "display_callback" else prefix_work
    check(lower_records <= observed["driver_records"] and lower_work <= observed["driver_work"] <= observed["maximum_work"], "caller ledger retains all applicable accepted charges")


def component_hashes(probe, positioned_sources):
    return {"math": math_hash(probe).hex(), "text": text_hash(probe).hex(), "numbers": number_hash(probe).hex(), "markers": marker_hash(probe).hex(), "images": image_hash(probe).hex(), "anchors": anchor_hash(probe, positioned_sources).hex()}


def verify(probe):
    check(probe["wire"] == SCHEMA, "column display diagnostic schema")
    expected = {"wire", "case", "selection_mode", "source_fingerprint", "source_canonical", "source_package_fingerprint", "source_measurements_fingerprint", "source_coverage", "original_graph", "source_text_buffers", "admitted_fingerprint", "admitted_canonical_jcs", "limits_fingerprint", "limits_canonical_jcs", "fingerprint", "component_fingerprints", "pages", "original_fragments", "text", "math", "numbers", "markers", "images", "separators", "anchors", "nonpainting", "unpositioned", "original_unpositioned", "paints", "observed"}
    check(set(probe) == expected, "closed column display diagnostic fields")
    check(type(probe["case"]) is str and probe["case"] and "/" not in probe["case"], "actual callback case label")
    check(set(probe["observed"]) == {"scope", "maximum_work", "builder_prior", "source_records", "source_work", "source_spool", "display_records", "display_work", "driver_records", "driver_work", "driver_spool", "balance_candidates"}, "closed observation fields")
    check(set(probe["source_coverage"]) == {"semantic_fragments", "repeated_fragments", "header_variant_fragments", "unreferenced_definitions", "semantic_math", "repeated_math"}, "closed source closure fields")
    for value in probe["source_coverage"].values():
        integer(value, 64, False)
    for name in ("source_text_buffers",):
        integer(probe[name], 32, False)
    for name in ("maximum_work", "source_records", "source_work", "source_spool", "display_records", "display_work"):
        integer(probe["observed"][name], 64, False)
    for value in probe["observed"].values():
        if type(value) is int:
            integer(value, 64, False)
    source = decode_source(probe)
    resources = verify_resources(probe, source)
    verify_geometry(probe, source)
    verify_text(probe, resources)
    verify_math(probe, source, resources)
    verify_numbers(probe, source, resources)
    verify_markers(probe, resources)
    verify_images(probe, resources)
    positioned_sources = verify_anchors(probe)
    hashes = component_hashes(probe, positioned_sources)
    check(probe["component_fingerprints"] == hashes, "exact component canonical fingerprints")
    check(merge_hash(probe, hashes) == hash_bytes(probe["fingerprint"]), "complete authored-order column display fingerprint")
    verify_charges(probe, resources)
    return [len(probe["pages"]), len(probe["original_fragments"]), len(probe["paints"]), len(source["variants"]), len(source["balances"])]


def reseal_display(probe):
    # This intentionally recomputes output hashes only. Original immutable
    # receipt/shape metadata remains independent of the altered projected data.
    sources = [a for f in probe["original_fragments"] if f["line"] is not None for a in f["line"]["anchors"]]
    hashes = component_hashes(probe, sources)
    probe["component_fingerprints"] = hashes
    probe["fingerprint"] = merge_hash(probe, hashes).hex()


def tamper_checks(probe):
    actions = []
    def change(path, value, reseal=False):
        def action(changed):
            parent = changed
            for key in path[:-1]:
                parent = parent[key]
            parent[path[-1]] = value
            if reseal:
                reseal_display(changed)
        actions.append((str(path) + (" resealed" if reseal else ""), action))
    def missing(path):
        def action(changed):
            parent = changed
            for key in path[:-1]:
                parent = parent[key]
            del parent[path[-1]]
        actions.append(("missing " + str(path), action))
    missing(["source_coverage"])
    missing(["original_graph", "binding_epoch"])
    missing(["observed", "source_records"])
    change(["wire"], "typaxis.book-2-body-display-probe/1")
    change(["selection_mode"], "fabricated_selection")
    change(["unexpected_field"], True)
    change(["fingerprint"], "00" * 32)
    change(["source_fingerprint"], "00" * 32)
    change(["limits_fingerprint"], "00" * 32)
    change(["admitted_fingerprint"], "00" * 32)
    change(["source_package_fingerprint"], "00" * 32)
    change(["source_measurements_fingerprint"], "00" * 32)
    change(["source_canonical"], probe["source_canonical"][:-1])
    change(["original_graph", "binding_epoch"], "00" * 32)
    change(["observed", "display_records"], probe["observed"]["display_records"] + 1)
    change(["observed", "display_work"], probe["observed"]["source_work"])
    change(["observed", "source_spool"], len(probe["source_canonical"]) - 1)
    change(["source_coverage", "unreferenced_definitions"], probe["source_coverage"]["unreferenced_definitions"] + 1)
    change(["observed", "balance_candidates"], (1 << 16))
    if probe["pages"]:
        value = copy.deepcopy(probe["pages"][0]["columns"])
        value[0]["bounds"][0] += 1
        change(["pages", 0, "columns"], value)
        change(["pages", 0, "columns"], probe["pages"][0]["columns"][:-1])
    if probe["original_fragments"]:
        fragment = probe["original_fragments"][0]
        change(["original_fragments", 0, "owner"], fragment["owner"] + 1)
        change(["original_fragments", 0, "column"], None if fragment["column"] is not None else 0)
        change(["original_fragments", 0, "graph", "line_fingerprint"], "00" * 32)
        for index, original in enumerate(probe["original_fragments"]):
            if original["line"] is not None:
                missing(["original_fragments", index, "line", "owner"])
                break
    for name in ("text", "math", "numbers", "markers", "images"):
        if probe[name]:
            draw = probe[name][0]
            change([name, 0, "owner"], draw["owner"] + 1)
            change([name], probe[name][:-1])
    if probe["text"]:
        draw = probe["text"][0]
        change(["text", 0, "text"], draw["text"] + "x", True)
        span = list(draw["span"])
        span[2] += 1
        change(["text", 0, "span"], span, True)
        if draw["glyphs"]:
            glyphs = copy.deepcopy(draw["glyphs"])
            glyphs[0][0] ^= 1
            change(["text", 0, "glyphs"], glyphs, True)
            glyphs = copy.deepcopy(draw["glyphs"])
            glyphs[0][1] += 1
            change(["text", 0, "glyphs"], glyphs, True)
        if draw["bounds"] is not None:
            bounds = list(draw["bounds"])
            bounds[1] += 1
            change(["text", 0, "bounds"], bounds, True)
    if probe["math"]:
        draw = probe["math"][0]
        missing(["math", 0, "original"])
        change(["math", 0, "baseline"], draw["baseline"] + 1)
        change(["math", 0, "actual_text"], "changed source semantics")
        if draw["paint"]["kind"] == "vector":
            change(["math", 0, "paint", "scale"], draw["paint"]["scale"] + 1, True)
            matrix = list(draw["paint"]["matrix"])
            matrix[4] += 1
            change(["math", 0, "paint", "matrix"], matrix)
        elif draw["paint"]["paints"]:
            paints = copy.deepcopy(draw["paint"]["paints"])
            if "rule" in paints[0]:
                paints[0]["rule"][0] += 1
            else:
                paints[0]["gid"] ^= 1
            change(["math", 0, "paint", "paints"], paints, True)
            change(["math", 0, "original", "computation_fingerprint"], "00" * 32)
    if probe["numbers"] and probe["numbers"][0]["clusters"]:
        cluster = probe["numbers"][0]["clusters"][0]
        if cluster["glyphs"]:
            glyphs = copy.deepcopy(cluster["glyphs"])
            glyphs[0][2] += 1
            change(["numbers", 0, "clusters", 0, "glyphs"], glyphs, True)
    if probe["markers"]:
        missing(["markers", 0, "original", "owner"])
        change(["markers", 0, "owner"], probe["markers"][0]["owner"] + 1, True)
        change(["markers", 0, "baseline"], probe["markers"][0]["baseline"] + 1, True)
        if probe["markers"][0]["clusters"]:
            span = list(probe["markers"][0]["clusters"][0]["span"])
            span[0] += 1
            change(["markers", 0, "clusters", 0, "span"], span, True)
    if probe["images"]:
        bounds = list(probe["images"][0]["viewport"])
        bounds[0] += 1
        change(["images", 0, "viewport"], bounds, True)
        change(["images", 0, "image_sha256"], "00" * 32, True)
    if probe["anchors"]:
        change(["anchors", 0, "x"], probe["anchors"][0]["x"] + 1, True)
        change(["anchors"], probe["anchors"][:-1])
    if probe["nonpainting"]:
        change(["nonpainting"], probe["nonpainting"][:-1])
    if probe["unpositioned"]:
        change(["unpositioned"], probe["unpositioned"][:-1])
    if probe["separators"]:
        ink = list(probe["separators"][0]["ink"])
        ink[0] += 1
        change(["separators", 0, "ink"], ink, True)
    if len(probe["paints"]) > 1:
        paints = list(probe["paints"])
        paints[0], paints[1] = paints[1], paints[0]
        change(["paints"], paints)
    for label, action in actions:
        changed = copy.deepcopy(probe)
        try:
            action(changed)
            verify(changed)
        except (AssertionError, KeyError, ValueError, TypeError, IndexError):
            continue
        raise AssertionError(f"tampered actual probe accepted: {label}")
    return len(actions)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    paths = sorted(args.directory.glob("*.json"))
    check(paths, "no actual column display probes")
    totals = [0] * 5
    rejections = 0
    scopes = {"automatic_driver": 0, "retained_source_cut": 0}
    observations = {"source_callback": 0, "display_callback": 0, "retained_source_cut": 0}
    for path in paths:
        data = path.read_bytes()
        probe = json.loads(data)
        check(path.name == probe["case"] + "-" + digest(data).hex() + ".json", "actual callback artifact filename SHA")
        try:
            counts = verify(probe)
            if args.self_test:
                rejections += tamper_checks(probe)
        except Exception as error:
            raise AssertionError(f"{path}: {error}") from error
        totals = [x + y for x, y in zip(totals, counts)]
        scopes[probe["selection_mode"]] += 1
        observations[probe["observed"]["scope"]] += 1
    print(f"PASS: {len(paths)} actual column displays, {totals[0]} pages, {totals[1]} original fragments, {totals[2]} paints, {totals[3]} actual header graph associations, {totals[4]} balanced source receipts, {rejections} tamper rejections")
    print(f"Selection evidence: {scopes['automatic_driver']} sources selected automatically; {scopes['retained_source_cut']} explicit retained source cuts (no natural driver convergence claim).")
    print("Observation scopes: " + ", ".join(f"{name}={count}" for name, count in observations.items()) + ".")
    print("Budget evidence: exact retained display record reservations; accepted work prefixes and supplied inclusive caps; automatic caller record/work/spool/candidate caps. Retained cuts report no invented driver counters.")
    print("Scope: sealed source/resource identities and original-data display projection; no whole layout/font/vector engine replay or balanced PDF acceptance.")


if __name__ == "__main__":
    main()
