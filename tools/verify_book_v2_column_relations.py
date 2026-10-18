#!/usr/bin/env python3
"""Verify Column source reading relations on authentic marked roots.

The unchanged marked checker independently verifies original JCS, decoded source,
physical paints, MCIDs, marked streams and source hierarchy. This checker derives
authored table/list/reference facts and the Note reading tree, then reconstructs
successful relation reservations and folded canonical preimages. Selected first
reference discovery remains a linked original pagination fact. No serialized
relation product bytes, final PDF, annotations or failed-attempt replay is claimed.
"""
from __future__ import annotations

import argparse
import copy
import json
import sys
from collections import Counter
from pathlib import Path

import verify_book_v2_column_marked as marked

check = marked.check
integer = marked.integer
strict_equal = marked.strict_equal
fields = marked.fields
hash_bytes = marked.hash_bytes
digest = marked.digest
PDFBudget = marked.PDFBudget
u = marked.u
SCHEMA = "typaxis.book-2-column-structure-relations-probe/1"
ALGORITHM = "typaxis.book-2-structure-relations/1"
MAX_U64 = (1 << 64) - 1
SLOTS = marked.SLOTS
# Fixed units for the frozen 64-bit Rust build. These follow the actual source
# field layouts; accepting arbitrary positive units would allow resealed charge
# substitutions. The checkpoint records the Rust compiler and binary hashes.
ABI_UNITS = dict(related_node=144, note=112, edge=96, usize=8)


def original_facts(probe):
    """Derive source registries from the already independently verified package."""
    original = probe["marked_probe"]
    document = original["source_package"]["wire"]["document"]
    nodes = original["source_structure"]["nodes"]
    values = marked.owner_values(document)
    indices = {tuple(node["key"]): i for i, node in enumerate(nodes)}
    check(len(indices) == len(nodes), "unique original logical source keys")
    definitions = document["footnotes"]
    definition_ids = {value["footnote_id"]: i for i, value in enumerate(definitions)}
    check(len(definition_ids) == len(definitions), "unique authored note IDs")
    references = {}

    def visit(value, origin):
        if isinstance(value, list):
            for child in value:
                visit(child, origin)
        elif isinstance(value, dict):
            if value.get("kind") == "footnote_reference":
                owner = value["node_id"]
                check(owner not in references, "unique authored note reference")
                references[owner] = (definition_ids[value["footnote_id"]], origin)
            for child in value.values():
                if isinstance(child, (dict, list)):
                    visit(child, origin)

    visit(document["blocks"], None)
    for di, note in enumerate(definitions):
        visit(note["blocks"], di)
    ordered = [
        node["key"][0]
        for node in nodes
        if node["key"][1] == "Source" and node["key"][0] in references
    ]
    check(len(ordered) == len(references), "all original reference owners retained")
    refs = [
        dict(
            owner=owner,
            target=references[owner][0],
            source_definition=references[owner][1],
        )
        for owner in ordered
    ]
    tables, lists = [], []
    for node in nodes:
        if node["key"][1] != "Source":
            continue
        owner = node["key"][0]
        value = values[owner]
        if value.get("kind") == "list":
            lists.append(dict(owner=owner, ordered=value["ordered"]))
        if value.get("kind") != "table":
            continue
        rows, cells = [], []
        for section in ("head", "body"):
            for row in value[section]:
                start = len(cells)
                for cell in row["cells"]:
                    attributes = nodes[indices[cell["node_id"], "Source"]]["cell"]
                    check(
                        attributes is not None, "original table cell source attributes"
                    )
                    cells.append(dict(owner=cell["node_id"], **attributes))
                rows.append(
                    dict(
                        owner=row["node_id"], section=section, cells=[start, len(cells)]
                    )
                )
        tables.append(dict(owner=owner, rows=rows, cells=cells))
    return dict(
        document=document,
        nodes=nodes,
        indices=indices,
        definitions=definitions,
        references=refs,
        tables=tables,
        lists=lists,
        maximum_depth=original["source_package"]["max_ast_depth"],
    )


def header_pairs(tables, budget, emit):
    """Count the two actual source table walks; no paint-order lookup."""
    for table in tables:
        budget.step()
        head_end = 0
        for row in table["rows"]:
            budget.step()
            if row["section"] != "head":
                break
            head_end = row["cells"][1]
        for cell in table["cells"][head_end:]:
            budget.step()
            end = cell["column"] + cell["colspan"]
            for header in table["cells"][:head_end]:
                budget.step()
                h_end = header["column"] + header["colspan"]
                if cell["column"] < h_end and header["column"] < end:
                    emit(cell["owner"], header["owner"])


def canonical_hash(
    source_fp, nodes, order, headers, related, notes, edges, budget, _report=False
):
    fp = budget.fold(hash_bytes(source_fp), ALGORITHM.encode())

    def number(value):
        nonlocal fp
        number_value = MAX_U64 if value is None else value
        encoded = number_value.to_bytes(8, "big") if _report else u(number_value)
        fp = budget.fold(fp, encoded)

    for node in nodes:
        for value in (
            node["parent"],
            node["first_child"],
            node["next"],
            node["depth"],
            *node["header_range"],
            *node["related_range"],
            node["note_index"],
        ):
            number(value)
        fp = budget.fold(
            fp,
            bytes(
                [
                    (
                        0
                        if node["numbering"] is None
                        else (1 if node["numbering"] == "Decimal" else 2)
                    )
                ]
            ),
        )
    for value in [*order, *headers, *related]:
        number(value)
    for note in notes:
        for value in (
            note["node"],
            note["link"],
            note["label"],
            int(note["painted"]),
            note["first"],
            note["last"],
            note["return"],
            note["reading"],
            note["after"],
        ):
            number(value)
    for edge in edges:
        for value in (
            edge["reference"],
            edge["note"],
            edge["definition"],
            edge["source_definition"],
            edge["reference_link"],
            edge["reference_label"],
            edge["definition_link"],
            edge["definition_label"],
            int(edge["painted"]),
            edge["next"],
        ):
            number(value)
    return fp


