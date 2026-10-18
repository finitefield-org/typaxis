#!/usr/bin/env python3
"""Check actual Book /2 column balance layout probes independently.

Decodes source-bound input and receipt bytes and checks source occupancy,
capacity progress and physical geometry. Table checks bind recorded source
events and capacity prefixes; they do not independently replay the whole
table search or its skipped capacity intervals. This is layout/receipt
evidence; it does not authorize or accept a balanced PDF.
"""

import argparse
import copy
import hashlib
import json
import struct
from collections import Counter
from pathlib import Path


ALGORITHM = "typaxis.book-2-column-balance/1"
ALGORITHM_HASH = hashlib.sha256(ALGORITHM.encode()).digest()
CAPACITY_TRACE_HASH = hashlib.sha256(b"typaxis.book-2-column-capacity-trace/1").digest()
ZERO_HASH = bytes(32)
TABLE_GROUP_HASH = hashlib.sha256(b"typaxis.book-2-table-capacity-group/1").digest()
TABLE_KEEP_HASH = hashlib.sha256(b"typaxis.book-2-column-table-keep-group/1").digest()
TABLE_OBSERVATION_HASH = hashlib.sha256(b"typaxis.book-2-table-capacity-observation/1").digest()


class InvalidProbe(ValueError):
    """The recorded evidence does not satisfy the balance contract."""


def check(condition, message):
    if not condition:
        raise InvalidProbe(message)


def integer(value, name, minimum=0, maximum=(1 << 64) - 1):
    check(type(value) is int and minimum <= value <= maximum,
          f"{name}: expected integer in [{minimum}, {maximum}]")
    return value


def signed(value, name):
    return integer(value, name, -(1 << 63), (1 << 63) - 1)


def boolean(value, name):
    check(type(value) is bool, f"{name}: expected boolean")
    return value


def binary(value, name, size=None):
    if isinstance(value, str):
        check(len(value) % 2 == 0 and value == value.lower(),
              f"{name}: expected lowercase hex")
        try:
            result = bytes.fromhex(value)
        except ValueError as error:
            raise InvalidProbe(f"{name}: invalid hex") from error
        check(result.hex() == value, f"{name}: noncanonical hex")
    else:
        check(isinstance(value, list), f"{name}: expected hex or byte array")
        result = bytes(integer(v, name, 0, 255) for v in value)
    if size is not None:
        check(len(result) == size, f"{name}: expected {size} bytes")
    return result


def fingerprint(value, name):
    return binary(value, name, 32)


def sha256(data):
    return hashlib.sha256(data).digest()


class Reader:
    def __init__(self, data, name):
        self.data = data
        self.name = name
        self.at = 0

    def take(self, count):
        check(0 <= count <= len(self.data) - self.at,
              f"{self.name}: truncated at byte {self.at}")
        start = self.at
        self.at += count
        return self.data[start:self.at]

    def number(self, kind):
        size = struct.calcsize(kind)
        return struct.unpack(">" + kind, self.take(size))[0]

    def flag(self):
        value = self.number("B")
        check(value in (0, 1), f"{self.name}: nonboolean byte")
        return bool(value)

    def count(self, minimum_size):
        count = self.number("Q")
        check(count <= (len(self.data) - self.at) // minimum_size,
              f"{self.name}: count exceeds remaining encoded data")
        return count

    def done(self):
        check(self.at == len(self.data),
              f"{self.name}: {len(self.data) - self.at} trailing bytes")


def read_rect(reader):
    present = reader.flag()
    value = [reader.number("q") for _ in range(4)]
    if not present:
        check(value == [0, 0, 0, 0], "absent rect has nonzero coordinates")
        return None
    check(value[2] > 0 and value[3] > 0, "rect has nonpositive size")
    return value


def read_table_cursor(reader):
    present = reader.flag()
    cursor = dict(table=reader.number("Q"), row=reader.number("Q"),
                  caption_item=reader.number("Q"), offset=reader.number("q"),
                  initial=reader.flag(), started_rows=reader.flag(),
                  cells=reader.take(32))
    if not present:
        check(all(v == 0 or v == ZERO_HASH for v in cursor.values()),
              "absent table cursor has nonzero state")
        return None
    check(cursor["offset"] >= 0, "negative table cursor offset")
    return cursor


def read_state(reader):
    state = dict(page=reader.number("I"), empty=reader.flag())
    has_name = reader.flag()
    name = reader.number("Q")
    check(has_name or name == 0, "absent named page has nonzero index")
    state.update(name=name if has_name else None,
                 item=reader.number("Q"), table=reader.number("Q"),
                 table_end=reader.number("Q"), cursor=read_table_cursor(reader))
    check(state["table"] <= state["table_end"], "root table cursor past end")
    definitions = []
    for index in range(reader.count(1)):
        status = reader.number("B")
        check(status in (0, 1, 2), "invalid definition demand status")
        definition = dict(status=status)
        if status:
            definition["first_reference"] = reader.number("I")
        if status == 1:
            definition.update(definition=reader.number("Q"), item=reader.number("Q"))
            check(definition["definition"] == index, "pending definition index mismatch")
            has_table, started = reader.flag(), reader.flag()
            table = reader.number("Q")
            check(has_table or table == 0, "absent note table has nonzero index")
            definition.update(table=table if has_table else None, started=started,
                              cursor=read_table_cursor(reader))
        definitions.append(definition)
    pending = [reader.number("Q") for _ in range(reader.count(8))]
    check(len(pending) == len(set(pending)), "duplicate pending definition")
    check(set(pending) == {i for i, d in enumerate(definitions) if d["status"] == 1},
          "pending queue differs from demand states")
    state.update(definitions=definitions, pending=pending)
    return state


def read_part(reader):
    part = dict(top=reader.number("q"), height=reader.number("q"))
    part["items"] = [reader.number("Q"), reader.number("Q")] if reader.flag() else None
    part["table_fingerprint"] = reader.take(32)
    check(part["top"] >= 0 and part["height"] >= 0, "negative part occupancy")
    check((part["items"] is not None) != (part["table_fingerprint"] != ZERO_HASH),
          "part must own one ordinary range or one table")
    if part["items"] is not None:
        check(part["items"][0] < part["items"][1], "empty ordinary range")
    table = None
    if reader.flag():
        before, after = read_table_cursor(reader), read_table_cursor(reader)
        check(before is not None and after is not None, "baseline table has absent source cursors")
        table = dict(before=before, after=after, available=reader.number("q"),
                     used=reader.number("q"), header_height=reader.number("q"),
                     repeats_header=reader.flag(), has_caption=reader.flag(),
                     header_variant=reader.take(32))
        cells = []
        for _ in range(reader.count(60)):
            cells.append(dict(owner=reader.number("I"), index=reader.number("Q"),
                              top=reader.number("q"), height=reader.number("q"),
                              offset_before=reader.number("q"), offset_after=reader.number("q"),
                              content=[reader.number("Q"), reader.number("Q")]))
        table.update(cells=cells,
                     semantic_ranges=[[reader.number("Q"), reader.number("Q")]
                                      for _ in range(reader.count(16))])
    check((table is not None) == (part["table_fingerprint"] != ZERO_HASH),
          "baseline table event/fingerprint presence mismatch")
    part["table"] = table
    return part


def decode_input(data):
    reader = Reader(data, "balance input")
    result = dict(algorithm=reader.take(32), limits=reader.take(32),
                  package=reader.take(32), measurements=reader.take(32),
                  lines=reader.take(32), blocks=reader.take(32),
                  body=read_rect(reader), footnote=read_rect(reader))
    check(result["algorithm"] == ALGORITHM_HASH, "input algorithm mismatch")
    check(result["body"] is not None, "input has no authored body")
    count = reader.count(49)
    check(1 <= count <= 65535, "invalid column count")
    result.update(extent=reader.number("q"), entry=read_state(reader), next=read_state(reader))
    columns = []
    for _ in range(count):
        bounds = read_rect(reader)
        check(bounds is not None, "input column has no bounds")
        used = reader.number("q")
        parts = [read_part(reader) for _ in range(reader.count(49))]
        columns.append(dict(bounds=bounds, used=used, parts=parts))
    result["columns"] = columns
    reader.done()
    check(result["entry"]["page"] + 1 == result["next"]["page"],
          "baseline endpoint has wrong page ordinal")
    return result


def read_rejection(reader):
    rejection = dict(column=reader.number("H"), owner=reader.number("I"),
                     start=reader.number("Q"), end=reader.number("Q"),
                     required=reader.number("q"), deficit=reader.number("q"))
    rejection["table_group"] = reader.take(32) if reader.flag() else None
    return rejection


def decode_receipt(data):
    reader = Reader(data, "balance receipt")
    result = dict(algorithm=reader.take(32), input=reader.take(32),
                  selected=reader.number("q"))
    check(result["algorithm"] == ALGORITHM_HASH, "receipt algorithm mismatch")
    candidates = []
    for _ in range(reader.count(49)):
        candidate = dict(target=reader.number("q"), terminal=reader.flag(),
                         rejection_fingerprint=reader.take(32))
        start = reader.at
        candidate["rejections"] = [read_rejection(reader) for _ in range(reader.count(39))]
        candidate["rejection_bytes"] = data[start:reader.at]
        candidate["capacity_fingerprint"] = reader.take(32)
        candidate["capacity_bytes"] = reader.take(reader.count(1))
        candidates.append(candidate)
    result["candidates"] = candidates
    reader.done()
    check(candidates, "empty balance candidate trace")
    return result


def read_capacity_cursor(reader, depth=0):
    check(depth < 128, "table witness cursor exceeds bounded source depth")
    value = dict(table=reader.number("Q"), row=reader.number("Q"),
                 caption_item=reader.number("Q"), offset=reader.number("q"),
                 initial=reader.flag(), started_rows=reader.flag(), cells=reader.take(32),
                 cell_state=reader.flag(), nested=reader.flag())
    check(value["offset"] >= 0, "negative capacity cursor offset")
    if value["nested"]:
        has_remaining, has_name, transitions = reader.flag(), reader.flag(), reader.flag()
        remaining, name = reader.number("q"), reader.number("Q")
        check(has_remaining or remaining == 0, "absent nested remaining height is nonzero")
        check(has_name or name == 0, "absent nested named page is nonzero")
        check(remaining >= 0, "negative nested remaining height")
        cells = []
        for _ in range(reader.count(9)):
            cells.append(dict(next=reader.number("Q"),
                              child=read_capacity_cursor(reader, depth + 1) if reader.flag() else None))
        value.update(remaining=remaining if has_remaining else None,
                     name=name if has_name else None, named_transitions=transitions,
                     positions=cells)
    else:
        has_remaining, remaining = reader.flag(), reader.number("q")
        check(has_remaining or remaining == 0, "absent row remaining height is nonzero")
        check(remaining >= 0, "negative row remaining height")
        value.update(remaining=remaining if has_remaining else None,
                     positions=[dict(next=reader.number("Q"), child=None)
                                for _ in range(reader.count(8))])
    return value


