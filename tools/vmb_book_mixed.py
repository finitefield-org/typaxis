"""Source and original-glyph joins for authored mixed body/formula fixtures."""
import copy
from pathlib import Path

from vmb_book_inputs import canonical_bytes,digest,read_canonical,require,verify_fixture_inputs
from vmb_probe_io import contained_bytes,same_json

FONT_FILE = 'mixed-body-spacing/ipaexm.ttf'
FONT_SHA = '7a306386f930fee80922f71eebf4ffe0f1ff2817da8e619230953487673d71c7'
SUPPORT = {'id':'mixed-body-spacing-1','plan':'mixed-body-spacing/plan.json',
           'provenance':'mixed-body-spacing/provenance.json','cases':6}
PROVENANCE = {'engine_id':'vmb-go-math-svg','engine_version':'2.0.0','rules_version':'vmb.typaxis-safe-svg/1'}
CASES = {'ascii-before': [['p', ['A ', ['inline', 'fraction-inline-720896'], 'B']]], 'ascii-after': [['p', ['A ', ['inline', 'fraction-inline-720896'], ' B']]], 'ascii-double': [['p', ['A  ', ['inline', 'fraction-inline-720896'], '  B']]], 'japanese-adjacent': [['p', ['本文', ['inline', 'equivalence-inline-720896'], '前後']]], 'consecutive-formulas': [['p', ['A', ['inline', 'fraction-inline-720896'], ['inline', 'negation-inline-786431'], 'B']]], 'block-page': [['p', ['A B']], ['block', 'fraction-block-786431'], ['page', None], ['p', ['本文', ['inline', 'fraction-inline-720896'], '前後']]]}


def source_inputs(fixture):
    facts=verify_fixture_inputs(fixture)
    index=read_canonical(fixture,'fixture-index.json')
    require(same_json(index['provenance'].get('mixed_body_spacing_support'),SUPPORT),'registered mixed body support')
    provenance=read_canonical(fixture,SUPPORT['provenance']);font=provenance['font']
    require(provenance['algorithm']=='typaxis.vmb-mixed-body-support/1'
            and provenance['kind']=='authored-real-engine-mixed-body-fixtures'
            and provenance['plan']==SUPPORT['plan']
            and provenance['plan_sha256']==digest(contained_bytes(fixture,SUPPORT['plan'])), 'authored mixed body provenance')
    require(font['file']==FONT_FILE and font['job_uri']=='fonts/ipaexm.ttf' and font['family']=='IPAexMincho'
            and font['media_type']=='sfnt-truetype-glyf' and type(font['face_index']) is int and font['face_index']==0
            and font['sha256']==digest(contained_bytes(fixture,FONT_FILE))==FONT_SHA
            and type(font['byte_length']) is int and font['byte_length']==7835672
            and font['version']=='004.01' and font['original_book_font'] is False,'unchanged original mixed body font')
    for role in ('license','readme'):
        require(font[role+'_file'] in index['provenance']['license_and_notice_files']
                and font[role+'_sha256']==digest(contained_bytes(fixture,font[role+'_file'])), 'original IPA font notice '+role)
    plan=read_canonical(fixture,SUPPORT['plan'])
    require(set(plan)=={'algorithm','id','source_case_ledger','source_case_ledger_sha256','cases','original_book','approved_layout_golden'}
            and plan['algorithm']=='typaxis.vmb-mixed-body-plan/1' and plan['id']==SUPPORT['id']
            and plan['source_case_ledger']=='cases.json' and plan['source_case_ledger_sha256']==digest(contained_bytes(fixture,'cases.json'))
            and same_json(plan['cases'],CASES)
            and plan['original_book'] is False and plan['approved_layout_golden'] is False,'fixed authored mixed body source plan')
    return {'fixture_index_sha256':facts['index_sha256'],'plan':plan,'plan_sha256':provenance['plan_sha256'],'font':font,
            'cases':read_canonical(fixture,'cases.json')}