def derive_relations(facts, first_demand, size, budget):
    """Independent child vectors derive semantics; explicit steps replay charges."""
    source = facts["nodes"]
    count = len(source)
    definitions = facts["definitions"]
    refs = facts["references"]
    header_count = 0

    def count_header(_cell, _header):
        nonlocal header_count
        header_count += 1

    header_pairs(facts["tables"], budget, count_header)
    nd, ne, nr = len(definitions), len(refs), 2 * len(refs)
    records = 2 * count + nd + ne + header_count + nr
    spool = (
        count * (size["related_node"] + size["usize"])
        + nd * size["note"]
        + ne * size["edge"]
        + (header_count + nr) * size["usize"]
    )
    budget.reserve(records, spool, 0)
    budget.step(6)
    parents = [node["parent"] for node in source]
    children = [[] for _ in source]
    nodes = []
    for index, node in enumerate(source):
        budget.step()
        nodes.append(
            dict(
                parent=node["parent"],
                first_child=node["first_child"],
                next=node["next_sibling"],
                depth=0,
                headers=[],
                related=[],
                numbering=None,
                note_index=None,
                header_range=[0, 0],
                related_range=[0, 0],
            )
        )
        if node["parent"] is not None:
            children[node["parent"]].append(index)
    budget.step(count)  # previous-sibling initialization
    lookup = sorted(
        ((owner, SLOTS.index(slot)), index)
        for (owner, slot), index in facts["indices"].items()
    )

    def locate(owner, slot):
        result = marked.find_lookup(lookup, (owner, SLOTS.index(slot)), budget)
        check(result is not None, "source relation owner/slot exists")
        return result

    notes, edges, per_note = [], [], [[] for _ in definitions]
    for di, definition in enumerate(definitions):
        budget.step()
        owner = definition["node_id"]
        node, link, label = (
            locate(owner, slot) for slot in ("Source", "FootnoteLink", "FootnoteLabel")
        )
        check(
            source[node]["role"] == "Note"
            and parents[node] == 0
            and parents[link] == node
            and parents[label] == link,
            "original Note/link/label hierarchy",
        )
        nodes[node]["note_index"] = di
        notes.append(
            dict(
                node=node,
                link=link,
                label=label,
                painted=source[label]["first_binding"] is not None,
                first=None,
                last=None,
                **{"return": None},
                reading=None,
                after=None,
            )
        )
    for ri, ref in enumerate(refs):
        budget.step()
        di, owner = ref["target"], ref["owner"]
        node, link, label = (
            locate(owner, slot) for slot in ("Source", "FootnoteLink", "FootnoteLabel")
        )
        check(
            source[node]["role"] == "Reference"
            and parents[link] == node
            and parents[label] == link,
            "original reference/link/label hierarchy",
        )
        ancestor, origin = source[node]["parent"], None
        while ancestor is not None:
            budget.step()
            if nodes[ancestor]["note_index"] is not None:
                origin = nodes[ancestor]["note_index"]
                break
            ancestor = source[ancestor]["parent"]
        strict_equal(
            origin, ref["source_definition"], "authored definition reference origin"
        )
        painted = source[label]["first_binding"] is not None
        note = notes[di]
        check(
            not painted or note["painted"],
            "painted reference demands painted original Note",
        )
        if note["last"] is not None:
            edges[note["last"]]["next"] = ri
        else:
            note["first"] = ri
        note["last"] = ri
        if painted and note["return"] is None:
            note["return"] = ri
        nodes[node]["related"].append(note["node"])
        nodes[note["node"]]["related"].append(node)
        per_note[di].append(ri)
        edges.append(
            dict(
                reference=node,
                note=note["node"],
                definition=di,
                source_definition=origin,
                reference_link=link,
                reference_label=label,
                definition_link=note["link"],
                definition_label=note["label"],
                painted=painted,
                next=None,
            )
        )
    budget.step(nd)  # explicit unpainted definition count
    check(
        type(first_demand) is list and len(first_demand) == nd,
        "complete selected first-reference forest",
    )

    def anchor(reference):
        branch = reference
        while source[branch]["parent"] is not None:
            parent = source[branch]["parent"]
            budget.step()
            if source[parent]["role"] in ("P", "H1", "H2", "H3", "H4", "H5", "H6") or (
                source[parent]["role"] == "Lbl" and source[parent]["key"][1] == "Source"
            ):
                return parent, branch
            branch = parent
        raise AssertionError("authored inline-bearing reference ancestor")

    def detach(target):
        parent = parents[target]
        check(parent is not None, "linked Note before relocation")
        children[parent].remove(target)
        parents[target] = None

    def insert(target, parent, after):
        check(
            parents[target] is None and parents[after] == parent,
            "authored Note relocation branch",
        )
        children[parent].insert(children[parent].index(after) + 1, target)
        parents[target] = parent

    for di, note in enumerate(notes):
        budget.step()
        first = first_demand[di]
        check(
            (first is not None) == note["painted"],
            "selected demand equals actual Note label binding",
        )
        if first is None:
            continue
        integer(first, 32, False)
        chosen = None
        for ei in per_note[di]:
            budget.step()
            edge = edges[ei]
            if source[edge["reference"]]["key"][0] == first:
                check(
                    edge["painted"] and chosen is None,
                    "unique painted selected first reference",
                )
                chosen = ei
        check(chosen is not None, "selected first reference belongs to original target")
        parent, branch = anchor(edges[chosen]["reference"])
        note.update(reading=chosen, after=branch)
        detach(note["node"])
        insert(note["node"], parent, branch)
    for note in notes:
        budget.step()
        if not note["painted"]:
            continue
        ancestor, seen = note["node"], set()
        while ancestor is not None:
            budget.step()
            check(
                ancestor not in seen and len(seen) < count,
                "selected first-reference forest has no dependency cycle",
            )
            seen.add(ancestor)
            ancestor = parents[ancestor]
    for di, note in enumerate(notes):
        budget.step()
        if not note["painted"]:
            continue
        target, selected = note["node"], None
        for ei in per_note[di]:
            budget.step()
            if not edges[ei]["painted"]:
                continue
            parent, branch = anchor(edges[ei]["reference"])
            ancestor, seen, cycle = parent, set(), False
            while ancestor is not None:
                check(
                    ancestor not in seen and len(seen) < count,
                    "acyclic current reading ancestry",
                )
                seen.add(ancestor)
                budget.step()
                if ancestor == target:
                    cycle = True
                    break
                ancestor = parents[ancestor]
            if not cycle:
                selected = ei, parent, branch
        check(
            selected is not None, "painted Note has a legal external reading reference"
        )
        ei, parent, branch = selected
        note.update(reading=ei, after=branch)
        key = (source[edges[ei]["reference"]]["key"][0], source[target]["key"][0])
        detach(target)
        siblings = children[parent]
        after = branch
        for nxt in siblings[siblings.index(branch) + 1 :]:
            budget.step()
            other_di = nodes[nxt]["note_index"]
            if other_di is None:
                break
            other = notes[other_di]
            if other["after"] != branch:
                break
            oe = other["reading"]
            check(oe is not None, "prior attached Note has actual reading reference")
            if (source[edges[oe]["reference"]]["key"][0], source[nxt]["key"][0]) > key:
                break
            after = nxt
        insert(target, parent, after)
    next_siblings = [None] * count
    for siblings in children:
        for left, right in zip(siblings, siblings[1:]):
            next_siblings[left] = right
    order, current = [], 0
    while current is not None:
        budget.step()
        check(
            len(order) < count and nodes[current]["depth"] == 0,
            "complete unique final reading traversal",
        )
        parent = parents[current]
        nodes[current]["parent"] = parent
        nodes[current]["first_child"] = (
            children[current][0] if children[current] else None
        )
        nodes[current]["next"] = next_siblings[current]
        nodes[current]["depth"] = 1 if parent is None else nodes[parent]["depth"] + 1
        check(parent is not None or current == 0, "one original document reading root")
        check(
            nodes[current]["depth"] <= probe_depth(facts),
            "original maximum source nesting depth",
        )
        order.append(current)
        if children[current]:
            current = children[current][0]
            continue
        cursor = current
        while True:
            budget.step()
            if next_siblings[cursor] is not None:
                current = next_siblings[cursor]
                break
            if parents[cursor] is None:
                current = None
                break
            cursor = parents[cursor]
    check(len(order) == count, "no lost reading-tree source node")
    for value in facts["lists"]:
        budget.step()
        index = locate(value["owner"], "Source")
        check(source[index]["role"] == "L", "authored list relation owner")
        nodes[index]["numbering"] = "Decimal" if value["ordered"] else "Disc"
    headers = []

    def add_header(cell, header):
        index, hi = locate(cell, "Source"), locate(header, "Source")
        check(
            source[index]["role"] == "TD" and source[hi]["role"] == "TH",
            "original table header roles",
        )
        if not nodes[index]["headers"]:
            nodes[index]["header_range"] = [len(headers), len(headers)]
        check(
            nodes[index]["header_range"][1] == len(headers),
            "contiguous original table Headers source range",
        )
        nodes[index]["headers"].append(hi)
        headers.append(hi)
        nodes[index]["header_range"][1] += 1

    header_pairs(facts["tables"], budget, add_header)
    check(len(headers) == header_count, "two identical original header passes")
    related = []
    for node in nodes:
        budget.step()
        node["related_range"] = [len(related), len(related) + len(node["related"])]
        related.extend(node["related"])
    check(len(related) == nr, "all reciprocal reference/Note source links")
    budget.step(nr)  # fixed related-vector initialization
    budget.step(2 * ne)  # actual reciprocal-edge fill
    return dict(
        nodes=nodes,
        order=order,
        notes=notes,
        edges=edges,
        headers=headers,
        related=related,
        header_count=header_count,
        relocated=sum(note["painted"] for note in notes),
        unpainted=sum(not note["painted"] for note in notes),
        definition_edges=sum(edge["source_definition"] is not None for edge in edges),
    )


