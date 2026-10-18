"""Source-bound fraction-rule and diagonal-negation controls for VMB books.

The additional eight authored cases are separate from the original twenty-case
small corpus. A diagonal overlay must be a distinct source contour; the old
logical-negation glyph and a precomposed not-equal outline cannot replace it.
"""
import copy
import re
from decimal import Decimal
from pathlib import Path
from xml.etree import ElementTree as ET

from pypdf import PdfReader, PdfWriter
from pypdf.generic import ContentStream

from vmb_book_inputs import (CASE_FIELDS, OCCURRENCE_FIELDS, canonical_bytes, digest, geometry,
    path_inventory, pointer, read_canonical, require, verify_fixture_inputs)
from vmb_probe_io import contained_bytes, same_json, strict_json


TEX = {'fraction-rule':(r'\frac{1}{2}','one half'),
       'negation-slash':(r'a\not{=}b','a is not equal to b')}
ENGINE_INDEX = 'thin-strokes/engine/fixture-index.json'
VARIANTS = [(family,mode,size) for family in TEX for mode in ('inline','block') for size in (720896,786431)]
CASE_IDS = [f'{family}-{mode}-{size}' for family,mode,size in VARIANTS]
KINDS = ('deleted','duplicated','clipped')


def source_inputs(root):
    primary = verify_fixture_inputs(root)
    index = read_canonical(root,'fixture-index.json')
    require(index['provenance'].get('additional_corpora') == [{'id':'thin-strokes-1',
        'case_ledger':'thin-strokes/cases.json','occurrence_ledger':'thin-strokes/occurrences.json',
        'engine_index':ENGINE_INDEX,'provenance':'thin-strokes/provenance.json','cases':8}],
        'registered complete additional thin-stroke corpus')
    original_engine = strict_json(contained_bytes(root,index['engine_corpus']['index']))['cases'][0]['engine_artifact']
    identity = {key:value for key,value in original_engine.items() if key not in ('RequestSHA256','SVG_SHA256')}
    corpus = strict_json(contained_bytes(root,ENGINE_INDEX))
    require(corpus['algorithm'] == 'vmb.typaxis-engine-fixtures/1'
            and corpus['rules_version'] == 'vmb.typaxis-safe-svg/1'
            and len(corpus['cases']) == 8, 'fixed eight-case thin-stroke engine corpus')
    require([(case['case_id'],case['display'],case['font_size_raw']) for case in corpus['cases']] == VARIANTS,
            'complete ordered actual-engine thin-stroke requests')
    provenance = read_canonical(root,'thin-strokes/provenance.json')
    require(provenance['algorithm'] == 'typaxis.vmb-thin-stroke-provenance/1'
            and provenance['kind'] == 'authored-test-expressions'
            and provenance['base_vmb_commit'] == '9efff4a509e641d17cf7be9a3ee0c9b47ad04488'
            and provenance['engine_index'] == ENGINE_INDEX
            and provenance['engine_index_sha256'] == digest(contained_bytes(root,ENGINE_INDEX))
            and provenance['approved_layout_golden'] is False, 'explicit overlay/engine/source provenance')
    for name in ('original_source','overlay_source'):
        record = provenance[name]
        require(digest(contained_bytes(root,record['path'])) == record['sha256'], 'retained generator source '+name)
    require(isinstance(provenance['generator_binary_sha256'],str)
            and re.fullmatch('[0-9a-f]{64}',provenance['generator_binary_sha256']) is not None
            and provenance['license_and_notice_files'], 'thin-stroke binary and license provenance')
    for path in provenance['license_and_notice_files']:
        require(bool(contained_bytes(root,path)), 'thin-stroke source/license notice')
    cases = read_canonical(root,'thin-strokes/cases.json')
    occurrences = read_canonical(root,'thin-strokes/occurrences.json')
    require(isinstance(cases,list) and len(cases) == 8 and [row['case_id'] for row in cases] == CASE_IDS,
            'complete ordered thin-stroke case ledger')
    require(isinstance(occurrences,list) and len(occurrences) == 8, 'complete thin-stroke occurrence ledger')
    next_node,facts = 1,[]
    for ordinal,(case,occurrence,source) in enumerate(zip(cases,occurrences,corpus['cases'])):
        require(set(case) == CASE_FIELDS | {'thin_path_index','thin_path_kind'}, 'complete thin-stroke case fields')
        family,mode,size = VARIANTS[ordinal]
        tex,speech = TEX[family]
        require(source['tex'] == tex and source['speech'] == speech and case['alt'] == speech
                and case['actual_text'] is None and case['language'] == 'en'
                and case['display'] == mode and type(case['font_size_raw']) is int and case['font_size_raw'] == size,
                'authored thin-stroke source and semantics')
        require(case['engine_source_pointer'] == '/cases/'+str(ordinal)
                and same_json(pointer(corpus,case['engine_source_pointer']),source), 'thin-stroke engine pointer')
        require(case['source_svg_file'] == 'thin-strokes/engine/'+source['source_svg']
                and case['admitted_svg_file'] == 'thin-strokes/engine/'+source['derived_svg']
                and case['source_tex_file'] == 'thin-strokes/tex/'+case['case_id']+'.tex', 'fixed thin-stroke source paths')
        raw,admitted = contained_bytes(root,case['source_svg_file']),contained_bytes(root,case['admitted_svg_file'])
        require(contained_bytes(root,case['source_tex_file']) == tex.encode()
                and digest(raw) == case['source_svg_sha256'] == source['source_sha256']
                and digest(admitted) == case['admitted_svg_sha256'] == source['derived_sha256'], 'thin-stroke source hashes')
        artifact = source['engine_artifact']
        require(artifact['EngineID'] == 'vmb-go-math-svg' and artifact['EngineVersion'] == '2.0.0'
                and artifact['SVG_SHA256'] == 'sha256:'+digest(raw)
                and same_json({key:value for key,value in artifact.items() if key not in ('RequestSHA256','SVG_SHA256')},identity)
                and isinstance(artifact.get('RequestSHA256'),str)
                and re.fullmatch('sha256:[0-9a-f]{64}',artifact['RequestSHA256']) is not None,
                'actual fixed thin-stroke engine artifact')
        require(same_json(case['metrics'],source['metrics']) and type(case['expected_path_count']) is int
                and case['expected_path_count'] == source['path_count'] == (3 if family == 'fraction-rule' else 4)
                and same_json(case['spacing'],{'before':0,'after':0}), 'thin-stroke source metrics/counts/spacing')
        counts = geometry(raw,admitted,case,tex)
        require(type(case['thin_path_index']) is int and case['thin_path_index'] == 2
                and case['thin_path_kind'] == family, 'actual thin source contribution identity')
        paths = path_inventory(ET.fromstring(raw))
        stroke = paths[2]
        xs = [value for _,args in stroke for value in args[::2]]
        ys = [value for _,args in stroke for value in args[1::2]]
        width,height = max(xs)-min(xs),max(ys)-min(ys)
        if family == 'fraction-rule':
            require([op for op,_ in stroke] == ['M','L','L','L','Z'] and width > height > 0,
                    'actual rectangular fraction rule')
        else:
            require(sum(op == 'M' for op,_ in stroke) == sum(op == 'Z' for op,_ in stroke) == 1
                    and height > 2*width > 0, 'separate diagonal contour, not an equality/negation glyph')
            previous = None
            long_diagonals = 0
            for op,args in stroke:
                if op == 'L' and previous is not None:
                    dx,dy = args[0]-previous[0],args[1]-previous[1]
                    long_diagonals += int(dx*dy < 0 and 4*abs(dy) > 3*height)
                if args:
                    previous = args[-2:]
            require(long_diagonals == 2, 'two long edges of the real negation slash')
        require(set(occurrence) == OCCURRENCE_FIELDS
                and all(type(occurrence[key]) is int for key in ('node_id','image_id','source_ordinal')),
                'typed complete thin-stroke occurrence')
        owner = next_node+int(mode == 'inline')
        require(occurrence['node_id'] == owner and occurrence['image_id'] == occurrence['source_ordinal'] == ordinal
                and occurrence['case_id'] == case['case_id'] and occurrence['placement_kind'] == mode
                and occurrence['source_file'] == ENGINE_INDEX and occurrence['source_pointer'] == '/cases/'+str(ordinal),
                'dense source-bound thin-stroke occurrences')
        next_node = owner+1
        facts.append(counts)
    return {'cases':cases,'occurrences':occurrences,'engine_cases':corpus['cases'],'geometry':facts,
            'fixture_index_sha256':primary['index_sha256'],'engine_index_sha256':digest(contained_bytes(root,ENGINE_INDEX))}


