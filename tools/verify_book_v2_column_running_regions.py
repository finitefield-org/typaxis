"""Independent Column running-region evidence checks.

The local shape and line cuts are issuer witnesses. This checker projects their
actual glyphs and validates original source/master associations; it does not
reshape text or replay the whole preparation/pagination engine. A complete
normal result also requires the joined font/CID/text stages. Historical
checkers remain unchanged.
"""

import argparse
import copy
from collections import Counter
import hashlib
import json
import re
from pathlib import Path
import struct
import tempfile

import verify_book_v2_column_display as display
import verify_book_v2_column_resources as resources
import verify_book_v2_column_paints as paints
import verify_book_v2_column_marked as marked
import verify_book_v2_column_paint_images as image_paints

SCHEMA = "typaxis.book-2-column-running-regions-probe/1"
ALGORITHM = "typaxis.book-2-page-region-display/1"
JOIN_ALGORITHM = "typaxis.book-2-column-page-region-join/1"


def check(condition, message):
    if not condition:
        raise AssertionError(message)


def fields(value, names, message):
    check(type(value) is dict and set(value) == set(names), message)
    return value


def integer(value, bits=64, signed=True):
    check(type(value) is int, "integer, without bool substitution")
    low = -(1 << (bits - 1)) if signed else 0
    high = (1 << (bits - int(signed))) - 1
    check(low <= value <= high, "integer representation range")
    return value


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def digest(value):
    return hashlib.sha256(value).digest()


def fingerprint(value):
    check(type(value) is str and len(value) == 64, "32-byte hexadecimal fingerprint")
    result = bytes.fromhex(value)
    check(result.hex() == value, "lowercase canonical fingerprint")
    return result


def rect(value):
    check(type(value) is list and len(value) == 4, "physical rectangle")
    result = [integer(v) for v in value]
    check(result[2] > 0 and result[3] > 0, "positive frame extents")
    return result


def source_slice(wire, span):
    check(type(span) is list and len(span) == 3, "original region text span")
    text_id, start, end = [integer(v, 32, False) for v in span]
    check(start <= end, "original region text byte range")
    buffers = wire["text_buffers"]
    check(text_id < len(buffers), "original region text buffer")
    raw = buffers[text_id]["utf8"].encode()
    check(end <= len(raw), "original region text byte endpoint")
    return raw[start:end].decode()


def selected_master(wire, page, name, charge=None):
    """Original rule ordering, including source-order ties and 1-based parity."""
    integer(page, 32, False)
    check(page < (1 << 32) - 1, "one-based physical page number")
    check(name is None or type(name) is str, "actual selected source page name")
    data = wire["page_masters"]
    master_id = data["default_master_id"]
    winner = None
    for rule in data["selection_rules"]:
        if charge is not None:
            charge(1)
        parity = rule["parity"]
        check(parity in ("any", "odd", "even"), "original authored parity")
        if parity != "any" and (page + 1) % 2 != int(parity == "odd"):
            continue
        first, named = rule["first"], rule["named_page"]
        check(first is None or type(first) is bool, "authored first-page condition")
        if first is not None and first != (page == 0):
            continue
        if named is not None and named != name:
            continue
        priority = (
            (int(named is not None), int(first is not None), int(parity != "any")),
            integer(rule["source_order"], 32, False),
        )
        if winner is None or priority > winner:
            winner, master_id = priority, rule["master_id"]
    masters = data["masters"]
    check(
        [m["master_id"] for m in masters]
        == sorted(set(m["master_id"] for m in masters)),
        "original sorted unique master identities",
    )
    # Typed selection binary-searches the base and advanced master vectors.
    # The wire's single table is the authentic input to both retained views.
    matches = [m for m in masters if m["master_id"] == master_id]
    check(len(matches) == 1, "original selected master exists exactly once")
    if charge is not None:
        for _ in range(2):
            low, high = 0, len(masters)
            while low < high:
                charge(1)
                middle = low + (high - low) // 2
                if masters[middle]["master_id"] < master_id:
                    low = middle + 1
                elif masters[middle]["master_id"] > master_id:
                    high = middle
                else:
                    break
    return matches[0]


def expected_region_set(wire, pages):
    expected = []
    for index, page in enumerate(pages):
        check(page["index"] == index, "complete physical page order including blanks")
        master = selected_master(wire, index, page["named_page"])
        check(
            page["selected_master_id"] == master["master_id"],
            "actual source-selected master",
        )
        check(
            page["full_body"]
            == [master["body"][n] for n in ("x", "y", "width", "height")],
            "authored full body is not a measurement envelope",
        )
        note = master.get("footnote")
        check(
            page["declared_note"]
            == (
                None
                if note is None
                else [note[n] for n in ("x", "y", "width", "height")]
            ),
            "authored full-page note frame",
        )
        for kind in ("header", "footer"):
            source = master.get(kind + "_content")
            if source is None:
                continue
            frame = master[kind]
            check(frame is not None, "authored region content has a frame")
            expected.append(
                dict(
                    page=index,
                    kind=kind,
                    owner=source["node_id"],
                    frame=[frame[n] for n in ("x", "y", "width", "height")],
                    source=source,
                )
            )
    return expected


