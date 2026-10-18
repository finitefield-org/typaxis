"""Source topology and physical checks for formula-only public VMB fixtures.

The input verifier owns the authored engine ledger. This module independently
joins source parents/languages to the PDF graph and its actual paint bindings.
Body glyphs, notes, navigation and general book layout retain separate gates.
"""
import hashlib
from pathlib import Path

from pypdf import PdfReader
from pypdf.generic import ArrayObject, BooleanObject, DictionaryObject, IndirectObject, NumberObject, TextStringObject

from verify_pdf_structure import _canonical_language
from verify_vmb_scale_pdf import occurrences, verify_source_pdf
from vmb_book_inputs import verify_fixture_inputs, verify_prepared_job
from vmb_probe_io import contained_bytes, same_json, strict_json


ROLE_MAP = {'/Em':'/Span','/Exercise':'/Div','/Proof':'/Div','/Result':'/Div','/Strong':'/Span'}


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def integer(value):
    require(type(value) in (int,NumberObject) and value >= 0, 'nonnegative PDF integer')
    return int(value)


def reference(value):
    require(isinstance(value,IndirectObject) and value.idnum > 0 and value.generation >= 0,
            'indirect PDF object reference')
    return value.idnum,value.generation


def language(value):
    require(isinstance(value,str), 'source/PDF language string')
    return _canonical_language(value,'VMB source/PDF language')


def true(value):
    return type(value) is bool and value is True or isinstance(value,BooleanObject) and value.value is True


def equation_text(package, number):
    """Read the author's exact number through its separate identity mapping."""
    text_span,span = number['text_span'],number['span']
    require(set(text_span) == {'text_id','start_byte','end_byte'}
            and set(span) == {'source_id','start_byte','end_byte'}
            and all(type(value) is int and value >= 0 for value in [*text_span.values(),*span.values()])
            and text_span['start_byte'] < text_span['end_byte'] and span['start_byte'] < span['end_byte'],
            'typed nonempty equation-number source spans')
    require(type(number['minimum_gap']) is int and number['minimum_gap'] > 0, 'positive authored number gap')
    buffers = package['text_buffers']
    require(all(type(buffer['text_id']) is int and buffer['text_id'] >= 0 for buffer in buffers)
            and len({buffer['text_id'] for buffer in buffers}) == len(buffers), 'unique typed equation text buffers')
    matches = [buffer for buffer in buffers if buffer['text_id'] == text_span['text_id']]
    require(len(matches) == 1, 'one authored equation text buffer')
    buffer = matches[0]
    require(isinstance(buffer['utf8'],str), 'authored equation text is a string')
    raw = buffer['utf8'].encode('utf-8')
    require(text_span['end_byte'] <= len(raw), 'equation text inside source buffer')
    value = raw[text_span['start_byte']:text_span['end_byte']].decode('utf-8')
    require(value and not value.isspace() and not any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value),
            'meaningful authored equation number')
    overlapping = [mapping for mapping in buffer['mappings']
        if mapping['text_range']['start_byte'] < text_span['end_byte']
        and text_span['start_byte'] < mapping['text_range']['end_byte']]
    require(len(overlapping) == 1 and same_json(overlapping[0],{'kind':'identity','source_span':span,
        'text_range':{'start_byte':text_span['start_byte'],'end_byte':text_span['end_byte']}})
        and span['end_byte']-span['start_byte'] == len(value.encode('utf-8')),
        'exact owned equation-number identity mapping')
    return value


