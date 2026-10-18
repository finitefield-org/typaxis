"""Synthetic Safe-SVG budget boundaries for the VMB book runner.

These small, explicitly synthetic figures isolate declaration/work limits.
They are not real-engine or original-book examples and do not replace the
5,000-distinct placement, mixed-body, layout, or whole-command memory gates.
"""
import copy
import re
from fractions import Fraction
from pathlib import Path
from xml.etree import ElementTree as ET

from vmb_book_inputs import canonical_bytes, digest, read_canonical, require, verify_fixture_inputs
from vmb_probe_io import contained_bytes, same_json, strict_json


PROVENANCE = {'engine_id':'typaxis.synthetic-resource-boundary','engine_version':'1',
              'rules_version':'typaxis.synthetic-limit-fixture/1'}
SOURCE = b'Synthetic resource budget fixture\n'
ALT = 'Synthetic square for resource budget boundaries'
SQUARE = 'M 0 0 L 1 0 L 1 1 L 0 1 Z'
INSET = 'M 0.25 0.25 L 0.75 0.25 L 0.75 0.75 L 0.25 0.75 Z'
REQUIRED_CASES = frozenset({
    'images-default-exact','images-default-over','images-config-exact','images-config-over',
    'nodes-exact','nodes-over','segments-exact','segments-over','depth-exact','depth-over',
    'clip-replay-exact','clip-replay-over','outer-clip-over','nodes-total-exact','nodes-total-over',
    'segments-total-exact','segments-total-over','order-z-first','order-a-first','depth-per-resource',
    'aliases-work-exact','aliases-work-over'})


def svg(kind):
    root = '<svg xmlns="http://www.w3.org/2000/svg" width="1pt" height="1pt" viewBox="0 0 1 1">'
    path = '<path d="'+(INSET if kind == 'inset' else SQUARE)+'"/>'
    if kind in ('simple','inset'):
        body = '<g fill="currentColor">'+path+'</g>'
    elif kind == 'deep':
        body = '<g fill="currentColor"><g>'+path+'</g></g>'
    else:
        require(kind == 'clips', 'known synthetic boundary SVG')
        body = ('<defs><clipPath id="c">'+path+'</clipPath></defs>'
                +('<g fill="currentColor" clip-path="url(#c)">'+path+'</g>')*2)
    return (root+body+'</svg>\n').encode()


def specs():
    result = []
    def add(name, kinds=('simple',), uris=None, count=None, limits=None, failure=None):
        uris = uris or (('z.svg',) if len(kinds) == 1 else ('z.svg','a.svg'))
        result.append({'case_id':name,'assets':[{'uri':uri,'kind':kind} for uri,kind in zip(uris,kinds)],
            'declarations':count if count is not None else len(kinds),'limits':limits or {},'failure':failure})
    for label,limit,config in [('default',8192,{}),('config',1024,{'max_images':1024})]:
        add('images-'+label+'-exact',count=limit,limits=config)
        add('images-'+label+'-over',count=limit+1,limits=config,
            failure={'kind':'images','resource_index':limit,'limit':limit,'observed':limit+1})
    for label,kind,key,limit,charge,element in [
            ('nodes','simple','max_vector_nodes',3,'nodes',3),
            ('segments','simple','max_vector_path_segments',10,'stored_segment',3),
            ('depth','deep','max_vector_nesting_depth',4,'nesting',4),
            ('clip-replay','clips','max_vector_path_segments',30,'clip_replay',7)]:
        add(label+'-exact',(kind,),limits={key:limit})
        add(label+'-over',(kind,),limits={key:limit-1},failure={
            'kind':charge,'resource_index':0,'limit':limit-1,'observed':limit,'used_before_resource':0,
            'element_number':element})
    add('outer-clip-over',limits={'max_vector_path_segments':4},failure={'kind':'stored_segment',
        'resource_index':0,'limit':4,'observed':5,'used_before_resource':0,'element_number':1})
    for label,key,limit,charge,used in [('nodes','max_vector_nodes',6,'nodes',3),
                                     ('segments','max_vector_path_segments',20,'stored_segment',10)]:
        add(label+'-total-exact',('simple','inset'),limits={key:limit})
        add(label+'-total-over',('simple','inset'),limits={key:limit-1},failure={
            'kind':charge,'resource_index':1,'limit':limit-1,'observed':limit,'used_before_resource':used,
            'element_number':3})
    for label,kinds,uris in [('z',('simple','inset'),('z.svg','a.svg')),
                             ('a',('inset','simple'),('a.svg','z.svg'))]:
        add('order-'+label+'-first',kinds,uris,limits={'max_vector_nodes':2},failure={
            'kind':'nodes','resource_index':0,'limit':2,'observed':3,'used_before_resource':0,'element_number':3})
    add('depth-per-resource',('simple','inset'),limits={'max_vector_nesting_depth':3})
    add('aliases-work-exact',('simple','simple'),limits={'max_vector_path_segments':20})
    add('aliases-work-over',('simple','simple'),limits={'max_vector_path_segments':19},failure={
        'kind':'stored_segment','resource_index':1,'limit':19,'observed':20,'used_before_resource':10,'element_number':3})
    require(len(result) == 22 and {row['case_id'] for row in result} == REQUIRED_CASES,
            'complete fixed synthetic boundary matrix')
    return result