class ProjectionBudget:
    """Exact projection steps and folds; no upstream shaping-work claim."""

    def __init__(self):
        self.work = 0

    def step(self, count=1):
        self.work += integer(count, 64, False)

    def fold(self, before, payload):
        self.step((32 + len(payload) + 63) // 64)
        return digest(before + payload)


def project_region(probe, local, region, instance_table):
    """Replay two original-line visits and the copied-glyph projection fold."""
    check(
        local["scope"] == "local_source_companion", "honest local region witness scope"
    )
    for key in ("owner", "page", "kind", "frame", "layout_fingerprint"):
        check(local[key] == region[key], "original/projected region association " + key)
    frame = rect(region["frame"])
    wire = probe["source_package"]
    budget = ProjectionBudget()
    budget.step()
    origins = local["origins"]
    items = [item for origin in origins for item in origin["items"]]
    text_items = [
        (origin, item)
        for origin in origins
        for item in origin["items"]
        if item["kind"] == "text"
    ]
    budget.step(len(origins) + len(items))
    slots = 1 + sum(1 + len(item["glyphs"]) for _, item in text_items)
    check(region["owned_records"] == slots, "actual region owned record reservation")
    fp = digest(ALGORITHM.encode())
    for value in (
        region["layout_fingerprint"],
        probe["admitted_fingerprint"],
        probe["limits_fingerprint"],
        probe["binding_epoch"],
    ):
        fp = budget.fold(fp, fingerprint(value))
    budget.step(len(origins) + len(items))
    projected = []
    for origin, item in text_items:
        font = origin["font"]
        check(
            source_slice(wire, item["source"]) == item["text"],
            "original region UTF-8 source text",
        )
        instance = instance_table[integer(item["font_instance"], 32, False)]
        check(
            instance["face_id"] == font["face"]
            and instance["sha256"] == font["sha256"]
            and instance["face_index"] == font["index"],
            "actual local font instance/source face",
        )
        glyphs, original = item["glyphs"], item["original_glyphs"]
        check(len(glyphs) == len(original), "complete local shaped glyph sequence")
        check(
            [g[0] for g in glyphs] == [g["gid"] for g in original],
            "original local glyph IDs/order",
        )
        advance = sum(integer(g["advance_x"]) for g in original)
        check(
            advance == item["advance"] and advance >= 0,
            "original local cluster advance",
        )
        height = integer(font["ascent"]) - integer(font["descent"])
        check(height >= 0, "source font logical height")
        x, y = integer(origin["x"]), integer(origin["y"])
        copied = [
            [integer(gid, 16, False), integer(x + gx), integer(y + gy)]
            for gid, gx, gy in glyphs
        ]
        bounds = (
            None
            if not advance or not height
            else [
                integer(x + item["pen_x"]),
                integer(y + origin["baseline"] - font["ascent"]),
                advance,
                height,
            ]
        )
        budget.step(len(item["text"].encode()) + len(glyphs))
        identity = struct.pack(
            ">8I",
            integer(item["owner"], 32, False),
            integer(origin["paragraph"], 32, False),
            integer(origin["line"], 32, False),
            *[integer(v, 32, False) for v in item["source"]],
            integer(font["face"], 32, False),
            integer(item["font_instance"], 32, False),
        )
        fp = budget.fold(fp, identity)
        raw = item["text"].encode()
        for start in range(0, len(raw), 104):
            fp = budget.fold(fp, raw[start : start + 104])
        for gid, gx, gy in copied:
            fp = budget.fold(fp, struct.pack(">Hqq", gid, gx, gy))
        projected.append(
            dict(
                owner=item["owner"],
                paragraph=origin["paragraph"],
                line=origin["line"],
                text=item["text"],
                source=item["source"],
                font=dict(
                    instance=item["font_instance"],
                    instance_fingerprint=instance["table_fingerprint"],
                    **font,
                ),
                bounds=bounds,
                glyphs=copied,
            )
        )
    check(
        region["draws"] == projected,
        "complete original local glyph projection, without a Body fragment",
    )
    check(region["projection_work"] == budget.work, "actual region projection work")
    check(
        region["fingerprint"] == fp.hex(), "exact running-region projection fingerprint"
    )
    before, after = local["builder_before"], local["builder_after"]
    check(
        after == [before[0] + slots, before[1] + budget.work],
        "genuine local companion builder prefix",
    )
    return slots, budget.work


def cascade(wire, block):
    """Ordinary original paragraph cascade, without line breaking or shaping."""
    rules = wire["style_sheet"]["rules"]
    by_id = {r["style_id"]: r for r in rules}
    check(len(by_id) == len(rules), "unique original style identities")
    winners = {}
    for matched in rules:
        parts = matched["selector"].split(".")
        if parts[0] != block["kind"] or not set(parts[1:]) <= set(block["classes"]):
            continue
        chain, seen = [matched], {matched["style_id"]}
        while chain[-1]["extends"] is not None:
            key = chain[-1]["extends"]
            check(key in by_id and key not in seen, "original style parent graph")
            seen.add(key)
            chain.append(by_id[key])
        for depth, origin in enumerate(reversed(chain)):
            for order, declaration in enumerate(origin["declarations"]):
                priority = (
                    declaration["important"],
                    len(parts) - 1,
                    matched["source_order"],
                    depth,
                    order,
                )
                name = declaration["name"]
                if name not in winners or priority > winners[name][0]:
                    winners[name] = (priority, declaration["value"])
    return {k: v[1] for k, v in winners.items()}


def verify_local_source(probe, local, expected, admitted):
    fields(
        local,
        (
            "scope",
            "owner",
            "page",
            "kind",
            "frame",
            "layout_fingerprint",
            "shape_fingerprint",
            "line_context_fingerprint",
            "reshape_passes",
            "candidate_steps",
            "builder_before",
            "builder_after",
            "origins",
            "clusters",
        ),
        "complete original local region witness",
    )
    for key in ("owner", "page", "kind", "frame"):
        check(local[key] == expected[key], "selected original region " + key)
    fingerprint(local["layout_fingerprint"])
    fingerprint(local["shape_fingerprint"])
    if local["line_context_fingerprint"] is not None:
        fingerprint(local["line_context_fingerprint"])
    check(
        0
        < integer(local["reshape_passes"], 16, False)
        <= admitted["limits"]["base"]["max_line_reshape_passes"],
        "local converged line-pass witness",
    )
    check(
        integer(local["candidate_steps"], 64, False)
        <= probe["observed"]["maximum_work"],
        "local line-candidate witness within supplied ceiling",
    )
    origins = local["origins"]
    check(
        local["clusters"]
        == [v for o in origins for v in o["items"] if v["kind"] == "text"],
        "complete flattened original cluster order",
    )
    blocks = expected["source"]["blocks"]
    sources = {}
    for pi, block in enumerate(blocks):
        check(
            block["kind"] in ("paragraph", "heading"),
            "supported original region block grammar",
        )
        for item in block["children"]:
            check(item["node_id"] not in sources, "unique region source inline owners")
            sources[item["node_id"]] = (pi, block, item)
    seen = Counter()
    spans = {}
    last = (-1, -1)
    for origin in origins:
        fields(
            origin,
            (
                "paragraph_index",
                "paragraph",
                "line",
                "x",
                "y",
                "baseline",
                "font",
                "items",
            ),
            "original region line",
        )
        pi = integer(origin["paragraph_index"], 32, False)
        check(pi < len(blocks), "original local paragraph index")
        block = blocks[pi]
        check(origin["paragraph"] == block["node_id"], "original paragraph owner")
        position = (pi, integer(origin["line"], 32, False))
        check(
            position > last
            and (position[1] == last[1] + 1 if pi == last[0] else position[1] == 0),
            "complete ordered local paragraph lines",
        )
        last = position
        for key in ("x", "y", "baseline"):
            integer(origin[key])
        check(
            origin["x"] >= local["frame"][0] and origin["y"] >= local["frame"][1],
            "local origin uses actual physical frame",
        )
        font = origin["font"]
        if font is not None:
            fields(
                font,
                ("face", "sha256", "index", "size", "ascent", "descent", "line_gap"),
                "original local font",
            )
            display.font_identity(font, admitted["fonts"])
            style = cascade(probe["source_package"], block)
            check(
                style.get("font_size", {}).get("value") == font["size"],
                "font size from original region ordinary style",
            )
            families = style.get("font_family", {}).get("families")
            check(families is not None, "original region font-family declaration")
            matches = [
                f
                for name in families
                for f in admitted["fonts"].values()
                if f["family"] == name
            ]
            check(
                matches and matches[0]["font_face_id"] == font["face"],
                "original ordered font-family resolution",
            )
        for inline, item in enumerate(origin["items"]):
            check(
                item["inline"] == inline and item["owner"] in sources,
                "original line inline/source owner",
            )
            spi, sb, source = sources[item["owner"]]
            check(spi == pi and sb == block, "original region paragraph affiliation")
            seen[item["owner"]] += 1
            if item["kind"] == "text":
                fields(
                    item,
                    (
                        "kind",
                        "inline",
                        "owner",
                        "paragraph",
                        "line",
                        "source",
                        "text",
                        "origin",
                        "baseline",
                        "font_instance",
                        "pen_x",
                        "advance",
                        "glyphs",
                        "original_glyphs",
                    ),
                    "local shaped source cluster",
                )
                check(
                    source["kind"] == "text" and font is not None,
                    "original text source kind/font",
                )
                check(
                    item["paragraph"] == block["node_id"]
                    and item["line"] == origin["line"]
                    and item["origin"] == [origin["x"], origin["y"]]
                    and item["baseline"] == origin["baseline"],
                    "cluster actual local line association",
                )
                text_id, start, end = item["source"]
                source_span = source["text_span"]
                check(
                    text_id == source_span["text_id"]
                    and source_span["start_byte"]
                    <= start
                    < end
                    <= source_span["end_byte"],
                    "cluster subspan of actual original text node",
                )
                check(
                    source_slice(probe["source_package"], item["source"])
                    == item["text"],
                    "cluster actual UTF8 bytes",
                )
                spans.setdefault(item["owner"], []).append((start, end))
                check(
                    len(item["glyphs"]) == len(item["original_glyphs"]) > 0,
                    "actual local glyph array completeness",
                )
                pen_x, pen_y = integer(item["pen_x"]), 0
                for glyph, raw in zip(item["glyphs"], item["original_glyphs"]):
                    fields(
                        raw,
                        ("gid", "advance_x", "advance_y", "offset_x", "offset_y"),
                        "original shaped glyph metrics",
                    )
                    check(
                        glyph
                        == [
                            raw["gid"],
                            pen_x + raw["offset_x"],
                            origin["baseline"] - pen_y - raw["offset_y"],
                        ],
                        "local glyph positions from original advances/offsets",
                    )
                    pen_x += integer(raw["advance_x"])
                    pen_y += integer(raw["advance_y"])
            else:
                fields(
                    item,
                    ("kind", "inline", "owner", "break_kind", "source"),
                    "original local break",
                )
                check(
                    item["kind"] == "break"
                    and source["kind"] in ("soft_break", "hard_break"),
                    "original authored break kind",
                )
                check(
                    item["break_kind"]
                    == {"soft_break": "Allowed", "hard_break": "Mandatory"}[
                        source["kind"]
                    ],
                    "original break semantics",
                )
                span = source["span"]
                check(
                    item["source"]
                    == [span["source_id"], span["start_byte"], span["end_byte"]],
                    "original break source span",
                )
    check(
        set(pi for pi, _ in [(o["paragraph_index"], o["line"]) for o in origins])
        == set(range(len(blocks))),
        "all original region paragraphs including empty lines",
    )
    for owner, (_, _, source) in sources.items():
        if source["kind"] == "text":
            span = source["text_span"]
            cursor = span["start_byte"]
            for start, end in sorted(spans.get(owner, [])):
                check(
                    start == cursor,
                    "exact original source text coverage without gaps/duplicates",
                )
                cursor = end
            check(cursor == span["end_byte"], "all original source bytes projected")
        else:
            check(seen[owner] == 1, "every original region break once")


def verify_base(probe):
    base = probe["base_display"]
    fields(
        base,
        (
            "wire",
            "case",
            "selection_mode",
            "source_fingerprint",
            "source_canonical",
            "source_package_fingerprint",
            "source_measurements_fingerprint",
            "source_coverage",
            "original_graph",
            "source_text_buffers",
            "admitted_fingerprint",
            "admitted_canonical_jcs",
            "limits_fingerprint",
            "limits_canonical_jcs",
            "fingerprint",
            "component_fingerprints",
            "pages",
            "original_fragments",
            "text",
            "math",
            "numbers",
            "markers",
            "images",
            "separators",
            "anchors",
            "nonpainting",
            "unpositioned",
            "original_unpositioned",
            "paints",
            "observed",
        ),
        "complete historical unjoined display leaf",
    )
    check(
        base["wire"] == display.SCHEMA and base["case"] == probe["case"],
        "actual unjoined Column display companion",
    )
    check(
        base["selection_mode"] == "automatic_driver",
        "actual automatically selected underlying Column source",
    )
    observation = fields(
        base["observed"],
        (
            "scope",
            "maximum_work",
            "builder_prior",
            "source_records",
            "source_work",
            "source_spool",
            "display_records",
            "display_work",
            "driver_records",
            "driver_work",
            "driver_spool",
            "balance_candidates",
        ),
        "explicit local companion observations",
    )
    check(
        observation["scope"] == "local_source_companion"
        and observation["builder_prior"] == [0, 0],
        "explicit local companion scope",
    )
    check(
        all(
            observation[k] is None
            for k in (
                "driver_records",
                "driver_spool",
                "driver_work",
                "balance_candidates",
            )
        ),
        "local companion has no invented callback counters",
    )
    for key in (
        "maximum_work",
        "source_records",
        "source_work",
        "source_spool",
        "display_records",
        "display_work",
    ):
        integer(observation[key], 64, False)
    for key in (
        "source_fingerprint",
        "source_package_fingerprint",
        "admitted_fingerprint",
        "admitted_canonical_jcs",
        "limits_fingerprint",
        "limits_canonical_jcs",
    ):
        check(base[key] == probe[key], "genuine unjoined source identity " + key)
    source = display.decode_source(base)
    admitted = display.verify_resources(base, source)
    check(
        admitted["epoch"] == fingerprint(probe["binding_epoch"]),
        "actual source binding epoch",
    )
    display.verify_geometry(base, source)
    display.verify_text(base, admitted)
    display.verify_math(base, source, admitted)
    display.verify_numbers(base, source, admitted)
    display.verify_markers(base, admitted)
    display.verify_images(base, admitted)
    hashes = display.component_hashes(base, display.verify_anchors(base))
    check(
        base["component_fingerprints"] == hashes, "unjoined actual six component folds"
    )
    check(
        display.merge_hash(base, hashes) == fingerprint(base["fingerprint"]),
        "unjoined authored paint order/fingerprint",
    )
    check(
        observation["display_records"]
        == observation["source_records"] + display.reservation_delta(base),
        "exact local display record reservation",
    )
    check(
        observation["source_work"]
        < observation["display_work"]
        <= observation["maximum_work"],
        "local display successful work prefix",
    )
    check(
        len(base["source_canonical"]) <= observation["source_spool"],
        "retained source canonical spool lower bound",
    )
    return source, admitted


def base_page(base, paint):
    tag, index = paint
    draw = base[
        {
            "text": "text",
            "marker": "markers",
            "math": "math",
            "image": "images",
            "number": "numbers",
            "separator": "separators",
        }[tag]
    ][index]
    return (
        base["original_fragments"][draw["fragment"]]["page"]
        if tag in ("marker", "image")
        else draw["page"]
    )


def verify_join(probe):
    base, regions = probe["base_display"], probe["regions"]
    join = probe["join"]
    authored = any(
        m.get("header_content") is not None or m.get("footer_content") is not None
        for m in probe["source_package"]["page_masters"]["masters"]
    )
    if not authored:
        check(
            join is None
            and not regions
            and probe["paints"] == base["paints"]
            and probe["display_fingerprint"] == base["fingerprint"],
            "exact no-authored-region fast path",
        )
        return None, []
    fields(
        join,
        (
            "base_fingerprint",
            "base_records",
            "base_work",
            "prior_records",
            "prior_work",
            "records",
            "work",
            "fingerprint",
        ),
        "actual successful join observation",
    )
    check(
        join["base_fingerprint"] == base["fingerprint"],
        "actual base six-stream content identity",
    )
    check(
        join["base_records"] >= base["observed"]["display_records"]
        and join["base_work"] >= base["observed"]["display_work"],
        "actual driver base retains local source/display lower bounds",
    )
    previous = [join["base_records"], join["base_work"]]
    for region in regions:
        before = [
            region["records"] - region["owned_records"],
            region["work"] - region["projection_work"],
        ]
        check(
            all(before[i] >= previous[i] for i in (0, 1)),
            "actual region receipts retain preceding accepted prefix",
        )
        previous = [region["records"], region["work"]]
    check(
        join["prior_records"] >= previous[0] and join["prior_work"] >= previous[1],
        "join caller retains all actual successful region builds",
    )
    records = max(
        join["base_records"] + sum(r["owned_records"] for r in regions),
        join["prior_records"],
    )
    work = max(
        join["base_work"] + sum(r["projection_work"] for r in regions),
        join["prior_work"],
    )
    work += 1
    for page in probe["pages"]:
        work += 3

        def step(n):
            nonlocal work
            work += n

        selected_master(
            probe["source_package"], page["index"], page["named_page"], step
        )
    expected, locators = [], []
    fp = fingerprint(base["fingerprint"])
    if regions:
        additional = sum(len(r["draws"]) for r in regions)
        records += len(base["paints"]) + additional + len(regions) + 1
        records += additional + 1
        body_cursor, region_cursor = 0, 0
        for page in range(len(probe["pages"])):
            while body_cursor < len(base["paints"]):
                work += 1
                paint = base["paints"][body_cursor]
                actual = base_page(base, paint)
                if actual > page:
                    break
                check(actual == page, "unjoined body page order")
                expected.append(paint)
                body_cursor += 1
            while (
                region_cursor < len(regions) and regions[region_cursor]["page"] == page
            ):
                work += 1
                for draw in range(len(regions[region_cursor]["draws"])):
                    work += 2
                    expected.append(["region", len(locators)])
                    locators.append((region_cursor, draw))
                region_cursor += 1
        check(
            body_cursor == len(base["paints"]) and region_cursor == len(regions),
            "complete body/region merge",
        )
        fp = digest(fp + JOIN_ALGORITHM.encode())
        work += 1
        for region in regions:
            work += 1
            fp = digest(fp + fingerprint(region["fingerprint"]))
    else:
        expected = base["paints"]
    check(
        probe["paints"] == expected,
        "complete page/body/header/footer/draw/locator merge",
    )
    check(
        join["records"] == records and join["work"] == work,
        "exact coexisting paint+locator reservations and join work",
    )
    check(
        join["fingerprint"] == probe["display_fingerprint"] == fp.hex(),
        "exact joined display fingerprint",
    )
    return join, locators


def derive_uses(probe, instances, locators):
    base = probe["base_display"]
    byface = {i["face_id"]: i["id"] for i in instances}
    origins = []
    for paint, (tag, index) in enumerate(base["paints"]):
        if tag == "text":
            font = base["original_fragments"][base["text"][index]["fragment"]]["line"][
                "font"
            ]
        elif tag in ("marker", "number"):
            font = base[{"marker": "markers", "number": "numbers"}[tag]][index][
                "original"
            ]["font"]
        elif tag == "math" and base["math"][index]["original"]["kind"] == "native":
            font = base["math"][index]["original"]
        else:
            continue
        origins.append(
            dict(paint=paint, draw=[tag, index], instance=byface[font["face"]])
        )
    baseuses, baseslots = resources.derive_font_uses(
        dict(display_probe=base, original_font_origins=origins)
    )
    basetext = paints.text_slots(dict(display_probe=base))
    usemap = {}
    for use in baseuses:
        usemap.setdefault(tuple(use["typed_paint"]), []).append(use)
    basemap = {tuple(p): i for i, p in enumerate(base["paints"])}
    uses, slots, textslots = [], [], []
    expected_origins = []
    for paint, (tag, index) in enumerate(probe["paints"]):
        if tag == "region":
            ri, di = locators[index]
            region = probe["regions"][ri]
            draw = region["draws"][di]
            uses.append(
                dict(
                    paint=paint,
                    typed_paint=[tag, index],
                    slot=0,
                    instance=draw["font"]["instance"],
                    source=dict(kind="text", span=draw["source"]),
                    text=draw["text"],
                    glyphs=[g[0] for g in draw["glyphs"]],
                    size=draw["font"]["size"],
                )
            )
            expected_origins.append(
                dict(paint=paint, draw=[tag, index], instance=draw["font"]["instance"])
            )
            slots.append(1)
            textslots.append(
                [dict(page=region["page"], slot=0, glyphs=draw["glyphs"], rule=None)]
            )
        else:
            bi = basemap[(tag, index)]
            slots.append(baseslots[bi])
            textslots.append(basetext[bi])
            for original in usemap.get((tag, index), []):
                use = dict(original)
                use["paint"] = paint
                uses.append(use)
            if (tag, index) in usemap:
                expected_origins.append(
                    dict(
                        paint=paint,
                        draw=[tag, index],
                        instance=usemap[(tag, index)][0]["instance"],
                    )
                )
    resources.strict_equal(
        probe["original_font_origins"],
        expected_origins,
        "all retained typed original font-instance origins including regions",
    )
    return uses, slots, textslots


def verify_selection(probe, instances, uses, slots, budget):
    stage = fields(
        probe["fonts_selection"],
        ("fingerprint", "charges", "fonts", "uses"),
        "joined font selection",
    )
    resources.strict_equal(
        stage["uses"], uses, "all original base and running-region font occurrences"
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
        digest(b"typaxis.book-2-font-selection/1"),
        fingerprint(probe["display_fingerprint"]),
    )
    fp = budget.fold(fp, fingerprint(probe["admitted_fingerprint"]))
    budget.charge(work=len(slots) + sum(slots))
    for use in uses:
        budget.charge(work=len(use["glyphs"]))
        fp = budget.fold(
            fp,
            display.u(use["paint"])
            + display.u(use["slot"])
            + display.u(use["instance"], 4),
        )
    expected = {}
    for instance, gid in resources.charged_sort(glyphs, budget, 1, lambda x: x):
        budget.step()
        if gid in expected.get(instance, []):
            continue
        if instance not in expected:
            original = instances[instance]
            for payload in (
                fingerprint(original["sha256"]),
                fingerprint(original["table_fingerprint"]),
                display.u(instance, 4),
            ):
                fp = budget.fold(fp, payload)
            expected[instance] = []
        expected[instance].append(gid)
        fp = budget.fold(fp, display.u(gid, 2))
    check(len(stage["fonts"]) == len(expected), "complete original selected font union")
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
            "selected original font",
        )
        check(
            font["instance"] == instance and font["selected"] == selected,
            "unique original GID union includes all running glyphs",
        )
        for key in ("table_fingerprint", "face_id", "face_index"):
            check(
                font[key] == instances[instance][key],
                "selected actual font instance " + key,
            )
        check(
            font["source"]["sha256"] == instances[instance]["sha256"],
            "original font source bytes identity",
        )
    check(
        stage["fingerprint"] == fp.hex(), "joined font selection exact canonical fold"
    )
    budget.matches(stage)