def prepare(root, output):
    root,output = Path(root).resolve(),Path(output).resolve()
    require(not output.is_relative_to(root) and not root.is_relative_to(output), 'thin-stroke output outside source inputs')
    facts = source_inputs(root)
    package = read_canonical(root,'package-template.json')
    output.mkdir()
    def copy_input(relative):
        path = output/relative
        path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(contained_bytes(root,relative))
    for face in package['resources']['font_faces']:
        copy_input(face['uri'])
    offset,source = 0,[]
    for ordinal,(case,occurrence) in enumerate(zip(facts['cases'],facts['occurrences'])):
        tex = contained_bytes(root,case['source_tex_file'])
        span = {'source_id':0,'start_byte':offset,'end_byte':offset+len(tex)}
        package['text_buffers'].append({'text_id':ordinal,'utf8':tex.decode(),'mappings':[
            {'kind':'identity','source_span':span,'text_range':{'start_byte':0,'end_byte':len(tex)}}]})
        uri = case['admitted_svg_file']
        copy_input(uri)
        package['resources']['images'].append({'image_id':ordinal,'uri':uri,'media_type':'svg-safe-2',
            'expected_sha256':case['admitted_svg_sha256'],'vector_provenance':{'engine_id':'vmb-go-math-svg',
                'engine_version':'2.0.0','rules_version':'vmb.typaxis-safe-svg/1'}})
        node = {'kind':'math_vector' if case['display'] == 'inline' else 'math_vector_block',
            'node_id':occurrence['node_id'],'image_id':ordinal,'span':span,'metrics':copy.deepcopy(case['metrics']),
            'alt':case['alt'],'actual_text':None,
            'source_tex':{'text_span':{'text_id':ordinal,'start_byte':0,'end_byte':len(tex)}}}
        if case['display'] == 'inline':
            node['spacing'] = {'before':0,'after':0}
            block = {'kind':'paragraph','node_id':occurrence['node_id']-1,'span':span,'classes':[],'children':[node]}
        else:
            node.update(classes=[],equation_number=None)
            block = node
        package['document']['blocks'].append(block)
        source.append(tex+b'\n')
        offset += len(tex)+1
    data = b''.join(source)
    (output/'input.tsf').write_bytes(data)
    package['sources'] = [{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(data),'sha256':digest(data)}]
    (output/'document-package.json').write_bytes(canonical_bytes(package))
    return verify_job(root,output)