def path_ops(element):
    require(element.tag.rsplit('}',1)[-1] == 'path' and set(element.attrib) == {'d'}, 'synthetic boundary path')
    tokens = element.attrib['d'].split()
    result,offset = [],0
    while offset < len(tokens):
        op = tokens[offset]
        require(op in ('M','L','Z'), 'fixed synthetic M/L/Z vocabulary')
        arity = 0 if op == 'Z' else 2
        values = tokens[offset+1:offset+arity+1]
        require(len(values) == arity, 'complete synthetic path operands')
        points = tuple(round(Fraction(value)*65536) for value in values)
        result.append([op,list(points)])
        offset += arity+1
    require([op for op,_ in result] == ['M','L','L','L','Z'], 'complete synthetic rectangle')
    return result


def source_model(raw):
    """Interpret the small authored subset independently of the PDF/trace."""
    root = ET.fromstring(raw)
    require(root.tag.rsplit('}',1)[-1] == 'svg' and root.attrib == {
        'width':'1pt','height':'1pt','viewBox':'0 0 1 1'}, 'fixed synthetic physical viewport')
    nodes = list(root.iter())
    def depth(node):
        return 1+max((depth(child) for child in node),default=0)
    definitions,clips,draws = {},[],[]
    commands,segments = 2,5  # The implicit viewport clip is real charged work.
    id_bytes = 0
    for element in nodes:
        tag = element.tag.rsplit('}',1)[-1]
        require(tag in ('svg','g','defs','clipPath','path'), 'fixed synthetic element vocabulary')
        if tag == 'clipPath':
            require(set(element.attrib) == {'id'} and len(element) == 1, 'one local synthetic clip contour')
            key = element.attrib['id']
            require(key not in definitions, 'unique synthetic clip identity')
            definitions[key] = path_ops(element[0])
            id_bytes += len(key.encode())
        if tag == 'path':
            segments += len(path_ops(element))
    def visit(element, inherited):
        nonlocal commands,id_bytes
        tag = element.tag.rsplit('}',1)[-1]
        if tag == 'defs':
            require(not element.attrib, 'plain synthetic definitions')
            return
        current = list(inherited)
        if tag == 'g':
            require(set(element.attrib) <= {'fill','clip-path'}
                    and element.get('fill','currentColor') == 'currentColor', 'opaque synthetic group')
            if 'clip-path' in element.attrib:
                value = element.attrib['clip-path']
                match = re.fullmatch(r'url\(#([A-Za-z][A-Za-z0-9]*)\)',value)
                require(match is not None and match[1] in definitions, 'resolved local synthetic clip')
                clips.append(match[1]);id_bytes += len(match[1].encode())
                current.append(definitions[match[1]])
                commands += 2
        if tag == 'path':
            draws.append({'path':path_ops(element),'clips':current})
            commands += 1
        else:
            for child in element:
                visit(child,current)
    visit(root,[])
    require(draws and set(clips) == set(definitions), 'nonempty paint and fully used synthetic clips')
    replay = sum(len(definitions[key]) for key in clips)
    return {'nodes':len(nodes),'stored_segments':segments,'clip_replay':replay,'path_work':segments+replay,
        'max_depth':depth(root),'commands':commands,'source_clip_id_bytes':id_bytes,
        'allocation_charge':len(nodes)*64+segments*80+commands*48+id_bytes,'draws':draws}


def config_bytes(spec):
    return ('contract = "typaxis.contract/1.4"\n[limits]\n'+''.join(
        name+' = '+str(value)+'\n' for name,value in sorted(spec['limits'].items()))).encode()