def source_tree(package, *, body_text=False):
    require(type(body_text) is bool, 'explicit source body-text scope')
    require(package['contract'] == 'typaxis.contract/1.4', 'formula structure public contract')
    document = package['document']
    require(not document['footnotes'] and not package['outline']['entries'], 'formula-only source structure scope')
    seen = set()

    def node(value, inherited, root=False, *, number=False, parent_kind=None):
        owner = value['node_id']
        require(type(owner) is int and owner >= 0 and owner not in seen, 'unique typed source structure owner')
        seen.add(owner)
        current_language = language(value.get('language') or inherited)
        kind = 'equation_number' if number else 'document' if root else value['kind']
        children = []
        alt = None
        if kind == 'document':
            role,children = '/Document',value['blocks']
        elif kind == 'paragraph':
            role,children = '/P',value['children']
        elif kind == 'semantic_container':
            require(value['semantic_kind'] in ('result','proof','exercise'), 'public formula container kind')
            role,children = '/'+value['semantic_kind'].title(),value['blocks']
        elif kind in ('math_vector','math_vector_block'):
            role,alt = '/Formula',value['alt']
        elif kind == 'equation_number':
            require(number and parent_kind == 'math_vector_block', 'equation number is an authored formula child')
            equation_text(package,value)
            role = '/Span'
        elif kind == 'text' and body_text:
            require(parent_kind == 'paragraph' and set(value) == {'kind','node_id','span','text_span'},
                    'plain authored paragraph text source')
            require(set(value['text_span']) == {'text_id','start_byte','end_byte'}
                    and all(type(item) is int and item >= 0 for item in value['text_span'].values())
                    and value['text_span']['start_byte'] < value['text_span']['end_byte'],
                    'typed nonempty body text source range')
            role = '/Span'
        elif kind == 'page_break' and body_text:
            require(parent_kind in ('document','semantic_container')
                    and set(value) == {'kind','node_id','span','classes'} and value['classes'] == [],
                    'plain authored page break source')
            return None
        elif kind == 'soft_break':
            require(parent_kind == 'paragraph' and set(value) == {'kind','node_id','span'},
                    'source soft break belongs to a paragraph')
            span = value['span']
            require(set(span) == {'source_id','start_byte','end_byte'}
                    and all(type(item) is int and item >= 0 for item in span.values())
                    and span['start_byte'] < span['end_byte'], 'typed nonempty source soft break')
            return None  # Explicit source break, without paint or structure text.
        elif kind in ('figure','vector_figure'):
            require(not value['caption'], 'formula scale figure has no caption glyphs')
            role,alt = '/Figure',value['alt']
        else:
            raise ValueError('formula-only structure does not cover source kind '+repr(kind))
        require(alt is None or isinstance(alt,str) and bool(alt), 'source occurrence alternative')
        lowered = [node(child,current_language,parent_kind=kind) for child in children]
        lowered = [child for child in lowered if child is not None]
        if kind == 'math_vector_block' and value['equation_number'] is not None:
            label = value['equation_number']
            outer,inner = value['span'],label['span']
            require(type(outer['source_id']) is type(outer['start_byte']) is type(outer['end_byte']) is int
                    and type(inner['source_id']) is type(inner['start_byte']) is type(inner['end_byte']) is int
                    and outer['source_id'] == inner['source_id']
                    and 0 <= outer['start_byte'] <= inner['start_byte'] < inner['end_byte'] <= outer['end_byte'],
                    'equation number belongs to its source formula')
            lowered.append(node(label,current_language,number=True,parent_kind=kind))
        return {'node_id':owner,'role':role,'language':current_language,'alt':alt,'children':lowered}
    return node(document,document['language'],True)


def number_tree(root):
    seen = set()

    def visit(ref):
        key = reference(ref)
        require(key not in seen, 'acyclic PDF number tree')
        seen.add(key)
        tree = ref.get_object()
        require(isinstance(tree,DictionaryObject), 'PDF number tree dictionary')
        if '/Nums' in tree:
            require(set(tree) <= {'/Nums','/Limits'} and '/Kids' not in tree, 'PDF number tree leaf keys')
            values = tree['/Nums']
            require(isinstance(values,ArrayObject) and len(values)%2 == 0, 'PDF number tree pairs')
            pairs = [(integer(values[i]),values[i+1]) for i in range(0,len(values),2)]
        else:
            require(set(tree) <= {'/Kids','/Limits'} and '/Kids' in tree, 'PDF number tree branch keys')
            kids = tree['/Kids']
            require(isinstance(kids,ArrayObject) and bool(kids), 'PDF number tree children')
            pairs = [pair for child in kids for pair in visit(child)]
        keys = [key for key,_ in pairs]
        require(all(a < b for a,b in zip(keys,keys[1:])), 'ordered unique PDF number tree keys')
        if '/Limits' in tree:
            limits = tree['/Limits']
            require(bool(keys) and isinstance(limits,ArrayObject) and len(limits) == 2
                    and list(map(integer,limits)) == [keys[0],keys[-1]], 'PDF number tree limits')
        return pairs
    return dict(visit(root))


