#!/usr/bin/env python3
"""Reconstruct Column navigation from authentic source and selected geometry.

Normal verification replays the unchanged relation checker, then derives source
Link/Anchor/outline registries, physical destinations and occurrence rectangles,
successful reservations and folded preimages. Coordinates remain Y-down. This
does not verify annotations, page transforms, final PDFs, failed attempts or
independent first-demand/Page-label convergence.
"""
from __future__ import annotations

import argparse
import copy
import json
import re
import sys
from collections import Counter
from pathlib import Path

import verify_book_v2_column_relations as relations

marked = relations.marked
display = marked.display
check = marked.check
fields = marked.fields
strict_equal = marked.strict_equal
integer = marked.integer
hash_bytes = marked.hash_bytes
digest = marked.digest
PDFBudget = marked.PDFBudget
u = marked.u
MAX_U64 = (1 << 64) - 1
SCHEMA = "typaxis.book-2-column-navigation-probe/1"
ALGORITHM = "typaxis.book-2-navigation-geometry/1"
# Fixed diagnostic units are filled from the actual frozen 64-bit Rust layouts.
ABI_UNITS = dict(
    destination=72,
    link=56,
    rectangle=80,
    range=16,
    outline=96,
    option_usize=16,
    option_first_bounds=56,
    usize=8,
)


def original_facts(probe):
    source = probe["relation_probe"]["marked_probe"]
    wire = source["source_package"]["wire"]
    document = wire["document"]
    nodes = source["source_structure"]["nodes"]
    owners = marked.owner_values(document)
    indices = {tuple(node["key"]): i for i, node in enumerate(nodes)}
    paragraphs = []
    can_own_number = set()

    def inlines(values):
        result = []
        for value in values:
            kind, owner = value["kind"], value["node_id"]
            target = None
            if kind == "link":
                authored = value["target"]
                target = dict(
                    kind=authored["kind"],
                    value=(
                        authored["uri"]
                        if authored["kind"] == "uri"
                        else authored["anchor_id"]
                    ),
                )
            reference = None
            if kind == "reference":
                reference = dict(
                    kind="anchor",
                    target=value["target"],
                    target_owner=None,
                    format=value["format"],
                )
            elif kind == "footnote_reference":
                reference = dict(kind="footnote", id=value["footnote_id"])
            span = value["span"]
            source_span = [span["source_id"], span["start_byte"], span["end_byte"]]
            content = dict(
                inline_math="native_math",
                emphasis="begin_emphasis",
                strong="begin_strong",
                link="begin_link",
            ).get(kind, kind)
            result.append(
                dict(
                    owner=owner,
                    kind=content,
                    span=source_span,
                    link=target,
                    reference=reference,
                )
            )
            if kind in ("emphasis", "strong", "link"):
                result.extend(inlines(value["children"]))
                result.append(
                    dict(
                        owner=owner,
                        kind="end_container",
                        span=source_span,
                        link=None,
                        reference=None,
                    )
                )
        return result

    def blocks(values):
        for value in values:
            kind = value["kind"]
            if kind != "page_break":
                can_own_number.add(value["node_id"])
            if kind in ("paragraph", "heading"):
                paragraphs.append(
                    dict(owner=value["node_id"], items=inlines(value["children"]))
                )
            elif kind == "semantic_container":
                blocks(value["blocks"])
            elif kind in ("figure", "vector_figure"):
                blocks(value["caption"])
            elif kind == "list":
                for item in value["items"]:
                    can_own_number.add(item["node_id"])
                    blocks(item["blocks"])
            elif kind == "description_list":
                for item in value["items"]:
                    term = item["term"]
                    can_own_number.update((item["node_id"], term["node_id"]))
                    paragraphs.append(
                        dict(owner=term["node_id"], items=inlines(term["children"]))
                    )
                    blocks(item["blocks"])
            elif kind == "table":
                blocks(value.get("caption") or [])
                for row in [*value["head"], *value["body"]]:
                    for cell in row["cells"]:
                        blocks(cell["blocks"])
            else:
                check(
                    kind in ("display_math", "math_vector_block", "page_break"),
                    "known authored source block",
                )

    blocks(document["blocks"])
    for note in document["footnotes"]:
        blocks(note["blocks"])
    anchors = {}
    for owner, value in owners.items():
        if value.get("kind") in ("anchor", "heading", "semantic_container"):
            name = value.get("anchor_id")
            if name is not None:
                check(name not in anchors, "unique original source anchor")
                anchors[name] = owner

    def descendant(child, ancestor):
        while child is not None:
            if child == ancestor:
                return True
            index = indices.get((child, "Source"))
            if index is None:
                # EquationNumber children have a deliberately separate source slot.
                number = indices.get((child, "EquationNumber"))
                check(number is not None, "original source/number child key")
                parent = nodes[number]["parent"]
            else:
                parent = nodes[index]["parent"]
            child = nodes[parent]["key"][0] if parent is not None else None
        return False

    numbers = {}
    buffers = {
        value["text_id"]: value["utf8"].encode() for value in wire["text_buffers"]
    }
    for binding in document.get("number_bindings") or []:
        name, owner, label = (
            binding["anchor_id"],
            binding["owner_node_id"],
            binding["label_node_id"],
        )
        check(
            name not in numbers and owner in can_own_number and label in owners,
            "authored number registry/eligible owner",
        )
        check(descendant(label, owner), "number label original owner subtree")
        leaf = owners[label]
        check(
            leaf.get("kind") == "text"
            or any(
                value.get("kind") == "math_vector_block"
                and value.get("equation_number") == leaf
                for value in owners.values()
            ),
            "authored number Text or original vector EquationNumber child",
        )
        span, whole = binding["text_span"], leaf["text_span"]
        check(
            span["text_id"] == whole["text_id"]
            and whole["start_byte"]
            <= span["start_byte"]
            < span["end_byte"]
            <= whole["end_byte"],
            "number label contained exact original UTF8 slice",
        )
        text = buffers[span["text_id"]][span["start_byte"] : span["end_byte"]].decode()
        check(
            text.strip() and not any(ord(c) < 32 or 127 <= ord(c) <= 159 for c in text),
            "nonempty control-free number label",
        )
        numbers[name] = text
        if name in anchors:
            check(
                descendant(anchors[name], owner), "existing number anchor owner subtree"
            )
        else:
            anchors[name] = owner
    anchor_records = [[name, owner] for name, owner in sorted(anchors.items())]
    for paragraph in paragraphs:
        for item in paragraph["items"]:
            reference = item["reference"]
            if reference is not None and reference["kind"] == "anchor":
                reference["target_owner"] = anchors[reference["target"]]
    outline = []
    stack = []
    outline_owners, outline_destinations = set(), set()
    previous_outline_owner = None
    for ordinal, entry in enumerate(wire["outline"]["entries"]):
        owner = entry["source_node_id"]
        check(
            entry["outline_id"] == ordinal and owner in owners,
            "dense original outline source",
        )
        value = owners[owner]
        check(
            value.get("kind") in ("heading", "semantic_container"),
            "eligible original outline owner",
        )
        check(
            entry["source_kind"] == value["kind"], "outline exact original source kind"
        )
        check(
            owner not in outline_owners
            and (previous_outline_owner is None or previous_outline_owner < owner),
            "outline unique increasing original source owner",
        )
        check(
            entry["destination"] not in outline_destinations,
            "outline unique original destination",
        )
        outline_owners.add(owner)
        outline_destinations.add(entry["destination"])
        previous_outline_owner = owner
        check(
            anchors.get(entry["destination"]) == owner
            and value.get("anchor_id") == entry["destination"],
            "outline exact source anchor",
        )
        level = integer(entry["level"], 8, False)
        check(1 <= level <= 6, "outline source level")
        if value["kind"] == "heading":
            check(value["level"] == level, "outline original heading level")
        while stack and stack[-1][0] >= level:
            stack.pop()
        expected_parent = stack[-1][1] if stack and stack[-1][0] + 1 == level else None
        check(
            entry["parent_outline_id"] == expected_parent
            and (level == 1 or expected_parent is not None),
            "outline exact original parent level",
        )
        outline.append(
            dict(
                id=ordinal,
                parent=entry["parent_outline_id"],
                level=level,
                target=entry["destination"],
                label=entry["label"],
            )
        )
        stack.append((level, ordinal))
    return dict(
        source=source,
        wire=wire,
        document=document,
        nodes=nodes,
        owners=owners,
        indices=indices,
        paragraphs=paragraphs,
        anchors=anchor_records,
        outline=outline,
        numbers=numbers,
        buffers=buffers,
    )