def prepare(fixture, output, spec):
    fixture,output = Path(fixture).resolve(),Path(output).resolve()
    require(not output.is_relative_to(fixture) and not fixture.is_relative_to(output),
            'boundary output outside source fixtures')
    verify_fixture_inputs(fixture)
    require(any(same_json(spec,known) for known in specs()), 'known exact boundary case specification')
    package = read_canonical(fixture,'package-template.json')
    output.mkdir()
    for face in package['resources']['font_faces']:
        path = output/face['uri'];path.parent.mkdir(parents=True,exist_ok=True)
        path.write_bytes(contained_bytes(fixture,face['uri']))
    for asset in spec['assets']:
        (output/asset['uri']).write_bytes(svg(asset['kind']))
    package['metadata']['title'] = 'Synthetic resource boundary: '+spec['case_id']
    images = []
    for image_id in range(spec['declarations']):
        asset = spec['assets'][image_id%len(spec['assets'])]
        images.append({'image_id':image_id,'uri':asset['uri'],'media_type':'svg-safe-2',
            'expected_sha256':digest(svg(asset['kind'])),'vector_provenance':PROVENANCE.copy()})
    package['resources']['images'] = images
    package['sources'] = [{'source_id':0,'uri':'input.tsf','utf8_byte_length':len(SOURCE),'sha256':digest(SOURCE)}]
    package['document']['blocks'] = [{'kind':'vector_figure','node_id':1,
        'span':{'source_id':0,'start_byte':0,'end_byte':len(SOURCE)-1},'classes':[],
        'image_id':spec['declarations']-1,'alt':ALT,'caption':[],'viewport':{'width':65536,'height':65536}}]
    (output/'input.tsf').write_bytes(SOURCE)
    (output/'document-package.json').write_bytes(canonical_bytes(package))
    if spec['limits']:
        (output/'limits.toml').write_bytes(config_bytes(spec))
    return verify_job(fixture,output,spec)


def verify_job(fixture, job, spec):
    require(any(same_json(spec,known) for known in specs()), 'fixed complete boundary expectation')
    verify_fixture_inputs(fixture)
    package = read_canonical(job,'document-package.json')
    template = read_canonical(fixture,'package-template.json')
    static = copy.deepcopy(package)
    require(static['metadata']['title'] == 'Synthetic resource boundary: '+spec['case_id'], 'explicit synthetic source label')
    static['metadata']['title'] = template['metadata']['title']
    static['resources']['images'] = [];static['sources'] = [];static['document']['blocks'] = []
    require(same_json(static,template), 'unchanged boundary template/fonts/page/text settings')
    for face in package['resources']['font_faces']:
        require(digest(contained_bytes(job,face['uri'])) == face['expected_sha256'], 'boundary font bytes')
    require(contained_bytes(job,'input.tsf') == SOURCE and same_json(package['sources'],[
        {'source_id':0,'uri':'input.tsf','utf8_byte_length':len(SOURCE),'sha256':digest(SOURCE)}]), 'exact boundary source file')
    require(same_json(package['document']['blocks'],[{'kind':'vector_figure','node_id':1,
        'span':{'source_id':0,'start_byte':0,'end_byte':len(SOURCE)-1},'classes':[],
        'image_id':spec['declarations']-1,'alt':ALT,'caption':[],'viewport':{'width':65536,'height':65536}}]),
        'one source-bound synthetic vector figure')
    images = package['resources']['images']
    require(len(images) == spec['declarations'], 'complete declaration boundary inventory')
    models = {}
    for asset in spec['assets']:
        raw = contained_bytes(job,asset['uri'])
        require(raw == svg(asset['kind']), 'exact authored synthetic SVG bytes')
        models[asset['uri']] = source_model(raw)
    for image_id,image in enumerate(images):
        asset = spec['assets'][image_id%len(spec['assets'])]
        require(same_json(image,{'image_id':image_id,'uri':asset['uri'],'media_type':'svg-safe-2',
            'expected_sha256':digest(svg(asset['kind'])),'vector_provenance':PROVENANCE}), 'typed complete synthetic declarations')
    require(not spec['limits'] or contained_bytes(job,'limits.toml') == config_bytes(spec), 'exact boundary configuration bytes')
    expected_files = {'document-package.json','input.tsf'} | {face['uri'] for face in package['resources']['font_faces']} | set(models)
    if spec['limits']:
        expected_files.add('limits.toml')
    require({str(path.relative_to(job)) for path in job.rglob('*') if path.is_file() or path.is_symlink()} == expected_files,
            'complete generated boundary input files')
    totals = {key:sum(models[image['uri']][key] for image in images)
              for key in ('nodes','stored_segments','clip_replay','path_work','allocation_charge')}
    totals['max_depth'] = max(model['max_depth'] for model in models.values())
    return {'case_id':spec['case_id'],'declarations':len(images),'source_occurrences':1,
        'distinct_svg_bytes':len({image['expected_sha256'] for image in images}),
        'declared_work':totals,'source_models':models,'package_sha256':digest(contained_bytes(job,'document-package.json')),
        'scope':'Synthetic declaration and charged-work boundaries; not real-engine scale or whole-command physical accounting.'}