def probe_depth(facts):
    return facts["maximum_depth"]


def source_inputs(facts):
    tables = []
    for table in facts["tables"]:
        tables.append(
            dict(
                owner=table["owner"],
                rows=[
                    dict(row, section=row["section"].title()) for row in table["rows"]
                ],
                cells=[
                    {
                        name: cell[name]
                        for name in ("owner", "row", "column", "colspan", "rowspan")
                    }
                    for cell in table["cells"]
                ],
            )
        )
    return dict(
        tables=tables,
        lists=facts["lists"],
        definitions=[
            dict(index=di, owner=value["node_id"])
            for di, value in enumerate(facts["definitions"])
        ],
        references=[
            dict(
                index=ri,
                owner=ref["owner"],
                definition=ref["target"],
                source_definition=ref["source_definition"],
            )
            for ri, ref in enumerate(facts["references"])
        ],
    )


def selected_witnesses(probe, facts):
    inputs = fields(
        probe["inputs"],
        (
            "tables",
            "lists",
            "definitions",
            "references",
            "has_selected_demand",
            "first_demand",
            "unreferenced",
            "selected_pages",
            "measurements_fingerprint",
            "math_terminal_fingerprint",
        ),
        "original relation inputs",
    )
    for name, value in source_inputs(facts).items():
        strict_equal(inputs[name], value, "authored source relation " + name)
    original = probe["marked_probe"]
    display = original["paint_probe"]["display_probe"]
    terminal = marked.display.decode_source(display)
    strict_equal(
        inputs["math_terminal_fingerprint"],
        display["source_fingerprint"],
        "same authentic Column math/source closure",
    )
    strict_equal(
        inputs["measurements_fingerprint"],
        display["source_measurements_fingerprint"],
        "same original selected measurement owner",
    )
    nd = len(facts["definitions"])
    pages = inputs["selected_pages"]
    check(type(pages) is list and bool(pages), "authentic selected source demand pages")
    strict_equal(
        inputs["has_selected_demand"], True, "selected original source demand exists"
    )
    check(len(pages) == len(terminal["pages"]), "all physical selected page witnesses")
    names = sorted(
        {
            rule["named_page"]
            for rule in original["source_package"]["wire"]["page_masters"][
                "selection_rules"
            ]
            if rule["named_page"] is not None
        }
    )
    balances = {value["page"]: value["fingerprint"] for value in terminal["balances"]}
    owners = marked.owner_values(facts["document"])
    prior_status = ["Unreferenced"] * nd
    first = [None] * nd
    first_pages = [None] * nd
    next_item = next_table = 0
    for ordinal, (page, physical) in enumerate(zip(pages, terminal["pages"])):
        fields(
            page,
            (
                "page",
                "name",
                "forced",
                "balanced",
                "next_item",
                "next_table",
                "is_complete",
                "pending",
                "statuses",
                "first_references",
            ),
            "authentic selected source page",
        )
        strict_equal(page["page"], ordinal, "dense selected original page index")
        strict_equal(page["page"], physical["page"], "same actual physical page")
        named_index = physical["named_index"]
        expected_name = None if named_index is None else names[named_index]
        strict_equal(
            page["name"], expected_name, "same named original physical master context"
        )
        strict_equal(
            page["balanced"],
            balances.get(ordinal),
            "same selected balanced or None receipt",
        )
        strict_equal(
            page["forced"],
            display["pages"][ordinal]["forced"],
            "same original forced owner in the unchanged marked physical snapshot",
        )
        if page["forced"] is not None:
            owner = integer(page["forced"], 32, False)
            check(
                owner in owners and owners[owner].get("kind") == "page_break",
                "authored forced source owner",
            )
        item, table = integer(page["next_item"], 64, False), integer(
            page["next_table"], 64, False
        )
        check(
            item >= next_item and table >= next_table,
            "nondecreasing original body source endpoints",
        )
        next_item, next_table = item, table
        check(type(page["is_complete"]) is bool, "typed source completion")
        strict_equal(
            page["is_complete"],
            item == pages[-1]["next_item"]
            and table == pages[-1]["next_table"]
            and not page["pending"],
            "source completion agrees with the completed original endpoint and demand queue",
        )
        check(
            type(page["statuses"]) is list and len(page["statuses"]) == nd,
            "every definition demand status",
        )
        check(
            type(page["first_references"]) is list
            and len(page["first_references"]) == nd,
            "every original selected first-reference owner",
        )
        pending = []
        for di, status in enumerate(page["statuses"]):
            check(
                status in ("Unreferenced", "Pending", "Complete"),
                "typed original demand status",
            )
            check(
                ("Unreferenced", "Pending", "Complete").index(status)
                >= ("Unreferenced", "Pending", "Complete").index(prior_status[di]),
                "original demand status never regresses",
            )
            reference = page["first_references"][di]
            check(
                (reference is None) == (status == "Unreferenced"),
                "first reference exactly for an actually demanded definition",
            )
            if reference is not None:
                integer(reference, 32, False)
                if first[di] is None:
                    first[di], first_pages[di] = reference, ordinal
                strict_equal(
                    reference, first[di], "retained first reference never changes"
                )
            if status == "Pending":
                pending.append(di)
        check(
            type(page["pending"]) is list
            and len(set(page["pending"])) == len(page["pending"]),
            "unique actual pending definition queue",
        )
        strict_equal(
            sorted(page["pending"]),
            pending,
            "pending queue and actual definition statuses",
        )
        prior_status = list(page["statuses"])
    check(
        pages[-1]["is_complete"] and not pages[-1]["pending"],
        "completed original selected body and notes",
    )
    strict_equal(
        inputs["first_demand"],
        first,
        "sealed relation view equals original final selected first-reference forest",
    )
    unreferenced = sum(status == "Unreferenced" for status in prior_status)
    strict_equal(
        inputs["unreferenced"],
        unreferenced,
        "explicit selected unreferenced definitions",
    )
    strict_equal(
        unreferenced,
        terminal["unreferenced"],
        "same independently checked closed-source unreferenced count",
    )
    # This constrains the linked selected witnesses by original target and actual
    # semantic bindings. It does not rediscover the joint pagination search.
    label_pages = {}
    for binding in original["source_structure"]["bindings"]:
        node = facts["nodes"][binding["node"]]
        if node["key"][1] == "FootnoteLabel":
            label_pages.setdefault(node["key"][0], []).append(binding["page"])
    reference_by_owner = {ref["owner"]: ref for ref in facts["references"]}
    for di, owner in enumerate(first):
        if owner is None:
            continue
        check(
            owner in reference_by_owner and reference_by_owner[owner]["target"] == di,
            "selected first reference original target",
        )
        check(
            owner in label_pages and min(label_pages[owner]) <= first_pages[di],
            "selected first reference already has an actual semantic binding by its demand page",
        )
    return first