def union(a, b):
    if a is None:
        return list(display.rect(b))
    a, b = display.rect(a), display.rect(b)
    x, y = min(a[0], b[0]), min(a[1], b[1])
    right = max(integer(a[0] + a[2]), integer(b[0] + b[2]))
    bottom = max(integer(a[1] + a[3]), integer(b[1] + b[3]))
    return display.rect([x, y, right - x, bottom - y])


def physical_inputs(facts):
    source = facts["source"]
    draws = source["paint_probe"]["display_probe"]
    arrays = dict(
        text="text",
        marker="markers",
        math="math",
        image="images",
        number="numbers",
        separator="separators",
    )
    bounds = []
    for stream, index in draws["paints"]:
        value = draws[arrays[stream]][index]
        if stream in ("text", "marker", "number"):
            rect = value["bounds"]
        elif stream == "math":
            paint = value["paint"]
            rect = paint["bounds"] if paint["kind"] == "native" else paint["viewport"]
        elif stream == "image":
            rect = value["viewport"]
        else:
            check(stream == "separator", "known selected physical stream")
            rect = None
        bounds.append(None if rect is None else display.rect(rect))
    raw_groups = []
    fragments = draws["original_fragments"]
    # The frozen Display wire retains Break kind/placed-inline index, not its
    # owner. Each authored control contributes one retained logical unit. Join
    # the complete authored sequence to unique selected graph/paragraph lines,
    # never treating a placed cluster index as an original source-site index.
    needed = {
        fragments[value["fragment"]]["owner"]
        for value in draws["nonpainting"]
        if fragments[value["fragment"]]["line"]["items"]
    }
    source_controls = {
        paragraph["owner"]: [
            site["owner"]
            for site in paragraph["items"]
            if site["kind"] in ("soft_break", "hard_break")
        ]
        for paragraph in facts["paragraphs"]
        if paragraph["owner"] in needed
    }
    selected_lines = {}
    for fragment in fragments:
        line = fragment["line"]
        if line is None or fragment["owner"] not in needed:
            continue
        check(line["owner"] == fragment["owner"], "original line/paragraph owner")
        key = (fragment["owner"], fragment["graph"]["line_fingerprint"])
        rows = selected_lines.setdefault(key, {})
        metadata = [(item["inline"], item["kind"]) for item in line["items"]]
        if line["line"] in rows:
            check(
                rows[line["line"]] == metadata,
                "same original graph line control census on repeated occurrences",
            )
        else:
            rows[line["line"]] = metadata
    break_owners = {}
    for key, rows in selected_lines.items():
        authored = source_controls[key[0]]
        cursor = 0
        for line_index, metadata in sorted(rows.items()):
            controls = []
            for _, kind in metadata:
                if kind == "break":
                    check(
                        cursor < len(authored),
                        "selected break sequence does not exceed original controls",
                    )
                    controls.append(authored[cursor])
                    cursor += 1
            break_owners[key, line_index] = controls
        check(
            cursor == len(authored),
            "complete original break control sequence for selected graph/paragraph",
        )
    for index, group in enumerate(source["marked_scopes"]["groups"]):
        raw_groups.append(
            dict(
                group=index,
                page=group["page"],
                fragment=group["fragment"],
                column=fragments[group["fragment"]]["column"],
                artifact=group["artifact"] is not None,
                paints=bounds[group["paints"][0] : group["paints"][1]],
            )
        )
    lines = []
    for value in draws["nonpainting"]:
        fragment = fragments[value["fragment"]]
        lines.append(
            dict(
                owner=fragment["owner"],
                page=fragment["page"],
                fragment=fragment["fragment"],
                column=fragment["column"],
                bounds=fragment["bounds"],
                repeated=value["repeated"],
                breakowners=(
                    break_owners[
                        (fragment["owner"], fragment["graph"]["line_fingerprint"]),
                        fragment["line"]["line"],
                    ]
                    if fragment["line"]["items"]
                    else []
                ),
            )
        )
    anchors = [
        dict(
            owner=value["owner"],
            page=fragments[value["fragment"]]["page"],
            fragment=value["fragment"],
            column=fragments[value["fragment"]]["column"],
            x=value["x"],
            baseline=value["baseline"],
            repeated=value["repeated"],
        )
        for value in draws["anchors"]
    ]
    return dict(
        raw_groups=raw_groups,
        nonpainting_lines=lines,
        inline_anchors=anchors,
        page_count=len(draws["pages"]),
    )