def expected_diagnostics(spec):
    """Derive the exact source location from the authored XML, never a trace."""
    failure = spec['failure']
    require(failure is not None, 'negative boundary specification')
    kind,index = failure['kind'],failure['resource_index']
    location = {'kind':'package_json','uri':'document-package.json',
                'json_pointer':'/resources/images/'+str(index),'byte_offset':None}
    if kind == 'images':
        code,message,notes = 'P1102',f"DocumentPackage Images budget {failure['limit']} was exceeded by {failure['observed']}",[]
    else:
        asset = spec['assets'][index%len(spec['assets'])]
        raw = svg(asset['kind'])
        elements = list(re.finditer(rb'<(svg|g|defs|clipPath|path)\b[^>]*>',raw))
        element = elements[failure['element_number']-1]
        tag = element[1].decode('ascii')
        start,end = element.span()
        parts = ['element='+tag,'element_number='+str(failure['element_number'])]
        if tag == 'path':
            parts.append('path='+str(sum(match[1] == b'path' for match in elements[:failure['element_number']])))
        if kind == 'stored_segment' and tag == 'path':
            token = re.search(rb'\bZ\b',element[0])
            require(token is not None, 'authored closing segment token')
            start,end = element.start()+token.start(),element.start()+token.end()
            parts += ['subpath=1','segment=5','attribute=d','token_percent=%5A','token_truncated=false']
        elif kind == 'stored_segment':
            require(tag == 'svg', 'implicit viewport allocation source')
            parts.append('attribute=viewBox')
        elif kind == 'clip_replay':
            token = re.search(rb'clip-path="([^"]+)"',element[0])
            require(token is not None, 'authored local clip reference token')
            start,end = element.start()+token.start(1),element.start()+token.end(1)
            parts += ['attribute=clip-path','token_percent='+''.join('%'+format(value,'02X') for value in token[1]),
                      'token_truncated=false']
        parts += [f'svg_byte={start}..{end}','line='+str(raw[:start].count(b'\n')+1),
                  'byte_column='+str(start-raw.rfind(b'\n',0,start))]
        scope = 'resource-local' if kind == 'nesting' else 'document-total'
        code = {'nodes':'R7120','stored_segment':'R7121','clip_replay':'R7121','nesting':'R7122'}[kind]
        message = 'svg_safe_2 budget_exceeded'
        notes = [{'location':None,'message':value} for value in ['resource='+asset['uri'],'; '.join(parts),
            f"scope={scope}; charge={kind}; limit={failure['limit']}; observed={failure['observed']}; used_before_resource={failure['used_before_resource']}"]]
    return {'contract':'typaxis.contract/1.4','diagnostics':[
        {'code':code,'severity':'error','message':message,'location':location,'notes':notes}]}


def validate_failure(spec, record, stdout, stderr, diagnostics):
    expected = expected_diagnostics(spec)
    diagnostic = expected['diagnostics'][0]
    require(type(record['exit_code']) is int and record['exit_code'] == (1 if spec['failure']['kind'] == 'images' else 5)
            and record['timed_out'] is False and record['state'] == 'complete', 'correct terminal boundary failure status')
    require(not stdout and stderr == ('error: '+diagnostic['code']+': '+diagnostic['message']+'\n').encode(),
            'exact boundary failure command output')
    require(same_json(diagnostics,expected), 'exact boundary failure diagnostic and authored source location')


