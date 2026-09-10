#!/usr/bin/env python3
"""Independently check actual source-page PDF bytes and their complete object graph.

This verifies inspectable Book-2 output, not public-profile or PDF/UA approval.
"""
import argparse
import copy
import hashlib
import io
import json
import re
from pathlib import Path
import xml.etree.ElementTree as ET
from pypdf import PdfReader
from pypdf.generic import ArrayObject, ContentStream, DecodedStreamObject, DictionaryObject, FloatObject, NameObject, NumberObject
from verify_book_v2_navigation_geometry import verify as verify_navigation


def original(value):
    return value.original_bytes if hasattr(value, 'original_bytes') else bytes(value)


def refid(value):
    return value.idnum if hasattr(value, 'idnum') else value.indirect_reference.idnum


def operations(data, reader):
    stream = DecodedStreamObject()
    stream.set_data(data)
    return ContentStream(stream, reader).operations


def verify_xref(data):
    trailer = re.search(rb'startxref\n([0-9]+)\n%%EOF\n$', data)
    assert trailer
    start = int(trailer[1])
    lines = data[start:].splitlines(keepends=True)
    assert lines[0] == b'xref\n'
    first, count = map(int, lines[1].split())
    assert first == 0
    free = {}
    for i, entry in enumerate(lines[2:2 + count]):
        assert len(entry) == 20
        offset, generation, status = entry.split()
        offset, generation = int(offset), int(generation)
        if status == b'n':
            assert generation == 0
            assert data[offset:].startswith(f'{i} 0 obj\n'.encode())
        else:
            assert status == b'f' and generation == (65535 if i == 0 else 0)
            free[i] = offset
    assert lines[2 + count] == b'trailer\n'
    assert re.search(rb'/Size\s+' + str(count).encode() + rb'\b', lines[3 + count])
    seen = {0}
    cursor = free[0]
    while cursor:
        assert cursor in free and cursor not in seen
        seen.add(cursor)
        cursor = free[cursor]
    assert seen == set(free)


def selected_master(masters, page, name=None):
    # Independently rank the authored rule facts, using one-based physical parity.
    choices=[]
    for rule in masters['selection_rules']:
        if rule['parity']!='any' and (rule['parity']=='odd') != ((page+1)%2==1):continue
        if rule['first'] is not None and rule['first'] != (page==0):continue
        if rule['named_page'] is not None and rule['named_page'] != name:continue
        choices.append(((rule['named_page'] is not None,rule['first'] is not None,rule['parity']!='any',rule['source_order']),rule['master_id']))
    chosen=max(choices)[1] if choices else masters['default_master_id']
    return next(m for m in masters['masters'] if m['master_id']==chosen)


def source_page_names(probe):
    """Check page names against original body owners, excluding definition paint."""
    wire=probe['wire']
    sheet={r['style_id']:r for r in wire['style_sheet']['rules']}
    def declarations(rule,seen=()):
        assert rule['style_id'] not in seen
        parent=rule.get('extends')
        return ((declarations(sheet[parent],seen+(rule['style_id'],)) if parent else [])+rule['declarations'])
    owners={}
    def visit(value,used=None):
        if isinstance(value,list):
            for child in value:visit(child,used)
        elif isinstance(value,dict):
            kind=value.get('kind')
            winner=None
            for rule in sheet.values():
                relevant=[d for d in declarations(rule) if d['name']=='page']
                if not relevant:continue
                selector=rule['selector']
                match=re.fullmatch(r'[a-z_]+(?:\.[A-Za-z0-9_-]+)*',selector)
                assert match,('unsupported page selector in independent check',selector)
                tag,*classes=selector.split('.')
                if tag!=kind or not all(cls in value.get('classes',[]) for cls in classes):continue
                for order,d in enumerate(relevant):
                    key=(d['important'],len(classes),rule['source_order'],order)
                    if winner is None or key>winner[0]:winner=(key,d['value'])
            if winner is not None:
                declaration=winner[1]
                if declaration['kind']=='string':used=declaration['value']
                else:assert declaration=={'kind':'keyword','value':'auto'}
            if 'node_id' in value:owners[value['node_id']]=used
            for field in ('blocks','children','items','term','caption','head','body','cells','equation_number'):
                if field in value:visit(value[field],used)
    visit(wire['document']['blocks'])
    nav=probe['relations']['navigation']
    names=nav['assembly'].get('page_names',[None]*nav['raw_page_count'])
    assert len(names)==nav['raw_page_count']
    for group in probe['groups']:
        if group['owner'] in owners:assert names[group['page']]==owners[group['owner']],('source page name',group['owner'],names[group['page']],owners[group['owner']])
    return names