def source_wire(fixture,facts,name):
    require(name in CASES,'registered mixed body case')
    package = read_canonical(fixture, 'package-template.json')
    cases = read_canonical(fixture, 'cases.json')
    by_id = {case['case_id']:(index,case) for index,case in enumerate(cases)}
    package['resources']['font_faces'] = [{'font_face_id':0,'family':'IPAexMincho',
        'uri':'fonts/ipaexm.ttf','media_type':'sfnt-truetype-glyf','face_index':0,'expected_sha256':FONT_SHA}]
    for rule in package['style_sheet']['rules']:
        for declaration in rule['declarations']:
            if declaration['name'] == 'font_family': declaration['value']['families'] = ['IPAexMincho']
    for image_id,case in enumerate(cases):
        package['resources']['images'].append({'image_id':image_id,'uri':case['admitted_svg_file'],
            'media_type':'svg-safe-2','expected_sha256':case['admitted_svg_sha256'],'vector_provenance':PROVENANCE.copy()})
    chunks, events, blocks = [], [], []
    offset, owner, group = 0, 2, 0
    def span(start, end): return {'source_id':0,'start_byte':start,'end_byte':end}
    def source(text):
        nonlocal offset
        raw = text.encode('utf-8'); start, text_id = offset, len(package['text_buffers'])
        end = offset + len(raw)
        package['text_buffers'].append({'text_id':text_id,'utf8':text,'mappings':[
            {'kind':'identity','source_span':span(start,end),'text_range':{'start_byte':0,'end_byte':len(raw)}}]})
        chunks.append(raw+b'\n'); offset = end+1
        return span(start,end), {'text_id':text_id,'start_byte':0,'end_byte':len(raw)}
    def leaf(item):
        nonlocal owner
        node_id = owner; owner += 1
        if isinstance(item,str):
            origin, text_span = source(item)
            node = {'kind':'text','node_id':node_id,'span':origin,'text_span':text_span}
            events.append({'node_id':node_id,'kind':'text','text':item,'group':group})
        else:
            kind, case_id = item; image_id, case = by_id[case_id]
            origin, text_span = source(contained_bytes(fixture,case['source_tex_file']).decode('utf-8'))
            node = {'kind':'math_vector' if kind == 'inline' else 'math_vector_block','node_id':node_id,
                'span':origin,'image_id':image_id,'metrics':copy.deepcopy(case['metrics']),
                'alt':case['alt'],'actual_text':case['actual_text'],'source_tex':{'text_span':text_span}}
            if kind == 'inline': node['spacing'] = copy.deepcopy(case['spacing'])
            else: node.update(classes=[],equation_number=None)
            events.append({'node_id':node_id,'kind':'formula','text':case['actual_text'] or case['alt'],
                           'case_id':case_id,'group':group})
        return node
    for kind, values in facts['plan']['cases'][name]:
        group += 1
        if kind == 'p':
            paragraph_id = owner; owner += 1; start = offset
            children = [leaf(value) for value in values]
            blocks.append({'kind':'paragraph','node_id':paragraph_id,'span':span(start,offset-1),
                           'classes':[],'children':children})
        elif kind == 'page':
            chunks.append(b'\n')
            blocks.append({'kind':'page_break','node_id':owner,'span':span(offset,offset+1),'classes':[]})
            offset += 1; owner += 1
        else: blocks.append(leaf((kind,values)))
    data = b''.join(chunks)
    package['sources'] = [{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(data),'sha256':digest(data)}]
    package['document']['blocks'] = [{'kind':'semantic_container','semantic_kind':'result','node_id':1,
        'span':span(0,len(data)-1),'classes':[],'anchor_id':None,'blocks':blocks}]
    package['metadata']['title'] = 'Authored mixed text/formula probe: '+name
    return package,data,events


def prepare(fixture,output,name):
    fixture,output=Path(fixture).resolve(),Path(output).resolve()
    require(not output.is_relative_to(fixture) and not fixture.is_relative_to(output),'mixed body output outside source fixture')
    facts=source_inputs(fixture);package,data,events=source_wire(fixture,facts,name)
    output.mkdir()
    copies=[(FONT_FILE,facts['font']['job_uri'])]+[(case['admitted_svg_file'],case['admitted_svg_file']) for case in facts['cases']]
    for origin,target in copies:
        path=output/target;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(contained_bytes(fixture,origin))
    (output/'input.tsf').write_bytes(data);(output/'document-package.json').write_bytes(canonical_bytes(package))
    return verify_job(fixture,output,name)


def verify_job(fixture,job,name):
    facts=source_inputs(fixture);package,data,events=source_wire(fixture,facts,name)
    require(contained_bytes(job,'document-package.json')==canonical_bytes(package),'exact authored mixed body package')
    require(contained_bytes(job,'input.tsf')==data,'exact authored body spaces and TeX source bytes')
    require(digest(contained_bytes(job,facts['font']['job_uri']))==FONT_SHA,'unchanged mixed body source font')
    for case in facts['cases']:
        require(digest(contained_bytes(job,case['admitted_svg_file']))==case['admitted_svg_sha256'],'unchanged real-engine mixed body SVG')
    require({str(path.relative_to(job)) for path in job.rglob('*') if path.is_file() or path.is_symlink()}
            =={'document-package.json','input.tsf',facts['font']['job_uri']}|{case['admitted_svg_file'] for case in facts['cases']},'complete mixed body job inventory')
    return {'algorithm':'typaxis.vmb-mixed-body-job/1','case_id':name,'fixture_index_sha256':facts['fixture_index_sha256'],
        'plan_sha256':facts['plan_sha256'],'font_sha256':FONT_SHA,'semantic_owners':len(events),
        'formulas':sum(row['kind']=='formula' for row in events),'body_nodes':sum(row['kind']=='text' for row in events),
        'package_sha256':digest(contained_bytes(job,'document-package.json'))}