def verify_text_commands(root, probe, streams, slots, budget):
    stage = fields(
        probe["text_commands"],
        ("fingerprint", "charges", "bytes", "commands"),
        "running text commands",
    )
    adapter = dict(probe, display_probe=dict(paints=probe["paints"]))
    budget.step(len(slots))
    data, commands, ranges = paints.encode_text(adapter, slots, streams, budget)
    budget.reserve(
        len(commands) + len(slots) + 1,
        len(commands) * probe["size_of"]["text_command"]
        + len(slots) * probe["size_of"]["range"]
        + len(data),
        len(data),
    )
    budget.step()
    check(
        paints.encode_text(adapter, slots, streams, budget) == (data, commands, ranges),
        "both physical text encoding passes",
    )
    check(
        len(stage["commands"]) == len(commands),
        "all base/running glyph command occurrences",
    )
    for actual, expected in zip(stage["commands"], commands):
        fields(
            actual,
            ("paint", "typed_paint", "slot", "usage", "page", "range", "actual_text"),
            "original running text command",
        )
        view = dict(actual)
        token = view.pop("actual_text")
        reference = dict(expected)
        interval = reference.pop("actual_text")
        check(view == reference, "physical glyph slot/page/CID/byte range")
        check((token is None) == (interval is None), "occurrence ActualText presence")
        if interval is not None:
            check(
                paints.artifact(root, token) == streams[0][interval[0] : interval[1]],
                "original exact ActualText token",
            )
    check(
        paints.artifact(root, stage["bytes"]) == data,
        "independent physical running Tf/Tm/Tj bytes",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        digest(b"typaxis.book-2-text-commands/1"),
        fingerprint(probe["font_objects"]["fingerprint"]),
    )
    fp = budget.fold(fp, digest(data))
    for command in commands:
        fp = budget.fold(
            fp,
            display.u(command["paint"])
            + bytes([int(command["slot"] is not None)])
            + display.u(command["slot"] or 0)
            + bytes([int(command["usage"] is not None)])
            + display.u(command["usage"] or 0)
            + display.u(command["page"], 4)
            + display.u(command["range"][0])
            + display.u(command["range"][1]),
        )
    for start, end in ranges:
        fp = budget.fold(fp, display.u(start) + display.u(end))
    check(stage["fingerprint"] == fp.hex(), "running text command canonical fold")
    budget.matches4(stage)
    return len(commands)