def master_for(probe,page):
    names=probe['relations']['navigation']['assembly'].get('page_names')
    return selected_master(probe['wire']['page_masters'],page,None if names is None else names[page])


def verify(reader, probe):
    nav = probe['relations']['navigation']
    assembly = nav['assembly']
    raw_references = {}

    def source_references(value):
        if isinstance(value, list):
            for item in value:
                source_references(item)
        elif isinstance(value, dict):
            if value.get('kind') == 'reference' and value['format'] == 'page':
                raw_references[value['node_id']] = value
            for name in ('term', 'blocks', 'children', 'items', 'caption', 'head', 'body', 'cells', 'equation_number', 'footnotes'):
                if name in value:
                    source_references(value[name])

    source_references(probe['wire']['document'])
    candidates = assembly['candidates']
    assert [owner for owner, _ in candidates] == sorted(raw_references)
    expected_references = []
    keys = {tuple(n['key']): i for i, n in enumerate(probe['nodes'])}
    for owner, candidate in candidates:
        assert isinstance(candidate, int) and 0 < candidate <= 0xffffffff
        link = next(l for l in nav['links'] if l['node'] == keys[owner, 'ReferenceLink'])
        assert link['target'][0] == 'destination'
        destination = nav['destinations'][link['target'][1]]
        assert destination['kind'] == ['anchor', raw_references[owner]['target']]
        position = destination['position']
        placed = link['first'] is not None
        target = None if position is None else position[0] + 1
        assert not placed or target is not None
        expected_references.append(dict(owner=owner, candidate=candidate, target=target, placed=placed))
        label_node = keys[owner, 'ReferenceLabel']
        groups = [probe['groups'][b['group']] for b in probe['bindings'] if b['node'] == label_node]
        labels = []
        for group in groups:
            for args, op in operations(group['bytes'].encode(), reader):
                if op == b'BDC' and '/ActualText' in args[1]:
                    labels.append(str(args[1]['/ActualText']))
        assert ''.join(labels) == (str(candidate) if placed else '')
    assert assembly['references'] == expected_references
    assert assembly['labels_match'] == all(r['candidate'] == r['target'] if r['target'] is not None
                                          else not r['placed'] for r in expected_references)
    masters = probe['wire']['page_masters']
    pages = reader.pages
    assert len(pages) == len(assembly['pages']) == nav['raw_page_count']
    root = reader.trailer['/Root']
    assert refid(root) == assembly['catalog']
    assert root['/MarkInfo']['/Marked']
    assert root['/ViewerPreferences']['/DisplayDocTitle']
    assert root['/Lang'] == probe['nodes'][0]['language']
    page_annots = []
    for i, page in enumerate(pages):
        master=master_for(probe,i)
        width,height=master['width']/65536,master['height']/65536
        trim = master['trim']
        expected_trim = [trim['x'] / 65536, (master['height'] - trim['y'] - trim['height']) / 65536,
                         (trim['x'] + trim['width']) / 65536, (master['height'] - trim['y']) / 65536]
        assert refid(page) == assembly['pages'][i]
        assert list(page['/MediaBox']) == [0, 0, width, height]
        assert list(page['/TrimBox']) == expected_trim
        assert page['/StructParents'] == i
        assert page['/Tabs'] == '/S'
        actual = page.get_contents().operations
        assert actual[:2] == [([], b'q'), ([1, 0, 0, -1, 0, height], b'cm')]
        assert actual[-1] == ([], b'Q')
        expected = []
        for group in probe['groups']:
            if group['page'] == i:
                expected.extend(operations(group['bytes'].encode(), reader))
        # The marked-page owner isolates its complete group sequence as well
        # as the assembly's source-page transform and each individual group.
        assert actual[2:-1] == [([], b'q')] + expected + [([], b'Q')]
        resources = page['/Resources']
        for args, op in actual:
            if op == b'Tf':
                assert args[0] in resources['/Font']
                font = resources['/Font'][args[0]]
                assert font['/Type'] == '/Font'
                assert '/ToUnicode' in font
                if font['/Subtype'] == '/Type0':
                    descriptor = font['/DescendantFonts'][0].get_object()['/FontDescriptor']
                    assert '/FontFile2' in descriptor or '/FontFile3' in descriptor
            if op == b'Do':
                assert args[0] in resources['/XObject']
        annotations = list(page.get('/Annots', []))
        assert len(annotations) == nav['page_counts'][i]
        page_annots.extend(annotations)
    assert len(page_annots) == len(nav['rectangles'])
    tree = root['/StructTreeRoot']
    pairs = tree['/IDTree']['/Names']
    assert len(pairs) == 2 * len(probe['nodes'])
    node_refs = []
    for i in range(len(probe['nodes'])):
        assert original(pairs[2 * i]) == i.to_bytes(4, 'big')
        node_refs.append(pairs[2 * i + 1])
    assert len(tree['/K']) == 1 and refid(tree['/K'][0]) == refid(node_refs[0])
    parents_raw = tree['/ParentTree']['/Nums']
    parents = dict(zip(parents_raw[::2], parents_raw[1::2]))
    assert list(parents) == list(range(len(pages) + len(page_annots)))
    assert tree['/ParentTreeNextKey'] == len(parents)
    seen_mcids, seen_objr = set(), set()
    rel_nodes = probe['relations']['nodes']
    for i, (source, rel) in enumerate(zip(probe['nodes'], rel_nodes)):
        node = node_refs[i].get_object()
        assert node['/Type'] == '/StructElem'
        assert node['/S'] == '/' + source['role']
        assert node['/Lang'] == source['language']
        assert node.get('/Alt') == source['alt']
        assert node.get('/TypaxisSourceKind') == source['semantic_kind']
        if source['role'] == 'Caption':
            assert '/A' not in node, 'caption acquired table-cell attributes'
        assert original(node['/ID']) == i.to_bytes(4, 'big')
        parent = tree if rel['parent'] is None else node_refs[rel['parent']]
        assert refid(node['/P']) == refid(parent)
        assert [refid(v) for v in node.get('/Ref', [])] == [refid(node_refs[j]) for j in rel['related']]
        if rel['list_numbering']:
            assert node['/A']['/O'] == '/List'
            assert node['/A']['/ListNumbering'] == '/' + rel['list_numbering']
        if source['cell'] is not None:
            cell = source['cell']
            attr = node['/A']
            assert attr['/O'] == '/Table'
            # Source cell record: table, row, column, colspan, rowspan, header.
            assert attr.get('/Scope') == ('/Column' if cell['header'] else None)
            assert attr.get('/ColSpan', 1) == cell['colspan']
            assert attr.get('/RowSpan', 1) == cell['rowspan']
            assert [original(v) for v in attr.get('/Headers', [])] == [j.to_bytes(4, 'big') for j in rel['headers']]
        children = []
        cursor = rel['first_child']
        while cursor is not None:
            children.append(cursor)
            cursor = rel_nodes[cursor]['next']
        bindings = [b for b in probe['bindings'] if b['node'] == i]
        rectangles = [j for j, r in enumerate(nav['rectangles']) if nav['links'][r['link']]['node'] == i]
        entries = node['/K']
        assert len(entries) == len(bindings) + len(children) + len(rectangles)
        for entry, binding in zip(entries, bindings):
            assert entry['/Type'] == '/MCR'
            pi, mcid = binding['page'], binding['mcid']
            assert refid(entry['/Pg']) == refid(pages[pi])
            assert entry['/MCID'] == mcid
            assert (pi, mcid) not in seen_mcids
            seen_mcids.add((pi, mcid))
            assert refid(parents[pi][mcid]) == refid(node_refs[i])
        first = len(bindings)
        assert [refid(v) for v in entries[first:first + len(children)]] == [refid(node_refs[j]) for j in children]
        first += len(children)
        for entry, ai in zip(entries[first:], rectangles):
            assert entry['/Type'] == '/OBJR'
            assert refid(entry['/Pg']) == refid(pages[nav['rectangles'][ai]['page']])
            assert refid(entry['/Obj']) == refid(page_annots[ai])
            assert ai not in seen_objr
            seen_objr.add(ai)
            assert refid(parents[len(pages) + ai]) == refid(node_refs[i])
    assert len(seen_mcids) == len(probe['bindings'])
    assert len(seen_objr) == len(page_annots)
    for pi in range(len(pages)):
        assert len(parents[pi]) == sum(b['page'] == pi for b in probe['bindings'])

    def destination(value, di):
        expected = nav['destinations'][di]['position']
        assert expected is not None
        pi, _, x, y = expected
        assert refid(value[0]) == refid(pages[pi])
        target_height=master_for(probe,pi)['height']/65536
        assert list(value[1:4]) == ['/XYZ', x / 65536, target_height - y / 65536]
        assert value[4].__class__.__name__ == 'NullObject'

    for ai, (ref, rectangle) in enumerate(zip(page_annots, nav['rectangles'])):
        annotation = ref.get_object()
        assert annotation['/Type'] == '/Annot' and annotation['/Subtype'] == '/Link'
        assert annotation['/F'] == 4 and list(annotation['/Border']) == [0, 0, 0]
        assert annotation['/StructParent'] == len(pages) + ai
        assert refid(annotation['/P']) == refid(pages[rectangle['page']])
        x, y, w, h = rectangle['bounds']
        height=master_for(probe,rectangle['page'])['height']/65536
        assert list(annotation['/Rect']) == [x / 65536, height - (y + h) / 65536,
                                             (x + w) / 65536, height - y / 65536]
        target = nav['links'][rectangle['link']]['target']
        if target[0] == 'uri':
            assert annotation['/A']['/S'] == '/URI'
            assert original(annotation['/A']['/URI']) == target[1].encode()
        else:
            destination(annotation['/Dest'], target[1])
    named = [(i, d) for i, d in enumerate(nav['destinations']) if d['kind'][0] == 'anchor' and d['position'] is not None]
    if named:
        entries = root['/Names']['/Dests']['/Names']
        assert len(entries) == len(named) * 2
        for (i, d), name, dest in zip(named, entries[::2], entries[1::2]):
            assert original(name) == d['kind'][1].encode()
            destination(dest, i)
    else:
        assert '/Names' not in root
    if nav['outline']:
        outline_root = root['/Outlines']
        assert outline_root['/Count'] == len(nav['outline'])
        ordered = []

        def visit(ref):
            node = ref.get_object()
            ordered.append(ref)
            if '/First' in node:
                visit(node.raw_get('/First'))
            if '/Next' in node:
                visit(node.raw_get('/Next'))
        visit(outline_root.raw_get('/First'))
        assert len(ordered) == len(nav['outline'])
        for i, (ref, expected) in enumerate(zip(ordered, nav['outline'])):
            node = ref.get_object()
            assert node['/Title'] == nav['source_outline'][i]['label']
            assert refid(node['/Parent']) == refid(outline_root if expected['parent'] is None else ordered[expected['parent']])
            for key, field in [('/Prev', 'previous'), ('/Next', 'next'), ('/First', 'first'), ('/Last', 'last')]:
                if expected[field] is None:
                    assert key not in node
                else:
                    assert refid(node[key]) == refid(ordered[expected[field]])
            assert node.get('/Count', 0) == expected['descendants']
            destination(node['/Dest'], expected['destination'])
    else:
        assert '/Outlines' not in root
    xmp = root['/Metadata'].get_data()
    xml = ET.fromstring(xmp)
    assert b'<pdfuaid:part>' not in xmp
    metadata = probe['wire']['metadata']
    for key, field in [('/Title', 'title'), ('/Author', 'author'), ('/Subject', 'subject')]:
        assert reader.metadata.get(key) == metadata[field]
    ns = dict(dc='http://purl.org/dc/elements/1.1/', rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#',
              pdf='http://ns.adobe.com/pdf/1.3/', xmp='http://ns.adobe.com/xap/1.0/')
    for path, field, repetitions in [('dc:title/rdf:Alt/rdf:li', 'title', 2),
                                     ('dc:description/rdf:Alt/rdf:li', 'subject', 2),
                                     ('dc:creator/rdf:Seq/rdf:li', 'author', 1),
                                     ('dc:identifier', 'identifier', 1),
                                     ('xmp:CreateDate', 'created', 1),
                                     ('xmp:ModifyDate', 'modified', 1)]:
        actual = [e.text or '' for e in xml.findall('.//' + path, ns)]
        assert actual == ([metadata[field]] * repetitions if metadata[field] is not None else [])
    assert [e.text for e in xml.findall('.//dc:subject/rdf:Bag/rdf:li', ns)] == metadata['keywords']
    assert reader.metadata.get('/Keywords') == ('; '.join(metadata['keywords']) if metadata['keywords'] else None)
    for key, field in [('/CreationDate', 'created'), ('/ModDate', 'modified')]:
        expected = metadata[field]
        assert reader.metadata.get(key) == ('D:' + re.sub('[-:T]', '', expected) if expected is not None else None)
    return len(pages), len(node_refs), len(page_annots)


