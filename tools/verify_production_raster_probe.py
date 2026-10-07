#!/usr/bin/env python3
"""Independent selected-raster PDF payload/placement/tag checks, not a public gate."""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import subprocess
from pathlib import Path
from PIL import Image
from pypdf import PdfReader
from pypdf.generic import ContentStream, NameObject, TextStringObject, DecodedStreamObject


class Failure(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def compose(old, new):
    a, b, c, d, e, f = old
    A, B, C, D, E, F = new
    return [a*A+c*B, b*A+d*B, a*C+c*D, b*C+d*D, a*E+c*F+e, b*E+d*F+f]


def refid(value):
    return value.indirect_reference.idnum


def paints(reader):
    result = []
    for page_index, page in enumerate(reader.pages):
        matrix, stack, marked = [1, 0, 0, 1, 0, 0], [], []
        for operands, op in ContentStream(page.get_contents(), reader).operations:
            if op == b"q":
                stack.append(matrix[:])
            elif op == b"Q":
                require(bool(stack), "unbalanced graphics restore")
                matrix = stack.pop()
            elif op == b"cm":
                matrix = compose(matrix, list(map(float, operands)))
            elif op == b"BDC":
                marked.append((str(operands[0]), operands[1].get("/MCID")))
            elif op == b"BMC":
                marked.append((str(operands[0]), None))
            elif op == b"EMC":
                require(bool(marked), "unbalanced marked content")
                marked.pop()
            elif op == b"Do":
                image = page["/Resources"]["/XObject"][operands[0]]
                if image["/Subtype"] == "/Image":
                    require(marked and marked[-1][0] == "/Figure", "image Figure MCID")
                    require(marked[-1][1] is not None, "image lacks MCID")
                    result.append((page_index, matrix[:], marked[-1][1], image))
        require(not stack and not marked, "unbalanced page state")
    return result


def verify(reader, expected, original):
    require(len(reader.pages) == expected["page_count"], "page count")
    require(hashlib.sha256(original).hexdigest() == expected["source_sha256"], "source hash")
    actual = paints(reader)
    require(len(actual) == len(expected["draws"]), "raster draw count")
    require(len({refid(p[3]) for p in actual}) == 1, "shared image object")
    root = reader.trailer["/Root"]["/StructTreeRoot"]
    nums = root["/ParentTree"]["/Nums"]
    parents = dict(zip(nums[::2], nums[1::2]))
    figure_ids = set()
    for (page_index, matrix, mcid, image), draw in zip(actual, expected["draws"]):
        require(page_index == draw["page"], "raster page")
        x, y, w, h = [v/65536 for v in draw["viewport_raw"]]
        height = float(reader.pages[page_index].mediabox.height)
        target = [w, 0, 0, h, x, height-y-h]
        require(all(abs(a-b) < 0.00001 for a,b in zip(matrix, target)), "raster matrix/orientation")
        node = parents[page_index][mcid].get_object()
        require(node["/S"] == "/Figure" and node["/Alt"] == draw["alt"], "Figure Alt")
        require("/ActualText" not in node, "invented raster ActualText")
        require(refid(node) not in figure_ids, "merged source Figures")
        figure_ids.add(refid(node))
        mcrs = [k.get_object() for k in node["/K"] if k.get_object().get("/Type") == "/MCR"]
        require(len(mcrs) == 1 and mcrs[0]["/MCID"] == mcid, "Figure MCR")
        require(refid(mcrs[0]["/Pg"]) == refid(reader.pages[page_index]), "Figure MCR page")
        require(image["/Width"] == expected["pixel_width"] and image["/Height"] == expected["pixel_height"], "image dimensions")
        require(image["/BitsPerComponent"] == 8, "image bit depth")
    image = actual[0][3]
    source = Image.open(io.BytesIO(original)).convert("RGBA")
    if expected["source"].endswith(".jpg"):
        require(image["/Filter"] == "/DCTDecode", "JPEG filter")
        require(image["/DecodeParms"]["/ColorTransform"] == 1, "JPEG color transform")
        require(Image.open(io.BytesIO(image.get_data())).convert("RGBA").tobytes() == source.tobytes(), "JPEG pixels")
    else:
        require(image["/Filter"] == "/FlateDecode", "PNG lossless filter")
        mode = {"/DeviceRGB":"RGB", "/DeviceGray":"L"}.get(image["/ColorSpace"])
        require(mode is not None, "PNG color space")
        require(image.get_data() == source.convert(mode).tobytes(), "PNG color pixels")
    alpha = source.getchannel("A")
    if alpha.getextrema()[0] < 255:
        require("/SMask" in image, "missing alpha mask")
        mask = image["/SMask"]
        require(mask["/Width"] == source.width and mask["/Height"] == source.height, "alpha dimensions")
        require(mask["/ColorSpace"] == "/DeviceGray" and mask["/BitsPerComponent"] == 8, "alpha format")
        require(mask.get_data() == alpha.tobytes(), "alpha pixels")
    else:
        require("/SMask" not in image, "spurious opaque mask")
    return actual


def negatives(path, expected, source):
    def remove_draw(reader):
        page = reader.pages[expected["draws"][0]["page"]]
        data = page.get_contents().get_data()
        page[NameObject("/Contents")] = stream(data.replace(b"/PBR0 Do", b"", 1))

    def duplicate_draw(reader):
        page = reader.pages[expected["draws"][0]["page"]]
        data = page.get_contents().get_data()
        page[NameObject("/Contents")] = stream(data.replace(b"/PBR0 Do", b"/PBR0 Do /PBR0 Do", 1))

    def wrong_alt(reader):
        page, _, mcid, _ = paints(reader)[0]
        nums = reader.trailer["/Root"]["/StructTreeRoot"]["/ParentTree"]["/Nums"]
        dict(zip(nums[::2], nums[1::2]))[page][mcid].get_object()[NameObject("/Alt")] = TextStringObject("wrong")

    def shift(reader):
        page = reader.pages[expected["draws"][0]["page"]]
        data = page.get_contents().get_data()
        page[NameObject("/Contents")] = stream(data.replace(b"/PBR0 Do", b"1 0 0 1 1 0 cm /PBR0 Do", 1))

    checks = [("missing draw", remove_draw), ("duplicate draw", duplicate_draw), ("wrong Alt", wrong_alt), ("shifted placement", shift)]
    def flip(reader):
        page = reader.pages[expected["draws"][0]["page"]]
        data = page.get_contents().get_data()
        page[NameObject("/Contents")] = stream(data.replace(b"/PBR0 Do", b"1 0 0 -1 0 1 cm /PBR0 Do", 1))
    checks.append(("flipped image", flip))
    if expected["source"] == "orientation-alpha.png":
        def no_mask(reader):
            del paints(reader)[0][3]["/SMask"]
        checks.append(("missing mask", no_mask))
        def changed_mask(reader):
            image = paints(reader)[0][3]
            prior = image["/SMask"]
            replacement = stream(bytes([255,255,255,255]))
            for key in ["/Type","/Subtype","/Width","/Height","/ColorSpace","/BitsPerComponent"]:
                replacement[NameObject(key)] = prior[key]
            image[NameObject("/SMask")] = replacement
        checks.append(("changed alpha", changed_mask))
    for label, mutate in checks:
        reader = PdfReader(path)
        mutate(reader)
        try:
            verify(reader, expected, source)
        except Failure:
            continue
        raise Failure(f"mutation accepted: {label}")
    return len(checks)


def stream(data):
    output = DecodedStreamObject()
    output.set_data(data)
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe-root", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--mutool", required=True)
    parser.add_argument("--pdftotext", required=True)
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[1]
    args.output_root.mkdir(parents=True, exist_ok=True)
    reports = []
    for expected_file in sorted(args.probe_root.glob("*.expected.json")):
        expected = json.loads(expected_file.read_text())
        name = expected["source"]
        path = args.probe_root / f"{name}.pdf"
        fixture = (repository/"samples/machine-package/profiles/production-book-1/combined/job" if name.endswith(".jpg") else
                   repository/"samples/machine-package/staging/production-book-1/vmb-book/raster") / name
        source = fixture.read_bytes()
        verify(PdfReader(path), expected, source)
        rejected = negatives(path, expected, source)
        output = args.output_root/name
        output.mkdir(exist_ok=True)
        subprocess.run([args.mutool,"draw","-q","-r","144","-F","png","-o",str(output/"page-%d.png"),str(path)],check=True,capture_output=True)
        subprocess.run([args.pdftotext,"-raw","-enc","UTF-8",str(path),str(output/"text.txt")],check=True,capture_output=True)
        text = (output/"text.txt").read_text()
        require("VMB diagram" not in text and "Same pixels" not in text, "Alt substituted for extracted body")
        mupdf = subprocess.run([args.mutool,"draw","-q","-F","txt",str(path)],check=True,capture_output=True).stdout.decode()
        (output/"mupdf-text.txt").write_text(mupdf)
        logical = "A one half equals two quartersBone half equals two quartersBB"
        for extractor, extracted in [("Poppler raw",text),("MuPDF",mupdf)]:
            # Remove only extractor-inserted line/page separators, not authored
            # spaces or duplicate body/caption/formula occurrences.
            require(extracted.translate(str.maketrans("","","\r\n\f")) == logical,
                    f"{extractor} exact source-order text/caption/formula extraction")
        # Figure pixels are distinct from the independently verified SVG/text
        # probes. Sample quadrant centers away from interpolation/edge coverage.
        if name == "orientation-alpha.png":
            for draw in expected["draws"]:
                rendered = Image.open(output/f"page-{draw['page']+1}.png").convert("RGB")
                x,y,w,h = [v/32768 for v in draw["viewport_raw"]]
                for u,v,color in [(0.25,0.25,(255,0,0)),(0.75,0.25,(127,255,127)),(0.25,0.75,(191,191,255)),(0.75,0.75,(255,255,255))]:
                    actual = rendered.getpixel((int(x+w*u),int(y+h*v)))
                    require(max(abs(a-b) for a,b in zip(actual,color))<=2, "rendered orientation/alpha")
        reports.append({"source":name,"draws":len(expected["draws"]),"rejected_mutations":rejected,"pdf_sha256":hashlib.sha256(path.read_bytes()).hexdigest(),
                        "encoded_bytes":expected["encoded_bytes"],"peak_spool":expected["peak_spool"],"retained_spool":expected["retained_spool"]})
    require(len(reports) == 3, "expected all three PNG/JPEG cases")
    report = {"algorithm":"typaxis.independent-raster-probe/1","public_build":False,"full_book":False,
              "cases":reports,"failed_checks":0,"rejected_mutations":sum(r["rejected_mutations"] for r in reports)}
    (args.output_root/"observed.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))


if __name__ == "__main__":
    main()