def verify_font_metrics(probe, sources):
    faces = {s["selected"]["face_id"]: s for s in sources}
    for local in probe["original_region_lines"]:
        for origin in local["origins"]:
            font = origin["font"]
            if font is None:
                continue
            # Empty lines need not select a font resource. A nonempty region
            # necessarily binds the original bytes in the selected union.
            if font["face"] not in faces:
                check(
                    not origin["items"], "used region font retained in selected union"
                )
                continue
            hhea = faces[font["face"]]["tables"][b"hhea"]
            units = faces[font["face"]]["selected"]["units_per_em"]
            for key, offset in (("ascent", 4), ("descent", 6), ("line_gap", 8)):
                # Python round is exact integer rational ties-to-even.
                raw = (
                    int.from_bytes(hhea[offset : offset + 2], "big", signed=True)
                    * font["size"]
                )
                q, r = divmod(abs(raw), units)
                value = q + int(2 * r > units or (2 * r == units and q % 2))
                value = -value if raw < 0 else value
                check(font[key] == value, "original sfnt hhea scaled metric " + key)


def verify_semantics(probe, locators):
    check(
        len(probe["semantic_paints"]) == len(probe["paints"]),
        "complete semantic paint observations",
    )
    region_owners = set()

    def collect(value):
        if isinstance(value, dict):
            if "node_id" in value:
                region_owners.add(value["node_id"])
            for child in value.values():
                collect(child)
        elif isinstance(value, list):
            for child in value:
                collect(child)

    for region in expected_region_set(probe["source_package"], probe["pages"]):
        collect(region["source"])
    base = probe["base_display"]
    base_properties = marked.properties(dict(paint_probe=dict(display_probe=base)))
    base_properties = {
        tuple(paint): p for paint, p in zip(base["paints"], base_properties)
    }
    for i, (tag, index) in enumerate(probe["paints"]):
        value = probe["semantic_paints"][i]
        if tag != "region":
            p = base_properties[(tag, index)]
            alt, actual = p["alt"], p["actual"]
            if tag == "math":
                alt, actual = (
                    base["math"][index]["alternative"],
                    base["math"][index]["actual_text"],
                )
            elif tag == "image":
                image = base["images"][index]
                alt = image["alternative"]
                actual = (
                    image["original"].get("actual_text")
                    if image["original"]["kind"] == "vector"
                    else None
                )
            resources.strict_equal(
                value,
                dict(
                    paint=i,
                    page=p["page"],
                    owner=p["owner"],
                    fragment=p["fragment"],
                    artifact=p["artifact"],
                    alternative=alt,
                    actual_text=actual,
                ),
                "complete original body semantic metadata",
            )
            continue
        ri, di = locators[index]
        region = probe["regions"][ri]
        draw = region["draws"][di]
        expected = dict(
            paint=i,
            page=region["page"],
            owner=draw["owner"],
            fragment=ri,
            artifact="RunningHeader" if region["kind"] == "header" else "RunningFooter",
            alternative=None,
            actual_text=None,
        )
        check(
            value == expected,
            "first running occurrence is pagination artifact with no body fragment",
        )
        region_owners.update([region["owner"], draw["owner"], draw["paragraph"]])
        matching = [
            g
            for g in probe["marked_scopes"]["groups"]
            if g["paints"][0] <= i < g["paints"][1]
        ]
        check(len(matching) == 1, "each region paint marked once")
        group = matching[0]
        check(
            group["page"] == region["page"]
            and group["artifact"] == expected["artifact"]
            and group["mcid"] is None
            and group["source_binding"] is None,
            "running artifact has no MCID/source-tree binding",
        )
    check(
        not region_owners.intersection(probe["source_structure_owners"]),
        "running source omitted from semantic structure",
    )
    check(
        not region_owners.intersection(probe["navigation_owners"]),
        "running source omitted from body navigation",
    )


def merged_data_view(probe):
    """Borrow already verified source arrays and actual joined paint ordering.

    This data view feeds private independent encoders, never a Body receipt or
    the historical top-level display verifier. Its region occurrences remain
    their distinct typed 'region' keys throughout selection and byte encoding.
    """
    view = dict(probe["base_display"])
    view["paints"] = probe["paints"]
    view["fingerprint"] = probe["display_fingerprint"]
    return dict(
        probe,
        display_probe=view,
        observed=dict(constructors=probe["downstream"]["constructors"][:8]),
    )


def verify_image_stages(root, probe, admitted, pdf, ctor):
    adapter = merged_data_view(probe)
    resources.verify_original_images(root, adapter, admitted)
    boundary = fields(
        probe["image_boundary"],
        ("text_prefix", "resource_prior", "vector_prior_output"),
        "actual encoded text-before-image handoff",
    )
    resources.strict_equal(
        boundary["text_prefix"], pdf.triple4(), "actual completed text prefix"
    )
    resources.strict_equal(
        boundary["resource_prior"],
        [pdf.records, pdf.spool, pdf.work],
        "single CFF-session next-stage source budgets after text",
    )
    check(
        boundary["vector_prior_output"] == pdf.output,
        "image/vector output begins after actual text",
    )
    ctor(4)
    image_budget = resources.Budget([pdf.records, pdf.spool, pdf.work])
    resources.verify_images(root, adapter, admitted, image_budget)
    raster = resources.verify_rasters(root, adapter, image_budget)
    pdf.records, pdf.spool, pdf.work = image_budget.triple()
    counts = image_paints.verify_image_paints(root, adapter, pdf)
    check(
        pdf.triple4() == probe["downstream"]["encoded_prefix"],
        "all actual text and image command contributions before scope constructor",
    )
    return fingerprint(probe["image_commands"]["fingerprint"]), dict(
        counts, raster_programs=sum(p is not None for p in probe["rasters"]["programs"])
    )