def source_labels(facts):
    def text(value):
        kind = value["kind"]
        if kind == "text":
            span = value["text_span"]
            return facts["buffers"][span["text_id"]][
                span["start_byte"] : span["end_byte"]
            ].decode()
        if kind in ("soft_break", "hard_break"):
            return " "
        if kind == "anchor":
            return ""
        check(
            kind in ("heading", "emphasis", "strong", "link"),
            "source heading reference has only admissible text atoms",
        )
        return "".join(text(child) for child in value["children"])

    anchors = dict(facts["anchors"])
    expected = {}
    for value in facts["owners"].values():
        if value.get("kind") != "reference" or value["format"] == "page":
            continue
        target = value["target"]
        if value["format"] == "number":
            check(target in facts["numbers"], "reference has authored source number")
            label = facts["numbers"][target]
        else:
            check(value["format"] == "text", "known original reference format")
            entries = [entry for entry in facts["outline"] if entry["target"] == target]
            if entries:
                check(len(entries) == 1, "unique explicit source outline label")
                label = entries[0]["label"]
            else:
                owner = facts["owners"][anchors[target]]
                check(
                    owner.get("kind") == "heading",
                    "Text label fallback is a prepared heading",
                )
                label = text(owner)
        check(bool(label), "nonempty original generated reference label")
        expected[value["node_id"]] = label.encode()
    draws = facts["source"]["paint_probe"]["display_probe"]
    for draw in draws["text"]:
        if draw["generated"] and draw["owner"] in expected:
            _, first, last = draw["span"]
            check(
                0 <= first <= last <= len(expected[draw["owner"]]),
                "actual generated reference slice",
            )
            strict_equal(
                draw["text"],
                expected[draw["owner"]][first:last].decode(),
                "generated Text/Number label exact source bytes",
            )
    return expected


