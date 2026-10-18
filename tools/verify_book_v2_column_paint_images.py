#!/usr/bin/env python3
"""Verify column image/vector paint contributions from their original IR.

The caller verifies the sealed source/display and resource leaf evidence first.
This module independently derives physical vector operators, alpha states,
image/Form objects and occurrence placements, then replays their four counters.
IR fingerprints bind the original admitted keys and source receipts. It does
not independently implement the SVG parser or grant complete Column PDF,
marked-content, structure or whole-command memory acceptance.
"""
from __future__ import annotations

import copy
import re
from fractions import Fraction

import verify_book_v2_column_resources as resource

check = resource.check
fields = resource.fields
strict_equal = resource.strict_equal
integer = resource.integer
digest = resource.digest
domain = resource.domain
hash_bytes = resource.hash_bytes
u = resource.u
i = resource.i

VECTOR = "typaxis.book-2-vector-programs/1"
OBJECTS = "typaxis.book-2-image-objects/1"
COMMANDS = "typaxis.book-2-image-commands/1"
ONE = 65536
MAX_COORDINATE = 1_000_000 * ONE


def binary(root, descriptor):
    resource.verify_artifact_fields(descriptor)
    data = resource.artifact(root, descriptor["file"], descriptor["sha256"])
    check(len(data) == descriptor["length"], "image paint artifact exact length")
    return data


def number(raw):
    """Exact canonical decimal of a signed 16.16 scalar, without float use."""
    integer(raw, 64, True)
    whole, remainder = divmod(abs(raw), ONE)
    fraction = f"{remainder * 152_587_890_625:016d}".rstrip("0")
    return ("-" if raw < 0 else "") + str(whole) + ("." + fraction if fraction else "")


def mul(left, right):
    return integer(round(Fraction(left * right, ONE)), 64, True)