def verify_scope_projection(probe, pdf, command_fp, locators):
    fields(
        probe["marked_scopes"],
        ("fingerprint", "charges", "groups"),
        "complete marked-scope contribution",
    )
    base = probe["base_display"]
    ordinary = marked.properties(dict(paint_probe=dict(display_probe=base)))
    properties = {tuple(p): v for p, v in zip(base["paints"], ordinary)}
    props = []
    for tag, index in probe["paints"]:
        if tag == "region":
            ri, di = locators[index]
            region = probe["regions"][ri]
            draw = region["draws"][di]
            props.append(
                dict(
                    owner=draw["owner"],
                    page=region["page"],
                    fragment=ri,
                    role="Text",
                    artifact=(
                        "RunningHeader"
                        if region["kind"] == "header"
                        else "RunningFooter"
                    ),
                    alt=None,
                    actual=None,
                    stream="region",
                    draw=index,
                )
            )
        else:
            props.append(properties[(tag, index)])
    counted = marked.visit_groups(props, pdf)
    closing = sum(len(marked.end_bytes(g)) for g in counted)
    sink = paints.Sink(pdf)
    groups = marked.visit_groups(props, pdf)
    for g in groups:
        start = len(sink.bytes)
        marked.begin_bytes(g, sink)
        g["begin_range"] = [start, len(sink.bytes)]
    data = bytes(sink.bytes)
    pdf.reserve(
        len(groups) + len(probe["pages"]) + 1,
        len(groups) * probe["size_of"]["scope"]
        + len(probe["pages"]) * probe["size_of"]["range"]
        + len(data),
        len(data) + closing,
    )
    pdf.step()
    second = paints.Sink(pdf)
    again = marked.visit_groups(props, pdf)
    for g in again:
        start = len(second.bytes)
        marked.begin_bytes(g, second)
        g["begin_range"] = [start, len(second.bytes)]
    check(
        groups == again and bytes(second.bytes) == data,
        "two exact marked scope source visits",
    )
    cursor = 0
    for page in range(len(probe["pages"])):
        pdf.step()
        while cursor < len(groups) and groups[cursor]["page"] == page:
            pdf.step()
            cursor += 1
    check(cursor == len(groups), "all actual running and body scope pages")
    actual = probe["marked_scopes"]["groups"]
    check(
        len(actual) == len(groups),
        "complete original body and first-running scope groups",
    )
    for reported, want in zip(actual, groups):
        fields(
            reported,
            ("owner", "page", "artifact", "mcid", "paints", "source_binding"),
            "actual scope source association",
        )
        for key in ("owner", "page", "artifact", "mcid", "paints"):
            resources.strict_equal(
                reported[key], want[key], "exact original scope " + key
            )
        if want["artifact"] is not None:
            check(
                reported["source_binding"] is None,
                "artifact scope has no source binding",
            )
    pdf.step((len(data) + 63) // 64 + 1)
    fp = marked.scope_hash(groups, data, len(probe["pages"]), command_fp.hex(), pdf)
    check(
        fp.hex() == probe["marked_scopes"]["fingerprint"],
        "original scope boundaries and canonical fold",
    )
    pdf.matches4(probe["marked_scopes"])


def verify_downstream(root, probe, admitted, instances, locators, join):
    stage = fields(
        probe["downstream"],
        (
            "encoded_prefix",
            "marked_prefix",
            "relations_prefix",
            "caller_before",
            "constructors",
            "content",
            "structure",
            "relations",
            "navigation",
        ),
        "actual downstream issuer prefixes",
    )
    ctors = stage["constructors"]
    caller = paints.PDFBudget(stage["caller_before"])
    check(len(ctors) == 13, "all thirteen actual constructor observations")
    for c in ctors:
        paints.PDFBudget(c)
    first = ctors[0]
    check(
        caller.records < first[0] and caller.work <= first[3],
        "caller source/display floors retained before first constructor",
    )
    if join is not None:
        check(
            first[0] == join["records"] + 1 and first[3] == join["work"],
            "actual joined display font constructor prefix",
        )
    else:
        check(
            first[0] >= probe["base_display"]["observed"]["display_records"] + 1
            and first[3] >= probe["base_display"]["observed"]["display_work"],
            "no-region actual source floor",
        )
    check(
        first[1]
        == max(
            probe["base_display"]["observed"]["source_spool"], stage["caller_before"][1]
        )
        and first[2] == stage["caller_before"][2],
        "unchanged source spool/caller output before fonts",
    )
    uses, slots, textslots = derive_uses(probe, instances, locators)
    budget = resources.Budget([first[0], first[1], first[3]])
    verify_selection(probe, instances, uses, slots, budget)
    sources = resources.verify_closures(root, probe, budget)
    resources.verify_programs(root, probe, sources, budget)
    resources.verify_cids(probe, uses, sources, budget)
    verify_font_metrics(probe, sources)
    pdf = paints.PDFBudget([budget.records, budget.spool, first[2], budget.work])

    def ctor(index, extra_work=0):
        pdf.charge(records=1, work=extra_work)
        check(
            ctors[index] == pdf.triple4(),
            "accepted downstream constructor prefix " + str(index),
        )

    ctor(1)
    streams = paints.verify_font_streams(root, probe, pdf)
    ctor(2)
    paints.verify_font_objects(root, probe, streams, pdf)
    ctor(3)
    commands = verify_text_commands(root, probe, streams, textslots, pdf)
    command_fp, image_counts = verify_image_stages(root, probe, admitted, pdf, ctor)
    ctor(8)
    verify_scope_projection(probe, pdf, command_fp, locators)
    ctor(9)
    # The wire intentionally retains later Content/source/relations/navigation
    # arrays as issuer facts; their complete record sets live in the separately
    # validated structural wire rather than being invented by this checker.
    for value in (
        stage["encoded_prefix"],
        probe["marked_scopes"]["charges"],
        stage["content"],
        stage["structure"],
        stage["relations"],
        stage["navigation"],
        stage["marked_prefix"],
        stage["relations_prefix"],
    ):
        paints.PDFBudget(value)
    lower = pdf.triple4()
    for index, c in enumerate(ctors[10:], 10):
        check(
            all(c[i] >= lower[i] for i in (0, 1, 3)),
            "accepted downstream R/S/W prefix " + str(index),
        )
        lower = c
    check(
        stage["structure"] == stage["marked_prefix"]
        and stage["relations"] == stage["relations_prefix"],
        "actual completed typed source/relations handoffs",
    )
    check(
        stage["content"][2]
        == stage["structure"][2]
        == stage["relations"][2]
        == stage["navigation"][2],
        "settled Content output is unchanged by source/relations/navigation",
    )
    check(
        ctors[10]
        == [
            stage["content"][0] + 1,
            stage["content"][1],
            stage["content"][2],
            stage["content"][3],
        ],
        "Source constructor has no output credit restoration",
    )
    check(
        ctors[11]
        == [
            stage["structure"][0] + 1,
            stage["structure"][1],
            stage["structure"][2],
            stage["structure"][3],
        ],
        "Relation constructor exact completed source budget",
    )
    check(
        ctors[12]
        == [
            stage["relations"][0] + 1,
            stage["relations"][1],
            stage["relations"][2],
            stage["relations"][3],
        ],
        "Navigation constructor exact completed relations budget",
    )
    observed = probe["observed"]
    check(
        [observed[k] for k in ("records", "spool", "output", "work")]
        == stage["navigation"],
        "actual navigation callback accepted final quartet",
    )
    return (
        len(uses),
        sum(len(f["selected"]) for f in probe["fonts_selection"]["fonts"]),
        commands,
        image_counts,
    )


TOP_FIELDS = (
    "wire",
    "case",
    "source_package",
    "source_canonical_jcs",
    "source_package_fingerprint",
    "source_fingerprint",
    "binding_epoch",
    "admitted_canonical_jcs",
    "admitted_fingerprint",
    "display_fingerprint",
    "limits_canonical_jcs",
    "limits_fingerprint",
    "observed",
    "base_display",
    "original_font_instances",
    "original_font_origins",
    "original_images",
    "original_page_names",
    "pages",
    "original_region_lines",
    "join",
    "paints",
    "semantic_paints",
    "regions",
    "fonts_selection",
    "closures",
    "programs",
    "cids",
    "font_streams",
    "font_objects",
    "text_commands",
    "images_selection",
    "rasters",
    "vector_programs",
    "image_objects",
    "image_commands",
    "image_boundary",
    "marked_scopes",
    "source_structure_owners",
    "navigation_owners",
    "downstream",
    "size_of",
)


def verify(root, probe):
    fields(probe, TOP_FIELDS, "closed running-region wire")
    check(
        probe["wire"] == SCHEMA
        and type(probe["case"]) is str
        and probe["case"]
        and "/" not in probe["case"],
        "actual region callback schema/case",
    )

    def artifact_fields(value):
        if isinstance(value, dict):
            if "file" in value and "sha256" in value:
                resources.verify_artifact_fields(value)
            else:
                for child in value.values():
                    artifact_fields(child)
        elif isinstance(value, list):
            for child in value:
                artifact_fields(child)

    for name in (
        "fonts_selection",
        "programs",
        "font_streams",
        "font_objects",
        "text_commands",
        "original_images",
        "images_selection",
        "rasters",
        "vector_programs",
        "image_objects",
        "image_commands",
    ):
        artifact_fields(probe[name])
    check(
        canonical(probe["source_package"]) == probe["source_canonical_jcs"],
        "complete original source JCS bytes",
    )
    check(
        digest(probe["source_canonical_jcs"].encode())
        == fingerprint(probe["source_package_fingerprint"]),
        "original package canonical SHA",
    )
    source, admitted = verify_base(probe)
    fields(
        probe["observed"],
        ("scope", "maximum_work", "records", "spool", "output", "work", "line_passes"),
        "actual region driver observation",
    )
    check(
        probe["observed"]["scope"] == "navigation_driver",
        "explicit successful navigation driver scope",
    )
    sizes = fields(
        probe["size_of"],
        (
            "paint",
            "usize",
            "locator",
            "font_glyph",
            "sfnt_rewrite_table",
            "selected_font",
            "selected_font_use",
            "closed_font",
            "font_program",
            "cid_font",
            "binding",
            "cid_use",
            "cid",
            "font_stream",
            "font_object",
            "text_command",
            "scope",
            "option_range",
            "range",
            "selected_image",
            "selected_image_use",
            "raster_slot",
            "vector_slot",
            "vector_state",
            "image_object",
            "image_command",
        ),
        "actual diagnostic reservation units",
    )
    check(
        all(integer(v, 64, False) > 0 for v in sizes.values()),
        "actual positive reservation sizes",
    )
    check(
        sizes["usize"] in (4, 8)
        and sizes["locator"] == 2 * sizes["usize"]
        and sizes["cid"] == 2,
        "actual locator/pointer/CID footprint",
    )
    check(
        sizes["paint"] == 2 * sizes["usize"], "unchanged two-word typed paint footprint"
    )
    names = probe["original_page_names"]
    check(
        type(names) is list
        and len(names) == len(set(names))
        and all(type(n) is str and n for n in names),
        "original unique source page-name registry",
    )
    declared = {
        d["value"]["value"]
        for r in probe["source_package"]["style_sheet"]["rules"]
        for d in r["declarations"]
        if d["name"] == "page"
    }
    check(
        set(names) <= declared, "retained source page names originate in authored style"
    )
    check(
        len(probe["pages"]) == len(source["pages"]),
        "all physical pages including blanks",
    )
    for page, actual in zip(probe["pages"], source["pages"]):
        fields(
            page,
            (
                "index",
                "named_page",
                "selected_master_id",
                "full_body",
                "declared_note",
                "columns",
                "balance_fingerprint",
            ),
            "actual selected page",
        )
        check(
            page["columns"] == [c["bounds"] for c in actual["columns"]]
            and page["declared_note"] == actual["footnote"]
            and page["full_body"] == actual["body"],
            "actual source-terminal frames",
        )
        check(
            (page["named_page"] is None) == (actual["named_index"] is None),
            "actual selected named-source presence",
        )
        check(
            page["named_page"]
            == (
                None
                if actual["named_index"] is None
                else probe["original_page_names"][actual["named_index"]]
            ),
            "exact original source-page name registry binding",
        )
        balance = next(
            (b for b in source["balances"] if b["page"] == page["index"]), None
        )
        check(
            page["balance_fingerprint"]
            == (None if balance is None else balance["fingerprint"]),
            "actual sealed balance occurrence",
        )
    expected = expected_region_set(probe["source_package"], probe["pages"])
    check(
        len(expected) == len(probe["regions"]) == len(probe["original_region_lines"]),
        "complete selected region/local witness set",
    )
    instances = resources.verify_font_instances(
        dict(
            original_font_instances=probe["original_font_instances"],
            display_probe=probe["base_display"],
        ),
        admitted,
    )
    local_prefix = [2 * len(admitted["fonts"]) + 1] * 2
    for local, region, want in zip(
        probe["original_region_lines"], probe["regions"], expected
    ):
        fields(
            region,
            (
                "page",
                "owner",
                "kind",
                "frame",
                "fingerprint",
                "layout_fingerprint",
                "records",
                "work",
                "owned_records",
                "projection_work",
                "draws",
            ),
            "complete owned region projection",
        )
        resources.strict_equal(
            local["builder_before"],
            local_prefix,
            "persistent zero-prior local projection builder prefix",
        )
        verify_local_source(probe, local, want, admitted)
        project_region(probe, local, region, instances)
        local_prefix = local["builder_after"]
    join, locators = verify_join(probe)
    verify_semantics(probe, locators)
    body_owners = set()

    def owners(value):
        if isinstance(value, dict):
            if "node_id" in value:
                body_owners.add(value["node_id"])
            for child in value.values():
                owners(child)
        elif isinstance(value, list):
            for child in value:
                owners(child)

    owners(probe["source_package"]["document"])
    for name in ("source_structure_owners", "navigation_owners"):
        check(
            type(probe[name]) is list
            and all(integer(o, 32, False) in body_owners for o in probe[name]),
            "actual body source owner namespace " + name,
        )
    uses, gids, commands, image_counts = verify_downstream(
        root, probe, admitted, instances, locators, join
    )
    caps = admitted["limits"]["base"]
    observed = probe["observed"]
    check(
        0 < len(probe["pages"]) <= caps["max_pages"],
        "inclusive complete physical-page cap",
    )
    for key in ("maximum_work", "records", "spool", "output", "work", "line_passes"):
        integer(observed[key], 64, False)
    check(
        observed["maximum_work"] == probe["base_display"]["observed"]["maximum_work"]
        and observed["line_passes"] <= caps["max_line_reshape_passes"],
        "same work ceiling and cumulative line-pass cap",
    )
    check(
        sum(local["reshape_passes"] for local in probe["original_region_lines"])
        <= observed["line_passes"],
        "actual driver retains all original selected-region reshape passes",
    )
    for values in [
        *probe["downstream"]["constructors"],
        *[
            probe[k]["charges"]
            for k in (
                "font_streams",
                "font_objects",
                "text_commands",
                "vector_programs",
                "image_objects",
                "image_commands",
                "marked_scopes",
            )
        ],
        *[
            probe["downstream"][k]
            for k in (
                "encoded_prefix",
                "content",
                "structure",
                "relations",
                "navigation",
            )
        ],
    ]:
        r, s, o, w = paints.PDFBudget(values).triple4()
        check(
            r <= caps["max_fragments"]
            and s <= caps["max_spool_bytes"]
            and o <= caps["max_output_bytes"]
            and w <= observed["maximum_work"],
            "all captured accepted four-axis stage peaks within caps",
        )
    for name in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "images_selection",
        "rasters",
    ):
        r, s, w = resources.Budget(probe[name]["charges"]).triple()
        check(
            r <= caps["max_fragments"]
            and s <= caps["max_spool_bytes"]
            and w <= observed["maximum_work"],
            "captured successful resource stage caps",
        )
    for font, program in zip(
        probe["fonts_selection"]["fonts"], probe["programs"]["fonts"]
    ):
        check(
            len(font["selected"]) <= caps["max_cids_per_font"]
            and program["program"]["length"]
            <= admitted["limits"]["extension"]["max_font_subset_bytes"],
            "original CID/subset caps",
        )
    check(
        0
        < probe["font_objects"]["first"]
        < probe["font_objects"]["next"]
        <= caps["max_pdf_objects"] + 1
        or not probe["font_objects"]["objects"]
        and 0
        < probe["font_objects"]["first"]
        == probe["font_objects"]["next"]
        <= caps["max_pdf_objects"] + 1,
        "actual exclusive font object cap",
    )
    check(
        observed["records"] <= caps["max_fragments"]
        and observed["spool"] <= caps["max_spool_bytes"]
        and observed["output"] <= caps["max_output_bytes"]
        and observed["work"] <= observed["maximum_work"],
        "inclusive actual callback caps",
    )
    return dict(
        pages=len(probe["pages"]),
        regions=len(expected),
        region_draws=len(locators),
        paints=len(probe["paints"]),
        font_uses=uses,
        unique_gids=gids,
        text_commands=commands,
        navigation_prefix=probe["downstream"]["navigation"],
        **image_counts,
    )


