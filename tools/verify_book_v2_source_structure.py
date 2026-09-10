#!/usr/bin/env python3
"""Independently compare book-2 source topology and actual MCID commands.

These are source-registry probes, not a finished PDF, final note reading order,
annotation/Headers closure, or a PDF/UA acceptance result.
"""
import argparse
import copy
import json
import re
from pathlib import Path
from pypdf.generic import ContentStream, DecodedStreamObject

KINDS = set('result proof exercise solution example counterexample remark note warning common_error formalization_note quote exercise_part choice hint assumption'.split())
ROLES = {'description_list': 'L', 'description_item': 'LI', 'description_term': 'Lbl',
         'paragraph': 'P', 'list': 'L', 'list_item': 'LI', 'table': 'Table',
         'table_row': 'TR', 'figure': 'Figure', 'vector_figure': 'Figure',
         'text': 'Span', 'inline_math': 'Formula', 'display_math': 'Formula',
         'inline_vector': 'Figure', 'math_vector': 'Formula', 'math_vector_block': 'Formula',
         'emphasis': 'Em', 'strong': 'Strong', 'link': 'Link', 'reference': 'Reference',
         'footnote_reference': 'Reference', 'footnote_definition': 'Note', 'equation_number': 'Span'}


def source_nodes(document):
    output = []

    def add(value, kind, parent, inherited, *, slot='Source', role=None, cell=None):
        if kind in ('anchor', 'soft_break', 'hard_break', 'page_break'):
            return None
        language = value.get('language') or inherited
        semantic = value.get('semantic_kind')
        if kind == 'semantic_container':
            assert semantic in KINDS
            role = 'BlockQuote' if semantic == 'quote' else 'Sect'
        if kind == 'heading':
            assert 1 <= value['level'] <= 6
            role = 'H' + str(value['level'])
        span = value.get('span')
        entry = dict(key=[value['node_id'], slot], parent_key=parent,
                     role=role or ROLES[kind], semantic_kind=semantic if kind == 'semantic_container' else None,
                     language=language, span=None if span is None else [span['source_id'], span['start_byte'], span['end_byte']],
                     alt=value.get('speech') if kind in ('inline_math', 'display_math') else value.get('alt'), cell=cell)
        output.append(entry)
        return entry

    def generated(node, slot, role):
        entry = dict(node, key=[node['key'][0], slot], parent_key=node['key'], role=role,
                     semantic_kind=None, alt=None, cell=None)
        output.append(entry)
        return entry

    def walk(values, parent):
        for value in values:
            kind = value['kind']
            entry = add(value, kind, parent['key'], parent['language'])
            if entry is None:
                continue
            if kind == 'description_list':
                assert 'ordered' not in value and 'start' not in value
                assert value['items']
                for item in value['items']:
                    assert item['blocks'] and item['term']['children']
                    assert 'kind' not in item['term']
                    li = add(item, 'description_item', entry['key'], entry['language'])
                    term = add(item['term'], 'description_term', li['key'], li['language'])
                    walk(item['term']['children'], term)
                    body = generated(li, 'ListBody', 'LBody')
                    walk(item['blocks'], body)
            elif kind == 'list':
                for item in value['items']:
                    li = add(item, 'list_item', entry['key'], entry['language'])
                    generated(li, 'ListLabel', 'Lbl')
                    body = generated(li, 'ListBody', 'LBody')
                    walk(item['blocks'], body)
            elif kind == 'table':
                if 'caption' in value:
                    assert isinstance(value['caption'], list) and value['caption']
                    walk(value['caption'], generated(entry, 'Caption', 'Caption'))
                occupied = set()
                ordinal = 0
                for section, slot, role in [('head', 'TableHead', 'THead'), ('body', 'TableBody', 'TBody')]:
                    group = generated(entry, slot, role)
                    section_end = ordinal + len(value[section])
                    for row in value[section]:
                        tr = add(row, 'table_row', group['key'], group['language'])
                        column = 0
                        for cell in row['cells']:
                            while (ordinal, column) in occupied:
                                column += 1
                            assert column + cell['colspan'] <= len(value['columns'])
                            assert ordinal + cell['rowspan'] <= section_end
                            for r in range(ordinal, ordinal + cell['rowspan']):
                                for c in range(column, column + cell['colspan']):
                                    assert (r, c) not in occupied
                                    occupied.add((r, c))
                            attrs = dict(table=value['node_id'], row=ordinal, column=column,
                                         colspan=cell['colspan'], rowspan=cell['rowspan'], header=section == 'head')
                            td = add(cell, 'table_cell', tr['key'], tr['language'], role='TH' if section == 'head' else 'TD', cell=attrs)
                            walk(cell['blocks'], td)
                            column += cell['colspan']
                        ordinal += 1
            elif kind in ('figure', 'vector_figure'):
                if value['caption']:
                    walk(value['caption'], generated(entry, 'Caption', 'Caption'))
            elif kind == 'reference':
                generated(generated(entry, 'ReferenceLink', 'Link'), 'ReferenceLabel', 'Span')
            elif kind == 'footnote_reference':
                generated(generated(entry, 'FootnoteLink', 'Link'), 'FootnoteLabel', 'Lbl')
            elif kind == 'math_vector_block':
                if value.get('equation_number') is not None:
                    add(value['equation_number'], 'equation_number', entry['key'], entry['language'])
            else:
                walk(value.get('children', value.get('blocks', [])), entry)

    root = add(document, 'document', None, document['language'], role='Document')
    walk(document['blocks'], root)
    for note in document['footnotes']:
        entry = add(note, 'footnote_definition', root['key'], root['language'])
        generated(generated(entry, 'FootnoteLink', 'Link'), 'FootnoteLabel', 'Lbl')
        walk(note['blocks'], entry)
    return output