def layout_model(fixture,name):
    """Single-line source plans and an explicit page break; no PDF/trace input."""
    from fractions import Fraction
    from vmb_book_font import TrueType
    facts=source_inputs(fixture);package,_,events=source_wire(fixture,facts,name)
    font=TrueType(contained_bytes(fixture,FONT_FILE));size=786432
    by_owner={event['node_id']:event for event in events}
    master=package['page_masters']['masters'][0];body=master['body']
    styles={}
    for rule in package['style_sheet']['rules']:
        values=styles.setdefault(rule['selector'],{})
        require(rule['extends'] is None,'flat mixed fixture source styles')
        for declaration in rule['declarations']:
            require(declaration['important'] is False,'ordinary mixed source declaration')
            values[declaration['name']]=declaration['value']
    def length(selector,key):
        value=styles[selector][key]
        require(value['kind']=='length' and type(value['value']) is int,'exact mixed source style length')
        return value['value']
    require(length('paragraph','font_size')==length('semantic_container','font_size')==size
            and length('paragraph','line_height')==917504
            and styles['paragraph']['text_align']=={'kind':'keyword','value':'start'}
            and styles['math_vector_block']['text_align']=={'kind':'keyword','value':'center'},'fixed mixed source styles')
    def advance(char):
        value=Fraction(font.metrics(font.codepoint(ord(char)))[0]*size,font.units)
        require(value.denominator==1,'source body advance on fixed grid')
        return int(value)
    source_asc=font.ascender*size//font.units;source_desc=-font.descender*size//font.units
    rows=[];page=0;used=after=0
    for block in package['document']['blocks'][0]['blocks']:
        if block['kind']=='page_break':page+=1;used=after=0;continue
        selector='paragraph' if block['kind']=='paragraph' else 'math_vector_block'
        before=0 if used==0 else after+length(selector,'space_before')
        top=body['y']+used+before
        if block['kind']=='paragraph':
            children=block['children'];asc,desc=source_asc,source_desc
            pen,left,right=0,0,0;members=[]
            for child in children:
                event=by_owner[child['node_id']]
                if child['kind']=='text':
                    width=sum(advance(char) for char in event['text'])
                    members.append({'node_id':child['node_id'],'kind':'text','pen':pen,'text':event['text'],'width':width})
                else:
                    metric=child['metrics'];width=metric['advance']
                    asc=max(asc,metric['ascent']);desc=max(desc,metric['descent'])
                    left=min(left,pen+metric['origin_x']);right=max(right,pen+metric['origin_x']+metric['viewport']['width'])
                    members.append({'node_id':child['node_id'],'kind':'formula','pen':pen,'metric':metric,'image_id':child['image_id']})
                    require(child['spacing']=={'before':0,'after':0},'registered mixed formula spacing remains zero')
                pen+=width;right=max(right,pen)
            height=max(asc+desc,length(selector,'line_height'))
            baseline=top+asc+round(Fraction(height-asc-desc,2))
            require(right-left<=body['width']-length(selector,'start_indent')-length(selector,'end_indent'),
                    'authored mixed paragraph fits one source line')
            origin=body['x']+length(selector,'start_indent')-left
            for row in members:
                row.update(page_index=page,baseline=baseline,x=origin+row.pop('pen'))
                if row['kind']=='formula':
                    metric=row.pop('metric');row['x']+=metric['origin_x']
                    row.update(y=baseline-metric['baseline'],width=metric['viewport']['width'],height=metric['viewport']['height'])
                else:
                    row.update(y=top,height=height,glyph_positions=[])
                    x=row['x']
                    for char in row['text']:
                        row['glyph_positions'].append({'unicode':char,'source_gid':font.codepoint(ord(char)),
                                                      'x':x,'baseline':baseline})
                        x+=advance(char)
                rows.append(row)
        else:
            metric=block['metrics'];height=metric['viewport']['height']
            rows.append({'node_id':block['node_id'],'kind':'formula','page_index':page,'image_id':block['image_id'],
                'x':body['x']+(body['width']-metric['viewport']['width'])//2,'y':top,
                'baseline':top+metric['baseline'],'width':metric['viewport']['width'],'height':height})
        used+=before+height;after=length(selector,'space_after')
        require(used<=body['height'],'source mixed content fits the declared page')
    return {'algorithm':'typaxis.vmb-mixed-source-layout/1','case_id':name,'pages':page+1,'placements':rows,
            'scope':'Original simple-glyph advances and formula metrics; single source lines and explicit page break.'}


