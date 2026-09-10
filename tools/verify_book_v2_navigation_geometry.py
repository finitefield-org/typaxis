#!/usr/bin/env python3
"""Reconstruct navigation from original source and actual selected paint geometry.

Checks the navigation owner, not page object transforms, annotations or PDF/UA.
"""
import argparse
import copy
import json
import re
from pathlib import Path
from verify_book_v2_structure_relations import verify as verify_relations


def union(rectangles):
    rectangles = [r for r in rectangles if r is not None]
    if not rectangles:
        return None
    assert all(r[2] > 0 and r[3] > 0 for r in rectangles)
    left = min(r[0] for r in rectangles)
    top = min(r[1] for r in rectangles)
    return [left, top, max(r[0] + r[2] for r in rectangles) - left,
            max(r[1] + r[3] for r in rectangles) - top]


def verify(probe):
    verify_relations(probe)
    nodes = probe['nodes']
    rel = probe['relations']
    nav = rel['navigation']
    keys = {tuple(n['key']): i for i, n in enumerate(nodes)}
    raw = {}
    parents = {}

    def walk(value, parent=None):
        if isinstance(value, list):
            for item in value:
                walk(item, parent)
        elif isinstance(value, dict):
            if 'node_id' in value:
                assert value['node_id'] not in raw
                raw[value['node_id']] = value
                parents[value['node_id']] = parent
                parent = value['node_id']
            for name in ('term', 'blocks', 'children', 'items', 'caption', 'head', 'body', 'cells', 'equation_number', 'footnotes'):
                if name in value:
                    walk(value[name], parent)

    walk(probe['wire']['document'])
    anchors = nav['source_anchors']
    assert len({name for name, _ in anchors}) == len(anchors)
    source_anchors = {v['anchor_id']: i for i, v in raw.items() if v.get('anchor_id') is not None}
    number_labels = {}
    def descendant(child, owner):
        while child is not None:
            if child == owner:
                return True
            child = parents[child]
        return False
    bindings = probe['wire']['document'].get('number_bindings', [])
    if 'number_bindings' in probe['wire']['document']:
        assert isinstance(bindings, list) and bindings
    for b in bindings:
        assert set(b) == {'anchor_id', 'owner_node_id', 'label_node_id', 'text_span'}
        name, owner, label = b['anchor_id'], b['owner_node_id'], b['label_node_id']
        assert name and name not in number_labels
        target, leaf = raw[owner], raw[label]
        assert target.get('kind') in ('paragraph', 'heading', 'semantic_container', 'figure',
            'vector_figure', 'list', 'description_list', 'table', 'math_vector_block', 'display_math') or (
            target.get('kind') is None and parents[owner] is not None and
            raw[parents[owner]].get('kind') in ('list', 'description_list')) or (
            target.get('kind') is None and parents[owner] is not None and
            raw[parents[owner]].get('term') is target)
        assert descendant(label, owner)
        assert leaf.get('kind') == 'text' or (
            raw[parents[label]].get('kind') == 'math_vector_block' and
            raw[parents[label]].get('equation_number') is leaf)
        original, selected = leaf['text_span'], b['text_span']
        assert selected['text_id'] == original['text_id']
        assert original['start_byte'] <= selected['start_byte'] < selected['end_byte'] <= original['end_byte']
        buffer = probe['wire']['text_buffers'][selected['text_id']]
        assert buffer['text_id'] == selected['text_id']
        text = buffer['utf8'].encode('utf-8')[selected['start_byte']:selected['end_byte']].decode('utf-8')
        assert text.strip() and all(ord(c) >= 32 and not 127 <= ord(c) <= 159 for c in text)
        number_labels[name] = text
        if name in source_anchors:
            assert descendant(source_anchors[name], owner)
        else:
            source_anchors[name] = owner
    assert anchors == [[name, owner] for name, owner in sorted(source_anchors.items())]
    na, nd = len(anchors), len(rel['notes'])
    destinations = [dict(owner=owner, kind=['anchor', name], position=None) for name, owner in anchors]
    destinations += [dict(owner=nodes[n['node']]['key'][0], kind=['footnote', i], position=None)
                     for i, n in enumerate(rel['notes'])]
    destinations += [dict(owner=nodes[e['reference']]['key'][0], kind=['reference', i], position=None)
                     for i, e in enumerate(rel['edges'])]
    names = {name: i for i, (name, _) in enumerate(anchors)}
    links = []
    roots = {}

    def link(node, target):
        assert node not in roots
        roots[node] = len(links)
        links.append(dict(node=node, target=target, first=None))

    for value in raw.values():
        kind = value.get('kind')
        if kind == 'link':
            target = value['target']
            link(keys[value['node_id'], 'Source'], ['uri', target['uri']] if target['kind'] == 'uri'
                 else ['destination', names[target['anchor_id']]])
        elif kind == 'reference':
            link(keys[value['node_id'], 'ReferenceLink'], ['destination', names[value['target']]])
    for edge in rel['edges']:
        link(edge['ref_link'], ['destination', na + edge['definition']])
    for note in rel['notes']:
        if note['return'] is not None:
            link(note['link'], ['destination', na + nd + note['return']])
    active = []
    for i, node in enumerate(nodes):
        parent = node['parent']
        if parent is not None:
            assert parent < i
        inherited = active[parent] if parent is not None else None
        if i in roots and inherited is not None:
            assert nav.get('error') == ['conflicting_links', node['key'][0]]
            return len(destinations), len(links), 0, 0, 0, 1
        active.append(roots.get(i, inherited))
    assert len(nav['raw_groups']) == len(probe['groups'])
    binding = {b['group']: b['node'] for b in probe['bindings']}
    first = [None] * len(nodes)
    def nonpainting(node):
        kind = node.get('kind')
        if kind in ('anchor', 'soft_break', 'hard_break'):
            return True
        if kind == 'text':
            span = node['text_span']
            return span['start_byte'] == span['end_byte']
        if kind in ('paragraph', 'heading', 'emphasis', 'strong', 'link'):
            return all(nonpainting(child) for child in node['children'])
        return False

    for line in nav['nonpainting_lines']:
        paragraph = raw[line['owner']]
        assert paragraph['kind'] in ('paragraph', 'heading')
        if line.get('breaks'):
            def descendants(node):
                return [node['node_id']] + [owner for child in node.get('children', []) for owner in descendants(child)]
            owners = descendants(paragraph)
            assert len(set(line['breaks'])) == len(line['breaks'])
            for owner in line['breaks']:
                assert owner in owners and raw[owner]['kind'] in ('soft_break', 'hard_break')
        else:
            assert nonpainting(paragraph)
        assert line['bounds'][2] > 0 and line['bounds'][3] > 0
        if line['repeated']:
            continue
        cursor = keys[line['owner'], 'Source']
        while cursor is not None:
            if first[cursor] is None:
                first[cursor] = (line['page'], line['fragment'], line['bounds'][:])
            cursor = nodes[cursor]['parent']
    last = [None] * len(links)
    rectangles = []
    if 'error' not in nav:
        assert len(nav['page_counts']) == nav['raw_page_count']
    page_counts = [0] * nav['raw_page_count']
    for gi, group in enumerate(nav['raw_groups']):
        assert group['page'] == probe['groups'][gi]['page']
        if group['artifact']:
            assert gi not in binding
            continue
        assert gi in binding
        rect = union(group['paints'])
        if rect is None:
            continue
        node = binding[gi]
        location = group['page'], group['fragment']
        cursor = node
        while cursor is not None:
            old = first[cursor]
            if old is None or location < old[:2]:
                first[cursor] = (*location, rect[:])
            elif old[:2] == location:
                first[cursor] = (*location, union([old[2], rect]))
            cursor = nodes[cursor]['parent']
        li = active[node]
        if li is None:
            continue
        prior = last[li]
        if prior is not None and (rectangles[prior]['page'], rectangles[prior]['fragment']) == location:
            rectangles[prior]['bounds'] = union([rectangles[prior]['bounds'], rect])
            continue
        index = len(rectangles)
        if prior is not None:
            rectangles[prior]['next'] = index
        else:
            links[li]['first'] = index
        last[li] = index
        rectangles.append(dict(link=li, page=location[0], fragment=location[1], group=gi, bounds=rect, next=None))
        page_counts[location[0]] += 1

    def position(node):
        value = first[node]
        return [value[0], value[1], *value[2][:2]] if value is not None else None

    for d in destinations[:na]:
        key = d['owner'], 'Source'
        if key in keys:
            d['position'] = position(keys[key])
    for a in nav['inline_anchors']:
        if a['repeated']:
            continue
        candidates = [d for d in destinations[:na] if d['owner'] == a['owner']]
        assert len(candidates) == 1
        d = candidates[0]
        assert d['position'] is None
        d['position'] = [a['page'], a['fragment'], a['x'], a['y']]
    for i, note in enumerate(rel['notes']):
        destinations[na + i]['position'] = position(note['node'])
        assert (destinations[na + i]['position'] is not None) == note['painted']
    for i, edge in enumerate(rel['edges']):
        destinations[na + nd + i]['position'] = position(edge['ref_label'])
        assert (destinations[na + nd + i]['position'] is not None) == edge['painted']
    missing = next((destinations[l['target'][1]] for l in links
                    if l['first'] is not None and l['target'][0] == 'destination'
                    and destinations[l['target'][1]]['position'] is None), None)
    if missing is None:
        missing = next((destinations[names[o['target']]] for o in nav['source_outline']
                        if destinations[names[o['target']]]['position'] is None), None)
    if missing is not None:
        assert nav.get('error') == ['unplaced_destination', missing['owner']]
        return len(destinations), len(links), 0, 0, 0, 1
    assert 'error' not in nav
    assert destinations == nav['destinations']
    assert rectangles == nav['rectangles']
    assert page_counts == nav['page_counts']
    assert links == nav['links']
    outline = []
    ids = {}
    children = {None: []}
    for i, source in enumerate(nav['source_outline']):
        assert source['id'] not in ids
        parent = ids[source['parent']] if source['parent'] is not None else None
        ids[source['id']] = i
        target = names[source['target']]
        assert destinations[target]['position'] is not None
        siblings = children[parent]
        previous = siblings[-1] if siblings else None
        if previous is not None:
            outline[previous]['next'] = i
        siblings.append(i)
        children[i] = []
        outline.append(dict(destination=target, parent=parent, first=None, last=None,
                            previous=previous, next=None, descendants=0))
    for i in reversed(range(len(outline))):
        if children[i]:
            outline[i]['first'] = children[i][0]
            outline[i]['last'] = children[i][-1]
            outline[i]['descendants'] = sum(1 + outline[j]['descendants'] for j in children[i])
    assert outline == nav['outline']
    # Source-derived Text/Number labels must also occur in the actual marked bytes,
    # not merely in a navigation or shaping receipt.
    def plain(value):
        if isinstance(value, list):
            return ''.join(plain(v) for v in value)
        kind = value.get('kind')
        if kind == 'text':
            span = value['text_span']
            buf = next(b['utf8'] for b in probe['wire']['text_buffers'] if b['text_id'] == span['text_id'])
            return buf.encode('utf-8')[span['start_byte']:span['end_byte']].decode('utf-8')
        if kind in ('soft_break', 'hard_break'):
            return ' '
        if kind == 'anchor':
            return ''
        assert kind in ('heading', 'emphasis', 'strong', 'link')
        return plain(value['children'])

    for source in raw.values():
        if source.get('kind') != 'reference' or source['format'] not in ('text', 'number'):
            continue
        target = source['target']
        entry = next((o for o in nav['source_outline'] if o['target'] == target), None)
        expected = number_labels[target] if source['format'] == 'number' else (
            entry['label'] if entry is not None else plain(raw[anchors[names[target]][1]]))
        chunks = []
        for gi, group in enumerate(probe['groups']):
            if gi in binding and group['owner'] == source['node_id'] and group['role'] == 'Text':
                tokens = re.findall(r'/ActualText\s*<([0-9a-fA-F]+)>', group['bytes'])
                assert len(tokens) == 1
                chunks.append(bytes.fromhex(tokens[0]).decode('utf-16'))
        if chunks:
            assert ''.join(chunks) == expected
    wrapped = sum(len({r['fragment'] for r in rectangles if r['link'] == i}) > 1 for i in range(len(links)))
    return len(destinations), len(links), len(rectangles), len(outline), wrapped, 0