def derive_navigation(facts, physical, rel, size, budget):
    nodes, paragraphs, anchors = facts["nodes"], facts["paragraphs"], facts["anchors"]
    n, na, nd, ne = len(nodes), len(anchors), len(rel["notes"]), len(rel["edges"])
    lc = ne
    for paragraph in paragraphs:
        budget.step()
        for site in paragraph["items"]:
            budget.step()
            lc += int(
                site["link"] is not None
                or (
                    site["reference"] is not None
                    and site["reference"]["kind"] == "anchor"
                )
            )
    for note in rel["notes"]:
        budget.step()
        lc += int(note["return"] is not None)
    counts = [
        na + nd + ne,
        lc,
        len(physical["raw_groups"]),
        physical["page_count"],
        len(facts["outline"]),
        n,
        n,
    ]
    for name, count in zip(list(ABI_UNITS)[:7], counts):
        budget.reserve(count, count * size[name], 0)
        budget.step()
    active, first = [], []
    for _ in range(n):
        budget.step()
        active.append(None)
        first.append(None)
    destinations = []
    for name, owner in anchors:
        budget.step()
        destinations.append(dict(owner=owner, kind=["anchor", name], position=None))
    for index, note in enumerate(rel["notes"]):
        budget.step()
        destinations.append(
            dict(
                owner=nodes[note["node"]]["key"][0],
                kind=["footnote", index],
                position=None,
            )
        )
    for index, edge in enumerate(rel["edges"]):
        budget.step()
        destinations.append(
            dict(
                owner=nodes[edge["reference"]]["key"][0],
                kind=["reference", index],
                position=None,
            )
        )
    lookup = sorted(
        ((owner, marked.SLOTS.index(slot)), index)
        for (owner, slot), index in facts["indices"].items()
    )

    def locate(owner, slot):
        value = marked.find_lookup(lookup, (owner, marked.SLOTS.index(slot)), budget)
        check(value is not None, "original source key lookup")
        return value

    def anchor_index(name):
        for index, destination in enumerate(destinations[:na]):
            budget.step()
            if destination["kind"] == ["anchor", name]:
                return index
        raise AssertionError("original destination anchor lookup")

    links = []

    def link(node, target):
        check(active[node] is None, "unique active source Link root")
        active[node] = len(links)
        links.append(dict(node=node, target=target, first=None, last=None))

    for paragraph in paragraphs:
        budget.step()
        for site in paragraph["items"]:
            budget.step()
            target, reference = site["link"], site["reference"]
            if target is not None:
                check(
                    reference is None, "source site cannot contain two navigation roots"
                )
                if target["kind"] == "internal":
                    selected = ["destination", anchor_index(target["value"])]
                else:
                    check(target["kind"] == "uri", "exact source Link target kind")
                    selected = ["uri", target["value"]]
                link(locate(site["owner"], "Source"), selected)
            elif reference is not None and reference["kind"] == "anchor":
                link(
                    locate(site["owner"], "ReferenceLink"),
                    ["destination", anchor_index(reference["target"])],
                )
    for edge in rel["edges"]:
        budget.step()
        link(edge["ref_link"], ["destination", na + edge["definition"]])
    for note in rel["notes"]:
        budget.step()
        if note["return"] is not None:
            link(note["link"], ["destination", na + nd + note["return"]])
    check(len(links) == lc, "complete original source Link count")
    for index, node in enumerate(nodes):
        budget.step()
        parent = node["parent"]
        if parent is not None:
            check(parent < index, "original preorder source parent")
            if active[index] is not None and active[parent] is not None:
                return dict(error=["conflicting_links", node["key"][0]])
            if active[index] is None:
                active[index] = active[parent]
    for line in physical["nonpainting_lines"]:
        budget.step()
        if line["repeated"]:
            continue
        cursor = locate(line["owner"], "Source")
        value = [line["page"], line["fragment"], list(line["bounds"])]
        while cursor is not None:
            budget.step()
            if first[cursor] is None:
                first[cursor] = copy.deepcopy(value)
            cursor = nodes[cursor]["parent"]
    bindings = {
        value["group"]: value["node"]
        for value in facts["source"]["source_structure"]["bindings"]
    }
    rectangles, page_ranges = [], []
    pages = facts["source"]["marked_content"]["pages"]
    for page_index, page in enumerate(pages):
        budget.step()
        start = len(rectangles)
        for gi in range(*page["groups"]):
            budget.step()
            if gi not in bindings:
                continue
            group = physical["raw_groups"][gi]
            check(
                group["page"] == page_index and not group["artifact"],
                "actual semantic group page/binding",
            )
            bounds = None
            for rect in group["paints"]:
                budget.step()
                if rect is not None:
                    bounds = union(bounds, rect)
            if bounds is None:
                continue
            location = [page_index, group["fragment"]]
            cursor = bindings[gi]
            while cursor is not None:
                budget.step()
                old = first[cursor]
                if old is None or tuple(location) < tuple(old[:2]):
                    first[cursor] = [*location, list(bounds)]
                elif location == old[:2]:
                    old[2] = union(old[2], bounds)
                cursor = nodes[cursor]["parent"]
            li = active[bindings[gi]]
            if li is not None:
                selected = links[li]
                last = selected["last"]
                if (
                    last is not None
                    and [rectangles[last]["page"], rectangles[last]["fragment"]]
                    == location
                ):
                    rectangles[last]["bounds"] = union(
                        rectangles[last]["bounds"], bounds
                    )
                    continue
                index = len(rectangles)
                if last is not None:
                    rectangles[last]["next"] = index
                else:
                    selected["first"] = index
                selected["last"] = index
                rectangles.append(
                    dict(
                        link=li,
                        page=page_index,
                        fragment=group["fragment"],
                        group=gi,
                        bounds=list(bounds),
                        next=None,
                    )
                )
        page_ranges.append([start, len(rectangles)])

    def position(index):
        value = first[index]
        return [*value[:2], *value[2][:2]] if value is not None else None

    for destination in destinations[:na]:
        budget.step()
        index = marked.find_lookup(
            lookup, (destination["owner"], marked.SLOTS.index("Source")), budget
        )
        if index is not None:
            destination["position"] = position(index)
    for anchor in physical["inline_anchors"]:
        budget.step()
        if anchor["repeated"]:
            continue
        found = False
        for destination in destinations[:na]:
            budget.step()
            if destination["owner"] == anchor["owner"]:
                check(
                    not found and destination["position"] is None,
                    "inline anchor fills one unresolved source destination",
                )
                destination["position"] = [
                    anchor["page"],
                    anchor["fragment"],
                    anchor["x"],
                    anchor["baseline"],
                ]
                found = True
        check(found, "actual source inline anchor destination exists")
    for index, note in enumerate(rel["notes"]):
        budget.step()
        destinations[na + index]["position"] = position(note["node"])
        check(
            (destinations[na + index]["position"] is not None) == note["painted"],
            "Note destination actual paint presence",
        )
    for index, edge in enumerate(rel["edges"]):
        budget.step()
        destinations[na + nd + index]["position"] = position(edge["ref_label"])
        check(
            (destinations[na + nd + index]["position"] is not None) == edge["painted"],
            "ReferenceLabel destination actual paint presence",
        )
    for selected in links:
        budget.step()
        if selected["first"] is not None and selected["target"][0] == "destination":
            target = destinations[selected["target"][1]]
            if target["position"] is None:
                return dict(error=["unplaced_destination", target["owner"]])
    outline, last_root = [], None
    for index, entry in enumerate(facts["outline"]):
        budget.step()
        destination = anchor_index(entry["target"])
        if destinations[destination]["position"] is None:
            return dict(
                error=["unplaced_destination", destinations[destination]["owner"]]
            )
        parent = None
        if entry["parent"] is not None:
            for j, prior in enumerate(facts["outline"][:index]):
                budget.step()
                if prior["id"] == entry["parent"]:
                    parent = j
                    break
            check(parent is not None, "original outline parent exists")
        previous = outline[parent]["last"] if parent is not None else last_root
        outline.append(
            dict(
                destination=destination,
                parent=parent,
                first=None,
                last=None,
                previous=previous,
                next=None,
                descendants=0,
            )
        )
        if previous is not None:
            outline[previous]["next"] = index
        if parent is not None:
            if outline[parent]["first"] is None:
                outline[parent]["first"] = index
            outline[parent]["last"] = index
        else:
            last_root = index
        cursor = parent
        while cursor is not None:
            budget.step()
            outline[cursor]["descendants"] += 1
            cursor = outline[cursor]["parent"]
    return dict(
        destinations=destinations,
        links=links,
        rectangles=rectangles,
        pages=page_ranges,
        outline=outline,
    )