REJECTION_ERRORS = (
    AssertionError,
    KeyError,
    IndexError,
    ValueError,
    TypeError,
    struct.error,
    UnicodeError,
)


def reseal_outputs(root, probe):
    """Recompute output-only seals, keeping the original source/local witnesses.

    This intentionally does not manufacture alternate line cuts or shaped
    glyphs. Those remain authentic issuer inputs and are never called a second
    independently shaped result by this checker.
    """
    for region in probe["regions"]:
        fp = digest(ALGORITHM.encode())
        for value in (
            region["layout_fingerprint"],
            probe["admitted_fingerprint"],
            probe["limits_fingerprint"],
            probe["binding_epoch"],
        ):
            fp = display.fold(fp, fingerprint(value))
        for draw in region["draws"]:
            payload = struct.pack(
                ">8I",
                draw["owner"],
                draw["paragraph"],
                draw["line"],
                *draw["source"],
                draw["font"]["face"],
                draw["font"]["instance"],
            )
            fp = display.fold(fp, payload)
            raw = draw["text"].encode()
            for start in range(0, len(raw), 104):
                fp = display.fold(fp, raw[start : start + 104])
            for gid, x, y in draw["glyphs"]:
                fp = display.fold(fp, struct.pack(">Hqq", gid, x, y))
        region["fingerprint"] = fp.hex()
    fp = fingerprint(probe["base_display"]["fingerprint"])
    if probe["regions"]:
        fp = digest(fp + JOIN_ALGORITHM.encode())
        for region in probe["regions"]:
            fp = display.fold(fp, fingerprint(region["fingerprint"]))
    probe["display_fingerprint"] = fp.hex()
    if probe["join"] is not None:
        probe["join"]["fingerprint"] = fp.hex()
    adapter = merged_data_view(probe)
    resources.reseal_resources(adapter)
    paints.reseal_font_paints(root, adapter)
    image_paints.reseal_image_paints(root, adapter)