def decode_capacity_certificate(data):
    reader = Reader(data, "table capacity certificate")
    check(reader.take(32) == TABLE_GROUP_HASH, "table capacity certificate algorithm mismatch")
    result = dict(measurements=reader.take(32), catalog=reader.take(32))
    headers, has_width, has_name = reader.flag(), reader.flag(), reader.flag()
    width, name = reader.number("q"), reader.number("Q")
    check((has_width and width > 0) or (not has_width and width == 0), "table certificate width presence mismatch")
    check(has_name or name == 0, "table certificate name presence mismatch")
    check(headers or result["catalog"] == ZERO_HASH, "absent header catalog has nonzero fingerprint")
    result.update(headers=headers, width=width if has_width else None,
                  name=name if has_name else None, current=reader.number("q"),
                  required=reader.number("q"), maximum=reader.number("q"),
                  trials=reader.number("Q"), skipped=reader.number("Q"))
    cursors = [read_capacity_cursor(reader) if reader.flag() else None for _ in range(4)]
    check(cursors[0] is not None and cursors[3] is not None, "capacity witness lacks original before/next cursors")
    hashes = [reader.take(32) for _ in range(6)]
    atoms = []
    for _ in range(3):
        length = reader.count(1)
        payload = reader.take(length)
        check(length >= 40 and length % 8 == 0, "capacity atom vector has invalid original topology length")
        atoms.append(payload)
    check(reader.take(8) == b"USEDHGHT", "table witness lacks actual used-height extension")
    current_present, lower_present = reader.flag(), reader.flag()
    current_used, lower_used, next_used = [reader.number("q") for _ in range(3)]
    check(current_present == (cursors[1] is not None) and lower_present == (cursors[2] is not None),
          "table used-height/cursor presence mismatch")
    check((current_present or current_used == 0) and (lower_present or lower_used == 0),
          "absent table selection has nonzero used height")
    check(0 <= current_used <= result["current"] and 0 <= lower_used < result["required"]
          and 0 <= next_used <= result["required"], "table actual used height exceeds replay capacity")
    result.update(current_used=current_used if current_present else None,
                  lower_used=lower_used if lower_present else None, next_used=next_used)
    reader.done()
    check(all(sha256(payload) == expected for payload, expected in zip(atoms, hashes[:3])),
          "table original atom SHA-256 mismatch")
    check(atoms[0] == atoms[1], "capacity at event minus one changes original source atoms")
    current = list(struct.unpack(">" + "Q" * (len(atoms[0]) // 8), atoms[0]))
    following = list(struct.unpack(">" + "Q" * (len(atoms[2]) // 8), atoms[2]))
    check(len(current) == len(following) and current != following
          and all(a <= b for a, b in zip(current, following)),
          "table capacity event does not strictly extend original source atoms")
    check(0 <= result["current"] < result["required"] <= result["maximum"],
          "table capacity event is nonpositive, nonincreasing or exceeds authored height")
    check(result["trials"] >= 3 and result["trials"] - 2 + result["skipped"]
          == result["required"] - result["current"], "table replay trial/skipped interval accounting mismatch")
    for cursor, fragment in zip(cursors[1:], hashes[3:]):
        check((cursor is None) == (fragment == ZERO_HASH), "capacity fragment/cursor presence mismatch")
    result.update(cursors=cursors, atom_hashes=hashes[:3], fragment_hashes=hashes[3:],
                  atoms=atoms, fingerprint=sha256(data))
    return result


def decode_keep_certificate(data):
    reader = Reader(data, "table keep composition")
    check(reader.take(32) == TABLE_KEEP_HASH, "table keep composition algorithm mismatch")
    result = dict(previous=reader.take(32), group=reader.take(32),
                  start=reader.number("Q"), end=reader.number("Q"),
                  top=reader.number("q"), required=reader.number("q"))
    reader.done()
    check(result["start"] <= result["end"] and 0 <= result["top"] <= result["required"],
          "table keep composition has invalid source extent/occupancy")
    result["fingerprint"] = sha256(data)
    return result


def decode_capacity_observation(data):
    reader = Reader(data, "actual table capacity observation")
    check(reader.take(32) == TABLE_OBSERVATION_HASH, "table observation algorithm mismatch")
    result = dict(measurements=reader.take(32), catalog=reader.take(32))
    headers, has_width, has_name = reader.flag(), reader.flag(), reader.flag()
    width, name = reader.number("q"), reader.number("Q")
    check((has_width and width > 0) or (not has_width and width == 0), "observation width presence mismatch")
    check(has_name or name == 0, "observation name presence mismatch")
    check(headers or result["catalog"] == ZERO_HASH, "absent observation catalog has nonzero fingerprint")
    capacity, maximum, used = reader.number("q"), reader.number("q"), reader.number("q")
    forced, owner, progress = reader.flag(), reader.number("I"), reader.flag()
    check(forced or owner == 0, "absent observation forced break has a source owner")
    cursors = [read_capacity_cursor(reader) if reader.flag() else None for _ in range(2)]
    check(all(c is not None for c in cursors), "observation lacks actual before/after source cursor")
    hashes = [reader.take(32) for _ in range(3)]
    atoms = []
    for _ in range(2):
        length = reader.count(1)
        payload = reader.take(length)
        check(length >= 40 and length % 8 == 0, "observation original topology length mismatch")
        atoms.append(payload)
    reader.done()
    check(all(sha256(payload) == expected for payload, expected in zip(atoms, hashes[:2])), "observation original atom SHA-256 mismatch")
    before = list(struct.unpack(">" + "Q" * (len(atoms[0]) // 8), atoms[0]))
    after = list(struct.unpack(">" + "Q" * (len(atoms[1]) // 8), atoms[1]))
    check(len(before) == len(after) and all(a <= b for a, b in zip(before, after))
          and progress == (before != after), "observation source-progress flag disagrees with original source atoms")
    check(0 <= used <= capacity <= maximum and maximum > 0, "observation used/capacity exceeds actual maximum")
    result.update(headers=headers, width=width if has_width else None, name=name if has_name else None,
                  current=capacity, required=capacity, maximum=maximum, next_used=used,
                  forced=owner if forced else None, progress=progress, cursors=cursors,
                  atoms=atoms, fingerprint=sha256(data), observation=True)
    return result


def read_source_point(reader):
    return dict(item=reader.number("Q"), table=reader.number("Q"), cursor=read_table_cursor(reader))


def decode_capacity_trace(data):
    reader = Reader(data, "column capacity trace")
    check(reader.take(32) == CAPACITY_TRACE_HASH, "capacity trace algorithm mismatch")
    result = dict(target=reader.number("q"))
    columns = []
    for _ in range(reader.count(194)):
        column = dict(index=reader.number("H"), before=read_source_point(reader), after=read_source_point(reader),
                      used=reader.number("q"))
        preceding_present, preceding = reader.flag(), reader.number("q")
        check(preceding_present or preceding == 0, "absent capacity prefix spacing is nonzero")
        check(preceding >= 0, "negative capacity prefix spacing")
        column["preceding"] = preceding if preceding_present else None
        column["parts"] = [read_part(reader) for _ in range(reader.count(50))]
        column["blocked_offset"] = reader.at
        column["blocked"] = reader.flag()
        columns.append(column)
    result["columns"] = columns
    reader.done()
    return result


def item_extent(item):
    return item["leading"] + item["height"] + item["trailing"]


def ordinary_group(items, start, end):
    check(0 <= start < end <= len(items), "ordinary group outside source")
    owner = items[start]["owner"]
    before = items[start]["before"]
    extent = 0
    at = start
    while at < end:
        item = items[at]
        check(item["source"] is not None, "ordinary keep group crosses forced break")
        if at != start:
            extent += items[at - 1]["after"] + item["before"]
        extent += item_extent(item)
        at += 1
        if not item["keep"]:
            break
    check(at == end or not items[at - 1]["keep"],
          "ordinary keep group ends before its legal cut")
    return dict(owner=owner, start=start, end=at, extent=extent,
                before=before, after=items[at - 1]["after"])


def ordinary_rejections(original, items, target):
    """Reconstruct capacity blockers from original metrics, without ranking."""
    check(all(part["table_fingerprint"] == ZERO_HASH
              for column in original["columns"] for part in column["parts"]),
          "ordinary blocker reconstruction cannot accept table parts")
    start = original["entry"]["item"]
    # The public next state can additionally consume an outside forced break.
    # A painted baseline part identifies the actual ordinary source endpoint.
    end = start
    for column in original["columns"]:
        for part in column["parts"]:
            first, last = part["items"]
            if items[first]["source"] is not None:
                end = max(end, last)
    at = start
    rejections = []
    for column in range(len(original["columns"])):
        used, preceding = 0, None
        while at < end:
            group = ordinary_group(items, at, end)
            gap = 0 if preceding is None else preceding + group["before"]
            required = used + gap + group["extent"]
            if required > target:
                rejections.append(dict(column=column, owner=group["owner"],
                                       start=group["start"], end=group["end"],
                                       required=required, deficit=required - target,
                                       table_group=None))
                break
            used, preceding, at = required, group["after"], group["end"]
    check(rejections, "failed ordinary trial has no positive capacity blocker")
    return rejections


def ordinary_part_extent(items, first, last):
    check(0 <= first < last <= len(items), "part source range outside original graph")
    if items[first]["source"] is None:
        check(last == first + 1, "forced break mixed with paintable items")
        return 0
    check(all(item["source"] is not None for item in items[first:last]),
          "ordinary part crosses an authored forced break")
    extent = sum(item_extent(item) for item in items[first:last])
    extent += sum(items[i - 1]["after"] + items[i]["before"]
                  for i in range(first + 1, last))
    return extent


def table_contents(table):
    return [cell["content"] for cell in table["cells"]] + ([] if table["caption"] is None else [table["caption"]["content"]])


def table_leaf_indices(index, tables, active=None):
    active = set() if active is None else active
    check(index not in active, "cyclic original table topology")
    active.add(index)
    result = []
    for contents in table_contents(tables[index]):
        for content in contents:
            source = content["source"]
            if source["kind"] == "item":
                result.append(source["index"])
            else:
                result.extend(table_leaf_indices(source["index"], tables, active))
    active.remove(index)
    return result


def validate_tables(probe, streams, nodes):
    tables = probe["source_tables"]
    flat = [item for stream in streams for item in stream]
    for index, table in enumerate(tables):
        check(table["index"] == index and table["source_index"] == index
              and table["owner"] in nodes and nodes[table["owner"]].get("kind") == "table",
              "measured table has a foreign source ordinal/owner")
        for key in ("height", "before", "after"):
            integer(table[key], "measured table." + key, 0, (1 << 63) - 1)
        boolean(table["keep"], "measured table keep")
        boolean(table["keep_together"], "measured table keep-together")
        for cell_index, cell in enumerate(table["cells"]):
            check(cell["index"] == cell_index and cell["source_index"] == cell_index
                  and cell["owner"] in nodes, "measured table cell source ordinal/owner mismatch")
        for contents in table_contents(table):
            end = 0
            for content in contents:
                source = content["source"]
                check(source["kind"] in ("item", "table"), "unknown original table content kind")
                if source["kind"] == "item":
                    item = flat[integer(source["index"], "table content item", 0, len(flat) - 1)]
                    top = end + item["before"] + item["leading"]
                    following = end + item["before"] + item_extent(item) + item["after"]
                else:
                    child_index = integer(source["index"], "child table index", 0, len(tables) - 1)
                    check(child_index > index, "nested table is not in original preorder")
                    child = tables[child_index]
                    top = end + child["before"]
                    following = top + child["height"] + child["after"]
                check(content["top"] == top and content["end"] == following,
                      "table content occupancy differs from original measured source/gaps")
                end = following
        for cell in table["cells"]:
            height = cell["content"][-1]["end"] if cell["content"] else 0
            check(cell["height"] == height, "natural cell height differs from original content occupancy")
        top = 0 if table["caption"] is None else table["caption"]["height"]
        if table["caption"] is not None:
            caption = table["caption"]
            check(caption["height"] == (caption["content"][-1]["end"] if caption["content"] else 0),
                  "caption height differs from original content occupancy")
        for row_index, row in enumerate(table["rows"]):
            check(row["index"] == row_index and row["owner"] in nodes and row["section"] in ("Head", "Body"),
                  "measured table row source mismatch")
            check(row["top"] == top, "table row origins do not close original caption/row extents")
            top += integer(row["height"], "natural row height", 0, (1 << 63) - 1)
        check(top == table["height"], "table natural height differs from original caption/row occupancy")
        leaves = table_leaf_indices(index, tables)
        check(len(leaves) == len(set(leaves)), "one original leaf appears twice inside a table topology")
    return tables


def validate_capacity_topology(certificate, tables):
    root = certificate["cursors"][0]["table"]

    def cursor(value):
        index = integer(value["table"], "capacity source table", 0, len(tables) - 1)
        table = tables[index]
        check(value["row"] <= len(table["rows"]) and value["caption_item"]
              <= (0 if table["caption"] is None else len(table["caption"]["content"])),
              "capacity cursor exceeds original row/caption topology")
        contents = table_contents(table) if value["nested"] else [c["content"] for c in table["cells"]]
        check(len(value["positions"]) == len(contents), "capacity cursor cell topology length mismatch")
        for position, content in zip(value["positions"], contents):
            check(position["next"] <= len(content), "capacity cell cursor exceeds original content count")
            child = position["child"]
            if child is not None:
                check(position["next"] < len(content) and content[position["next"]]["source"]
                      == {"kind": "table", "index": child["table"]}, "capacity nested cursor has a foreign original child")
                cursor(child)
    for value in certificate["cursors"]:
        if value is not None:
            check(value["table"] == root, "capacity certificate changes table owner")
            cursor(value)

    def find(value, index):
        if value is None:
            return None
        if value["table"] == index:
            return value["nested"]
        return next((found for p in value["positions"]
                     if (found := find(p["child"], index)) is not None), None)

    def atom_node(reader, index, nested, active, actual=None):
        check(index not in active, "cyclic capacity atom source topology")
        table = tables[index]
        row, caption, caption_done, header, terminal = [reader.number("Q") for _ in range(5)]
        caption_count = 0 if table["caption"] is None else len(table["caption"]["content"])
        check(row <= len(table["rows"]) and caption <= caption_count
              and caption_done in (0, 1) and header in (0, 1) and terminal in (0, 1),
              "capacity atom exceeds original topology or boolean range")
        check(not caption_done or (table["caption"] is not None and caption == caption_count),
              "caption atom completes an absent/unfinished original caption")
        check(not header or any(r["section"] == "Head" for r in table["rows"]),
              "header atom owns no original header source")
        check(not terminal or row == len(table["rows"]), "terminal table atom has incomplete original rows")
        if actual is not None:
            expected_row = 0 if actual["initial"] and not actual["started_rows"] else actual["row"]
            expected_done = table["caption"] is not None and actual["caption_item"] == caption_count and (
                actual["nested"] or actual["started_rows"] or actual["cell_state"] or actual["offset"] >= table["caption"]["height"])
            expected_header = actual["started_rows"] and any(r["section"] == "Head" for r in table["rows"])
            expected_terminal = not actual["initial"] and actual["started_rows"] and actual["row"] == len(table["rows"])
            check([row, caption, caption_done, header, terminal] ==
                  [expected_row, actual["caption_item"], int(expected_done), int(expected_header), int(expected_terminal)],
                  "capacity original atoms differ from actual scalar source cursor")
        active.add(index)
        contents = table_contents(table) if nested else [c["content"] for c in table["cells"]]
        for cell_index, content in enumerate(contents):
            following = reader.number("Q")
            check(following <= len(content), "capacity atom cell exceeds original content count")
            if actual is not None:
                check(following == actual["positions"][cell_index]["next"], "capacity atoms differ from actual original cell cursor")
            if nested:
                for source in (item["source"] for item in content if item["source"]["kind"] == "table"):
                    child = tables[source["index"]]
                    inferred = any(x["source"]["kind"] == "table" for c in table_contents(child) for x in c)
                    observed = next((found for c in certificate["cursors"]
                                     if (found := find(c, source["index"])) is not None), None)
                    atom_node(reader, source["index"], inferred if observed is None else observed, active)
        active.remove(index)
    if len(certificate["cursors"]) == 2:
        actual_cursors = certificate["cursors"]
    else:
        actual_cursors = [c or certificate["cursors"][0] for c in certificate["cursors"][1:]]
    for payload, actual in zip(certificate["atoms"], actual_cursors):
        reader = Reader(payload, "table original source atoms")
        atom_node(reader, root, certificate["cursors"][0]["nested"], set(), actual)
        reader.done()
    return tables[root]


def validate_table_certificates(raw, rejection, original, probe, tables, body):
    certificates = raw["table_group_certificates"]
    check(isinstance(certificates, list), "table certificate chain is not an array")
    if rejection["table_group"] is None:
        check(not certificates, "ordinary blocker carries an unbound table certificate")
        return 0
    check(certificates, "table blocker lacks actual bounded source-event certificates")
    previous, pending, last = ZERO_HASH, None, None
    admission, count, byte_count = 0, 0, 0
    for raw_bytes in certificates:
        data = binary(raw_bytes, "actual table certificate")
        byte_count += len(data)
        check(byte_count <= probe["observed"]["spool_charge"], "table certificate bytes exceed accepted cumulative spool")
        if data[:32] in (TABLE_GROUP_HASH, TABLE_OBSERVATION_HASH):
            check(pending is None, "uncomposed capacity certificate chain")
            pending = decode_capacity_certificate(data) if data[:32] == TABLE_GROUP_HASH else decode_capacity_observation(data)
            check(pending["measurements"] == original["measurements"], "capacity event belongs to foreign original measurements")
            check(pending["width"] == original["columns"][rejection["column"]]["bounds"][2]
                  and pending["maximum"] == original["body"][3]
                  and pending["name"] == original["next"]["name"], "capacity event has a foreign actual frame/name/height")
            validate_capacity_topology(pending, tables)
            count += 1
        elif data[:32] == TABLE_KEEP_HASH:
            check(pending is not None, "table keep composition has no preceding actual capacity event")
            composition = decode_keep_certificate(data)
            check(composition["previous"] == previous and composition["group"] == pending["fingerprint"],
                  "table keep composition breaks canonical witness hash chain")
            check(composition["start"] == rejection["start"] and composition["end"] <= rejection["end"],
                  "table keep composition changes original source range")
            check(composition["required"] == composition["top"] + pending["required"],
                  "table keep required extent differs from top plus nearest admission capacity")
            measured = tables[pending["cursors"][0]["table"]]
            leaves = table_leaf_indices(measured["index"], tables)
            check(not leaves or composition["end"] == max(leaves) + 1,
                  "table keep composition changes its original table source endpoint")
            if last is not None:
                check(composition["end"] >= last[1]["end"], "table keep chain moves backward in source")
                if pending["cursors"][0]["table"] != last[0]["cursors"][0]["table"]:
                    check(tables[last[0]["cursors"][0]["table"]]["keep"], "table keep chain crosses an authored legal cut")
            previous = composition["fingerprint"]
            admission = max(admission, composition["required"])
            last, pending = (pending, composition), None
        else:
            raise InvalidProbe("unknown table source-event certificate algorithm")
    if pending is not None:
        check(count == 1 and previous == ZERO_HASH, "partial capacity certificate has an unbound predecessor")
        check(pending["fingerprint"] == rejection["table_group"], "partial table source group SHA-256 mismatch")
        table = tables[pending["cursors"][0]["table"]]
        check(rejection["owner"] == table["owner"], "partial blocker has a foreign table source owner")
    else:
        check(previous == rejection["table_group"] and last is not None, "table keep source group SHA-256 mismatch")
        capacity, composition = last
        table = tables[capacity["cursors"][0]["table"]]
        expected = admission
        if rejection["end"] > composition["end"]:
            check(table["keep"], "table blocker adds an ordinary suffix without original keep")
            group = ordinary_group(body, composition["end"], rejection["end"])
            check(group["end"] == rejection["end"], "table keep suffix does not end on an original hard-legal group")
            expected = max(expected, composition["top"] + capacity["next_used"]
                           + table["after"] + group["before"] + group["extent"])
        check(rejection["required"] == expected, "table blocker differs from actual keep admission and used occupancy")
        first_bytes = binary(certificates[0], "first actual capacity event")
        first_capacity = decode_capacity_certificate(first_bytes) if first_bytes[:32] == TABLE_GROUP_HASH else decode_capacity_observation(first_bytes)
        first_table = tables[first_capacity["cursors"][0]["table"]]
        leaves = table_leaf_indices(first_table["index"], tables)
        first_leaf = min(leaves, default=rejection["start"])
        owner = body[rejection["start"]]["owner"] if leaves and rejection["start"] < first_leaf else first_table["owner"]
        check(rejection["owner"] == owner, "table keep blocker has a foreign original group owner")
    return count


def validate_table_event(event, part, tables, body_count):
    before, after = event["before"], event["after"]
    index = integer(before["table"], "baseline table index", 0, len(tables) - 1)
    table = tables[index]
    check(after["table"] == index, "table event changes original table ownership")
    check(0 <= before["row"] <= after["row"] <= len(table["rows"]), "table event row range outside original source")
    check(0 <= before["caption_item"] <= after["caption_item"]
          <= (0 if table["caption"] is None else len(table["caption"]["content"])),
          "table event caption range outside original source")
    check(0 <= before["offset"] <= after["offset"], "table event offsets are negative or decrease")
    if before["cells"] == ZERO_HASH and after["cells"] == ZERO_HASH:
        check(after["offset"] <= table["height"], "common table event exceeds original natural extent")
    check(event["has_caption"] == (table["caption"] is not None), "table event changes original caption ownership")
    check(0 <= event["used"] <= event["available"] and event["used"] == part["height"]
          and event["header_height"] >= 0, "table event height/capacity differs from original part")
    diff = after["offset"] - before["offset"]
    expected = diff if after["cells"] != ZERO_HASH or (not before["started_rows"] and event["has_caption"]) else diff + event["header_height"]
    check(event["used"] == expected, "table used height differs from actual original cursor/header occupancy")
    leaves = set(table_leaf_indices(index, tables))
    check(all(i < body_count for i in leaves), "body baseline table owns definition source items")
    selected_cells = set()
    for cell in event["cells"]:
        cell_index = integer(cell["index"], "selected source cell", 0, len(table["cells"]) - 1)
        measured = table["cells"][cell_index]
        check(cell["owner"] == measured["owner"] and cell_index not in selected_cells,
              "selected cell has duplicate/foreign original ownership")
        selected_cells.add(cell_index)
        check(0 <= cell["content"][0] <= cell["content"][1] <= len(measured["content"]),
              "selected cell content range outside original topology")
        check(0 <= cell["offset_before"] <= cell["offset_after"] <= measured["height"],
              "selected cell offsets outside original natural occupancy")
        check(0 <= cell["top"] and 0 <= cell["height"]
              and cell["top"] + cell["height"] <= event["used"],
              "selected cell escapes actual table part occupancy")
    covered = set()
    for first, last in event["semantic_ranges"]:
        check(0 <= first <= last <= body_count, "baseline semantic range outside original body")
        for item in range(first, last):
            check(item in leaves and item not in covered, "baseline table semantic range has foreign/duplicate source")
            covered.add(item)
    return table, leaves


def source_point(state):
    return {key: state[key] for key in ("item", "table", "cursor")}


def validate_capacity_prefix(candidate, original, probe, tables, body, ordinary):
    trace = decode_capacity_trace(candidate["capacity_bytes"])
    check(trace["target"] == candidate["target"] and len(trace["columns"]) == len(original["columns"]),
          "capacity trace changes target or physical column count")
    by_column = {r["column"]: r for r in candidate["rejections"]}
    check(len(by_column) == len(candidate["rejections"]), "duplicate canonical source blocker column")
    blocked = {c["index"] for c in trace["columns"] if c["blocked"]}
    check(set(by_column) == blocked, "canonical rejections omit/add an actual blocked column")
    body_owners = source_nodes(probe["authored_document"]["blocks"])
    children = {entry["source"]["index"] for table in tables for content in table_contents(table)
                for entry in content if entry["source"]["kind"] == "table"}
    roots = [t["index"] for t in tables if t["owner"] in body_owners and t["index"] not in children]
    point = source_point(original["entry"])
    ordinary_expected = {r["column"]: r for r in ordinary_rejections(original, body, trace["target"])} if ordinary else None
    for index, column in enumerate(trace["columns"]):
        check(column["index"] == index and column["before"] == point,
              "capacity trace loses original source-prefix continuity")
        used, preceding = 0, None
        for part in column["parts"]:
            if part["table"] is None:
                first, last = part["items"]
                check(first == point["item"] and point["cursor"] is None,
                      "capacity ordinary range has a source hole or foreign continuation")
                height = ordinary_part_extent(body, first, last)
                forced = body[first]["source"] is None
                gap = 0 if preceding is None or forced else preceding + body[first]["before"]
                check(part["top"] == used + gap and part["height"] == height,
                      "capacity ordinary prefix differs from actual original metrics/spacing")
                point = dict(item=last, table=point["table"], cursor=None)
                used, preceding = part["top"] + height, body[last - 1]["after"]
            else:
                event = part["table"]
                measured, leaves = validate_table_event(event, part, tables, len(body))
                check(event["before"]["table"] == point["table"]
                      and (point["cursor"] is None or point["cursor"] == event["before"]),
                      "capacity table prefix has a foreign original cursor/table")
                check(point["cursor"] is not None or event["before"]["initial"],
                      "capacity new table source does not begin at its actual initial cursor")
                check(not leaves or min(leaves) == point["item"], "capacity table prefix skips its original source start")
                gap = 0 if preceding is None else preceding + measured["before"]
                check(part["top"] == used + gap and event["available"] <= trace["target"] - part["top"],
                      "capacity table prefix changes actual outside gap or capacity")
                used, preceding = part["top"] + part["height"], measured["after"]
                after = event["after"]
                if after["row"] == len(measured["rows"]) and after["started_rows"]:
                    following = next((root for root in roots if root > point["table"]), original["entry"]["table_end"])
                    point = dict(item=max(leaves, default=point["item"] - 1) + 1, table=following, cursor=None)
                else:
                    point = dict(item=point["item"], table=point["table"], cursor=after)
        check(point == column["after"] and used == column["used"]
              and preceding == column["preceding"] and 0 <= used <= trace["target"],
              "capacity after-point/used/spacing differs from original traced parts")
        if ordinary:
            expected = ordinary_expected.get(index)
            check(column["blocked"] == (expected is not None), "ordinary capacity mask changes a proved actual blocker")
            if expected is not None:
                check(by_column[index] == expected, "ordinary capacity source-prefix blocker differs from original greedy proof")
                check(expected["start"] == point["item"], "ordinary blocker begins outside actual prefix endpoint")
        elif index in by_column:
            rejection = by_column[index]
            if rejection["table_group"] is None:
                group = ordinary_group(body, point["item"], len(body))
                gap = 0 if preceding is None else preceding + group["before"]
                required = used + gap + group["extent"]
                check(rejection == dict(column=index, owner=group["owner"], start=group["start"], end=group["end"],
                                        required=required, deficit=required - trace["target"], table_group=None),
                      "mixed ordinary blocker differs from actual traced prefix and original next group")
            else:
                raw = probe["pages"][original["entry"]["page"]]["balance"]["candidates"]
                reported = next(c for c in raw if c["target_height"] == candidate["target"])
                witness = next(r for r in reported["rejections"] if r["column"] == index)
                first = binary(witness["table_group_certificates"][0], "capacity boundary witness")
                proof = decode_capacity_certificate(first) if first[:32] == TABLE_GROUP_HASH else decode_capacity_observation(first)
                partial = next((p["table"] for p in reversed(column["parts"])
                                if p["table"] is not None and p["table"]["after"]["row"] < len(tables[p["table"]["before"]["table"]]["rows"])), None)
                expected_before = partial["before"] if partial is not None else point["cursor"]
                if expected_before is not None:
                    check({key: proof["cursors"][0][key] for key in expected_before} == expected_before,
                          "table capacity witness changes the actual blocked original cursor")
                else:
                    check(proof["cursors"][0]["table"] == point["table"], "table capacity witness changes actual next table source owner")
    return trace


def validate_original_occupancy(original, items, tables):
    """Recompute the original full-height occupancy, including real gaps."""
    body = original["body"]
    full = body[3]
    cursor = original["entry"]["item"]
    extent = 0
    for index, column in enumerate(original["columns"]):
        bounds = column["bounds"]
        check(bounds[1] == body[1] and bounds[3] == full,
              f"original column {index}: height/origin differs from authored body")
        check(body[0] <= bounds[0] and bounds[0] + bounds[2] <= body[0] + body[2],
              f"original column {index}: outside authored body width")
        if index:
            prior = original["columns"][index - 1]["bounds"]
            check(prior[0] + prior[2] <= bounds[0], "original columns overlap")
        used, preceding = 0, None
        for part in column["parts"]:
            if part["table"] is not None:
                table, leaves = validate_table_event(part["table"], part, tables, len(items))
                gap = 0 if preceding is None else preceding + table["before"]
                check(part["top"] == used + gap, "baseline table top differs from original outside spacing")
                used = part["top"] + part["height"]
                check(used <= full and part["table"]["available"] <= full - part["top"],
                      "original table capacity exceeds authored full column height")
                preceding = table["after"]
                if part["table"]["after"]["row"] == len(table["rows"]) and part["table"]["after"]["started_rows"]:
                    cursor = max(leaves, default=cursor - 1) + 1
                continue
            first, last = part["items"]
            check(first == cursor, "baseline column parts have a source hole or duplicate")
            item = items[first] if first < len(items) else None
            check(item is not None, "baseline part starts beyond source")
            forced = item["source"] is None
            gap = 0 if preceding is None or forced else preceding + item["before"]
            expected = ordinary_part_extent(items, first, last)
            check(part["top"] == used + gap, "baseline part top differs from original spacing")
            check(part["height"] == expected, "baseline part height differs from original metrics")
            used = part["top"] + expected
            check(0 <= used <= full, "baseline part exceeds authored full height")
            preceding = items[last - 1]["after"]
            cursor = last
        check(column["used"] == used, "baseline used height differs from actual source occupancy")
        extent += max(0, used)
    endpoint = original["next"]["item"]
    check(cursor <= endpoint <= len(items), "baseline source endpoint outside original graph")
    check(all(item["source"] is None for item in items[cursor:endpoint]),
          "baseline endpoint skips paintable source")
    check(extent > 0 and original["extent"] == extent,
          "initial extent differs from recomputed positive original occupancy")
    return extent


def validate_trace(receipt, original, maximum):
    count = len(original["columns"])
    full = original["body"][3]
    first = max(1, min(full, (original["extent"] + count - 1) // count))
    candidates = receipt["candidates"]
    check(len(candidates) <= maximum, "local candidate trace exceeds effective inclusive cap")
    check(candidates[0]["target"] == first, "first target is not exact ceil(E/N)")
    fingerprints = set()
    for index, candidate in enumerate(candidates):
        target = candidate["target"]
        check(0 < target <= full, "candidate target is not positive or exceeds authored height")
        last = index + 1 == len(candidates)
        check(candidate["terminal"] == last, "only the final candidate may be terminal")
        rejections = candidate["rejections"]
        if last:
            check(not rejections and candidate["rejection_fingerprint"] == ZERO_HASH,
                  "terminal candidate has a rejection payload")
            check(target == receipt["selected"], "terminal target differs from selected height")
            check(candidate["capacity_fingerprint"] == ZERO_HASH and not candidate["capacity_bytes"],
                  "terminal candidate carries a rejected capacity prefix")
            continue
        check(rejections, "nonterminal candidate has no source blocker")
        check(candidate["capacity_fingerprint"] == sha256(candidate["capacity_bytes"]),
              "capacity prefix SHA-256 mismatch")
        calculated = sha256(candidate["rejection_bytes"])
        check(candidate["rejection_fingerprint"] == calculated, "rejection SHA-256 mismatch")
        check(calculated not in fingerprints, "duplicate rejection hash/oscillation")
        fingerprints.add(calculated)
        columns = set()
        for rejection in rejections:
            check(rejection["column"] < count, "blocker column outside original frames")
            check(rejection["column"] not in columns, "duplicate blocker for one column")
            columns.add(rejection["column"])
            check(rejection["start"] < rejection["end"] or
                  (rejection["start"] == rejection["end"] and rejection["table_group"] is not None),
                  "empty ordinary blocker source range")
            check(rejection["required"] > target and rejection["deficit"] > 0,
                  "blocker does not prove a positive deficit")
            check(rejection["required"] - target == rejection["deficit"],
                  "deficit differs from required minus target")
        least = min(rejections, key=lambda r: (r["deficit"], r["column"], r["start"]))
        next_target = candidates[index + 1]["target"]
        check(next_target == target + least["deficit"], "next target is not the canonical minimum progress")
        check(target < next_target <= full, "target does not increase within authored height")
    return first


def encode_rejections(rejections):
    data = bytearray(struct.pack(">Q", len(rejections)))
    for rejection in rejections:
        data.extend(struct.pack(">HIQQqq", rejection["column"], rejection["owner"],
                                rejection["start"], rejection["end"],
                                rejection["required"], rejection["deficit"]))
        group = rejection["table_group"]
        data.append(int(group is not None))
        if group is not None:
            data.extend(group)
    return bytes(data)


def encode_receipt(receipt):
    data = bytearray(receipt["algorithm"] + receipt["input"])
    data.extend(struct.pack(">qQ", receipt["selected"], len(receipt["candidates"])))
    for candidate in receipt["candidates"]:
        data.extend(struct.pack(">qB", candidate["target"], int(candidate["terminal"])))
        data.extend(candidate["rejection_fingerprint"])
        data.extend(encode_rejections(candidate["rejections"]))
        data.extend(candidate["capacity_fingerprint"])
        data.extend(struct.pack(">Q", len(candidate["capacity_bytes"])))
        data.extend(candidate["capacity_bytes"])
    return bytes(data)


def normalize_items(values, name):
    check(isinstance(values, list), f"{name}: expected measured source item array")
    result = []
    for index, value in enumerate(values):
        item = dict(owner=integer(value["owner"], name + ".owner", 0, (1 << 32) - 1),
                    source=value["source"], keep=boolean(value["keep"], name + ".keep"))
        for key in ("height", "before", "after", "leading", "trailing"):
            item[key] = signed(value[key], f"{name}[{index}].{key}")
        check(all(item[key] >= 0 for key in ("height", "before", "after", "leading", "trailing")),
              f"{name}[{index}]: negative source occupancy/spacing")
        if item["source"] is None:
            check(item_extent(item) == 0 and not item["keep"],
                  "forced break has painting occupancy or hard keep")
        if "index" in value:
            check(value["index"] == index, "source array index mismatch")
        if "consumed" in value:
            check(value["consumed"] == item_extent(item), "reported consumed height differs from metrics")
        item.update(x=signed(value["x"], name + ".x"),
                    width=integer(value["width"], name + ".width", 1, (1 << 63) - 1))
        if item["source"] is not None and item["source"].get("kind") == "paragraph":
            units = item["source"]["units"]
            check(units is None or (isinstance(units, list) and len(units) == 2
                  and all(type(n) is int and n >= 0 for n in units) and units[0] <= units[1]),
                  "invalid original line unit range")
            item["units"] = units
        result.append(item)
    return result


def normalize_reported_candidates(values):
    check(isinstance(values, list), "candidate report is not an array")
    result = []
    for index, value in enumerate(values):
        check(value["index"] == index + 1, "candidate ordinal is not local and consecutive")
        rejections = []
        for raw in value["rejections"]:
            rejection = {key: integer(raw[key], "rejection." + key)
                         for key in ("column", "owner")}
            check(isinstance(raw["source_items"], list) and len(raw["source_items"]) == 2,
                  "rejection source range is not a pair")
            rejection.update(start=integer(raw["source_items"][0], "rejection.start"),
                             end=integer(raw["source_items"][1], "rejection.end"),
                             required=signed(raw["required_height"], "rejection.required"),
                             deficit=signed(raw["deficit"], "rejection.deficit"))
            group = raw["table_group_fingerprint"]
            rejection["table_group"] = None if group is None else fingerprint(group, "table group")
            rejections.append(rejection)
        result.append(dict(target=signed(value["target_height"], "candidate.target"),
                           terminal=boolean(value["terminal"], "candidate.terminal"),
                           rejection_fingerprint=fingerprint(value["rejection_fingerprint"], "rejection fingerprint"),
                           rejections=rejections,
                           capacity_fingerprint=fingerprint(value["capacity_fingerprint"], "capacity fingerprint"),
                           capacity_bytes=binary(value["capacity_canonical_bytes"], "capacity canonical bytes")))
    return result


def json_candidates(decoded):
    return [{key: value for key, value in candidate.items() if key != "rejection_bytes"}
            for candidate in decoded["candidates"]]


def rect(value, name):
    check(isinstance(value, list) and len(value) == 4, f"{name}: expected rectangle")
    result = [signed(n, name) for n in value]
    check(result[2] > 0 and result[3] > 0, f"{name}: nonpositive rectangle size")
    return result


def source_token(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)


def source_nodes(document):
    nodes = {}

    def walk(value):
        if isinstance(value, dict):
            if "node_id" in value:
                owner = integer(value["node_id"], "authored node_id", 0, (1 << 32) - 1)
                check(owner not in nodes, "duplicate authored source owner")
                nodes[owner] = value
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
    walk(document)
    return nodes


def authored_rect(value):
    if value is None:
        return None
    return rect([value[key] for key in ("x", "y", "width", "height")], "authored master rect")


def validate_master(page, masters):
    winner, master_id = None, masters["default_master_id"]
    for rule in masters["selection_rules"]:
        parity = rule["parity"]
        check(parity in ("any", "odd", "even"), "unknown authored page parity")
        if parity != "any" and (page["index"] + 1) % 2 != (1 if parity == "odd" else 0):
            continue
        first = rule.get("first")
        if first is not None and first != (page["index"] == 0):
            continue
        name = rule.get("named_page")
        if name is not None and name != page["name"]:
            continue
        priority = ((int(name is not None), int(first is not None), int(parity != "any")), rule["source_order"])
        if winner is None or priority > winner:
            winner, master_id = priority, rule["master_id"]
    matches = [m for m in masters["masters"] if m["master_id"] == master_id]
    check(len(matches) == 1, "authored selected master is absent or ambiguous")
    master = matches[0]
    body = authored_rect(master["body"])
    check(page["body_bounds"] == body and page["declared_footnotes"] == authored_rect(master.get("footnote")),
          "actual page does not retain the selected authored body/note regions")
    layout = master.get("column_layout")
    count = 1 if layout is None else integer(layout["count"], "authored column count", 1, 65535)
    gap = 0 if layout is None else integer(layout["gap"], "authored column gap", 0, (1 << 63) - 1)
    requires = layout is not None and layout["balance"] == "last_page"
    check(page["requires_balance"] == requires, "reported balance policy differs from selected authored master")
    check(len(page["columns"]) == count, "actual column count differs from selected authored master")
    available = body[2] - gap * (count - 1)
    check(available >= count, "authored columns have no positive capacity")
    width, residual = divmod(available, count)
    for index, column in enumerate(page["columns"]):
        expected = [body[0] + index * (width + gap), body[1],
                    width + (residual if index + 1 == count else 0), body[3]]
        check(column["original_bounds"] == expected, "authored column partition differs from exact residual-width partition")


def validate_limits(probe):
    data = probe["limits_canonical_jcs"]
    check(isinstance(data, str), "effective limits canonical JCS is not text")
    value = json.loads(data)
    check(set(value) == {"algorithm", "base", "extension"}
          and value["algorithm"] == "typaxis.m4-effective-resource-limits/1", "effective limits algorithm/schema mismatch")
    check(json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False) == data,
          "effective limits bytes are not canonical sorted JCS")
    calculated = sha256(data.encode())
    check(calculated == fingerprint(probe["limits_fingerprint"], "effective limits fingerprint"),
          "effective limits SHA-256 mismatch")
    for section in ("base", "extension"):
        check(isinstance(value[section], dict) and value[section], "effective limit section missing")
        for name, limit in value[section].items():
            integer(limit, "effective limits." + name)
    for name, limit in probe["caps"].items():
        check(name in value["base"] and value["base"][name] == limit,
              "reported effective cap differs from sealed base limits")
    return calculated


def validate_demand_states(original, probe, nodes, streams):
    references = {}
    for node in source_nodes(probe["authored_document"]["blocks"]).values():
        if node.get("kind") == "footnote_reference":
            references.setdefault(node["footnote_id"], node["node_id"])
    definitions = probe["authored_document"]["footnotes"]
    names = probe["source_page_names"]
    check(len(names) == len(set(names)) and all(isinstance(name, str) and name for name in names),
          "named page source registry is invalid")
    for state in (original["entry"], original["next"]):
        check(len(state["definitions"]) == len(definitions), "input demand has a foreign definition registry")
        if state["name"] is not None:
            integer(state["name"], "input named page", 0, len(names) - 1)
        for index, demand in enumerate(state["definitions"]):
            if demand["status"]:
                check(demand["first_reference"] == references.get(definitions[index]["footnote_id"]),
                      "input demand belongs to a foreign first reference owner")
            if demand["status"] == 1:
                check(demand["item"] <= len(streams[index + 1]), "pending definition cursor beyond original source")
    next_name = original["next"]["name"]
    actual_name = probe["pages"][original["entry"]["page"]]["name"]
    check((None if next_name is None else names[next_name]) == actual_name,
          "input endpoint changes the selected named-page state")


def validate_source_lines(probe, streams, nodes):
    paragraphs = probe["source_paragraphs"]
    for index, paragraph in enumerate(paragraphs):
        check(paragraph["index"] == index and paragraph["owner"] in nodes,
              "original paragraph ordinal/owner mismatch")
        if paragraph["lines"] is None:
            continue
        end = 0
        for line_index, line in enumerate(paragraph["lines"]):
            check(line["index"] == line_index, "original line ordinal mismatch")
            units = line["units"]
            check(isinstance(units, list) and len(units) == 2
                  and all(type(n) is int and n >= 0 for n in units) and units[0] <= units[1],
                  "invalid original paragraph units")
            check(units[0] == end, "original paragraph line units have gap/overlap")
            end = units[1]
            integer(line["height"], "original line height", 1, (1 << 63) - 1)
            integer(line["width"], "original line width", 1, (1 << 63) - 1)
            signed(line["baseline"], "original line baseline")
    for stream in streams:
        for item in stream:
            check(item["owner"] in nodes, "measured item owner absent from authored source")
            source = item["source"]
            if source is None:
                check(nodes[item["owner"]].get("kind") == "page_break",
                      "forced source item is not an authored page break")
                continue
            check(source["kind"] in ("paragraph", "figure", "vector_block", "native_math_block"),
                  "unknown measured source kind")
            if source["kind"] != "paragraph":
                integer(source["index"], "block source index", 0, (1 << 32) - 1)
                continue
            paragraph_index = integer(source["paragraph_index"], "paragraph index", 0, len(paragraphs) - 1)
            paragraph = paragraphs[paragraph_index]
            check(paragraph["owner"] == item["owner"] and paragraph["lines"] is not None,
                  "source item belongs to a foreign paragraph")
            line_index = integer(source["line_index"], "line index", 0, len(paragraph["lines"]) - 1)
            line = paragraph["lines"][line_index]
            for actual, expected in ((source["units"], line["units"]),
                                     (source["line_height"], line["height"]),
                                     (source["line_width"], line["width"]),
                                     (source["line_baseline"], line["baseline"])):
                check(actual == expected, "source item differs from original selected line")
            check(item["height"] == line["height"], "source height differs from original line metrics")


def normalize_selected_table(raw, tables):
    index = integer(raw["table_index"], "selected table index", 0, len(tables) - 1)
    cursors = []
    for prefix in ("before", "after"):
        cursors.append(dict(table=index, row=raw[prefix + "_row"],
                            caption_item=raw[prefix + "_caption_item"], offset=raw[prefix + "_offset"],
                            initial=raw[prefix + "_initial"], started_rows=raw[prefix + "_started_rows"],
                            cells=fingerprint(raw[prefix + "_progress"], "selected cell-progress fingerprint")))
    variant = raw["header_variant_fingerprint"]
    return dict(before=cursors[0], after=cursors[1], available=raw["available_height"],
                used=raw["used_height"], header_height=raw["header_height"],
                repeats_header=raw["repeats_header"], has_caption=tables[index]["caption"] is not None,
                header_variant=ZERO_HASH if variant is None else fingerprint(variant, "selected header graph source"),
                cells=[dict(owner=c["owner"], index=c["cell_index"], top=c["top"], height=c["height"],
                            offset_before=c["offset_before"], offset_after=c["offset_after"], content=c["content_range"])
                       for c in raw["cells"]], semantic_ranges=raw["semantic_ranges"])


def normalize_parts(parts, tables):
    result = []
    for part in parts:
        table = part["table"]
        result.append(dict(top=signed(part["top"], "selected part top"),
                           height=signed(part["height"], "selected part height"),
                           items=part["items"],
                           table_fingerprint=ZERO_HASH if table is None else fingerprint(table["fingerprint"], "table part"),
                           table=None if table is None else normalize_selected_table(table, tables)))
    return result


def validate_selected_columns(page, items, measured_body, tables):
    """Rebuild ordinary fragment origins from the actually selected ranges."""
    expected, regions = {}, []
    fragments = page["fragments"]
    next_fragment = 0
    for index, column in enumerate(page["columns"]):
        check(column["index"] == index, "physical column ordinal mismatch")
        bounds = rect(column["bounds"], "selected column bounds")
        authored = rect(column["original_bounds"], "authored column bounds")
        body = rect(page["body_bounds"], "authored body bounds")
        check(bounds[:3] == authored[:3] and authored[1] == body[1] and authored[3] == body[3],
              "selected column origin/width or authored height changed")
        target = body[3] if page["balance"] is None else page["balance"]["selected_height"]
        check(bounds[3] == target, "selected effective column height differs from target")
        range_start, range_end = column["fragment_range"]
        check(range_start == next_fragment and range_start <= range_end <= len(fragments),
              "physical column fragment ranges have a gap/overlap")
        next_fragment = range_end
        for position in range(range_start, range_end):
            fragment = fragments[position]
            check(fragment["column"] == index and fragment["definition"] is None,
                  "body fragment is outside its actual column range")
        used, preceding = 0, None
        parts = normalize_parts(column["parts"], tables)
        for part_index, part in enumerate(parts):
            if part["table"] is not None:
                table, leaves = validate_table_event(part["table"], part, tables, len(items))
                gap = 0 if preceding is None else preceding + table["before"]
                check(part["top"] == used + gap, "selected table origin differs from original outside spacing")
                check(part["table"]["available"] <= bounds[3] - part["top"], "selected table exceeds actual effective frame capacity")
                used = part["top"] + part["height"]
                preceding = table["after"]
                regions.append(dict(column=index, bounds=[bounds[0], bounds[1] + part["top"], bounds[2], part["height"]],
                                    leaves=leaves, semantic={i for first, last in part["table"]["semantic_ranges"] for i in range(first, last)},
                                    table=table, event=part["table"]))
                positions = {}
                for cell in part["table"]["cells"]:
                    content = table["cells"][cell["index"]]["content"]
                    for child in content[cell["content"][0]:cell["content"][1]]:
                        if child["source"]["kind"] == "item":
                            positions[child["source"]["index"]] = bounds[1] + part["top"] + cell["top"] + child["top"] - cell["offset_before"]
                regions[-1]["positions"] = positions
                continue
            check(isinstance(part["items"], list) and len(part["items"]) == 2,
                  "ordinary selected part has no original range")
            first, last = part["items"]
            height = ordinary_part_extent(items, first, last)
            forced = items[first]["source"] is None
            gap = 0 if preceding is None or forced else preceding + items[first]["before"]
            check(part["top"] == used + gap and part["height"] == height,
                  "selected part occupancy differs from original metrics/spacing")
            local = 0
            for at in range(first, last):
                item = items[at]
                if at != first:
                    local += items[at - 1]["after"] + item["before"]
                if item["source"] is not None:
                    check(at not in expected, "selected ordinary source item appears twice on page")
                    expected[at] = dict(column=index,
                        bounds=[item["x"] + bounds[0] - measured_body[0],
                                bounds[1] + part["top"] + local + item["leading"],
                                item["width"], item["height"]])
                local += item_extent(item)
            check(forced or not items[last - 1]["keep"] or last == len(items)
                  or (part_index + 1 < len(parts) and parts[part_index + 1]["table"] is not None),
                  "selected column ends at an illegal hard-keep cut")
            used = part["top"] + part["height"]
            preceding = items[last - 1]["after"]
        check(column["used_height"] == used and 0 <= used <= bounds[3],
              "selected used height differs from actual occupancy or exceeds target")
    check(all(f["definition"] is not None and f["column"] is None
              for f in fragments[next_fragment:]), "unowned fragment after body columns")
    return expected, regions


def validate_fragment_coverage(probe, streams, nodes, tables):
    body_origin = rect(probe["source_body_bounds"], "measured source body")
    note_origin = probe["source_footnote_bounds"]
    if note_origin is not None:
        note_origin = rect(note_origin, "measured source footnote")
    counts = Counter()
    semantic = repeated = headers = semantic_math = repeated_math = 0
    for page in probe["pages"]:
        expected, regions = validate_selected_columns(page, streams[0], body_origin, tables)
        for index, fragment in enumerate(page["fragments"]):
            check(fragment["index"] == index, "physical fragment ordinal mismatch")
            definition = fragment["definition"]
            stream = 0 if definition is None else integer(definition, "fragment definition", 0, len(streams) - 2) + 1
            header = fragment["header"]
            if header is None:
                original_item = fragment["item"]
            else:
                integer(header["global_item_index"], "actual header graph item")
                source = fragment["source"]
                owned = [(i, item) for i, item in enumerate(streams[stream]) if item["owner"] == fragment["owner"]
                         and item["source"] is not None and item["source"]["kind"] == source["kind"]]
                if source["kind"] == "paragraph":
                    owned = [(i, item) for i, item in owned if item["source"]["paragraph_index"] == source["paragraph_index"]]
                else:
                    owned = [(i, item) for i, item in owned if item["source"] == source]
                check(owned, "actual header graph has no original source owner/kind")
                original_item = owned[0][0]
            item_index = integer(original_item, "fragment original item", 0, len(streams[stream]) - 1)
            item = streams[stream][item_index]
            check(item["source"] is not None and fragment["owner"] == item["owner"],
                  "fragment paints a forced item or has a foreign original owner")
            check(fragment["owner"] in nodes, "fragment owner absent from authored document")
            is_repeated = boolean(fragment["repeated"], "fragment repeated")
            if header is not None:
                check(is_repeated, "header width variant is not an artifact")
                fingerprint(header["fingerprint"], "header source fingerprint")
                fingerprint(header["lines_fingerprint"], "header graph fingerprint")
                headers += 1
            if header is None:
                check(source_token(fragment["source"]) == source_token(item["source"]),
                      "fragment source units/metrics differ from original source")
            else:
                source = fragment["source"]
                check(source["kind"] == item["source"]["kind"], "header artifact changes original source kind")
                if source["kind"] == "paragraph":
                    original_lines = [i for i in streams[stream] if i["owner"] == fragment["owner"] and i["source"] is not None
                                      and i["source"]["kind"] == "paragraph" and i["source"]["paragraph_index"] == source["paragraph_index"]]
                    check(original_lines, "header actual graph has no original paragraph owner")
                    first, last = source["units"]
                    check(min(i["source"]["units"][0] for i in original_lines) <= first <= last
                          <= max(i["source"]["units"][1] for i in original_lines), "header actual units exceed original paragraph ownership")
                else:
                    check(source == item["source"], "header block changes original source asset")
            bounds = rect(fragment["bounds"], "physical fragment bounds")
            if header is None:
                check(bounds[2:] == [item["width"], item["height"]], "fragment width/height differs from original source metrics")
            elif fragment["source"]["kind"] == "paragraph":
                check(bounds[3] == fragment["source"]["line_height"], "header fragment height differs from actual selected graph")
            if definition is None:
                if item_index in expected:
                    check(bounds == expected[item_index]["bounds"] and fragment["column"] == expected[item_index]["column"],
                          "ordinary fragment origin differs from actual column/source occupancy")
                else:
                    candidates = [r for r in regions if r["column"] == fragment["column"] and item_index in r["leaves"]]
                    check(candidates, "table fragment has no selected original table source region")
                    check(any(r["bounds"][1] <= bounds[1] and bounds[1] + bounds[3] <= r["bounds"][1] + r["bounds"][3]
                              for r in candidates), "table fragment escapes selected part vertical occupancy")
                    column_bounds = page["columns"][fragment["column"]]["bounds"]
                    check(column_bounds[0] <= bounds[0] and bounds[0] + bounds[2] <= column_bounds[0] + column_bounds[2],
                          "table fragment escapes actual physical column width")
                    if not is_repeated:
                        check(any(item_index in r["semantic"] for r in candidates), "table fragment paints an unselected semantic source leaf")
                    if header is None:
                        check(bounds[0] == item["x"] + column_bounds[0] - body_origin[0], "table body fragment has a foreign physical x origin")
                        positions = [r["positions"][item_index] for r in candidates if item_index in r["positions"]]
                        if positions:
                            check(bounds[1] in positions, "table cell fragment y differs from actual cell/source offsets")
                    elif any(r["table"]["index"] == header["table_index"] for r in candidates):
                        check(any(r["event"]["header_variant"] == fingerprint(header["fingerprint"], "actual header source")
                                  for r in candidates if r["table"]["index"] == header["table_index"]),
                              "header fragment uses an unselected actual width/source variant")
            else:
                check(fragment["column"] is None and note_origin is not None,
                      "definition fragment belongs to a body column")
                note = page["placed_footnotes"]
                check(note is not None, "definition fragment has no physical footnote region")
                note = rect(note, "placed footnote bounds")
                check(bounds[0] == item["x"] + note[0] - note_origin[0],
                      "definition fragment x differs from full-width note origin")
                check(note[0] <= bounds[0] and note[1] <= bounds[1]
                      and bounds[0] + bounds[2] <= note[0] + note[2]
                      and bounds[1] + bounds[3] <= note[1] + note[3],
                      "definition fragment escapes its physical note region")
            source = fragment["source"]
            if source["kind"] == "paragraph":
                check(fragment["baseline"] == bounds[1] + source["line_baseline"],
                      "paragraph baseline differs from original local baseline plus actual origin")
                check(fragment["viewport"] is None, "paragraph has a block viewport")
            elif fragment["viewport"] is not None:
                rect(fragment["viewport"], "physical viewport")
            if is_repeated:
                repeated += 1
            else:
                counts[stream, item_index] += 1
                semantic += 1
            for math in fragment["math"]:
                check(math["owner"] in nodes and math["kind"] in ("native", "vector"),
                      "math terminal has a foreign source owner/kind")
                check(math["owner"] in source_nodes(nodes[fragment["owner"]]), "math source belongs to a foreign semantic fragment owner")
                if "fingerprint" in math:
                    fingerprint(math["fingerprint"], "original math source fingerprint")
                if is_repeated:
                    repeated_math += 1
                else:
                    semantic_math += 1
    unreferenced = 0
    for stream_index, items in enumerate(streams):
        status = "Complete" if stream_index == 0 else probe["source_items"]["definitions"][stream_index - 1]["status"]
        complete = status == "Complete" or status.startswith("Complete {")
        unused = status == "Unreferenced"
        check(complete or unused, "final source demand is still pending or unknown")
        unreferenced += int(unused)
        for index, item in enumerate(items):
            expected = 1 if complete and item["source"] is not None else 0
            check(counts[stream_index, index] == expected,
                  "semantic original source has duplicate/missing/unreferenced consumption")
    actual = dict(semantic_fragments=semantic, repeated_fragments=repeated,
                  header_variant_fragments=headers, unreferenced_definitions=unreferenced,
                  semantic_math=semantic_math, repeated_math=repeated_math)
    check(probe["source_coverage"] == actual, "source closure counters differ from actual physical sources")
    return semantic, repeated


def verify(probe):
    check(probe["schema"] == "typaxis.book-2-column-balance-probe/1", "unknown probe schema")
    check(isinstance(probe["case"], str) and probe["case"], "missing actual fixture case")
    nodes = source_nodes(probe["authored_document"])
    limits_hash = validate_limits(probe)
    body = normalize_items(probe["source_items"]["body"], "body")
    definitions = probe["source_items"]["definitions"]
    check(len(definitions) == len(probe["authored_document"]["footnotes"]), "definition source registry length differs from authored source")
    streams = [body]
    for index, definition in enumerate(definitions):
        check(definition["index"] == index, "definition ordinal mismatch")
        streams.append(normalize_items(definition["items"], "definition"))
    validate_source_lines(probe, streams, nodes)
    tables = validate_tables(probe, streams, nodes)
    pages = probe["pages"]
    check(pages, "no actual physical pages")
    maximum = integer(probe["caps"]["max_column_balance_candidates"], "effective candidate cap", 1, 65535)
    eligible = None
    for index, page in enumerate(pages):
        check(page["index"] == index, "physical page ordinal mismatch")
        boolean(page["requires_balance"], "page balance policy")
        validate_master(page, probe["authored_page_masters"])
        occupied = sum(max(0, column["used_height"]) for column in page["columns"])
        if page["balance"] is not None:
            occupied = max(occupied, decode_input(binary(page["balance"]["input_canonical_bytes"], "eligibility baseline"))["extent"])
        if occupied > 0 or any(f["definition"] is not None for f in page["fragments"]):
            eligible = index
    total_candidates = receipts = table_events = 0
    previous_item = 0
    for index, page in enumerate(pages):
        body_bounds = rect(page["body_bounds"], "authored page body")
        next_item = integer(page["next_item"], "next body source item", previous_item, len(body))
        positive = sum(max(0, column["used_height"]) for column in page["columns"]) > 0
        required = index == eligible and page["requires_balance"] and positive
        balance = page["balance"]
        check(not required or balance is not None, "positive eligible page lacks a balance receipt")
        check(balance is None or (index == eligible and page["requires_balance"]),
              "balance receipt is on an earlier, blank or notes-only page")
        if balance is not None:
            receipts += 1
            canonical = binary(balance["canonical_bytes"], "canonical receipt")
            original_bytes = binary(balance["input_canonical_bytes"], "canonical input")
            decoded = decode_receipt(canonical)
            original = decode_input(original_bytes)
            check(sha256(canonical) == fingerprint(balance["fingerprint"], "receipt fingerprint"), "receipt SHA-256 mismatch")
            check(sha256(original_bytes) == decoded["input"] == fingerprint(balance["input_fingerprint"], "input fingerprint"), "retained original input SHA-256 mismatch")
            check(original["limits"] == limits_hash, "balance input belongs to foreign effective limits")
            check(original["measurements"] == fingerprint(probe["source_measurements_fingerprint"], "original measurement fingerprint")
                  and original["lines"] == fingerprint(probe["source_line_fingerprint"], "original line graph fingerprint")
                  and original["package"] == fingerprint(probe["source_package_fingerprint"], "original package fingerprint"),
                  "balance input belongs to a foreign measured source graph")
            check(original["body"] == body_bounds and original["footnote"] == page["declared_footnotes"], "balance input changed authored body/note regions")
            check(original["entry"]["page"] == index and original["entry"]["item"] == previous_item
                  and original["next"]["item"] == next_item and original["next"]["table"] == page["next_table"]
                  and original["next"]["pending"] == page["pending_definitions"],
                  "balance input changed original source/demand endpoint")
            check([c["bounds"] for c in original["columns"]] == [c["original_bounds"] for c in page["columns"]], "balance input original column frames differ from actual authored frames")
            validate_demand_states(original, probe, nodes, streams)
            extent = validate_original_occupancy(original, body, tables)
            first = validate_trace(decoded, original, maximum)
            check(balance["extent"] == extent and balance["full_height"] == body_bounds[3]
                  and balance["initial_height"] == first and balance["selected_height"] == decoded["selected"], "reported E/H/ceil/selected differs from independent decode")
            check(normalize_reported_candidates(balance["candidates"]) == json_candidates(decoded), "reported candidate fields differ from canonical receipt")
            ordinary = all(p["table"] is None for c in original["columns"] for p in c["parts"])
            for candidate, reported in zip(decoded["candidates"][:-1], balance["candidates"][:-1]):
                validate_capacity_prefix(candidate, original, probe, tables, body, ordinary)
                for rejection, raw in zip(candidate["rejections"], reported["rejections"]):
                    table_events += validate_table_certificates(raw, rejection, original, probe, tables, body)
                    if not ordinary and rejection["table_group"] is None:
                        group = ordinary_group(body, rejection["start"], len(body))
                        check(group["end"] == rejection["end"] and group["owner"] == rejection["owner"]
                              and rejection["required"] >= group["extent"], "mixed ordinary blocker differs from its original legal source group")
                if ordinary:
                    expected = ordinary_rejections(original, body, candidate["target"])
                    check(candidate["rejections"] == expected, "capacity blockers differ from actual original hard-keep groups")
            total_candidates += len(decoded["candidates"])
        previous_item = next_item
    check(previous_item == len(body) and not pages[-1]["pending_definitions"], "physical sequence does not consume its original terminal endpoint")
    semantic, repeated = validate_fragment_coverage(probe, streams, nodes, tables)
    observed = probe["observed"]
    check(observed["scope"] == "callback_search", "unknown cumulative observation scope")
    observed_candidates = integer(observed["balance_candidates"], "observed candidates", total_candidates, maximum)
    records = integer(observed["record_charge"], "observed records", 0, probe["caps"]["max_fragments"])
    spool = integer(observed["spool_charge"], "observed spool", 0, probe["caps"]["max_spool_bytes"])
    integer(observed["work_steps"], "observed work")
    check(not receipts or (records > 0 and spool > 0 and observed_candidates > 0), "balance work/candidate/spool observations are missing")
    return dict(pages=len(pages), receipts=receipts, candidates=total_candidates,
                semantic=semantic, repeated=repeated, table_events=table_events)


def refresh_receipt(balance):
    """Reseal a mutated report so self-tests also exercise semantic checks."""
    candidates = normalize_reported_candidates(balance["candidates"])
    for raw, decoded in zip(balance["candidates"], candidates):
        calculated = ZERO_HASH if decoded["terminal"] else sha256(encode_rejections(decoded["rejections"]))
        decoded["rejection_fingerprint"] = calculated
        raw["rejection_fingerprint"] = calculated.hex()
    decoded = dict(algorithm=ALGORITHM_HASH,
                   input=fingerprint(balance["input_fingerprint"], "input fingerprint"),
                   selected=balance["selected_height"], candidates=candidates)
    data = encode_receipt(decoded)
    balance["canonical_bytes"] = list(data)
    balance["fingerprint"] = sha256(data).hex()


def tamper_checks(probe):
    mutations = []

    def add(name, mutate):
        mutations.append((name, mutate))

    if probe["source_items"]["body"]:
        add("source owner", lambda p: p["source_items"]["body"][0].update(owner=(1 << 32) - 1))
    add("cumulative candidate cap", lambda p: p["caps"].update(max_column_balance_candidates=0))
    add("cumulative observation", lambda p: p["observed"].update(balance_candidates=p["caps"]["max_column_balance_candidates"] + 1))
    add("source coverage count", lambda p: p["source_coverage"].update(semantic_fragments=p["source_coverage"]["semantic_fragments"] + 1))
    fragment_page = next((i for i, page in enumerate(probe["pages"]) if page["fragments"]), None)
    if fragment_page is not None:
        add("missing semantic source", lambda p: p["pages"][fragment_page]["fragments"].pop())
        add("physical fragment owner", lambda p: p["pages"][fragment_page]["fragments"][0].update(owner=(1 << 32) - 1))
        add("actual physical origin", lambda p: p["pages"][fragment_page]["fragments"][0]["bounds"].__setitem__(1, p["pages"][fragment_page]["fragments"][0]["bounds"][1] + 1))
        paragraph_fragment = next((i for i, f in enumerate(probe["pages"][fragment_page]["fragments"]) if f["source"]["kind"] == "paragraph"), None)
        if paragraph_fragment is not None:
            add("actual baseline", lambda p: p["pages"][fragment_page]["fragments"][paragraph_fragment].update(baseline=p["pages"][fragment_page]["fragments"][paragraph_fragment]["baseline"] + 1))
            add("actual source units", lambda p: p["pages"][fragment_page]["fragments"][paragraph_fragment]["source"]["units"].__setitem__(1, p["pages"][fragment_page]["fragments"][paragraph_fragment]["source"]["units"][1] + 1))
    balanced = next((i for i, page in enumerate(probe["pages"]) if page["balance"] is not None), None)
    if balanced is not None:
        add("source graph fingerprint", lambda p: p.update(source_line_fingerprint="00" * 32))
        add("source measurement fingerprint", lambda p: p.update(source_measurements_fingerprint="00" * 32))
        add("source package fingerprint", lambda p: p.update(source_package_fingerprint="00" * 32))
        def change_balance(p, callback):
            value = p["pages"][balanced]["balance"]
            callback(value)
            refresh_receipt(value)

        add("missing eligible receipt", lambda p: p["pages"][balanced].update(balance=None))
        add("initial original extent", lambda p: p["pages"][balanced]["balance"].update(extent=p["pages"][balanced]["balance"]["extent"] + 1))
        add("initial ceil", lambda p: p["pages"][balanced]["balance"].update(initial_height=p["pages"][balanced]["balance"]["initial_height"] + 1))
        add("original full height", lambda p: p["pages"][balanced]["balance"].update(full_height=p["pages"][balanced]["balance"]["full_height"] + 1))
        add("selected height", lambda p: change_balance(p, lambda b: b.update(selected_height=b["selected_height"] + 1)))
        add("selected physical height", lambda p: p["pages"][balanced]["columns"][0]["bounds"].__setitem__(3, p["pages"][balanced]["columns"][0]["bounds"][3] + 1))
        add("canonical receipt bit", lambda p: p["pages"][balanced]["balance"]["canonical_bytes"].__setitem__(0, p["pages"][balanced]["balance"]["canonical_bytes"][0] ^ 1))
        add("canonical input bit", lambda p: p["pages"][balanced]["balance"]["input_canonical_bytes"].__setitem__(0, p["pages"][balanced]["balance"]["input_canonical_bytes"][0] ^ 1))
        add("canonical input truncation", lambda p: p["pages"][balanced]["balance"]["input_canonical_bytes"].pop())
        add("source input fingerprint", lambda p: change_balance(p, lambda b: b.update(input_fingerprint="00" * 32)))
        add("nonpositive target", lambda p: change_balance(p, lambda b: b["candidates"][0].update(target_height=0)))
        add("target beyond authored height", lambda p: change_balance(p, lambda b: b["candidates"][-1].update(target_height=b["full_height"] + 1)))
        add("false terminal", lambda p: change_balance(p, lambda b: b["candidates"][-1].update(terminal=False)))
        if len(probe["pages"][balanced]["balance"]["candidates"]) > 1:
            def trace_bit(p):
                candidate = p["pages"][balanced]["balance"]["candidates"][0]
                candidate["capacity_canonical_bytes"][0] ^= 1
                candidate["capacity_fingerprint"] = sha256(bytes(candidate["capacity_canonical_bytes"])).hex()
                refresh_receipt(p["pages"][balanced]["balance"])
            add("capacity prefix domain with recomputed SHA", trace_bit)
            def trace_before(p):
                candidate = p["pages"][balanced]["balance"]["candidates"][0]
                candidate["capacity_canonical_bytes"][57] ^= 1
                candidate["capacity_fingerprint"] = sha256(bytes(candidate["capacity_canonical_bytes"])).hex()
                refresh_receipt(p["pages"][balanced]["balance"])
            add("capacity prefix source before-point with recomputed SHA", trace_before)
            add("nonincreasing target", lambda p: change_balance(p, lambda b: b["candidates"][1].update(target_height=b["candidates"][0]["target_height"])))
            add("wrong blocker owner with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"][0].update(owner=(1 << 32) - 1)))
            add("wrong blocker range with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"][0]["source_items"].__setitem__(1, b["candidates"][0]["rejections"][0]["source_items"][1] + 1)))
            add("nonpositive deficit with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"][0].update(deficit=0)))
            add("wrong required height with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"][0].update(required_height=b["candidates"][0]["rejections"][0]["required_height"] + 1)))
            add("wrong blocker column with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"][0].update(column=len(p["pages"][balanced]["columns"]))))
            add("missing blocker with recomputed SHA", lambda p: change_balance(p, lambda b: b["candidates"][0]["rejections"].pop()))
            def plausible_larger_blocker(p):
                balance = p["pages"][balanced]["balance"]
                for rejection in balance["candidates"][0]["rejections"]:
                    rejection["required_height"] += 1
                    rejection["deficit"] += 1
                for candidate in balance["candidates"][1:]:
                    candidate["target_height"] += 1
                balance["selected_height"] += 1
                for column in p["pages"][balanced]["columns"]:
                    column["bounds"][3] += 1
                refresh_receipt(balance)
            add("plausible larger blocker with all hashes/targets resealed", plausible_larger_blocker)
            parsed = decode_input(binary(probe["pages"][balanced]["balance"]["input_canonical_bytes"], "input"))
            if all(part["table"] is None for column in parsed["columns"] for part in column["parts"]):
                def coupled_ordinary_mask(p):
                    balance = p["pages"][balanced]["balance"]
                    candidate = balance["candidates"][0]
                    trace = decode_capacity_trace(bytes(candidate["capacity_canonical_bytes"]))
                    column = next(c for c in trace["columns"] if c["blocked"])
                    candidate["rejections"] = [r for r in candidate["rejections"] if r["column"] != column["index"]]
                    candidate["capacity_canonical_bytes"][column["blocked_offset"]] = 0
                    candidate["capacity_fingerprint"] = sha256(bytes(candidate["capacity_canonical_bytes"])).hex()
                    refresh_receipt(balance)
                add("ordinary coupled rejection/mask deletion with all SHA resealed", coupled_ordinary_mask)
        # These alter original metrics while preserving their reported sum.
        ordinary_item = next((i for i, item in enumerate(probe["source_items"]["body"]) if item["source"] is not None), None)
        if ordinary_item is not None:
            def leading(p):
                item = p["source_items"]["body"][ordinary_item]
                item["leading"] += 1
                item["consumed"] += 1
            add("original marker leading", leading)
        original = decode_input(binary(probe["pages"][balanced]["balance"]["input_canonical_bytes"], "input"))
        internal = next((part["items"][0] + 1 for column in original["columns"] for part in column["parts"]
                         if part["items"] is not None and part["items"][1] > part["items"][0] + 1), None)
        if internal is not None:
            add("original internal spacing", lambda p: p["source_items"]["body"][internal].update(before=p["source_items"]["body"][internal]["before"] + 1))
        boundary = next((part["items"][1] - 1 for column in probe["pages"][balanced]["columns"] for part in column["parts"]
                         if part["items"] is not None and 0 < part["items"][1] < len(probe["source_items"]["body"])
                         and probe["source_items"]["body"][part["items"][1] - 1]["source"] is not None
                         and not probe["source_items"]["body"][part["items"][1] - 1]["keep"]), None)
        if boundary is not None:
            add("original hard-keep group", lambda p: p["source_items"]["body"][boundary].update(keep=True))
    for name, mutate in mutations:
        changed = copy.deepcopy(probe)
        mutate(changed)
        try:
            verify(changed)
        except (InvalidProbe, KeyError, IndexError, TypeError, ValueError, struct.error):
            continue
        raise InvalidProbe(f"tampered {name} was accepted")
    return len(mutations)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path, help="actual probe files or a column-balance directory")
    parser.add_argument("--self-test", action="store_true", help="reject mutations of each actual probe")
    args = parser.parse_args()
    paths = sorted({child for path in args.paths
                    for child in (sorted(path.glob("*.json")) if path.is_dir() else [path])})
    check(paths, "no actual column balance probes")
    totals = Counter()
    tampered = 0
    for path in paths:
        try:
            raw = path.read_bytes()
            suffix = path.stem.rsplit("-", 1)[-1]
            check(len(suffix) == 64 and sha256(raw).hex() == suffix,
                  "probe filename does not bind exact callback artifact bytes")
            probe = json.loads(raw)
            totals.update(verify(probe))
            if args.self_test:
                tampered += tamper_checks(probe)
        except (InvalidProbe, KeyError, IndexError, TypeError, ValueError, struct.error) as error:
            raise InvalidProbe(f"{path}: {error}") from error
    print(f"PASS: {len(paths)} actual layout probes, {totals['pages']} physical pages, "
          f"{totals['receipts']} balance receipts, {totals['candidates']} local candidates, "
          f"{totals['semantic']} semantic fragments, {totals['repeated']} artifacts, "
          f"{totals['table_events']} bounded table witnesses, "
          f"{tampered} tamper rejections; no balanced PDF acceptance")
    if totals["table_events"]:
        print("Table source events and prefixes are checked; whole-kernel/interval replay is outside this verifier.")


if __name__ == "__main__":
    main()