def verify_pdf(fixture,job,pdf,name):
    """Join all actual body glyphs, Forms, semantic owners and source positions."""
    from decimal import Decimal
    from fractions import Fraction
    from pypdf import PdfReader
    from pypdf.generic import ContentStream
    from verify_vmb_scale_pdf import PagePaint,fixed,multiply,verify_form_geometry,verify_placement
    from vmb_book_font import TrueType,verify_text_font
    from vmb_book_pdf import integer,verify_source_structure
    source=verify_job(fixture,job,name);facts=source_inputs(fixture)
    package,_,events=source_wire(fixture,facts,name)
    graph=verify_source_structure(pdf,package,body_text=True)
    bindings={row['node_id']:row for row in graph['source_mcr_bindings']}
    require(set(bindings)=={row['node_id'] for row in events},'complete mixed source MCR owners')
    nodes={}
    def visit(value):
        if isinstance(value,dict):
            if 'node_id' in value:nodes[value['node_id']]=value
            for child in value.values():visit(child)
        elif isinstance(value,list):
            for child in value:visit(child)
    visit(package['document'])
    font=TrueType(contained_bytes(fixture,FONT_FILE))
    characters={char for event in events if event['kind']=='text' for char in event['text']}
    master=package['page_masters']['masters'][0];body=master['body'];reader=PdfReader(pdf,strict=True)
    model=layout_model(fixture,name);positions={row['node_id']:row for row in model['placements']}
    require(len(reader.pages)==model['pages'],'source mixed page count')
    font_models,form_models={},set();rows=[];index=0;maximum_error=0
    class BodyPaint(PagePaint):
        def __init__(self,resources):
            super().__init__(resources);self.font_name=None;self.font_stack=[]
        def step(self,args,op):
            if op==b'q':self.font_stack.append(self.font_name)
            if op==b'Q':
                require(self.font_stack,'body font state restore');self.font_name=self.font_stack.pop()
            if op==b'Tf':self.font_name=args[0]
            if op==b'Tj' and self.render_mode==0:
                raw=getattr(args[0],'original_bytes',None) if len(args)==1 else None
                require(self.in_text and isinstance(raw,bytes) and len(raw)==2 and self.black
                        and self.font_size==Decimal(12) and self.text_parameters=={b'Tc':0,b'Tw':0,b'Tz':100,b'TL':0,b'Ts':0},
                        'one original body glyph and exact source text state')
                return ('glyph',self.font_name,int.from_bytes(raw,'big'),multiply(self.matrix,self.text_matrix),self.clip)
            return super().step(args,op)
    def inside(rect,limit):
        require(rect[0]>=limit[0]-1 and rect[1]>=limit[1]-1 and rect[2]<=limit[2]+1 and rect[3]<=limit[3]+1,
                'source body glyph is fully visible in page body and clip')
    for page_index,page in enumerate(reader.pages):
        require(tuple(map(fixed,page.mediabox))==(0,0,master['width'],master['height'])
                and tuple(page.get('/CropBox',page.mediabox))==tuple(page.mediabox)
                and page.get('/Rotate',0)==0 and page.get('/UserUnit',1)==1 and not page.get('/Annots'),'mixed source page geometry')
        resources=page['/Resources'];paint=BodyPaint(resources);stack=[];active=None
        for args,op in ContentStream(page.get_contents(),reader).operations:
            event=paint.step(args,op)
            if op==b'BDC':
                props=args[1]
                if '/MCID' in props:
                    require(active is None and not stack and index<len(events),'one source mixed MCR scope')
                    want=events[index];binding=bindings[want['node_id']]
                    require(args[0]==('/Formula' if want['kind']=='formula' else '/Span')
                            and set(props)=={'/MCID','/Lang'} and str(props['/Lang'])==package['document']['language']
                            and integer(props['/MCID'])==binding['mcid']
                            and binding['page_object']==[page.indirect_reference.idnum,page.indirect_reference.generation],
                            'source mixed MCR role, language and page')
                    active={'want':want,'actual':[],'glyphs':[],'paint':[],'anchors':[]};stack.append('mcr')
                else:
                    require(active is not None and stack==['mcr'] and args[0]=='/Span' and set(props)=={'/ActualText'},'exact mixed ActualText owner')
                    active['actual'].append(str(props['/ActualText']));stack.append('actual')
            elif op==b'EMC':
                require(stack,'balanced mixed marked content');closed=stack.pop()
                if closed=='mcr':
                    require(active is not None and not stack,'complete mixed MCR')
                    want=active['want'];node=nodes[want['node_id']];position=positions[want['node_id']]
                    require(active['actual']==[want['text']],'exact authored mixed text including every space')
                    require(page_index==position['page_index'],'source mixed placement page')
                    if want['kind']=='formula':
                        require(len(active['paint'])==len(active['anchors'])==1 and not active['glyphs'],'one source mixed Form and anchor')
                        painting,form,key=active['paint'][0];image=package['resources']['images'][node['image_id']]
                        if (key,image['expected_sha256']) not in form_models:
                            verify_form_geometry(contained_bytes(job,image['uri']),form,reader);form_models.add((key,image['expected_sha256']))
                        rect=verify_placement(node,painting,active['anchors'],page.mediabox,body)
                        x,y=fixed(painting[1][4]),master['height']-fixed(painting[1][5])
                        baseline=master['height']-fixed(active['anchors'][0][5])
                        error=max(abs(x-position['x']),abs(y-position['y']),abs(baseline-position['baseline']))
                        rows.append({'node_id':want['node_id'],'kind':'formula','page_index':page_index,'bounds_raw':list(rect),
                                     'world_matrix':[str(value) for value in painting[1]],'source_position_error_raw':error})
                    else:
                        require(not active['paint'] and not active['anchors'] and len(active['glyphs'])==len(want['text']),
                                'all source body glyphs and no additional paint')
                        glyph_rows=[];error=0
                        for ordinal,(glyph,verified,key) in enumerate(active['glyphs']):
                            _,_,cid,matrix,clip=glyph;expected=position['glyph_positions'][ordinal]
                            mapped=next((item for item in verified['cids'] if item['cid']==cid),None)
                            require(mapped is not None and mapped['unicode']==expected['unicode']
                                    and matrix[:4]==tuple(map(Decimal,(1,0,0,1))),'painted body glyph matches original character')
                            x,baseline=fixed(matrix[4]),master['height']-fixed(matrix[5])
                            error=max(error,abs(x-expected['x']),abs(baseline-expected['baseline']))
                            if expected['unicode']!=' ':
                                bounds=font.glyph(mapped['source_gid'])['bbox']
                                rect=[Fraction(x)+Fraction(bounds[0]*786432,font.units),
                                      Fraction(master['height']-baseline)+Fraction(bounds[1]*786432,font.units),
                                      Fraction(x)+Fraction(bounds[2]*786432,font.units),
                                      Fraction(master['height']-baseline)+Fraction(bounds[3]*786432,font.units)]
                                inside(rect,(body['x'],master['height']-body['y']-body['height'],body['x']+body['width'],master['height']-body['y']))
                                if clip is not None:inside(rect,tuple(map(fixed,clip)))
                            glyph_rows.append({'unicode':expected['unicode'],'source_gid':mapped['source_gid'],'cid':cid,
                                               'font_object':list(key),'world_matrix':[str(value) for value in matrix]})
                        rows.append({'node_id':want['node_id'],'kind':'text','page_index':page_index,'glyphs':glyph_rows,
                                     'source_position_error_raw':error})
                    require(error<=1,'independent source mixed baseline, advance and placement at node '+str(want['node_id']))
                    maximum_error=max(maximum_error,error);index+=1;active=None
            elif op==b'BMC':raise ValueError('unexpected mixed artifact')
            if event is not None:
                require(active is not None and stack==['mcr','actual'],'all mixed paint belongs to authored source text')
                if event[0]=='anchor':active['anchors'].append(event[1])
                elif event[0]=='paint':
                    ref=resources['/XObject'].raw_get(args[0]);active['paint'].append((event,ref.get_object(),(ref.idnum,ref.generation)))
                else:
                    require(event[0]=='glyph','known source mixed paint kind')
                    ref=resources['/Font'].raw_get(event[1]);key=(ref.idnum,ref.generation)
                    if key not in font_models:font_models[key]=verify_text_font(ref.get_object(),font,characters)
                    active['glyphs'].append((event,font_models[key],key))
        paint.finish();require(not stack and active is None and not paint.font_stack,'complete mixed page state')
    require(index==len(events) and len(font_models)==1,'complete source mixed paint inventory')
    return {'algorithm':'typaxis.vmb-mixed-source-paint/1','source':source,'pdf_sha256':digest(Path(pdf).read_bytes()),
        'structure':graph,'placements':rows,'fonts':[{'pdf_font_object':list(key),**value} for key,value in sorted(font_models.items())],
        'source_layout':model,'maximum_observed_coordinate_error_raw':maximum_error,'coordinate_tolerance_raw':1,
        'body_glyphs':sum(len(row.get('glyphs',[])) for row in rows),'original_source_render_verified':False}