def reported_nodes(derived):
    return [
        dict(
            parent=node["parent"],
            first_child=node["first_child"],
            next_sibling=node["next"],
            depth=node["depth"],
            headers=node["headers"],
            related=node["related"],
            list_numbering=node["numbering"],
        )
        for node in derived["nodes"]
    ]


def reported_edges(derived):
    return [
        dict(
            reference=edge["reference"],
            note=edge["note"],
            definition=edge["definition"],
            source_definition=edge["source_definition"],
            ref_link=edge["reference_link"],
            ref_label=edge["reference_label"],
            def_link=edge["definition_link"],
            def_label=edge["definition_label"],
            painted=edge["painted"],
            next=edge["next"],
        )
        for edge in derived["edges"]
    ]


def _strict_report(actual, expected, original, label):
    """Private matrix comparison: shared originals were verified in full.

    Only path-copied mutations reach this helper. A complete original-probe SHA
    is rechecked after the matrix; normal and changed-input verification uses
    strict_equal on every record.
    """
    if actual is original:
        return
    check(type(actual) is type(expected), label + " primitive type")
    if isinstance(expected, dict):
        check(set(actual) == set(expected), label + " fields")
        for name in expected:
            _strict_report(actual[name], expected[name], original[name], label)
    elif isinstance(expected, list):
        check(len(actual) == len(expected), label + " length")
        for index, value in enumerate(expected):
            _strict_report(actual[index], value, original[index], label)
    else:
        check(actual == expected, label + " value")