def run_negative(typaxis, job, case, spec, executor, validators):
    import verify_machine_profile as machine
    from verify_vmb_book import PROFILE,files
    before = files(job)
    common = ['--package-root','.', '--profile',PROFILE]
    if spec['limits']:
        common += ['--config',str(job/'limits.toml')]
    results,raw_diagnostics = [],[]
    for operation in ('check','build'):
        directory = case/operation;directory.mkdir()
        argv = [typaxis,operation+'-package','document-package.json',*common,
                '--emit-diagnostics',directory/'diagnostics.json']
        if operation == 'build':
            argv += ['-o',directory/'output.pdf','--trace',directory/'trace.json','--trace-text',
                     '--emit-build-manifest',directory/'manifest.json','--no-compress']
        record,out,err = executor.run(operation+'-package',argv,cwd=job)
        raw = (directory/'diagnostics.json').read_bytes()
        diagnostics = strict_json(raw)
        machine._validate_instance(validators,'diagnostics.schema.json',diagnostics,'boundary failure')
        validate_failure(spec,record,out,err,diagnostics)
        raw_diagnostics.append(raw)
        require(not (directory/'output.pdf').exists() and not (directory/'trace.json').exists(),
                'no output PDF or success trace after boundary failure')
        result = {'operation':operation,'exit_code':record['exit_code'],'diagnostics_sha256':digest(raw),
                  'no_pdf':True,'no_trace':True}
        if operation == 'build':
            manifest = strict_json((directory/'manifest.json').read_bytes())
            machine._validate_instance(validators,'build-manifest.schema.json',manifest,'boundary failed manifest')
            require(manifest['status'] == 'failed' and manifest['output'] is None and manifest['layout'] is None
                    and manifest['input_profile'] == PROFILE and manifest['contract'] == 'typaxis.contract/1.4',
                    'truthful failed build manifest')
            for key in ('safe_vector_manifest','math_vector_manifest','tagged_pdf_manifest','book_navigation_manifest'):
                require(manifest[key] is None and manifest[key+'_fingerprint'] is None, 'no successful child manifest on failure')
            require(manifest['package_input']['sha256'] == before['document-package.json']['sha256']
                    and manifest['package_input']['bytes'] == before['document-package.json']['byte_length'],
                    'failure binds exact source package')
            result['effective_config_sha256'] = manifest['config_sha256']
        results.append(result)
    require(raw_diagnostics[0] == raw_diagnostics[1], 'check and build identical failure diagnostics')
    require(files(job) == before, 'failed public commands preserve every source input')
    return {'commands':results,'check_build_diagnostics_identical':True,'inputs_preserved':True,
            'diagnostic':expected_diagnostics(spec)}


def verify_manifest(job, manifest, facts):
    package = read_canonical(job,'document-package.json')
    images = package['resources']['images']
    require(len(manifest['images']) == len(images) and manifest['math_vector_manifest']['facts'] == [],
            'all declarations admitted and no invented formula facts')
    for declaration,image in zip(images,manifest['images']):
        model = facts['source_models'][declaration['uri']]
        require(image['image_id'] == declaration['image_id'] and image['uri'] == declaration['uri']
                and image['sha256'] == declaration['expected_sha256']
                and image['bytes'] == len(contained_bytes(job,declaration['uri']))
                and type(image['decoded_bytes']) is int and image['decoded_bytes'] == model['allocation_charge'],
                'independent source charge for each admitted declaration')
    safe = manifest['safe_vector_manifest']
    require(len(safe['resources']) == facts['distinct_svg_bytes'] and safe['placement_count'] == 1,
            'distinct byte resources and single authored occurrence')
    by_id = {}
    for resource in safe['resources']:
        aliases = resource['aliases']
        require(aliases, 'nonempty boundary alias group')
        model = facts['source_models'][aliases[0]['uri']]
        require(resource['allocation_charge_id'] == 'typaxis.safe-vector-allocation-charge/2'
                and resource['allocation_charge'] == model['allocation_charge'], 'independently calculated resource charge')
        used = any(alias['image_id'] == len(images)-1 for alias in aliases)
        require(resource['total_placement_count'] == int(used) and len(resource['placements']) == int(used)
                and (type(resource['pdf_form_object_number']) is int if used else resource['pdf_form_object_number'] is None),
                'unused declarations do not invent PDF Forms or occurrences')
        for alias in aliases:
            owner = alias['image_id']
            require(type(owner) is int and owner not in by_id and 0 <= owner < len(images), 'typed complete boundary alias IDs')
            require(alias['admission_allocation_charge'] == facts['source_models'][images[owner]['uri']]['allocation_charge']
                    and alias['placement_count'] == int(owner == len(images)-1), 'per-declaration alias charge and placement count')
            by_id[owner] = alias
    require(set(by_id) == set(range(len(images))) and sum(image['decoded_bytes'] for image in manifest['images'])
            == facts['declared_work']['allocation_charge'], 'complete declared charge total')
    return {'declarations':len(images),'distinct_resources':len(safe['resources']),'placed_resources':1,
            'allocation_charge_id':'typaxis.safe-vector-allocation-charge/2',
            'declared_allocation_charge':facts['declared_work']['allocation_charge'],
            'scope':'Pinned Safe-SVG admission charge, not physical heap bytes or a whole-command memory bound.'}


