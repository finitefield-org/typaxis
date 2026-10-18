#!/usr/bin/env python3
"""Verify Column marked body fragments against original authored source.

The frozen paint checker verifies physical display/resources/paint contributions.
This checker derives the logical source registry, scope boundaries, local marked
page streams and nonpainting extraction anchors, then replays captured successful
record/spool/output/work reservations, including signed output settlement. It is
not a final Column PDF, selected note reading order, annotation/Headers closure,
or independent layout/shaping/SVG-parser/failure-attempt replay.
"""
from __future__ import annotations

import argparse
import ast
import copy
import json
import re
import sys
import tempfile
from collections import Counter
from pathlib import Path

import verify_book_v2_column_paints as paint
import verify_book_v2_column_display as display
import verify_book_v2_column_resources as resource

check = display.check
integer = display.integer
digest = display.digest
domain = display.domain
hash_bytes = display.hash_bytes
u = display.u
i = display.i
fields = resource.fields
strict_equal = resource.strict_equal
artifact = paint.artifact
PDFBudget = paint.PDFBudget
Sink = paint.Sink

SCHEMA = "typaxis.book-2-column-marked-probe/1"
SCOPES = "typaxis.book-2-marked-scopes/1"
CONTENT = "typaxis.book-2-marked-content/1"
STRUCTURE = "typaxis.book-2-source-structure/1"
ROLES = ("Text", "Label", "Formula", "Figure", "EquationNumber", "Separator")
ARTIFACTS = ("RepeatedHeader", "FootnoteSeparator", "RunningHeader", "RunningFooter")
SLOTS = (
    "Source",
    "ListLabel",
    "ListBody",
    "TableHead",
    "TableBody",
    "Caption",
    "FootnoteLink",
    "FootnoteLabel",
    "ReferenceLink",
    "ReferenceLabel",
)
MAX_U64 = (1 << 64) - 1
ANCHOR_GLYPH = b"1000 0 0 0 1000 1000 d1\n"
ANCHOR_CMAP = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Typaxis) /Ordering (SemanticAnchor) /Supplement 0 >> def\n/CMapName /TypaxisSemanticAnchor def\n/CMapType 2 def\n1 begincodespacerange\n<00> <00>\nendcodespacerange\n1 beginbfchar\n<00> <FFFC>\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"
GRANDFATHERED = (
    "art-lojban",
    "cel-gaulish",
    "en-GB-oed",
    "i-ami",
    "i-bnn",
    "i-default",
    "i-enochian",
    "i-hak",
    "i-klingon",
    "i-lux",
    "i-mingo",
    "i-navajo",
    "i-pwn",
    "i-tao",
    "i-tay",
    "i-tsu",
    "no-bok",
    "no-nyn",
    "sgn-BE-FR",
    "sgn-BE-NL",
    "sgn-CH-DE",
    "zh-guoyu",
    "zh-hakka",
    "zh-min",
    "zh-min-nan",
    "zh-xiang",
)


def canonical_language(raw):
    """Independently derive the admitted registry-free BCP47 spelling."""
    check(
        type(raw) is str and 0 < len(raw) <= 255 and raw.isascii(),
        "bounded ASCII source language tag",
    )
    check(
        re.fullmatch(r"[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*", raw) is not None,
        "source language lexical grammar",
    )
    fixed = {tag.lower(): tag for tag in GRANDFATHERED}
    if raw.lower() in fixed:
        return fixed[raw.lower()]
    parts = raw.split("-")
    if parts[0].lower() == "x":
        check(
            len(parts) > 1 and all(len(part) <= 8 for part in parts[1:]),
            "source private language grammar",
        )
        return raw.lower()
    primary = parts[0]
    check(
        primary.isalpha() and 2 <= len(primary) <= 8, "source primary language grammar"
    )
    result, index = [primary.lower()], 1
    if len(primary) <= 3:
        for _ in range(3):
            if (
                index == len(parts)
                or len(parts[index]) != 3
                or not parts[index].isalpha()
            ):
                break
            result.append(parts[index].lower())
            index += 1
    if index < len(parts) and len(parts[index]) == 4 and parts[index].isalpha():
        result.append(parts[index].title())
        index += 1
    if index < len(parts) and (
        (len(parts[index]) == 2 and parts[index].isalpha())
        or (len(parts[index]) == 3 and parts[index].isdigit())
    ):
        result.append(parts[index].upper())
        index += 1
    variants = set()
    while index < len(parts):
        part = parts[index]
        if not (5 <= len(part) <= 8 or (len(part) == 4 and part[0].isdigit())):
            break
        lower = part.lower()
        check(lower not in variants, "unique source language variants")
        variants.add(lower)
        result.append(lower)
        index += 1
    extensions = {}
    while index < len(parts) and len(parts[index]) == 1 and parts[index].lower() != "x":
        singleton = parts[index].lower()
        check(singleton not in extensions, "unique source language singleton")
        first = index
        index += 1
        while index < len(parts) and 2 <= len(parts[index]) <= 8:
            index += 1
        check(index > first + 1, "nonempty source language extension")
        extensions[singleton] = [part.lower() for part in parts[first:index]]
    private = []
    if index < len(parts) and parts[index].lower() == "x":
        check(
            index + 1 < len(parts)
            and all(len(part) <= 8 for part in parts[index + 1 :]),
            "nonempty source private-use suffix",
        )
        private = [part.lower() for part in parts[index:]]
        index = len(parts)
    check(index == len(parts), "complete source language grammar")
    for singleton in sorted(extensions):
        result.extend(extensions[singleton])
    result.extend(private)
    return "-".join(result)


def source_deriver():
    """Load only the unchanged, trusted source walker, without its pypdf import."""
    path = Path(__file__).with_name("verify_book_v2_source_structure.py")
    raw = path.read_bytes()
    tree = ast.parse(raw, filename=str(path))
    selected = []
    for node in tree.body:
        if isinstance(node, ast.Assign) and any(
            isinstance(target, ast.Name) and target.id in ("KINDS", "ROLES")
            for target in node.targets
        ):
            selected.append(node)
        elif isinstance(node, ast.FunctionDef) and node.name == "source_nodes":
            selected.append(node)
    check(len(selected) == 3, "complete trusted source walker dependencies")
    namespace = {}
    exec(
        compile(ast.Module(body=selected, type_ignores=[]), str(path), "exec"),
        namespace,
    )
    return namespace["source_nodes"], digest(raw).hex()


SOURCE_NODES, SOURCE_DERIVER_SHA = source_deriver()


def inherited_hashes():
    return {
        name: digest(Path(__file__).with_name(name).read_bytes()).hex()
        for name in (
            "verify_book_v2_column_paints.py",
            "verify_book_v2_column_paint_images.py",
            "verify_book_v2_column_resources.py",
            "verify_book_v2_column_display.py",
            "verify_book_v2_source_structure.py",
        )
    }


def optional_index(value, count, label):
    if value is None:
        return None
    result = integer(value, 64, False)
    check(result < count, label)
    return result


def byte_range(value, length, label):
    check(type(value) is list and len(value) == 2, label + " range shape")
    start, end = [integer(n, 64, False) for n in value]
    check(start <= end <= length, label + " bounded range")
    return start, end


def key(value):
    check(type(value) is list and len(value) == 2 and value[1] in SLOTS, "source key")
    return integer(value[0], 32, False), SLOTS.index(value[1])


def fold_string(budget, fp, value):
    check(value is None or type(value) is str, "source string type")
    data = None if value is None else value.encode()
    fp = budget.fold(fp, u(MAX_U64 if data is None else len(data)))
    if data is not None:
        for first in range(0, len(data), 96):
            fp = budget.fold(fp, data[first : first + 96])
    return fp


def authored_nodes(document):
    # Serde's optional caption may be represented as null. The Rust walker
    # skips None; the historical wire helper uses omission for the same case.
    normalized = copy.deepcopy(document)

    def normalize(value):
        if isinstance(value, dict):
            if value.get("kind") == "table" and value.get("caption", False) is None:
                del value["caption"]
            for child in value.values():
                normalize(child)
        elif isinstance(value, list):
            for child in value:
                normalize(child)

    normalize(normalized)
    nodes = SOURCE_NODES(normalized)
    for node in nodes:
        node["language"] = canonical_language(node["language"])
    return nodes


def owner_values(document):
    owners = {}

    def walk(value):
        if isinstance(value, dict):
            if "node_id" in value:
                owner = integer(value["node_id"], 32, False)
                check(owner not in owners, "unique original source node owner")
                owners[owner] = value
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    walk(document)
    return owners


def verify_package(root, probe):
    stage = probe["source_package"]
    # The emitter retains the official package encoder's complete JCS and the
    # exact decoded buffers; no diagnostic draw text replaces those bytes.
    fields(
        stage,
        (
            "canonical_jcs",
            "canonical_sha256",
            "source_fingerprint",
            "flow_fingerprint",
            "wire",
            "text_buffers",
            "max_ast_depth",
            "max_ast_nodes",
        ),
        "original package",
    )
    wire = display.sealed_jcs(stage["canonical_jcs"], stage["canonical_sha256"])
    strict_equal(wire, stage["wire"], "original package JCS/wire correspondence")
    strict_equal(
        stage["canonical_sha256"],
        probe["paint_probe"]["display_probe"]["source_package_fingerprint"],
        "authored package same column source owner",
    )
    check(wire.get("contract") == "typaxis.contract/1.5", "authored Book /2 contract")
    strict_equal(
        stage["canonical_sha256"],
        stage["source_fingerprint"],
        "same original package admission fingerprint",
    )
    strict_equal(
        stage["flow_fingerprint"],
        probe["navigation"]["flow_fingerprint"],
        "same authentic syntax flow owner",
    )
    buffers = []
    for index, descriptor in enumerate(stage["text_buffers"]):
        fields(descriptor, ("id", "bytes"), "decoded source text buffer")
        data = artifact(root, descriptor["bytes"])
        text = data.decode("utf-8")
        buffers.append(data)
        original = wire["text_buffers"][index]
        check(
            descriptor["id"] == index == original["text_id"],
            "dense original decoded text buffer ID",
        )
        strict_equal(
            text,
            original["utf8"],
            "original decoded text buffer bytes inside admitted JCS",
        )
        check(text.encode() == data, "original UTF8 roundtrip")
    check(
        len(buffers) == len(wire["text_buffers"]),
        "all exact original decoded text buffers",
    )
    return (
        wire,
        buffers,
        authored_nodes(wire["document"]),
        owner_values(wire["document"]),
    )


