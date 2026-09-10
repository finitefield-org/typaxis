#!/usr/bin/env python3
"""Verify source-preserving Note reading order and table header relations.

Uses original document structure, actual MCID bindings and the sealed page
selection's first-demand owners. This is not a final PDF/UA validator.
"""
import argparse
import copy
import json
from pathlib import Path
from verify_book_v2_source_structure import verify as verify_source


def verify(probe):
    verify_source(probe)
    source = probe['nodes']
    output = probe['relations']
    document = probe['wire']['document']
    keys = {tuple(n['key']): i for i, n in enumerate(source)}
    raw = {}
    refs = []

    def collect(value, origin=None):
        if isinstance(value, list):
            for item in value:
                collect(item, origin)
        elif isinstance(value, dict):
            if 'node_id' in value:
                assert value['node_id'] not in raw
                raw[value['node_id']] = value
            if value.get('kind') == 'footnote_reference':
                refs.append((value['node_id'], value['footnote_id'], origin))
            for name in ('term', 'blocks', 'children', 'items', 'caption', 'head', 'body', 'cells', 'equation_number'):
                if name in value:
                    collect(value[name], origin)

    collect(document['blocks'])
    definitions = document['footnotes']
    definition_by_id = {n['footnote_id']: i for i, n in enumerate(definitions)}
    for i, note in enumerate(definitions):
        collect(note, i)
    painted_nodes = {b['node'] for b in probe['bindings']}
    notes = []
    for d in definitions:
        owner = d['node_id']
        label = keys[owner, 'FootnoteLabel']
        notes.append(dict(node=keys[owner, 'Source'], link=keys[owner, 'FootnoteLink'], label=label,
                          painted=label in painted_nodes, first=None, reading=None, after=None, **{'return': None}))
    edges = []
    per_note = [[] for _ in notes]
    related = [[] for _ in source]
    for i, (owner, target, origin) in enumerate(refs):
        di = definition_by_id[target]
        note = notes[di]
        ref = keys[owner, 'Source']
        label = keys[owner, 'FootnoteLabel']
        painted = label in painted_nodes
        assert not painted or note['painted']
        if painted and origin is not None:
            assert notes[origin]['painted']
        e = dict(reference=ref, note=note['node'], definition=di, source_definition=origin,
                 ref_link=keys[owner, 'FootnoteLink'], ref_label=label,
                 def_link=note['link'], def_label=note['label'], painted=painted, next=None)
        edges.append(e)
        if per_note[di]:
            edges[per_note[di][-1]]['next'] = i
        else:
            note['first'] = i
        if painted and note['return'] is None:
            note['return'] = i
        per_note[di].append(i)
        related[ref].append(note['node'])
        related[note['node']].append(ref)
    assert output['edges'] == edges
    assert len(output['first_demand']) == len(notes)
    seeds = []
    for di, owner in enumerate(output['first_demand']):
        assert (owner is not None) == notes[di]['painted']
        if owner is None:
            seeds.append(None)
        else:
            candidates = [i for i in per_note[di] if source[edges[i]['reference']]['key'][0] == owner and edges[i]['painted']]
            assert len(candidates) == 1
            seeds.append(candidates[0])
    # Every actually demanded definition is rooted in a body reference. A
    # dependency cycle must not be smuggled into the selected first-demand data.
    for di, seed in enumerate(seeds):
        seen = set()
        current = di if seed is not None else None
        while current is not None:
            assert current not in seen
            seen.add(current)
            edge = seeds[current]
            assert edge is not None
            current = edges[edge]['source_definition']

    parents = [n['parent'] for n in source]
    children = [[] for _ in source]
    for i, p in enumerate(parents):
        if p is not None:
            children[p].append(i)

    def anchor(ref):
        branch = ref
        while True:
            parent = source[branch]['parent']
            assert parent is not None
            if (source[parent]['role'] in ('P', 'H1', 'H2', 'H3', 'H4', 'H5', 'H6')
                    or (source[parent]['role'] == 'Lbl' and source[parent]['key'][1] == 'Source')):
                return parent, branch
            branch = parent

    def detach(i):
        children[parents[i]].remove(i)
        parents[i] = None

    def insert(i, p, after):
        assert parents[after] == p and parents[i] is None
        children[p].insert(children[p].index(after) + 1, i)
        parents[i] = p

    for di, seed in enumerate(seeds):
        if seed is not None:
            p, branch = anchor(edges[seed]['reference'])
            notes[di].update(reading=seed, after=branch)
            detach(notes[di]['node'])
            insert(notes[di]['node'], p, branch)
    note_by_node = {n['node']: i for i, n in enumerate(notes)}
    for di, note in enumerate(notes):
        if not note['painted']:
            continue
        selected = None
        for ei in per_note[di]:
            if not edges[ei]['painted']:
                continue
            p, branch = anchor(edges[ei]['reference'])
            ancestors = set()
            cursor = p
            while cursor is not None:
                assert cursor not in ancestors
                ancestors.add(cursor)
                cursor = parents[cursor]
            if note['node'] not in ancestors:
                selected = ei, p, branch
        assert selected is not None
        ei, p, branch = selected
        note.update(reading=ei, after=branch)
        detach(note['node'])
        key = source[edges[ei]['reference']]['key'][0], source[note['node']]['key'][0]
        after = branch
        siblings = children[p]
        for nxt in siblings[siblings.index(branch) + 1:]:
            if nxt not in note_by_node:
                break
            other = notes[note_by_node[nxt]]
            if other['after'] != branch:
                break
            other_key = source[edges[other['reading']]['reference']]['key'][0], source[nxt]['key'][0]
            if other_key > key:
                break
            after = nxt
        insert(note['node'], p, after)
    assert output['notes'] == notes
    order = []
    depths = {}
    stack = [(0, 1)]
    while stack:
        i, depth = stack.pop()
        assert i not in depths
        depths[i] = depth
        order.append(i)
        stack.extend((c, depth + 1) for c in reversed(children[i]))
    assert len(order) == len(source)
    assert output['order'] == order
    assert len(output['nodes']) == len(source)
    header_count = 0
    moved = 0
    for i, (original, node) in enumerate(zip(source, output['nodes'])):
        assert node['parent'] == parents[i]
        moved += parents[i] != original['parent']
        assert node['depth'] == depths[i]
        assert node['first_child'] == (children[i][0] if children[i] else None)
        siblings = children[parents[i]] if parents[i] is not None else [0]
        at = siblings.index(i)
        assert node['next'] == (siblings[at + 1] if at + 1 < len(siblings) else None)
        assert node['related'] == related[i]
        headers = []
        cell = original['cell']
        if cell is not None and not cell['header']:
            columns = set(range(cell['column'], cell['column'] + cell['colspan']))
            for hi, h in enumerate(source):
                h = h['cell']
                if h is not None and h['header'] and h['table'] == cell['table']:
                    if columns.intersection(range(h['column'], h['column'] + h['colspan'])):
                        headers.append(hi)
        assert node['headers'] == headers
        header_count += len(headers)
        expected_numbering = None
        if original['role'] == 'L':
            source_list = raw[original['key'][0]]
            if source_list['kind'] == 'list':
                expected_numbering = 'Decimal' if source_list['ordered'] else 'Disc'
            else:
                assert source_list['kind'] == 'description_list'
                assert 'ordered' not in source_list and 'start' not in source_list
        assert node['list_numbering'] == expected_numbering
    return len(notes), len(edges), header_count, moved, sum(not n['painted'] for n in notes)


