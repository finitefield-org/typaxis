"""Source and physical checks for the authored 400-formula VMB chapter."""
import copy
from pathlib import Path

from vmb_book_inputs import canonical_bytes,digest,read_canonical,require,verify_fixture_inputs
from vmb_probe_io import contained_bytes,same_json


FONT_SHA = 'b4c632e3cdf9acc7f28758fb5a323c8524d7fc6660d46904d9b6cbe2809c419c'
FONT_FILE = 'chapter-layout/DejaVuSans.ttf'
PLAN_FILE = 'chapter-layout/plan.json'
SUPPORT = {'id':'chapter-400-1','plan':PLAN_FILE,'provenance':'chapter-layout/provenance.json',
           'placements':400,'equation_numbers':200}
TITLE = 'Authored 400-formula chapter from real VMB engine fixtures'
PROVENANCE = {'engine_id':'vmb-go-math-svg','engine_version':'2.0.0','rules_version':'vmb.typaxis-safe-svg/1'}


def source_inputs(fixture):
    facts = verify_fixture_inputs(fixture)
    index = read_canonical(fixture,'fixture-index.json')
    require(same_json(index['provenance'].get('chapter_layout_support'),SUPPORT), 'registered authored chapter support')
    provenance = read_canonical(fixture,SUPPORT['provenance'])
    require(provenance['algorithm'] == 'typaxis.vmb-chapter-support/1'
            and provenance['kind'] == 'authored-real-engine-fixture-chapter'
            and provenance['plan'] == PLAN_FILE and provenance['plan_sha256'] == digest(contained_bytes(fixture,PLAN_FILE)),
            'authored chapter source provenance')
    font = provenance['font']
    require(font['file'] == FONT_FILE and font['job_uri'] == 'fonts/DejaVuSans.ttf'
            and font['family'] == 'DejaVu Sans' and font['media_type'] == 'sfnt-truetype-glyf'
            and type(font['face_index']) is int and font['face_index'] == 0
            and font['unicode_characters'] == '()0123456789'
            and font['sha256'] == digest(contained_bytes(fixture,FONT_FILE)) == FONT_SHA
            and type(font['byte_length']) is int and font['byte_length'] == len(contained_bytes(fixture,FONT_FILE)),
            'unchanged visible original number font')
    require(font['license_file'] in index['provenance']['license_and_notice_files']
            and font['license_sha256'] == digest(contained_bytes(fixture,font['license_file'])), 'original font license retained')
    plan = read_canonical(fixture,PLAN_FILE)
    require(set(plan) == {'algorithm','id','source_case_ledger','source_case_ledger_sha256','language','container',
                         'blocks','counts','original_book','approved_layout_golden'}
            and plan['algorithm'] == 'typaxis.vmb-chapter-plan/1' and plan['id'] == 'chapter-400-1'
            and plan['source_case_ledger'] == 'cases.json'
            and plan['source_case_ledger_sha256'] == digest(contained_bytes(fixture,'cases.json'))
            and plan['language'] == 'en-US' and same_json(plan['container'],{'node_id':1,'kind':'result'})
            and same_json(plan['counts'],{'formulas':400,'equation_numbers':200,'paragraphs':50,'soft_breaks':150,'source_nodes':802})
            and plan['original_book'] is False and plan['approved_layout_golden'] is False,
            'complete honest authored chapter plan')
    # Validate the authored plan independently of generated PDF/trace positions.
    expected,node_id,label = [],2,0
    for repetition in range(20):
        for family in ('fraction','parentheses','equivalence','negation','long'):
            for size in (720896,786431):
                if repetition%4 == 0:
                    block = {'kind':'paragraph','node_id':node_id,'children':[]};node_id += 1
                    for ordinal in range(4):
                        block['children'].append({'kind':'math_vector','node_id':node_id,'case_id':f'{family}-inline-{size}'});node_id += 1
                        if ordinal < 3:
                            block['children'].append({'kind':'soft_break','node_id':node_id});node_id += 1
                    expected.append(block)
                label += 1
                expected.append({'kind':'math_vector_block','node_id':node_id,'case_id':f'{family}-block-{size}',
                    'equation_number':{'node_id':node_id+1,'text':f'({label})','minimum_gap':65536}});node_id += 2
    require(node_id == 802 and same_json(plan['blocks'],expected), 'fixed complete source chapter order and owners')
    return {'fixture_index_sha256':facts['index_sha256'],'cases':read_canonical(fixture,'cases.json'),
            'plan':plan,'plan_sha256':provenance['plan_sha256'],'font':font}