def tamper_cases(probe):
    """Deterministic output/affiliation/observed-budget controls."""
    cases = []

    def change(label, path, value, reseal=True):
        cases.append((label, path, value, reseal))

    for name in (
        "source_package_fingerprint",
        "source_fingerprint",
        "binding_epoch",
        "admitted_fingerprint",
        "limits_fingerprint",
        "display_fingerprint",
    ):
        change(name + " seal", [name], bytes(32).hex(), False)
    change(
        "extra unselected region",
        ["regions"],
        probe["regions"] + [probe["regions"][0]] if probe["regions"] else [dict()],
        False,
    )
    if probe["regions"]:
        for key in (
            "page",
            "owner",
            "frame",
            "layout_fingerprint",
            "fingerprint",
            "owned_records",
            "projection_work",
        ):
            value = probe["regions"][0][key]
            replacement = (
                value + 1
                if type(value) is int
                else (
                    [value[0] + 1, *value[1:]]
                    if type(value) is list
                    else bytes(32).hex()
                )
            )
            change(
                "actual region " + key,
                ["regions", 0, key],
                replacement,
                key != "fingerprint",
            )
        change("missing selected region", ["regions"], probe["regions"][:-1], True)
    for name in ("pages", "original_region_lines", "paints", "semantic_paints"):
        if probe[name]:
            change("incomplete " + name, [name], probe[name][:-1], False)
    if probe["pages"]:
        for key in (
            "named_page",
            "selected_master_id",
            "full_body",
            "declared_note",
            "columns",
            "balance_fingerprint",
        ):
            value = probe["pages"][0][key]
            replacement = (
                "unselected-region-master"
                if key in ("named_page", "selected_master_id")
                else (
                    bytes(32).hex()
                    if key == "balance_fingerprint"
                    else (None if value is not None else [1, 2, 3, 4])
                )
            )
            change("source-selected page " + key, ["pages", 0, key], replacement, False)
    for ri, region in enumerate(probe["regions"]):
        if not region["draws"]:
            continue
        draw = region["draws"][0]
        for key in ("owner", "paragraph", "line", "source", "text", "bounds"):
            value = draw[key]
            replacement = (
                value + 1
                if type(value) is int
                else (
                    value + "!"
                    if type(value) is str
                    else (
                        None
                        if value is not None and key == "bounds"
                        else (
                            [1, 2, 3, 4]
                            if key == "bounds"
                            else [value[0], value[1], value[2] + 1]
                        )
                    )
                )
            )
            change(
                "resealed region draw " + key,
                ["regions", ri, "draws", 0, key],
                replacement,
            )
        for key in (
            "instance",
            "instance_fingerprint",
            "face",
            "sha256",
            "index",
            "size",
            "ascent",
            "descent",
            "line_gap",
        ):
            value = draw["font"][key]
            change(
                "resealed original font " + key,
                ["regions", ri, "draws", 0, "font", key],
                value + 1 if type(value) is int else bytes(32).hex(),
            )
        for axis in range(3):
            glyph = list(draw["glyphs"][0])
            glyph[axis] += 1
            change(
                "resealed glyph " + str(axis),
                ["regions", ri, "draws", 0, "glyphs", 0],
                glyph,
            )
        local = probe["original_region_lines"][ri]
        for oi, origin in enumerate(local["origins"]):
            if not origin["items"]:
                continue
            item = origin["items"][0]
            change(
                "original local owner affiliation",
                ["original_region_lines", ri, "origins", oi, "items", 0, "owner"],
                item["owner"] + 1,
                False,
            )
            change(
                "original local missing inline",
                ["original_region_lines", ri, "origins", oi, "items"],
                origin["items"][:-1],
                False,
            )
            break
        break
    for i, (tag, index) in enumerate(probe["paints"]):
        if tag != "region":
            continue
        for key, value in (
            ("owner", probe["semantic_paints"][i]["owner"] + 1),
            ("page", probe["semantic_paints"][i]["page"] + 1),
            ("fragment", probe["semantic_paints"][i]["fragment"] + 1),
            ("artifact", None),
            ("actual_text", "counterfeit"),
            ("alternative", "counterfeit"),
        ):
            change(
                "first running artifact " + key,
                ["semantic_paints", i, key],
                value,
                False,
            )
        change("locator wrong occurrence", ["paints", i], ["region", index + 1], False)
        break
    for i, group in enumerate(probe["marked_scopes"]["groups"]):
        if group["artifact"] not in ("RunningHeader", "RunningFooter"):
            continue
        for key, value in (
            ("mcid", 0),
            ("source_binding", group["owner"]),
            ("artifact", None),
            ("owner", group["owner"] + 1),
        ):
            change(
                "running semantic refusal " + key,
                ["marked_scopes", "groups", i, key],
                value,
                False,
            )
        break
    if probe["regions"]:
        owner = probe["regions"][0]["owner"]
        for key in ("source_structure_owners", "navigation_owners"):
            change("running owner excluded " + key, [key], probe[key] + [owner], False)
    for name in (
        "fonts_selection",
        "closures",
        "programs",
        "cids",
        "font_streams",
        "font_objects",
        "text_commands",
        "images_selection",
        "rasters",
        "vector_programs",
        "image_objects",
        "image_commands",
        "marked_scopes",
    ):
        change(name + " fingerprint", [name, "fingerprint"], bytes(32).hex(), False)
        for axis, value in enumerate(probe[name]["charges"]):
            if value:
                change(
                    name + " accepted axis " + str(axis),
                    [name, "charges", axis],
                    value - 1,
                    False,
                )
    for i, values in enumerate(probe["downstream"]["constructors"]):
        for axis, value in enumerate(values):
            if value:
                change(
                    "constructor accepted prefix " + str((i, axis)),
                    ["downstream", "constructors", i, axis],
                    value - 1,
                    False,
                )
    if probe["fonts_selection"]["uses"]:
        use = probe["fonts_selection"]["uses"][0]
        for key in ("paint", "slot", "instance", "size"):
            change(
                "resealed selected occurrence " + key,
                ["fonts_selection", "uses", 0, key],
                use[key] + 1,
            )
        change(
            "resealed selected source GID",
            ["fonts_selection", "uses", 0, "glyphs", 0],
            use["glyphs"][0] + 1,
        )
        change(
            "resealed CID occurrence",
            ["cids", "uses", 0, "cids", 0],
            probe["cids"]["uses"][0]["cids"][0] + 1,
        )
    if probe["font_objects"]["objects"]:
        change(
            "resealed font object role",
            ["font_objects", "objects", 0, "role"],
            "Program",
        )
    if probe["text_commands"]["commands"]:
        change(
            "resealed physical command page",
            ["text_commands", "commands", 0, "page"],
            probe["text_commands"]["commands"][0]["page"] + 1,
        )
    return cases


def tamper_checks(root, probe, missing_seen):
    rejected = 0
    original = digest(canonical(probe).encode())

    def reject(bad, label):
        nonlocal rejected
        try:
            verify(root, bad)
        except REJECTION_ERRORS:
            rejected += 1
        else:
            raise AssertionError("running-region tamper accepted: " + label)

    for path in resources.schema_paths(probe):
        if path[0] == "base_display" and len(path) > 1:
            # Existing display graph metadata stays under the unchanged strict
            # leaf validators and their retained historical mutation proof.
            continue
        normalized = tuple(0 if type(v) is int else v for v in path)
        if normalized in missing_seen:
            continue
        missing_seen.add(normalized)
        bad = copy.deepcopy(probe)
        container, key = resources.location(bad, path)
        del container[key]
        reject(bad, "missing " + ".".join(map(str, path)))
    for path in (
        (),
        ("join",),
        *(("regions", i) for i in range(min(1, len(probe["regions"])))),
        *(
            ("original_region_lines", i)
            for i in range(min(1, len(probe["original_region_lines"])))
        ),
    ):
        original_value = probe
        for key in path:
            original_value = original_value[key]
        if type(original_value) is not dict:
            continue
        bad = copy.deepcopy(probe)
        target = bad
        for key in path:
            target = target[key]
        target["unissued_extra_field"] = 0
        reject(bad, "extra closed metadata " + str(path))
    for label, path, value, reseal in tamper_cases(probe):
        bad = copy.deepcopy(probe)
        container, key = resources.location(bad, path)
        container[key] = value
        if reseal:
            reseal_outputs(root, bad)
        reject(bad, label)
    for label, adapter in image_paints.image_tamper_cases(merged_data_view(probe)):
        bad = {key: adapter[key] for key in TOP_FIELDS if key != "observed"}
        bad["observed"] = copy.deepcopy(probe["observed"])
        if "missing fingerprint" not in label:
            reseal_outputs(root, bad)
        reject(bad, "image helper resealed " + label)
    check(
        digest(canonical(probe).encode()) == original,
        "original complete callback unchanged by local controls",
    )
    return rejected


def artifact_tamper_checks(root, probe):
    cases = []

    def changed_program(raw):
        entries = [
            struct.unpack(">4sIII", raw[at : at + 16])
            for at in range(12, 12 + 16 * int.from_bytes(raw[4:6], "big"), 16)
        ]
        glyf = next(e for e in entries if e[0] == b"glyf")
        if glyf[3]:
            return paints.altered_subset_glyph(raw)
        # Some authentic diagnostic fonts contain no outlines. Mutate their
        # real advance table while preserving every sfnt/whole checksum.
        data = bytearray(raw)
        hmtx = next(e for e in entries if e[0] == b"hmtx")
        check(hmtx[3] >= 4, "actual nonempty advance table")
        data[hmtx[2] + 1] ^= 1
        head = next(e[2] for e in entries if e[0] == b"head")
        data[head + 8 : head + 12] = bytes(4)
        for index, (_, _, offset, length) in enumerate(entries):
            data[16 + 16 * index : 20 + 16 * index] = display.u(
                resources.checksum(bytes(data[offset : offset + length])), 4
            )
        data[head + 8 : head + 12] = display.u(
            (0xB1B0AFBA - resources.checksum(bytes(data))) & 0xFFFFFFFF, 4
        )
        check(
            resources.checksum(bytes(data)) == 0xB1B0AFBA,
            "negative advance program valid checksum",
        )
        return bytes(data)

    if probe["programs"]["fonts"]:
        cases.append(
            (
                "valid-checksum subset glyph",
                ("programs", "fonts", 0, "program"),
                changed_program,
            )
        )
    if probe["font_streams"]["fonts"]:

        def changed_cmap(raw):
            match = re.search(rb"beginbfchar\n<[0-9A-F]{4}> <([0-9A-F]+)>", raw)
            if match:
                start, end = match.span(1)
                token = bytearray(raw[start:end])
                token[-1] = ord("1") if token[-1] != ord("1") else ord("2")
                return raw[:start] + bytes(token) + raw[end:]
            # A fully ambiguous font can legitimately have no Unicode claims.
            # Keep the CMap syntax valid while changing its declared code range.
            return raw.replace(b"<FFFF>", b"<FFFE>", 1)

        cases.extend(
            (
                (
                    "original Unicode CMap",
                    ("font_streams", "fonts", 0, "to_unicode"),
                    changed_cmap,
                ),
                (
                    "CID subset GID map",
                    ("font_streams", "fonts", 0, "auxiliary"),
                    lambda raw: raw[:-1] + bytes([raw[-1] ^ 1]),
                ),
            )
        )
    if probe["font_objects"]["objects"]:
        cases.append(
            (
                "font object dictionary",
                ("font_objects", "bytes"),
                lambda raw: raw.replace(b"/Identity-H", b"/Identity-V", 1),
            )
        )
    if probe["text_commands"]["commands"]:

        def shift(raw):
            end = raw.index(b" Tm")
            data = bytearray(raw)
            pos = end - 1
            while data[pos] not in b"0123456789":
                pos -= 1
            data[pos] = ord("1") if data[pos] != ord("1") else ord("2")
            return bytes(data)

        cases.append(("physical text baseline", ("text_commands", "bytes"), shift))
    cases.extend(image_paints.image_artifact_tamper_cases(merged_data_view(probe)))
    for index, program in enumerate(probe["rasters"]["programs"]):
        if program is None or program["encoding"] != "Flate":
            continue
        cases.append(
            (
                "original raster pixel",
                ("rasters", "programs", index, "program"),
                paints.altered_raster_pixel,
            )
        )
        if program["alpha"] is not None:
            cases.append(
                (
                    "original raster alpha",
                    ("rasters", "programs", index, "alpha", "program"),
                    paints.altered_raster_pixel,
                )
            )
        break
    count = 0
    with tempfile.TemporaryDirectory(
        prefix="typaxis-region-negative-", dir=root.parent
    ) as directory:
        target = Path(directory)
        for file in root.glob("*.bin"):
            (target / file.name).hardlink_to(file)
        for label, path, change in cases:
            bad = copy.deepcopy(probe)
            container, key = resources.location(bad, path)
            descriptor = container[key]
            raw = paints.artifact(root, descriptor)
            changed = change(raw)
            check(
                changed != raw, "actual artifact tamper changes original bytes " + label
            )
            sha = digest(changed).hex()
            name = "negative-" + sha + ".bin"
            (target / name).write_bytes(changed)
            container[key] = dict(file=name, sha256=sha, length=len(changed))
            if path[0] == "programs":
                bad["programs"]["fonts"][0]["sha256"] = sha
            if path[0] == "rasters" and path[-2] != "alpha":
                bad["rasters"]["programs"][path[2]]["sha256"] = sha
            reseal_outputs(target, bad)
            try:
                verify(target, bad)
            except REJECTION_ERRORS:
                count += 1
            else:
                raise AssertionError("resealed actual artifact accepted " + label)
    return count


