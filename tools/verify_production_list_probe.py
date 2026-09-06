#!/usr/bin/env python3
"""Independent generated-list-label PDF checks; this is not a public/full-book gate.

Rust asserts source/frame/baseline invariants before exporting selected receipts.
This script checks the serialized PDF independently, exact extraction for the
fixed source cases, and visible label ink against a counterfactual without labels.
"""
from __future__ import annotations
import argparse
import json
import math
import subprocess
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw
from pypdf import PdfReader, PdfWriter
from pypdf.generic import ContentStream, NameObject, FloatObject, TextStringObject, DecodedStreamObject

SPEECH = "one half equals two quarters"
INTRO = "A " + SPEECH + "B"
# Source-order expectations, with each extractor's fixed page/line framing.
POPPLER = {
    "nested": INTRO+"\n"+SPEECH+"\f9. B\n• B\f• B\n10. B\f",
    "block-first": INTRO+"\n• "+SPEECH+"\fB\f",
    "inline-first": "• "+INTRO+"\n"+SPEECH+"\fB\f",
    "raster-first": INTRO+"\n"+SPEECH+"\f•\nB\nB\f",
}
MUPDF = {
    "nested": INTRO+"\n\n"+SPEECH+"\n\n\f\n9.\nB\n\n•\nB\n\n\f\n•\nB\n10.\nB\n\n\f\n",
    "block-first": INTRO+"\n\n•\n"+SPEECH+"\n\n\f\nB\n\n\f\n",
    "inline-first": "•\n"+INTRO+"\n\n"+SPEECH+"\n\n\f\nB\n\n\f\n",
    "raster-first": INTRO+"\n\n"+SPEECH+"\n\n\f\n•\n\nB\nB\n\n\f\n",
}