def source_wire(fixture, facts):
    package = read_canonical(fixture,'package-template.json')
    cases = {case['case_id']:(ordinal,case) for ordinal,case in enumerate(facts['cases'])}
    font = facts['font']
    package['resources']['font_faces'] = [{'font_face_id':0,'family':font['family'],'uri':font['job_uri'],
        'media_type':font['media_type'],'face_index':font['face_index'],'expected_sha256':FONT_SHA}]
    for rule in package['style_sheet']['rules']:
        for declaration in rule['declarations']:
            if declaration['name'] == 'font_family':declaration['value']['families'] = [font['family']]
    for image_id,case in enumerate(facts['cases']):
        package['resources']['images'].append({'image_id':image_id,'uri':case['admitted_svg_file'],'media_type':'svg-safe-2',
            'expected_sha256':case['admitted_svg_sha256'],'vector_provenance':PROVENANCE.copy()})
    chunks,events,blocks = [],[],[]
    offset = 0
    def span(start,end):return {'source_id':0,'start_byte':start,'end_byte':end}
    def formula(authored, group, paragraph=None):
        nonlocal offset
        image_id,case = cases[authored['case_id']]
        tex = contained_bytes(fixture,case['source_tex_file'])
        numbered = authored['kind'] == 'math_vector_block'
        label = authored['equation_number']['text'].encode() if numbered else b''
        text,start = tex+label,offset
        end,text_id = offset+len(text),len(package['text_buffers'])
        mappings = [{'kind':'identity','source_span':span(start,start+len(tex)),
                     'text_range':{'start_byte':0,'end_byte':len(tex)}}]
        if numbered:mappings.append({'kind':'identity','source_span':span(start+len(tex),end),
                                     'text_range':{'start_byte':len(tex),'end_byte':len(text)}})
        package['text_buffers'].append({'text_id':text_id,'utf8':text.decode(),'mappings':mappings})
        node = {'kind':authored['kind'],'node_id':authored['node_id'],'image_id':image_id,'span':span(start,end),
            'metrics':copy.deepcopy(case['metrics']),'alt':case['alt'],'actual_text':case['actual_text'],
            'source_tex':{'text_span':{'text_id':text_id,'start_byte':0,'end_byte':len(tex)}}}
        events.append({'node':node,'case':case,'paragraph_id':paragraph,'flow_group':group,'kind':'formula'})
        if numbered:
            number = authored['equation_number']
            node.update(classes=[],equation_number={'node_id':number['node_id'],'minimum_gap':number['minimum_gap'],
                'span':span(start+len(tex),end),'text_span':{'text_id':text_id,'start_byte':len(tex),'end_byte':len(text)}})
            events.append({'node':node['equation_number'],'text':number['text'],'formula_node_id':node['node_id'],
                           'flow_group':group,'kind':'number'})
        else:node['spacing'] = copy.deepcopy(case['spacing'])
        chunks.append(text+b'\n');offset = end+1
        return node
    for authored in facts['plan']['blocks']:
        if authored['kind'] == 'paragraph':
            children,start = [],offset
            for child in authored['children']:
                if child['kind'] == 'soft_break':children.append({'kind':'soft_break','node_id':child['node_id'],'span':span(offset-1,offset)})
                else:children.append(formula(child,authored['node_id'],authored['node_id']))
            blocks.append({'kind':'paragraph','node_id':authored['node_id'],'span':span(start,offset-1),'classes':[],'children':children})
        else:blocks.append(formula(authored,authored['node_id']))
    data = b''.join(chunks)
    package['sources'] = [{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(data),'sha256':digest(data)}]
    package['document']['blocks'] = [{'kind':'semantic_container','semantic_kind':'result','node_id':1,
        'span':span(0,len(data)-1),'classes':[],'anchor_id':None,'blocks':blocks}]
    package['metadata']['title'] = TITLE
    require(len(events) == 600 and len(package['text_buffers']) == 400, 'complete authored chapter source events')
    return package,data,events


def prepare(fixture, output):
    fixture,output = Path(fixture).resolve(),Path(output).resolve()
    require(not output.is_relative_to(fixture) and not fixture.is_relative_to(output), 'chapter output outside source fixture')
    facts = source_inputs(fixture)
    package,data,_ = source_wire(fixture,facts)
    output.mkdir()
    copy_files = [(FONT_FILE,facts['font']['job_uri'])]+[(case['admitted_svg_file'],case['admitted_svg_file']) for case in facts['cases']]
    for source,target in copy_files:
        path = output/target;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(contained_bytes(fixture,source))
    (output/'input.tsf').write_bytes(data)
    (output/'document-package.json').write_bytes(canonical_bytes(package))
    return verify_job(fixture,output)


def verify_job(fixture, job):
    facts = source_inputs(fixture)
    package,data,events = source_wire(fixture,facts)
    require(contained_bytes(job,'document-package.json') == canonical_bytes(package), 'exact source-authored chapter package')
    require(contained_bytes(job,'input.tsf') == data, 'exact chapter TeX and number source bytes')
    require(digest(contained_bytes(job,facts['font']['job_uri'])) == FONT_SHA, 'original chapter font bytes')
    for case in facts['cases']:
        require(digest(contained_bytes(job,case['admitted_svg_file'])) == case['admitted_svg_sha256'], 'complete chapter SVG source bytes')
    expected = {'document-package.json','input.tsf',facts['font']['job_uri']}|{case['admitted_svg_file'] for case in facts['cases']}
    require({str(path.relative_to(job)) for path in job.rglob('*') if path.is_file() or path.is_symlink()} == expected,
            'complete generated chapter input inventory')
    return {'algorithm':'typaxis.vmb-book-chapter-job/1','fixture_index_sha256':facts['fixture_index_sha256'],
        'plan_sha256':facts['plan_sha256'],'font_sha256':FONT_SHA,'source_nodes':802,'formulas':400,
        'equation_numbers':200,'paragraphs':50,'soft_breaks':150,'source_images':20,
        'distinct_images':len({case['admitted_svg_sha256'] for case in facts['cases']}),
        'package_sha256':digest(contained_bytes(job,'document-package.json'))}