def text_slice(buffers, span):
    if type(span) is list:
        check(len(span) == 3, "selected exact text span shape")
        span = dict(zip(("text_id", "start_byte", "end_byte"), span))
    check(type(span) is dict, "authored text span")
    index = integer(span["text_id"], 32, False)
    check(index < len(buffers), "original text buffer ID")
    start = integer(span["start_byte"], 32, False)
    end = integer(span["end_byte"], 32, False)
    check(start <= end <= len(buffers[index]), "authored text span bounds")
    return buffers[index][start:end].decode("utf-8")


def verify_source_maps(wire, buffers):
    """Bind complete identity-covered original sources without a host re-read."""
    sources = wire["sources"]
    chunks = [[] for _ in sources]
    for index, source in enumerate(sources):
        check(source["source_id"] == index, "dense original source IDs")
        integer(source["utf8_byte_length"], 32, False)
        hash_bytes(source["sha256"])
    for buffer, actual in zip(wire["text_buffers"], buffers):
        cursor = 0
        for mapping in buffer["mappings"]:
            start, end = byte_range(
                [
                    mapping["text_range"]["start_byte"],
                    mapping["text_range"]["end_byte"],
                ],
                len(actual),
                "original decoded/source mapping",
            )
            check(start == cursor, "complete ordered original decoded mappings")
            cursor = end
            data = actual[start:end]
            data.decode("utf-8")
            span = mapping["source_span"]
            kind = mapping["kind"]
            check(
                kind in ("identity", "replacement", "inserted"),
                "original source mapping kind",
            )
            if kind == "inserted":
                check(
                    span is None,
                    "inserted original decoded text has no fabricated source span",
                )
                continue
            check(span is not None, "original non-inserted mapping source span")
            source_id = integer(span["source_id"], 32, False)
            check(source_id < len(sources), "original mapping source ID")
            source_start, source_end = byte_range(
                [span["start_byte"], span["end_byte"]],
                sources[source_id]["utf8_byte_length"],
                "original source span",
            )
            if kind == "identity":
                check(
                    source_end - source_start == len(data),
                    "identity source byte extent",
                )
                chunks[source_id].append((source_start, source_end, data))
        check(cursor == len(actual), "original decoded mappings cover every UTF8 byte")
    complete = 0
    for source, fragments in zip(sources, chunks):
        joined = bytearray()
        gap = False
        for start, end, data in sorted(fragments):
            if start > len(joined):
                gap = True
                break
            overlap = min(len(joined), end) - start
            check(
                bytes(joined[start : start + overlap]) == data[:overlap],
                "overlapping original identity maps agree",
            )
            joined.extend(data[overlap:])
        if not gap and len(joined) == source["utf8_byte_length"]:
            check(
                digest(joined).hex() == source["sha256"],
                "complete original identity-mapped source SHA",
            )
            complete += 1
    return complete, len(sources) - complete


def verify_navigation(probe, wire, nodes, owners):
    nav = fields(
        probe["navigation"],
        ("flow_fingerprint", "languages", "language_children", "ast_node_count"),
        "original syntax navigation",
    )
    limits = json.loads(probe["paint_probe"]["display_probe"]["limits_canonical_jcs"])[
        "base"
    ]
    package = probe["source_package"]
    strict_equal(
        package["max_ast_depth"],
        limits["max_ast_nesting_depth"],
        "same original source AST depth limit",
    )
    strict_equal(
        package["max_ast_nodes"],
        limits["max_ast_nodes"],
        "same original source AST node limit",
    )
    expected, children = [], []

    def enum_name(value):
        return "".join(part.title() for part in value.split("_"))

    for node in nodes:
        if node["key"][1] != "Source":
            continue
        owner = node["key"][0]
        original = owners[owner]
        if "kind" in original:
            kind = original["kind"]
        elif owner == wire["document"]["node_id"]:
            kind = "document"
        elif original in wire["document"]["footnotes"]:
            kind = "footnote_definition"
        elif "term" in original:
            kind = "description_item"
        elif "cells" in original:
            kind = "table_row"
        elif "colspan" in original:
            kind = "table_cell"
        elif "children" in original:
            kind = "description_term"
        elif "text_span" in original:
            children.append(
                dict(
                    owner=owner,
                    parent=node["parent_key"][0],
                    language=node["language"],
                    span=node["span"],
                )
            )
            continue
        else:
            check("blocks" in original, "original implicit list item kind")
            kind = "list_item"
        number = original.get("equation_number")
        expected.append(
            dict(
                owner=owner,
                parent=None if node["parent_key"] is None else node["parent_key"][0],
                kind=enum_name(kind),
                span=node["span"],
                language=node["language"],
                page_region=None,
                number=(
                    None
                    if number is None
                    else dict(
                        owner=number["node_id"],
                        span=[
                            number["span"][name]
                            for name in ("source_id", "start_byte", "end_byte")
                        ],
                    )
                ),
            )
        )
    # Region containers and break records carry AST charges but are excluded
    # from the language registry. Region text and its paragraph/heading inherit
    # the original document language; they are never logical body nodes.
    region_node_count = 0
    for master in wire["page_masters"]["masters"]:
        for name in ("header_content", "footer_content"):
            region = master.get(name)
            if region is None:
                continue
            region_node_count += 1
            for block in region["blocks"]:
                region_node_count += 1 + len(block["children"])
                span = block["span"]
                expected.append(
                    dict(
                        owner=block["node_id"],
                        parent=wire["document"]["node_id"],
                        kind=enum_name(block["kind"]),
                        span=[
                            span[name]
                            for name in ("source_id", "start_byte", "end_byte")
                        ],
                        language=wire["document"]["language"],
                        page_region=region["node_id"],
                        number=None,
                    )
                )
                for inline in block["children"]:
                    if inline["kind"] == "text":
                        span = inline["span"]
                        expected.append(
                            dict(
                                owner=inline["node_id"],
                                parent=block["node_id"],
                                kind="Text",
                                span=[
                                    span[name]
                                    for name in ("source_id", "start_byte", "end_byte")
                                ],
                                language=wire["document"]["language"],
                                page_region=region["node_id"],
                                number=None,
                            )
                        )
    expected.sort(key=lambda entry: entry["owner"])
    check(
        len(nav["languages"]) == len(expected),
        "complete authored language owner registry",
    )
    for actual, wanted in zip(nav["languages"], expected):
        fields(actual, tuple(wanted), "original language owner")
        view = dict(actual)
        language = view.pop("language")
        authored = dict(wanted)
        raw_language = authored.pop("language")
        strict_equal(
            language,
            canonical_language(raw_language),
            "exact source BCP47 canonical spelling and inheritance",
        )
        strict_equal(
            view,
            authored,
            "exact authored language kind/parent/span/number/region owner",
        )
    children.sort(key=lambda entry: entry["owner"])
    check(
        len(nav["language_children"]) == len(children),
        "every authored equation-number language child",
    )
    for actual, wanted in zip(nav["language_children"], children):
        fields(actual, tuple(wanted), "source equation-number child")
        view, authored = dict(actual), dict(wanted)
        strict_equal(
            view.pop("language"),
            canonical_language(authored.pop("language")),
            "equation number inherits exact canonical Formula language",
        )
        strict_equal(view, authored, "equation number original child owner/parent/span")
    table_column_count = 0

    def table_columns(value):
        nonlocal table_column_count
        if isinstance(value, dict):
            if value.get("kind") == "table":
                table_column_count += len(value["columns"])
            for child in value.values():
                table_columns(child)
        elif isinstance(value, list):
            for child in value:
                table_columns(child)

    table_columns(wire["document"])
    bindings = wire["document"].get("number_bindings")
    wire_nodes = (
        len(owners)
        + table_column_count
        + region_node_count
        + sum(
            master.get("column_layout") is not None
            for master in wire["page_masters"]["masters"]
        )
        + 2
        + len(wire["metadata"]["keywords"])
        + len(wire["outline"]["entries"])
        + (0 if bindings is None else 1 + len(bindings))
    )
    ast_count = integer(nav["ast_node_count"], 64, False)
    check(
        wire_nodes <= ast_count <= limits["max_ast_nodes"],
        "original wire plus parsed-math AST charge bound",
    )
    # The parsed native-math AST count belongs to its original computation
    # receipt; this source-registry checker does not reimplement that parser.
    generated = sum(node["key"][1] != "Source" for node in nodes)
    check(
        ast_count + generated <= limits["max_ast_nodes"],
        "all generated source slots fit the authentic cumulative AST cap",
    )