def verify_extraction(raw,events,extractor):
    """Preserve every authored space; permit framing only between flow groups."""
    require(extractor in ('MuPDF','Poppler'),'known mixed extractor')
    groups=[]
    for event in events:
        if not groups or groups[-1][0]!=event['group']:groups.append([event['group'],''])
        groups[-1][1]+=event['text']
    value=raw.decode('utf-8');offset=0
    for ordinal,(_,text) in enumerate(groups):
        require(value.startswith(text,offset),extractor+': exact mixed authored text and spaces at group '+str(ordinal))
        offset+=len(text)
        if ordinal+1<len(groups):
            start=offset
            while offset<len(value) and value[offset] in '\r\n\f':offset+=1
            require(offset>start,extractor+': required source flow-group boundary')
    # Neither leading/trailing ASCII spaces nor tabs are framing characters.
    require(all(char in '\r\n\f' for char in value[offset:]),extractor+': no unowned text or spaces')
    require('\f' in value[offset:],extractor+': terminal page framing')
    return {'source_strings':len(events),'source_flow_groups':len(groups),'authored_spaces':sum(text.count(' ') for _,text in groups),
            'raw_utf8_sha256':digest(raw),'all_authored_spaces_preserved':True}


def render(fixture,job,pdf,name,proof,output,*,tools):
    """Unchanged original font and raw SVG at source-calculated positions."""
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
            'mixed raster output outside source inputs')
    require(proof['pdf_sha256']==digest(Path(pdf).read_bytes()) and proof['source']==verify_job(fixture,job,name),
            'mixed raster binds the checked source and PDF')
    facts=source_inputs(fixture);package,_,events=source_wire(fixture,facts,name)
    layout=layout_model(fixture,name)
    require(same_json(layout,proof['source_layout']),'mixed raster uses independent source layout')
    font=TrueType(contained_bytes(fixture,FONT_FILE));characters={char for event in events if event['kind']=='text' for char in event['text']}
    master=package['page_masters']['masters'][0]
    width,height=Decimal(master['width'])/65536,Decimal(master['height'])/65536
    grouped={page:[] for page in range(layout['pages'])}
    for row in layout['placements']:grouped[row['page_index']].append(row)
    output.mkdir();logs,results,failures=[],[],[]
    with tempfile.TemporaryDirectory(prefix='render-',dir=output) as directory:
        scratch=Path(directory);forms={}
        def form_for(image_id):
            if image_id not in forms:
                case=facts['cases'][image_id]
                source_case={'tex':contained_bytes(fixture,case['source_tex_file']).decode(),
                    'speech':case['alt'],'display':case['display'],'font_size_raw':case['font_size_raw'],'metrics':case['metrics'],
                    'path_count':case['expected_path_count'],'segment_count':case['expected_stored_segments']-5}
                raw,_=original_svg(contained_bytes(fixture,case['source_svg_file']),contained_bytes(fixture,case['admitted_svg_file']),
                    source_case,expected_font_size_raw=case['font_size_raw'])
                viewport=case['metrics']['viewport']
                forms[image_id]=source_form(raw,Decimal(viewport['width'])/65536,Decimal(viewport['height'])/65536,
                    scratch,'source-'+str(image_id),tools['rsvg-convert'],lambda argv:command(argv,logs),
                    source_to_reference_scale=Decimal(case['font_size_raw'])*4096/(65536*65536))
            return forms[image_id]
        for page_index,rows in grouped.items():
            writer=PdfWriter();writer.pdf_header=b'%PDF-1.7'
            page=writer.add_blank_page(width=float(width),height=float(height))
            page[NameObject('/MediaBox')]=ArrayObject([NumberObject(0),NumberObject(0),ExactNumber(width),ExactNumber(height)])
            objects=DictionaryObject();font_ref=reference_font(writer,font,characters,name='IPAexMincho')
            data=[f'q\n1 0 0 -1 0 {height} cm\n0 0 0 rg\n']
            def point(raw):return format(Decimal(raw)/65536,'f')
            for row in rows:
                if row['kind']=='formula':
                    image_id=row['image_id'];resource='/R'+str(image_id)
                    if NameObject(resource) not in objects:objects[NameObject(resource)]=writer._add_object(form_for(image_id).clone(writer))
                    data.append(f'q\n1 0 0 1 {point(row["x"])} {point(row["y"])} cm\n{resource} Do\nQ\n')
                else:
                    for glyph in row['glyph_positions']:
                        data.append(f'q\n0 g\nBT /RB 12 Tf 0 Tr\n0 Tc 0 Tw 100 Tz 0 TL 0 Ts\n'
                            f'1 0 0 -1 {point(glyph["x"])} {point(glyph["baseline"])} Tm <{glyph["source_gid"]:04X}> Tj\nET\nQ\n')
            data.append('Q\n');stream=DecodedStreamObject();stream.set_data(''.join(data).encode())
            page[NameObject('/Contents')]=writer._add_object(stream.flate_encode())
            page[NameObject('/Resources')]=DictionaryObject({NameObject('/XObject'):objects,
                NameObject('/Font'):DictionaryObject({NameObject('/RB'):font_ref})})
            reference_pdf=scratch/('reference-'+str(page_index)+'.pdf');writer.write(reference_pdf)
            for dpi in DPIS:
                for renderer in ('MuPDF','Poppler'):
                    paths=[]
                    for label,source,ordinal in [('reference',reference_pdf,1),('observed',pdf,page_index+1)]:
                        path=scratch/(label+'.png');paths.append(path)
                        argv=([tools['mutool'],'draw','-q','-r',str(dpi),'-c','rgb','-o',str(path),str(source),str(ordinal)]
                            if renderer=='MuPDF' else [tools['pdftoppm'],'-f',str(ordinal),'-l',str(ordinal),'-r',str(dpi),
                                '-singlefile','-png',str(source),str(path.with_suffix(''))])
                        command(argv,logs)
                    with Image.open(paths[0]) as image:reference_mask=ink(image)
                    with Image.open(paths[1]) as image:observed_mask=ink(image)
                    require(reference_mask.size==observed_mask.size,'unaltered mixed renderer dimensions')
                    for row in rows:
                        bounds=[row['x'],master['height']-row['y']-row['height'],row['x']+row['width'],master['height']-row['y']]
                        inner=crop_box(bounds,height,dpi);box=(inner[0]-2,inner[1]-2,inner[2]+2,inner[3]+2)
                        require(box[0]>=0 and box[1]>=0 and box[2]<=observed_mask.width and box[3]<=observed_mask.height,
                                'mixed source viewport and full two-pixel ring')
                        reference_crop,observed_crop=reference_mask.crop(box),observed_mask.crop(box)
                        counts=compare_masks(reference_crop,observed_crop,(2,2,box[2]-box[0]-2,box[3]-box[1]-2))
                        require(counts['reference_ink']>0,'nonempty original body/formula ink in source crop')
                        result={'node_id':row['node_id'],'kind':row['kind'],'page_index':page_index,'renderer':renderer,'dpi':dpi,
                            'crop_pixels':list(box),'reference_mask_sha256':digest(reference_crop.tobytes()),
                            'observed_mask_sha256':digest(observed_crop.tobytes()),**counts}
                        results.append(result)
                        if not counts['passed']:
                            if len(failures)<12:
                                stem=f'failure-{page_index}-{row["node_id"]}-{dpi}-{renderer}'
                                reference_crop.save(output/(stem+'-reference.png'));observed_crop.save(output/(stem+'-observed.png'))
                            failures.append(result)
    for filename,value in [('commands.json',logs),('comparisons.json',results),('failures.json',failures)]:
        (output/filename).write_bytes(canonical_bytes(value))
    summary={'algorithm':'typaxis.vmb-mixed-original-source-render/1','policy':POLICY,'case_id':name,
        'pages':layout['pages'],'comparisons':len(results),'failed':len(failures),
        'math_comparisons':sum(row['kind']=='formula' for row in results),'body_comparisons':sum(row['kind']=='text' for row in results),
        'mask_exact':sum(row['reference_mask_sha256']==row['observed_mask_sha256'] for row in results),
        'source_font_sha256':FONT_SHA,'pdf_sha256':digest(Path(pdf).read_bytes()),
        'reference':'Unchanged original sfnt plus raw SVG /5, placed from the independent source model.',
        'passed':len(results)==len(layout['placements'])*6 and not failures}
    (output/'summary.json').write_bytes(canonical_bytes(summary))
    require(summary['passed'],'all original mixed body/formula raster comparisons pass')
    return summary