def verify(root, probe, _cache=None):
    fields(
        probe,
        ("wire", "case", "marked_probe", "inputs", "relations", "observed", "size_of"),
        "Column structure relation probe",
    )
    check(
        probe["wire"] == SCHEMA and probe["case"] == probe["marked_probe"]["case"],
        "actual Column relations schema/case",
    )
    original = probe["marked_probe"]
    identity = _cache is not None and original is _cache["marked_object"]
    if identity:
        previous = dict(_cache["marked_result"])
        facts = _cache["facts"]
    else:
        previous = marked.verify(root, original)
        facts = original_facts(probe)
    size = fields(
        probe["size_of"],
        ("related_node", "note", "edge", "usize"),
        "actual relation reservation units",
    )
    for value in size.values():
        check(integer(value, 64, False) > 0, "positive relation reservation unit")
    strict_equal(size, ABI_UNITS, "frozen 64-bit relation source field layouts")
    observed = fields(
        probe["observed"],
        (
            "scope",
            "maximum_work",
            "constructors",
            "caller_before",
            "caller_after",
            "marked_prefix",
            "source_prefix",
            "ancestor_callback",
        ),
        "complete relation observation",
    )
    check(
        observed["scope"]
        in ("relation_driver", "source_callback", "retained_source_cut"),
        "explicit captured relation success scope",
    )
    strict_equal(
        observed["maximum_work"],
        original["observed"]["maximum_work"],
        "same cumulative marked/relation work cap",
    )
    check(
        type(observed["constructors"]) is list and len(observed["constructors"]) == 12,
        "complete twelve authentic constructors",
    )
    strict_equal(
        observed["constructors"][:11],
        original["observed"]["constructors"],
        "unchanged eleven marked constructor prefix",
    )
    source_prefix = original["source_structure"]["charges"]
    strict_equal(
        observed["source_prefix"],
        source_prefix,
        "same complete marked source result before relation construction",
    )
    strict_equal(
        observed["ancestor_callback"],
        original["observed"]["ancestor_callback"],
        "actual marked driver or manual source ancestor observation",
    )
    if observed["scope"] == "relation_driver":
        check(
            original["observed"]["scope"] == "marked_driver",
            "relations layered on actual marked driver",
        )
        strict_equal(
            observed["marked_prefix"],
            source_prefix,
            "authentic marked callback before relation constructor",
        )
        strict_equal(
            observed["marked_prefix"],
            original["observed"]["ancestor_callback"],
            "unchanged embedded actual marked observation",
        )
        strict_equal(
            observed["caller_before"],
            original["observed"]["caller_before"],
            "same whole command starting prefix",
        )
    else:
        check(
            original["observed"]["scope"] == observed["scope"],
            "manual selected source lineage",
        )
        check(
            observed["caller_before"] is None
            and observed["caller_after"] is None
            and observed["marked_prefix"] is None,
            "manual relation stage invents no whole driver observation",
        )
    budget = PDFBudget(source_prefix)
    settled_output = budget.output
    budget.reserve(1, 0, 0)
    budget.matches4(observed["constructors"][11])
    input_key = digest(
        marked.display.canonical(
            [
                probe["inputs"],
                size,
                observed["marked_prefix"],
                observed["source_prefix"],
                observed["ancestor_callback"],
                source_prefix,
            ]
        ).encode()
    )
    cached = identity and input_key == _cache.get("input_key")
    if cached:
        derived, fp = _cache["derived"], _cache["fingerprint"]
        budget = PDFBudget(list(_cache["final_budget"]))
    else:
        first = selected_witnesses(probe, facts)
        derived = derive_relations(facts, first, size, budget)
        strict_equal(
            derived["unpainted"],
            probe["inputs"]["unreferenced"],
            "unpainted Note/source closure count",
        )
        fp = canonical_hash(
            original["source_structure"]["fingerprint"],
            derived["nodes"],
            derived["order"],
            derived["headers"],
            derived["related"],
            derived["notes"],
            derived["edges"],
            budget,
        )
    stage = fields(
        probe["relations"],
        ("fingerprint", "charges", "nodes", "reading_order", "notes", "edges"),
        "retained source relation result",
    )
    expected_report = (
        _cache["expected_report"]
        if cached
        else dict(
            nodes=reported_nodes(derived),
            reading_order=derived["order"],
            notes=derived["notes"],
            edges=reported_edges(derived),
        )
    )
    for name, expected in expected_report.items():
        if cached:
            _strict_report(
                stage[name],
                expected,
                _cache["original_report"][name],
                "exact original relation " + name,
            )
        else:
            strict_equal(stage[name], expected, "exact original relation " + name)
    check(
        fp == hash_bytes(stage["fingerprint"]),
        "exact folded relation canonical preimages",
    )
    budget.matches4(stage)
    check(
        budget.output == settled_output,
        "relations inherit settled output with zero credit or additional output",
    )
    if observed["scope"] == "relation_driver":
        strict_equal(
            observed["caller_after"],
            budget.triple4(),
            "complete relation success retained by caller",
        )
    base = json.loads(original["paint_probe"]["display_probe"]["limits_canonical_jcs"])[
        "base"
    ]
    maximum_work = integer(observed["maximum_work"], 64, False)
    for values in [*observed["constructors"], source_prefix, stage["charges"]]:
        r, s, o, w = PDFBudget(values).triple4()
        check(
            r <= base["max_fragments"]
            and s <= base["max_spool_bytes"]
            and o <= base["max_output_bytes"]
            and w <= maximum_work,
            "inclusive complete relation R/S/O/W ceilings",
        )
    result = dict(
        previous,
        scope=observed["scope"],
        relation_nodes=len(derived["nodes"]),
        reading_nodes=len(derived["order"]),
        relation_notes=len(derived["notes"]),
        reference_edges=len(derived["edges"]),
        header_relations=len(derived["headers"]),
        reciprocal_relations=len(derived["related"]),
        relocated_notes=derived["relocated"],
        explicit_unpainted_notes=derived["unpainted"],
        definition_reference_edges=derived["definition_edges"],
    )
    if _cache is not None and _cache.get("capture"):
        _cache.update(
            facts=facts,
            derived=copy.deepcopy(derived),
            fingerprint=bytes(fp),
            input_key=bytes(input_key),
            final_budget=list(budget.triple4()),
            expected_report=copy.deepcopy(expected_report),
            original_report=stage,
        )
    return result