def verify(probe):
    expected = source_nodes(probe['wire']['document'])
    nodes, bindings, groups = probe['nodes'], probe['bindings'], probe['groups']
    assert len(nodes) == len(expected)
    keys = {tuple(n['key']): i for i, n in enumerate(expected)}
    assert len(keys) == len(expected)
    children = [[] for _ in nodes]
    for i, (node, source) in enumerate(zip(nodes, expected)):
        for field in ('key', 'role', 'semantic_kind', 'span', 'alt', 'cell'):
            assert node[field] == source[field], (i, field, node[field], source[field])
        assert node['language'].casefold() == source['language'].casefold()
        parent = None if source['parent_key'] is None else keys[tuple(source['parent_key'])]
        assert node['parent'] == parent
        assert node['depth'] == (1 if parent is None else nodes[parent]['depth'] + 1)
        if parent is not None:
            assert parent < i
            children[parent].append(i)
    for i, wanted in enumerate(children):
        seen = []
        child = nodes[i]['first_child']
        while child is not None:
            assert child not in seen
            seen.append(child)
            child = nodes[child]['next_sibling']
        assert seen == wanted
    by_group = {b['group']: (i, b) for i, b in enumerate(bindings)}
    assert len(by_group) == len(bindings)
    assert [b['group'] for b in bindings] == sorted(by_group)
    by_node = [[] for _ in nodes]
    mcids = {}
    artifacts = 0
    for index, group in enumerate(groups):
        stream = DecodedStreamObject()
        stream.set_data(group['bytes'].encode('utf-8'))
        scopes = [args for args, op in ContentStream(stream, None).operations if op == b'BDC']
        assert scopes
        tag, properties = scopes[0]
        if tag == '/Artifact':
            artifacts += 1
            assert index not in by_group
            assert all('/MCID' not in props and '/ActualText' not in props for _, props in scopes)
            continue
        assert sum('/MCID' in p for _, p in scopes) == 1
        actual_mcid = int(properties['/MCID'])
        page = group['page']
        assert actual_mcid == mcids.get(page, 0)
        mcids[page] = actual_mcid + 1
        binding_index, binding = by_group[index]
        assert (binding['page'], binding['mcid']) == (page, actual_mcid)
        slot = 'Source'
        role = group['role']
        if role == 'Text' and (group['owner'], 'FootnoteLabel') in keys:
            slot = 'FootnoteLabel'
        elif role == 'Text' and (group['owner'], 'ReferenceLabel') in keys:
            slot = 'ReferenceLabel'
        if role == 'Label':
            owner = nodes[keys[group['owner'], 'Source']]
            slot = 'ListLabel' if owner['role'] == 'LI' else 'FootnoteLabel'
        node_index = keys[group['owner'], slot]
        assert binding['node'] == node_index
        expected_roles = {'Text': ('Span', 'Reference', 'Lbl'), 'Label': ('Lbl',), 'Formula': ('Formula',),
                          'Figure': ('Figure',), 'EquationNumber': ('Span',)}
        assert nodes[node_index]['role'] in expected_roles[role]
        by_node[node_index].append(binding_index)
    assert len(bindings) + artifacts == len(groups)
    for i, wanted in enumerate(by_node):
        seen = []
        binding = nodes[i]['first_binding']
        while binding is not None:
            assert binding not in seen
            seen.append(binding)
            binding = bindings[binding]['next']
        assert seen == wanted
    return len(nodes), len(bindings), artifacts


