"""Strict source ledgers for authored VMB engine fixtures.

These ledgers bind the saved engine corpus to authored fixture occurrences. They
do not turn those occurrences into original-book or approved layout goldens.
Only the opaque, explicit M/L/C/Z vocabulary of this engine corpus is accepted.
"""
import copy
import hashlib
import json
import re
from fractions import Fraction
from pathlib import Path
from xml.etree import ElementTree as ET

from vmb_probe_io import contained_bytes, same_json, strict_json


FAMILIES = ('fraction', 'parentheses', 'equivalence', 'negation', 'long')
SIZES = (720896, 786431)
REQUIRED_CASES = frozenset(f'{family}-{display}-{size}' for family in FAMILIES
                         for display in ('inline', 'block') for size in SIZES)
REQUIRED_CLI_TESTS = frozenset({
    'vmb_book_engine_fixtures_check_and_build',
    'vmb_book_engine_fixture_hash_mismatch_is_rejected',
    'vmb_book_chapter_terminal_whitespace_is_admitted',
})
CASE_FIELDS = {'case_id','source_tex_file','source_svg_file','admitted_svg_file',
    'source_svg_sha256','admitted_svg_sha256','metrics','spacing','alt','actual_text',
    'language','expected_path_count','expected_stored_segments','expected_path_work',
    'font_size_raw','display','engine_source_pointer'}
OCCURRENCE_FIELDS = {'node_id','image_id','case_id','placement_kind','source_file',
                     'source_pointer','source_ordinal'}
SHA = re.compile(r'[0-9a-f]{64}\Z')
NUMBER = re.compile(r'-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?\Z')


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def canonical_bytes(value):
    """RFC 8785 ordering for the integer-only fixture-ledger vocabulary, plus LF."""
    def encode(item):
        if item is None or type(item) is bool:
            return json.dumps(item)
        if type(item) is int:
            require(abs(item) <= 2**53-1, 'ledger integer outside exact JSON range')
            return str(item)
        if isinstance(item, str):
            require(not any(0xD800 <= ord(c) <= 0xDFFF for c in item), 'ledger surrogate')
            return json.dumps(item, ensure_ascii=False)
        if isinstance(item, list):
            return '['+','.join(encode(child) for child in item)+']'
        require(isinstance(item, dict) and all(isinstance(key, str) for key in item),
                'ledger supports only objects, arrays, strings, exact integers and literals')
        for key in item:
            require(not any(0xD800 <= ord(c) <= 0xDFFF for c in key), 'ledger surrogate')
        keys = sorted(item, key=lambda key: key.encode('utf-16-be'))
        return '{'+','.join(encode(key)+':'+encode(item[key]) for key in keys)+'}'
    return (encode(value)+'\n').encode('utf-8')


def read_canonical(root, relative):
    data = contained_bytes(root, relative)
    value = strict_json(data)
    require(data == canonical_bytes(value), relative+': canonical JCS+LF required')
    return value


def pointer(document, path):
    require(isinstance(path, str) and (path == '' or path.startswith('/')), 'JSON pointer syntax')
    current = document
    if not path:
        return current
    for token in path[1:].split('/'):
        require(not re.search(r'~(?![01])', token), 'JSON pointer escape')
        token = token.replace('~1','/').replace('~0','~')
        if isinstance(current, list):
            require(bool(re.fullmatch(r'0|[1-9][0-9]*',token)), 'JSON pointer array index')
            require(int(token) < len(current), 'JSON pointer array range')
            current = current[int(token)]
        else:
            require(isinstance(current,dict) and token in current, 'JSON pointer member')
            current = current[token]
    return current


def number(token):
    require(isinstance(token,str) and NUMBER.fullmatch(token), 'engine decimal coordinate')
    return Fraction(token)


def raw(value):
    return round(value*65536)