def verify_pdf(fixture, job, pdf, placement_output=None):
    """Read every actual Form/glyph and join it to source and the MCR graph."""
    from decimal import Decimal
    from fractions import Fraction
    from pypdf import PdfReader
    from pypdf.generic import ContentStream
    from verify_vmb_scale_pdf import PagePaint,decimal,fixed,multiply,verify_form_geometry,verify_placement
    from vmb_book_font import TrueType,verify_number_font
    from vmb_book_pdf import integer,verify_source_structure
    if placement_output is not None:
        require(not placement_output.exists() and all(not placement_output.resolve().is_relative_to(Path(path).resolve())
            for path in (fixture,job)), 'chapter placement output outside source inputs and not existing')
    source = verify_job(fixture,job)
    facts = source_inputs(fixture)
    package,_,expected = source_wire(fixture,facts)
    graph = verify_source_structure(pdf,package)
    bindings = {row['node_id']:row for row in graph['source_mcr_bindings']}
    require(set(bindings) == {row['node']['node_id'] for row in expected}, 'all chapter source MCR owners')
    original_font = TrueType(contained_bytes(fixture,FONT_FILE))
    characters = {char for event in expected if event['kind'] == 'number' for char in event['text']}
    require(characters == set('()0123456789'), 'complete authored chapter number character inventory')
    font_size = 786432  # Authored number text style is exactly 12pt.
    master = package['page_masters']['masters'][0]
    body = master['body'];reader = PdfReader(pdf,strict=True)
    font_models,form_models = {},set()
    forms,form_rows,number_rows,pages = set(),[],[],[]
    expected_index = 0

    class NumberPaint(PagePaint):
        def __init__(self, resources):
            super().__init__(resources)
            self.font_name,self.font_stack = None,[]

        def step(self, args, op):
            if op == b'q':self.font_stack.append(self.font_name)
            if op == b'Q':
                require(self.font_stack, 'chapter font-state restore')
                self.font_name = self.font_stack.pop()
            if op == b'Tf':self.font_name = args[0]
            if op == b'Tj' and self.render_mode == 0:
                raw = getattr(args[0],'original_bytes',None) if len(args) == 1 else None
                require(self.in_text and isinstance(raw,bytes) and len(raw) == 2 and self.black
                        and self.font_size == Decimal(12)
                        and self.text_parameters == {b'Tc':0,b'Tw':0,b'Tz':100,b'TL':0,b'Ts':0},
                        'one opaque source number glyph with authored text parameters')
                return ('glyph',self.font_name,int.from_bytes(raw,'big'),multiply(self.matrix,self.text_matrix),self.clip)
            return super().step(args,op)

    def inside(rect, limit):
        require(rect[0] >= limit[0]-1 and rect[1] >= limit[1]-1
                and rect[2] <= limit[2]+1 and rect[3] <= limit[3]+1, 'complete visible source number glyph bounds')

    for page_index,page in enumerate(reader.pages):
        require(tuple(map(fixed,page.mediabox)) == (0,0,master['width'],master['height'])
                and tuple(page.get('/CropBox',page.mediabox)) == tuple(page.mediabox)
                and page.get('/Rotate',0) == 0 and page.get('/UserUnit',1) == 1 and not page.get('/Annots'),
                'source chapter page geometry')
        pages.append({'page_index':page_index,'media_box':[str(value) for value in page.mediabox]})
        resources = page['/Resources'];paint = NumberPaint(resources)
        stack,active = [],None
        for args,op in ContentStream(page.get_contents(),reader).operations:
            event = paint.step(args,op)
            if op == b'BDC':
                props = args[1]
                if '/MCID' in props:
                    require(active is None and not stack and expected_index < len(expected), 'one source chapter MCR scope')
                    want = expected[expected_index]
                    binding = bindings[want['node']['node_id']]
                    require(args[0] == ('/Formula' if want['kind'] == 'formula' else '/Span')
                            and set(props) == {'/MCID','/Lang'} and str(props['/Lang']) == package['document']['language']
                            and integer(props['/MCID']) == binding['mcid']
                            and binding['page_object'] == [page.indirect_reference.idnum,page.indirect_reference.generation],
                            'source owner, page paint and MCR join')
                    active = {'want':want,'anchors':[],'paint':[],'glyphs':[],'actual':[]}
                    stack.append('mcr')
                else:
                    require(active is not None and stack == ['mcr'] and args[0] == '/Span'
                            and set(props) == {'/ActualText'}, 'exact authored chapter ActualText scope')
                    active['actual'].append(str(props['/ActualText']));stack.append('actual')
            elif op == b'EMC':
                require(stack, 'matched chapter marked content')
                closed = stack.pop()
                if closed == 'mcr':
                    require(active is not None and not stack, 'complete chapter MCR')
                    want,node = active['want'],active['want']['node']
                    text = (node['actual_text'] or node['alt']) if want['kind'] == 'formula' else want['text']
                    require(active['actual'] == [text], 'exact source semantic text for each chapter owner')
                    if want['kind'] == 'formula':
                        require(len(active['paint']) == len(active['anchors']) == 1 and not active['glyphs'],
                                'one source formula Form and supplemental anchor')
                        painting,form,key = active['paint'][0]
                        image = package['resources']['images'][node['image_id']]
                        model_key = (key,image['expected_sha256'])
                        if model_key not in form_models:
                            verify_form_geometry(contained_bytes(job,image['uri']),form,reader)
                            form_models.add(model_key)
                        rect = verify_placement(node,painting,active['anchors'],page.mediabox,body)
                        matrix = painting[1]
                        row = {'ordinal':len(form_rows),'node_id':node['node_id'],'image_id':node['image_id'],
                            'kind':node['kind'],'page_index':page_index,'bounds_raw':list(rect),
                            'world_matrix':[str(value) for value in matrix],
                            'baseline_raw':master['height']-fixed(active['anchors'][0][5]),
                            'paragraph_id':want['paragraph_id'],'flow_group':want['flow_group']}
                        form_rows.append(row);forms.add(key)
                    else:
                        require(not active['paint'] and not active['anchors'] and len(active['glyphs']) == len(text),
                                'exact source number glyph count without additional paint')
                        glyph_rows,total,baseline = [],0,None
                        for ordinal,(glyph_event,model,key) in enumerate(active['glyphs']):
                            _,name,cid,matrix,clip = glyph_event
                            require(matrix[:4] == tuple(map(Decimal,(1,0,0,1))), 'unit upright chapter number glyph')
                            mapped = next((row for row in model['cids'] if row['cid'] == cid),None)
                            require(mapped is not None and mapped['unicode'] == text[ordinal], 'each painted CID matches its authored character')
                            x,y = fixed(matrix[4]),fixed(matrix[5])
                            if baseline is None:baseline = (x,y)
                            require(abs(x-baseline[0]-total) <= 1 and abs(y-baseline[1]) <= 1,
                                    'source hmtx advances and shared number baseline')
                            original = original_font.glyph(mapped['source_gid'])
                            scaled = [Fraction(value*font_size,original_font.units) for value in original['bbox']]
                            rect = (Fraction(x)+scaled[0],Fraction(y)+scaled[1],Fraction(x)+scaled[2],Fraction(y)+scaled[3])
                            inside(rect,(body['x'],master['height']-body['y']-body['height'],
                                         body['x']+body['width'],master['height']-body['y']))
                            if clip is not None:inside(rect,tuple(map(fixed,clip)))
                            advance = Fraction(mapped['advance_units']*font_size,original_font.units)
                            require(advance.denominator == 1, 'exact authored number advance on fixed source grid')
                            total += int(advance)
                            glyph_rows.append({'cid':cid,'unicode':text[ordinal],'source_gid':mapped['source_gid'],
                                'font_object':list(key),'font_size_raw':font_size,'world_matrix':[str(value) for value in matrix]})
                        formula = next(row for row in reversed(form_rows) if row['node_id'] == want['formula_node_id'])
                        require(formula['page_index'] == page_index and abs(baseline[0]+total-body['x']-body['width']) <= 1,
                                'source number is right aligned beside its formula on the same page')
                        formula_source = next(row['node'] for row in expected if row['node']['node_id'] == want['formula_node_id'])
                        top = master['height']-formula['bounds_raw'][3]
                        center_baseline = Fraction(2*top+formula_source['metrics']['viewport']['height'],2)
                        center_baseline += Fraction((original_font.ascender+original_font.descender)*font_size,2*original_font.units)
                        require(abs(Fraction(master['height']-baseline[1])-center_baseline) <= 1,
                                'number baseline from original font metrics and source formula viewport')
                        require(baseline[0]-formula['bounds_raw'][2] >= node['minimum_gap']-1, 'authored minimum number gap')
                        number_rows.append({'node_id':node['node_id'],'formula_node_id':want['formula_node_id'],
                            'page_index':page_index,'text':text,'source_width_raw':total,'glyphs':glyph_rows})
                    expected_index += 1;active = None
            elif op == b'BMC':raise ValueError('unexpected chapter artifact scope')
            if event is not None:
                require(active is not None and stack == ['mcr','actual'], 'all chapter paint belongs to exact source text')
                if event[0] == 'anchor':active['anchors'].append(event[1])
                elif event[0] == 'paint':
                    ref = resources['/XObject'].raw_get(args[0]);key = (ref.idnum,ref.generation)
                    active['paint'].append((event,ref.get_object(),key))
                else:
                    require(event[0] == 'glyph', 'known source chapter paint kind')
                    ref = resources['/Font'].raw_get(event[1]);key = (ref.idnum,ref.generation)
                    if key not in font_models:font_models[key] = verify_number_font(ref.get_object(),original_font,characters)
                    active['glyphs'].append((event,font_models[key],key))
        paint.finish()
        require(not stack and active is None and not paint.font_stack, 'balanced complete chapter page state')
    require(expected_index == len(expected) and len(form_rows) == 400 and len(number_rows) == 200
            and sum(len(row['glyphs']) for row in number_rows) == 892
            and len(forms) == 14 and len(font_models) == 1, 'complete source chapter painting inventory')
    proof = {'algorithm':'typaxis.vmb-chapter-source-paint/1','source':source,'pdf_sha256':digest(pdf.read_bytes()),
        'structure':graph,'pages':pages,'placements':form_rows,'equation_numbers':number_rows,
        'fonts':[{'pdf_font_object':list(key),**value} for key,value in sorted(font_models.items())],
        'formula_forms':len(forms),'visible_number_glyphs':892,'full_layout_verified':False,'raw_source_render_verified':False,
        'scope':'Source, structure, Form/glyph identity, number advances/alignment/gap/baseline; full pagination and raster remain.'}
    if placement_output is not None:
        with placement_output.open('xb') as output:output.write(canonical_bytes(proof))
    return proof