def damage_controls(fixture,job,pdf,name,output,*,executor,tools,checked):
    """Keep source structure/Form matrices while damaging body glyphs or spaces."""
    import re
    from pypdf import PdfReader,PdfWriter
    from pypdf.generic import DecodedStreamObject
    from vmb_book_pdf import verify_source_structure
    from verify_vmb_scale_render import precise_matrices
    require(all(not output.resolve().is_relative_to(Path(path).resolve()) for path in (fixture,job)),
            'mixed damage output outside source inputs')
    output.mkdir();baseline=output/'unchanged-clone.pdf';PdfWriter(clone_from=pdf).write(baseline)
    proof=verify_pdf(fixture,job,baseline,name)
    package,_,events=source_wire(fixture,source_inputs(fixture),name)
    graph=verify_source_structure(baseline,package,body_text=True);matrices=precise_matrices(baseline)
    target=next(event for event in events if event['kind']=='text')
    binding=next(row for row in graph['source_mcr_bindings'] if row['node_id']==target['node_id'])
    page_index=next(index for index,page in enumerate(PdfReader(baseline,strict=True).pages)
                    if [page.indirect_reference.idnum,page.indirect_reference.generation]==binding['page_object'])
    kinds=['removed-glyph','duplicated-glyph']
    if name in ('ascii-before','ascii-after','ascii-double'):kinds+=['added-space','deleted-space']
    marker=re.compile(rb'/(Formula|Span) << /MCID ([0-9]+) /Lang <[0-9A-F]+> >> BDC\n')
    controls=[]
    for kind in kinds:
        writer=PdfWriter(clone_from=baseline);page=writer.pages[page_index];raw=page.get_contents().get_data()
        markers=list(marker.finditer(raw));start=next(row for row in markers if int(row[2])==binding['mcid'])
        require(start[1]==b'Span','private body source scope')
        ordinal=markers.index(start);end=markers[ordinal+1].start() if ordinal+1<len(markers) else len(raw)
        segment=raw[start.start():end]
        if kind in ('removed-glyph','duplicated-glyph'):
            glyph=re.search(rb'<[0-9A-F]{4}> Tj',segment);require(glyph is not None,'private body glyph exists')
            original=glyph[0];replacement=b'' if kind=='removed-glyph' else original+b' '+original
        else:
            text=target['text'];modified=text+' ' if kind=='added-space' else text.replace(' ','',1)
            require(text!=modified,'private authored space really changes')
            original=b'/ActualText <'+(b'\xfe\xff'+text.encode('utf-16-be')).hex().upper().encode()+b'>'
            replacement=b'/ActualText <'+(b'\xfe\xff'+modified.encode('utf-16-be')).hex().upper().encode()+b'>'
        require(segment.count(original)>=1,'exact private body bytes')
        changed=segment.replace(original,replacement,1)
        stream=DecodedStreamObject();stream.set_data(raw[:start.start()]+changed+raw[end:]);page.replace_contents(stream)
        damaged=output/(kind+'.pdf');writer.write(damaged)
        require(verify_source_structure(damaged,package,body_text=True)==graph,'private mixed source graph retained')
        require(precise_matrices(damaged)==matrices,'private mixed Form matrices retained exactly')
        try:verify_pdf(fixture,job,damaged,name)
        except ValueError as error:rejection=str(error)
        else:raise ValueError('private body/space damage was accepted: '+kind)
        row={'kind':kind,'pdf':damaged.name,'pdf_sha256':digest(damaged.read_bytes()),
             'source_graph_preserved':True,'formula_matrices_preserved':True,'source_paint_rejected':True,'error':rejection}
        if kind in ('added-space','deleted-space'):
            extraction={}
            for tool,argv in [('Poppler',[tools['pdftotext'],'-enc','UTF-8','-raw',damaged,'-']),
                              ('MuPDF',[tools['mutool'],'draw','-q','-F','txt',damaged])]:
                _,raw_text,_=checked(executor,'mixed-control-extract',argv,stdout=True)
                (output/(kind+'-'+tool+'.txt')).write_bytes(raw_text)
                try:verify_extraction(raw_text,events,tool)
                except ValueError as error:extraction[tool]={'rejected':True,'error':str(error),'utf8_sha256':digest(raw_text)}
                else:raise ValueError('extractor accepted changed authored space: '+tool)
            row['extraction']=extraction
        if kind=='removed-glyph' and name in ('ascii-before','japanese-adjacent'):
            # Render-only calibration of a deliberately invalid PDF. The original
            # source/layout remain the reference; no new paint success is issued.
            private_proof=copy.deepcopy(proof);private_proof['pdf_sha256']=digest(damaged.read_bytes())
            calibration=output/'removed-glyph-render'
            try:render(fixture,job,damaged,name,private_proof,calibration,tools=tools)
            except ValueError:
                summary=read_canonical(calibration,'summary.json');comparisons=read_canonical(calibration,'comparisons.json')
                require(summary['passed'] is False and summary['failed']>0,'private missing-body raster really fails')
                target_rows=[item for item in comparisons if item['node_id']==target['node_id']]
                require(len(target_rows)==6 and any(not item['passed'] for item in target_rows),'private body source crop rejects missing glyph')
                row['raster_calibration']={'comparisons':summary['comparisons'],'failed':summary['failed'],
                    'target_comparisons':6,'target_failures':sum(not item['passed'] for item in target_rows),
                    'scope':'Deliberately invalid PDF, original positive source/layout retained for render-only calibration.'}
            else:raise ValueError('raster accepted a deliberately missing visible body glyph')
        controls.append(row)
    result={'algorithm':'typaxis.vmb-mixed-private-controls/1','passed':True,'case_id':name,'baseline_clone_passed':True,
            'controls':controls,'damaged_pdfs':len(controls)}
    (output/'observed.json').write_bytes(canonical_bytes(result));return result