def tamper_checks(data, probe):
    actions = [lambda r: r.pages[0].__setitem__(NameObject('/StructParents'), NumberObject(999)),
               lambda r: r.pages[0].__setitem__(NameObject('/MediaBox'), ArrayObject([NumberObject(0)] * 4)),
               lambda r: r.trailer['/Root']['/StructTreeRoot'].__setitem__(NameObject('/ParentTreeNextKey'), NumberObject(999999))]
    heights=[master_for(probe,page)['height']/65536
             for page in range(probe['relations']['navigation']['raw_page_count'])]
    if len(set(heights))>1:
        different=next(i for i,h in enumerate(heights) if h!=heights[0])
        actions.append(lambda r:r.pages[different].__setitem__(NameObject('/MediaBox'),r.pages[0]['/MediaBox']))
        actions.append(lambda r:r.pages[different].__setitem__(NameObject('/TrimBox'),r.pages[0]['/TrimBox']))
        def wrong_destination_height(reader):
            for page in reader.pages:
                for ref in page.get('/Annots',[]):
                    annotation=ref.get_object()
                    if '/Dest' not in annotation:continue
                    destination=annotation['/Dest']
                    target=next(i for i,p in enumerate(reader.pages) if refid(p)==refid(destination[0]))
                    if heights[target]!=heights[0]:
                        destination[3]=FloatObject(float(destination[3])+heights[0]-heights[target])
                        return
            raise AssertionError('missing cross-height link target')
        navigation=probe['relations']['navigation']
        targets=[navigation['links'][r['link']]['target'] for r in navigation['rectangles']]
        if any(target[0]!='uri' and navigation['destinations'][target[1]]['position'] is not None
               and heights[navigation['destinations'][target[1]]['position'][0]]!=heights[0] for target in targets):
            actions.append(wrong_destination_height)
    caption = next((i for i, n in enumerate(probe['nodes']) if n['role'] == 'Caption'
                    and probe['nodes'][n['parent']]['role'] == 'Table'), None)
    if caption is not None:
        def caption_node(reader):
            return reader.trailer['/Root']['/StructTreeRoot']['/IDTree']['/Names'][2 * caption + 1].get_object()
        actions.extend([
            lambda r: caption_node(r).__setitem__(NameObject('/S'), NameObject('/TH')),
            lambda r: caption_node(r).__setitem__(NameObject('/A'), DictionaryObject({NameObject('/O'): NameObject('/Table')})),
            lambda r: caption_node(r).__setitem__(NameObject('/P'), r.trailer['/Root']['/StructTreeRoot'].indirect_reference),
        ])
    if probe['relations']['navigation']['rectangles']:
        actions.append(lambda r: r.pages[next(i for i, p in enumerate(r.pages) if p.get('/Annots'))]['/Annots'][0].get_object().__setitem__(NameObject('/Rect'), ArrayObject([NumberObject(0)] * 4)))
    if probe['bindings']:
        def change_mcid_parent(reader):
            tree = reader.trailer['/Root']['/StructTreeRoot']
            values = tree['/ParentTree']['/Nums']
            for value in values[1::2]:
                if isinstance(value, ArrayObject) and value:
                    value[0] = tree['/K'][0]
                    return
            raise AssertionError('missing tested page MCID parent')
        actions.append(change_mcid_parent)
    for action in actions:
        reader = PdfReader(io.BytesIO(data), strict=True)
        action(reader)
        try:
            verify(reader, probe)
        except (AssertionError, KeyError, IndexError, TypeError, ValueError):
            continue
        raise AssertionError('tampered PDF accepted')
    # Change the on-disk xref independently of pypdf's object cache.
    xref = int(re.search(rb'startxref\n([0-9]+)\n%%EOF\n$', data)[1])
    lines = data[xref:].splitlines(keepends=True)
    count = int(lines[1].split()[1])
    entries_start = xref + len(lines[0]) + len(lines[1])
    used = next(i for i, line in enumerate(lines[2:2 + count]) if line[17:18] == b'n')
    changes = [(used, int(lines[2 + used][:10]) + 1),
               (0, 0 if int(lines[2][:10]) else used)]
    for index, value in changes:
        changed = bytearray(data)
        start = entries_start + 20 * index
        changed[start:start + 10] = f'{value:010d}'.encode()
        try:
            verify_xref(bytes(changed))
        except AssertionError:
            continue
        raise AssertionError('tampered xref accepted')
    reference_changes = []
    assembly = probe['relations']['navigation']['assembly']
    if assembly['references']:
        reference_changes = [
            lambda a: a['references'][0].__setitem__('target', 0),
            lambda a: a['references'][0].__setitem__('placed', not a['references'][0]['placed']),
            lambda a: a.__setitem__('labels_match', not a['labels_match']),
            lambda a: a['candidates'].append(a['candidates'][0]),
        ]
        if assembly['references'][0]['placed']:
            def change_candidate(a):
                # Even a coherently altered observation must match actual PDF text.
                a['candidates'][0][1] += 1
                a['references'][0]['candidate'] += 1
            reference_changes.append(change_candidate)
    for change in reference_changes:
        changed = copy.deepcopy(probe)
        change(changed['relations']['navigation']['assembly'])
        try:
            verify(PdfReader(io.BytesIO(data), strict=True), changed)
        except (AssertionError, KeyError, IndexError, TypeError, ValueError):
            continue
        raise AssertionError('tampered page-reference feedback accepted')
    return len(actions) + len(changes) + len(reference_changes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    totals = [0, 0, 0]
    count = rejected = unsupported = driver_count = 0
    inputs = [(path, json.loads(path.read_text())) for path in sorted(args.directory.glob('*.json'))]
    displays = {}
    for _, probe in inputs:
        if 'display' in probe:
            displays.setdefault(tuple(probe['display']), []).append(probe)
    for path in sorted(args.directory.glob('*.driver')):
        driver = json.loads(path.read_text())
        matches = displays[tuple(driver['display'])]
        p = copy.deepcopy(matches[0])
        # Each component probe has its own PDF numbering. Compare the original
        # source graph and selected paints, then inspect the callback's bytes.
        for match in matches[1:]:
            for field in ('wire', 'nodes', 'bindings', 'groups'):
                assert p[field] == match[field]
        p['relations']['navigation']['assembly'] = driver['assembly']
        assert driver['assembly']['labels_match']
        assert driver['passes'][0] >= (2 if driver['assembly']['references'] else 1)
        assert driver['passes'][2] >= driver['passes'][0] * 2
        assert driver['charges'][2] >= driver['assembly']['bytes']
        inputs.append((path, p))
        driver_count += 1
    for path, p in inputs:
        verify_navigation(p)
        nav = p['relations']['navigation']
        if 'error' in nav:
            continue
        assembly = nav['assembly']
        masters = p['wire']['page_masters']
        choices=[master_for(p,page) for page in range(nav['raw_page_count'])]
        unsupported_master=any(m['header_content'] is not None or m['footer_content'] is not None or m['column_layout'] is not None for m in choices)
        if unsupported_master:
            assert assembly['error'] == 'UnsupportedPageMaster'
            unsupported += 1
            continue
        if 'error' in assembly and any(assembly['frame'] != [m['body'][key] for key in ['x','y','width','height']] for m in choices):
            assert assembly['error'] == 'PageFrameMismatch'
            unsupported += 1
            continue
        assert 'error' not in assembly
        source_page_names(p)
        expected_frames = [[m['body'][key] for key in ['x','y','width','height']] for m in choices]
        assert assembly.get('page_frames', [assembly['frame']] * len(choices)) == expected_frames
        if 'footnote_regions' in assembly:
            expected_notes = [[m['footnote'][key] for key in ['x','y','width','height']] if p['wire']['document']['footnotes'] else None for m in choices]
            assert assembly['footnote_regions'] == expected_notes
        data = Path(assembly['pdf']).read_bytes()
        assert len(data) == assembly['bytes']
        assert list(hashlib.sha256(data).digest()) == assembly['hash']
        verify_xref(data)
        try:
            result = verify(PdfReader(io.BytesIO(data), strict=True), p)
            if args.self_test:
                rejected += tamper_checks(data, p)
        except Exception as e:
            raise AssertionError(f'{path}: {e}') from e
        totals = [a + b for a, b in zip(totals, result)]
        count += 1
    assert count, 'no assembled PDFs'
    print(f'PASS: {count} source PDFs ({driver_count} actual driver callbacks), {totals[0]} pages, {totals[1]} structure nodes, {totals[2]} annotations, {unsupported} explicit unsupported inputs, {rejected} tamper rejections')


if __name__ == '__main__':
    main()