def layout_model(fixture):
    """Calculate this authored chapter's positions without opening any PDF."""
    from fractions import Fraction
    from vmb_book_font import TrueType
    facts = source_inputs(fixture)
    package,_,events = source_wire(fixture,facts)
    font = TrueType(contained_bytes(fixture,FONT_FILE))
    master = package['page_masters']['masters'][0];body = master['body']
    def style(selector):
        result = {}
        for rule in package['style_sheet']['rules']:
            if rule['selector'] == selector:
                require(rule['extends'] is None, 'flat authored chapter style cascade')
                for declaration in rule['declarations']:
                    require(declaration['important'] is False, 'plain authored chapter declarations')
                    result[declaration['name']] = declaration['value']
        return result
    def length(values,name):
        value = values[name]
        require(value['kind'] == 'length' and type(value['value']) is int, 'exact source chapter style length')
        return value['value']
    paragraph,block,container = map(style,('paragraph','math_vector_block','semantic_container'))
    require(paragraph['text_align'] == {'kind':'keyword','value':'start'}
            and block['text_align'] == {'kind':'keyword','value':'center'}
            and all(values['keep_with_next'] == {'kind':'boolean','value':False} for values in (paragraph,block,container))
            and length(block,'start_indent') == length(block,'end_indent') == 0
            and length(container,'start_indent') == length(container,'end_indent') == 0,
            'fixed unindented authored chapter regions and alignment')
    number_size,number_height = length(container,'font_size'),length(container,'line_height')
    require(number_size == 786432 and number_height == 917504, 'authored chapter number size and line height')
    asc,desc = Fraction(font.ascender*number_size,font.units),Fraction(-font.descender*number_size,font.units)
    require(asc.denominator == desc.denominator == 1 and asc+desc <= number_height, 'source number font strut fits its line')
    number_baseline = int(asc)+round(Fraction(number_height-int(asc+desc),2))
    by_owner = {row['node']['node_id']:row for row in events}
    items = []
    for authored in facts['plan']['blocks']:
        if authored['kind'] == 'paragraph':
            formulae = [by_owner[child['node_id']] for child in authored['children'] if child['kind'] == 'math_vector']
            metric = formulae[0]['node']['metrics'];spacing = formulae[0]['node']['spacing']
            require(all(row['node']['metrics'] == metric and row['node']['spacing'] == spacing for row in formulae)
                    and spacing == {'before':0,'after':0}, 'homogeneous source formula wrap paragraphs')
            width = body['width']-length(paragraph,'start_indent')-length(paragraph,'end_indent')
            def extent(count):
                left = min(0,metric['origin_x'])
                right = max(count*metric['advance'],(count-1)*metric['advance']+metric['origin_x']+metric['viewport']['width'])
                return left,right
            fitting = [count for count in range(1,5) if extent(count)[1]-extent(count)[0] <= width]
            require(fitting and max(fitting) in (1,4), 'fixed chapter has forced single or complete four-formula lines')
            count = max(fitting);lines = 4//count
            line_height = max(metric['ascent']+metric['descent'],length(paragraph,'line_height'))
            baseline = metric['ascent']+round(Fraction(line_height-metric['ascent']-metric['descent'],2))
            for line in range(lines):
                members = formulae[line*count:(line+1)*count]
                left,_ = extent(count)
                items.append({'kind':'paragraph','owner':authored['node_id'],'line_index':line,'paragraph_lines':lines,
                    'height':line_height,'before':length(paragraph,'space_before') if line == 0 else 0,
                    'after':length(paragraph,'space_after') if line+1 == lines else 0,'trailing':0,
                    'baseline_offset':baseline,'members':[row['node']['node_id'] for row in members],
                    'x':body['x']+length(paragraph,'start_indent')-left,
                    'source_inline_size':width,'source_visual_size':extent(count)[1]-extent(count)[0]})
        else:
            node = by_owner[authored['node_id']]['node'];height = max(node['metrics']['viewport']['height'],number_height)
            items.append({'kind':'block','owner':authored['node_id'],'height':height,
                'before':length(block,'space_before'),'after':length(block,'space_after'),'trailing':0,
                'members':[node['node_id']],
                'viewport_offset':round(Fraction(height-node['metrics']['viewport']['height'],2)),
                'number_baseline_offset':round(Fraction(height-number_height,2))+number_baseline})
    require(len(items) == 280, 'all eighty inline lines and two hundred blocks from source')
    items[0]['before'] += length(container,'space_before')
    items[-1]['after'] += length(container,'space_after')
    decisions,start = [],0
    while start < len(items):
        used,end,candidates = 0,start,[]
        while end < len(items):
            item = items[end]
            before = 0 if end == start else items[end-1]['after']+item['before']
            next_used = used+before+item['height']+item['trailing']
            if next_used > body['height']:break
            used = next_used;end += 1
            isolation = 0
            if end < len(items) and item['kind'] == 'paragraph' and items[end]['owner'] == item['owner']:
                first = items[start]['line_index'] if items[start]['owner'] == item['owner'] else 0
                on_page = item['line_index']+1-first
                remaining = item['paragraph_lines']-item['line_index']-1
                isolation = int(on_page < 2)+int(remaining < 2)
            cost = 0 if end == len(items) else isolation*1_000_000+(body['height']-used)*1_000_000//body['height']
            candidates.append({'end':end,'used_height':used,'cost':cost,'widow_orphan_isolations':isolation})
        require(candidates, 'source chapter content fits a body page')
        selected = candidates[-1] if end == len(items) else min(candidates,key=lambda row:(row['cost'],row['end']))
        decisions.append({'page_index':len(decisions),'start':start,'end':selected['end'],
                          'used_height':selected['used_height'],'selected_cost':selected['cost'],'candidates':candidates})
        start = selected['end']
    formula_positions,number_positions = [],[]
    for decision in decisions:
        used = 0
        for index in range(decision['start'],decision['end']):
            item = items[index]
            before = 0 if index == decision['start'] else items[index-1]['after']+item['before']
            top = body['y']+used+before
            for ordinal,owner in enumerate(item['members']):
                node = by_owner[owner]['node'];metric = node['metrics']
                if item['kind'] == 'paragraph':
                    x = item['x']+ordinal*metric['advance']+metric['origin_x']
                    y = top+item['baseline_offset']-metric['baseline']
                else:
                    x = body['x']+(body['width']-metric['viewport']['width'])//2
                    y = top+item['viewport_offset']
                formula_positions.append({'node_id':owner,'page_index':decision['page_index'],'x':x,'y':y,
                    'baseline':y+metric['baseline'],'width':metric['viewport']['width'],'height':metric['viewport']['height']})
                if item['kind'] == 'block':
                    label = next(row for row in events if row['kind'] == 'number' and row['formula_node_id'] == owner)
                    width = sum(Fraction(font.metrics(font.codepoint(ord(char)))[0]*number_size,font.units) for char in label['text'])
                    require(width.denominator == 1, 'exact authored number source width')
                    number_positions.append({'node_id':label['node']['node_id'],'formula_node_id':owner,
                        'page_index':decision['page_index'],'x':body['x']+body['width']-int(width),
                        'baseline':top+item['number_baseline_offset'],'text':label['text'],'width':int(width)})
            used += before+item['height']+item['trailing']
        require(used == decision['used_height'], 'source pagination conservation')
    return {'algorithm':'typaxis.vmb-chapter-source-layout/1','scope':'Independent fixed chapter source metrics and public break policy; no PDF or trace inputs.',
        'pages':len(decisions),'inline_lines':80,'block_lines':200,'page_decisions':decisions,
        'formula_positions':formula_positions,'number_positions':number_positions}


