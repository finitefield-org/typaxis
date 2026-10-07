#!/usr/bin/env python3
"""Check the fixed ABAB selected-body number probe, not public/full-book acceptance."""
from pathlib import Path
import argparse
import json
import subprocess
from PIL import Image, ImageChops
from pypdf import PdfReader, PdfWriter
from pypdf.generic import ContentStream, NameObject, DecodedStreamObject


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def verify(pdf, output):
    reader = PdfReader(pdf, strict=True)
    require(len(reader.pages) == 1, "expected one diagnostic page")
    operations = ContentStream(reader.pages[0].get_contents(), reader).operations
    stack, groups = [], []
    current = None
    for index, (args, op) in enumerate(operations):
        if op == b"BDC":
            props = args[1]
            outer = "/MCID" in props
            if outer:
                require(current is None, "nested MCID")
                current = {"mcid": int(props["/MCID"]), "role": str(args[0]),
                           "texts": [], "start": index}
                groups.append(current)
            if "/ActualText" in props:
                require(current is not None, "unowned replacement text")
                current["texts"].append(str(props["/ActualText"]))
            stack.append(outer)
        elif op == b"BMC":
            stack.append(False)
        elif op == b"EMC":
            require(bool(stack), "unbalanced marked content")
            if stack.pop():
                current["end"] = index + 1
                current = None
    require(not stack, "unclosed marked content")
    speech = "one half equals two quarters"
    require([g["texts"] for g in groups] == [["A "], [speech], ["B"], [speech], ["ABAB"], ["B"]],
            "exact source-order replacement groups")
    number = groups[4]
    require(number["role"] == "/Span", "independent number Span")
    root = reader.trailer["/Root"]["/StructTreeRoot"]
    nums = root["/ParentTree"]["/Nums"]
    require(list(nums[::2]) == [0] and reader.pages[0]["/StructParents"] == 0, "page parent keys")
    parents = nums[1].get_object()
    require(len(parents) == len(groups), "dense parent array")
    for group, ref in zip(groups, parents):
        require(ref.get_object()["/S"] == group["role"], "ParentTree role")
    node = parents[number["mcid"]].get_object()
    require("/ActualText" not in node, "duplicate structural replacement")
    require(node["/P"]["/S"] == "/Formula", "number parent formula")
    kids = node["/K"]
    require(len(kids) == 1 and kids[0].get_object()["/MCID"] == number["mcid"], "number MCR")
    for command in (["pdftotext", "-enc", "UTF-8", "-raw", str(pdf), "-"],
                    ["mutool", "draw", "-q", "-F", "txt", str(pdf)]):
        text = subprocess.run(command, check=True, capture_output=True).stdout.decode("utf-8")
        require("".join(text.split()) == "".join(("A " + speech + "B" + speech + "ABAB" + "B").split()),
                f"independent extraction: {command[0]}")
    output.mkdir(parents=True, exist_ok=True)
    writer = PdfWriter()
    writer.clone_document_from_reader(reader)
    stream = ContentStream(writer.pages[0].get_contents(), writer)
    del stream.operations[number["start"]:number["end"]]
    replacement = DecodedStreamObject()
    replacement.set_data(stream.get_data())
    writer.pages[0][NameObject("/Contents")] = writer._add_object(replacement)
    without = output / "without-number.pdf"
    writer.write(without)
    images = []
    for name, path in [("number", pdf), ("without-number", without)]:
        png = output / f"{name}.png"
        subprocess.run(["mutool", "draw", "-q", "-r", "144", "-o", str(png), str(path), "1"],
                       check=True, capture_output=True)
        images.append(Image.open(png).convert("RGB"))
    delta = ImageChops.difference(*images)
    bbox = delta.getbbox()
    require(bbox is not None, "number must have visible ink")
    delta.save(output / "number-ink-difference.png")
    result = {"public_build": False, "full_book": False, "pages": 1, "number": "ABAB",
              "groups": len(groups), "extractors": ["Poppler", "MuPDF"], "number_ink_bbox_px": bbox}
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n")
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pdf", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    print(json.dumps(verify(args.pdf, args.output), indent=2))