def canonical_hash(source_fp, derived, source_outline, budget):
    fp = budget.fold(hash_bytes(source_fp), ALGORITHM.encode())

    def number(value):
        nonlocal fp
        fp = budget.fold(fp, u(MAX_U64 if value is None else value))

    def string(value):
        nonlocal fp
        check(type(value) is str, "exact borrowed source navigation string")
        encoded = value.encode()
        number(len(encoded))
        for start in range(0, len(encoded), 96):
            fp = budget.fold(fp, encoded[start : start + 96])

    for destination in derived["destinations"]:
        number(destination["owner"])
        kind, value = destination["kind"]
        fp = budget.fold(fp, bytes([("anchor", "footnote", "reference").index(kind)]))
        string(value) if kind == "anchor" else number(value)
        position = destination["position"]
        fp = budget.fold(fp, bytes([int(position is not None)]))
        if position is not None:
            number(position[0])
            number(position[1])
            fp = budget.fold(fp, display.i(position[2]))
            fp = budget.fold(fp, display.i(position[3]))
    for link in derived["links"]:
        for value in (link["node"], link["first"], link["last"]):
            number(value)
        kind, value = link["target"]
        fp = budget.fold(fp, bytes([("destination", "uri").index(kind)]))
        number(value) if kind == "destination" else string(value)
    for rect in derived["rectangles"]:
        for key in ("link", "page", "fragment", "group", "next"):
            number(rect[key])
        for value in display.rect(rect["bounds"]):
            fp = budget.fold(fp, display.i(value))
    for start, end in derived["pages"]:
        number(start)
        number(end)
    for outline, source in zip(derived["outline"], source_outline):
        for key in (
            "destination",
            "parent",
            "first",
            "last",
            "previous",
            "next",
            "descendants",
        ):
            number(outline[key])
        number(source["id"])
        number(source["level"])
        string(source["label"])
    return fp


def public_report(derived):
    if "error" in derived:
        return dict(error=list(derived["error"]))
    return dict(
        destinations=derived["destinations"],
        links=[
            {k: v for k, v in link.items() if k != "last"} for link in derived["links"]
        ],
        rectangles=derived["rectangles"],
        page_ranges=derived["pages"],
        outline=derived["outline"],
    )