def verify_job(root, job):
    facts = source_inputs(root)
    package = read_canonical(job,'document-package.json')
    static = copy.deepcopy(package)
    static['document']['blocks'] = []
    static['resources']['images'] = []
    static['text_buffers'] = []
    static['sources'] = []
    require(same_json(static,read_canonical(root,'package-template.json')), 'unchanged thin-stroke template/fonts/page settings')
    require(len(package['document']['blocks']) == len(package['resources']['images'])
            == len(package['text_buffers']) == 8 and len(package['sources']) == 1, 'complete prepared thin-stroke inventory')
    for face in package['resources']['font_faces']:
        require(digest(contained_bytes(job,face['uri'])) == face['expected_sha256'], 'thin-stroke fixed font bytes')
    source = package['sources'][0]
    data = contained_bytes(job,'input.tsf')
    require(same_json(source,{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(data),'sha256':digest(data)}),
            'prepared thin-stroke source file')
    offset = 0
    for ordinal,(case,occurrence,block) in enumerate(zip(facts['cases'],facts['occurrences'],package['document']['blocks'])):
        tex = contained_bytes(root,case['source_tex_file'])
        span = {'source_id':0,'start_byte':offset,'end_byte':offset+len(tex)}
        if case['display'] == 'inline':
            require(set(block) == {'kind','node_id','span','classes','children'} and block['kind'] == 'paragraph'
                    and type(block['node_id']) is int and block['node_id'] == occurrence['node_id']-1
                    and block['classes'] == [] and same_json(block['span'],span) and len(block['children']) == 1,
                    'source-authored thin-stroke paragraph')
            node = block['children'][0]
            require(set(node) == {'kind','node_id','image_id','span','metrics','alt','actual_text','source_tex','spacing'}
                    and node['kind'] == 'math_vector' and same_json(node['spacing'],case['spacing']), 'thin-stroke inline spacing')
        else:
            node = block
            require(set(node) == {'kind','node_id','image_id','span','metrics','alt','actual_text','source_tex','classes','equation_number'}
                    and node['kind'] == 'math_vector_block' and node['classes'] == [] and node['equation_number'] is None,
                    'source-authored thin-stroke block formula')
        require(type(node['node_id']) is type(node['image_id']) is int and node['node_id'] == occurrence['node_id']
                and node['image_id'] == ordinal and same_json(node['span'],span) and same_json(node['metrics'],case['metrics'])
                and node['alt'] == case['alt'] and node['actual_text'] is None, 'prepared thin-stroke source owner/facts')
        require(same_json(node['source_tex'],{'text_span':{'text_id':ordinal,'start_byte':0,'end_byte':len(tex)}})
                and same_json(package['text_buffers'][ordinal],{'text_id':ordinal,'utf8':tex.decode(),'mappings':[
                    {'kind':'identity','source_span':span,'text_range':{'start_byte':0,'end_byte':len(tex)}}]}),
                'exact prepared thin-stroke TeX source spans')
        require(data[offset:offset+len(tex)+1] == tex+b'\n', 'exact thin-stroke source occurrence bytes')
        image = package['resources']['images'][ordinal]
        require(same_json(image,{'image_id':ordinal,'uri':case['admitted_svg_file'],'media_type':'svg-safe-2',
            'expected_sha256':case['admitted_svg_sha256'],'vector_provenance':{'engine_id':'vmb-go-math-svg',
                'engine_version':'2.0.0','rules_version':'vmb.typaxis-safe-svg/1'}})
            and digest(contained_bytes(job,image['uri'])) == image['expected_sha256'], 'prepared thin-stroke resource')
        offset += len(tex)+1
    require(offset == len(data), 'all prepared thin-stroke source bytes consumed')
    return {'algorithm':'typaxis.vmb-thin-stroke-job/1','cases':8,'placements':8,
            'fixture_index_sha256':facts['fixture_index_sha256'],'engine_index_sha256':facts['engine_index_sha256'],
            'package_sha256':digest(contained_bytes(job,'document-package.json'))}