def tamper_checks(probe):
    changes = [lambda p: p['relations']['nodes'][0].update(depth=2),
               lambda p: p['relations']['order'].reverse()]
    if probe['relations']['edges']:
        changes.append(lambda p: p['relations']['edges'][0].update(note=0))
        changes.append(lambda p: p['relations']['nodes'][p['relations']['edges'][0]['reference']].update(related=[]))
    note = next((i for i, n in enumerate(probe['relations']['notes']) if n['painted']), None)
    if note is not None:
        changes.append(lambda p: p['relations']['notes'][note].update(after=0))
        changes.append(lambda p: p['relations']['first_demand'].__setitem__(note, None))
    cell = next((i for i, n in enumerate(probe['relations']['nodes']) if n['headers']), None)
    if cell is not None:
        changes.append(lambda p: p['relations']['nodes'][cell].update(headers=[]))
    for change in changes:
        candidate = copy.deepcopy(probe)
        change(candidate)
        try:
            verify(candidate)
        except (AssertionError, KeyError, IndexError, TypeError, ValueError):
            continue
        raise AssertionError('tampered structure relations accepted')
    return len(changes)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.json'))
    assert paths, 'no relation probes'
    totals = [0] * 5
    rejections = 0
    for path in paths:
        probe = json.loads(path.read_text())
        try:
            counts = verify(probe)
            if args.self_test:
                rejections += tamper_checks(probe)
        except Exception as error:
            raise AssertionError(f'{path}: {error}') from error
        totals = [a + b for a, b in zip(totals, counts)]
    print(f'PASS: {len(paths)} probes, {totals[0]} notes, {totals[1]} reference edges, {totals[2]} header relations, {totals[3]} moved notes, {totals[4]} explicit unplaced notes, {rejections} tamper rejections')


if __name__ == '__main__':
    main()
