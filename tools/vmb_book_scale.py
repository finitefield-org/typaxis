"""Run or revalidate the fixed real-engine scale cases for the book runner.

Cache use is explicit, pinned by the separate book tool policy, and reported as
reused evidence. Source/paint/structure/extraction checks still run on the exact
PDF bytes. This module does not establish independent layout or performance.
"""
import shutil
from pathlib import Path

from vmb_book_inputs import require
from vmb_probe_io import contained_bytes, extracted_occurrences, same_json, strict_json


CASES = {'distinct':'distinct-5000','mixed':'mixed-5000','alias':'alias-8000'}


def formula_speech(package):
    from verify_vmb_scale_pdf import occurrences
    # The formula anchor supplies semantic text. Figure alternatives belong to
    # the source structure join; a PNG has no formula text anchor.
    return [node.get('actual_text') or node['alt'] for node in occurrences(package['document'])
            if node['kind'] in ('math_vector','math_vector_block')]


def indexed_files(root, pinned_sha, algorithm, sha, check_file):
    raw = contained_bytes(root,'artifact-index.json')
    require(sha(root/'artifact-index.json') == pinned_sha, 'pinned evidence artifact index')
    index = strict_json(raw)
    require(index['algorithm'] == algorithm and isinstance(index['files'],list) and bool(index['files']),
            'complete evidence artifact index')
    names = set()
    for record in index['files']:
        check_file(root,record)
        require(record['path'] not in names, 'unique evidence artifact paths')
        names.add(record['path'])
    return index,names


def copy_indexed_tree(root, prefix, output, names):
    selected = sorted(name for name in names if name.startswith(prefix+'/'))
    require(selected, 'nonempty indexed source tree')
    output.mkdir()
    for name in selected:
        relative = name[len(prefix)+1:]
        target = output/relative
        target.parent.mkdir(parents=True,exist_ok=True)
        # Read through the contained-path guard even after index validation.
        target.write_bytes(contained_bytes(root,name))


def prepare_sources(args, selected, executor, helpers):
    sha,check_file,checked,write_json = helpers
    policy = selected['policy']
    corpus = args.output_root/'scale-engine-corpus'
    cached = args.scale_build_evidence
    if cached is not None:
        pins = policy['scale_cache']
        require(sha(args.typaxis) == pins['typaxis_sha256'], 'cached builds require the identical CLI executable')
        _,names = indexed_files(cached,pins['artifact_index_sha256'],
            'typaxis.vmb-scale-evidence-files/1',sha,check_file)
        copy_indexed_tree(cached,'engine-corpus',corpus,names)
        write_json(args.output_root/'scale-build-evidence.json',{
            'origin':'reused-public-command-evidence','source_root':str(cached),
            'artifact_index_sha256':pins['artifact_index_sha256'],'verified_indexed_files':len(names),
            'fresh_public_scale_commands':False})
        return corpus,names
    generator = policy['scale_generator']
    path = Path(generator['path'])
    if not path.is_absolute():
        path = args.repository_root/path
    require(sha(path) == generator['sha256'], 'pinned actual-engine generator executable')
    checked(executor,'generate-scale-corpus',[path,'-corpus','distinct-5000','-output',corpus],stdout=True)
    require(sha(corpus/'fixture-index.json') == generator['engine_corpus_sha256'], 'fixed engine corpus identity')
    return corpus,None