def path_inventory(svg):
    result = []
    for group in svg:
        tag = group.tag.rsplit('}',1)[-1]
        if tag == 'metadata':
            continue
        require(tag == 'g' and group.attrib == {'fill':'currentColor'}, 'engine opaque path group')
        for child in group:
            require(child.tag.rsplit('}',1)[-1] == 'path' and set(child.attrib) == {'d'}
                    and not len(child), 'engine path vocabulary')
            tokens = child.attrib['d'].split()
            operations, offset = [], 0
            while offset < len(tokens):
                op = tokens[offset]
                require(op in {'M','L','C','Z'}, 'engine explicit path operation')
                arity = {'M':2,'L':2,'C':6,'Z':0}[op]
                values = tokens[offset+1:offset+arity+1]
                require(len(values) == arity, 'engine path operand count')
                operations.append((op,tuple(map(number,values))))
                offset += arity+1
            require(operations and operations[0][0] == 'M'
                    and any(op in ('L','C') for op,_ in operations), 'engine nonempty painted path')
            result.append(operations)
    require(result, 'engine nonempty SVG paths')
    return result


def geometry(source, admitted, case, tex):
    original, converted = ET.fromstring(source), ET.fromstring(admitted)
    require(original.tag.rsplit('}',1)[-1] == converted.tag.rsplit('}',1)[-1] == 'svg', 'engine SVG root')
    require(set(original.attrib) == {'role','aria-label','lang','dir','data-math-display',
        'data-math-baseline','data-math-origin-x','data-math-advance','viewBox','width','height'},
        'engine original root vocabulary')
    require(original.attrib['role'] == 'img' and original.attrib['dir'] == 'ltr'
            and original.attrib['lang'] == case['language'] == 'en'
            and original.attrib['aria-label'] == case['alt']
            and original.attrib['data-math-display'] == case['display'], 'engine source semantics')
    metadata = [child for child in original if child.tag.rsplit('}',1)[-1] == 'metadata']
    require(len(metadata) == 1 and metadata[0].attrib == {'data-format':'vmb-math-metadata-v2'}
            and metadata[0].text == tex and not len(metadata[0]), 'engine source TeX')
    box = tuple(map(number,original.attrib['viewBox'].split()))
    require(len(box) == 4 and box[:2] == (0,0) and min(box[2:]) > 0, 'engine source viewport')
    for dimension, extent in zip(('width','height'),box[2:]):
        text = original.attrib[dimension]
        require(text.endswith('em') and number(text[:-2])*65536 == extent, 'engine source em dimensions')
    require(type(case['font_size_raw']) is int and case['font_size_raw'] in SIZES, 'engine fixture font size')
    scale = Fraction(case['font_size_raw'],65536*65536)
    height = box[3]*scale
    baseline = number(original.attrib['data-math-baseline'])*scale
    metrics = {'advance':raw(number(original.attrib['data-math-advance'])*scale),
        'ascent':raw(baseline),'descent':raw(height-baseline),
        'origin_x':raw(-number(original.attrib['data-math-origin-x'])*scale),'baseline':raw(baseline),
        'viewport':{'width':raw(box[2]*scale),'height':raw(height)}}
    require(same_json(metrics,case['metrics']), 'engine independently converted metrics')
    require(set(converted.attrib) == {'width','height','viewBox'}, 'admitted root vocabulary')
    extents = []
    for dimension in ('width','height'):
        value = converted.attrib[dimension]
        require(value.endswith('pt'), 'admitted physical pt dimensions')
        extents.append(number(value[:-2]))
        require(raw(extents[-1]) == metrics['viewport'][dimension], 'admitted viewport matches metrics')
    require(tuple(map(number,converted.attrib['viewBox'].split())) == (0,0,*extents), 'admitted physical viewBox')
    before, after = path_inventory(original), path_inventory(converted)
    require(len(before) == len(after) == case['expected_path_count'], 'engine source path count')
    segments = 0
    for left,right in zip(before,after):
        require(len(left) == len(right), 'engine source segment count')
        for (op,args),(other,values) in zip(left,right):
            require(op == other and all(abs(raw(a*scale)-raw(b)) <= 1 for a,b in zip(args,values)),
                    'engine source control points within one raw unit')
            segments += 1
    require(type(case['expected_stored_segments']) is int and type(case['expected_path_work']) is int
            and case['expected_stored_segments'] == case['expected_path_work'] == segments+5,
            'engine segment/work charges include the outer clip')
    return {'paths':len(before),'source_segments':segments,'stored_segments':segments+5,'path_work':segments+5}