def verify_authored_draws(probe, owners, buffers, wire):
    source = probe["paint_probe"]["display_probe"]
    for draw in source["math"]:
        original = owners[draw["owner"]]
        if original["kind"] in ("inline_math", "display_math"):
            alt = actual = original["speech"]
        else:
            check(
                original["kind"] in ("math_vector", "math_vector_block"),
                "authored Formula kind",
            )
            alt = original["alt"]
            actual = original.get("actual_text")
            if actual is None:
                actual = alt
        strict_equal(draw["alternative"], alt, "original Formula Speech/Alt")
        strict_equal(draw["actual_text"], actual, "original Formula parent ActualText")
    for draw in source["images"]:
        original = owners[draw["owner"]]
        check(
            original["kind"] in ("figure", "vector_figure", "inline_vector"),
            "authored Figure kind",
        )
        strict_equal(
            draw["alternative"], original["alt"], "original Figure alternative"
        )
        if original["kind"] != "figure":
            strict_equal(
                draw["original"]["actual_text"],
                original.get("actual_text"),
                "authored Figure ActualText, without Formula fallback",
            )
    for draw in source["numbers"]:
        original = owners[draw["owner"]]
        parent = owners[draw["parent"]]
        check(
            parent.get("equation_number") == original,
            "equation number exact original Formula child",
        )
        strict_equal(
            draw["text"],
            text_slice(buffers, original["text_span"]),
            "whole authored equation number text",
        )
    for draw in source["text"]:
        if draw["generated"]:
            original = owners[draw["owner"]]
            if original.get("kind") == "reference" and original["format"] == "number":
                bindings = wire["document"].get("number_bindings") or []
                matched = [
                    binding
                    for binding in bindings
                    if binding["anchor_id"] == original["target"]
                ]
                check(
                    len(matched) == 1,
                    "generated reference has one original authored number binding",
                )
                label = text_slice(buffers, matched[0]["text_span"]).encode()
                _, start, end = draw["span"]
                check(
                    0 <= start <= end <= len(label),
                    "selected generated number label extent",
                )
                strict_equal(
                    draw["text"],
                    label[start:end].decode(),
                    "generated source-number cluster exact authored binding text",
                )
            continue
        original = owners[draw["owner"]]
        check(original.get("kind") == "text", "draw belongs to original text source")
        span = dict(zip(("text_id", "start_byte", "end_byte"), draw["span"]))
        strict_equal(
            draw["text"],
            text_slice(buffers, span),
            "selected cluster exact authored UTF8 slice",
        )
        whole = original["text_span"]
        check(
            span["text_id"] == whole["text_id"]
            and whole["start_byte"]
            <= span["start_byte"]
            <= span["end_byte"]
            <= whole["end_byte"],
            "selected cluster remains inside original text owner",
        )


def properties(probe):
    source = probe["paint_probe"]["display_probe"]
    fragments = source["original_fragments"]
    result = []
    role = dict(
        text="Text",
        marker="Label",
        math="Formula",
        image="Figure",
        number="EquationNumber",
        separator="Separator",
    )
    arrays = dict(
        text="text",
        marker="markers",
        math="math",
        image="images",
        number="numbers",
        separator="separators",
    )
    for stream, index in source["paints"]:
        check(stream in arrays, "known source-owned Column paint")
        draw = source[arrays[stream]][index]
        fragment = draw.get("fragment", draw.get("before_fragment"))
        owner = draw.get("owner")
        if stream in ("marker", "image"):
            page = fragments[fragment]["page"]
        else:
            page = draw["page"]
        repeated = draw.get("repeated", False)
        artifact_kind = (
            "RepeatedHeader"
            if repeated
            else ("FootnoteSeparator" if stream == "separator" else None)
        )
        alt = draw.get("alternative") if stream in ("math", "image") else None
        actual = (
            draw.get("actual_text")
            if stream == "math"
            else (
                draw["original"].get("actual_text")
                if stream == "image" and draw["original"]["kind"] == "vector"
                else None
            )
        )
        check(
            stream != "math" or actual is not None, "Formula actual source text exists"
        )
        result.append(
            dict(
                owner=owner,
                page=page,
                fragment=fragment,
                role=role[stream],
                artifact=artifact_kind,
                alt=None if artifact_kind else alt,
                actual=None if artifact_kind else actual,
                stream=stream,
                draw=index,
            )
        )
    return result


def property_key(p):
    return p["owner"], p["page"], p["fragment"], p["role"], p["artifact"]


def visit_groups(properties, budget):
    result = []
    start, previous, next_mcid = 0, None, 0
    while start < len(properties):
        budget.step()
        p = properties[start]
        check(
            previous is None or p["page"] >= previous, "actual source paint page order"
        )
        if p["page"] != previous:
            previous, next_mcid = p["page"], 0
        end = start + 1
        if p["role"] == "Text":
            while end < len(properties):
                budget.step()
                if property_key(properties[end]) != property_key(p):
                    break
                end += 1
        mcid = None if p["artifact"] else next_mcid
        if mcid is not None:
            next_mcid += 1
        result.append(dict(p, paints=[start, end], mcid=mcid))
        start = end
    return result


def begin_bytes(group, sink):
    kind = group["artifact"]
    if kind is not None:
        sink.extend(
            {
                "RepeatedHeader": b"/Artifact << /Type /Pagination /Subtype /Header >> BDC\n",
                "FootnoteSeparator": b"/Artifact << /Type /Layout >> BDC\n",
                "RunningHeader": b"/Artifact << /Type /Pagination /Subtype /Header >> BDC\n",
                "RunningFooter": b"/Artifact << /Type /Pagination /Subtype /Footer >> BDC\n",
            }[kind]
        )
        return
    tag = {
        "Text": b"Span",
        "EquationNumber": b"Span",
        "Label": b"Lbl",
        "Formula": b"Formula",
        "Figure": b"Figure",
    }[group["role"]]
    sink.extend(b"/" + tag + b" << /MCID ")
    sink.unsigned(group["mcid"])
    sink.extend(b" >> BDC\n")
    if group["actual"] is not None:
        sink.extend(b"/Span << /ActualText ")
        sink.utf16(group["actual"], True)
        sink.extend(b" >> BDC\n")


def end_bytes(group):
    return b"EMC\nEMC\n" if group["actual"] is not None else b"EMC\n"


def scope_hash(groups, begin_data, page_count, source_fp, budget):
    fp = budget.fold(domain(SCOPES), hash_bytes(source_fp))
    fp = budget.fold(fp, digest(begin_data))
    for g in groups:
        fp = budget.fold(
            fp,
            bytes([g["owner"] is not None])
            + u(g["owner"] or 0, 4)
            + u(g["page"], 4)
            + u(g["fragment"])
            + bytes(
                [
                    ROLES.index(g["role"]),
                    0 if g["artifact"] is None else ARTIFACTS.index(g["artifact"]) + 1,
                ]
            )
            + b"".join(u(n) for n in (*g["paints"], *g["begin_range"])),
        )
    return budget.fold(fp, u(page_count))