def reused_builds(args, kind, case, names, validators, helpers):
    sha,check_file,checked,write_json,artifact_checks = helpers
    source = args.scale_build_evidence
    job = case/'job'
    copy_indexed_tree(source,kind+'-job',job,names)
    evidence = case/'reused-public-commands'
    evidence.mkdir()
    command_facts = []
    for phase in ('check','build1','build2'):
        name = f'public-{kind}-{phase}.command.json'
        require(name in names, 'indexed successful public command')
        raw = contained_bytes(source,name)
        record = strict_json(raw)
        argv = record['argv']
        operation = 'check-package' if phase == 'check' else 'build-package'
        require(record['state'] == 'complete' and type(record['exit_code']) is int and record['exit_code'] == 0
                and argv[:4] == ['/usr/bin/time','-v',str(args.typaxis),operation]
                and '--profile' in argv and argv[argv.index('--profile')+1] == 'typaxis.machine-pdf/production-book-1'
                and not any(arg.startswith('--limits') or arg in ('--config','--config-json') for arg in argv)
                and not record.get('extra_env')
                and not any(key.startswith('TYPAXIS_') and value is not None for key,value in record['env'].items()),
                'unchanged public CLI/profile/default-config cached commands')
        log_name = Path(record['log']).name
        require(log_name in names and sha(source/log_name) == record['log_sha256'], 'indexed public command log')
        (evidence/name).write_bytes(raw)
        (evidence/log_name).write_bytes(contained_bytes(source,log_name))
        command_facts.append({'phase':phase,'command_sha256':sha(evidence/name),'exit_code':0,
            'verifier_source_drift_recorded':record['source_drift_during_command']})
    check_name = f'{kind}-output-v2/check/diagnostics.json'
    require(check_name in names, 'indexed public check diagnostics')
    (case/'check-diagnostics.json').write_bytes(contained_bytes(source,check_name))
    import verify_machine_profile as machine
    diagnostics = strict_json((case/'check-diagnostics.json').read_bytes())
    machine._validate_instance(validators,'diagnostics.schema.json',diagnostics,'cached public check diagnostics')
    require(diagnostics['diagnostics'] == [], 'positive cached check diagnostics')
    package = strict_json(contained_bytes(job,'document-package.json'))
    package_record = {'byte_length':(job/'document-package.json').stat().st_size,'sha256':sha(job/'document-package.json')}
    for ordinal in (1,2):
        output = case/('build'+str(ordinal))
        output.mkdir()
        for target,old in [('output.pdf','output.pdf'),('manifest.json','manifest.json'),
                           ('trace.json','trace.json'),('build-diagnostics.json','diagnostics.json')]:
            name = f'{kind}-output-v2/build{ordinal}/{old}'
            require(name in names, 'indexed public build artifact')
            (output/target).write_bytes(contained_bytes(source,name))
        manifest,_ = artifact_checks(output,package,validators,package_record)
        require(manifest['config_sha256'] == args.scale_config_sha256, 'cached effective config identity')
    for name in ('output.pdf','manifest.json','trace.json','build-diagnostics.json'):
        require((case/'build1'/name).read_bytes() == (case/'build2'/name).read_bytes(),
                'cached deterministic public artifact '+name)
        shutil.copyfile(case/'build1'/name,case/name)
    return {'origin':'reused-public-command-evidence','fresh_public_commands':False,
        'package_sha256':package_record['sha256'],'pdf_sha256':sha(case/'output.pdf'),
        'artifact_byte_identity':True,'commands':command_facts}


def render_cache(args, selected, sha, check_file):
    source = args.scale_render_evidence
    if source is None:
        return None
    pins = selected['policy']['render_cache']
    index,names = indexed_files(source,pins['artifact_index_sha256'],
        'typaxis.vmb-book-render-cache/1',sha,check_file)
    from verify_vmb_scale_render import POLICY
    require(same_json(index['render_policy'],POLICY), 'cached fixed raster policy')
    ledger = 'producer/numeric-producer-source.json'
    require(ledger in names and sha(source/ledger) == pins['producer_source_ledger_sha256']
            == index['producer_source_ledger_sha256'], 'cached renderer source ledger')
    dependencies = strict_json(contained_bytes(source,ledger))
    require(same_json(dependencies,index['producer_dependencies']) and set(dependencies) == {
        'tools/vmb_probe_io.py','tools/prepare_vmb_distinct_probe.py','tools/verify_vmb_scale_pdf.py',
        'tools/verify_vmb_scale_render.py','tools/vmb_svg_reference.py'}, 'complete producer dependency set')
    for name,digest in dependencies.items():
        require(sha(args.repository_root/name) == digest and 'producer/source/'+name in names
                and sha(source/'producer/source'/name) == digest, 'cached renderer current dependency identity')
    return names


def reused_render(args, kind, case, selected, placements, corpus, names, helpers):
    sha,write_json = helpers
    from verify_vmb_scale_render import POLICY, mask_decision
    source = args.scale_render_evidence
    output = case/'render'
    output.mkdir()
    for name in ('summary.json','comparisons.json'):
        member = kind+'/'+name
        require(member in names, 'indexed complete scale raster evidence')
        (output/name).write_bytes(contained_bytes(source,member))
    summary = strict_json((output/'summary.json').read_bytes())
    require(summary['algorithm'] == POLICY['algorithm'] and same_json(summary['policy'],POLICY)
            and summary['case'] == kind and summary['passed'] is True and summary['failed_comparisons'] == 0
            and summary['source_package_sha256'] == placements['source_package_sha256']
            and summary['pdf_sha256'] == placements['pdf_sha256']
            and summary['corpus_sha256'] == sha(corpus/'fixture-index.json')
            and summary['all_case_pages'] is True and summary['selected_pages'] == list(range(len(placements['pages']))),
            'cached raster binds exact package/PDF/corpus/policy and every page')
    for name,record in summary['tools'].items():
        require(name in ('mutool','pdftoppm','rsvg-convert') and record['sha256'] == sha(selected['paths'][name]),
                'cached raster actual tool identity')
    require(set(summary['tools']) == {'mutool','pdftoppm','rsvg-convert'}, 'complete cached raster tools')
    for command in summary['commands']:
        require(type(command['exit_code']) is int and command['exit_code'] == 0
                and command['stdout_bytes'] == command['stderr_bytes'] == 0, 'cached warning-free render commands')
    required = {(row['ordinal'],row['page_index'],renderer,dpi) for row in placements['placements']
                if row['kind'] != 'figure' for renderer in ('MuPDF','Poppler') for dpi in POLICY['render_dpis']}
    rows = strict_json((output/'comparisons.json').read_bytes())
    observed = set()
    for row in rows:
        key = (row['ordinal'],row['page_index'],row['renderer'],row['dpi'])
        require(type(row['ordinal']) is int and type(row['page_index']) is int and type(row['dpi']) is int
                and key in required and key not in observed, 'unique complete cached raster comparison')
        require(all(type(row[name]) is int and row[name] >= 0 for name in ('reference_ink','observed_ink',
            'area_difference','missing_over_one_pixel','extra_over_one_pixel'))
            and row['area_difference'] == abs(row['reference_ink']-row['observed_ink'])
            and row['passed'] is True and mask_decision(row), 'cached raster decisions satisfy fixed tolerances')
        observed.add(key)
    require(observed == required and summary['comparisons'] == len(rows), 'all cached source formula masks covered')
    command_name = 'producer/numeric-'+kind+'.command.json'
    require(command_name in names, 'cached successful renderer invocation')
    command = strict_json(contained_bytes(source,command_name))
    require(command['state'] == 'complete' and command['exit_code'] == 0 and command['source_drift'] == []
            and command['dependency_source_sha256'] == selected['policy']['render_cache']['producer_source_ledger_sha256'],
            'cached renderer invocation source and success')
    for channel in ('stdout','stderr'):
        name = 'producer/numeric-'+kind+'.'+channel
        require(name in names and sha(source/name) == command[channel+'_sha256'], 'cached render command output hash')
    write_json(output/'reuse.json',{'origin':'reused-raw-svg-render-evidence',
        'artifact_index_sha256':selected['policy']['render_cache']['artifact_index_sha256'],
        'producer_source_sha256':command['dependency_source_sha256'],'fresh_render_commands':False})
    return {'origin':'reused-raw-svg-render-evidence','passed':True,'comparisons':len(rows),
        'summary_sha256':sha(output/'summary.json'),'comparisons_sha256':sha(output/'comparisons.json')}