def verify_pdf(root, job, pdf, placements=None, *, tools):
    require(placements is None or all(not placements.resolve().is_relative_to(Path(path).resolve())
            for path in (root,job)), 'thin-stroke placement evidence outside all source inputs')
    verify_job(root,job)
    from verify_vmb_scale_pdf import occurrences,verify_source_pdf
    from vmb_book_pdf import verify_source_structure
    package = strict_json(contained_bytes(job,'document-package.json'))
    inventory = {'case':'actual-engine-thin-strokes','resources':8,'placements':8,
        'distinct_resources':len({image['expected_sha256'] for image in package['resources']['images']})}
    paint = verify_source_pdf(job,pdf,package,inventory,list(occurrences(package['document'])),placements,
        extraction_tools={name:tools[name] for name in ('mutool','pdftotext')},
        include_structure_bindings=True,strict_extractor_output=True,scope='actual-engine-thin-strokes')
    graph = verify_source_structure(pdf,package)
    require(paint['source_mcr_bindings'] == graph['source_mcr_bindings'], 'thin-stroke paint and source graph join')
    return {'paint':paint,'structure':graph}


def render(root, job, pdf, placements, output, *, tools):
    require(all(not output.resolve().is_relative_to(Path(path).resolve()) for path in (root,job)),
            'thin-stroke render output outside all source inputs')
    from verify_vmb_scale_render import original_svg,render_source_pages
    facts = source_inputs(root)
    originals,counts = [],[]
    for case,engine in zip(facts['cases'],facts['engine_cases']):
        source,count = original_svg(contained_bytes(root,case['source_svg_file']),contained_bytes(root,case['admitted_svg_file']),
            engine,expected_font_size_raw=case['font_size_raw'])
        originals.append(source)
        counts.append(count)
    inventory = {'case':'actual-engine-thin-strokes','resources':8,'placements':8,
        'distinct_resources':len({case['admitted_svg_sha256'] for case in facts['cases']})}
    return render_source_pages(job,pdf,placements,output,{'cases':facts['engine_cases']},inventory,originals,counts,
        facts['engine_index_sha256'],tool_paths={name:tools[name] for name in ('mutool','pdftoppm','rsvg-convert')})