def verify(root, probe, _cache=None):
    fields(
        probe,
        (
            "wire",
            "case",
            "relation_probe",
            "navigation_source",
            "raw_groups",
            "inline_anchors",
            "nonpainting_lines",
            "navigation",
            "error",
            "observed",
            "size_of",
        ),
        "Column Navigation probe",
    )
    check(
        probe["wire"] == SCHEMA and probe["case"] == probe["relation_probe"]["case"],
        "actual Navigation schema/case",
    )
    source = probe["relation_probe"]
    identity = _cache is not None and source is _cache["relation_object"]
    if identity:
        previous, facts = dict(_cache["relation_result"]), _cache["facts"]
    else:
        previous = relations.verify(root, source)
        facts = original_facts(probe)
    size = fields(
        probe["size_of"], ABI_UNITS, "all seven actual Navigation reservation units"
    )
    strict_equal(size, ABI_UNITS, "frozen 64-bit Navigation field layouts")
    observed = fields(
        probe["observed"],
        (
            "scope",
            "parent_scope",
            "maximum_work",
            "constructors",
            "caller_before",
            "caller_after",
            "relations_prefix",
            "source_prefix",
            "ancestor_callback",
            "build_after",
        ),
        "complete Navigation observation",
    )
    check(
        observed["scope"]
        in (
            "navigation_driver",
            "relation_callback",
            "source_callback",
            "retained_source_cut",
        ),
        "explicit captured Navigation scope",
    )
    strict_equal(
        observed["parent_scope"],
        source["observed"]["scope"],
        "authentic declared nested relation lineage",
    )
    strict_equal(
        observed["maximum_work"],
        source["observed"]["maximum_work"],
        "same cumulative Navigation work cap",
    )
    check(
        type(observed["constructors"]) is list and len(observed["constructors"]) == 13,
        "complete thirteen actual constructors",
    )
    strict_equal(
        observed["constructors"][:12],
        source["observed"]["constructors"],
        "unchanged twelve relation constructor prefix",
    )
    prefix = source["relations"]["charges"]
    strict_equal(observed["source_prefix"], prefix, "actual completed relation quartet")
    strict_equal(
        observed["ancestor_callback"],
        (
            source["observed"]["caller_after"]
            if observed["parent_scope"] == "relation_driver"
            else source["observed"]["ancestor_callback"]
        ),
        "actual relation/source ancestor observation",
    )
    if observed["scope"] == "navigation_driver":
        check(
            source["observed"]["scope"] == "relation_driver",
            "Navigation layered on actual relation driver",
        )
        strict_equal(
            observed["relations_prefix"],
            prefix,
            "actual relation callback before constructor13",
        )
        strict_equal(
            observed["caller_before"],
            source["observed"]["caller_before"],
            "unchanged whole command starting prefix",
        )
    else:
        check(
            (
                observed["scope"] == "source_callback"
                and observed["parent_scope"] == "source_callback"
            )
            or (
                observed["scope"] == "relation_callback"
                and observed["parent_scope"] == "relation_driver"
            )
            or (
                observed["scope"] == "retained_source_cut"
                and observed["parent_scope"] == "retained_source_cut"
            ),
            "manual Navigation source lineage",
        )
        check(
            observed["relations_prefix"] is None
            and observed["caller_before"] is None
            and observed["caller_after"] is None,
            "manual Navigation invents no whole command observation",
        )
    budget = PDFBudget(prefix)
    settled_output = budget.output
    budget.reserve(1, 0, 0)
    budget.matches4(observed["constructors"][12])
    reported_inputs = {
        name: probe[name]
        for name in (
            "navigation_source",
            "raw_groups",
            "inline_anchors",
            "nonpainting_lines",
        )
    }
    # Reported geometry/site copies cannot replace the authentic nested source.
    # A changed reported input is checked against immutable independently derived
    # facts before hashing the complete key; identity skips are self-test only.
    if identity and "expected_inputs" in _cache:
        for name, expected in _cache["expected_inputs"].items():
            relations._strict_report(
                reported_inputs[name],
                expected,
                _cache["original_inputs"][name],
                "exact source-owned Navigation input " + name,
            )
    key_objects = [
        *reported_inputs.values(),
        size,
        observed["relations_prefix"],
        observed["source_prefix"],
        observed["ancestor_callback"],
        prefix,
    ]
    unchanged_key = (
        identity
        and "key_objects" in _cache
        and all(a is b for a, b in zip(key_objects, _cache["key_objects"]))
    )
    key = (
        _cache["input_key"]
        if unchanged_key
        else digest(
            display.canonical(
                [
                    reported_inputs,
                    size,
                    observed["relations_prefix"],
                    observed["source_prefix"],
                    observed["ancestor_callback"],
                    prefix,
                ]
            ).encode()
        )
    )
    cached = identity and key == _cache.get("input_key")
    if cached:
        derived, fp = _cache["derived"], _cache["fingerprint"]
        budget = PDFBudget(list(_cache["final_budget"]))
        expected_inputs, expected_report = (
            _cache["expected_inputs"],
            _cache["expected_report"],
        )
    else:
        physical = physical_inputs(facts)
        source_labels(facts)
        expected_inputs = dict(
            navigation_source=dict(
                paragraphs=facts["paragraphs"],
                anchors=[
                    dict(id=name, owner=owner) for name, owner in facts["anchors"]
                ],
                outline=facts["outline"],
            ),
            **{
                name: physical[name]
                for name in ("raw_groups", "inline_anchors", "nonpainting_lines")
            },
        )
        strict_equal(
            reported_inputs,
            expected_inputs,
            "independently derived source/physical Navigation inputs",
        )
        derived = derive_navigation(facts, physical, source["relations"], size, budget)
        fp = (
            None
            if "error" in derived
            else canonical_hash(
                source["relations"]["fingerprint"], derived, facts["outline"], budget
            )
        )
        expected_report = public_report(derived)
    stage = probe["navigation"]
    if fp is None:
        check(
            observed["scope"] != "navigation_driver",
            "semantic Navigation refusal cannot deliver a Navigationdriver callback",
        )
        check(stage is None, "a source semantic refusal issues no Navigation result")
        error = fields(
            probe["error"],
            ("kind", "owner", "resource", "charges"),
            "actual source Navigation refusal",
        )
        strict_equal(
            error["kind"], derived["error"][0], "precise original source refusal kind"
        )
        strict_equal(
            error["owner"], derived["error"][1], "precise original source refusal owner"
        )
        check(
            error["resource"] is None,
            "source semantic refusal is not a fabricated resource failure",
        )
        budget.matches4(error)
    else:
        check(probe["error"] is None, "successful Navigation has no fabricated error")
        fields(
            stage,
            {"fingerprint", "charges", *expected_report},
            "complete Navigation stage",
        )
        for name, expected in expected_report.items():
            if cached:
                relations._strict_report(
                    stage[name],
                    expected,
                    _cache["original_report"][name],
                    "exact Navigation " + name,
                )
            else:
                strict_equal(
                    stage[name],
                    expected,
                    "exact independently derived Navigation " + name,
                )
        check(
            fp == hash_bytes(stage["fingerprint"]),
            "Navigation folded canonical preimages",
        )
        budget.matches4(stage)
    budget.matches4(observed["build_after"])
    check(
        budget.output == settled_output,
        "Navigation inherits settled O unchanged with no new credit",
    )
    if observed["scope"] == "navigation_driver":
        strict_equal(
            observed["caller_after"],
            budget.triple4(),
            "actual final Navigation quartet retained by caller",
        )
    base = json.loads(
        facts["source"]["paint_probe"]["display_probe"]["limits_canonical_jcs"]
    )["base"]
    maximum_work = integer(observed["maximum_work"], 64, False)
    for value in [*observed["constructors"], prefix, observed["build_after"]]:
        r, s, o, w = PDFBudget(value).triple4()
        check(
            r <= base["max_fragments"]
            and s <= base["max_spool_bytes"]
            and o <= base["max_output_bytes"]
            and w <= maximum_work,
            "inclusive authentic Navigation R/S/O/W caps",
        )
    result = dict(
        previous,
        scope=observed["scope"],
        navigation_destinations=len(derived.get("destinations", [])),
        navigation_links=len(derived.get("links", [])),
        navigation_rectangles=len(derived.get("rectangles", [])),
        navigation_outlines=len(derived.get("outline", [])),
        navigation_semantic_refusals=int("error" in derived),
        navigation_pages=len(derived.get("pages", [])),
    )
    if _cache is not None and _cache.get("capture"):
        _cache.update(
            facts=facts,
            derived=copy.deepcopy(derived),
            fingerprint=fp,
            input_key=bytes(key),
            key_objects=key_objects,
            final_budget=list(budget.triple4()),
            expected_inputs=copy.deepcopy(expected_inputs),
            original_inputs=reported_inputs,
            expected_report=copy.deepcopy(expected_report),
            original_report=stage,
        )
    return result


class _ReportHash:
    def fold(self, prior, value):
        check(
            len(prior) == 32 and len(value) <= 96,
            "reported Navigation closed fold width",
        )
        return digest(prior + value)


def reseal_navigation(probe, _cache=None):
    """Reseal reported records only, never source-derived expected values."""
    stage = probe["navigation"]
    if stage is None:
        return
    facts = (
        _cache["facts"]
        if _cache is not None and probe["relation_probe"] is _cache["relation_object"]
        else original_facts(probe)
    )
    reported = dict(
        stage, links=copy.deepcopy(stage["links"]), pages=stage["page_ranges"]
    )
    for link in reported["links"]:
        cursor, seen, last = link["first"], set(), None
        while cursor is not None:
            check(cursor not in seen, "reported Link chain acyclic")
            seen.add(cursor)
            last = cursor
            cursor = reported["rectangles"][cursor]["next"]
        link["last"] = last
    stage = dict(stage)
    probe["navigation"] = stage
    stage["fingerprint"] = canonical_hash(
        probe["relation_probe"]["relations"]["fingerprint"],
        reported,
        facts["outline"],
        _ReportHash(),
    ).hex()