class Writer:
    """Each write is one actual bounded Encoder::extend reservation/work event."""

    def __init__(self, budget):
        self.budget = budget
        self.data = bytearray()

    def write(self, value):
        if type(value) is str:
            value = value.encode("ascii")
        check(type(value) is bytes, "physical encoder byte chunk")
        self.budget.step((len(value) + 63) // 64 + 1)
        self.data.extend(value)

    def unsigned(self, value):
        self.write(str(integer(value, 64, False)))

    def decimal(self, raw):
        # text_encoding::number / font_encoding::decimal uses one stack token.
        self.write(number(raw))

    def fixed(self, raw):
        # vector Fixed formats sign, whole, dot and bounded fraction separately.
        integer(raw, 64, True)
        if raw < 0:
            self.write("-")
        whole, remainder = divmod(abs(raw), ONE)
        self.unsigned(whole)
        if remainder:
            self.write(".")
            self.write(f"{remainder * 152_587_890_625:016d}".rstrip("0"))

    def format(self, template, *values):
        """The literal and value writes of the frozen physical vector formatter."""
        pieces = template.split("{}")
        check(len(pieces) == len(values) + 1, "bounded vector format slots")
        if pieces[0]:
            self.write(pieces[0])
        for value, literal in zip(values, pieces[1:]):
            if type(value) is tuple:
                check(value[0] == "integer", "integer vector format argument")
                self.unsigned(value[1])
            elif type(value) is str:
                self.write(value)
            else:
                self.fixed(value)
            if literal:
                self.write(literal)


def vector_transform(value):
    fields(value, ("a", "d", "e", "f"), "original vector transform")
    for key in ("a", "d"):
        check(integer(value[key], 32, True) != 0, "nonzero original vector scale")
    for key in ("e", "f"):
        check(
            abs(integer(value[key], 64, True)) <= MAX_COORDINATE,
            "bounded vector translation",
        )
    return value


def compose(left, right):
    value = dict(
        a=mul(left["a"], right["a"]),
        d=mul(left["d"], right["d"]),
        e=mul(left["a"], right["e"]) + left["e"],
        f=mul(left["d"], right["f"]) + left["f"],
    )
    return vector_transform(value)


def path(value):
    check(type(value) is list and value, "nonempty original vector path")
    current = start = None
    for segment in value:
        fields(segment, ("kind", "points"), "original path segment")
        kind = segment["kind"]
        count = {"move": 1, "line": 1, "quadratic": 2, "cubic": 3, "close": 0}
        check(kind in count, "closed original path segment kind")
        points = segment["points"]
        check(
            type(points) is list and len(points) == count[kind],
            "original path point arity",
        )
        for point in points:
            check(type(point) is list and len(point) == 2, "original vector point pair")
            check(
                all(abs(integer(n, 64, True)) <= MAX_COORDINATE for n in point),
                "original point bounds",
            )
        if kind == "move":
            current = start = points[0]
        else:
            check(current is not None, "legal vector path has current subpath")
            current = start if kind == "close" else points[-1]
    return value


def rgb(value):
    check(type(value) is list and len(value) == 3, "original RGB paint triple")
    return [integer(n, 8, False) for n in value]


def paint(value):
    check(type(value) is dict, "original vector paint object")
    kind = value.get("kind")
    check(
        kind in ("none", "current-color", "fixed-rgb8"),
        "closed original vector paint kind",
    )
    fields(
        value,
        ("kind", "rgb") if kind == "fixed-rgb8" else ("kind",),
        "original vector paint",
    )
    if kind == "fixed-rgb8":
        rgb(value["rgb"])
    return value


def stroke_style(value):
    check(value["line_cap"] in ("butt", "round", "square"), "original stroke line cap")
    check(
        value["line_join"] in ("miter", "round", "bevel"), "original stroke line join"
    )
    check(integer(value["width"], 64, True) > 0, "positive original stroke width")
    check(
        integer(value["miter_limit"], 64, True) > 0,
        "positive original stroke miter limit",
    )


def verify_ir(ir, image, fingerprint):
    version = {2: "1", 3: "2"}.get(image["key"][0])
    check(version is not None, "vector IR only for original admitted vector key")
    expected = (
        "algorithm",
        "allocation",
        "clips",
        "draws",
        "intrinsic_height",
        "intrinsic_width",
        "parser",
        "path_work",
        "root_scale",
        "view_box",
    ) + (("root_viewport_clip",) if version == "2" else ())
    fields(ir, expected, "original canonical vector IR")
    check(
        ir["algorithm"] == image["key"][3] == "typaxis.safe-vector-ir/" + version,
        "original IR algorithm identity",
    )
    check(
        ir["parser"] == image["key"][2] == "typaxis.safe-svg-parser/" + version,
        "original IR parser identity",
    )
    if version == "2":
        check(
            ir["root_viewport_clip"] == "outermost",
            "original root viewport clip semantics",
        )
    for key in ("intrinsic_width", "intrinsic_height"):
        check(integer(ir[key], 64, True) > 0, "positive original vector intrinsic size")
    check(integer(ir["root_scale"], 32, True) > 0, "positive original root scale")
    view = ir["view_box"]
    check(type(view) is list and len(view) == 4, "original viewBox quartet")
    for raw in view:
        integer(raw, 64, True)
    check(view[2] > 0 and view[3] > 0, "positive original viewBox dimensions")
    allocation = fields(
        ir["allocation"],
        (
            "algorithm",
            "charge",
            "nodes",
            "paint_or_clip_commands",
            "source_clip_id_bytes",
            "stored_segments",
        ),
        "original IR allocation metadata",
    )
    check(
        allocation["algorithm"] == "typaxis.safe-vector-allocation-charge/" + version,
        "original IR allocation algorithm",
    )
    for key in (
        "charge",
        "nodes",
        "paint_or_clip_commands",
        "source_clip_id_bytes",
        "stored_segments",
    ):
        integer(allocation[key], 64, False)
    check(
        allocation["charge"] == image["decoded_bytes"],
        "original IR admitted allocation charge",
    )
    integer(ir["path_work"], 64, False)
    check(
        type(ir["clips"]) is list and type(ir["draws"]) is list,
        "original IR clip/draw arrays",
    )
    stored = 0
    for index, clip in enumerate(ir["clips"]):
        fields(
            clip,
            ("clip_id", "fill_rule", "path", "transform"),
            "original clip definition",
        )
        check(integer(clip["clip_id"], 32, False) == index, "dense original clip IDs")
        check(clip["fill_rule"] in ("nonzero", "evenodd"), "original clip fill rule")
        stored += len(path(clip["path"]))
        vector_transform(clip["transform"])
    for draw in ir["draws"]:
        fields(
            draw,
            ("clips", "fill", "fill_rule", "path", "stroke", "transform"),
            "original vector draw",
        )
        check(draw["fill_rule"] in ("nonzero", "evenodd"), "original draw fill rule")
        stored += len(path(draw["path"]))
        vector_transform(draw["transform"])
        check(type(draw["clips"]) is list, "original draw clip uses")
        for use in draw["clips"]:
            fields(use, ("clip_id", "transform"), "original clip use")
            check(
                integer(use["clip_id"], 32, False) < len(ir["clips"]),
                "original clip definition membership",
            )
            vector_transform(use["transform"])
        if version == "2":
            fields(draw["fill"], ("alpha", "paint"), "original fill layer")
            check(
                integer(draw["fill"]["alpha"], 32, False) <= ONE,
                "original fill alpha range",
            )
            paint(draw["fill"]["paint"])
            fields(
                draw["stroke"],
                ("alpha", "line_cap", "line_join", "miter_limit", "paint", "width"),
                "original stroke layer",
            )
            check(
                integer(draw["stroke"]["alpha"], 32, False) <= ONE,
                "original stroke alpha range",
            )
            paint(draw["stroke"]["paint"])
            stroke_style(draw["stroke"])
        else:
            if draw["fill"] is not None:
                rgb(draw["fill"])
            if draw["stroke"] is not None:
                fields(
                    draw["stroke"],
                    ("color", "line_cap", "line_join", "miter_limit", "width"),
                    "original v1 stroke",
                )
                rgb(draw["stroke"]["color"])
                stroke_style(draw["stroke"])
    check(
        stored + 5 == allocation["stored_segments"],
        "original IR stored segments including synthetic outer clip",
    )
    canonical = resource.display.canonical(
        {"algorithm": "typaxis.safe-vector-ir-fingerprint/" + version, "ir": ir}
    ).encode()
    check(
        digest(canonical) == hash_bytes(fingerprint) == hash_bytes(image["key"][4]),
        "canonical IR hash binds original selected image key",
    )
    return version


def state_pairs(ir):
    if ir["algorithm"].endswith("/2"):
        return [
            (draw["fill"]["alpha"], draw["stroke"]["alpha"]) for draw in ir["draws"]
        ]
    return [(ONE, ONE) for _ in ir["draws"]]


def collect_states(ir, budget, size):
    values = state_pairs(ir)
    count = len(values)
    budget.charge(records=count, spool=count * size)
    budget.step()
    budget.step(count)

    def sift(root, end):
        while True:
            budget.step()
            child = root * 2 + 1
            if child >= end:
                return
            if child + 1 < end:
                budget.step()
                if values[child] < values[child + 1]:
                    child += 1
            budget.step()
            if values[root] >= values[child]:
                return
            budget.step()
            values[root], values[child] = values[child], values[root]
            root = child

    for root in reversed(range(count // 2)):
        sift(root, count)
    for end in reversed(range(1, count)):
        budget.step()
        values[0], values[end] = values[end], values[0]
        sift(0, end)
    retained = 0
    for index in range(count):
        budget.step()
        if retained == 0 or values[retained - 1] != values[index]:
            values[retained], values[index] = values[index], values[retained]
            retained += 1
    values = values[:retained]
    check(values == sorted(set(state_pairs(ir))), "charged alpha-state unique sort")
    return values


def encode_path(out, segments, matrix=None):
    current = start = None

    def point(value):
        if matrix is None:
            return value
        value = [
            mul(matrix["a"], value[0]) + matrix["e"],
            mul(matrix["d"], value[1]) + matrix["f"],
        ]
        check(
            all(abs(integer(n, 64, True)) <= MAX_COORDINATE for n in value),
            "bounded transformed clip point",
        )
        return value

    for segment in segments:
        kind = segment["kind"]
        values = [point(value) for value in segment["points"]]
        if kind == "close":
            out.write("h\n")
            current = start
        elif kind in ("move", "line"):
            out.format("{} {} " + ("m\n" if kind == "move" else "l\n"), *values[0])
            current = values[0]
            if kind == "move":
                start = current
        else:
            if kind == "quadratic":
                control, endpoint = values
                first = [
                    round(Fraction(a + 2 * b, 3)) for a, b in zip(current, control)
                ]
                second = [
                    round(Fraction(a + 2 * b, 3)) for a, b in zip(endpoint, control)
                ]
                values = [first, second, endpoint]
            out.format(
                "{} {} {} {} {} {} c\n", *(raw for value in values for raw in value)
            )
            current = values[-1]


def encode_rgb(out, values, operator):
    out.format(
        "{} {} {} {}\n", *(round(Fraction(n * ONE, 255)) for n in values), operator
    )


def encode_state(out, fill, stroke):
    out.write("<< /Type /ExtGState /ca ")
    out.decimal(fill)
    out.write(" /CA ")
    out.decimal(stroke)
    out.write(" >>")


def encode_program(ir, pairs, budget):
    out = Writer(budget)
    out.write("q\n0 0 ")
    out.format("{} {} re W n\n", ir["intrinsic_width"], ir["intrinsic_height"])
    scale = ir["root_scale"]
    out.format(
        "{} 0 0 {} {} {} cm\n",
        scale,
        scale,
        -mul(scale, ir["view_box"][0]),
        -mul(scale, ir["view_box"][1]),
    )
    version2 = ir["algorithm"].endswith("/2")
    for draw, pair in zip(ir["draws"], state_pairs(ir)):
        out.write("q\n")
        for use in draw["clips"]:
            clip = ir["clips"][use["clip_id"]]
            encode_path(out, clip["path"], compose(use["transform"], clip["transform"]))
            out.write("W* n\n" if clip["fill_rule"] == "evenodd" else "W n\n")
        transform = draw["transform"]
        out.format(
            "{} 0 0 {} {} {} cm\n", *(transform[k] for k in ("a", "d", "e", "f"))
        )
        budget.step(len(pairs).bit_length() + 1)
        out.write("/GS")
        out.unsigned(pairs.index(pair))
        out.write(" gs\n")
        if version2:
            fill, stroke = draw["fill"]["paint"], draw["stroke"]["paint"]
        else:
            fill = (
                {"kind": "none"}
                if draw["fill"] is None
                else {"kind": "fixed-rgb8", "rgb": draw["fill"]}
            )
            stroke = (
                {"kind": "none"}
                if draw["stroke"] is None
                else {"kind": "fixed-rgb8", "rgb": draw["stroke"]["color"]}
            )
        for value, operator in ((fill, "rg"), (stroke, "RG")):
            if value["kind"] == "fixed-rgb8":
                encode_rgb(out, value["rgb"], operator)
        if stroke["kind"] != "none":
            style = draw["stroke"]
            out.format(
                "{} w\n{} J\n{} j\n{} M\n",
                style["width"],
                ("integer", {"butt": 0, "round": 1, "square": 2}[style["line_cap"]]),
                ("integer", {"miter": 0, "round": 1, "bevel": 2}[style["line_join"]]),
                style["miter_limit"],
            )
        check(
            fill["kind"] != "none" or stroke["kind"] != "none",
            "original vector draw has enabled paint",
        )
        if fill["kind"] != "none":
            encode_path(out, draw["path"])
            out.write("f*\n" if draw["fill_rule"] == "evenodd" else "f\n")
        if stroke["kind"] != "none":
            encode_path(out, draw["path"])
            out.write("S\n")
        out.write("Q\n")
    out.write("Q")
    content = bytes(out.data)
    dictionaries = []
    ranges = []
    for fill, stroke in pairs:
        start = len(out.data)
        encode_state(out, fill, stroke)
        ranges.append([start, len(out.data)])
        dictionaries.append(bytes(out.data[start:]))
    return content, dictionaries, ranges, bytes(out.data)


def constructor(probe, budget, index):
    budget.charge(records=1)
    budget.matches4(probe["observed"]["constructors"][index])


def verify_vectors(root, probe, budget):
    stage = fields(
        probe["vector_programs"],
        ("fingerprint", "charges", "prior_output", "programs"),
        "vector program stage",
    )
    images = probe["images_selection"]["images"]
    raster = probe["rasters"]
    strict_equal(
        [budget.records, budget.spool, budget.work],
        raster["charges"],
        "actual raster prefix before vector constructor",
    )
    check(
        integer(stage["prior_output"], 64, False) == budget.output,
        "actual text output prefix before vector constructor",
    )
    constructor(probe, budget, 5)
    check(
        type(stage["programs"]) is list and len(stage["programs"]) == len(images),
        "dense raster/vector program slots",
    )
    budget.charge(
        records=len(images) + 1, spool=len(images) * probe["size_of"]["vector_slot"]
    )
    budget.step()
    fingerprint = budget.fold(domain(VECTOR), hash_bytes(raster["fingerprint"]))
    result = []
    for index, (image, program) in enumerate(zip(images, stage["programs"])):
        budget.step()
        if image["key"][0] < 2:
            check(
                program is None and raster["programs"][index] is not None,
                "original raster has no vector program",
            )
            result.append(None)
            continue
        check(
            program is not None and raster["programs"][index] is None,
            "original vector has no raster program",
        )
        fields(
            program,
            ("fingerprint", "ir", "ir_fingerprint", "bbox", "content", "states"),
            "actual vector program",
        )
        ir = program["ir"]
        verify_ir(ir, image, program["ir_fingerprint"])
        strict_equal(
            program["bbox"],
            [0, 0, ir["intrinsic_width"], ir["intrinsic_height"]],
            "original IR physical Form bbox",
        )
        pairs = collect_states(ir, budget, probe["size_of"]["vector_state"])
        first = encode_program(ir, pairs, budget)
        budget.charge(records=1, spool=len(first[3]), output=len(first[3]))
        budget.step()
        second = encode_program(ir, pairs, budget)
        check(first == second, "independent vector two-pass byte/range equality")
        content, dictionaries, ranges, data = second
        check(
            binary(root, program["content"]) == content,
            "exact original IR physical vector content",
        )
        check(
            type(program["states"]) is list and len(program["states"]) == len(pairs),
            "exact original alpha state inventory",
        )
        for value, pair, dictionary in zip(program["states"], pairs, dictionaries):
            fields(
                value,
                ("fill_alpha", "stroke_alpha", "dictionary"),
                "actual vector alpha state",
            )
            strict_equal(
                [value["fill_alpha"], value["stroke_alpha"]],
                list(pair),
                "original alpha state order",
            )
            check(
                binary(root, value["dictionary"]) == dictionary,
                "exact original alpha dictionary",
            )
        budget.step((len(data) + 63) // 64 + 1)
        fp = budget.fold(domain(VECTOR), hash_bytes(image["key"][1]))
        fp = budget.fold(fp, hash_bytes(program["ir_fingerprint"]))
        fp = budget.fold(fp, digest(data))
        for pair, extent in zip(pairs, ranges):
            fp = budget.fold(
                fp, u(pair[0], 4) + u(pair[1], 4) + u(extent[0]) + u(extent[1])
            )
        check(
            fp == hash_bytes(program["fingerprint"]),
            "vector program canonical fingerprint",
        )
        fingerprint = budget.fold(fingerprint, u(index))
        fingerprint = budget.fold(fingerprint, fp)
        result.append(
            dict(
                content=content, dictionaries=dictionaries, bbox=program["bbox"], ir=ir
            )
        )
    check(
        fingerprint == hash_bytes(stage["fingerprint"]),
        "aggregate vector canonical fingerprint",
    )
    budget.matches4(stage)
    return result


def encode_reference(out, value):
    out.unsigned(value)
    out.write(" 0 R")


def encode_stream(out, data):
    out.write(" /Length ")
    out.unsigned(len(data))
    out.write(" >>\nstream\n")
    out.write(data)
    out.write("\nendstream")


def encode_objects(probe, vectors, raster_bytes, first, budget):
    out = Writer(budget)
    objects, groups = [], []
    next_id = first
    for index, vector in enumerate(vectors):
        budget.step()
        before = len(objects)
        raster = probe["rasters"]["programs"][index]
        if vector is None:
            count = 1 + (raster["alpha"] is not None)
        else:
            count = len(vector["dictionaries"]) + 1
        for part in range(count):
            budget.step()
            start = len(out.data)
            out.unsigned(next_id)
            out.write(" 0 obj\n")
            if vector is None:
                mask = part == 1
                out.write("<<")
                out.write(" /Type /XObject /Subtype /Image /Width ")
                out.unsigned(raster["width"])
                out.write(" /Height ")
                out.unsigned(raster["height"])
                out.write(" /ColorSpace /")
                out.write(
                    "DeviceGray" if mask or raster["color"] == "Gray" else "DeviceRGB"
                )
                out.write(" /BitsPerComponent ")
                out.unsigned(8)
                if not mask and raster["encoding"] == "Jpeg":
                    check(
                        raster["alpha"] is None and raster["jpeg"] is not None,
                        "original JPEG has no alpha mask",
                    )
                    out.write(" /Filter /DCTDecode /DecodeParms << /ColorTransform ")
                    out.unsigned(0 if raster["color"] == "Gray" else 1)
                    out.write(" >>")
                else:
                    check(
                        mask or raster["encoding"] == "Flate",
                        "original normalized raster stream encoding",
                    )
                    out.write(" /Filter /FlateDecode")
                if not mask and raster["alpha"] is not None:
                    out.write(" /SMask ")
                    encode_reference(out, next_id + 1)
                encode_stream(out, raster_bytes[index][1 if mask else 0])
                role, state = ("SoftMask" if mask else "Image"), None
            elif part == 0:
                out.write("<< /Type /XObject /Subtype /Form /FormType 1 /BBox [")
                for at, raw in enumerate(vector["bbox"]):
                    if at:
                        out.write(" ")
                    out.decimal(raw)
                out.write("] /Resources << /ExtGState <<")
                for at in range(len(vector["dictionaries"])):
                    budget.step()
                    out.write(" /GS")
                    out.unsigned(at)
                    out.write(" ")
                    encode_reference(out, next_id + at + 1)
                out.write(" >> >>")
                encode_stream(out, vector["content"])
                role, state = "Form", None
            else:
                out.write(vector["dictionaries"][part - 1])
                role, state = "ExtGState", part - 1
            out.write("\nendobj\n")
            objects.append(
                dict(
                    id=next_id,
                    image=index,
                    role=role,
                    state=state,
                    range=[start, len(out.data)],
                )
            )
            next_id += 1
        groups.append([before, len(objects)])
    return bytes(out.data), objects, groups


def verify_objects(root, probe, vectors, budget):
    stage = fields(
        probe["image_objects"],
        ("fingerprint", "charges", "first", "next", "bytes", "objects", "resources"),
        "image object stage",
    )
    images = probe["images_selection"]["images"]
    count, credit, raster_bytes = 0, 0, []
    # The constructor record is accepted before its source/state counting work.
    budget.charge(records=1)
    for index, vector in enumerate(vectors):
        budget.step()
        raster = probe["rasters"]["programs"][index]
        if vector is None:
            check(
                raster["color"] in ("Gray", "Rgb"),
                "original supported raster color space",
            )
            original = binary(root, raster["program"])
            alpha = raster["alpha"]
            mask = None if alpha is None else binary(root, alpha["program"])
            if alpha is not None:
                strict_equal(
                    [alpha["width"], alpha["height"], alpha["encoding"]],
                    [raster["width"], raster["height"], "Flate"],
                    "original alpha mask geometry and encoding",
                )
            count += 1 + (alpha is not None)
            raster_bytes.append((original, mask))
        else:
            credit += len(vector["content"])
            for value in vector["dictionaries"]:
                budget.step()
                credit += len(value)
            count += len(vector["dictionaries"]) + 1
            raster_bytes.append(None)
    first = integer(stage["first"], 32, False)
    check(
        first > 0 and first + count <= 2**32 - 1,
        "bounded nonzero actual image object range",
    )
    check(
        first == probe["font_objects"]["next"],
        "image objects follow actual font object range",
    )
    source = probe["display_probe"]
    limits = resource.display.sealed_jcs(
        source["limits_canonical_jcs"],
        source["limits_fingerprint"],
        "typaxis.m4-effective-resource-limits/1",
    )["base"]
    check(
        first + count - 1 <= limits["max_pdf_objects"],
        "inclusive global image object number cap",
    )
    check(
        credit <= probe["vector_programs"]["charges"][2],
        "vector content credit was previously charged",
    )
    budget.matches4(probe["observed"]["constructors"][6])
    first_pass = encode_objects(probe, vectors, raster_bytes, first, budget)
    data, expected, groups = first_pass
    metadata = (
        count * probe["size_of"]["image_object"]
        + len(images) * probe["size_of"]["range"]
    )
    check(
        len(data) >= credit, "one-time vector payload output credit fits actual objects"
    )
    budget.charge(
        records=count + len(images) + 1,
        spool=metadata + len(data),
        output=len(data) - credit,
    )
    budget.step()
    second_pass = encode_objects(probe, vectors, raster_bytes, first, budget)
    check(first_pass == second_pass, "independent image object two-pass equality")
    strict_equal(
        stage["objects"], expected, "exact original image/Form object inventory"
    )
    strict_equal(
        stage["resources"],
        [expected[a]["id"] for a, _ in groups],
        "actual resource first-object identities",
    )
    check(stage["next"] == first + count, "actual image object next identity")
    check(
        binary(root, stage["bytes"]) == data,
        "exact original image/Form/SoftMask object bytes",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        domain(OBJECTS), hash_bytes(probe["vector_programs"]["fingerprint"])
    )
    fp = budget.fold(fp, u(first, 4))
    fp = budget.fold(fp, digest(data))
    roles = {"Image": 0, "SoftMask": 1, "Form": 2, "ExtGState": 3}
    for value in expected:
        block = (
            u(value["id"], 4)
            + u(value["image"])
            + u(value["range"][0])
            + u(value["range"][1])
            + bytes([roles[value["role"]], value["state"] is not None])
            + u(value["state"] or 0)
        )
        check(len(block) == 38, "image object canonical metadata width")
        fp = budget.fold(fp, block)
    for start, end in groups:
        fp = budget.fold(fp, u(start) + u(end))
    check(fp == hash_bytes(stage["fingerprint"]), "image objects canonical fingerprint")
    budget.matches4(stage)
    return dict(
        objects=count,
        forms=sum(vector is not None for vector in vectors),
        masks=sum(value is not None and value[1] is not None for value in raster_bytes),
        credit=credit,
    )


def actual_image_uses(probe):
    """Use the physical Column component graph, without resource-use helpers."""
    source = probe["display_probe"]
    result = []
    selected = probe["images_selection"]["images"]
    originals = probe["original_images"]
    for at, (tag, index) in enumerate(source["paints"]):
        if tag == "image":
            draw = source["images"][index]
            original, vector = draw["original"], draw["vector"]
            viewport = draw["viewport"]
            page = source["original_fragments"][draw["fragment"]]["page"]
        elif tag == "math" and source["math"][index]["original"]["kind"] == "vector":
            draw = source["math"][index]
            original, vector = draw["original"], draw["paint"]
            viewport = vector["viewport"]
            page = draw["page"]
        else:
            continue
        identity = originals[original["image"]]
        check(
            identity["id"] == original["image"]
            and identity["sha256"] == original["source_sha256"],
            "actual image occurrence original admitted identity",
        )
        matches = [
            n for n, image in enumerate(selected) if image["key"] == identity["key"]
        ]
        check(len(matches) == 1, "actual occurrence belongs to one selected payload")
        if vector is None:
            geometry = dict(kind="raster", viewport=viewport)
        else:
            key = identity["key"]
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
        result.append(
            dict(
                paint=at,
                typed_paint=[tag, index],
                resource=matches[0],
                image_id=original["image"],
                owner=original["owner"],
                page=page,
                geometry=geometry,
            )
        )
    strict_equal(
        probe["images_selection"]["uses"],
        result,
        "complete authored-order actual Column image occurrences",
    )
    return result


def encode_placement(out, usage):
    geometry = usage["geometry"]
    out.write("q\n")
    if geometry["kind"] == "raster":
        x, y, width, height = geometry["viewport"]
        out.decimal(width)
        out.write(" 0 0 ")
        out.decimal(-height)
        out.write(" ")
        out.decimal(x)
        out.write(" ")
        out.decimal(y + height)
        out.write(" cm\n/")
    else:
        for operator in ("rg", "RG"):
            encode_rgb(out, geometry["color"], operator)
        for at, raw in enumerate(geometry["matrix"]):
            if at:
                out.write(" ")
            out.fixed(raw)
        out.write(" cm\n/")
    out.write("BI")
    out.unsigned(usage["resource"])
    # Vector placement emits the final newline in the caller; raster emits it.
    out.write(" Do\nQ" + ("\n" if geometry["kind"] == "raster" else ""))
    if geometry["kind"] == "vector":
        out.write("\n")


def encode_commands(probe, uses, budget):
    out = Writer(budget)
    commands = []
    for usage in uses:
        budget.step()
        start = len(out.data)
        encode_placement(out, usage)
        commands.append(
            dict(
                paint=usage["paint"],
                typed_paint=usage["typed_paint"],
                resource=usage["resource"],
                object=probe["image_objects"]["resources"][usage["resource"]],
                page=usage["page"],
                owner=usage["owner"],
                range=[start, len(out.data)],
            )
        )
    return bytes(out.data), commands


def verify_commands(root, probe, budget):
    stage = fields(
        probe["image_commands"],
        ("fingerprint", "charges", "bytes", "commands"),
        "image command stage",
    )
    constructor(probe, budget, 7)
    uses = actual_image_uses(probe)
    first = encode_commands(probe, uses, budget)
    data, commands = first
    budget.charge(
        records=len(uses) + 1,
        spool=len(uses) * probe["size_of"]["image_command"] + len(data),
        output=len(data),
    )
    budget.step()
    second = encode_commands(probe, uses, budget)
    check(first == second, "independent image command two-pass equality")
    strict_equal(
        stage["commands"], commands, "exact actual Column occurrence command inventory"
    )
    check(
        binary(root, stage["bytes"]) == data,
        "exact actual Column raster/vector placement bytes",
    )
    budget.step((len(data) + 63) // 64 + 1)
    fp = budget.fold(
        domain(COMMANDS), hash_bytes(probe["image_objects"]["fingerprint"])
    )
    fp = budget.fold(fp, digest(data))
    for value in commands:
        block = (
            u(value["paint"])
            + u(value["resource"])
            + u(value["object"], 4)
            + u(value["page"], 4)
            + u(value["owner"], 4)
            + u(value["range"][0])
            + u(value["range"][1])
        )
        check(len(block) == 44, "image command canonical metadata width")
        fp = budget.fold(fp, block)
    check(
        fp == hash_bytes(stage["fingerprint"]), "image commands canonical fingerprint"
    )
    budget.matches4(stage)
    return len(commands)


def verify_image_paints(root, probe, budget):
    """Consume actual raster/text prefixes and replay the three image stages."""
    sizes = probe["size_of"]
    for key in (
        "vector_slot",
        "vector_state",
        "image_object",
        "image_command",
        "range",
    ):
        check(integer(sizes[key], 64, False) > 0, "actual image paint reservation unit")
    constructors = probe["observed"]["constructors"]
    check(
        type(constructors) is list and len(constructors) == 8,
        "complete paint constructor prefix chain",
    )
    for value in constructors:
        check(
            type(value) is list and len(value) == 4,
            "constructor record/spool/output/work tuple",
        )
        for raw in value:
            integer(raw, 64, False)
    vectors = verify_vectors(root, probe, budget)
    counts = verify_objects(root, probe, vectors, budget)
    counts["image_commands"] = verify_commands(root, probe, budget)
    counts["vector_draws"] = sum(
        len(vector["ir"]["draws"]) for vector in vectors if vector is not None
    )
    counts["vector_states"] = sum(
        len(vector["dictionaries"]) for vector in vectors if vector is not None
    )
    return counts


def reseal_image_paints(root, probe):
    """Rehash *reported* facts for tamper trials, without certifying them.

    This deliberately uses the mutated artifact bytes and metadata. Successful
    rejection therefore proves more than refusal of a stale checksum. It never
    updates the original source/display receipts or resource selection keys.
    """
    fold = resource.fold
    stage = probe["vector_programs"]
    fp = fold(domain(VECTOR), hash_bytes(probe["rasters"]["fingerprint"]))
    for index, value in enumerate(stage["programs"]):
        if value is None:
            continue
        content = binary(root, value["content"])
        data = bytearray(content)
        ranges = []
        for state in value["states"]:
            start = len(data)
            data.extend(binary(root, state["dictionary"]))
            ranges.append([start, len(data)])
        leaf = fold(
            domain(VECTOR),
            hash_bytes(probe["images_selection"]["images"][index]["key"][1]),
        )
        leaf = fold(leaf, hash_bytes(value["ir_fingerprint"]))
        leaf = fold(leaf, digest(data))
        for state, extent in zip(value["states"], ranges):
            leaf = fold(
                leaf,
                u(state["fill_alpha"], 4)
                + u(state["stroke_alpha"], 4)
                + u(extent[0])
                + u(extent[1]),
            )
        value["fingerprint"] = leaf.hex()
        fp = fold(fold(fp, u(index)), leaf)
    stage["fingerprint"] = fp.hex()
    stage = probe["image_objects"]
    fp = fold(domain(OBJECTS), hash_bytes(probe["vector_programs"]["fingerprint"]))
    fp = fold(fp, u(stage["first"], 4))
    fp = fold(fp, digest(binary(root, stage["bytes"])))
    roles = {"Image": 0, "SoftMask": 1, "Form": 2, "ExtGState": 3}
    for value in stage["objects"]:
        fp = fold(
            fp,
            u(value["id"], 4)
            + u(value["image"])
            + u(value["range"][0])
            + u(value["range"][1])
            + bytes([roles[value["role"]], value["state"] is not None])
            + u(value["state"] or 0),
        )
    cursor = 0
    for index in range(len(probe["images_selection"]["images"])):
        next_cursor = cursor + sum(
            value["image"] == index for value in stage["objects"]
        )
        fp = fold(fp, u(cursor) + u(next_cursor))
        cursor = next_cursor
    stage["fingerprint"] = fp.hex()
    stage = probe["image_commands"]
    fp = fold(domain(COMMANDS), hash_bytes(probe["image_objects"]["fingerprint"]))
    fp = fold(fp, digest(binary(root, stage["bytes"])))
    for value in stage["commands"]:
        fp = fold(
            fp,
            u(value["paint"])
            + u(value["resource"])
            + u(value["object"], 4)
            + u(value["page"], 4)
            + u(value["owner"], 4)
            + u(value["range"][0])
            + u(value["range"][1]),
        )
    stage["fingerprint"] = fp.hex()
    return probe


def image_tamper_cases(probe):
    """Typed metadata, capacity, owner and source substitutions for the caller."""
    cases = []

    def change(label, route, value):
        bad = copy.deepcopy(probe)
        target = bad
        for key in route[:-1]:
            target = target[key]
        target[route[-1]] = value
        cases.append((label, bad))

    for stage in ("vector_programs", "image_objects", "image_commands"):
        for index, counter in enumerate(("records", "spool", "output", "work")):
            change(
                stage + " wrong " + counter,
                [stage, "charges", index],
                probe[stage]["charges"][index] + 1,
            )
        missing = copy.deepcopy(probe)
        del missing[stage]["fingerprint"]
        cases.append((stage + " missing fingerprint", missing))
    for at in (5, 6, 7):
        for index, counter in enumerate(("records", "spool", "output", "work")):
            change(
                "image constructor wrong " + counter,
                ["observed", "constructors", at, index],
                probe["observed"]["constructors"][at][index] + 1,
            )
    change(
        "vector lost text output prefix",
        ["vector_programs", "prior_output"],
        probe["vector_programs"]["prior_output"] + 1,
    )
    for index, value in enumerate(probe["vector_programs"]["programs"]):
        if value is None:
            continue
        change(
            "vector wrong physical bbox",
            ["vector_programs", "programs", index, "bbox", 2],
            value["bbox"][2] + 1,
        )
        change(
            "vector foreign original IR fingerprint",
            ["vector_programs", "programs", index, "ir_fingerprint"],
            bytes(32).hex(),
        )
        if value["states"]:
            state = value["states"][0]
            change(
                "vector alpha metadata substitution",
                ["vector_programs", "programs", index, "states", 0, "fill_alpha"],
                (state["fill_alpha"] + 1) % (ONE + 1),
            )
            if len(value["states"]) > 1:
                bad = copy.deepcopy(probe)
                bad["vector_programs"]["programs"][index]["states"].reverse()
                cases.append(("vector state order substitution", bad))
        if value["ir"]["draws"]:
            bad = copy.deepcopy(probe)
            leaf = bad["vector_programs"]["programs"][index]
            leaf["ir"]["draws"][0]["path"][0]["points"][0][0] += 1
            version = leaf["ir"]["algorithm"].rsplit("/", 1)[1]
            leaf["ir_fingerprint"] = digest(
                resource.display.canonical(
                    {
                        "algorithm": "typaxis.safe-vector-ir-fingerprint/" + version,
                        "ir": leaf["ir"],
                    }
                ).encode()
            ).hex()
            cases.append(("vector modified path and resealed IR hash", bad))
        break
    stage = probe["image_objects"]
    change(
        "image objects overlap original font range",
        ["image_objects", "first"],
        stage["first"] + 1,
    )
    change(
        "image objects wrong next identity",
        ["image_objects", "next"],
        stage["next"] + 1,
    )
    if stage["objects"]:
        value = stage["objects"][0]
        change(
            "image object foreign payload owner",
            ["image_objects", "objects", 0, "image"],
            value["image"] + 1,
        )
        change(
            "image object wrong global identity",
            ["image_objects", "objects", 0, "id"],
            value["id"] + 1,
        )
        change(
            "image object wrong byte range",
            ["image_objects", "objects", 0, "range", 1],
            value["range"][1] - 1,
        )
        change(
            "image object wrong role",
            ["image_objects", "objects", 0, "role"],
            "ExtGState" if value["role"] != "ExtGState" else "Image",
        )
        change(
            "image resource wrong object reference",
            ["image_objects", "resources", 0],
            stage["resources"][0] + 1,
        )
    if probe["image_commands"]["commands"]:
        value = probe["image_commands"]["commands"][0]
        for name in ("owner", "page", "paint", "resource", "object"):
            change(
                "image command foreign " + name,
                ["image_commands", "commands", 0, name],
                value[name] + 1,
            )
        change(
            "image command foreign typed paint",
            ["image_commands", "commands", 0, "typed_paint"],
            ["image", 2**32 - 1],
        )
        bad = copy.deepcopy(probe)
        bad["image_commands"]["commands"].pop()
        cases.append(("missing original image occurrence command", bad))
    return cases


def image_artifact_tamper_cases(probe):
    """Actual bytes to alter; the main checker writes and rehashes each copy."""
    cases = []
    for index, value in enumerate(probe["vector_programs"]["programs"]):
        if value is None:
            continue
        cases.append(
            (
                "vector changed viewport clip semantics",
                ["vector_programs", "programs", index, "content"],
                lambda b: b.replace(b" re W n\n", b" re W* n\n", 1),
            )
        )
        cases.append(
            (
                "vector changed alpha state reference",
                ["vector_programs", "programs", index, "content"],
                lambda b: b.replace(b"/GS0 gs", b"/GS999 gs", 1),
            )
        )
        if value["states"]:
            cases.append(
                (
                    "vector changed actual alpha dictionary",
                    ["vector_programs", "programs", index, "states", 0, "dictionary"],
                    lambda b: b.replace(b" /ca ", b" /CA ", 1),
                )
            )
        break
    if probe["image_objects"]["objects"]:
        cases.append(
            (
                "image changed declared stream length",
                ["image_objects", "bytes"],
                lambda b: re.sub(
                    rb"/Length (\d+)",
                    lambda m: b"/Length " + str(int(m[1]) + 1).encode(),
                    b,
                    count=1,
                ),
            )
        )
        cases.append(
            (
                "image changed object dictionary identity",
                ["image_objects", "bytes"],
                lambda b: b.replace(b"/Type /XObject", b"/Type /Wrong", 1),
            )
        )
        if any(
            value["role"] == "SoftMask" for value in probe["image_objects"]["objects"]
        ):
            cases.append(
                (
                    "image changed alpha object reference",
                    ["image_objects", "bytes"],
                    lambda b: re.sub(
                        rb"/SMask (\d+)",
                        lambda m: b"/SMask " + str(int(m[1]) + 1).encode(),
                        b,
                        count=1,
                    ),
                )
            )
        if any(value["role"] == "Form" for value in probe["image_objects"]["objects"]):
            cases.append(
                (
                    "Form changed alpha state object reference",
                    ["image_objects", "bytes"],
                    lambda b: re.sub(
                        rb"/GS0 (\d+)",
                        lambda m: b"/GS0 " + str(int(m[1]) + 1).encode(),
                        b,
                        count=1,
                    ),
                )
            )
    if probe["image_commands"]["commands"]:
        cases.append(
            (
                "image changed occurrence Do payload",
                ["image_commands", "bytes"],
                lambda b: re.sub(rb"/BI\d+ Do", b"/BI999 Do", b, count=1),
            )
        )
        cases.append(
            (
                "image inserted unowned marked-content scope",
                ["image_commands", "bytes"],
                lambda b: b.replace(b"q\n", b"q\n/MCID 0 BDC\n", 1),
            )
        )
        cases.append(
            (
                "image changed actual physical matrix",
                ["image_commands", "bytes"],
                lambda b: b.replace(b" cm\n", b" 1 0 0 1 0 0 cm\n", 1),
            )
        )
    return cases