def verify_fixture_inputs(root):
    root = Path(root)
    index = read_canonical(root,'fixture-index.json')
    require(set(index) == {'algorithm','cases','engine_corpus','files','ledger_scope','case_ids',
        'required_cli_tests','case_ledger','occurrence_ledger','provenance','package_template'},
        'complete fixture index fields')
    require(index['algorithm'] == 'typaxis.vmb-book-fixture/1', 'VMB fixture identity')
    require(index['ledger_scope'] == 'authored-engine-occurrences/1', 'VMB fixture source scope')
    files = index['files']
    require(isinstance(files,list) and files, 'nonempty fixture file inventory')
    names = set()
    for item in files:
        require(set(item) == {'path','byte_length','sha256'} and type(item['byte_length']) is int
                and item['byte_length'] >= 0 and isinstance(item['sha256'],str) and SHA.fullmatch(item['sha256']),
                'typed fixture file record')
        require(item['path'] not in names and item['path'] != 'fixture-index.json', 'unique fixture file inventory')
        data = contained_bytes(root,item['path'])
        require(len(data) == item['byte_length'] and digest(data) == item['sha256'], 'fixture file hash/length: '+item['path'])
        names.add(item['path'])
    actual = {str(path.relative_to(root)) for path in root.rglob('*') if path.is_file() or path.is_symlink()}
    require(actual == names | {'fixture-index.json'}, 'complete fixture file inventory')
    require(set(index['case_ids']) == REQUIRED_CASES and len(index['case_ids']) == len(REQUIRED_CASES), 'required fixture case set')
    require(set(index['required_cli_tests']) == REQUIRED_CLI_TESTS
            and len(index['required_cli_tests']) == len(REQUIRED_CLI_TESTS), 'required CLI test name set')
    require(index['case_ledger'] == 'cases.json' and index['occurrence_ledger'] == 'occurrences.json', 'fixed fixture ledgers')
    require(index['package_template'] == 'package-template.json' and index['package_template'] in names,
            'contained authored package template')
    provenance = index['provenance']
    require(provenance['kind'] == 'authored-test-expressions' and provenance['engine_source']
            and provenance['engine_revision'] == index['engine_corpus']['generator_revision']
            and provenance['license_and_notice_files']
            and all(path in names for path in provenance['license_and_notice_files']), 'fixture source/license provenance')
    engine_path = index['engine_corpus']['index']
    engine = strict_json(contained_bytes(root,engine_path))
    require(engine['algorithm'] == 'vmb.typaxis-engine-fixtures/1'
            and engine['rules_version'] == 'vmb.typaxis-safe-svg/1'
            and len(engine['cases']) == 20, 'fixed source engine corpus')
    cases = read_canonical(root,index['case_ledger'])
    occurrences = read_canonical(root,index['occurrence_ledger'])
    require(isinstance(cases,list) and len(cases) == len(REQUIRED_CASES), 'all required fixture cases')
    require(isinstance(occurrences,list) and len(occurrences) == len(cases), 'all authored fixture occurrences')
    by_id, facts = {}, []
    for case in cases:
        require(isinstance(case,dict) and set(case) == CASE_FIELDS, 'complete fixture case fields')
        case_id = case['case_id']
        require(case_id in REQUIRED_CASES and case_id not in by_id, 'unique required case identity')
        require(case['source_tex_file'] == 'tex/'+case_id+'.tex', 'contained exact TeX fixture path')
        source = pointer(engine,case['engine_source_pointer'])
        require(case_id == f"{source['case_id']}-{source['display']}-{source['font_size_raw']}", 'case engine identity')
        require(case['display'] == source['display'] and case['font_size_raw'] == source['font_size_raw'], 'case engine display/size')
        require(case['source_svg_file'] == 'engine-v2/'+source['source_svg']
                and case['admitted_svg_file'] == 'engine-v2/'+source['derived_svg'], 'source engine SVG paths')
        original = contained_bytes(root,case['source_svg_file'])
        admitted = contained_bytes(root,case['admitted_svg_file'])
        tex = contained_bytes(root,case['source_tex_file']).decode('utf-8')
        require(tex == source['tex'] and case['alt'] == source['speech'] and case['alt'] not in ('','数式')
                and case['actual_text'] is None and case['language'] == 'en', 'authored fixture semantic source')
        require(digest(original) == case['source_svg_sha256'] == source['source_sha256']
                and digest(admitted) == case['admitted_svg_sha256'] == source['derived_sha256'], 'case source/derived SHA')
        artifact = source['engine_artifact']
        require(artifact['EngineID'] == 'vmb-go-math-svg' and artifact['EngineVersion'] == '2.0.0'
                and artifact['SVG_SHA256'] == 'sha256:'+digest(original), 'real engine source artifact')
        require(same_json(case['metrics'],source['metrics']) and case['expected_path_count'] == source['path_count']
                and type(case['expected_path_count']) is int and case['expected_path_count'] > 0, 'case engine facts')
        require(same_json(case['spacing'],{'before':0,'after':0}), 'authored formula-only spacing')
        facts.append(geometry(original,admitted,case,tex))
        by_id[case_id] = case
    require(set(by_id) == REQUIRED_CASES, 'required case coverage')
    seen_nodes, seen_images, seen_cases = set(),set(),set()
    next_node = 1
    for ordinal,item in enumerate(occurrences):
        require(isinstance(item,dict) and set(item) == OCCURRENCE_FIELDS, 'complete occurrence fields')
        require(all(type(item[key]) is int and item[key] >= 0 for key in ('node_id','image_id','source_ordinal')),
                'typed occurrence identities')
        require(item['node_id'] not in seen_nodes and item['image_id'] not in seen_images
                and item['case_id'] not in seen_cases, 'unique authored occurrence joins')
        require(item['image_id'] == item['source_ordinal'] == ordinal, 'dense source occurrence order')
        require(item['case_id'] in by_id, 'known occurrence case')
        case = by_id[item['case_id']]
        expected_node = next_node+1 if case['display'] == 'inline' else next_node
        require(item['node_id'] == expected_node, 'authored dense preorder node identities')
        next_node = expected_node+1
        require(item['source_file'] == engine_path and item['source_pointer'] == case['engine_source_pointer']
                and same_json(pointer(engine,item['source_pointer']),engine['cases'][ordinal]), 'source pointer/ordinal join')
        require(item['placement_kind'] == case['display'], 'occurrence placement kind')
        seen_nodes.add(item['node_id']);seen_images.add(item['image_id']);seen_cases.add(item['case_id'])
    return {'algorithm':'typaxis.vmb-authored-fixture-inputs/1','cases':cases,'occurrences':occurrences,
            'geometry':facts,'index_sha256':digest(contained_bytes(root,'fixture-index.json')),
            'files':len(files),'required_cli_tests':sorted(REQUIRED_CLI_TESTS),
            'scope':'Authored engine fixtures; original full-book and approved layout gates remain separate.'}