def verify_scopes(root, probe, budget, props):
    stage = fields(
        probe["marked_scopes"], ("fingerprint", "charges", "groups"), "marked scopes"
    )
    counted = visit_groups(props, budget)
    closing = sum(len(end_bytes(g)) for g in counted)
    page_count = len(probe["paint_probe"]["display_probe"]["pages"])
    sink = Sink(budget)
    groups = visit_groups(props, budget)
    for g in groups:
        start = len(sink.bytes)
        begin_bytes(g, sink)
        g["begin_range"] = [start, len(sink.bytes)]
    data = bytes(sink.bytes)
    size = probe["size_of"]
    metadata = len(groups) * size["scope"] + page_count * size["range"]
    budget.reserve(
        len(groups) + page_count + 1, metadata + len(data), len(data) + closing
    )
    budget.step()
    again = Sink(budget)
    second = visit_groups(props, budget)
    for g in second:
        start = len(again.bytes)
        begin_bytes(g, again)
        g["begin_range"] = [start, len(again.bytes)]
    strict_equal(groups, second, "two identical authentic scope projection passes")
    check(bytes(again.bytes) == data, "two identical scope boundary encodings")
    pages, cursor = [], 0
    for page in range(page_count):
        budget.step()
        start = cursor
        while cursor < len(groups) and groups[cursor]["page"] == page:
            budget.step()
            cursor += 1
        pages.append([start, cursor])
    check(cursor == len(groups), "every scope on an actual physical page")
    check(len(stage["groups"]) == len(groups), "all actual source scope groups")
    for reported, g in zip(stage["groups"], groups):
        fields(
            reported,
            (
                "owner",
                "page",
                "fragment",
                "role",
                "artifact",
                "mcid",
                "paints",
                "begin",
                "end",
                "alternative",
                "actual_text",
            ),
            "marked source scope",
        )
        wanted = {
            name: g[name]
            for name in (
                "owner",
                "page",
                "fragment",
                "role",
                "artifact",
                "mcid",
                "paints",
            )
        }
        wanted.update(alternative=g["alt"], actual_text=g["actual"])
        actual = dict(reported)
        check(
            artifact(root, actual.pop("begin")) == data[slice(*g["begin_range"])],
            "actual scope begin property string",
        )
        check(
            artifact(root, actual.pop("end")) == end_bytes(g),
            "actual scope end boundary",
        )
        strict_equal(
            actual, wanted, "source-owned semantic properties and artifact boundaries"
        )
    budget.step((len(data) + 63) // 64 + 1)
    fp = scope_hash(
        groups,
        data,
        page_count,
        probe["paint_probe"]["image_commands"]["fingerprint"],
        budget,
    )
    check(fp == hash_bytes(stage["fingerprint"]), "marked scope canonical hash")
    budget.matches4(stage)
    return groups, pages, data


def anchor_for(probe, group_index, group):
    if group["actual"] is None:
        return None
    check(
        group["paints"][1] - group["paints"][0] == 1,
        "Formula/Figure replacement has one actual paint",
    )
    source = probe["paint_probe"]["display_probe"]
    stream, index = source["paints"][group["paints"][0]]
    if stream == "math":
        draw = source["math"][index]
        if draw["paint"]["kind"] == "native":
            return None
        viewport, baseline = draw["paint"]["viewport"], draw["baseline"]
    elif stream == "image":
        draw = source["images"][index]
        viewport = draw["viewport"]
        baseline = source["original_fragments"][draw["fragment"]]["baseline"]
        check(baseline is not None, "semantic Figure original fragment baseline")
    else:
        return None
    return dict(
        page=group["page"],
        group=group_index,
        paint=group["paints"][0],
        viewport=viewport,
        baseline=baseline,
    )


def encode_anchor_command(sink, anchor):
    x, _, width, height = display.rect(anchor["viewport"])
    sink.extend(b"BT /")
    sink.extend(b"BMA")
    sink.extend(b" 1 Tf 3 Tr 0 Tc 0 Tw 100 Tz 0 TL 0 Ts ")
    sink.number(width)
    sink.extend(b" 0 0 ")
    sink.number(-height)
    sink.push(32)
    sink.number(x)
    sink.push(32)
    sink.number(anchor["baseline"])
    sink.extend(b" Tm <00> Tj ET\n")


def encode_anchor_objects(sink, first, needed):
    objects = []
    if not needed:
        return objects
    base = len(sink.bytes)
    for offset, role in enumerate(("Font", "Glyph", "ToUnicode")):
        sink.budget.step()
        start = len(sink.bytes) - base
        sink.unsigned(first + offset)
        sink.extend(b" 0 obj\n")
        if role == "Font":
            sink.extend(b"<< /Type /Font /Subtype /Type3 /Name /")
            sink.extend(b"BMA")
            sink.extend(
                b" /FontBBox [0 0 1000 1000] /FontMatrix [0.001 0 0 0.001 0 0] /CharProcs << /anchor "
            )
            sink.unsigned(first + 1)
            sink.extend(b" 0 R")
            sink.extend(
                b" >> /Encoding << /Type /Encoding /Differences [0 /anchor] >> /FirstChar 0 /LastChar 0 /Widths [1000] /Resources << >> /ToUnicode "
            )
            sink.unsigned(first + 2)
            sink.extend(b" 0 R")
            sink.extend(b" >>")
        else:
            payload = ANCHOR_GLYPH if role == "Glyph" else ANCHOR_CMAP
            sink.extend(b"<< /Length ")
            sink.unsigned(len(payload))
            sink.extend(b" >>\nstream\n")
            sink.extend(payload)
            sink.extend(b"\nendstream")
        sink.extend(b"\nendobj\n")
        objects.append(
            dict(id=first + offset, role=role, range=[start, len(sink.bytes) - base])
        )
    return objects


def whole_text(probe, group):
    source = probe["paint_probe"]["display_probe"]
    start, end = group["paints"]
    result = []
    for stream, index in source["paints"][start:end]:
        if group["role"] == "Text":
            check(
                stream == "text", "whole selected text owns only exact source clusters"
            )
            result.append(source["text"][index]["text"])
        elif group["role"] == "Label":
            check(
                stream == "marker" and end == start + 1,
                "whole selected label source owner",
            )
            result.append(source["markers"][index]["text"])
        elif group["role"] == "EquationNumber":
            check(
                stream == "number" and end == start + 1,
                "whole selected number source owner",
            )
            result.append(source["numbers"][index]["text"])
        else:
            raise AssertionError("invalid inline whole-text role")
    return "".join(result)


def encode_content(root, probe, groups, pages, begin_data, budget):
    sink = Sink(budget)
    text_stage = probe["paint_probe"]["text_commands"]
    text_data = artifact(root, text_stage["bytes"])
    image_stage = probe["paint_probe"]["image_commands"]
    image_data = artifact(root, image_stage["bytes"])
    text_by_paint = [[] for _ in probe["paint_probe"]["display_probe"]["paints"]]
    for command in text_stage["commands"]:
        text_by_paint[command["paint"]].append(command)
    image_by_paint = {command["paint"]: command for command in image_stage["commands"]}
    check(
        len(image_by_paint) == len(image_stage["commands"]),
        "unique source image occurrence per paint",
    )
    actual_pages, ranges, anchors = [], [], []
    paint_cursor, text_cursor, image_cursor = 0, 0, 0
    for page, page_groups in enumerate(pages):
        budget.step()
        page_start = len(sink.bytes)
        sink.extend(b"q\n")
        for group_index in range(*page_groups):
            g = groups[group_index]
            budget.step()
            check(
                g["paints"][0] == paint_cursor and g["page"] == page,
                "marked groups preserve actual complete paint prefix",
            )
            start = len(sink.bytes)
            sink.extend(b"q\n")
            sink.extend(begin_data[slice(*g["begin_range"])])
            inline = g["artifact"] is None and g["role"] in (
                "Text",
                "Label",
                "EquationNumber",
            )
            if inline:
                check(
                    g["actual"] is None,
                    "whole source text and Formula replacement never nest",
                )
                budget.step(g["paints"][1] - g["paints"][0])
                sink.extend(b"/Span << /ActualText ")
                sink.utf16(whole_text(probe, g), True)
                sink.extend(b" >> BDC\n")
            anchor = anchor_for(probe, group_index, g)
            if anchor is not None:
                budget.step()
                encode_anchor_command(sink, anchor)
                anchors.append(anchor)
            for index in range(*g["paints"]):
                budget.step()
                commands = text_by_paint[index]
                for command in commands:
                    budget.step()
                    check(
                        command == text_stage["commands"][text_cursor]
                        and command["page"] == page,
                        "complete ordered original text-command occurrence",
                    )
                    check(
                        command["usage"] is None
                        or inline
                        or g["actual"] is not None
                        or g["artifact"] is not None,
                        "source parent owns glyph extraction",
                    )
                    sink.extend(
                        text_data[
                            slice(
                                *byte_range(
                                    command["range"], len(text_data), "text command"
                                )
                            )
                        ]
                    )
                    text_cursor += 1
                command = image_by_paint.get(index)
                if command is not None:
                    check(
                        not commands
                        and command == image_stage["commands"][image_cursor]
                        and command["page"] == page
                        and command["owner"] == g["owner"],
                        "distinct original image occurrence and source owner",
                    )
                    sink.extend(
                        image_data[
                            slice(
                                *byte_range(
                                    command["range"], len(image_data), "image command"
                                )
                            )
                        ]
                    )
                    sink.extend(b"\n")
                    image_cursor += 1
                paint_cursor += 1
            if inline:
                sink.extend(b"EMC\n")
            sink.extend(end_bytes(g))
            sink.extend(b"Q\n")
            ranges.append([start, len(sink.bytes)])
        sink.extend(b"Q\n")
        actual_pages.append(
            dict(page=page, groups=page_groups, range=[page_start, len(sink.bytes)])
        )
    check(
        paint_cursor == len(text_by_paint)
        and text_cursor == len(text_stage["commands"])
        and image_cursor == len(image_stage["commands"]),
        "marked body consumes every original physical command exactly once",
    )
    return sink, actual_pages, ranges, anchors


def content_hash(probe, data, pages, ranges, anchors, budget):
    fp = budget.fold(domain(CONTENT), hash_bytes(probe["marked_scopes"]["fingerprint"]))
    fp = budget.fold(
        fp, hash_bytes(probe["paint_probe"]["text_commands"]["fingerprint"])
    )
    fp = budget.fold(fp, digest(data))
    for page in pages:
        fp = budget.fold(
            fp,
            u(page["page"], 4)
            + b"".join(u(n) for n in (*page["groups"], *page["range"])),
        )
    for start, end in ranges:
        fp = budget.fold(fp, u(start) + u(end))
    for anchor in anchors:
        fp = budget.fold(
            fp,
            u(anchor["page"], 4)
            + u(anchor["group"])
            + u(anchor["paint"])
            + display.rect_bytes(anchor["viewport"])
            + i(anchor["baseline"]),
        )
    return fp


def verify_content(root, probe, budget, groups, pages, begin_data):
    stage = fields(
        probe["marked_content"],
        (
            "fingerprint",
            "charges",
            "pages",
            "groups",
            "anchors",
            "anchor_objects",
            "anchor_bytes",
            "body_length",
            "next_object",
            "anchor_font",
        ),
        "marked content",
    )
    previous_output = budget.output
    raw_text = artifact(root, probe["paint_probe"]["text_commands"]["bytes"])
    raw_image = artifact(root, probe["paint_probe"]["image_commands"]["bytes"])
    credit = len(raw_text) + len(raw_image)
    for index, group in enumerate(groups):
        budget.step()
        credit += (
            group["begin_range"][1] - group["begin_range"][0] + len(end_bytes(group))
        )
    token_credit = 0
    for command in probe["paint_probe"]["text_commands"]["commands"]:
        budget.step()
        if command["actual_text"] is not None:
            token_credit += len(artifact(root, command["actual_text"]))
    credit += token_credit
    check(
        credit <= previous_output,
        "first marked credit is part of accepted output prefix",
    )
    first_anchor = max(
        probe["paint_probe"]["font_objects"]["next"],
        probe["paint_probe"]["image_objects"]["next"],
    )
    sink, actual_pages, ranges, anchors = encode_content(
        root, probe, groups, pages, begin_data, budget
    )
    body_length = len(sink.bytes)
    objects = encode_anchor_objects(sink, first_anchor, bool(anchors))
    data = bytes(sink.bytes)
    object_count = len(objects)
    size = probe["size_of"]
    metadata = (
        len(pages) * size["page"]
        + len(groups) * size["range"]
        + len(anchors) * size["anchor"]
        + object_count * size["anchor_object"]
    )
    budget.output -= credit
    budget.reserve(
        len(pages) + len(groups) + len(anchors) + object_count + 1,
        metadata + len(data),
        len(data),
    )
    budget.step()
    second, second_pages, second_ranges, second_anchors = encode_content(
        root, probe, groups, pages, begin_data, budget
    )
    second_objects = encode_anchor_objects(second, first_anchor, bool(anchors))
    check(bytes(second.bytes) == data, "two exact marked encoding passes")
    strict_equal(
        (second_pages, second_ranges, second_anchors, second_objects),
        (actual_pages, ranges, anchors, objects),
        "two identical marked metadata passes",
    )
    check(
        stage["body_length"] == body_length
        and stage["next_object"] == first_anchor + object_count,
        "actual body length and anchor exclusive object cursor",
    )
    strict_equal(
        stage["anchors"],
        anchors,
        "actual nonpainting anchor original physical geometry",
    )
    strict_equal(
        stage["anchor_objects"],
        objects,
        "nonpainting Type3/font/glyph/ToUnicode object topology",
    )
    check(
        artifact(root, stage["anchor_bytes"]) == data[body_length:],
        "exact invisible semantic object bytes",
    )
    check(
        len(stage["pages"]) == len(actual_pages),
        "every actual marked page including blank pages",
    )
    for reported, expected in zip(stage["pages"], actual_pages):
        fields(reported, ("page", "groups", "bytes"), "marked page")
        strict_equal(
            {k: reported[k] for k in ("page", "groups")},
            {k: expected[k] for k in ("page", "groups")},
            "actual marked page range",
        )
        check(
            artifact(root, reported["bytes"]) == data[slice(*expected["range"])],
            "complete actual local q/Q marked page stream",
        )
    check(len(stage["groups"]) == len(ranges), "complete marked group byte fragments")
    for reported, expected in zip(stage["groups"], ranges):
        check(
            artifact(root, reported) == data[slice(*expected)],
            "exact grouped commands and source replacement scopes",
        )
    strict_equal(
        stage["anchor_font"],
        first_anchor if objects else None,
        "only actual vector replacement installs the nonpainting extraction font",
    )
    settlement = fields(
        probe["settlement"],
        (
            "initial_credit",
            "raw_before_output",
            "materialized_length",
            "reserved_output",
            "net_output",
        ),
        "signed marked output settlement",
    )
    strict_equal(
        settlement,
        dict(
            initial_credit=credit,
            raw_before_output=previous_output,
            materialized_length=len(data),
            reserved_output=previous_output - credit + len(data),
            net_output=len(data) - credit,
        ),
        "authoritative first-build signed marked output settlement",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = content_hash(probe, data, actual_pages, ranges, anchors, budget)
    check(fp == hash_bytes(stage["fingerprint"]), "marked body canonical hash")
    budget.matches4(stage)
    return actual_pages, ranges, anchors, data


def visitor_work(document, language_count):
    """Original source traversal, including lookup bounds and generated slots."""
    lookup = 2 + max(0, language_count.bit_length() - 1)
    total = lookup + 1  # root source()/Collect::node

    def generated():
        return lookup + 3  # depth, bounded lookup, AST charge, retained node

    def blocks(values):
        result = 0
        for value in values:
            result += 2  # depth + match traversal
            kind = value["kind"]
            if kind == "page_break":
                continue
            result += lookup + 1
            if kind in ("paragraph", "heading"):
                result += inlines(value["children"])
            elif kind == "semantic_container":
                result += blocks(value["blocks"])
            elif kind == "list":
                for item in value["items"]:
                    result += 1 + lookup + 1 + 2 * generated()
                    result += blocks(item["blocks"])
            elif kind == "description_list":
                for item in value["items"]:
                    result += 2 * (1 + lookup + 1) + generated()
                    result += inlines(item["term"]["children"]) + blocks(item["blocks"])
            elif kind == "table":
                result += 1
                if value.get("caption") is not None:
                    result += generated() + blocks(value["caption"])
                for section in ("head", "body"):
                    result += generated()
                    for row in value[section]:
                        result += 1 + lookup + 1
                        for cell in row["cells"]:
                            result += 1 + lookup + 1 + blocks(cell["blocks"])
            elif kind in ("figure", "vector_figure"):
                if value["caption"]:
                    result += generated() + blocks(value["caption"])
            elif kind == "math_vector_block":
                result += lookup
                if value.get("equation_number") is not None:
                    result += 2  # depth + Collect::node
            else:
                check(
                    kind == "display_math", "supported authored block source traversal"
                )
        return result

    def inlines(values):
        result = 0
        for value in values:
            result += 2
            kind = value["kind"]
            if kind in ("anchor", "soft_break", "hard_break"):
                continue
            result += lookup + 1
            if kind in ("emphasis", "strong", "link"):
                result += inlines(value["children"])
            elif kind in ("reference", "footnote_reference"):
                result += 2 * generated()
        return result

    total += blocks(document["blocks"])
    for note in document["footnotes"]:
        total += 1 + lookup + 1 + 2 * generated() + blocks(note["blocks"])
    return total


def heap_sort_lookup(values, budget):
    values = list(values)

    def sift(root, end):
        while True:
            budget.step()
            child = root * 2 + 1
            if child >= end:
                return
            if child + 1 < end:
                budget.step()
                if values[child][0] < values[child + 1][0]:
                    child += 1
            budget.step()
            if values[root][0] >= values[child][0]:
                return
            budget.step()
            values[root], values[child] = values[child], values[root]
            root = child

    for root in reversed(range(len(values) // 2)):
        sift(root, len(values))
    for end in reversed(range(1, len(values))):
        budget.step()
        values[0], values[end] = values[end], values[0]
        sift(0, end)
    return values


def find_lookup(lookup, wanted, budget):
    low, high = 0, len(lookup)
    while low < high:
        budget.step()
        mid = low + (high - low) // 2
        if lookup[mid][0] < wanted:
            low = mid + 1
        elif lookup[mid][0] > wanted:
            high = mid
        else:
            return lookup[mid][1]
    return None


def structure_hash(probe, nodes, bindings, budget):
    fp = budget.fold(
        hash_bytes(probe["marked_content"]["fingerprint"]), STRUCTURE.encode()
    )
    fp = budget.fold(fp, hash_bytes(probe["navigation"]["flow_fingerprint"]))
    for node in nodes:
        fp = budget.fold(fp, u(node["key"][0], 4))
        fp = budget.fold(fp, u(SLOTS.index(node["key"][1]), 1))
        fp = budget.fold(fp, u(MAX_U64 if node["parent"] is None else node["parent"]))
        for value in (
            node["role"],
            node["semantic_kind"],
            node["language"],
            node["alt"],
        ):
            fp = fold_string(budget, fp, value)
        fp = budget.fold(fp, u(int(node["span"] is not None), 1))
        if node["span"] is not None:
            for value in node["span"]:
                fp = budget.fold(fp, u(value, 4))
        fp = budget.fold(fp, u(int(node["cell"] is not None), 1))
        if node["cell"] is not None:
            for name in ("table", "row", "column", "colspan", "rowspan", "header"):
                fp = budget.fold(fp, u(int(node["cell"][name]), 4))
    for binding in bindings:
        for name in ("node", "group", "page", "mcid"):
            fp = budget.fold(fp, u(binding[name]))
    return fp


def verify_structure(probe, budget, wire, expected, groups):
    stage = fields(
        probe["source_structure"],
        ("fingerprint", "charges", "nodes", "bindings", "group_bindings"),
        "source structure",
    )
    navigation = fields(
        probe["navigation"],
        ("flow_fingerprint", "languages", "language_children", "ast_node_count"),
        "original syntax navigation",
    )
    languages = navigation["languages"]
    count, semantic = len(expected), sum(g["mcid"] is not None for g in groups)
    traversal_work = visitor_work(wire["document"], len(languages))
    budget.step(traversal_work)
    budget.step(len(groups))
    size = probe["size_of"]
    metadata = (
        count * (size["source_node"] + size["lookup"])
        + semantic * size["binding"]
        + len(groups) * size["option_usize"]
    )
    budget.reserve(count * 2 + semantic + len(groups), metadata, 0)
    budget.step(4)
    budget.step(traversal_work)
    nodes = []
    for entry in expected:
        node = dict(entry)
        node.pop("parent_key")
        node.update(
            parent=None,
            depth=1,
            first_child=None,
            next_sibling=None,
            first_binding=None,
        )
        nodes.append(node)
    budget.step(count)
    lookup = heap_sort_lookup(
        [(key(node["key"]), index) for index, node in enumerate(nodes)], budget
    )
    budget.step(max(0, len(lookup) - 1))
    check(len({k for k, _ in lookup}) == count, "unique source-generated keys")
    check(
        expected[0]["key"] == [0, "Source"] and expected[0]["parent_key"] is None,
        "original authored document root",
    )
    last_children = {}
    for index, entry in enumerate(expected):
        budget.step()
        if entry["parent_key"] is not None:
            parent = find_lookup(lookup, key(entry["parent_key"]), budget)
            check(
                parent is not None and parent < index, "source preorder parent registry"
            )
            nodes[index]["parent"] = parent
            nodes[index]["depth"] = nodes[parent]["depth"] + 1
            check(
                nodes[index]["depth"] <= probe["source_package"]["max_ast_depth"],
                "original logical source depth cap",
            )
            if parent in last_children:
                nodes[last_children[parent]]["next_sibling"] = index
            else:
                nodes[parent]["first_child"] = index
            last_children[parent] = index
    for record in languages:
        budget.step()
        index = find_lookup(lookup, (record["owner"], 0), budget)
        check(
            (index is not None) == (record["page_region"] is None),
            "body language present and running region absent from logical structure",
        )
        if index is not None:
            strict_equal(
                nodes[index]["language"],
                record["language"],
                "independently derived exact effective language",
            )
    for child in navigation["language_children"]:
        budget.step()
        index = find_lookup(lookup, (child["owner"], 0), budget)
        check(index is not None, "equation-number source registry entry")
        strict_equal(
            nodes[index]["span"], child["span"], "equation-number original source span"
        )
        strict_equal(
            nodes[index]["language"],
            child["language"],
            "equation-number exact canonical inherited language",
        )
        strict_equal(
            expected[index]["parent_key"],
            [child["parent"], "Source"],
            "equation-number original Formula parent",
        )
    bindings, group_bindings, last_bindings = [], [], {}
    for group_index, group in enumerate(groups):
        budget.step()
        if group["mcid"] is None:
            check(
                group["artifact"] is not None, "artifact scope has no content binding"
            )
            group_bindings.append(None)
            continue
        check(
            group["artifact"] is None and group["owner"] is not None,
            "semantic MCID source owner",
        )
        index = find_lookup(lookup, (group["owner"], 0), budget)
        check(index is not None, "semantic group original Source key")
        if group["role"] == "Label":
            slot = "ListLabel" if nodes[index]["role"] == "LI" else "FootnoteLabel"
            index = find_lookup(lookup, (group["owner"], SLOTS.index(slot)), budget)
            check(index is not None, "label's generated logical source slot")
        if group["role"] == "Text" and nodes[index]["role"] == "Reference":
            index = find_lookup(
                lookup, (group["owner"], SLOTS.index("FootnoteLabel")), budget
            )
            if index is None:
                index = find_lookup(
                    lookup, (group["owner"], SLOTS.index("ReferenceLabel")), budget
                )
            check(index is not None, "reference text belongs to actual Link label")
        allowed = dict(
            Text=("Span", "Reference", "Lbl"),
            Label=("Lbl",),
            Formula=("Formula",),
            Figure=("Figure",),
            EquationNumber=("Span",),
        )
        check(
            nodes[index]["role"] in allowed[group["role"]],
            "semantic scope/source logical role",
        )
        strict_equal(
            nodes[index]["alt"],
            group["alt"],
            "source alternative attached to actual semantic owner",
        )
        binding_index = len(bindings)
        if index in last_bindings:
            bindings[last_bindings[index]]["next"] = binding_index
        else:
            nodes[index]["first_binding"] = binding_index
        last_bindings[index] = binding_index
        bindings.append(
            dict(
                node=index,
                group=group_index,
                page=group["page"],
                mcid=group["mcid"],
                next=None,
            )
        )
        group_bindings.append(binding_index)
    check(
        len(bindings) == semantic,
        "bijection of semantic MCIDs and original source bindings",
    )
    check(
        len(stage["nodes"]) == count,
        "all original source nodes including unpainted definitions",
    )
    for reported, expected_node in zip(stage["nodes"], nodes):
        wanted = dict(expected_node)
        wanted["alternative"] = wanted.pop("alt")
        fields(reported, tuple(wanted), "retained source node")
        strict_equal(
            reported,
            wanted,
            "original source topology/slots/spans/languages/table attributes",
        )
    strict_equal(stage["bindings"], bindings, "ordered source-node MCID binding chains")
    group_views = [
        (
            None
            if index is None
            else [bindings[index][name] for name in ("node", "group", "page", "mcid")]
        )
        for index in group_bindings
    ]
    strict_equal(
        stage["group_bindings"],
        group_views,
        "every actual semantic group bound once and artifact unbound",
    )
    fp = structure_hash(probe, nodes, bindings, budget)
    check(fp == hash_bytes(stage["fingerprint"]), "source hierarchy canonical hash")
    budget.matches4(stage)
    return nodes, bindings


def constructor(probe, index, budget):
    wanted = budget.triple4()
    wanted[0] += 1
    strict_equal(
        probe["observed"]["constructors"][index],
        wanted,
        "accepted marked constructor prefix",
    )
    budget.charge(records=1)


def verify(root, probe, verified_paint=None, prefix_cache=None, capture=None):
    fields(
        probe,
        (
            "wire",
            "case",
            "paint_probe",
            "source_package",
            "navigation",
            "marked_scopes",
            "marked_content",
            "source_structure",
            "settlement",
            "observed",
            "size_of",
        ),
        "Column marked probe",
    )
    check(
        probe["wire"] == SCHEMA and probe["case"] == probe["paint_probe"]["case"],
        "actual Column marked schema/case",
    )
    if verified_paint is None:
        previous = paint.verify(root, probe["paint_probe"])
    else:
        # Adversarial marked tests reuse only the already verified unchanged
        # paint JSON and its immutable original artifacts. No altered ancestor
        # can use this local optimization or gain a historical receipt.
        identity_fast = (
            prefix_cache is not None
            and len(verified_paint) == 3
            and probe["paint_probe"] is verified_paint[2]
        )
        if not identity_fast:
            check(
                digest(display.canonical(probe["paint_probe"]).encode())
                == verified_paint[0],
                "unchanged verified paint ancestor",
            )
        previous = dict(verified_paint[1])
    check(
        prefix_cache is None or verified_paint is not None,
        "only local adversarial runs cache verified ancestors",
    )
    source_key = digest(
        display.canonical([probe["source_package"], probe["navigation"]]).encode()
    )
    source_cached = (
        prefix_cache is not None and prefix_cache["source_key"] == source_key
    )
    if source_cached:
        wire, buffers, source_nodes, owners, complete_sources, unresolved_sources = (
            prefix_cache["source"]
        )
    else:
        wire, buffers, source_nodes, owners = verify_package(root, probe)
        complete_sources, unresolved_sources = verify_source_maps(wire, buffers)
        verify_authored_draws(probe, owners, buffers, wire)
        verify_navigation(probe, wire, source_nodes, owners)
    if capture is not None:
        capture["source_key"] = source_key
        capture["source"] = (
            wire,
            buffers,
            source_nodes,
            owners,
            complete_sources,
            unresolved_sources,
        )
    size = fields(
        probe["size_of"],
        (
            "scope",
            "page",
            "anchor",
            "anchor_object",
            "source_node",
            "lookup",
            "binding",
            "range",
            "option_usize",
        ),
        "actual marked reservation units",
    )
    for value in size.values():
        check(integer(value, 64, False) > 0, "positive actual marked reservation unit")
    observed = fields(
        probe["observed"],
        (
            "scope",
            "maximum_work",
            "caller_before",
            "caller_after",
            "ancestor_callback",
            "constructors",
            "encoded_before",
        ),
        "marked observation",
    )
    check(
        observed["scope"]
        in ("marked_driver", "source_callback", "retained_source_cut"),
        "explicit captured successful marked evidence scope",
    )
    check(
        len(observed["constructors"]) == 11, "all paint and marked stage constructors"
    )
    strict_equal(
        observed["constructors"][:8],
        probe["paint_probe"]["observed"]["constructors"],
        "unchanged complete paint prefix constructors",
    )
    budget = PDFBudget(probe["paint_probe"]["image_commands"]["charges"])
    if observed["scope"] == "marked_driver":
        strict_equal(
            observed["encoded_before"],
            budget.triple4(),
            "actual complete paint prefix before marked encoding",
        )
    else:
        check(
            observed["encoded_before"] is None,
            "manual source does not invent an encoded-driver observation",
        )
    constructor(probe, 8, budget)
    scopes_key = digest(
        display.canonical(
            [probe["marked_scopes"], size, observed["constructors"][:9]]
        ).encode()
    )
    scopes_cached = source_cached and prefix_cache["scopes_key"] == scopes_key
    if scopes_cached:
        groups, pages, begin_data = prefix_cache["scopes"]
        budget = PDFBudget(prefix_cache["scopes_budget"])
    else:
        groups, pages, begin_data = verify_scopes(
            root, probe, budget, properties(probe)
        )
    if capture is not None:
        capture["scopes_key"] = scopes_key
        capture["scopes"] = groups, pages, begin_data
        capture["scopes_budget"] = budget.triple4()
    constructor(probe, 9, budget)
    content_key = digest(
        display.canonical(
            [
                probe["marked_content"],
                probe["settlement"],
                size,
                observed["constructors"][:10],
            ]
        ).encode()
    )
    content_cached = scopes_cached and prefix_cache["content_key"] == content_key
    if content_cached:
        actual_pages, ranges, anchors, data = prefix_cache["content"]
        budget = PDFBudget(prefix_cache["content_budget"])
    else:
        actual_pages, ranges, anchors, data = verify_content(
            root, probe, budget, groups, pages, begin_data
        )
    if capture is not None:
        capture["content_key"] = content_key
        capture["content"] = actual_pages, ranges, anchors, data
        capture["content_budget"] = budget.triple4()
    settled_output = budget.output
    constructor(probe, 10, budget)
    check(
        budget.output == settled_output,
        "source structure inherits settled output without reviving consumed credit",
    )
    nodes, bindings = verify_structure(probe, budget, wire, source_nodes, groups)
    base = json.loads(probe["paint_probe"]["display_probe"]["limits_canonical_jcs"])[
        "base"
    ]
    maximum_work = integer(observed["maximum_work"], 64, False)
    strict_equal(
        maximum_work,
        probe["paint_probe"]["observed"]["maximum_work"],
        "same cumulative marked work cap",
    )
    for quartet in [
        *observed["constructors"],
        *[
            probe[name]["charges"]
            for name in ("marked_scopes", "marked_content", "source_structure")
        ],
    ]:
        r, s, o, w = PDFBudget(quartet).triple4()
        check(
            r <= base["max_fragments"]
            and s <= base["max_spool_bytes"]
            and o <= base["max_output_bytes"]
            and w <= maximum_work,
            "inclusive captured marked stage R/S/O/W limits",
        )
    if observed["scope"] == "marked_driver":
        check(
            probe["paint_probe"]["observed"]["scope"] == "paint_driver",
            "actual marked driver paint phase",
        )
        strict_equal(
            observed["caller_before"],
            probe["paint_probe"]["observed"]["caller_before"],
            "same actual whole caller starting prefix",
        )
        strict_equal(
            observed["caller_after"],
            budget.triple4(),
            "caller receives authoritative settled output and complete success prefix",
        )
        strict_equal(
            observed["ancestor_callback"],
            budget.triple4(),
            "marked driver actual callback observation",
        )
    else:
        check(
            observed["caller_before"] is None and observed["caller_after"] is None,
            "manual marked chain has no complete driver counter claim",
        )
        strict_equal(
            observed["ancestor_callback"],
            probe["paint_probe"]["observed"]["ancestor_callback"],
            "manual source retains the actual ancestor observation separately",
        )
        check(
            observed["scope"] == probe["paint_probe"]["observed"]["scope"],
            "manual marked/source selection lineage",
        )
    return dict(
        previous,
        scope=observed["scope"],
        marked_pages=len(actual_pages),
        groups=len(groups),
        semantic_groups=len(bindings),
        artifact_groups=sum(g["artifact"] is not None for g in groups),
        repeated_headers=sum(g["artifact"] == "RepeatedHeader" for g in groups),
        formula_groups=sum(
            g["role"] == "Formula" and g["artifact"] is None for g in groups
        ),
        equation_numbers=sum(
            g["role"] == "EquationNumber" and g["artifact"] is None for g in groups
        ),
        semantic_anchors=len(anchors),
        anchor_objects=len(probe["marked_content"]["anchor_objects"]),
        source_nodes=len(nodes),
        source_bindings=len(bindings),
        marked_bytes=len(data),
        output_credit=probe["settlement"]["initial_credit"],
        complete_identity_sources=complete_sources,
        unresolved_original_sources=unresolved_sources,
    )


def reseal_marked(root, probe):
    """Reissue only reported stage hashes, never independent expected facts."""
    budget = PDFBudget([0, 0, 0, 0])
    groups, begin = [], bytearray()
    for reported in probe["marked_scopes"]["groups"]:
        g = {
            name: reported[name]
            for name in (
                "owner",
                "page",
                "fragment",
                "role",
                "artifact",
                "mcid",
                "paints",
            )
        }
        g.update(alt=reported["alternative"], actual=reported["actual_text"])
        first = len(begin)
        begin.extend(artifact(root, reported["begin"]))
        g["begin_range"] = [first, len(begin)]
        groups.append(g)
    probe["marked_scopes"]["fingerprint"] = scope_hash(
        groups,
        bytes(begin),
        len(probe["marked_content"]["pages"]),
        probe["paint_probe"]["image_commands"]["fingerprint"],
        budget,
    ).hex()
    content = probe["marked_content"]
    data, pages, ranges = bytearray(), [], []
    for page in content["pages"]:
        first = len(data)
        page_bytes = artifact(root, page["bytes"])
        data.extend(page_bytes)
        pages.append(
            dict(page=page["page"], groups=page["groups"], range=[first, len(data)])
        )
        cursor = first + 2  # actual page q/LF
        for index in range(*page["groups"]):
            length = len(artifact(root, content["groups"][index]))
            ranges.append([cursor, cursor + length])
            cursor += length
    data.extend(artifact(root, content["anchor_bytes"]))
    content["fingerprint"] = content_hash(
        probe, bytes(data), pages, ranges, content["anchors"], budget
    ).hex()
    nodes = []
    for reported in probe["source_structure"]["nodes"]:
        node = dict(reported)
        node["alt"] = node.pop("alternative")
        nodes.append(node)
    probe["source_structure"]["fingerprint"] = structure_hash(
        probe, nodes, probe["source_structure"]["bindings"], budget
    ).hex()
    return probe


def reseal_unchanged_prefix(root, bad, original):
    # This is an adversarial hash reporter, not the positive verifier. Node,
    # binding, counter and language mutations do not change the already sealed
    # physical scope/content dictionaries or their artifact descriptors.
    if (
        bad["marked_scopes"] == original["marked_scopes"]
        and bad["marked_content"] == original["marked_content"]
    ):
        nodes = []
        for reported in bad["source_structure"]["nodes"]:
            node = dict(reported)
            node["alt"] = node.pop("alternative")
            nodes.append(node)
        bad["source_structure"]["fingerprint"] = structure_hash(
            bad, nodes, bad["source_structure"]["bindings"], PDFBudget([0, 0, 0, 0])
        ).hex()
        return bad
    return reseal_marked(root, bad)


def at(value, path):
    for part in path:
        value = value[part]
    return value


def marked_copy(probe):
    # The adversarial cases own new marked/package metadata only. The historical
    # paint ancestor remains the same immutable object and is SHA-checked before
    # any prefix cache can be used; no mutation path below enters it.
    return {
        name: value if name == "paint_probe" else copy.deepcopy(value)
        for name, value in probe.items()
    }


def metadata_tamper_cases(probe):
    """Source facts, dense MCIDs, complete chains and signed ledgers."""

    def case(label, path, value):
        check(
            path[0] != "paint_probe",
            "marked tamper cannot alter its immutable paint ancestor",
        )
        bad = marked_copy(probe)
        container = at(bad, path[:-1])
        container[path[-1]] = value
        return label, bad

    for name in (
        "source_package",
        "navigation",
        "marked_scopes",
        "marked_content",
        "source_structure",
        "settlement",
        "observed",
        "size_of",
    ):
        bad = marked_copy(probe)
        del bad[name]
        yield "missing " + name, bad
        bad = marked_copy(probe)
        bad[name]["unissued_field"] = 0
        yield "unknown " + name, bad
    for name in ("marked_scopes", "marked_content", "source_structure"):
        for axis in range(4):
            path = (name, "charges", axis)
            yield case(f"{name} charge {axis}", path, at(probe, path) + 1)
        yield case(name + " zero hash", (name, "fingerprint"), bytes(32).hex())
        bad = marked_copy(probe)
        del bad[name]["fingerprint"]
        yield "missing " + name + " hash", bad
    for ctor in range(11):
        for axis in range(4):
            path = ("observed", "constructors", ctor, axis)
            yield case(f"constructor {ctor} axis {axis}", path, at(probe, path) + 1)
    for name in probe["settlement"]:
        path = ("settlement", name)
        yield case("signed settlement " + name, path, at(probe, path) + 1)
    yield case(
        "source JCS identity", ("source_package", "source_fingerprint"), bytes(32).hex()
    )
    yield case(
        "source syntax owner", ("source_package", "flow_fingerprint"), bytes(32).hex()
    )
    for index, node in enumerate(probe["source_structure"]["nodes"]):
        prefix = ("source_structure", "nodes", index)
        for name, replacement in (
            ("role", "WrongRole"),
            ("language", "zz"),
            ("alternative", "wrong source alternative"),
            ("semantic_kind", "wrong_semantic"),
            ("depth", node["depth"] + 1),
            ("parent", None if node["parent"] is not None else 0),
            ("first_child", None if node["first_child"] is not None else 0),
            ("next_sibling", None if node["next_sibling"] is not None else 0),
            ("first_binding", None if node["first_binding"] is not None else 0),
        ):
            yield case(f"node {index} {name}", (*prefix, name), replacement)
        yield case(
            f"node {index} source owner", (*prefix, "key", 0), node["key"][0] + 1
        )
        yield case(
            f"node {index} source slot",
            (*prefix, "key", 1),
            "ListBody" if node["key"][1] != "ListBody" else "Source",
        )
        yield case(
            f"node {index} noncanonical language case",
            (*prefix, "language"),
            node["language"].swapcase(),
        )
        if node["span"] is not None:
            yield case(
                f"node {index} source span", (*prefix, "span", 1), node["span"][1] + 1
            )
        if node["cell"] is not None:
            for name in node["cell"]:
                value = node["cell"][name]
                yield case(
                    f"node {index} table {name}",
                    (*prefix, "cell", name),
                    not value if name == "header" else value + 1,
                )
    for index, group in enumerate(probe["marked_scopes"]["groups"]):
        prefix = ("marked_scopes", "groups", index)
        for name, value in (
            ("owner", (group["owner"] or 0) + 1),
            ("page", group["page"] + 1),
            ("fragment", group["fragment"] + 1),
            ("role", "Figure" if group["role"] != "Figure" else "Formula"),
            ("artifact", None if group["artifact"] is not None else "RepeatedHeader"),
            ("mcid", 0 if group["mcid"] is None else group["mcid"] + 1),
            ("alternative", "wrong alt"),
            ("actual_text", "wrong parent text"),
        ):
            yield case(f"group {index} {name}", (*prefix, name), value)
        for boundary in range(2):
            yield case(
                f"group {index} paint boundary {boundary}",
                (*prefix, "paints", boundary),
                group["paints"][boundary] + 1,
            )
    for index, binding in enumerate(probe["source_structure"]["bindings"]):
        for name, value in binding.items():
            path = ("source_structure", "bindings", index, name)
            yield case(
                f"binding {index} {name}", path, 0 if value is None else value + 1
            )
    for index, binding in enumerate(probe["source_structure"]["group_bindings"]):
        if binding is not None:
            yield case(
                f"group {index} binding omission",
                ("source_structure", "group_bindings", index),
                None,
            )
        else:
            yield case(
                f"artifact {index} fabricated binding",
                ("source_structure", "group_bindings", index),
                [0, index, 0, 0],
            )
    for index, anchor in enumerate(probe["marked_content"]["anchors"]):
        for name in ("page", "group", "paint", "baseline"):
            yield case(
                f"anchor {index} {name}",
                ("marked_content", "anchors", index, name),
                anchor[name] + 1,
            )
        for coordinate in range(4):
            yield case(
                f"anchor {index} geometry {coordinate}",
                ("marked_content", "anchors", index, "viewport", coordinate),
                anchor["viewport"][coordinate] + 1,
            )
    for index, obj in enumerate(probe["marked_content"]["anchor_objects"]):
        yield case(
            f"anchor object {index} ID",
            ("marked_content", "anchor_objects", index, "id"),
            obj["id"] + 1,
        )
        yield case(
            f"anchor object {index} role",
            ("marked_content", "anchor_objects", index, "role"),
            "Glyph" if obj["role"] != "Glyph" else "Font",
        )
        yield case(
            f"anchor object {index} range",
            ("marked_content", "anchor_objects", index, "range", 0),
            obj["range"][0] + 1,
        )
    for name in ("body_length", "next_object"):
        yield case(
            "marked " + name,
            ("marked_content", name),
            probe["marked_content"][name] + 1,
        )
    for index, record in enumerate(probe["navigation"]["languages"]):
        for name, value in (
            ("owner", record["owner"] + 1),
            ("parent", 0 if record["parent"] is None else record["parent"] + 1),
            ("kind", "WrongKind"),
            ("language", "zz"),
            ("page_region", 0 if record["page_region"] is None else None),
        ):
            yield case(
                f"language {index} {name}",
                ("navigation", "languages", index, name),
                value,
            )
        yield case(
            f"language {index} noncanonical case",
            ("navigation", "languages", index, "language"),
            record["language"].swapcase(),
        )
    for index, record in enumerate(probe["navigation"]["language_children"]):
        for name in ("owner", "parent"):
            yield case(
                f"number child {index} {name}",
                ("navigation", "language_children", index, name),
                record[name] + 1,
            )
        yield case(
            f"number child {index} language",
            ("navigation", "language_children", index, "language"),
            "zz",
        )
        yield case(
            f"number child {index} noncanonical case",
            ("navigation", "language_children", index, "language"),
            record["language"].swapcase(),
        )
    if probe["navigation"]["languages"]:
        bad = marked_copy(probe)
        record = bad["navigation"]["languages"][0]
        record["language"] = record["language"].swapcase()
        for node in bad["source_structure"]["nodes"]:
            if node["key"][0] == record["owner"]:
                node["language"] = record["language"]
        yield "joint SHA-resealed navigation/source language case", bad
    for stage, name in (
        ("marked_scopes", "groups"),
        ("source_structure", "nodes"),
        ("source_structure", "bindings"),
        ("source_structure", "group_bindings"),
        ("marked_content", "pages"),
        ("marked_content", "groups"),
    ):
        if probe[stage][name]:
            yield case(
                stage + " " + name + " deletion", (stage, name), probe[stage][name][:-1]
            )
            yield case(
                stage + " " + name + " duplication",
                (stage, name),
                probe[stage][name] + [probe[stage][name][-1]],
            )


def artifact_tamper_cases(probe):
    # Every actual artifact class is exercised. Sampling one representative of
    # each boundary/group/page class avoids repeatedly copying identical bytes.
    stage = probe["marked_scopes"]
    for kind in ("begin", "end"):
        for index, group in enumerate(stage["groups"]):
            yield "scope " + kind, ("marked_scopes", "groups", index, kind)
            break
    content = probe["marked_content"]
    for name in ("pages", "groups"):
        if content[name]:
            path = ("marked_content", name, 0)
            yield "marked " + name, (*path, "bytes") if name == "pages" else path
    if content["anchor_bytes"]["length"]:
        yield "semantic Type3 anchor objects", ("marked_content", "anchor_bytes")
    if probe["source_package"]["text_buffers"]:
        yield "original decoded source UTF8", (
            "source_package",
            "text_buffers",
            0,
            "bytes",
        )


def self_test(root, probe):
    previous = paint.verify(root, probe["paint_probe"])
    verified_paint = (
        digest(display.canonical(probe["paint_probe"]).encode()),
        previous,
        probe["paint_probe"],
    )
    prefix_cache = {}
    verify(root, probe, verified_paint, capture=prefix_cache)
    count = 0

    def reject(bad, label, directory=root):
        nonlocal count
        try:
            verify(directory, bad, verified_paint, prefix_cache=prefix_cache)
        except (
            AssertionError,
            KeyError,
            ValueError,
            IndexError,
            TypeError,
            OverflowError,
        ):
            count += 1
            return
        raise AssertionError("accepted marked tamper: " + label)

    for label, bad in metadata_tamper_cases(probe):
        # Missing fields and deletion cases stay missing. Reissuing a hash may
        # itself require absent fields, but must not repair the corrupted wire.
        if not label.startswith("missing ") and "zero hash" not in label:
            try:
                reseal_unchanged_prefix(root, bad, probe)
            except (
                AssertionError,
                KeyError,
                ValueError,
                IndexError,
                TypeError,
                OverflowError,
            ):
                pass
        reject(bad, label)
    with tempfile.TemporaryDirectory(prefix="column-marked-negative-") as directory:
        directory = Path(directory)
        names = set()

        def collect(value):
            if isinstance(value, dict):
                if {"file", "sha256", "length"}.issubset(value):
                    names.add(value["file"])
                for child in value.values():
                    collect(child)
            elif isinstance(value, list):
                for child in value:
                    collect(child)

        collect(probe)
        # Actual copies preserve the historical safe loader's rule that every
        # artifact resolves inside its probe directory; no symlink bypass.
        for name in names:
            source_path = root / name
            check(
                source_path.resolve().parent == root.resolve(),
                "original safe artifact root",
            )
            (directory / name).write_bytes(source_path.read_bytes())
        for label, path in artifact_tamper_cases(probe):
            original = at(probe, path)
            raw = artifact(root, original)
            if not raw:
                continue
            changed = bytearray(raw)
            changed[len(changed) // 2] ^= 1
            relative = "marked-tamper.bin"
            directory.joinpath(relative).write_bytes(changed)
            bad = marked_copy(probe)
            replacement = dict(
                file=relative, sha256=digest(changed).hex(), length=len(changed)
            )
            parent = at(bad, path[:-1])
            parent[path[-1]] = replacement
            reseal_marked(directory, bad)
            reject(bad, label + " SHA-valid resealed bytes", directory)
    check(
        digest(display.canonical(probe["paint_probe"]).encode()) == verified_paint[0],
        "complete shared paint SHA rechecked after every adversarial matrix",
    )
    return count


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probes", type=Path)
    parser.add_argument("--summary", type=Path)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    paths = sorted(args.probes.rglob("*.json"))
    paths = [
        path for path in paths if json.loads(path.read_text()).get("wire") == SCHEMA
    ]
    check(paths, "actual marked probes present")
    counts, scopes, cases = Counter(), Counter(), []
    for path in paths:
        probe = json.loads(path.read_text())
        result = verify(path.parent, probe)
        scopes[result.pop("scope")] += 1
        result.pop("selection_mode", None)
        counts.update(result)
        cases.append(probe["case"])
    summary = dict(
        schema="typaxis.book-2-column-marked-verification/1",
        probes=len(paths),
        cases=cases,
        scopes=dict(scopes),
        totals=dict(counts),
        source_deriver_sha256=SOURCE_DERIVER_SHA,
        inherited_checker_sha256=inherited_hashes(),
        negative_tests=0,
        scope="captured successful Column marked fragments and original source topology; final PDF and failure-attempt replay pending",
        verification_limits=[
            "Alt/Speech is verified in retained scope/source metadata; this kernel does not serialize final structure /Alt properties.",
            "Marked fragments contain local q/Q and existing physical commands; global MediaBox/page flip belongs to the later page/object owner.",
            "Complete identity-mapped source SHA values are reconstructed from exact JCS/decoded buffers, without a host source reread; unresolved mappings are counted explicitly.",
            "The source-flow fingerprint is a linked original issuer fact; full syntax/style/native-math preparation and parsed native-math AST counts are not independently reimplemented.",
            "Original shaping, layout/page kernels and the complete SVG parser are not independently replayed; original CID-CFF positive acceptance remains behind the missing original-font gate.",
            "Only captured successful build counters are independently replayed; constructor/build/callback failures, retries and one-time-credit failure behavior are separate CLI assertions.",
            "A lower settled output contribution does not make a full-driver cap below earlier raw-output peaks admissible.",
            "Final selected source relations, table Headers, footnote reading order, annotations, navigation, running regions and a complete Column PDF remain pending.",
        ],
    )
    if args.self_test:
        for path in paths:
            negatives = self_test(path.parent, json.loads(path.read_text()))
            summary["negative_tests"] += negatives
            print(
                f"marked self-test {path.name}: {negatives} rejected changes",
                file=sys.stderr,
                flush=True,
            )
    if args.summary:
        args.summary.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    print(json.dumps(summary, sort_keys=True))


if __name__ == "__main__":
    main()