def form_events(form, reader):
    """Read actual paint and clip events for the fixed synthetic M/L/Z subset."""
    from pypdf.generic import ContentStream
    from verify_vmb_scale_pdf import fixed
    require(form['/Subtype'] == '/Form' and tuple(map(fixed,form['/BBox'])) == (0,0,65536,65536)
            and '/Group' not in form and list(form.get('/Matrix',[1,0,0,1,0,0])) == [1,0,0,1,0,0],
            'plain unit synthetic Form')
    clips,stack,path,events = [],[],[],[]
    pending = False
    viewport = [['M',[0,0]],['L',[65536,0]],['L',[65536,65536]],['L',[0,65536]],['Z',[]]]
    for args,op in ContentStream(form,reader).operations:
        if op == b'q':
            require(not args and not path and not pending, 'boundary Form save')
            stack.append(copy.deepcopy(clips))
        elif op == b'Q':
            require(not args and stack and not path and not pending, 'boundary Form restore')
            clips = stack.pop()
        elif op == b'cm':
            require(tuple(map(fixed,args)) == (65536,0,0,65536,0,0), 'boundary identity transform')
        elif op == b'gs':
            require(len(args) == 1, 'boundary Form state reference')
            state = form['/Resources']['/ExtGState'][args[0]].get_object()
            require(set(state) <= {'/Type','/ca','/CA','/BM','/SMask'}
                    and state.get('/ca',1) == state.get('/CA',1) == 1
                    and state.get('/BM','/Normal') == '/Normal' and state.get('/SMask','/None') == '/None',
                    'opaque normal boundary Form paint')
        elif op in (b'm',b'l'):
            require(len(args) == 2 and not pending and (bool(path) if op == b'l' else not path), 'boundary path operands/order')
            path.append(['M' if op == b'm' else 'L',list(map(fixed,args))])
        elif op == b'h':
            require(not args and path and not pending, 'boundary close contour')
            path.append(['Z',[]])
        elif op == b're':
            require(not path and not pending and len(args) == 4, 'boundary rectangular clip')
            x,y,w,h = map(fixed,args)
            path = [['M',[x,y]],['L',[x+w,y]],['L',[x+w,y+h]],['L',[x,y+h]],['Z',[]]]
        elif op == b'W':
            require(not args and path and not pending, 'boundary nonzero clip rule')
            pending = True
        elif op == b'n':
            require(not args and path and pending, 'boundary clip commitment')
            clips.append(path);path = [];pending = False
        elif op == b'f':
            require(not args and path and not pending and clips and clips[0] == viewport,
                    'boundary opaque fill within authored viewport')
            events.append({'path':path,'clips':copy.deepcopy(clips[1:])});path = []
        else:
            raise ValueError('unsupported boundary Form operation '+repr(op))
    require(not stack and not path and not pending and not clips, 'balanced boundary Form state')
    return events