def run_cases(args, executor, selected, validators, helpers):
    sha,check_file,checked,write_json,artifact_checks,run_public_builds,ua_check = helpers
    from prepare_vmb_distinct_probe import prepare,verify as verify_corpus
    from verify_vmb_scale_pdf import verify as verify_pdf
    from verify_vmb_scale_render import render_pages
    from vmb_book_pdf import verify_source_structure
    corpus,build_names = prepare_sources(args,selected,executor,(sha,check_file,checked,write_json))
    index,source = verify_corpus(corpus)
    require(source['index_sha256'] == selected['policy']['scale_generator']['engine_corpus_sha256'],
            'source-verified fixed scale corpus')
    render_names = render_cache(args,selected,sha,check_file)
    results = {}
    for kind,case_id in CASES.items():
        case = args.output_root/case_id
        case.mkdir()
        observation = {'algorithm':'typaxis.vmb-book-case-observed/1','case_id':case_id,'phases':{},'passed':False}
        try:
            job = case/'job'
            if build_names is None:
                prepare(corpus,job,kind,index,source.copy())
                public = run_public_builds(args.typaxis,job,case,executor,validators)
            else:
                public = reused_builds(args,kind,case,build_names,validators,
                    (sha,check_file,checked,write_json,artifact_checks))
            observation['phases']['public_commands'] = public
            placement_path = case/'placements.json'
            paint = verify_pdf(job,case/'output.pdf',placement_path,include_structure_bindings=True,
                extraction_tools={name:selected['paths'][name] for name in ('mutool','pdftotext')})
            package = strict_json(contained_bytes(job,'document-package.json'))
            graph = verify_source_structure(case/'output.pdf',package)
            require(paint['source_mcr_bindings'] == graph['source_mcr_bindings'], 'scale source hierarchy and paint MCR join')
            write_json(case/'source-paint-structure.json',{'paint':paint,'structure':graph})
            observation['phases']['source_paint_structure'] = {'passed':True,
                'sha256':sha(case/'source-paint-structure.json'),'declarations':paint['declarations'],
                'placements':paint['placements'],'forms':paint['forms'],'pages':paint['pages']}
            # Retain raw extractor output and runner command/RSS records, too.
            expected = formula_speech(package)
            for name,tail in [('pdftotext',['-enc','UTF-8','-raw',case/'output.pdf','-']),
                              ('mutool',['draw','-q','-F','txt',case/'output.pdf'])]:
                _,out,_ = checked(executor,'scale-extract-'+name,[selected['paths'][name],*tail],stdout=True)
                extracted_occurrences(out,expected,name)
            placements = strict_json(placement_path.read_bytes())
            if render_names is None:
                raster = render_pages(job,corpus,case/'output.pdf',placements,case/'render',
                    tool_paths={name:selected['paths'][name] for name in ('mutool','pdftoppm','rsvg-convert')})
                raster['origin'] = 'fresh-raw-svg-render-commands'
            else:
                raster = reused_render(args,kind,case,selected,placements,corpus,render_names,(sha,write_json))
            observation['phases']['raw_svg_render'] = raster
            observation['phases']['pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,selected['paths'])
            observation['passed'] = True
        except Exception as error:
            observation['error'] = {'type':type(error).__name__,'message':str(error)}
        write_json(case/'observed.json',observation)
        results[case_id] = observation
    return results