def reseal_relations(probe, _cache=None, _slow=False):
    """Reissue reported relation folds, never independently expected records."""
    reusable = _cache is not None and probe["marked_probe"] is _cache["marked_object"]
    facts = _cache["facts"] if reusable else original_facts(probe)
    stage = probe["relations"]
    nodes = [
        dict(
            parent=value["parent"],
            first_child=value["first_child"],
            next=value["next_sibling"],
            depth=value["depth"],
            headers=list(value["headers"]),
            related=list(value["related"]),
            numbering=value["list_numbering"],
            note_index=None,
            header_range=[0, 0],
            related_range=[0, 0],
        )
        for value in stage["nodes"]
    ]
    notes = copy.deepcopy(stage["notes"])
    for di, note in enumerate(notes):
        nodes[note["node"]]["note_index"] = di
    edges = [
        dict(
            reference=value["reference"],
            note=value["note"],
            definition=value["definition"],
            source_definition=value["source_definition"],
            reference_link=value["ref_link"],
            reference_label=value["ref_label"],
            definition_link=value["def_link"],
            definition_label=value["def_label"],
            painted=value["painted"],
            next=value["next"],
        )
        for value in stage["edges"]
    ]
    headers = []
    for table in facts["tables"]:
        for row in table["rows"]:
            if row["section"] == "head":
                continue
            for cell in table["cells"][row["cells"][0] : row["cells"][1]]:
                node = nodes[facts["indices"][cell["owner"], "Source"]]
                if node["headers"]:
                    node["header_range"] = [
                        len(headers),
                        len(headers) + len(node["headers"]),
                    ]
                    headers.extend(node["headers"])
    related = []
    for node in nodes:
        node["related_range"] = [len(related), len(related) + len(node["related"])]
        related.extend(node["related"])
    fingerprint = canonical_hash(
        probe["marked_probe"]["source_structure"]["fingerprint"],
        nodes,
        stage["reading_order"],
        headers,
        related,
        notes,
        edges,
        PDFBudget([0, 0, 0, 0]) if _slow else _ReportHash(),
        _report=not _slow,
    ).hex()
    # A path-copy can share an untouched stage with the original. Its reported
    # fingerprint always gets a distinct dictionary before assignment.
    probe["relations"] = dict(stage, fingerprint=fingerprint)