class Failure(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Failure(message)


def refid(value):
    return value.indirect_reference.idnum


def stream(data):
    result=DecodedStreamObject()
    result.set_data(data)
    return result


def groups(reader):
    output=[]
    for page_index,page in enumerate(reader.pages):
        stack=[]
        font_size=None
        matrix=None
        rendering=None
        current=None
        for operands,op in ContentStream(page.get_contents(),reader).operations:
            if op==b"BDC":
                props=operands[1]
                if "/MCID" in props:
                    require(current is None,"nested MCID")
                    current={"page":page_index,"mcid":int(props["/MCID"]),"role":str(operands[0])[1:],"text":[],"glyphs":[]}
                    output.append(current)
                if "/ActualText" in props:
                    require(current is not None,"unowned ActualText")
                    current["text"].append(str(props["/ActualText"]))
                stack.append("/MCID" in props)
            elif op==b"BMC":
                stack.append(False)
            elif op==b"EMC":
                require(bool(stack),"unbalanced EMC")
                if stack.pop():current=None
            elif op==b"Tf":font_size=float(operands[1])
            elif op==b"Tr":rendering=int(operands[0])
            elif op==b"Tm":matrix=list(map(float,operands))
            elif op==b"Tj" and current is not None and current["role"]=="Lbl":
                require(rendering==0,"invisible label")
                require(matrix is not None and matrix[:4]==[1,0,0,-1],"label text matrix")
                current["glyphs"].append((matrix[4],matrix[5],font_size))
        require(not stack,"unclosed marked content")
    return output


def verify(reader,expected):
    require(expected["algorithm"]=="typaxis.production-list-probe/1" and expected["public_build"] is False and expected["full_book"] is False,"diagnostic probe scope")
    require(len(reader.pages)==expected["pages"],"page count")
    actual=groups(reader)
    require(len(actual)==len(expected["groups"]),"group count")
    root=reader.trailer["/Root"]["/StructTreeRoot"]
    nums=root["/ParentTree"]["/Nums"]
    parents=dict(zip(nums[::2],nums[1::2]))
    require(list(parents)==list(range(len(reader.pages))),"ParentTree keys")
    for group,want in zip(actual,expected["groups"]):
        require(all(group[k]==want[k] for k in ("page","mcid","role")),"reading group identity/order")
        require(group["text"]==([want["text"]] if want["text"] else []),"exact group ActualText")
        page=group["page"]
        require(reader.pages[page]["/StructParents"]==page,"page StructParents")
        node=parents[page][group["mcid"]].get_object()
        require(node["/S"]=="/"+group["role"],"ParentTree role")
    labels=[g for g in actual if g["role"]=="Lbl"]
    require(len(labels)==len(expected["labels"]),"label count")
    seen=set()
    for label,want in zip(labels,expected["labels"]):
        page,mcid=label["page"],label["mcid"]
        require((page,mcid)==(want["page"],want["mcid"]),"label page/MCID")
        node=parents[page][mcid].get_object()
        require(refid(node)==want["structure_object"] and refid(node) not in seen,"label owner")
        seen.add(refid(node))
        require("/ActualText" not in node,"duplicate structure-level replacement")
        require(label["text"]==[want["text"]],"label replacement")
        item=node["/P"]
        require(item["/S"]=="/LI" and refid(item)==want["item_object"],"label item parent")
        kids=[k.get_object() for k in item["/K"]]
        require([k.get("/S") for k in kids]==["/Lbl","/LBody"],"LI Lbl/LBody order")
        require(refid(kids[0])==refid(node) and refid(kids[1]["/P"])==refid(item),"list child backlink")
        require(item["/P"]["/S"]=="/L","LI parent list")
        numbering=item["/P"]["/A"]
        require(numbering["/O"]=="/List" and numbering["/ListNumbering"]==("/Disc" if want["text"]=="•" else "/Decimal"),"list numbering kind")
        mcrs=[k.get_object() for k in node["/K"]]
        require(len(mcrs)==1 and mcrs[0]["/Type"]=="/MCR" and mcrs[0]["/MCID"]==mcid,"single Lbl MCR")
        require(refid(mcrs[0]["/Pg"])==refid(reader.pages[page]),"Lbl MCR page")
        require(len(label["glyphs"])==len(want["glyphs_raw"]),"label glyph count")
        for (x,y,size),(X,Y) in zip(label["glyphs"],want["glyphs_raw"]):
            require(abs(x-X/65536)<0.00001 and abs(y-Y/65536)<0.00001,"label glyph position")
            require(abs(size-want["font_size_raw"]/65536)<0.00001,"label font size")
    return actual


def negatives(path,expected):
    first=expected["labels"][0]
    def label_node(reader):
        nums=reader.trailer["/Root"]["/StructTreeRoot"]["/ParentTree"]["/Nums"]
        return dict(zip(nums[::2],nums[1::2]))[first["page"]][first["mcid"]].get_object()
    def edit_operations(reader,mode):
        page=reader.pages[first["page"]]
        content=ContentStream(page.get_contents(),reader)
        stack=[];label=False;changed=False;output=[]
        for operands,op in content.operations:
            if op==b"BDC":
                stack.append(label)
                if "/MCID" in operands[1]:label=int(operands[1]["/MCID"])==first["mcid"]
                if label and not changed and mode=="text" and "/ActualText" in operands[1]:
                    operands[1][NameObject("/ActualText")]=TextStringObject("wrong");changed=True
            elif op==b"EMC":label=stack.pop()
            if label and not changed and op==b"Tm" and mode=="position":
                operands[5]=FloatObject(float(operands[5])+3);changed=True
            if label and not changed and op==b"Tj" and mode in ("missing","duplicate"):
                changed=True
                if mode=="missing":continue
                output.append((operands,op))
            output.append((operands,op))
        require(changed,"mutation did not apply")
        content.operations=output
        page[NameObject("/Contents")]=stream(content.get_data())
    def wrong_role(r):label_node(r)[NameObject("/S")]=NameObject("/Span")
    def wrong_actual(r):label_node(r)[NameObject("/ActualText")]=TextStringObject("wrong")
    def duplicate_mcr(r):label_node(r)["/K"].append(label_node(r)["/K"][0])
    def wrong_item(r):label_node(r)["/P"][NameObject("/S")]=NameObject("/P")
    def remove_child(r):label_node(r)["/P"]["/K"].pop(0)
    changes=[("missing",lambda r:edit_operations(r,"missing")),("duplicate",lambda r:edit_operations(r,"duplicate")),
             ("position",lambda r:edit_operations(r,"position")),("paint_text",lambda r:edit_operations(r,"text")),
             ("role",wrong_role),("replacement",wrong_actual),("duplicate_mcr",duplicate_mcr),
             ("item_parent",wrong_item),("missing_label_child",remove_child)]
    for name,change in changes:
        reader=PdfReader(path);change(reader)
        try:verify(reader,expected)
        except Failure:continue
        raise Failure("mutation accepted: "+name)
    return [name for name,_ in changes]


def without_labels(path,output):
    writer=PdfWriter(clone_from=path)
    for page in writer.pages:
        content=ContentStream(page.get_contents(),writer)
        stack=[];label=False;kept=[]
        for operands,op in content.operations:
            if op==b"BDC":
                stack.append(label)
                if "/MCID" in operands[1]:label=operands[0]=="/Lbl"
            elif op==b"BMC":stack.append(label)
            elif op==b"EMC":label=stack.pop()
            if op==b"Tj" and label:continue
            kept.append((operands,op))
        content.operations=kept;page.replace_contents(content)
    writer.write(output)


def render_check(path,expected,out,mutool):
    missing=out/(path.stem+"-without-labels.pdf");without_labels(path,missing)
    patterns=[]
    for suffix,pdf in [("paint",path),("without",missing)]:
        pattern=out/(path.stem+"-"+suffix+"-%d.png")
        subprocess.run([mutool,"draw","-q","-r","144","-o",str(pattern),str(pdf)],check=True,capture_output=True)
        patterns.append(pattern)
    ink=[]
    for index in range(expected["pages"]):
        a=Image.open(str(patterns[0]).replace("%d",str(index+1))).convert("RGB")
        b=Image.open(str(patterns[1]).replace("%d",str(index+1))).convert("RGB")
        diff=ImageChops.difference(a,b).convert("L")
        mask=Image.new("L",a.size);draw=ImageDraw.Draw(mask)
        for label in expected["labels"]:
            if label["page"]!=index:continue
            x,y,w,h=[v/65536*2 for v in label["bounds_raw"]]
            box=(max(0,math.floor(x)-2),max(0,math.floor(y)-2),min(a.width,math.ceil(x+w)+2),min(a.height,math.ceil(y+h)+2))
            require(diff.crop(box).getbbox() is not None,"label has no visible ink")
            ink.append(label["owner"]);draw.rectangle(box,fill=255)
        require(ImageChops.subtract(diff,mask).getbbox() is None,"label removal changed other paint")
    require(len(ink)==len(expected["labels"]),"visible label count")
    return ink


def main():
    parser=argparse.ArgumentParser()
    parser.add_argument("probe",type=Path);parser.add_argument("output",type=Path)
    parser.add_argument("--mutool",default="/opt/homebrew/bin/mutool")
    parser.add_argument("--pdftotext",default="/opt/homebrew/bin/pdftotext")
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    observations=[]
    for name in POPPLER:
        path=args.probe/(name+".pdf");expected=json.loads(path.with_suffix(".expected.json").read_text())
        verify(PdfReader(path),expected)
        rejected=negatives(path,expected)
        poppler=subprocess.check_output([args.pdftotext,"-raw","-enc","UTF-8",str(path),"-"]).decode()
        mupdf=subprocess.check_output([args.mutool,"draw","-F","txt",str(path)],stderr=subprocess.DEVNULL).decode()
        require(poppler==POPPLER[name],"exact Poppler extraction: "+repr(poppler))
        require(mupdf==MUPDF[name],"exact MuPDF extraction: "+repr(mupdf))
        ink=render_check(path,expected,args.output,args.mutool)
        observations.append({"case":name,"pages":expected["pages"],"visible_labels":len(ink),"negative_checks":rejected,"extraction":"exact"})
    report={"algorithm":"typaxis.independent-list-probe/1","public_build":False,"full_book":False,"cases":observations,"failed_checks":0,"rejected_mutations":sum(len(c["negative_checks"]) for c in observations)}
    (args.output/"observed.json").write_text(json.dumps(report,indent=2)+"\n")
    print(json.dumps(report,indent=2))

if __name__=="__main__":main()