def tamper_cases(probe):
    # Every scalar/option and all collections/required fields are exercised.
    def walk(value, path, reseal):
        if isinstance(value, dict):
            for key, child in value.items():
                yield "missing-" + repr((*path, key)), (*path, key), None, "delete"
                yield from walk(child, (*path, key), reseal)
        elif isinstance(value, list):
            if value:
                yield "omit-" + repr(path), path, value[:-1], reseal
                yield "duplicate-" + repr(path), path, [
                    *value,
                    copy.deepcopy(value[0]),
                ], reseal
                if len(value) > 1 and value[0] != value[-1]:
                    yield "reorder-" + repr(path), path, list(reversed(value)), reseal
            for index, child in enumerate(value):
                yield from walk(child, (*path, index), reseal)
        else:
            yield "scalar-" + repr(path), path, relations.changed_scalar(value), (
                False if path[-1] == "fingerprint" else reseal
            )

    for name in probe:
        yield "missing-top-" + name, (name,), None, "delete"
    yield "unknown-top", ("unknown_field",), 1, False
    for name in (
        "navigation",
        "navigation_source",
        "raw_groups",
        "inline_anchors",
        "nonpainting_lines",
        "error",
        "observed",
        "size_of",
    ):
        yield from walk(probe[name], (name,), name == "navigation")
        if isinstance(probe[name], dict):
            yield "unknown-" + name, (name, "unknown_field"), 1, False
    yield "foreign-wire", ("wire",), SCHEMA + "-foreign", False
    yield "foreign-case", ("case",), probe["case"] + "-foreign", False
    yield "foreign-relations-wire", (
        "relation_probe",
        "wire",
    ), relations.SCHEMA + "-foreign", False
    yield "foreign-original-source", (
        "relation_probe",
        "marked_probe",
        "source_package",
        "source_fingerprint",
    ), "00" * 32, False
    if (
        probe["error"] is not None
        and probe["observed"]["parent_scope"] == "relation_driver"
    ):
        forged = copy.deepcopy(probe["observed"])
        forged["scope"] = "navigation_driver"
        forged["parent_scope"] = "relation_driver"
        forged["caller_before"] = probe["relation_probe"]["observed"]["caller_before"]
        forged["caller_after"] = forged["build_after"]
        forged["relations_prefix"] = probe["relation_probe"]["relations"]["charges"]
        yield "forged-delivered-Navigation-callback-on-semantic-stop", (
            "observed",
        ), forged, "semantic_scope"


def self_test(root, probe):
    original_sha = digest(display.canonical(probe).encode())
    prefix = relations.verify(root, probe["relation_probe"])
    cache = dict(
        relation_object=probe["relation_probe"],
        relation_result=dict(prefix),
        facts=original_facts(probe),
        capture=True,
    )
    verify(root, probe, cache)
    cache["capture"] = False
    rejected = 0
    for label, path, value, reseal in tamper_cases(probe):
        bad = relations.path_copy(probe, path, value, reseal == "delete")
        if reseal is True:
            try:
                reseal_navigation(bad, cache)
            except (
                AssertionError,
                IndexError,
                KeyError,
                TypeError,
                ValueError,
                OverflowError,
            ):
                pass
        try:
            verify(root, bad, cache)
        except (
            AssertionError,
            IndexError,
            KeyError,
            TypeError,
            ValueError,
            OverflowError,
        ) as error:
            if reseal == "semantic_scope":
                check(
                    isinstance(error, AssertionError)
                    and str(error)
                    == "semantic Navigation refusal cannot deliver a Navigationdriver callback",
                    "joint semantic-scope mutation reaches the specific provenance guard",
                )
            rejected += 1
        else:
            raise AssertionError("altered Navigation accepted: " + label)
    check(
        digest(display.canonical(probe).encode()) == original_sha,
        "complete original source/relations/geometry/observations immutable after matrix",
    )
    return rejected


def source_deriver_negative_checks(probe):
    """Exercise local authored-outline guards without a checksum-only oracle.

    These controls deliberately enter the source deriver on changed raw fields;
    they are reported separately from full Navigation mutation verification.
    No successful source-preparation or product authority is issued.
    """
    original_sha = digest(display.canonical(probe).encode())
    path = (
        "relation_probe",
        "marked_probe",
        "source_package",
        "wire",
        "outline",
        "entries",
    )
    entries = probe["relation_probe"]["marked_probe"]["source_package"]["wire"][
        "outline"
    ]["entries"]
    controls = []
    if entries:
        first = copy.deepcopy(entries[0])
        first["source_kind"] = (
            "semantic_container" if first["source_kind"] == "heading" else "heading"
        )
        controls.append((0, first, "outline exact original source kind"))
    if len(entries) > 1:
        second = copy.deepcopy(entries[1])
        second["source_node_id"] = entries[0]["source_node_id"]
        second["source_kind"] = entries[0]["source_kind"]
        controls.append((1, second, "outline unique increasing original source owner"))
        second = copy.deepcopy(entries[1])
        second["destination"] = entries[0]["destination"]
        controls.append((1, second, "outline unique original destination"))
    for index, replacement, message in controls:
        bad = relations.path_copy(probe, (*path, index), replacement)
        try:
            original_facts(bad)
        except AssertionError as error:
            check(str(error) == message, "specific local authored-outline guard")
        else:
            raise AssertionError("altered authored outline source accepted: " + message)
    check(
        digest(display.canonical(probe).encode()) == original_sha,
        "original full probe unchanged after local source controls",
    )
    return len(controls)