def verify_source_structure(pdf, package, *, body_text=False):
    expected = source_tree(package,body_text=body_text)
    reader = PdfReader(Path(pdf),strict=True)
    catalog = reader.trailer['/Root']
    require(catalog['/Type'] == '/Catalog' and language(catalog['/Lang']) == expected['language'],
            'catalog language matches source')
    require(set(catalog.get('/MarkInfo',{})) == {'/Marked'} and true(catalog['/MarkInfo']['/Marked'])
            and set(catalog.get('/ViewerPreferences',{})) == {'/DisplayDocTitle'}
            and true(catalog['/ViewerPreferences']['/DisplayDocTitle']),
            'catalog accessible document state')
    require(not any(key in catalog for key in ('/AA','/OpenAction','/AcroForm','/OCProperties')),
            'plain formula catalog state')
    root_ref = catalog.raw_get('/StructTreeRoot')
    root_key = reference(root_ref)
    tree = root_ref.get_object()
    require(tree['/Type'] == '/StructTreeRoot' and tree.get('/RoleMap') == ROLE_MAP,
            'public source structure root/role map')
    roots = tree['/K']
    roots = roots if isinstance(roots,ArrayObject) else [tree.raw_get('/K')]
    require(len(roots) == 1, 'one source Document structure')
    parents = number_tree(tree.raw_get('/ParentTree'))
    page_by_key,arrays = {},{}
    for index,page in enumerate(reader.pages):
        key = reference(page.indirect_reference)
        require(key not in page_by_key and integer(page['/StructParents']) == index,
                'unique source page/StructParents')
        page_by_key[key] = index
        require(index in parents, 'ParentTree has every page')
        array = parents[index].get_object()
        require(isinstance(array,ArrayObject), 'page ParentTree array')
        arrays[key] = array
    require(set(parents) == set(range(len(reader.pages)))
            and integer(tree['/ParentTreeNextKey']) == len(reader.pages), 'complete ParentTree page keys')
    visited,bindings,marked = set(),[],set()

    def visit(ref, parent, source, inherited):
        key = reference(ref)
        require(key not in visited, 'acyclic unique source structure children')
        visited.add(key)
        value = ref.get_object()
        require(isinstance(value,DictionaryObject)
                and set(value) <= {'/Type','/S','/P','/K','/Lang','/Alt'}
                and value['/Type'] == '/StructElem' and value['/S'] == source['role']
                and reference(value.raw_get('/P')) == parent, 'source structure role/parent/keys')
        effective = language(value['/Lang']) if '/Lang' in value else inherited
        require(effective == source['language'], 'source structure effective language')
        if source['alt'] is None:
            require('/Alt' not in value, 'no invented structure alternative')
        else:
            require(isinstance(value.get('/Alt'),TextStringObject) and value['/Alt'] == source['alt'],
                    'source structure occurrence alternative')
        kids = value.get('/K',[])
        kids = kids if isinstance(kids,ArrayObject) else [value.raw_get('/K')]
        painted = source['role'] in ('/Formula','/Figure','/Span')
        require(len(kids) == len(source['children'])+int(painted), 'exact source structure children and own MCR')
        if painted:
            mcr = kids[0].get_object()
            require(isinstance(mcr,DictionaryObject) and set(mcr) == {'/Type','/Pg','/MCID'}
                    and mcr['/Type'] == '/MCR', 'source occurrence MCR shape')
            page_key,mcid = reference(mcr.raw_get('/Pg')),integer(mcr['/MCID'])
            require(page_key in arrays and mcid < len(arrays[page_key]), 'source MCR page/range')
            require(reference(arrays[page_key][mcid]) == key and (page_key,mcid) not in marked,
                    'ParentTree/MCR unique source join')
            marked.add((page_key,mcid))
            bindings.append({'node_id':source['node_id'],'structure_object':list(key),
                             'page_object':list(page_key),'mcid':mcid})
        for child,wanted in zip(kids[int(painted):],source['children']):
            visit(child,key,wanted,effective)
    visit(roots[0],root_key,expected,language(catalog['/Lang']))
    all_structures = set()
    for generation,objects in reader.xref.items():
        if generation == 65535:
            continue
        for number in objects:
            if number <= 0:
                continue
            obj = reader.get_object(IndirectObject(number,generation,reader))
            if isinstance(obj,DictionaryObject) and obj.get('/Type') == '/StructElem':
                all_structures.add((number,generation))
    for number in reader.xref_objStm:
        obj = reader.get_object(IndirectObject(number,0,reader))
        if isinstance(obj,DictionaryObject) and obj.get('/Type') == '/StructElem':
            all_structures.add((number,0))
    require(visited == all_structures, 'all PDF structure elements are source-reachable')
    require(len(marked) == sum(len(array) for array in arrays.values()), 'all ParentTree slots have source MCRs')
    return {'algorithm':'typaxis.vmb-formula-source-structure/1','source_structure_nodes':len(visited),
            'pages':len(reader.pages),'source_occurrences':len(bindings),
            'document_language':package['document']['language'],'source_mcr_bindings':bindings}