def form_for(reader, placements, ordinal):
    row = placements['placements'][ordinal]
    page = reader.pages[row['page_index']]
    calls = [args[0] for args,op in ContentStream(page.get_contents(),reader).operations if op == b'Do']
    local = [item for item in placements['placements'] if item['page_index'] == row['page_index']].index(row)
    return page['/Resources']['/XObject'][calls[local]].get_object()


def damage(data, kind, path_index, cut, height, path_count):
    """Change only the selected contour; retain every other byte/operand."""
    fills = list(re.finditer(rb'(?m)^f\r?$',data))
    require(len(fills) == path_count and type(path_index) is int and 0 <= path_index < len(fills),
            'source-complete thin-stroke fill inventory')
    end = fills[path_index].end()
    previous = fills[path_index-1].end() if path_index else 0
    lines = list(re.finditer(rb'(?m)^([^\r\n]+)\r?$',data[previous:fills[path_index].start()]))
    drawing = [line for line in lines if line.group(1).split()[-1] in (b'm',b'l',b'c',b'h')]
    require(drawing and drawing[0].group(1).endswith(b' m'), 'complete nonempty thin-stroke contour')
    start = previous+drawing[0].start()
    if kind == 'deleted':
        return data[:fills[path_index].start()]+b'n'+data[end:]
    if kind == 'duplicated':
        same_path = b'\n'.join(line.group(1) for line in drawing)+b'\nf'
        return data[:end]+b'\n'+same_path+data[end:]
    require(kind == 'clipped' and cut > 0 and height > 0, 'fixed thin-stroke clipping control')
    return data[:start]+f'q\n0 0 {cut} {height} re W n\n'.encode()+data[start:end]+b'\nQ'+data[end:]