def geometry_deriver_negative_checks(probe):
    """Test the diagnostic Break-owner join without claiming line-layout replay.

    A width-specific graph that retains only part of the authored control census
    is deliberately refused. These raw diagnostic controls bypass the inherited
    checksum oracle and are counted separately from whole Navigation mutations.
    """
    draws = probe["relation_probe"]["marked_probe"]["paint_probe"]["display_probe"]
    fragments = draws["original_fragments"]
    needed = {
        fragments[value["fragment"]]["owner"]
        for value in draws["nonpainting"]
        if fragments[value["fragment"]]["line"]["items"]
    }
    if not needed:
        return 0
    original_sha = digest(display.canonical(probe).encode())
    facts = original_facts(probe)
    physical_inputs(facts)
    candidates = [
        fragment
        for fragment in fragments
        if fragment["owner"] in needed
        and fragment["line"] is not None
        and any(item["kind"] == "break" for item in fragment["line"]["items"])
    ]
    check(bool(candidates), "actual selected control line for local join guards")
    selected = candidates[0]
    key = (
        selected["owner"],
        selected["graph"]["line_fingerprint"],
        selected["line"]["line"],
    )
    controls = []
    missing = copy.deepcopy(facts)
    for fragment in missing["source"]["paint_probe"]["display_probe"][
        "original_fragments"
    ]:
        line = fragment["line"]
        if (
            line is not None
            and (fragment["owner"], fragment["graph"]["line_fingerprint"], line["line"])
            == key
        ):
            at = next(
                i for i, item in enumerate(line["items"]) if item["kind"] == "break"
            )
            del line["items"][at]
    controls.append(
        (
            missing,
            "complete original break control sequence for selected graph/paragraph",
        )
    )
    conflict = copy.deepcopy(facts)
    duplicate = copy.deepcopy(selected)
    duplicate["line"]["items"] = []
    conflict["source"]["paint_probe"]["display_probe"]["original_fragments"].append(
        duplicate
    )
    controls.append(
        (conflict, "same original graph line control census on repeated occurrences")
    )
    other_lines = {
        fragment["line"]["line"]
        for fragment in candidates
        if (fragment["owner"], fragment["graph"]["line_fingerprint"]) == key[:2]
    }
    if len(other_lines) > 1:
        partial = copy.deepcopy(facts)
        for fragment in partial["source"]["paint_probe"]["display_probe"][
            "original_fragments"
        ]:
            line = fragment["line"]
            if (
                line is not None
                and (
                    fragment["owner"],
                    fragment["graph"]["line_fingerprint"],
                    line["line"],
                )
                == key
            ):
                fragment["graph"]["line_fingerprint"] = "00" * 32
        controls.append(
            (
                partial,
                "complete original break control sequence for selected graph/paragraph",
            )
        )
    for changed, message in controls:
        try:
            physical_inputs(changed)
        except AssertionError as error:
            check(str(error) == message, "specific local diagnostic control-join guard")
        else:
            raise AssertionError("altered selected control census accepted: " + message)
    check(
        digest(display.canonical(probe).encode()) == original_sha,
        "original full probe unchanged after local geometry controls",
    )
    return len(controls)


def inherited_hashes():
    result = relations.inherited_hashes()
    result["verify_book_v2_column_relations.py"] = digest(
        Path(__file__).with_name("verify_book_v2_column_relations.py").read_bytes()
    ).hex()
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--summary", "--output", dest="output", type=Path)
    args = parser.parse_args()
    paths = sorted(args.directory.glob("*.json"))
    check(bool(paths), "actual Column Navigation probes")
    cases, scopes, totals, negatives, source_negatives, geometry_negatives = (
        [],
        Counter(),
        Counter(),
        0,
        0,
        0,
    )
    for ordinal, path in enumerate(paths, 1):
        probe = json.loads(path.read_text())
        try:
            result = verify(path.parent, probe)
            if args.self_test:
                count = self_test(path.parent, probe)
                negatives += count
                source_count = source_deriver_negative_checks(probe)
                source_negatives += source_count
                geometry_count = geometry_deriver_negative_checks(probe)
                geometry_negatives += geometry_count
                if source_count:
                    print(
                        f"SOURCE-DERIVER {ordinal}/{len(paths)} {path.name} {source_count}",
                        file=sys.stderr,
                        flush=True,
                    )
                if geometry_count:
                    print(
                        f"GEOMETRY-DERIVER {ordinal}/{len(paths)} {path.name} {geometry_count}",
                        file=sys.stderr,
                        flush=True,
                    )
                print(
                    f"SELFTEST {ordinal}/{len(paths)} {path.name} {count}",
                    file=sys.stderr,
                    flush=True,
                )
        except Exception as error:
            raise AssertionError(str(path) + ": " + str(error)) from error
        cases.append(probe["case"])
        scopes[result.pop("scope")] += 1
        result.pop("selection_mode", None)
        totals.update(result)
    summary = dict(
        schema="typaxis.book-2-column-navigation-verification/1",
        probes=len(paths),
        cases=cases,
        scopes=dict(scopes),
        totals=dict(totals),
        negative_tests=negatives,
        source_deriver_negative_tests=source_negatives,
        geometry_deriver_negative_tests=geometry_negatives,
        inherited_checker_sha256=inherited_hashes(),
        source_deriver_sha256=marked.SOURCE_DERIVER_SHA,
        scope="Normal full relation/source replay followed by independently derived authored Link/Anchor/outline registries, six-stream physical Navigation geometry and exact successful R/S/O/W/folded preimages. Manual relation/source callback semantic ConflictingLinks/UnplacedDestination snapshots verify their precise source owner and genuine builder stop quartet; they do not imply a delivered Navigationdriver callback or general failed-attempt/retry trace. Direct raw outline-source guard controls are counted separately from whole Navigation mutations, without full syntax-preparation replay. Original selected first-demand facts remain inherited issuer-linked witnesses; no full pagination/Page-label convergence or PDF objects/annotations/serialization claim. Selftest-only immutable-prefix/expected cache retains all caps, constructors and caller checks with full original SHA before/after each matrix.",
    )
    raw = json.dumps(summary, indent=2) + "\n"
    if args.output is not None:
        args.output.write_text(raw)
    print(raw, end="")


if __name__ == "__main__":
    main()