def verify_pdf(job, pdf, spec, facts):
    from decimal import Decimal
    from pypdf import PdfReader
    from pypdf.generic import ContentStream
    from verify_vmb_scale_pdf import PagePaint,fixed
    from verify_vmb_scale_render import precise_matrices
    from vmb_book_pdf import integer,verify_source_structure
    package = read_canonical(job,'document-package.json')
    graph = verify_source_structure(pdf,package)
    reader = PdfReader(pdf,strict=True)
    require(len(reader.pages) == 1, 'one-page synthetic boundary PDF')
    page,master = reader.pages[0],package['page_masters']['masters'][0]
    require(tuple(map(fixed,page.mediabox)) == (0,0,master['width'],master['height'])
            and not page.get('/Annots') and page.get('/Rotate',0) == 0 and page.get('/UserUnit',1) == 1
            and tuple(page.get('/CropBox',page.mediabox)) == tuple(page.mediabox), 'source boundary page geometry')
    resources = page['/Resources']
    paint = PagePaint(resources)
    active,paints,mcids = None,[],[]
    for args,op in ContentStream(page.get_contents(),reader).operations:
        event = paint.step(args,op)
        require(event is None or event[0] == 'paint', 'no synthetic formula text anchors')
        if op == b'BDC':
            require(active is None and args[0] == '/Figure' and set(args[1]) == {'/MCID','/Lang'}
                    and str(args[1]['/Lang']) == package['document']['language'], 'source Figure marked content')
            active = integer(args[1]['/MCID']);mcids.append(active)
        elif op == b'EMC':
            require(active is not None, 'matched boundary marked content')
            active = None
        elif op == b'BMC':
            raise ValueError('unexpected synthetic boundary artifact')
        if event:
            require(active is not None, 'source-owned boundary paint')
            form = resources['/XObject'][args[0]].get_object()
            model = facts['source_models'][package['resources']['images'][-1]['uri']]
            require(form_events(form,reader) == model['draws'], 'exact source boundary paint paths and clip chains')
            paints.append((event,form))
    paint.finish()
    require(active is None and len(paints) == 1 and mcids == [0] and len(resources['/XObject']) == 1,
            'one source occurrence and no additional boundary paint')
    binding = graph['source_mcr_bindings']
    require(len(binding) == 1 and binding[0]['node_id'] == 1 and binding[0]['mcid'] == 0
            and binding[0]['page_object'] == [page.indirect_reference.idnum,page.indirect_reference.generation],
            'exact source graph, MCR and paint join')
    event,form = paints[0]
    _,matrix,clip = event
    require(matrix[:4] == tuple(map(Decimal,(1,0,0,-1))), 'unit boundary placement')
    x,y = matrix[4:]
    rect = (fixed(x),fixed(y-1),fixed(x+1),fixed(y))
    body = master['body']
    bounds = (body['x'],master['height']-body['y']-body['height'],body['x']+body['width'],master['height']-body['y'])
    for limit in [bounds]+([tuple(map(fixed,clip))] if clip is not None else []):
        require(rect[0] >= limit[0]-1 and rect[1] >= limit[1]-1 and rect[2] <= limit[2]+1 and rect[3] <= limit[3]+1,
                'boundary viewport fully visible in source body and page clips')
    exact = precise_matrices(pdf)
    require(len(exact) == 1 and len(exact[0]) == 1 and all(abs(fixed(a)-fixed(b)) <= 1 for a,b in zip(exact[0][0],matrix)),
            'precise boundary paint matrix join')
    row = {'ordinal':0,'node_id':1,'image_id':spec['declarations']-1,'kind':'vector_figure','page_index':0,
           'bounds_raw':list(rect),'world_matrix':[str(value) for value in exact[0][0]]}
    return {'algorithm':'typaxis.vmb-boundary-source-pdf/1','pdf_sha256':digest(pdf.read_bytes()),
            'source_package_sha256':facts['package_sha256'],'structure':graph,'placements':[row],
            'form_object':form.indirect_reference.idnum,'paint_events':len(model['draws'])}