def verify_prepared_job(job, fixture_root):
    facts = verify_fixture_inputs(fixture_root)
    package = read_canonical(job,'document-package.json')
    template = read_canonical(fixture_root,'package-template.json')
    static = copy.deepcopy(package)
    static['document']['blocks'] = []
    static['resources']['images'] = []
    static['text_buffers'] = []
    static['sources'] = []
    require(same_json(static,template), 'authored template/config/font declarations are unchanged')
    require(package['contract'] == 'typaxis.contract/1.4'
            and package['coordinate_unit'] == 'pdf_point_1_65536', 'fixture package contract')
    require(not package['document']['footnotes'] and not package['outline']['entries'], 'formula-only fixture scope')
    images = package['resources']['images']
    require(len(images) == len(facts['cases']), 'all fixture image declarations')
    for field in ('font_faces','images'):
        for item in package['resources'][field]:
            data = contained_bytes(job,item['uri'])
            require(digest(data) == item['expected_sha256'], 'prepared resource hash')
    require(len(package['sources']) == 1, 'one authored source file')
    source = package['sources'][0]
    data = contained_bytes(job,source['uri'])
    require(type(source['source_id']) is int and type(source['utf8_byte_length']) is int
            and source['source_id'] == 0 and source['utf8_byte_length'] == len(data)
            and source['sha256'] == digest(data), 'prepared authored source bytes')
    buffers = package['text_buffers']
    require(len(buffers) == len(facts['cases']), 'one exact TeX buffer per occurrence')
    require(len(package['document']['blocks']) == len(facts['cases']), 'all fixture blocks')
    by_id = {case['case_id']:case for case in facts['cases']}
    offset, seen = 0, set()
    for ordinal,(occurrence,block) in enumerate(zip(facts['occurrences'],package['document']['blocks'])):
        case = by_id[occurrence['case_id']]
        tex = contained_bytes(fixture_root,case['source_tex_file'])
        span = {'source_id':0,'start_byte':offset,'end_byte':offset+len(tex)}
        if occurrence['placement_kind'] == 'inline':
            require(set(block) == {'kind','node_id','span','classes','children'} and block['classes'] == []
                    and block['kind'] == 'paragraph' and type(block['node_id']) is int
                    and block['node_id'] == occurrence['node_id']-1
                    and len(block['children']) == 1 and same_json(block['span'],span), 'authored inline paragraph')
            node = block['children'][0]
            require(set(node) == {'kind','node_id','image_id','span','metrics','alt','actual_text','source_tex','spacing'}
                    and node['kind'] == 'math_vector' and same_json(node['spacing'],case['spacing']), 'authored inline spacing')
            require(block['node_id'] not in seen, 'unique paragraph node')
            seen.add(block['node_id'])
        else:
            node = block
            require(set(node) == {'kind','node_id','image_id','span','metrics','alt','actual_text','source_tex','classes','equation_number'}
                    and node['classes'] == [] and node['kind'] == 'math_vector_block'
                    and node['equation_number'] is None, 'authored block formula')
        require(node['node_id'] == occurrence['node_id'] and type(node['node_id']) is int
                and node['node_id'] not in seen and node['image_id'] == ordinal
                and type(node['image_id']) is int, 'authored node/image identity')
        seen.add(node['node_id'])
        require(same_json(node['span'],span) and same_json(node['metrics'],case['metrics'])
                and node['alt'] == case['alt'] and node['actual_text'] is None, 'authored occurrence metrics/semantics/span')
        expected_buffer = {'text_id':ordinal,'utf8':tex.decode('utf-8'),'mappings':[
            {'kind':'identity','source_span':span,'text_range':{'start_byte':0,'end_byte':len(tex)}}]}
        require(same_json(buffers[ordinal],expected_buffer), 'authored TeX buffer and source mapping')
        require(same_json(node['source_tex'],{'text_span':{'text_id':ordinal,'start_byte':0,'end_byte':len(tex)}}),
                'authored TeX span')
        image = images[ordinal]
        require(type(image['image_id']) is int and image['image_id'] == ordinal
                and image['media_type'] == 'svg-safe-2' and image['uri'] == case['admitted_svg_file']
                and image['expected_sha256'] == case['admitted_svg_sha256'], 'authored image declaration')
        require(same_json(image['vector_provenance'],{'engine_id':'vmb-go-math-svg','engine_version':'2.0.0',
                                                    'rules_version':'vmb.typaxis-safe-svg/1'}),
                'authored engine image provenance')
        require(data[offset:offset+len(tex)+1] == tex+b'\n', 'authored source occurrence bytes')
        offset += len(tex)+1
    require(offset == len(data), 'complete authored source occurrence inventory')
    return {'algorithm':'typaxis.vmb-authored-fixture-job/1','cases':len(facts['cases']),
            'occurrences':len(facts['occurrences']),'source_bytes':len(data),
            'fixture_index_sha256':facts['index_sha256'],
            'package_sha256':digest(contained_bytes(job,'document-package.json'))}


def verify_test_names(listing, output, required=REQUIRED_CLI_TESTS):
    """Require real discovery, one successful execution per name, and a nonzero summary."""
    require(set(required) == REQUIRED_CLI_TESTS, 'fixed required CLI tests')
    listed = re.findall(r'(?m)^([A-Za-z0-9_:]+): test$',listing)
    require(len(listed) == len(set(listed)), 'duplicate listed CLI tests')
    require(set(name for name in listed if name.startswith('vmb_book_')) == set(required), 'discovered CLI test names')
    executed = re.findall(r'(?m)^test ([A-Za-z0-9_:]+) \.\.\. ok$',output)
    require(len(executed) == len(set(executed)) and set(executed) == set(required), 'executed CLI test names')
    summaries = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;',output)
    require(len(summaries) == 1 and tuple(map(int,summaries[0][:4])) == (len(required),0,0,0), 'nonzero complete CLI test summary')
    return sorted(executed)