def verify_layout(fixture, proof):
    from decimal import Decimal
    from verify_vmb_scale_pdf import fixed
    facts = source_inputs(fixture);package,_,_ = source_wire(fixture,facts)
    master = package['page_masters']['masters'][0]
    model = layout_model(fixture)
    require(len(proof['pages']) == model['pages'], 'source chapter page count')
    formulae = {row['node_id']:row for row in proof['placements']}
    numbers = {row['node_id']:row for row in proof['equation_numbers']}
    require(set(formulae) == {row['node_id'] for row in model['formula_positions']}
            and set(numbers) == {row['node_id'] for row in model['number_positions']}, 'complete source layout owners')
    maximum_error = 0
    for want in model['formula_positions']:
        got = formulae[want['node_id']]
        error = max([abs(got['baseline_raw']-want['baseline'])]+[abs(a-b) for a,b in zip(got['bounds_raw'],
            [want['x'],master['height']-want['y']-want['height'],want['x']+want['width'],master['height']-want['y']])])
        maximum_error = max(maximum_error,error)
        require(got['page_index'] == want['page_index'] and error <= 1,
                'source-calculated formula line and page position at node '+str(want['node_id']))
    for want in model['number_positions']:
        got = numbers[want['node_id']];first = got['glyphs'][0]['world_matrix']
        error = max(abs(fixed(Decimal(first[4]))-want['x']),abs(master['height']-fixed(Decimal(first[5]))-want['baseline']))
        maximum_error = max(maximum_error,error)
        require(got['page_index'] == want['page_index'] and got['source_width_raw'] == want['width'] and error <= 1,
                'source-calculated equation number position at node '+str(want['node_id']))
    return {'passed':True,'source_model':model,'source_page_breaks':model['pages']-1,
            'inline_lines':80,'formula_positions':400,'number_positions':200,
            'coordinate_tolerance_raw':1,'maximum_observed_coordinate_error_raw':maximum_error}