def tamper_checks(probe):
    mutations = [lambda p: p['nodes'][0].update(role='Span'),
                 lambda p: p['nodes'][0].update(language='zz'),
                 lambda p: p['nodes'][1].update(parent=None)]
    if probe['bindings']:
        mutations.append(lambda p: p['bindings'][0].update(mcid=p['bindings'][0]['mcid'] + 1))
        group = probe['bindings'][0]['group']
        def change_command(p):
            value, count = re.subn(r'/MCID (\d+)', lambda m: '/MCID ' + str(int(m[1]) + 1), p['groups'][group]['bytes'], count=1)
            assert count == 1
            p['groups'][group]['bytes'] = value
        mutations.append(change_command)
    alt = next((i for i, n in enumerate(probe['nodes']) if n['alt'] is not None), None)
    if alt is not None:
        mutations.append(lambda p: p['nodes'][alt].update(alt='tampered alternative'))
    cell = next((i for i, n in enumerate(probe['nodes']) if n['cell'] is not None), None)
    if cell is not None:
        mutations.append(lambda p: p['nodes'][cell]['cell'].update(column=p['nodes'][cell]['cell']['column'] + 1))
    terms = [i for i, n in enumerate(probe['nodes'])
             if n['role'] == 'Lbl' and n['key'][1] == 'Source']
    if terms:
        index = terms[0]
        mutations.extend([
            lambda p: p['nodes'][index].update(role='P'),
            lambda p: p['nodes'][index].update(key=[p['nodes'][index]['key'][0], 'ListLabel']),
            lambda p: p['nodes'][index].update(parent=p['nodes'][index]['parent'] - 1),
        ])
    for mutate in mutations:
        candidate = copy.deepcopy(probe)
        mutate(candidate)
        try:
            verify(candidate)
        except (AssertionError, KeyError, IndexError, TypeError):
            continue
        raise AssertionError('tampered source registry accepted')
    return len(mutations)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('--self-test', action='store_true')
    args = parser.parse_args()
    paths = sorted(args.directory.glob('*.json'))
    assert paths, 'no source structure probes'
    totals = [0, 0, 0]
    tampered = 0
    for path in paths:
        probe = json.loads(path.read_text())
        try:
            counts = verify(probe)
            if args.self_test:
                tampered += tamper_checks(probe)
        except Exception as error:
            raise AssertionError(f'{path}: {error}') from error
        totals = [a + b for a, b in zip(totals, counts)]
    print(f'PASS: {len(paths)} source probes, {totals[0]} structure nodes, {totals[1]} actual MCID bindings, {totals[2]} excluded Artifacts, {tampered} tamper rejections')


if __name__ == '__main__':
    main()