def probe_inputs(directory, pdf_parent_directory=None):
    """Read genuine standalone or nested inputs without rewriting probe files.

    Manual old Navigation parents are inventoried explicitly and remain the
    responsibility of the independent Column PDF checker's old-parent branch.
    """
    entries, excluded = [], []
    paths = sorted(directory.glob("*.json"))
    check(paths, "actual standalone region callback probes required")
    for path in paths:
        raw = path.read_bytes()
        probe = json.loads(raw)
        check(probe["wire"] == SCHEMA, "standalone region probe wire")
        check(
            path.name == probe["case"] + "-" + digest(raw).hex() + ".json",
            "actual standalone probe raw filename SHA",
        )
        entries.append(
            dict(
                path=path,
                raw=raw,
                probe=probe,
                kind="standalone_region",
                outer_scope=None,
                parent_scope="navigation_driver",
            )
        )
    if pdf_parent_directory is not None:
        paths = sorted(pdf_parent_directory.glob("*.json"))
        check(paths, "actual full Column PDF parent probes required")
        for path in paths:
            raw = path.read_bytes()
            outer = json.loads(raw)
            check(
                outer["wire"] == "typaxis.book-2-column-pdf-probe/1",
                "actual enclosing Column PDF probe wire",
            )
            check(
                path.name == outer["case"] + "-" + digest(raw).hex() + ".json",
                "actual enclosing PDF probe raw filename SHA",
            )
            probe = outer["parent"]
            check(outer["case"] == probe["case"], "actual enclosing parent case")
            if probe["wire"] == "typaxis.book-2-column-navigation-probe/1":
                check(
                    outer["observed"]["scope"] == "retained_source_cut"
                    and outer["observed"]["parent_scope"] == "retained_source_cut"
                    and probe["observed"]["scope"] == "retained_source_cut"
                    and probe["observed"]["parent_scope"] == "retained_source_cut",
                    "excluded old Navigation parent has explicit retained-cut lineage",
                )
                excluded.append(
                    dict(
                        path=str(path.resolve()),
                        sha256=digest(raw).hex(),
                        case=outer["case"],
                        wire=probe["wire"],
                        scope="retained_source_cut",
                        verified=False,
                        reason="Old Navigation parent is checked by the separate Column PDF verifier, not this automatic region family.",
                    )
                )
                continue
            check(probe["wire"] == SCHEMA, "actual nested automatic region parent wire")
            check(
                outer["observed"]["scope"] == "pdf_driver"
                and outer["observed"]["parent_scope"] == "navigation_driver"
                and probe["observed"]["scope"] == "navigation_driver",
                "nested automatic region parent genuine driver lineage",
            )
            check(
                outer["source_canonical_jcs"] == probe["source_canonical_jcs"],
                "nested parent exact original source JCS",
            )
            check(
                outer["limits_canonical_jcs"] == probe["limits_canonical_jcs"],
                "nested parent exact actual limits JCS",
            )
            check(
                outer["observed"]["navigation_prefix"]
                == [
                    probe["observed"][key]
                    for key in ("records", "spool", "output", "work")
                ],
                "nested parent exact issued navigation prefix",
            )
            entries.append(
                dict(
                    path=path,
                    raw=raw,
                    probe=probe,
                    kind="pdf_parent",
                    outer_scope="pdf_driver",
                    parent_scope="navigation_driver",
                )
            )
    identities = [(entry["kind"], str(entry["path"].resolve())) for entry in entries]
    check(len(identities) == len(set(identities)), "unique actual input occurrences")
    return entries, excluded


def input_observation(entry):
    return dict(
        kind=entry["kind"],
        path=str(entry["path"].resolve()),
        sha256=digest(entry["raw"]).hex(),
        case=entry["probe"]["case"],
        parent_canonical_sha256=digest(canonical(entry["probe"]).encode()).hex(),
        outer_scope=entry["outer_scope"],
        parent_scope=entry["parent_scope"],
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument(
        "--pdf-parents",
        type=Path,
        help="Verify actual automatic region parents nested in these full PDF probes; inventory retained old Navigation parents separately.",
    )
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    modules = (
        ("verify_book_v2_column_display.py", display),
        ("verify_book_v2_column_resources.py", resources),
        ("verify_book_v2_column_paints.py", paints),
        ("verify_book_v2_column_marked.py", marked),
        ("verify_book_v2_column_paint_images.py", image_paints),
    )
    checker_sha = digest(Path(__file__).read_bytes()).hex()
    dependencies = {
        name: digest(Path(module.__file__).read_bytes()).hex()
        for name, module in modules
    }
    entries, excluded = probe_inputs(args.directory, args.pdf_parents)
    totals = Counter()
    cases = []
    observations = []
    rejections = 0
    missing_seen = set()
    for ordinal, entry in enumerate(entries, 1):
        path, probe = entry["path"], entry["probe"]
        result = verify(path.parent, probe)
        for k, v in result.items():
            if type(v) is int:
                totals[k] += v
        cases.append(probe["case"])
        observation = input_observation(entry)
        if args.self_test:
            count = tamper_checks(
                path.parent, probe, missing_seen
            ) + artifact_tamper_checks(path.parent, probe)
            rejections += count
            observation["negative_tests"] = count
            print(
                "SELFTEST",
                str(ordinal) + "/" + str(len(entries)),
                path.name,
                count,
                flush=True,
            )
        else:
            observation["negative_tests"] = 0
            print("VERIFIED", path.name, flush=True)
        check(
            path.read_bytes() == entry["raw"],
            "actual input raw bytes unchanged after controls",
        )
        check(
            digest(canonical(probe).encode()).hex()
            == observation["parent_canonical_sha256"],
            "actual parent object unchanged after controls",
        )
        observations.append(observation)
    for entry in excluded:
        check(
            digest(Path(entry["path"]).read_bytes()).hex() == entry["sha256"],
            "excluded retained old Navigation raw bytes unchanged",
        )
    check(
        digest(Path(__file__).read_bytes()).hex() == checker_sha,
        "checker source unchanged through complete run",
    )
    check(
        dependencies
        == {
            name: digest(Path(module.__file__).read_bytes()).hex()
            for name, module in modules
        },
        "inherited checker sources unchanged through complete run",
    )
    print(
        canonical(
            dict(
                schema="typaxis.book-2-column-running-regions-verification/1",
                probes=len(entries),
                cases=cases,
                scopes=dict(Counter({"navigation_driver": len(entries)})),
                input_kinds=dict(Counter(entry["kind"] for entry in entries)),
                inputs=observations,
                excluded_pdf_parents=excluded,
                totals=dict(totals),
                negative_tests=rejections,
                checker_sha256=checker_sha,
                inherited_checker_sha256=dependencies,
                verification_limits=[
                    "Standalone automatic region probes and actual automatic parents nested in full PDF probes are distinct input occurrences. Excluded retained old Navigation parents are not validated by this family.",
                    "Original local shape glyph IDs, line cuts, prepared page-name registry and driver source/shape prefixes are authentic issuer witnesses, not independent shaping/preparation replay.",
                    "Exact projection/join and font-to-text/image/vector/scope successful counters; later Content/source/relations/navigation arrays and broader failed attempts remain issuer observations and CLI assertions.",
                    "Original source/JCS, style size/family and original sfnt metrics are checked. Original image bytes/pixels and admitted vector IR are bound; this does not reimplement the SVG parser or CID-CFF positive gate.",
                    "No enclosing Column PDF or whole pagination acceptance.",
                ],
                scope="Original JCS/master/set, authentic local line/glyph witness projection, join R/W/folds, original font/CID/stream/object/text and image/vector bytes plus scope folds.",
            )
        )
    )


if __name__ == "__main__":
    main()