class _ReportHash:
    """Reported-only SHA: normal replay owns every reservation/work step."""

    fold = staticmethod(marked.display.fold)


def changed_scalar(value):
    if type(value) is bool:
        return not value
    if value is None:
        return 0
    if type(value) is int:
        return value + 1
    if value == "Decimal":
        return "Disc"
    if value == "Disc":
        return "Decimal"
    return value + "-tampered"


def tamper_cases(probe):
    """Every actual relation record/edge plus constructor, unit and issuer links."""
    yield "schema", ("wire",), SCHEMA + "-tampered", False
    yield "case", ("case",), probe["case"] + "-tampered", False
    stage = probe["relations"]
    for name in ("nodes", "notes", "edges", "reading_order"):
        if stage[name]:
            yield name + "-removed", ("relations", name), [], True
            yield name + "-duplicated", ("relations", name), stage[name] + [
                stage[name][0]
            ], True
    for index, node in enumerate(stage["nodes"]):
        for name, value in node.items():
            path = ("relations", "nodes", index, name)
            if name not in ("headers", "related"):
                yield f"node-{index}-{name}", path, changed_scalar(value), True
                continue
            yield f"node-{index}-{name}-extra", path, value + [0], True
            if len(value) > 1 and value != list(reversed(value)):
                yield f"node-{index}-{name}-reversed", path, list(reversed(value)), True
            for at, item in enumerate(value):
                yield f"node-{index}-{name}-{at}-removed", path, value[:at] + value[
                    at + 1 :
                ], True
                yield f"node-{index}-{name}-{at}-owner", path, value[:at] + [
                    item + 1
                ] + value[at + 1 :], True
                yield f"node-{index}-{name}-{at}-duplicate", path, value[:at] + [
                    item
                ] + value[at:], True
    for name in ("notes", "edges"):
        for index, value in enumerate(stage[name]):
            for field, scalar in value.items():
                yield f"{name}-{index}-{field}", (
                    "relations",
                    name,
                    index,
                    field,
                ), changed_scalar(scalar), True
    if len(stage["reading_order"]) > 1:
        yield "reading-order-reversed", ("relations", "reading_order"), list(
            reversed(stage["reading_order"])
        ), True
    for index, owner in enumerate(stage["reading_order"]):
        yield f"reading-order-{index}", (
            "relations",
            "reading_order",
            index,
        ), owner + 1, True
    for name, value in probe["size_of"].items():
        yield "unit-" + name, ("size_of", name), value + 1, False
    for axis, value in enumerate(stage["charges"]):
        yield f"relation-charge-{axis}-high", (
            "relations",
            "charges",
            axis,
        ), value + 1, False
        if value:
            yield f"relation-charge-{axis}-low", (
                "relations",
                "charges",
                axis,
            ), value - 1, False
    observed = probe["observed"]
    for index, quartet in enumerate(observed["constructors"]):
        for axis, value in enumerate(quartet):
            yield f"constructor-{index}-{axis}", (
                "observed",
                "constructors",
                index,
                axis,
            ), value + 1, False
    for name in (
        "caller_before",
        "caller_after",
        "marked_prefix",
        "source_prefix",
        "ancestor_callback",
    ):
        value = observed[name]
        if value is None:
            yield name + "-invented", ("observed", name), [0, 0, 0, 0], False
        else:
            yield name + "-removed", ("observed", name), None, False
            for axis, scalar in enumerate(value):
                yield f"{name}-{axis}", ("observed", name, axis), scalar + 1, False
    yield "work-ceiling", ("observed", "maximum_work"), observed[
        "maximum_work"
    ] + 1, False
    yield "scope", ("observed", "scope"), (
        "source_callback"
        if observed["scope"] == "relation_driver"
        else "relation_driver"
    ), False
    inputs = probe["inputs"]
    for name in ("measurements_fingerprint", "math_terminal_fingerprint"):
        yield "issuer-" + name, ("inputs", name), "00" * 32, False
    yield "selected-demand", ("inputs", "has_selected_demand"), not inputs[
        "has_selected_demand"
    ], False
    yield "unreferenced", ("inputs", "unreferenced"), inputs["unreferenced"] + 1, False
    yield "selected-pages-removed", ("inputs", "selected_pages"), [], False
    for index, owner in enumerate(inputs["first_demand"]):
        yield f"first-demand-{index}", ("inputs", "first_demand", index), (
            None if owner is not None else 0
        ), True
    for name in ("definitions", "references", "lists"):
        for index, record in enumerate(inputs[name]):
            for field, value in record.items():
                yield f"source-{name}-{index}-{field}", (
                    "inputs",
                    name,
                    index,
                    field,
                ), changed_scalar(value), False
    for index, table in enumerate(inputs["tables"]):
        yield f"source-table-{index}-owner", (
            "inputs",
            "tables",
            index,
            "owner",
        ), table["owner"] + 1, False
        for kind in ("rows", "cells"):
            for ri, record in enumerate(table[kind]):
                for field, value in record.items():
                    path = ("inputs", "tables", index, kind, ri, field)
                    if type(value) is list:
                        for ai, scalar in enumerate(value):
                            yield f"source-table-{index}-{kind}-{ri}-{field}-{ai}", (
                                *path,
                                ai,
                            ), scalar + 1, False
                    else:
                        yield f"source-table-{index}-{kind}-{ri}-{field}", path, changed_scalar(
                            value
                        ), False
    for index, page in enumerate(inputs["selected_pages"]):
        for name in ("page", "name", "forced", "balanced", "is_complete"):
            yield f"selected-page-{index}-{name}", (
                "inputs",
                "selected_pages",
                index,
                name,
            ), changed_scalar(page[name]), False
        for di, status in enumerate(page["statuses"]):
            yield f"selected-status-{index}-{di}", (
                "inputs",
                "selected_pages",
                index,
                "statuses",
                di,
            ), ("Pending" if status != "Pending" else "Complete"), False
        for di, reference in enumerate(page["first_references"]):
            yield f"selected-first-{index}-{di}", (
                "inputs",
                "selected_pages",
                index,
                "first_references",
                di,
            ), (None if reference is not None else 0), False
        if page["pending"]:
            yield f"selected-pending-{index}", (
                "inputs",
                "selected_pages",
                index,
                "pending",
            ), [], False
        else:
            yield f"selected-pending-{index}", (
                "inputs",
                "selected_pages",
                index,
                "pending",
            ), [0], False
    for name in probe:
        yield "missing-" + name, (name,), None, "delete"
    for section in ("inputs", "relations", "observed", "size_of"):
        for name in probe[section]:
            yield f"missing-{section}-{name}", (section, name), None, "delete"
    # A foreign marked object always reenters the unchanged normal marked
    # checker. There is no wire flag permitting a cached prefix to be forged.
    yield "foreign-marked-schema", (
        "marked_probe",
        "wire",
    ), marked.SCHEMA + "-tampered", False
    yield "foreign-marked-source-hash", (
        "marked_probe",
        "source_package",
        "source_fingerprint",
    ), "00" * 32, False


