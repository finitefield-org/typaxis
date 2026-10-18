#!/usr/bin/env python3
"""Run source-bound VMB book verification and retain every phase's evidence.

The complete fixture matrix is required for a successful fixtures-mode result.
Unavailable original-book fixtures and unfinished body/layout checks remain
explicit failed gates. A successful formula-only case cannot satisfy them.
"""
import argparse
import hashlib
import importlib.metadata
import json
import os
import platform
import re
import shutil
import signal
import subprocess
import sys
import time
from pathlib import Path
from xml.etree import ElementTree as ET

from vmb_book_inputs import canonical_bytes, digest, read_canonical, require
from vmb_book_inputs import REQUIRED_CLI_TESTS, verify_fixture_inputs, verify_test_names
from vmb_probe_io import contained_bytes, same_json, strict_json


ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT/'samples/machine-package/staging/production-book-1'
PROFILE = 'typaxis.machine-pdf/production-book-1'
GROUPS = ('engine-small','chapter-terminal-whitespace','original-max-paths','original-max-segments',
          'chapter-300-500','overflow','resource-boundaries','mixed-body-spacing','negation-slash-controls',
          'distinct-5000','mixed-5000','alias-8000')


def sha(path):
    with Path(path).open('rb') as source:
        return hashlib.file_digest(source,'sha256').hexdigest()


def write_json(path, value):
    with Path(path).open('xb') as output:
        output.write(canonical_bytes(value))


def files(root, exclude=()):
    result = {}
    for path in sorted(Path(root).rglob('*')):
        if path.relative_to(root).parts[0] in exclude:
            continue
        require(not path.is_symlink(), 'symlink verification input/output')
        if path.is_file():
            result[str(path.relative_to(root))] = {'byte_length':path.stat().st_size,'sha256':sha(path)}
    return result