def verify_fixture_pdf(fixture_root, job, pdf, placement_output=None, *, extraction_tools=None):
    source = verify_prepared_job(job,fixture_root)
    package = strict_json(contained_bytes(job,'document-package.json'))
    expected = list(occurrences(package['document']))
    inventory = {'case':'authored-engine','placements':source['occurrences'],
                 'resources':source['cases'],
                 'distinct_resources':len({image['expected_sha256'] for image in package['resources']['images']})}
    painted = verify_source_pdf(job,pdf,package,inventory,expected,placement_output,
        extraction_tools=extraction_tools,scope='authored-vmb-formula-fixture',
        include_structure_bindings=True,strict_extractor_output=True)
    graph = verify_source_structure(pdf,package)
    require(painted['source_mcr_bindings'] == graph['source_mcr_bindings'],
            'source hierarchy and actual paint bind the same owners')
    return {'source':source,'paint':painted,'structure':graph,
            'pdf_sha256':hashlib.sha256(Path(pdf).read_bytes()).hexdigest()}


def render_fixture_pdf(fixture_root, job, pdf, placements, output, *, tool_paths=None):
    from verify_vmb_scale_render import original_svg, render_source_pages
    require(not output.resolve().is_relative_to(Path(fixture_root).resolve())
            and not output.resolve().is_relative_to(job.resolve()), 'render output outside fixture inputs')
    facts = verify_fixture_inputs(fixture_root)
    source = verify_prepared_job(job,fixture_root)
    originals,counts,cases = [],[],[]
    for item in facts['cases']:
        case = {'tex':contained_bytes(fixture_root,item['source_tex_file']).decode('utf-8'),
            'speech':item['alt'],'display':item['display'],'font_size_raw':item['font_size_raw'],
            'metrics':item['metrics'],'path_count':item['expected_path_count'],
            'segment_count':item['expected_stored_segments']-5}
        original,observed = original_svg(contained_bytes(fixture_root,item['source_svg_file']),
            contained_bytes(fixture_root,item['admitted_svg_file']),case,
            expected_font_size_raw=item['font_size_raw'])
        originals.append(original);counts.append(observed);cases.append(case)
    inventory = {'case':'authored-engine','placements':source['occurrences'],'resources':source['cases']}
    return render_source_pages(job,pdf,placements,output,{'cases':cases},inventory,
                               originals,counts,facts['index_sha256'],tool_paths=tool_paths)


def verify_book_formula_pdf(job, package, expected, pdf, placement_output, render_output, *, tools):
    """Apply the current physical/graph kernels after the book input preflight.

    The source walker rejects body/note kinds it cannot yet verify. Original
    per-resource SVG sidecars remain mandatory for the independent render phase.
    """
    from verify_vmb_scale_render import original_svg, render_source_pages
    source = list(occurrences(package['document']))
    images = package['resources']['images']
    inventory = {'case':'book','placements':len(source),'resources':len(images),
                 'distinct_resources':len({image['expected_sha256'] for image in images})}
    painted = verify_source_pdf(job,pdf,package,inventory,source,placement_output,
        extraction_tools={key:tools[key] for key in ('mutool','pdftotext')},
        scope='book-formula-source',include_structure_bindings=True,strict_extractor_output=True)
    graph = verify_source_structure(pdf,package)
    require(painted['source_mcr_bindings'] == graph['source_mcr_bindings'],
            'book source graph and paint bind the same owners')
    originals,counts,cases = {},{},{}
    for image,record in zip(images,expected['image_files']):
        if image['media_type'] == 'png':
            continue
        require(image['media_type'] == 'svg-safe-2' and isinstance(record.get('original_svg'),dict),
                'book raw SVG sidecar for every vector resource')
        descriptor = record['original_svg']
        raw = contained_bytes(job,descriptor['path'])
        require(len(raw) == descriptor['byte_length'] and hashlib.sha256(raw).hexdigest() == descriptor['sha256'],
                'book original SVG sidecar hash')
        case = descriptor['engine_case']
        require(case.get('language','en') == 'en', 'book raw SVG verifier does not yet cover this source locale')
        reference,observed = original_svg(raw,contained_bytes(job,image['uri']),case,
                                         expected_font_size_raw=case['font_size_raw'])
        index = image['image_id']
        originals[index],counts[index],cases[index] = reference,observed,case
    placements = strict_json(placement_output.read_bytes())
    raster = render_source_pages(job,pdf,placements,render_output,{'cases':cases},inventory,originals,counts,
        hashlib.sha256(contained_bytes(job,'document-package.json')).hexdigest(),
        tool_paths={key:tools[key] for key in ('mutool','pdftoppm','rsvg-convert')})
    return {'paint':painted,'structure':graph,'raw_svg_render':raster}