def render(fixture, job, pdf, proof, output, *, tools):
    """Original SVG/font reference at independently calculated chapter positions."""
    import tempfile
    from decimal import Decimal
    from fractions import Fraction
    from PIL import Image
    from pypdf import PdfWriter
    from pypdf.generic import ArrayObject,DecodedStreamObject,DictionaryObject,NameObject,NumberObject
    from vmb_book_font import TrueType,reference_font
    from vmb_svg_reference import ExactNumber,source_form
    from verify_vmb_scale_render import DPIS,POLICY,command,compare_masks,crop_box,ink,original_svg
    require(all(not output.resolve().is_relative_to(Path(path).resolve()) for path in (fixture,job)),
            'chapter raster output outside source inputs')
    require(proof['pdf_sha256'] == digest(pdf.read_bytes()) and proof['source']['package_sha256']
            == digest(contained_bytes(job,'document-package.json')), 'chapter raster binds exact checked source and PDF')
    verify_job(fixture,job);layout = verify_layout(fixture,proof)['source_model']
    facts = source_inputs(fixture);package,_,events = source_wire(fixture,facts)
    font = TrueType(contained_bytes(fixture,FONT_FILE))
    master = package['page_masters']['masters'][0]
    width,height = Decimal(master['width'])/65536,Decimal(master['height'])/65536
    by_owner = {event['node']['node_id']:event for event in events}
    output.mkdir();logs,results,failures = [],[],[]
    grouped = {page:[] for page in range(layout['pages'])}
    for want in layout['formula_positions']:
        grouped[want['page_index']].append({'kind':'formula',**want})
    for want in layout['number_positions']:
        grouped[want['page_index']].append({'kind':'number',**want})
    original_font_ascent = font.ascender*786432//font.units
    leading = round(Fraction(917504-(font.ascender-font.descender)*786432//font.units,2))
    with tempfile.TemporaryDirectory(prefix='render-',dir=output) as directory:
        scratch = Path(directory);forms = {}
        def form_for(image_id):
            if image_id not in forms:
                case = facts['cases'][image_id]
                source_case = {'tex':contained_bytes(fixture,case['source_tex_file']).decode(),
                    'speech':case['alt'],'display':case['display'],'font_size_raw':case['font_size_raw'],'metrics':case['metrics'],
                    'path_count':case['expected_path_count'],'segment_count':case['expected_stored_segments']-5}
                raw,_ = original_svg(contained_bytes(fixture,case['source_svg_file']),contained_bytes(fixture,case['admitted_svg_file']),
                    source_case,expected_font_size_raw=case['font_size_raw'])
                viewport = case['metrics']['viewport']
                forms[image_id] = source_form(raw,Decimal(viewport['width'])/65536,Decimal(viewport['height'])/65536,
                    scratch,'source-'+str(image_id),tools['rsvg-convert'],lambda argv:command(argv,logs),
                    source_to_reference_scale=Decimal(case['font_size_raw'])*4096/(65536*65536))
            return forms[image_id]
        for page_index,rows in grouped.items():
            writer = PdfWriter();writer.pdf_header = b'%PDF-1.7'
            page = writer.add_blank_page(width=float(width),height=float(height))
            page[NameObject('/MediaBox')] = ArrayObject([NumberObject(0),NumberObject(0),ExactNumber(width),ExactNumber(height)])
            objects = DictionaryObject();font_ref = reference_font(writer,font,'()0123456789')
            data = [f'q\n1 0 0 -1 0 {height} cm\n0 0 0 rg\n']
            def point(raw):return format(Decimal(raw)/65536,'f')
            for row in rows:
                if row['kind'] == 'formula':
                    image_id = by_owner[row['node_id']]['node']['image_id'];name = '/R'+str(image_id)
                    if NameObject(name) not in objects:objects[NameObject(name)] = writer._add_object(form_for(image_id).clone(writer))
                    data.append(f'q\n1 0 0 1 {point(row["x"])} {point(row["y"])} cm\n{name} Do\nQ\n')
                else:
                    x = row['x']
                    for char in row['text']:
                        gid = font.codepoint(ord(char))
                        data.append(f'q\n0 g\nBT /RN 12 Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n'
                            f'1 0 0 -1 {point(x)} {point(row["baseline"])} Tm <{gid:04X}> Tj\nET\nQ\n')
                        advance = Fraction(font.metrics(gid)[0]*786432,font.units)
                        require(advance.denominator == 1, 'source reference number glyph advance')
                        x += int(advance)
            data.append('Q\n');stream = DecodedStreamObject();stream.set_data(''.join(data).encode())
            page[NameObject('/Contents')] = writer._add_object(stream.flate_encode())
            page[NameObject('/Resources')] = DictionaryObject({NameObject('/XObject'):objects,
                NameObject('/Font'):DictionaryObject({NameObject('/RN'):font_ref})})
            reference_pdf = scratch/('reference-'+str(page_index)+'.pdf');writer.write(reference_pdf)
            for dpi in DPIS:
                for renderer in ('MuPDF','Poppler'):
                    paths = []
                    for name,source,ordinal in [('reference',reference_pdf,1),('observed',pdf,page_index+1)]:
                        path = scratch/(name+'.png');paths.append(path)
                        argv = ([tools['mutool'],'draw','-q','-r',str(dpi),'-c','rgb','-o',str(path),str(source),str(ordinal)]
                            if renderer == 'MuPDF' else [tools['pdftoppm'],'-f',str(ordinal),'-l',str(ordinal),'-r',str(dpi),
                                '-singlefile','-png',str(source),str(path.with_suffix(''))])
                        command(argv,logs)
                    with Image.open(paths[0]) as image:reference_mask = ink(image)
                    with Image.open(paths[1]) as image:observed = ink(image)
                    require(reference_mask.size == observed.size, 'unaltered chapter renderer page dimensions')
                    for row in rows:
                        if row['kind'] == 'formula':bounds = [row['x'],master['height']-row['y']-row['height'],
                            row['x']+row['width'],master['height']-row['y']]
                        else:
                            top = row['baseline']-original_font_ascent-leading
                            bounds = [row['x'],master['height']-top-917504,row['x']+row['width'],master['height']-top]
                        inner = crop_box(bounds,height,dpi);box = (inner[0]-2,inner[1]-2,inner[2]+2,inner[3]+2)
                        require(box[0] >= 0 and box[1] >= 0 and box[2] <= observed.width and box[3] <= observed.height,
                                'complete chapter source viewport and two-pixel outer ring')
                        reference_crop,observed_crop = reference_mask.crop(box),observed.crop(box)
                        counts = compare_masks(reference_crop,observed_crop,(2,2,box[2]-box[0]-2,box[3]-box[1]-2))
                        if row['kind'] == 'number':require(counts['reference_ink'] > 0, 'visible original-font number reference')
                        result = {'node_id':row['node_id'],'kind':row['kind'],'page_index':page_index,'renderer':renderer,'dpi':dpi,
                            'crop_pixels':list(box),'reference_mask_sha256':digest(reference_crop.tobytes()),
                            'observed_mask_sha256':digest(observed_crop.tobytes()),**counts}
                        results.append(result)
                        if not counts['passed']:
                            if len(failures) < 12:
                                stem=f'failure-{page_index}-{row["node_id"]}-{dpi}-{renderer}'
                                reference_crop.save(output/(stem+'-reference.png'));observed_crop.save(output/(stem+'-observed.png'))
                            failures.append(result)
    for name,value in [('commands.json',logs),('comparisons.json',results),('failures.json',failures)]:
        (output/name).write_bytes(canonical_bytes(value))
    summary = {'algorithm':'typaxis.vmb-chapter-raw-source-render/1','policy':POLICY,'pages':layout['pages'],
        'comparisons':len(results),'failed':len(failures),'math_comparisons':sum(row['kind'] == 'formula' for row in results),
        'number_comparisons':sum(row['kind'] == 'number' for row in results),
        'mask_exact':sum(row['reference_mask_sha256'] == row['observed_mask_sha256'] for row in results),
        'source_font_sha256':FONT_SHA,'pdf_sha256':digest(pdf.read_bytes()),'placement_reference':'independent chapter source layout /1',
        'font_reference':'Unchanged original sfnt bytes and original cmap/hmtx; no target subset data.',
        'passed':len(results) == 3600 and not failures}
    (output/'summary.json').write_bytes(canonical_bytes(summary))
    require(summary['passed'], 'all original-source chapter raster comparisons pass')
    return summary


def extraction_tokens(events):
    return [{'node_id':row['node']['node_id'],'group':row['flow_group'],
             'text':(row['node']['actual_text'] or row['node']['alt']) if row['kind'] == 'formula' else row['text']}
            for row in events]


def verify_extraction(raw, expected, extractor):
    """Preserve literal speech; normalize only renderer-owned source boundaries."""
    require(expected and len({row['node_id'] for row in expected}) == len(expected), 'complete unique source extraction owners')
    value = raw.decode('utf-8').replace('\r\n','\n').replace('\r','\n');offset = 0
    while offset < len(value) and value[offset] in ' \t\n\f':offset += 1
    boundaries = 0
    for ordinal,row in enumerate(expected):
        require(value.startswith(row['text'],offset), extractor+': exact source semantic string '+str(ordinal))
        offset += len(row['text']);start = offset
        while offset < len(value) and value[offset] in ' \t\n\f':offset += 1
        if ordinal+1 < len(expected) and expected[ordinal+1]['group'] != row['group']:
            require(offset > start, extractor+': source flow-group boundary after '+str(ordinal));boundaries += 1
    require(offset == len(value), extractor+': no unowned extraction text')
    return {'source_strings':len(expected),'source_flow_groups':len({row['group'] for row in expected}),
            'required_group_boundaries':boundaries,'source_payloads_preserved':True}


def damage_controls(fixture, job, public_pdf, output):
    import re
    from decimal import Decimal
    from pypdf import PdfReader,PdfWriter
    from pypdf.generic import DecodedStreamObject
    from verify_vmb_scale_render import precise_matrices
    from vmb_book_pdf import verify_source_structure
    require(all(not output.resolve().is_relative_to(Path(path).resolve()) for path in (fixture,job)),
            'chapter damage output outside source inputs')
    output.mkdir()
    baseline = output/'unchanged-clone.pdf';PdfWriter(clone_from=public_pdf).write(baseline)
    proof = verify_pdf(fixture,job,baseline);verify_layout(fixture,proof)
    facts = source_inputs(fixture);package,_,_ = source_wire(fixture,facts)
    original_matrices = precise_matrices(baseline)
    graph = verify_source_structure(baseline,package)
    bindings = {row['node_id']:row for row in graph['source_mcr_bindings']}
    numbers = {row['text']:row for row in proof['equation_numbers']}
    number_cids = {row['unicode']:row['cid'] for row in proof['fonts'][0]['cids']}
    marker = re.compile(rb'/(Formula|Span) << /MCID ([0-9]+) /Lang <[0-9A-F]+> >> BDC\n')
    glyph = re.compile(rb'1 0 0 -1 (-?[0-9.]+) (-?[0-9.]+) Tm <([0-9A-F]{4})> Tj')
    controls = []
    for label in ('(1)','(99)','(200)'):
        source = numbers[label];page_index = source['page_index'];mcid = bindings[source['node_id']]['mcid']
        for kind in ('removed','substituted','shifted'):
            writer = PdfWriter(clone_from=baseline);page = writer.pages[page_index];data = page.get_contents().get_data()
            markers = list(marker.finditer(data))
            match = next(row for row in markers if int(row[2]) == mcid)
            require(match[1] == b'Span', 'private source number scope')
            index = markers.index(match);end = markers[index+1].start() if index+1 < len(markers) else len(data)
            segment = data[match.start():end];targets = list(glyph.finditer(segment))
            require(len(targets) == len(label), 'target only the authored number glyphs')
            target = targets[0]
            if kind == 'removed':
                original = b'<'+target[3]+b'> Tj';replacement = b''
            elif kind == 'substituted':
                original = b'<'+target[3]+b'> Tj';replacement = f'<{number_cids["9"]:04X}> Tj'.encode()
            else:
                original = target[0]
                replacement = b'1 0 0 -1 '+target[1]+b' '+format(Decimal(target[2].decode())+1,'f').encode()+b' Tm <'+target[3]+b'> Tj'
            require(segment.count(original) >= 1, 'exact private target bytes')
            changed = segment.replace(original,replacement,1)
            modified = data[:match.start()]+changed+data[end:]
            stream = DecodedStreamObject();stream.set_data(modified);page.replace_contents(stream)
            pdf = output/(label[1:-1]+'-'+kind+'.pdf');writer.write(pdf)
            require(precise_matrices(pdf) == original_matrices, 'all original formula matrices preserved by number damage')
            require(verify_source_structure(pdf,package) == graph, 'complete source graph preserved by number damage')
            try:verify_pdf(fixture,job,pdf)
            except ValueError as error:rejection = str(error)
            else:raise ValueError('number damage passed source paint verification: '+kind)
            controls.append({'label':label,'kind':kind,'pdf':pdf.name,'pdf_sha256':digest(pdf.read_bytes()),
                'source_paint_rejected':True,'error':rejection,'formula_matrices_preserved':True,'source_graph_preserved':True})
    # Move an inline Formula and its supplemental anchor together. Its internal
    # source geometry remains valid, so the independent layout must reject it.
    writer = PdfWriter(clone_from=baseline);page = writer.pages[0];data = page.get_contents().get_data()
    markers = list(marker.finditer(data));first = markers[0];end = markers[1].start()
    segment = data[first.start():end]
    anchor = re.search(rb'(-?[0-9.]+) (-?[0-9.]+) Tm <00> Tj',segment)
    form = re.search(rb'1 0 0 1 (-?[0-9.]+) (-?[0-9.]+) cm\n/V[0-9]+ Do',segment)
    require(anchor is not None and form is not None, 'actual inline anchor and source Form target')
    for match in sorted((anchor,form),key=lambda row:row.start(1),reverse=True):
        new = format(Decimal(match[1].decode())+1,'f').encode()
        segment = segment[:match.start(1)]+new+segment[match.end(1):]
    stream = DecodedStreamObject();stream.set_data(data[:first.start()]+segment+data[end:]);page.replace_contents(stream)
    pdf = output/'translated-inline.pdf';writer.write(pdf)
    require(verify_source_structure(pdf,package) == graph, 'source graph retained by translated inline control')
    private = verify_pdf(fixture,job,pdf)
    try:verify_layout(fixture,private)
    except ValueError as error:rejection = str(error)
    else:raise ValueError('source layout accepted translated inline Formula')
    controls.append({'kind':'translated-inline','pdf':pdf.name,'pdf_sha256':digest(pdf.read_bytes()),
        'source_paint_passed':True,'independent_layout_rejected':True,'error':rejection,'source_graph_preserved':True})
    result = {'algorithm':'typaxis.vmb-chapter-private-controls/1','passed':True,'baseline_clone_passed':True,
        'number_controls':9,'layout_controls':1,'controls':controls,
        'scope':'Private PDFs with unchanged source semantics/structure; glyph/source geometry and independent layout are mandatory.'}
    (output/'observed.json').write_bytes(canonical_bytes(result))
    return result


def run_case(args, executor, selected, validators, helpers):
    sha,write_json,run_public_builds,ua_check,checked = helpers
    case = args.output_root/'chapter-300-500';case.mkdir()
    result = {'algorithm':'typaxis.vmb-book-case-observed/1','case_id':'chapter-300-500','passed':False,'phases':{}}
    try:
        job = case/'job';result['phases']['source'] = prepare(args.fixture_root,job)
        result['phases']['public_commands'] = run_public_builds(args.typaxis,job,case,executor,validators)
        proof = verify_pdf(args.fixture_root,job,case/'output.pdf',case/'source-paint-structure.json')
        result['phases']['source_paint_structure_font'] = {'passed':True,'formulas':400,'equation_numbers':200,'glyphs':892,
            'sha256':sha(case/'source-paint-structure.json')}
        layout = verify_layout(args.fixture_root,proof);write_json(case/'source-layout.json',layout)
        result['phases']['source_layout'] = {key:value for key,value in layout.items() if key != 'source_model'}
        _,_,events = source_wire(args.fixture_root,source_inputs(args.fixture_root));expected = extraction_tokens(events)
        write_json(case/'expected-extraction.json',expected)
        extracted = {}
        for name,tail in [('pdftotext',['-enc','UTF-8','-raw',case/'output.pdf','-']),
                          ('mutool',['draw','-q','-F','txt',case/'output.pdf'])]:
            _,out,_ = checked(executor,'chapter-extract-'+name,[selected['paths'][name],*tail],stdout=True)
            extracted[name] = verify_extraction(out,expected,name)
        result['phases']['extraction'] = extracted
        result['phases']['raw_source_render'] = render(args.fixture_root,job,case/'output.pdf',proof,case/'render',tools=selected['paths'])
        result['phases']['damage_controls'] = damage_controls(args.fixture_root,job,case/'output.pdf',case/'controls')
        result['phases']['pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,selected['paths'])
        result['passed'] = True
    except Exception as error:result['error'] = {'type':type(error).__name__,'message':str(error)}
    write_json(case/'observed.json',result)
    return result