def path_copy(value, path, replacement, delete=False):
    """Copy changed ancestors; retained original containers remain immutable."""
    if not path:
        return copy.deepcopy(replacement)
    target = dict(value) if isinstance(value, dict) else list(value)
    if len(path) == 1:
        if delete:
            del target[path[0]]
        else:
            target[path[0]] = copy.deepcopy(replacement)
    else:
        target[path[0]] = path_copy(value[path[0]], path[1:], replacement, delete)
    return target


def self_test(root, probe):
    previous = marked.verify(root, probe["marked_probe"])
    original_sha = digest(marked.display.canonical(probe).encode())
    cache = dict(
        marked_object=probe["marked_probe"],
        marked_result=dict(previous),
        facts=original_facts(probe),
        capture=True,
    )
    verify(root, probe, cache)
    cache["capture"] = False
    count = 0
    for label, path, value, reseal in tamper_cases(probe):
        bad = path_copy(probe, path, value, reseal == "delete")
        if reseal is True:
            try:
                reseal_relations(bad, cache)
            except (
                AssertionError,
                IndexError,
                KeyError,
                TypeError,
                ValueError,
                OverflowError,
            ):
                # Malformed reported structures still reach the strict verifier;
                # their rejected reporter is not itself counted as a rejection.
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
        ):
            count += 1
        else:
            raise AssertionError("altered source relation accepted: " + label)
    check(
        digest(marked.display.canonical(probe).encode()) == original_sha,
        "unchanged complete original source/marked/relations/observations after matrix",
    )
    return count


def inherited_hashes():
    values = marked.inherited_hashes()
    for name in (
        "verify_book_v2_column_marked.py",
        "verify_book_v2_structure_relations.py",
    ):
        values[name] = digest(Path(__file__).with_name(name).read_bytes()).hex()
    return values


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--summary", "--output", dest="output", type=Path)
    args = parser.parse_args()
    paths = sorted(args.directory.glob("*.json"))
    check(bool(paths), "actual Column source relation probes")
    cases, scopes, totals, negatives = [], Counter(), Counter(), 0
    for ordinal, path in enumerate(paths):
        probe = json.loads(path.read_text())
        try:
            result = verify(path.parent, probe)
            if args.self_test:
                count = self_test(path.parent, probe)
                negatives += count
                print(
                    f"SELFTEST {ordinal + 1}/{len(paths)} {path.name} {count}",
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
        schema="typaxis.book-2-column-structure-relations-verification/1",
        probes=len(paths),
        cases=cases,
        scopes=dict(scopes),
        totals=dict(totals),
        negative_tests=negatives,
        inherited_checker_sha256=inherited_hashes(),
        source_deriver_sha256=marked.SOURCE_DERIVER_SHA,
        scope="Normal replay verifies unchanged marked source then exact successful relation semantics, source registries, reservations and folded canonical preimages. Selected first-demand/endpoint witnesses remain linked original issuer facts, not independent pagination/discovery replay. No serialized relation product bytes, final PDF, annotations, failed-attempt replay or original CID-CFF positive gate. Selftest-only cache binds complete inputs, units, marked-prefix authority and unchanged marked object; every constructor/cap/caller check remains active and full marked SHA is rechecked after each matrix.",
    )
    raw = json.dumps(summary, indent=2) + "\n"
    if args.output is not None:
        args.output.write_text(raw)
    print(raw, end="")


if __name__ == "__main__":
    main()