def damage_controls(root, job, public_pdf, placements, output, *, tools):
    require(all(not output.resolve().is_relative_to(Path(path).resolve()) for path in (root,job)),
            'thin-stroke damage output outside all source inputs')
    from verify_vmb_scale_render import precise_matrices
    from vmb_book_pdf import verify_source_structure
    facts = source_inputs(root)
    verify_job(root,job)
    package = strict_json(contained_bytes(job,'document-package.json'))
    original_sha = digest(public_pdf.read_bytes())
    require(placements['pdf_sha256'] == original_sha
            and placements['source_package_sha256'] == digest(contained_bytes(job,'document-package.json')),
            'thin-stroke controls bind the original checked PDF/package')
    output.mkdir()
    def save(path, value):
        with path.open('xb') as target:
            target.write(canonical_bytes(value))
    clone = output/'untampered-clone.pdf'
    PdfWriter(clone_from=public_pdf).write(clone)
    save(output/'untampered-clone-observed.json',verify_pdf(root,job,clone,tools=tools))
    source_reader = PdfReader(public_pdf,strict=True)
    before_forms = [digest(form_for(source_reader,placements,i).get_data()) for i in range(8)]
    matrices = precise_matrices(public_pdf)
    controls = []
    for ordinal,case in enumerate(facts['cases']):
        path = path_inventory(ET.fromstring(contained_bytes(root,case['admitted_svg_file'])))[case['thin_path_index']]
        xs = [value for _,args in path for value in args[::2]]
        cut = Decimal(round((min(xs)+max(xs))*65536/2))/65536
        height = Decimal(case['metrics']['viewport']['height'])/65536
        for kind in KINDS:
            name = f'{ordinal:02d}-{case["thin_path_kind"]}-{kind}'
            writer = PdfWriter(clone_from=public_pdf)
            form = form_for(writer,placements,ordinal)
            before = form.get_data()
            after = damage(before,kind,case['thin_path_index'],cut,height,case['expected_path_count'])
            require(before != after, 'actual target contour changes')
            form.set_data(after)
            pdf = output/(name+'.pdf')
            writer.write(pdf)
            try:
                verify_pdf(root,job,pdf,tools=tools)
            except ValueError as error:
                reason = str(error)
                require(reason.startswith('geometry '), 'damaged stroke rejected by source geometry')
            else:
                raise ValueError('damaged thin stroke accepted by the geometry verifier')
            reader = PdfReader(pdf,strict=True)
            after_forms = [digest(form_for(reader,placements,i).get_data()) for i in range(8)]
            changed = {i for i in range(8) if before_forms[i] != after_forms[i]}
            aliases = {i for i,other in enumerate(facts['cases'])
                       if other['admitted_svg_sha256'] == case['admitted_svg_sha256']}
            require(changed == aliases and ordinal in changed, 'only the selected shared Form changes')
            require(precise_matrices(pdf) == matrices, 'all original thin-stroke placement matrices preserved')
            graph = verify_source_structure(pdf,package)
            require(graph['source_occurrences'] == 8, 'complete source graph survives the geometry damage')
            private_placements = copy.deepcopy(placements)
            private_placements['pdf_sha256'] = digest(pdf.read_bytes())
            directory = output/(name+'-render')
            try:
                render(root,job,pdf,private_placements,directory,tools=tools)
            except ValueError:
                require((directory/'summary.json').is_file(), 'complete expected negative raster summary')
            summary = strict_json((directory/'summary.json').read_bytes())
            comparisons = strict_json((directory/'comparisons.json').read_bytes())
            failed = [row for row in comparisons if not row['passed']]
            require(summary['comparisons'] == len(comparisons) == 48
                    and summary['failed_comparisons'] == len(failed), 'every negative source mask combination measured')
            require(all(row['ordinal'] in changed for row in failed), 'unrelated formula masks remain valid')
            require(all(command['exit_code'] == 0 and command['stdout_bytes'] == command['stderr_bytes'] == 0
                        for command in summary['commands']), 'complete warning-free negative raster commands')
            target_failures = sum(row['ordinal'] == ordinal for row in failed)
            if kind in ('deleted','clipped'):
                require(target_failures > 0, 'deleted/clipped actual thin ink is detected')
            controls.append({'source_ordinal':ordinal,'case_id':case['case_id'],'kind':kind,
                'source_path_index':case['thin_path_index'],'geometry_error':reason,
                'pdf':pdf.name,'pdf_sha256':private_placements['pdf_sha256'],
                'source_form_sha256':digest(before),'damaged_form_sha256':digest(after),
                'changed_source_ordinals':sorted(changed),'source_structure_preserved':True,
                'placement_matrices_preserved':True,'raster_comparisons':len(comparisons),
                'raster_failures':len(failed),'target_raster_failures':target_failures,
                'raster_summary_sha256':digest((directory/'summary.json').read_bytes()),
                'comparisons_sha256':digest((directory/'comparisons.json').read_bytes())})
    require(len(controls) == 24 and {(row['source_ordinal'],row['kind']) for row in controls}
            == {(ordinal,kind) for ordinal in range(8) for kind in KINDS}, 'complete eight-case damage matrix')
    require(digest(public_pdf.read_bytes()) == original_sha, 'the public PDF is never modified by controls')
    result = {'algorithm':'typaxis.vmb-thin-stroke-controls/1','passed':True,'source_cases':8,
        'damaged_pdfs':24,'raster_comparisons':1152,'all_geometry_rejected':True,
        'scope':'Private damaged copies of an independently checked public PDF; original placements are fixed, not a layout oracle.',
        'public_pdf_sha256':original_sha,'controls':controls,'full_goal_complete':False}
    save(output/'observed.json',result)
    return {key:value for key,value in result.items() if key != 'controls'}


def run_case(args, executor, selected, validators, helpers):
    sha,write_json,run_public_builds,ua_check = helpers
    case = args.output_root/'negation-slash-controls'
    case.mkdir()
    observation = {'algorithm':'typaxis.vmb-book-case-observed/1','case_id':'negation-slash-controls',
                   'passed':False,'phases':{}}
    try:
        job = case/'job'
        observation['phases']['source'] = prepare(args.fixture_root,job)
        observation['phases']['public_commands'] = run_public_builds(args.typaxis,job,case,executor,validators)
        placement = case/'placements.json'
        proof = verify_pdf(args.fixture_root,job,case/'output.pdf',placement,tools=selected['paths'])
        write_json(case/'source-paint-structure.json',proof)
        observation['phases']['source_paint_structure'] = {'passed':True,'sha256':sha(case/'source-paint-structure.json')}
        placements = strict_json(placement.read_bytes())
        observation['phases']['raw_svg_render'] = render(args.fixture_root,job,case/'output.pdf',placements,case/'render',
                                                        tools=selected['paths'])
        observation['phases']['pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,selected['paths'])
        observation['phases']['damage_controls'] = damage_controls(args.fixture_root,job,case/'output.pdf',placements,
            case/'controls',tools=selected['paths'])
        observation['passed'] = True
    except Exception as error:
        observation['error'] = {'type':type(error).__name__,'message':str(error)}
    write_json(case/'observed.json',observation)
    return observation