class Executor:
    def __init__(self, output, timeout=1200):
        self.output,self.timeout,self.records = output,timeout,[]
        self.directory = output/'commands'
        self.directory.mkdir()
        self.env = os.environ.copy()
        self.removed = sorted(key for key in self.env if key.startswith(('TYPAXIS_','VMB_BOOK_'))
            or key in {'JAVA_HOME','JAVACMD','REPO','CLASSPATH_PREFIX','JAVA_OPTS','JAVACMD_OPTS',
                       'JAVA_TOOL_OPTIONS','_JAVA_OPTIONS','JDK_JAVA_OPTIONS','VERAPDF_OPTS'})
        for key in self.removed:
            self.env.pop(key)
        self.env.update(LC_ALL='C',PYTHONDONTWRITEBYTECODE='1')

    def run(self, role, argv, *, cwd=ROOT, extra_env=None):
        require(re.fullmatch(r'[a-z0-9-]+',role), 'command role')
        stem = f'{len(self.records):03d}-{role}'
        stdout,stderr = self.directory/(stem+'.stdout'),self.directory/(stem+'.stderr')
        record_path = self.directory/(stem+'.json')
        argv = list(map(str,argv))
        environment = {**self.env,**(extra_env or {})}
        started = time.monotonic_ns()
        base = {'argv':argv,'cwd':str(Path(cwd).resolve()),'role':role,
            'stdout':str(stdout.relative_to(self.output)),'stderr':str(stderr.relative_to(self.output)),
            'removed_environment_keys':self.removed,'explicit_environment':extra_env or {}}
        write_json(record_path,{**base,'state':'starting'})
        with stdout.open('xb') as out,stderr.open('xb') as err:
            try:
                process = subprocess.Popen(argv,cwd=cwd,env=environment,stdout=out,stderr=err,start_new_session=True)
            except OSError as error:
                record = {**base,'state':'spawn-failed','exit_code':None,'timed_out':False,
                    'elapsed_ns':time.monotonic_ns()-started,'error':str(error),
                    'stdout_bytes':0,'stderr_bytes':0,'stdout_sha256':sha(stdout),'stderr_sha256':sha(stderr)}
                record_path.write_bytes(canonical_bytes(record))
                self.records.append(record)
                return record,b'',b''
            record_path.write_bytes(canonical_bytes({**base,'state':'running','pid':process.pid}))
            timed_out = False
            while True:
                pid,status,usage = os.wait4(process.pid,os.WNOHANG)
                if pid:
                    break
                if time.monotonic_ns()-started > self.timeout*1_000_000_000:
                    timed_out = True
                    try:
                        os.killpg(process.pid,signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                    _,status,usage = os.wait4(process.pid,0)
                    break
                time.sleep(0.05)
            process.returncode = os.waitstatus_to_exitcode(status)
        record = {**base,'state':'complete','pid':process.pid,'exit_code':process.returncode,
            'timed_out':timed_out,'elapsed_ns':time.monotonic_ns()-started,
            'peak_rss_bytes':int(usage.ru_maxrss)*(1 if sys.platform == 'darwin' else 1024),
            'rss_scope':'wait4 child maximum; not simultaneous aggregate process-tree RSS',
            'stdout_bytes':stdout.stat().st_size,'stderr_bytes':stderr.stat().st_size,
            'stdout_sha256':sha(stdout),'stderr_sha256':sha(stderr)}
        record_path.write_bytes(canonical_bytes(record))
        self.records.append(record)
        return record,stdout.read_bytes(),stderr.read_bytes()


def checked(executor, role, argv, *, cwd=ROOT, extra_env=None, stdout=False, stderr=False):
    record,out,err = executor.run(role,argv,cwd=cwd,extra_env=extra_env)
    require(record['exit_code'] == 0 and not record['timed_out'], role+': command failed')
    require((stdout or not out) and (stderr or not err), role+': unexpected command output')
    return record,out,err


def check_file(base, record):
    require(isinstance(record,dict) and {'path','byte_length','sha256'} <= set(record)
            and type(record['byte_length']) is int and record['byte_length'] >= 0
            and isinstance(record['sha256'],str) and re.fullmatch(r'[0-9a-f]{64}',record['sha256']),
            'complete typed expected file record')
    data = contained_bytes(base,record['path'])
    require(len(data) == record['byte_length'] and digest(data) == record['sha256'],
            'expected file hash/length: '+record['path'])
    return data


def tool_policy(path, executor):
    policy = strict_json(path.read_bytes())
    require(policy['algorithm'] == 'typaxis.vmb-book-tool-policy/1', 'VMB book tool policy identity')
    from verify_vmb_scale_render import POLICY
    require(same_json(policy['render_policy'],POLICY), 'fixed current VMB render policy')
    require(sys.flags.optimize == 0, 'verification requires Python assertions enabled')
    for name,version in policy['python_packages'].items():
        require(importlib.metadata.version(name) == version, 'pinned Python package '+name)
    host = sys.platform+'-'+platform.machine()
    require(host in policy['hosts'], 'tool policy has no verified host entry for '+host)
    selected = policy['hosts'][host]
    paths = {}
    for name,record in selected['tools'].items():
        candidate = Path(record['path'])
        candidate = candidate if candidate.is_absolute() else ROOT/candidate
        candidate = candidate.resolve(strict=True)
        require(candidate.is_file() and candidate.stat().st_size == record['byte_length']
                and sha(candidate) == record['sha256'], 'pinned tool bytes: '+name)
        paths[name] = str(candidate)
    require(set(paths) == {'mutool','pdftoppm','pdftotext','pdfinfo','rsvg-convert','java','verapdf'},
            'complete PDF tool role set')
    executor.env['JAVACMD'] = paths['java']
    executor.env['JAVA_HOME'] = str(Path(paths['java']).parent.parent)
    versions = {}
    for name,record in selected['tools'].items():
        _,out,err = checked(executor,'version-'+name,[paths[name],*record['version_arguments']],stdout=True,stderr=True)
        require(out.decode('utf-8') == record['version_stdout']
                and err.decode('utf-8') == record['version_stderr'], 'pinned tool version output: '+name)
        versions[name] = {'path':paths[name],'sha256':record['sha256'],
                          'stdout':out.decode('utf-8'),'stderr':err.decode('utf-8')}
    for library,digest_value in selected['runtime_library_sha256'].items():
        require(sha(library) == digest_value, 'pinned runtime library: '+library)
    for record in policy['source_archives']:
        check_file(ROOT,record)
    for name,digest_value in policy['verapdf_jars'].items():
        require(sha(ROOT/name) == digest_value, 'pinned veraPDF jar: '+name)
    return {'policy_sha256':sha(path),'paths':paths,'versions':versions,
            'python_packages':policy['python_packages'],'policy':policy}


def cli_receipts(directory, typaxis, fixture_sha):
    require(set(path.name for path in directory.iterdir()) == {name+'.json' for name in REQUIRED_CLI_TESTS},
            'exact CLI invocation receipt set')
    result = []
    for name in sorted(REQUIRED_CLI_TESTS):
        value = read_canonical(directory,name+'.json')
        require(set(value) == {'algorithm','test_name','binary_path','binary_sha256','fixture_index_sha256',
                               'test_source_sha256','source_cases','source_occurrences'}, 'CLI invocation receipt fields')
        require(value['algorithm'] == 'typaxis.vmb-book-cli-test-invocation/1' and value['test_name'] == name
                and Path(value['binary_path']).resolve() == typaxis and value['binary_sha256'] == sha(typaxis)
                and value['test_source_sha256'] == sha(ROOT/'workspace/crates/typaxis-cli/tests/cli_end_to_end.rs')
                and value['fixture_index_sha256'] == fixture_sha
                and type(value['source_cases']) is int and value['source_cases'] == 20
                and type(value['source_occurrences']) is int and value['source_occurrences'] == 20,
                'CLI tests bind the requested executable and source corpus')
        result.append(value)
    return result


def run_cli_tests(args, executor, fixture_sha):
    binary = args.test_binary
    if binary is None:
        target = args.output_root/'compiler-cache'
        _,out,_ = checked(executor,'compile-cli-tests',['cargo','test','--locked','--offline',
            '--manifest-path',ROOT/'workspace/Cargo.toml','--target-dir',target,'-p','typaxis-cli',
            '--test','cli_end_to_end','--no-run','--message-format=json'],stdout=True,stderr=True,
            extra_env={'CARGO_PROFILE_DEV_DEBUG':'0','CARGO_PROFILE_TEST_DEBUG':'0','CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'2'})
        candidates = []
        for line in out.splitlines():
            value = strict_json(line)
            if value.get('reason') == 'compiler-artifact' and value.get('target',{}).get('name') == 'cli_end_to_end' and value.get('executable'):
                candidates.append(Path(value['executable']))
        require(len(candidates) == 1, 'one compiled CLI integration test executable')
        binary = candidates[0]
    binary = binary.resolve(strict=True)
    _,listing,_ = checked(executor,'cli-test-discovery',[binary,'vmb_book_','--list'],stdout=True)
    receipt_dir = args.output_root/'cli-test-invocations'
    receipt_dir.mkdir()
    environment = {'VMB_BOOK_TYPAXIS':str(args.typaxis),'VMB_BOOK_CLI_EVIDENCE_DIR':str(receipt_dir)}
    _,output,_ = checked(executor,'cli-tests',[binary,'vmb_book_','--test-threads=1'],stdout=True,extra_env=environment)
    names = verify_test_names(listing.decode(),output.decode())
    receipts = cli_receipts(receipt_dir,args.typaxis,fixture_sha)
    _,zero,_ = checked(executor,'cli-zero-match-control',[binary,'vmb_book_intentionally_absent','--exact'],stdout=True)
    try:
        verify_test_names(listing.decode(),zero.decode())
    except ValueError as error:
        rejection = str(error)
    else:
        raise ValueError('zero-test successful process was accepted by the fixture gate')
    return {'test_binary':str(binary),'test_binary_sha256':sha(binary),'names':names,
            'invocations':receipts,'actual_zero_match_rejected':True,'zero_match_error':rejection}


def artifact_checks(directory, package, validators, package_record):
    import verify_machine_profile as machine
    pdf = directory/'output.pdf'
    manifest = strict_json((directory/'manifest.json').read_bytes())
    trace = strict_json((directory/'trace.json').read_bytes())
    diagnostics = strict_json((directory/'build-diagnostics.json').read_bytes())
    for name,value in [('build-manifest.schema.json',manifest),('layout-trace.schema.json',trace),('diagnostics.schema.json',diagnostics)]:
        machine._validate_instance(validators,name,value,'VMB '+name)
    require(diagnostics['diagnostics'] == [] and manifest['status'] == 'built', 'positive public build diagnostics/status')
    require(manifest['output']['sha256'] == sha(pdf) and manifest['output']['bytes'] == pdf.stat().st_size,
            'public manifest binds actual PDF bytes')
    require(manifest['package_input']['sha256'] == package_record['sha256']
            and manifest['package_input']['bytes'] == package_record['byte_length']
            and manifest['package_input']['contract'] == package['contract'], 'public manifest binds exact source package')
    machine._assert_profile_receipt_closure({'profile':PROFILE},manifest,trace)
    authored_math = any(row['kind'] in ('math_vector','math_vector_block') for row in source_inventory(package))
    machine._assert_production_vector_closure({'profile':PROFILE},manifest,trace,require_math=authored_math)
    require(authored_math or manifest['math_vector_manifest']['facts'] == [], 'no invented formula facts')
    return manifest,trace


def run_public_builds(typaxis, job, case, executor, validators, config=None):
    package = strict_json(contained_bytes(job,'document-package.json'))
    before = files(job)
    common = ['--package-root','.', '--profile',PROFILE]
    if config is not None:
        common += ['--config',str(config)]
    checked(executor,'check-package',[typaxis,'check-package','document-package.json',*common,
        '--emit-diagnostics',case/'check-diagnostics.json'],cwd=job)
    import verify_machine_profile as machine
    check = strict_json((case/'check-diagnostics.json').read_bytes())
    machine._validate_instance(validators,'diagnostics.schema.json',check,'VMB check diagnostics')
    require(check['diagnostics'] == [], 'positive public check diagnostics')
    runs = []
    for ordinal in (1,2):
        directory = case/('build'+str(ordinal))
        directory.mkdir()
        checked(executor,'build-package',[typaxis,'build-package','document-package.json',*common,
            '-o',directory/'output.pdf','--trace',directory/'trace.json','--trace-text',
            '--emit-build-manifest',directory/'manifest.json','--emit-diagnostics',directory/'build-diagnostics.json',
            '--no-compress'],cwd=job)
        artifact_checks(directory,package,validators,before['document-package.json'])
        runs.append(directory)
    for name in ('output.pdf','manifest.json','trace.json','build-diagnostics.json'):
        require((runs[0]/name).read_bytes() == (runs[1]/name).read_bytes(), 'deterministic public artifact '+name)
        shutil.copyfile(runs[0]/name,case/name)
    require(files(job) == before, 'public commands preserve all source inputs')
    return {'origin':'fresh-public-commands','package_sha256':sha(job/'document-package.json'),
            'pdf_sha256':sha(case/'output.pdf'),'artifact_byte_identity':True,'input_files':before}


def ua_check(pdf, case, executor, tools):
    _,out,_ = checked(executor,'verapdf-ua1',[tools['verapdf'],'--format','xml','--flavour','ua1',pdf],stdout=True)
    (case/'verapdf.xml').write_bytes(out)
    return parse_ua_report(out)


def parse_ua_report(out):
    tree = ET.fromstring(out)
    reports = tree.findall('.//validationReport')
    require(len(reports) == 1, 'exactly one veraPDF validation report')
    report,batch = reports[0],tree.find('batchSummary')
    require(report is not None and report.get('jobEndStatus') == 'normal' and report.get('isCompliant') == 'true',
            'veraPDF UA1 normal compliant result')
    details = report.find('details')
    require(details is not None and details.get('failedChecks') == '0', 'veraPDF has no failed checks')
    require(batch is not None and batch.attrib == {'totalJobs':'1','failedToParse':'0','encrypted':'0',
        'outOfMemory':'0','veraExceptions':'0'}, 'veraPDF batch/parse coverage')
    passed = details.get('passedChecks','')
    require(re.fullmatch('[1-9][0-9]*',passed) is not None, 'veraPDF nonzero completed checks')
    return {'flavour':'ua1','passed_checks':int(passed),'failed_checks':0,
            'human_acceptance':False,'report_sha256':digest(out)}


def source_inventory(package):
    output,seen = [],set()
    def walk(value,pointer):
        if isinstance(value,list):
            for index,child in enumerate(value):
                walk(child,pointer+'/'+str(index))
        elif isinstance(value,dict):
            if 'node_id' in value:
                owner = value['node_id']
                require(type(owner) is int and owner >= 0 and owner not in seen, 'unique typed book source node')
                seen.add(owner)
            if value.get('kind') in ('math_vector','math_vector_block','figure','vector_figure'):
                require(type(value['image_id']) is int and value['image_id'] >= 0, 'typed book image occurrence')
                output.append({'node_id':value['node_id'],'image_id':value['image_id'],'kind':value['kind'],
                    'source_pointer':pointer,'source_ordinal':len(output),'node_sha256':digest(canonical_bytes(value))})
            for key in ('term','children','blocks','caption','head','body','items','cells','equation_number','footnotes'):
                if key in value:
                    walk(value[key],pointer+'/'+key)
    walk(package['document'],'/document')
    require(output, 'nonempty complete book source occurrence inventory')
    return output


def book_inputs(package_root, expected_path):
    expected = strict_json(expected_path.read_bytes())
    require(expected['algorithm'] == 'typaxis.vmb-book-expectation/1'
            and {'package','config','font_files','image_files','source_files','source_occurrences','fixture_evidence'} <= set(expected),
            'required book package/config/fonts/resources/source expectation')
    raw = check_file(package_root,expected['package'])
    require(expected['package']['path'] == 'document-package.json', 'book package path')
    package = strict_json(raw)
    check_file(package_root,expected['config'])
    require(isinstance(expected['config'].get('effective_sha256'),str)
            and re.fullmatch(r'[0-9a-f]{64}',expected['config']['effective_sha256']), 'expected effective config hash')
    for member,kind,id_key in [('font_files','font_faces','font_face_id'),('image_files','images','image_id'),('source_files','sources','source_id')]:
        declarations = package['sources'] if kind == 'sources' else package['resources'][kind]
        records = expected[member]
        require(isinstance(records,list) and len(records) == len(declarations), 'complete book file inventory '+member)
        seen = set()
        for declaration,record in zip(declarations,records):
            require(type(record[id_key]) is int and type(declaration[id_key]) is int
                    and record[id_key] >= 0 and record[id_key] not in seen
                    and record[id_key] == declaration[id_key] and record['path'] == declaration['uri'], 'book declaration/file join')
            seen.add(record[id_key])
            check_file(package_root,record)
            wanted = declaration['sha256'] if kind == 'sources' else declaration.get('expected_sha256')
            require(wanted == record['sha256'], 'book declared and expected file hash')
    inventory = source_inventory(package)
    require(same_json(inventory,expected['source_occurrences']), 'complete book source occurrence inventory')
    return expected,package,inventory


def verify_fixture_evidence(evidence, fixture_sha, policy_sha, typaxis):
    from vmb_book_scale import indexed_files
    require(isinstance(evidence,dict) and {'path','sha256','artifact_index_sha256'} <= set(evidence),
            'book requires the complete pinned fixture artifact index')
    source = Path(evidence['path'])
    require(source.is_absolute() and source.name == 'observed.json' and sha(source) == evidence['sha256'],
            'book requires pinned fixture observations')
    prior = strict_json(source.read_bytes())
    require(prior.get('algorithm') == 'typaxis.vmb-book-verification/1' and prior.get('mode') == 'fixtures'
            and prior.get('passed') is True and prior.get('fixture_index_sha256') == fixture_sha
            and isinstance(prior.get('groups'),dict) and set(prior['groups']) == set(GROUPS)
            and all(group.get('passed') is True for group in prior['groups'].values())
            and prior.get('typaxis',{}).get('sha256') == sha(typaxis),
            'book mode cannot omit the successful complete fixture gate')
    index,names = indexed_files(source.parent,evidence['artifact_index_sha256'],
        'typaxis.vmb-book-artifacts/1',sha,check_file)
    require(index.get('mode') == 'fixtures' and index.get('passed') is True
            and {'observed.json','tool-observation.json'} <= names, 'indexed successful fixture result')
    policy = strict_json(contained_bytes(source.parent,'tool-observation.json'))
    require(policy['policy_sha256'] == policy_sha, 'fixture evidence uses the same pinned tool policy')
    return {'observed_sha256':evidence['sha256'],'artifact_index_sha256':evidence['artifact_index_sha256'],
            'verified_indexed_files':len(names)}


def run_engine_case(args, executor, tools, validators):
    from prepare_vmb_book_fixtures import prepare
    from vmb_book_pdf import verify_fixture_pdf,render_fixture_pdf
    case = args.output_root/'engine-small'
    case.mkdir()
    observation = {'algorithm':'typaxis.vmb-book-case-observed/1','case_id':'engine-small','phases':{},'passed':False}
    try:
        job = case/'job'
        observation['phases']['source'] = prepare(args.fixture_root,job)
        observation['phases']['public_commands'] = run_public_builds(args.typaxis,job,case,executor,validators)
        placement = case/'placements.json'
        checked_pdf = verify_fixture_pdf(args.fixture_root,job,case/'output.pdf',placement,
            extraction_tools={key:tools[key] for key in ('mutool','pdftotext')})
        write_json(case/'source-paint-structure.json',checked_pdf)
        observation['phases']['source_paint_structure'] = {'passed':True,'sha256':sha(case/'source-paint-structure.json')}
        observed = strict_json(placement.read_bytes())
        observation['phases']['raw_svg_render'] = render_fixture_pdf(args.fixture_root,job,case/'output.pdf',observed,
            case/'render',tool_paths={key:tools[key] for key in ('mutool','pdftoppm','rsvg-convert')})
        observation['phases']['pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,tools)
        observation['passed'] = True
    except Exception as error:
        observation['error'] = {'type':type(error).__name__,'message':str(error)}
    write_json(case/'observed.json',observation)
    return observation


def main(arguments=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--typaxis',type=Path,required=True)
    parser.add_argument('--fixture-root',type=Path,default=BASE/'vmb-book')
    parser.add_argument('--output-root',type=Path,required=True)
    parser.add_argument('--mode',choices=('fixtures','book'),required=True)
    parser.add_argument('--package-root',type=Path)
    parser.add_argument('--expected',type=Path)
    parser.add_argument('--tool-policy',type=Path,default=BASE/'vmb-book-tool-policy.json')
    parser.add_argument('--test-binary',type=Path)
    parser.add_argument('--scale-build-evidence',type=Path,
        help='Explicitly reuse policy-pinned check/two-build evidence from an earlier scale run.')
    parser.add_argument('--scale-render-evidence',type=Path,
        help='Explicitly reuse policy-pinned complete raw-SVG raster evidence.')
    parser.add_argument('--command-timeout',type=int,default=1200)
    args = parser.parse_args(arguments)
    args.repository_root = ROOT
    args.typaxis,args.fixture_root,args.output_root = args.typaxis.resolve(),args.fixture_root.resolve(),args.output_root.resolve()
    if not args.typaxis.is_file():
        parser.error('--typaxis must name an existing executable')
    protected = [args.fixture_root]
    for name in ('scale_build_evidence','scale_render_evidence'):
        path = getattr(args,name)
        if path is not None:
            path = path.resolve(strict=True)
            require(path.is_dir(), 'scale evidence must be a directory')
            setattr(args,name,path)
            protected.append(path)
    if args.mode == 'book':
        require(args.package_root is not None and args.expected is not None, 'book mode requires --package-root and --expected')
        args.package_root = args.package_root.resolve()
        protected.append(args.package_root)
    require(args.command_timeout > 0 and all(not args.output_root.is_relative_to(path)
            and not path.is_relative_to(args.output_root) for path in protected), 'exclusive output outside all source inputs')
    args.output_root.mkdir(parents=True)
    executor = Executor(args.output_root,args.command_timeout)
    observation = {'algorithm':'typaxis.vmb-book-verification/1','mode':args.mode,'passed':False,
        'full_goal_complete':False,'groups':{name:{'passed':False,'state':'not-run'} for name in GROUPS},
        'verifier_sha256':sha(Path(__file__)),
        'verification_source_sha256':{str(path.relative_to(ROOT)):sha(path) for path in
            sorted((ROOT/'tools').glob('*.py')) if path.name.startswith(('vmb_','verify_vmb','prepare_vmb','verify_machine_profile','verify_pdf_structure'))}}
    try:
        facts = verify_fixture_inputs(args.fixture_root)
        observation['fixture_index_sha256'] = facts['index_sha256']
        selected = tool_policy(args.tool_policy,executor)
        write_json(args.output_root/'tool-observation.json',{key:value for key,value in selected.items() if key != 'policy'})
        import verify_machine_profile as machine
        validators = machine._schema_validators(ROOT)
        _,capabilities,_ = checked(executor,'capabilities',[args.typaxis,'capabilities','--format','json'],stdout=True)
        capabilities = strict_json(capabilities)
        machine._validate_instance(validators,'machine-capabilities.schema.json',capabilities,'VMB capabilities')
        machine._assert_profile_closure(capabilities,{PROFILE})
        tests = run_cli_tests(args,executor,facts['index_sha256'])
        observation['cli_tests'] = tests
        observation['groups']['chapter-terminal-whitespace'] = {'passed':True,'state':'verified-by-named-cli-test'}
        case = run_engine_case(args,executor,selected['paths'],validators)
        observation['groups']['engine-small'] = {'passed':case['passed'],'state':'verified' if case['passed'] else 'failed'}
        from vmb_book_thin import run_case as run_thin_case
        thin = run_thin_case(args,executor,selected,validators,(sha,write_json,run_public_builds,ua_check))
        observation['groups']['negation-slash-controls'] = {'passed':thin['passed'],
            'state':'verified' if thin['passed'] else 'failed'}
        from vmb_book_boundaries import run_case as run_boundaries
        boundaries = run_boundaries(args,executor,selected,validators,(sha,write_json,run_public_builds,ua_check))
        observation['groups']['resource-boundaries'] = {'passed':boundaries['passed'],
            'state':'verified' if boundaries['passed'] else 'failed'}
        from vmb_book_chapter import run_case as run_chapter
        chapter = run_chapter(args,executor,selected,validators,(sha,write_json,run_public_builds,ua_check,checked))
        observation['groups']['chapter-300-500'] = {'passed':chapter['passed'],
            'state':'verified' if chapter['passed'] else 'failed'}
        from vmb_book_mixed import run_case as run_mixed_body
        mixed_body = run_mixed_body(args,executor,selected,validators,(sha,write_json,run_public_builds,ua_check,checked))
        observation['groups']['mixed-body-spacing'] = {'passed':mixed_body['passed'],
            'state':'verified' if mixed_body['passed'] else 'failed'}
        from vmb_book_scale import run_cases
        args.scale_config_sha256 = selected['policy']['scale_cache']['config_sha256']
        scales = run_cases(args,executor,selected,validators,
            (sha,check_file,checked,write_json,artifact_checks,run_public_builds,ua_check))
        for name,result in scales.items():
            observation['groups'][name] = {'passed':result['passed'],
                'state':'verified' if result['passed'] else 'failed',
                'public_commands_origin':result.get('phases',{}).get('public_commands',{}).get('origin')}
        observation['groups']['original-max-paths']['reason'] = 'Original image 4187 source fixture is not yet registered.'
        observation['groups']['original-max-segments']['reason'] = 'Original image 2987 source fixture is not yet registered.'
        for name in GROUPS:
            group = observation['groups'][name]
            if group['state'] == 'not-run':
                group.update(state='required-work-remains',reason=group.get('reason','Required source/independent verifier group is not yet integrated.'))
        if args.mode == 'book':
            expected,package,inventory = book_inputs(args.package_root,args.expected)
            observation['book_inputs'] = {'package_sha256':digest(contained_bytes(args.package_root,'document-package.json')),
                'source_occurrences':len(inventory),'config_sha256':expected['config']['sha256']}
            observation['fixture_evidence'] = verify_fixture_evidence(expected['fixture_evidence'],facts['index_sha256'],
                selected['policy_sha256'],args.typaxis)
            case = args.output_root/'book'
            case.mkdir()
            write_json(case/'source-inventory.json',inventory)
            built = run_public_builds(args.typaxis,args.package_root,case,executor,validators,
                                     args.package_root/expected['config']['path'])
            manifest = strict_json((case/'manifest.json').read_bytes())
            require(manifest['config_sha256'] == expected['config']['effective_sha256'], 'book effective config hash')
            observation['book_public_commands'] = built
            from vmb_book_pdf import verify_book_formula_pdf
            observation['book_independent'] = verify_book_formula_pdf(args.package_root,package,expected,
                case/'output.pdf',case/'placements.json',case/'render',tools=selected['paths'])
            observation['book_pdf_ua_automated'] = ua_check(case/'output.pdf',case,executor,selected['paths'])
        observation['passed'] = all(group['passed'] for group in observation['groups'].values())
    except Exception as error:
        observation['error'] = {'type':type(error).__name__,'message':str(error)}
    observation['commands'] = len(executor.records)
    observation['typaxis'] = {'path':str(args.typaxis),'sha256':sha(args.typaxis)}
    observation['controlled_performance'] = False
    write_json(args.output_root/'observed.json',observation)
    records = files(args.output_root,exclude=('compiler-cache',))
    write_json(args.output_root/'artifact-index.json',{'algorithm':'typaxis.vmb-book-artifacts/1',
        'mode':args.mode,'passed':observation['passed'],'excluded':['reconstructible compiler-cache'],
        'files':[{'path':name,**value} for name,value in records.items()]})
    print(json.dumps({'passed':observation['passed'],'mode':args.mode,'commands':len(executor.records),
        'artifact_index_sha256':sha(args.output_root/'artifact-index.json'),'output_root':str(args.output_root)},indent=2))
    return 0 if observation['passed'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