def render(job, pdf, proof, output, *, tools):
    import tempfile
    from decimal import Decimal
    from PIL import Image
    from verify_vmb_scale_render import DPIS,POLICY,command,compare_masks,crop_box,ink,precise_matrices
    from vmb_svg_reference import source_form,reference_page
    require(not output.resolve().is_relative_to(job.resolve()), 'boundary render output outside source inputs')
    output.mkdir()
    package = read_canonical(job,'document-package.json')
    require(proof['pdf_sha256'] == digest(pdf.read_bytes())
            and proof['source_package_sha256'] == digest(contained_bytes(job,'document-package.json')),
            'boundary render binds checked PDF and authored package')
    master = package['page_masters']['masters'][0]
    width,height = Decimal(master['width'])/65536,Decimal(master['height'])/65536
    row = proof['placements'][0]
    image = package['resources']['images'][row['image_id']]
    logs,results = [],[]
    with tempfile.TemporaryDirectory(prefix='render-',dir=output) as directory:
        scratch = Path(directory)
        form = source_form(ET.fromstring(contained_bytes(job,image['uri'])),Decimal(1),Decimal(1),scratch,
            'source',tools['rsvg-convert'],lambda argv:command(argv,logs),source_to_reference_scale=Decimal(4096))
        reference = scratch/'reference.pdf'
        reference_page(reference,width,height,[row],precise_matrices(pdf)[0],lambda owner:form,lambda owner:None)
        for dpi in DPIS:
            for renderer in ('MuPDF','Poppler'):
                paths = []
                for name,source in [('reference',reference),('observed',pdf)]:
                    path = scratch/(name+'.png');paths.append(path)
                    argv = ([tools['mutool'],'draw','-q','-r',str(dpi),'-c','rgb','-o',str(path),str(source),'1']
                        if renderer == 'MuPDF' else [tools['pdftoppm'],'-f','1','-l','1','-r',str(dpi),
                            '-singlefile','-png',str(source),str(path.with_suffix(''))])
                    command(argv,logs)
                with Image.open(paths[0]) as image:
                    reference_mask = ink(image)
                with Image.open(paths[1]) as image:
                    observed = ink(image)
                require(reference_mask.size == observed.size, 'boundary unresized renderer page dimensions')
                inner = crop_box(row['bounds_raw'],height,dpi)
                box = (inner[0]-2,inner[1]-2,inner[2]+2,inner[3]+2)
                require(box[0] >= 0 and box[1] >= 0 and box[2] <= reference_mask.width and box[3] <= reference_mask.height,
                        'complete boundary source viewport and two-pixel outer ring')
                reference_crop,observed_crop = reference_mask.crop(box),observed.crop(box)
                counts = compare_masks(reference_crop,observed_crop,(2,2,box[2]-box[0]-2,box[3]-box[1]-2))
                results.append({'ordinal':0,'page_index':0,'dpi':dpi,'renderer':renderer,'crop_pixels':list(box),
                    'reference_mask_sha256':digest(reference_crop.tobytes()),'observed_mask_sha256':digest(observed_crop.tobytes()),**counts})
    for name,value in [('commands.json',logs),('comparisons.json',results)]:
        (output/name).write_bytes(canonical_bytes(value))
    summary = {'algorithm':POLICY['algorithm'],'policy':POLICY,'comparisons':len(results),
        'failed':sum(not row['passed'] for row in results),
        'mask_exact':sum(row['reference_mask_sha256'] == row['observed_mask_sha256'] for row in results),
        'scope':'Original synthetic SVG at observed placements; no independent layout or real-engine claim.'}
    (output/'summary.json').write_bytes(canonical_bytes(summary))
    require(summary['comparisons'] == 6 and summary['failed'] == 0, 'all fixed-policy boundary masks pass')
    return summary


def run_case(args, executor, selected, validators, helpers):
    sha,write_json,run_public_builds,ua_check = helpers
    root = args.output_root/'resource-boundaries';root.mkdir()
    observation = {'algorithm':'typaxis.vmb-book-case-observed/1','case_id':'resource-boundaries','passed':False,'cases':[]}
    for spec in specs():
        case = root/spec['case_id'];case.mkdir()
        result = {'case_id':spec['case_id'],'passed':False,'expected_success':spec['failure'] is None,'phases':{}}
        try:
            job = case/'job'
            facts = prepare(args.fixture_root,job,spec)
            result['phases']['source'] = facts
            if spec['failure'] is not None:
                result['phases']['public_failure'] = run_negative(args.typaxis,job,case,spec,executor,validators)
            else:
                result['phases']['public_commands'] = run_public_builds(args.typaxis,job,case,executor,validators,
                    job/'limits.toml' if spec['limits'] else None)
                result['phases']['declaration_charge'] = verify_manifest(job,strict_json((case/'manifest.json').read_bytes()),facts)
                proof = verify_pdf(job,case/'output.pdf',spec,facts)
                write_json(case/'source-paint-structure.json',proof)
                result['phases']['source_paint_structure'] = {'passed':True,'sha256':sha(case/'source-paint-structure.json')}
                result['phases']['raw_svg_render'] = render(job,case/'output.pdf',proof,case/'render',tools=selected['paths'])
                result['phases']['pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,selected['paths'])
            result['passed'] = True
        except Exception as error:
            result['error'] = {'type':type(error).__name__,'message':str(error)}
        write_json(case/'observed.json',result);observation['cases'].append(result)
    observation['passed'] = (len(observation['cases']) == len(REQUIRED_CASES)
        and {case['case_id'] for case in observation['cases']} == REQUIRED_CASES
        and all(case['passed'] for case in observation['cases']))
    observation['scope'] = 'Synthetic resource admission boundaries; real-engine scale and physical accounting remain separate gates.'
    write_json(root/'observed.json',observation)
    return observation