def tamper_checks(probe):
    n = probe['relations']['navigation']
    if 'error' in n:
        mutated = copy.deepcopy(probe)
        mutated['relations']['navigation']['error'][1] += 1
        try:
            verify(mutated)
        except (AssertionError, KeyError, IndexError, TypeError, ValueError):
            return 1
        raise AssertionError('tampered source rejection accepted')
    actions = [lambda p: p['relations']['navigation']['page_counts'].append(1)]
    if probe['wire']['document'].get('number_bindings'):
        actions += [lambda p: p['wire']['document']['number_bindings'][0].update(owner_node_id=0),
                    lambda p: p['wire']['document']['number_bindings'][0]['text_span'].update(end_byte=999999),
                    lambda p: p['wire']['document']['number_bindings'].append(copy.deepcopy(p['wire']['document']['number_bindings'][0]))]
    if n['rectangles']:
        actions += [lambda p: p['relations']['navigation']['rectangles'][0]['bounds'].__setitem__(0, -999999),
                    lambda p: p['relations']['navigation']['rectangles'][0].update(next=0),
                    lambda p: p['relations']['navigation']['links'][p['relations']['navigation']['rectangles'][0]['link']].update(first=None)]
    placed = next((i for i, d in enumerate(n['destinations']) if d['position'] is not None), None)
    if placed is not None:
        actions.append(lambda p: p['relations']['navigation']['destinations'][placed].update(position=None))
    if n['outline']:
        actions.append(lambda p: p['relations']['navigation']['outline'][0].update(descendants=99999))
    if n['nonpainting_lines']:
        actions.append(lambda p: p['relations']['navigation']['nonpainting_lines'][0]['bounds'].__setitem__(3, 0))
        if n['nonpainting_lines'][0].get('breaks'):
            actions.append(lambda p: p['relations']['navigation']['nonpainting_lines'][0]['breaks'].__setitem__(0, p['relations']['navigation']['nonpainting_lines'][0]['owner']))
    for action in actions:
        mutated = copy.deepcopy(probe)
        action(mutated)
        try:
            verify(mutated)
        except (AssertionError, KeyError, IndexError, TypeError, ValueError):
            continue
        raise AssertionError('tampered navigation accepted')
    return len(actions)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.json'))
    assert paths, 'no navigation probes'
    totals = [0] * 6
    rejected = 0
    for path in paths:
        p = json.loads(path.read_text())
        try:
            values = verify(p)
            if args.self_test:
                rejected += tamper_checks(p)
        except Exception as e:
            raise AssertionError(f'{path}: {e}') from e
        totals = [a + b for a, b in zip(totals, values)]
    print(f'PASS: {len(paths)} probes, {totals[0]} destinations, {totals[1]} source links, {totals[2]} annotation rectangles, {totals[3]} outline entries, {totals[4]} wrapped links, {totals[5]} semantic rejections, {rejected} tamper rejections')


if __name__ == '__main__':
    main()
