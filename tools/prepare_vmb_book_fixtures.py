#!/usr/bin/env python3
"""Compose formula-only authored fixture jobs beneath a new output directory."""
import argparse
import copy
import json
from pathlib import Path

from vmb_book_inputs import canonical_bytes, digest, read_canonical, require
from vmb_book_inputs import verify_fixture_inputs, verify_prepared_job
from vmb_probe_io import contained_bytes


def prepare(fixture_root, output):
    fixture_root, output = Path(fixture_root).resolve(), Path(output).resolve()
    require(not output.is_relative_to(fixture_root) and not fixture_root.is_relative_to(output),
            'generated job must be outside fixture inputs')
    facts = verify_fixture_inputs(fixture_root)
    package = read_canonical(fixture_root,'package-template.json')
    require(not package['document']['blocks'] and not package['document']['footnotes']
            and not package['resources']['images'] and not package['text_buffers'] and not package['sources']
            and not package['outline']['entries'], 'empty authored fixture template')
    output.mkdir(mode=0o700)  # Exclusive output, after complete input verification.

    def copy_input(relative):
        target = output/relative
        target.parent.mkdir(parents=True,exist_ok=True)
        target.write_bytes(contained_bytes(fixture_root,relative))
    for face in package['resources']['font_faces']:
        copy_input(face['uri'])
    cases = {case['case_id']:case for case in facts['cases']}
    source, offset = [],0
    for occurrence in facts['occurrences']:
        ordinal = occurrence['source_ordinal']
        case = cases[occurrence['case_id']]
        tex = contained_bytes(fixture_root,case['source_tex_file'])
        copy_input(case['admitted_svg_file'])
        span = {'source_id':0,'start_byte':offset,'end_byte':offset+len(tex)}
        package['text_buffers'].append({'text_id':ordinal,'utf8':tex.decode('utf-8'),'mappings':[
            {'kind':'identity','source_span':span,'text_range':{'start_byte':0,'end_byte':len(tex)}}]})
        package['resources']['images'].append({'image_id':ordinal,'uri':case['admitted_svg_file'],
            'media_type':'svg-safe-2','expected_sha256':case['admitted_svg_sha256'],
            'vector_provenance':{'engine_id':'vmb-go-math-svg','engine_version':'2.0.0',
                                 'rules_version':'vmb.typaxis-safe-svg/1'}})
        node = {'kind':'math_vector' if case['display'] == 'inline' else 'math_vector_block',
            'node_id':occurrence['node_id'],'image_id':ordinal,'span':span,
            'metrics':copy.deepcopy(case['metrics']),'alt':case['alt'],'actual_text':case['actual_text'],
            'source_tex':{'text_span':{'text_id':ordinal,'start_byte':0,'end_byte':len(tex)}}}
        if case['display'] == 'inline':
            node['spacing'] = copy.deepcopy(case['spacing'])
            block = {'kind':'paragraph','node_id':occurrence['node_id']-1,'span':span,'classes':[],
                     'children':[node]}
        else:
            node.update(classes=[],equation_number=None)
            block = node
        package['document']['blocks'].append(block)
        source.append(tex+b'\n')
        offset += len(tex)+1
    source = b''.join(source)
    (output/'input.tsf').write_bytes(source)
    package['sources'] = [{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(source),'sha256':digest(source)}]
    (output/'document-package.json').write_bytes(canonical_bytes(package))
    result = verify_prepared_job(output,fixture_root)
    (output/'fixture-source-observed.json').write_bytes(canonical_bytes(result))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fixture-root',type=Path,required=True)
    parser.add_argument('--output-root',type=Path,required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.fixture_root,args.output_root),indent=2))