def run_case(args,executor,selected,validators,helpers):
    sha,write_json,run_public_builds,ua_check,checked=helpers
    root=args.output_root/'mixed-body-spacing';root.mkdir();results=[]
    for name in CASES:
        case=root/name;case.mkdir()
        result={'algorithm':'typaxis.vmb-book-case-observed/1','case_id':name,'passed':False,'phases':{}}
        try:
            job=case/'job';result['phases']['source']=prepare(args.fixture_root,job,name)
            result['phases']['public_commands']=run_public_builds(args.typaxis,job,case,executor,validators)
            proof=verify_pdf(args.fixture_root,job,case/'output.pdf',name);write_json(case/'source-paint-structure.json',proof)
            result['phases']['source_paint_structure_layout']={'passed':True,'sha256':sha(case/'source-paint-structure.json'),
                'coordinate_error_raw':proof['maximum_observed_coordinate_error_raw'],'body_glyphs':proof['body_glyphs']}
            _,_,events=source_wire(args.fixture_root,source_inputs(args.fixture_root),name);write_json(case/'expected-extraction.json',events)
            extracts={}
            for tool,argv in [('Poppler',[selected['paths']['pdftotext'],'-enc','UTF-8','-raw',case/'output.pdf','-']),
                              ('MuPDF',[selected['paths']['mutool'],'draw','-q','-F','txt',case/'output.pdf'])]:
                _,raw,_=checked(executor,'mixed-extract',argv,stdout=True);(case/(tool+'.txt')).write_bytes(raw)
                extracts[tool]=verify_extraction(raw,events,tool)
            result['phases']['extraction']=extracts
            result['phases']['raw_source_render']=render(args.fixture_root,job,case/'output.pdf',name,proof,case/'render',tools=selected['paths'])
            result['phases']['damage_controls']=damage_controls(args.fixture_root,job,case/'output.pdf',name,case/'controls',
                executor=executor,tools=selected['paths'],checked=checked)
            result['phases']['pdf_ua_automated']=ua_check(case/'output.pdf',case,executor,selected['paths'])
            result['passed']=True
        except Exception as error:result['error']={'type':type(error).__name__,'message':str(error)}
        write_json(case/'observed.json',result);results.append(result)
    result={'case_id':'mixed-body-spacing','passed':all(row['passed'] for row in results),'cases':results,
            'scope':'Six authored mixed source fixtures and independent original glyph/SVG references; original full-book font gate remains.'}
    write_json(root/'observed.json',result);return result
