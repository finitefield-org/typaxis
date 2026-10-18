#!/usr/bin/env python3
"""Independent complete Column Page-label convergence snapshot verification.

Inputs retain their actual convergence scopes. The explicit adapters below
validate that authority and call the unchanged source/font/geometry/PDF kernels;
they never relabel a snapshot as a historical public driver callback. Original
shape/source preparation/copy requested bytes and earlier physical passes remain
issuer-linked witnesses. Complete successful attempts and bounded feedback are
verified; this is not a failed-attempt receipt or whole-book acceptance.
"""
from __future__ import annotations
import argparse
import copy
import hashlib
import importlib
import io
import json
import shutil
import struct
import sys
import tempfile
from collections import Counter
from pathlib import Path
import verify_book_v2_column_pdf as pdf
import verify_book_v2_column_running_regions as regions
import verify_book_v2_column_navigation as navigation

paints = pdf.paints
resources = regions.resources
display = regions.display
marked = regions.marked
check, fields, integer = regions.check, regions.fields, regions.integer
canonical, digest, fingerprint = regions.canonical, regions.digest, regions.fingerprint
strict_equal = pdf.strict_equal
sha = pdf.sha
artifact, PDFBudget = pdf.artifact, pdf.PDFBudget
TOP_FIELDS = regions.TOP_FIELDS
REGION_SCHEMA = regions.SCHEMA
ATTEMPT_SCHEMA = "typaxis.book-2-column-convergence-attempt/1"
SCHEMA = "typaxis.book-2-column-page-label-convergence-probe/1"
OBSERVED_SCHEMA = "typaxis.book-2-column-page-label-convergence-probe/2"
OBSERVED_ATTEMPT_SCHEMA = "typaxis.book-2-column-convergence-attempt/2"
SOURCE_PREFIX_LIMIT = "observed Source/display prefixes are issuer-linked witnesses, not allocator traces"
for _name in ("verify_base", "expected_region_set", "verify_local_source", "project_region",
              "verify_join", "verify_semantics"):
    globals()[_name] = getattr(regions, _name)
for _name in ("closed_structure", "verify_frames", "verify_marked_bytes", "prior_hashes", "assembly_replay"):
    globals()[_name] = getattr(pdf, _name)

# This closed /2 orchestration retains the pinned successful lower kernels.
# The new real Source/display witness replaces only the obsolete unchanged-
# spool premise. No historical snapshot is normalized or relabeled.
def verify_observed_downstream(root, probe, admitted, instances, locators, join, observed_source_prefix):
    stage = fields(
        probe["downstream"],
        (
            "encoded_prefix",
            "marked_prefix",
            "relations_prefix",
            "caller_before",
            "constructors",
            "content",
            "structure",
            "relations",
            "navigation",
        ),
        "actual downstream issuer prefixes",
    )
    ctors = stage["constructors"]
    caller = paints.PDFBudget(stage["caller_before"])
    check(len(ctors) == 13, "all thirteen actual constructor observations")
    for c in ctors:
        paints.PDFBudget(c)
    first = ctors[0]
    check(
        caller.records < first[0] and caller.work <= first[3],
        "caller source/display floors retained before first constructor",
    )
    if join is not None:
        check(
            first[0] == join["records"] + 1 and first[3] == join["work"],
            "actual joined display font constructor prefix",
        )
    else:
        check(
            first[0] >= probe["base_display"]["observed"]["display_records"] + 1
            and first[3] >= probe["base_display"]["observed"]["display_work"],
            "no-region actual source floor",
        )
    r, spool, output, work = paints.PDFBudget(observed_source_prefix).triple4()
    check(first == [r + 1, spool, output, work], "observed Source/display prefix binds the actual first constructor")
    check(spool >= max(probe["base_display"]["observed"]["source_spool"], stage["caller_before"][1])
          and output == stage["caller_before"][2], "observed Source retains terminal spool and caller output")
    uses, slots, textslots = regions.derive_uses(probe, instances, locators)
    budget = resources.Budget([first[0], first[1], first[3]])
    regions.verify_selection(probe, instances, uses, slots, budget)
    sources = resources.verify_closures(root, probe, budget)
    resources.verify_programs(root, probe, sources, budget)
    resources.verify_cids(probe, uses, sources, budget)
    regions.verify_font_metrics(probe, sources)
    pdf = paints.PDFBudget([budget.records, budget.spool, first[2], budget.work])

    def ctor(index, extra_work=0):
        pdf.charge(records=1, work=extra_work)
        check(
            ctors[index] == pdf.triple4(),
            "accepted downstream constructor prefix " + str(index),
        )

    ctor(1)
    streams = paints.verify_font_streams(root, probe, pdf)
    ctor(2)
    paints.verify_font_objects(root, probe, streams, pdf)
    ctor(3)
    commands = regions.verify_text_commands(root, probe, streams, textslots, pdf)
    command_fp, image_counts = regions.verify_image_stages(root, probe, admitted, pdf, ctor)
    ctor(8)
    regions.verify_scope_projection(probe, pdf, command_fp, locators)
    ctor(9)
    # The wire intentionally retains later Content/source/relations/navigation
    # arrays as issuer facts; their complete record sets live in the separately
    # validated structural wire rather than being invented by this checker.
    for value in (
        stage["encoded_prefix"],
        probe["marked_scopes"]["charges"],
        stage["content"],
        stage["structure"],
        stage["relations"],
        stage["navigation"],
        stage["marked_prefix"],
        stage["relations_prefix"],
    ):
        paints.PDFBudget(value)
    lower = pdf.triple4()
    for index, c in enumerate(ctors[10:], 10):
        check(
            all(c[i] >= lower[i] for i in (0, 1, 3)),
            "accepted downstream R/S/W prefix " + str(index),
        )
        lower = c
    check(
        stage["structure"] == stage["marked_prefix"]
        and stage["relations"] == stage["relations_prefix"],
        "actual completed typed source/relations handoffs",
    )
    check(
        stage["content"][2]
        == stage["structure"][2]
        == stage["relations"][2]
        == stage["navigation"][2],
        "settled Content output is unchanged by source/relations/navigation",
    )
    check(
        ctors[10]
        == [
            stage["content"][0] + 1,
            stage["content"][1],
            stage["content"][2],
            stage["content"][3],
        ],
        "Source constructor has no output credit restoration",
    )
    check(
        ctors[11]
        == [
            stage["structure"][0] + 1,
            stage["structure"][1],
            stage["structure"][2],
            stage["structure"][3],
        ],
        "Relation constructor exact completed source budget",
    )
    check(
        ctors[12]
        == [
            stage["relations"][0] + 1,
            stage["relations"][1],
            stage["relations"][2],
            stage["relations"][3],
        ],
        "Navigation constructor exact completed relations budget",
    )
    observed = probe["observed"]
    check(
        [observed[k] for k in ("records", "spool", "output", "work")]
        == stage["navigation"],
        "actual navigation callback accepted final quartet",
    )
    return (
        len(uses),
        sum(len(f["selected"]) for f in probe["fonts_selection"]["fonts"]),
        commands,
        image_counts,
    )


def verify_downstream(root, probe, admitted, instances, locators, join, observed_source_prefix=None):
    if observed_source_prefix is None:
        return regions.verify_downstream(root, probe, admitted, instances, locators, join)
    return verify_observed_downstream(root, probe, admitted, instances, locators, join, observed_source_prefix)

# Closed normal-path adapters: cache/manual branches from historical orchestration
# are absent. Each preserves the caller's original scope and data bytes.
def verify_region_attempt(root, probe, source_display_prefix=None):
    fields(probe, TOP_FIELDS, 'closed running-region wire')
    check(probe['wire'] == REGION_SCHEMA and type(probe['case']) is str and probe['case'] and ('/' not in probe['case']), 'actual region callback schema/case')

    def artifact_fields(value):
        if isinstance(value, dict):
            if 'file' in value and 'sha256' in value:
                resources.verify_artifact_fields(value)
            else:
                for child in value.values():
                    artifact_fields(child)
        elif isinstance(value, list):
            for child in value:
                artifact_fields(child)
    for name in ('fonts_selection', 'programs', 'font_streams', 'font_objects', 'text_commands', 'original_images', 'images_selection', 'rasters', 'vector_programs', 'image_objects', 'image_commands'):
        artifact_fields(probe[name])
    check(canonical(probe['source_package']) == probe['source_canonical_jcs'], 'complete original source JCS bytes')
    check(digest(probe['source_canonical_jcs'].encode()) == fingerprint(probe['source_package_fingerprint']), 'original package canonical SHA')
    source, admitted = verify_base(probe)
    fields(probe['observed'], ('scope', 'maximum_work', 'records', 'spool', 'output', 'work', 'line_passes'), 'actual region driver observation')
    check(probe['observed']['scope'] == 'convergence_attempt', 'explicit complete convergence attempt scope')
    sizes = fields(probe['size_of'], ('paint', 'usize', 'locator', 'font_glyph', 'sfnt_rewrite_table', 'selected_font', 'selected_font_use', 'closed_font', 'font_program', 'cid_font', 'binding', 'cid_use', 'cid', 'font_stream', 'font_object', 'text_command', 'scope', 'option_range', 'range', 'selected_image', 'selected_image_use', 'raster_slot', 'vector_slot', 'vector_state', 'image_object', 'image_command'), 'actual diagnostic reservation units')
    check(all((integer(v, 64, False) > 0 for v in sizes.values())), 'actual positive reservation sizes')
    check(sizes['usize'] in (4, 8) and sizes['locator'] == 2 * sizes['usize'] and (sizes['cid'] == 2), 'actual locator/pointer/CID footprint')
    check(sizes['paint'] == 2 * sizes['usize'], 'unchanged two-word typed paint footprint')
    names = probe['original_page_names']
    check(type(names) is list and len(names) == len(set(names)) and all((type(n) is str and n for n in names)), 'original unique source page-name registry')
    declared = {d['value']['value'] for r in probe['source_package']['style_sheet']['rules'] for d in r['declarations'] if d['name'] == 'page'}
    check(set(names) <= declared, 'retained source page names originate in authored style')
    check(len(probe['pages']) == len(source['pages']), 'all physical pages including blanks')
    for page, actual in zip(probe['pages'], source['pages']):
        fields(page, ('index', 'named_page', 'selected_master_id', 'full_body', 'declared_note', 'columns', 'balance_fingerprint'), 'actual selected page')
        check(page['columns'] == [c['bounds'] for c in actual['columns']] and page['declared_note'] == actual['footnote'] and (page['full_body'] == actual['body']), 'actual source-terminal frames')
        check((page['named_page'] is None) == (actual['named_index'] is None), 'actual selected named-source presence')
        check(page['named_page'] == (None if actual['named_index'] is None else probe['original_page_names'][actual['named_index']]), 'exact original source-page name registry binding')
        balance = next((b for b in source['balances'] if b['page'] == page['index']), None)
        check(page['balance_fingerprint'] == (None if balance is None else balance['fingerprint']), 'actual sealed balance occurrence')
    expected = expected_region_set(probe['source_package'], probe['pages'])
    check(len(expected) == len(probe['regions']) == len(probe['original_region_lines']), 'complete selected region/local witness set')
    instances = resources.verify_font_instances(dict(original_font_instances=probe['original_font_instances'], display_probe=probe['base_display']), admitted)
    local_prefix = [2 * len(admitted['fonts']) + 1] * 2
    for local, region, want in zip(probe['original_region_lines'], probe['regions'], expected):
        fields(region, ('page', 'owner', 'kind', 'frame', 'fingerprint', 'layout_fingerprint', 'records', 'work', 'owned_records', 'projection_work', 'draws'), 'complete owned region projection')
        resources.strict_equal(local['builder_before'], local_prefix, 'persistent zero-prior local projection builder prefix')
        verify_local_source(probe, local, want, admitted)
        project_region(probe, local, region, instances)
        local_prefix = local['builder_after']
    join, locators = verify_join(probe)
    verify_semantics(probe, locators)
    body_owners = set()

    def owners(value):
        if isinstance(value, dict):
            if 'node_id' in value:
                body_owners.add(value['node_id'])
            for child in value.values():
                owners(child)
        elif isinstance(value, list):
            for child in value:
                owners(child)
    owners(probe['source_package']['document'])
    for name in ('source_structure_owners', 'navigation_owners'):
        check(type(probe[name]) is list and all((integer(o, 32, False) in body_owners for o in probe[name])), 'actual body source owner namespace ' + name)
    if source_display_prefix is not None:
        observation(source_display_prefix, admitted['limits']['base'], probe['observed']['maximum_work'])
    uses, gids, commands, image_counts = verify_downstream(
        root, probe, admitted, instances, locators, join,
        None if source_display_prefix is None else source_display_prefix['charges'])
    caps = admitted['limits']['base']
    observed = probe['observed']
    check(0 < len(probe['pages']) <= caps['max_pages'], 'inclusive complete physical-page cap')
    for key in ('maximum_work', 'records', 'spool', 'output', 'work', 'line_passes'):
        integer(observed[key], 64, False)
    check(observed['maximum_work'] == probe['base_display']['observed']['maximum_work'] and observed['line_passes'] <= caps['max_line_reshape_passes'], 'same work ceiling and cumulative line-pass cap')
    check(sum((local['reshape_passes'] for local in probe['original_region_lines'])) <= observed['line_passes'], 'actual driver retains all original selected-region reshape passes')
    for values in [*probe['downstream']['constructors'], *[probe[k]['charges'] for k in ('font_streams', 'font_objects', 'text_commands', 'vector_programs', 'image_objects', 'image_commands', 'marked_scopes')], *[probe['downstream'][k] for k in ('encoded_prefix', 'content', 'structure', 'relations', 'navigation')]]:
        r, s, o, w = paints.PDFBudget(values).triple4()
        check(r <= caps['max_fragments'] and s <= caps['max_spool_bytes'] and (o <= caps['max_output_bytes']) and (w <= observed['maximum_work']), 'all captured accepted four-axis stage peaks within caps')
    for name in ('fonts_selection', 'closures', 'programs', 'cids', 'images_selection', 'rasters'):
        r, s, w = resources.Budget(probe[name]['charges']).triple()
        check(r <= caps['max_fragments'] and s <= caps['max_spool_bytes'] and (w <= observed['maximum_work']), 'captured successful resource stage caps')
    for font, program in zip(probe['fonts_selection']['fonts'], probe['programs']['fonts']):
        check(len(font['selected']) <= caps['max_cids_per_font'] and program['program']['length'] <= admitted['limits']['extension']['max_font_subset_bytes'], 'original CID/subset caps')
    check(0 < probe['font_objects']['first'] < probe['font_objects']['next'] <= caps['max_pdf_objects'] + 1 or (not probe['font_objects']['objects'] and 0 < probe['font_objects']['first'] == probe['font_objects']['next'] <= caps['max_pdf_objects'] + 1), 'actual exclusive font object cap')
    check(observed['records'] <= caps['max_fragments'] and observed['spool'] <= caps['max_spool_bytes'] and (observed['output'] <= caps['max_output_bytes']) and (observed['work'] <= observed['maximum_work']), 'inclusive actual callback caps')
    return dict(pages=len(probe['pages']), regions=len(expected), region_draws=len(locators), paints=len(probe['paints']), font_uses=uses, unique_gids=gids, text_commands=commands, navigation_prefix=probe['downstream']['navigation'], **image_counts)

def verify_attempt_pdf(root, probe):
    fields(probe, ('wire', 'case', 'source_canonical_jcs', 'limits_canonical_jcs', 'limits_fingerprint', 'pages', 'structural_probe', 'parent', 'navigation', 'engine', 'size_of', 'observed', 'assembly', 'font_objects', 'image_objects', 'marked', 'prior_stages'), 'closed actual Column PDF wire')
    check(probe['wire'] in (ATTEMPT_SCHEMA, OBSERVED_ATTEMPT_SCHEMA) and type(probe['case']) is str and probe['case'], 'actual complete Column PDF successful snapshot')
    import verify_book_v2_column_running_regions as regions
    core = importlib.import_module('verify_book_v2_pdf_assembly')
    old_nav = importlib.import_module('verify_book_v2_navigation_geometry')
    source, parent, prior = (probe['structural_probe'], probe['parent'], probe['prior_stages'])
    observed_keys = ('scope', 'parent_scope', 'maximum_work', 'constructors', 'caller_before', 'previous', 'navigation_prefix', 'build_after', 'caller_after')
    if probe['wire'] == OBSERVED_ATTEMPT_SCHEMA:
        observed_keys += ('source_display_prefix',)
    observed = fields(probe['observed'], observed_keys, 'closed authentic convergence-attempt observations')
    source_display_prefix = observed.get('source_display_prefix')
    strict_equal(probe['case'], parent['case'], 'same actual parent callback case')
    previous = verify_region_attempt(root, parent, source_display_prefix)
    check(probe['source_canonical_jcs'] == parent['source_canonical_jcs'], 'same complete source JCS input bytes')
    closed_structure(source)
    check(canonical(source['wire']) == probe['source_canonical_jcs'], 'same exact authentic original package JCS')
    strict_equal(source['wire'], parent['source_package'], 'unchanged source package and text buffers')
    check(probe['limits_canonical_jcs'] == parent['limits_canonical_jcs'] and probe['limits_fingerprint'] == parent['limits_fingerprint'] == sha(probe['limits_canonical_jcs'].encode()), 'same admitted inclusive resource limits')
    strict_equal(source['display'], list(bytes.fromhex(parent['display_fingerprint'])), 'same actual Column display including running regions')
    check(observed['scope'] == 'convergence_attempt' and observed['parent_scope'] == 'convergence_attempt', 'actual complete convergence attempt scope')
    strict_equal(observed['maximum_work'], parent['observed']['maximum_work'], 'same accepted command Work ceiling')
    ctors = observed['constructors']
    check(type(ctors) is list and len(ctors) == 14, 'all fourteen genuine constructor snapshots')
    strict_equal(ctors[:13], parent['downstream']['constructors'], 'unchanged thirteen genuine Navigation ancestor constructors')
    strict_equal(observed['caller_before'], observed['previous'], 'genuine previous whole caller quartet')
    strict_equal(observed['previous'], parent['downstream']['caller_before'], 'same whole command prior across typed phase bridges')
    strict_equal(observed['navigation_prefix'], probe['navigation']['charges'], 'genuine completed Navigation callback before constructor14')
    strict_equal(previous['navigation_prefix'], probe['navigation']['charges'], 'same authenticated parent Navigation quartet')
    strict_equal(probe['navigation'], prior['navigation'], 'same navigation result identity facts')
    fields(prior, ('syntax_flow_fingerprint', 'scopes', 'marked', 'structure', 'relations', 'navigation'), 'complete genuine canonical chain linkage')
    for key, earlier in (('scopes', parent['marked_scopes']), ('marked', dict(fingerprint=probe['marked']['fingerprint'], charges=parent['downstream']['content'])), ('structure', dict(charges=parent['downstream']['structure'], fingerprint=prior['structure']['fingerprint'])), ('relations', dict(charges=parent['downstream']['relations'], fingerprint=prior['relations']['fingerprint']))):
        fields(prior[key], ('fingerprint', 'charges'), 'actual canonical stage ' + key)
        strict_equal(prior[key]['fingerprint'], earlier['fingerprint'], 'same genuine ' + key + ' fingerprint')
        strict_equal(prior[key]['charges'], earlier['charges'], 'same genuine ' + key + ' quartet')
    fields(probe['engine'], ('name', 'version', 'rust_version', 'git_commit'), 'compiled issuer identity facts')
    check(probe['engine']['name'] == 'typaxis' and probe['engine']['version'] == '0.1.0', 'actual serialized engine name/version')
    check(all((probe['engine'][key] is None or type(probe['engine'][key]) is str for key in ('rust_version', 'git_commit'))), 'issuer-only runtime metadata type')
    size = fields(probe['size_of'], ('selected_page_master', 'object_range', 'page_reference', 'option_usize', 'usize'), 'actual assembly reservation sizes')
    strict_equal(size, dict(usize=8, object_range=16, option_usize=16, page_reference=20, selected_page_master=32), 'actual target ABI and readonly private reservation units')
    stage = fields(probe['assembly'], ('pdf', 'fingerprint', 'charges', 'first', 'next', 'objects', 'catalog', 'pages', 'candidates', 'references', 'labels_match'), 'full actual PDF assembly')
    earlier = source['relations']['navigation']['assembly']
    for key in ('catalog', 'pages', 'candidates', 'references', 'labels_match'):
        strict_equal(stage[key], earlier[key], 'same actual structural PDF field ' + key)
    verify_frames(core, dict(probe, parent=parent))
    old_nav.verify(source)
    expected_nodes = importlib.import_module('verify_book_v2_source_structure').source_nodes(source['wire']['document'])
    for node, expected in zip(source['nodes'], expected_nodes):
        strict_equal(node['language'], navigation.marked.canonical_language(expected['language']), 'exact canonical original source language')
    fields(probe['marked'], ('fingerprint', 'byte_length', 'next_object', 'anchor_font', 'anchor_bytes', 'anchor_objects', 'pages', 'groups', 'anchors'), 'complete actual source marked materialization')
    fields(probe['font_objects'], ('fingerprint', 'bytes', 'first', 'next', 'objects', 'resources'), 'all actual font objects')
    strict_equal({k: probe['font_objects'][k] for k in parent['font_objects'] if k != 'charges'}, {k: v for k, v in parent['font_objects'].items() if k != 'charges'}, 'same genuine font object chain')
    font_resources = []
    for index, font in enumerate(parent['cids']['fonts']):
        object_id = next((o['id'] for o in parent['font_objects']['objects'] if o['font'] == index and o['role'] == 'Type0'))
        font_resources.append(dict(index=index, instance=font['instance'], object=object_id))
    strict_equal(probe['font_objects']['resources'], font_resources, 'all original typed font resource names and object identities')
    fields(probe['image_objects'], ('fingerprint', 'bytes', 'first', 'next', 'objects', 'resources'), 'all actual image objects')
    image_parent = {k: v for k, v in parent['image_objects'].items() if k != 'charges'}
    image_parent['resources'] = [dict(index=i, object=o) for i, o in enumerate(image_parent['resources'])]
    strict_equal(probe['image_objects'], image_parent, 'same fully verified vector/raster image object chain')
    verify_marked_bytes(root, probe, regions)
    prior_hashes(probe)
    marked_pages = [artifact(root, p['bytes']) for p in probe['marked']['pages']]
    payload = dict(font_objects=artifact(root, probe['font_objects']['bytes']), image_objects=artifact(root, probe['image_objects']['bytes']), anchor_bytes=artifact(root, probe['marked']['anchor_bytes']), marked_pages=marked_pages)
    closed = dict(probe, _payload=payload)
    result = assembly_replay(core, root, closed)
    for key, value in (('first', result['plan']['first']), ('next', result['plan']['next']), ('catalog', result['plan']['first']), ('objects', result['objects']), ('pages', [result['plan']['page'] + 2 * i for i in range(len(probe['pages']))])):
        strict_equal(stage[key], value, 'independent complete PDF ' + key)
    check(artifact(root, stage['pdf']) == result['data'], 'every independently encoded final PDF byte')
    check(stage['fingerprint'] == result['fingerprint'], 'exact final source-bound PDF canonical fingerprint')
    result['budget'].matches4(stage)
    strict_equal(observed['build_after'], stage['charges'], 'genuine first Assembly build quartet')
    check(observed['caller_after'] is None, 'convergence attempt has no fictitious whole PDF-driver caller-after')
    check(earlier['pdf'] == stage['pdf']['file'] and earlier['hash'] == list(bytes.fromhex(stage['pdf']['sha256'])) and (earlier['bytes'] == len(result['data'])), 'same actual complete PDF artifact')
    core.verify_xref(result['data'])
    from pypdf import PdfReader
    core.verify(PdfReader(io.BytesIO(result['data']), strict=True), source)
    caps = json.loads(probe['limits_canonical_jcs'])['base']
    for quartet in [*ctors, *[prior[k]['charges'] for k in ('scopes', 'marked', 'structure', 'relations', 'navigation')], stage['charges']]:
        r, s, o, w = PDFBudget(quartet).triple4()
        check(r <= caps['max_fragments'] and s <= caps['max_spool_bytes'] and (o <= caps['max_output_bytes']) and (w <= observed['maximum_work']), 'all accepted upstream peaks and net final quartet fit inclusive caps')
    return dict(pages=len(probe['pages']), columns=sum((len(p['columns']) for p in probe['pages'])), running_regions=previous['regions'], source_nodes=len(source['nodes']), bindings=len(source['bindings']), annotations=len(source['relations']['navigation']['rectangles']), pdf_bytes=len(result['data']), pdf_objects=sum((v is not None for v in result['objects'])), replacement_credit=result['replacement_credit'], signed_output_delta=len(result['data']) - result['replacement_credit'])


OBS_FIELDS = ("charges", "candidate_passes", "balance_candidates", "width_passes",
              "width_refinements", "line_passes", "page_passes")
PASS_FIELDS = OBS_FIELDS[1:]
EVIDENCE_LIMITS = [
    "complete actual successful snapshots; early failure/retry prefixes are CLI controls",
    "local source/region companions and JSON artifacts are test-host work outside command counters",
    "TT fixture statistics do not establish a positive original CFF execution gate",
]
TOP = ("wire", "case", "fixture", "source_canonical_jcs", "source_census",
       "limits_canonical_jcs", "limits_fingerprint", "observed", "attempts",
       "early_failure_prefixes", "limits_of_evidence")
ATTEMPT = ("index", "values", "references", "source", "assembly_before_labels",
           "assembly_reservation_sizes", "after_labels", "decision",
           "display_fingerprint", "pdf_fingerprint", "pdf", "after_return")


def observation(value, caps, maximum):
    fields(value, OBS_FIELDS, "closed genuine parent observation")
    q = PDFBudget(value["charges"]).triple4()
    for key in PASS_FIELDS:
        integer(value[key], 64, False)
    check(q[0] <= caps["max_fragments"] and q[1] <= caps["max_spool_bytes"]
          and q[2] <= caps["max_output_bytes"] and q[3] <= maximum,
          "inclusive whole-command parent charge caps")
    check(value["balance_candidates"] <= caps["max_column_balance_candidates"],
          "cumulative balance candidate cap")
    for key in ("width_passes", "width_refinements", "page_passes"):
        check(value[key] <= caps["max_layout_passes"], "cumulative layout pass cap")
    check(value["line_passes"] <= caps["max_line_reshape_passes"],
          "cumulative shared line pass cap")
    return value


def retains(now, earlier, label, output_equal=False):
    check(all(now["charges"][i] >= earlier["charges"][i] for i in (0, 1, 3))
          and all(now[k] >= earlier[k] for k in PASS_FIELDS), label)
    if output_equal:
        check(now["charges"][2] == earlier["charges"][2], label + ": unchanged O")


def statistics(value, limits):
    fields(value, ("operations_used", "outline_segments_used", "cached_glyphs"),
           "actual persistent CFF session statistics")
    for item in value.values():
        integer(item, 64, False)
    ext = limits["extension"]
    check(value["operations_used"] <= ext["max_cff_charstring_operations"]
          and value["outline_segments_used"] <= ext["max_cff_outline_segments"],
          "inclusive aggregate CFF command session caps")
    return value


def authored_census(source):
    """Read the original AST, including unused definitions and every master."""
    owners, nodes = [], {}

    def walk(value):
        if type(value) is list:
            for item in value:
                walk(item)
        elif type(value) is dict:
            if "node_id" in value:
                owner = integer(value["node_id"], 32, False)
                check(owner not in nodes, "unique original authored owner")
                nodes[owner] = value
            if value.get("kind") == "reference" and value.get("format") == "page":
                owners.append(integer(value["node_id"], 32, False))
            for item in value.values():
                walk(item)
    walk(source["document"])
    check(len(owners) == len(set(owners)), "unique complete original Page owners")
    rows = []
    for ordinal, master in enumerate(source["page_masters"]["masters"]):
        for role in ("header_content", "footer_content"):
            content = master.get(role)
            if content is not None:
                check(type(content) is dict and type(content["blocks"]) is list,
                      "original running-region source grammar")
                integer(content["node_id"], 32, False)
                for block in content["blocks"]:
                    check(block["kind"] in ("paragraph", "heading"),
                          "exhaustive admitted running block grammar")
                    for inline in block["children"]:
                        check(inline["kind"] in ("text", "soft_break", "hard_break"),
                              "exhaustive running grammar contains no Page source")
            rows.append(dict(master=ordinal, master_id=master["master_id"],
                             role=role, present=content is not None, source=content,
                             page_reference_owners=[]))
    return dict(main_page_owners=sorted(owners), all_master_rows=rows), nodes


def map_fingerprint(values):
    encoded = b"typaxis.book-2-column-page-label-map/1\0"
    for owner, candidate in values:
        encoded += integer(owner, 32, False).to_bytes(4, "big")
        encoded += integer(candidate, 32, False).to_bytes(4, "big")
    return digest(encoded)


def physical_references(attempt, owners, nodes):
    leaf = attempt["pdf"]
    source = leaf["structural_probe"]
    nav = source["relations"]["navigation"]
    keys = {tuple(n["key"]): i for i, n in enumerate(source["nodes"])}
    result = []
    for owner, candidate in attempt["values"]:
        check(owner in owners and candidate > 0, "positive original Page candidate")
        matches = [v for v in nav["links"] if v["node"] == keys[owner, "ReferenceLink"]]
        check(len(matches) == 1, "one original Page ReferenceLink")
        link = matches[0]
        check(type(link["target"]) is list and link["target"][0] == "destination",
              "Page reference has actual internal destination")
        dest = nav["destinations"][link["target"][1]]
        strict_equal(dest["kind"], ["anchor", nodes[owner]["target"]],
                     "actual physical destination retains original Page target")
        position = dest["position"]
        target = None if position is None else integer(position[0], 32, False) + 1
        placed = link["first"] is not None
        check(not placed or target is not None, "painted Page link has placed target")
        result.append(dict(owner=owner, candidate=candidate, target=target, placed=placed))
    strict_equal(attempt["references"], result, "independent physical Page observations")
    for ref in attempt["references"]:
        fields(ref, ("owner", "candidate", "target", "placed"), "closed Page observation")
        check(type(ref["placed"]) is bool, "real Option placement boolean")
        if ref["target"] is not None:
            check(0 < integer(ref["target"], 32, False) <= len(leaf["pages"]),
                  "one-based actual physical target page")
    strict_equal(leaf["assembly"]["references"], result,
                 "same observations sealed by actual Assembly")
    strict_equal(leaf["assembly"]["candidates"], attempt["values"],
                 "same complete candidate map sealed by Assembly")
    matching = all(r["candidate"] == r["target"] if r["target"] is not None
                   else not r["placed"] for r in result)
    strict_equal(leaf["assembly"]["labels_match"], matching,
                 "actual Option semantics; unused-note None is not a fake page")
    return matching


def generated_labels(attempt):
    """Bind every actual selected Page cluster to candidate UTF8, including repeats.

    Graph cuts/glyphs are independently checked below by the projection kernels;
    this adds the missing source-generation relation rather than reshaping text.
    """
    values = dict(attempt["values"])
    slices = {}
    painted = set()
    base = attempt["pdf"]["parent"]["base_display"]
    for fragment in base["original_fragments"]:
        if fragment["line"] is None:
            continue
        for item in fragment["line"]["items"]:
            if item["kind"] != "text" or item["owner"] not in values:
                continue
            owner = item["owner"]
            source = item["source"]
            check(source["generated"] is True, "Page label has authentic generated source role")
            source_id, start, end = source["span"]
            check(source_id >= base["source_text_buffers"],
                  "Page label uses generated buffer namespace")
            raw = item["text"].encode()
            check(end - start == len(raw) and raw, "nonempty exact generated Page slice")
            key = (owner, source_id)
            positions = slices.setdefault(key, {})
            for offset, byte in enumerate(raw, start):
                check(offset not in positions or positions[offset] == byte,
                      "all repeated/variant source Page slices agree")
                positions[offset] = byte
            painted.add(owner)
    by_owner = Counter()
    for (owner, _), positions in slices.items():
        start, end = min(positions), max(positions) + 1
        check(set(positions) == set(range(start, end)), "complete original Page source slice")
        check(bytes(positions[i] for i in range(start, end)) == str(values[owner]).encode(),
              "actual generated decimal Page source matches current candidate")
        by_owner[owner] += 1
    for ref in attempt["references"]:
        check(not ref["placed"] or ref["owner"] in painted,
              "painted Page reference has actual original generated clusters")
    return len(slices)


def verify_outer(root, probe):
    """Validate all outer provenance/state links before expensive leaf replay."""
    fields(probe, TOP, "closed actual convergence wire")
    check(probe["wire"] in (SCHEMA, OBSERVED_SCHEMA) and type(probe["fixture"]) is str
          and probe["fixture"] and "/" not in probe["fixture"]
          and probe["case"] == "page-label-" + probe["fixture"],
          "actual convergence schema/case identity")
    observed_source = probe['wire'] == OBSERVED_SCHEMA
    strict_equal(probe["limits_of_evidence"], EVIDENCE_LIMITS + ([SOURCE_PREFIX_LIMIT] if observed_source else []), "explicit versioned proof limits")
    check(type(probe["source_canonical_jcs"]) is str, "original source JCS string")
    source = json.loads(probe["source_canonical_jcs"])
    strict_equal(canonical(source), probe["source_canonical_jcs"], "canonical original source bytes")
    census, nodes = authored_census(source)
    strict_equal(probe["source_census"], census, "full original Page/all-master census")
    owners = census["main_page_owners"]
    limits = json.loads(probe["limits_canonical_jcs"])
    strict_equal(canonical(limits), probe["limits_canonical_jcs"], "canonical complete limits bytes")
    strict_equal(probe["limits_fingerprint"], sha(probe["limits_canonical_jcs"].encode()),
                 "effective limit fingerprint")
    caps = limits["base"]
    obs = fields(probe["observed"], ("scope", "maximum_work", "caller_before", "caller_after",
                  "final_callback_count", "cff_after"), "actual outer caller observations")
    check(obs["scope"] == "converged_column_pdf_driver", "actual new public convergence scope")
    maximum = integer(obs["maximum_work"], 64, False)
    check(maximum > 0, "positive command work ceiling")
    observation(obs["caller_before"], caps, maximum)
    observation(obs["caller_after"], caps, maximum)
    check(type(obs["final_callback_count"]) is int and obs["final_callback_count"] == 1,
          "exactly one final immutable delivery")
    strict_equal(probe["early_failure_prefixes"], [], "only complete successful attempt evidence")
    attempts = probe["attempts"]
    check(type(attempts) is list and attempts and len(attempts) <= caps["max_layout_passes"] // 2,
          "nonempty bounded complete convergence sequence")
    check(bool(owners) or len(attempts) == 1, "no-Page source executes one actual PDF")
    values = [[o, 1] for o in owners]
    anchor, power, distance, previous_matching = map_fingerprint(values), 1, 0, None
    previous_after = obs["caller_before"]
    previous_cff = {"operations_used": 0, "outline_segments_used": 0, "cached_glyphs": 0}
    generated_count = 0
    census_before = census_after = None
    for index, attempt in enumerate(attempts):
        fields(attempt, ATTEMPT, "closed complete actual attempt")
        check(type(attempt["index"]) is int and attempt["index"] == index, "dense actual attempt order")
        strict_equal(attempt["values"], values, "exact paid feedback candidate sequence")
        for pair in attempt["values"]:
            check(type(pair) is list and len(pair) == 2, "ordered owner/candidate pair")
            integer(pair[0], 32, False)
            check(0 < integer(pair[1], 32, False), "positive candidate page")
        fields(attempt["source"], ("copied", "census_before", "census_after", "before", "after"),
               "actual source/copy issuer-prefix witnesses")
        src = attempt["source"]
        check(type(src["copied"]) is bool and src["copied"] == bool(owners),
              "source copy only when original Page census is nonempty")
        for name in ("census_before", "census_after", "before", "after"):
            observation(src[name], caps, maximum)
        if index == 0:
            census_before, census_after = src["census_before"], src["census_after"]
            retains(census_before, obs["caller_before"], "original preparation retains caller prefix", True)
            retains(census_after, census_before, "complete census retains preparation prefix", True)
            check(census_after["charges"][0] > census_before["charges"][0]
                  and census_after["charges"][3] > census_before["charges"][3],
                  "actual charged census record and traversal witness")
            expected = copy.deepcopy(census_after)
            expected["charges"][0] += 2 * len(owners)
            expected["charges"][1] += 16 * len(owners)
            expected["charges"][3] += 2 * len(owners) + (3 if owners else 1)
            strict_equal(src["before"], expected, "two counted candidate buffers and initial map fold")
        else:
            strict_equal(src["census_before"], census_before, "same original source preparation witness")
            strict_equal(src["census_after"], census_after, "same once-only complete census witness")
            strict_equal(src["before"], previous_after, "next candidate starts at genuine after-return owner")
        check(caps["max_layout_passes"] - src["before"]["page_passes"] >= 2,
              "inclusive whole-loop remaining two-pass admission")
        retains(src["after"], src["before"], "source candidate retains all accepted prefixes", True)
        strict_equal({k: src["after"][k] for k in PASS_FIELDS},
                     {k: src["before"][k] for k in PASS_FIELDS}, "source copy does not begin layout passes")
        if not owners:
            strict_equal(src["after"], src["before"], "no-Page source uses original without copy")
        leaf = attempt["pdf"]
        check(leaf["wire"] == (OBSERVED_ATTEMPT_SCHEMA if observed_source else ATTEMPT_SCHEMA), "matching versioned complete attempt family")
        strict_equal(leaf["case"], probe["case"] + "-attempt-" + str(index),
                     "same actual capture fixture and dense attempt identity")
        check(leaf["observed"]["scope"] == "convergence_attempt"
              and leaf["observed"]["parent_scope"] == "convergence_attempt"
              and leaf["parent"]["observed"]["scope"] == "convergence_attempt",
              "unaltered explicit private convergence-attempt lineage")
        check(leaf["observed"]["caller_after"] is None,
              "attempt does not invent a whole public PDF callback result")
        strict_equal(leaf["source_canonical_jcs"], probe["source_canonical_jcs"], "all candidates retain original JCS")
        strict_equal(leaf["limits_canonical_jcs"], probe["limits_canonical_jcs"], "same complete candidate caps")
        strict_equal(leaf["limits_fingerprint"], probe["limits_fingerprint"], "same candidate limit identity")
        strict_equal(leaf["observed"]["maximum_work"], maximum, "same candidate work cap")
        strict_equal(leaf["observed"]["caller_before"], previous_after["charges"], "actual previous whole caller quartet")
        strict_equal(leaf["observed"]["previous"], previous_after["charges"], "authentic previous typed PDF phase")
        observed_keys = ('scope', 'parent_scope', 'maximum_work', 'constructors', 'caller_before', 'previous', 'navigation_prefix', 'build_after', 'caller_after')
        if observed_source:
            observed_keys += ('source_display_prefix',)
        fields(leaf['observed'], observed_keys, 'closed versioned resource-entry observation')
        if observed_source:
            ready = observation(leaf['observed']['source_display_prefix'], caps, maximum)
            retains(ready, src['after'], 'observed physical/display work retains candidate Source', True)
            retains(attempt['after_labels'], ready, 'completed PDF retains observed Source/display prefix')
            strict_equal({k: ready[k] for k in PASS_FIELDS}, {k: attempt['after_labels'][k] for k in PASS_FIELDS}, 'downstream PDF does not start layout passes')
            r, sp, out, work = ready['charges']
            strict_equal(leaf['observed']['constructors'][0], [r + 1, sp, out, work], 'exact observed prefix before first resource constructor')
        strict_equal(attempt["assembly_reservation_sizes"], [32, 16, 20, 16], "actual Assembly private reservation ABI")
        strict_equal(attempt["assembly_before_labels"], leaf["assembly"]["charges"], "actual Assembly before Page feedback")
        strict_equal(attempt["assembly_before_labels"], leaf["observed"]["build_after"], "same genuine Assembly result quartet")
        after = observation(attempt["after_labels"], caps, maximum)
        before_labels = copy.deepcopy(after)
        before_labels["charges"] = list(attempt["assembly_before_labels"])
        retains(before_labels, src["after"], "complete physical/PDF attempt retains source prefix")
        parent = leaf["parent"]
        for name, slot in (("records", 0), ("spool", 1), ("output", 2), ("work", 3)):
            strict_equal(parent["observed"][name], leaf["observed"]["navigation_prefix"][slot],
                         "nested region observation remains Navigation, not Assembly/feedback")
        strict_equal(parent["observed"]["line_passes"], after["line_passes"], "genuine shared line-pass owner")
        matching = physical_references(attempt, owners, nodes)
        generated_count += generated_labels(attempt)
        strict_equal(attempt["display_fingerprint"], parent["display_fingerprint"], "actual complete display fingerprint")
        strict_equal(attempt["pdf_fingerprint"], leaf["assembly"]["fingerprint"], "actual complete PDF fingerprint")
        fingerprint(attempt["display_fingerprint"])
        fingerprint(attempt["pdf_fingerprint"])
        state = (attempt["display_fingerprint"], attempt["pdf_fingerprint"])
        ready = matching and (not owners or previous_matching == state)
        strict_equal(attempt["decision"], "ready" if ready else "retry", "independent two-complete-state decision")
        check(ready == (index == len(attempts) - 1), "only final complete attempt delivers")
        steps = len(owners) + 2
        if not ready:
            previous_matching = state if matching else None
            next_values = [[r["owner"], r["target"] if r["target"] is not None else r["candidate"]]
                           for r in attempt["references"]]
            if not matching:
                steps += len(owners) + 2
                next_hash = map_fingerprint(next_values)
                check(next_hash != anchor, "bounded Brent cycle rule: no repeated mismatch anchor")
                distance += 1
                if distance == power:
                    anchor, power, distance = next_hash, power * 2, 0
            values = next_values
        expected_quartet = list(attempt["assembly_before_labels"])
        expected_quartet[3] += steps
        strict_equal(after["charges"], expected_quartet, "exact separately paid feedback work after Assembly")
        returned = fields(attempt["after_return"], ("observation", "cff", "outcome_is_ok"),
                          "actual post-loan owner observation")
        strict_equal(returned["observation"], after, "actual post-return parent retains every accepted feedback prefix")
        check(returned["outcome_is_ok"] is True, "actual completed attempt Result")
        current_cff = statistics(returned["cff"], limits)
        check(all(current_cff[k] >= previous_cff[k] for k in current_cff),
              "one persistent nonreset CFF command session across attempts")
        previous_cff, previous_after = current_cff, returned["observation"]
    strict_equal(obs["caller_after"], previous_after, "final caller equals genuine post-return owner")
    strict_equal(statistics(obs["cff_after"], limits), previous_cff, "same final persistent CFF statistics")
    if all(font["kind"] == "TrueType" for attempt in attempts
           for font in attempt["pdf"]["parent"]["closures"]["fonts"]):
        strict_equal(previous_cff, {"operations_used": 0, "outline_segments_used": 0,
                                   "cached_glyphs": 0}, "TT fixtures make no positive CFF execution claim")
    retains(obs["caller_after"], obs["caller_before"], "complete outer command retains accepted caller prefix")
    if owners:
        check(len(attempts) >= 2, "nonempty Page source requires two complete matching states")
        previous, last = attempts[-2:]
        for key in ("display_fingerprint", "pdf_fingerprint"):
            strict_equal(previous[key], last[key], "two consecutive complete matching fingerprints")
        strict_equal(previous["pdf"]["assembly"]["pdf"], last["pdf"]["assembly"]["pdf"],
                     "two final matching actual PDF byte descriptors")
        check(artifact(root, previous["pdf"]["assembly"]["pdf"]) ==
              artifact(root, last["pdf"]["assembly"]["pdf"]), "two matching complete actual PDF payloads")
    return dict(attempts=len(attempts), page_owners=len(owners), generated_page_sources=generated_count,
                master_rows=len(census["all_master_rows"]), final_charges=obs["caller_after"]["charges"])


def verify(root, probe):
    summary = verify_outer(root, probe)
    totals = Counter()
    for attempt in probe["attempts"]:
        totals.update(verify_attempt_pdf(root, attempt["pdf"]))
    summary["complete_attempt_totals"] = dict(totals)
    return summary


def replace_path(value, path, transform):
    """Persistent test-host copy; unmodified actual payloads are never rewritten."""
    if not path:
        return transform(value)
    part, *rest = path
    result = value.copy()
    result[part] = replace_path(value[part], rest, transform)
    return result


REJECTION_ERRORS = (AssertionError, KeyError, IndexError, ValueError, TypeError,
                    UnicodeError, StopIteration, OverflowError, struct.error)
CONTROL_CONTEXT = {}
CONTROL_RECORDS = []


def reject(call, label, expected=None):
    try:
        call()
    except REJECTION_ERRORS as error:
        if expected is not None:
            check(expected in str(error), "control reached intended independent predicate: " + label
                  + "; actual=" + str(error))
        CONTROL_RECORDS.append(dict(CONTROL_CONTEXT, control=label,
                                    rejection_type=type(error).__name__,
                                    rejection_message=str(error)))
        return
    raise AssertionError("accepted altered convergence control: " + label)


def outer_controls(root, probe):
    """Every new closed-schema field plus actual state/semantic consistency rules."""
    count = 0
    paths = [(), ("observed",), ("source_census",)]
    for index, attempt in enumerate(probe["attempts"]):
        prefix = ("attempts", index)
        paths.extend([prefix, (*prefix, "source"), (*prefix, "after_return"),
                      (*prefix, "after_return", "cff"), (*prefix, "after_labels"),
                      (*prefix, "after_return", "observation")])
        paths.extend((*prefix, "source", k) for k in
                     ("census_before", "census_after", "before", "after"))
        paths.extend((*prefix, "references", i) for i in range(len(attempt["references"])))
        if probe['wire'] == OBSERVED_SCHEMA:
            paths.extend([(*prefix, 'pdf', 'observed'), (*prefix, 'pdf', 'observed', 'source_display_prefix')])
    paths.extend(("observed", key) for key in ("caller_before", "caller_after", "cff_after"))
    for path in paths:
        value = probe
        for part in path:
            value = value[part]
        for key in value:
            bad = replace_path(probe, path, lambda v, k=key: {n: x for n, x in v.items() if n != k})
            reject(lambda: verify_outer(root, bad), "missing " + repr((*path, key)))
            count += 1
        bad = replace_path(probe, path, lambda v: dict(v, unissued_extra_field=0))
        reject(lambda: verify_outer(root, bad), "extra " + repr(path))
        count += 1
    controls = []
    if probe['wire'] == OBSERVED_SCHEMA:
        controls.append((('wire',), lambda _: SCHEMA))
        for i in range(len(probe['attempts'])):
            for axis in range(4):
                controls.append((('attempts', i, 'pdf', 'observed', 'source_display_prefix', 'charges', axis), lambda v: v + 1))
            for key in PASS_FIELDS:
                controls.append((('attempts', i, 'pdf', 'observed', 'source_display_prefix', key), lambda v: v + 1))
    controls += [
        (("wire",), lambda v: "typaxis.book-2-column-pdf-probe/1"),
        (("case",), lambda v: v + "-changed"),
        (("fixture",), lambda v: v + "-changed"),
        (("observed", "scope"), lambda v: "pdf_driver"),
        (("observed", "final_callback_count"), lambda v: 2),
        (("observed", "final_callback_count"), lambda v: True),
        (("observed", "maximum_work"), lambda v: 0),
        (("early_failure_prefixes",), lambda v: [{"unreplayed": True}]),
        (("attempts",), lambda v: []),
        (("attempts",), lambda v: v + [v[-1]]),
        (("source_census", "main_page_owners"), lambda v: v + [0]),
        (("source_census", "all_master_rows"), lambda v: v[:-1]),
        (("limits_fingerprint",), lambda v: "00" * 32),
    ]
    if len(probe["attempts"]) > 1:
        controls += [(("attempts",), lambda v: v[1:]),
                     (("attempts",), lambda v: [*v[:-2], v[-1], v[-2]]),
                     (("attempts",), lambda v: [*v[:-1]])]
    for index, attempt in enumerate(probe["attempts"]):
        prefix = ("attempts", index)
        controls.extend([
            ((*prefix, "index"), lambda v: v + 1),
            ((*prefix, "decision"), lambda v: "ready" if v == "retry" else "retry"),
            ((*prefix, "display_fingerprint"), lambda v: "00" * 32),
            ((*prefix, "pdf_fingerprint"), lambda v: "00" * 32),
            ((*prefix, "assembly_before_labels", 0), lambda v: v + 1),
            ((*prefix, "assembly_reservation_sizes", 0), lambda v: v + 1),
            ((*prefix, "source", "copied"), lambda v: not v),
            ((*prefix, "after_return", "outcome_is_ok"), lambda v: False),
            ((*prefix, "pdf", "observed", "scope"), lambda v: "pdf_driver"),
            ((*prefix, "pdf", "observed", "caller_after"), lambda v: [0, 0, 0, 0]),
            ((*prefix, "pdf", "observed", "caller_before", 0), lambda v: v + 1),
            ((*prefix, "pdf", "observed", "previous", 3), lambda v: v + 1),
        ])
        for phase in ("census_before", "census_after", "before", "after"):
            prior = attempt["source"][phase]
            # Source/census allocation magnitudes are issuer facts. These are
            # actual monotonic/floor/identity controls, not a claimed replay of
            # private source preparation reservations.
            for axis in (0, 1, 3):
                if phase == "census_before":
                    replacement = attempt["source"]["census_after"]["charges"][axis] + 1
                elif phase == "after":
                    replacement = attempt["source"]["before"]["charges"][axis] - 1
                else:
                    replacement = prior["charges"][axis] + 1
                controls.append(((*prefix, "source", phase, "charges", axis),
                                 lambda v, r=replacement: r))
            for name in PASS_FIELDS:
                controls.append(((*prefix, "source", phase, name), lambda v: v + 1))
        controls.extend([
            ((*prefix, "values"), lambda v: [] if v else [[0, 1]]),
            ((*prefix, "values"), lambda v: v + ([v[0]] if v else [[0, 1]])),
            ((*prefix, "after_return", "cff", "operations_used"), lambda v: v + 1),
            ((*prefix, "after_return", "cff", "outline_segments_used"), lambda v: v + 1),
            ((*prefix, "after_return", "cff", "cached_glyphs"), lambda v: v + 1),
        ])
        if attempt["values"]:
            controls.append(((*prefix, "values"), lambda v: v[:-1]))
        if len(attempt["values"]) > 1:
            controls.append(((*prefix, "values"), lambda v: list(reversed(v))))
        for phase in ("after_labels",):
            for axis in range(4):
                controls.append(((*prefix, phase, "charges", axis), lambda v: v + 1))
        for axis in range(4):
            controls.append(((*prefix, "after_return", "observation", "charges", axis), lambda v: v + 1))
        for name in PASS_FIELDS:
            controls.append(((*prefix, "after_return", "observation", name), lambda v: v + 1))
        for i, ref in enumerate(attempt["references"]):
            controls.extend([
                ((*prefix, "values", i, 0), lambda v: (v + 1) % (1 << 32)),
                ((*prefix, "values", i, 1), lambda v: v + 1),
                ((*prefix, "references", i, "target"), lambda v: 1 if v is None else None),
                ((*prefix, "references", i, "placed"), lambda v: not v),
                ((*prefix, "references", i, "candidate"), lambda v: v + 1),
                ((*prefix, "references", i, "owner"), lambda v: v + 1),
            ])
    for path, transform in controls:
        bad = replace_path(probe, path, transform)
        reject(lambda: verify_outer(root, bad), "changed " + repr(path))
        count += 1
    return count


def nested_controls(root, attempt):
    """Bounded real leaf checks. Each calls the full new-scope adapter."""
    count = 0
    leaf = attempt["pdf"]
    controls = [
        (("wire",), lambda v: "typaxis.book-2-column-pdf-probe/1"),
        (("observed", "scope"), lambda v: "pdf_driver"),
        (("observed", "parent_scope"), lambda v: "navigation_driver"),
        (("observed", "caller_after"), lambda v: [0, 0, 0, 0]),
        (("observed", "constructors", 13, 0), lambda v: v + 1),
        (("assembly", "fingerprint"), lambda v: "00" * 32),
        (("assembly", "charges", 3), lambda v: v + 1),
        (("assembly", "pdf", "sha256"), lambda v: "00" * 32),
        (("parent", "observed", "scope"), lambda v: "navigation_driver"),
        (("parent", "display_fingerprint"), lambda v: "00" * 32),
        (("parent", "downstream", "constructors", 0, 0), lambda v: v + 1),
        (("parent", "text_commands", "fingerprint"), lambda v: "00" * 32),
        (("parent", "font_objects", "fingerprint"), lambda v: "00" * 32),
    ]
    for path, transform in controls:
        bad = replace_path(leaf, path, transform)
        reject(lambda: verify_attempt_pdf(root, bad), "nested " + repr(path))
        count += 1
    base = leaf["parent"]["base_display"]
    candidates = [(i, j) for i, fragment in enumerate(base["original_fragments"])
                  if fragment["line"] is not None
                  for j, item in enumerate(fragment["line"]["items"])
                  if item["kind"] == "text"]
    if candidates:
        i, j = candidates[0]
        prefix = ("parent", "base_display", "original_fragments", i, "line", "items", j)
        source_controls = [((*prefix, "text"), lambda v: v + "9"),
                           ((*prefix, "glyphs", 0, "x"), lambda v: v + 1)]
        for path, transform in source_controls:
            bad = replace_path(leaf, path, transform)
            reject(lambda: verify_attempt_pdf(root, bad), "actual original cluster " + repr(path))
            count += 1
    return count


def label_controls(root, probe):
    count = 0
    for index, attempt in enumerate(probe["attempts"]):
        owners = {o for o, _ in attempt["values"]}
        base = attempt["pdf"]["parent"]["base_display"]
        selected = [(i, j) for i, f in enumerate(base["original_fragments"]) if f["line"] is not None
                    for j, item in enumerate(f["line"]["items"])
                    if item["kind"] == "text" and item["owner"] in owners]
        for i, j in selected[:2]:
            prefix = ("attempts", index, "pdf", "parent", "base_display",
                      "original_fragments", i, "line", "items", j)
            for suffix, transform in ((('text',), lambda v: "8" if v != "8" else "9"),
                                      (('source', 'generated'), lambda v: False),
                                      (('source', 'span', 2), lambda v: v + 1)):
                bad = replace_path(probe, (*prefix, *suffix), transform)
                reject(lambda: verify_outer(root, bad), "actual Page generated source " + repr((*prefix, *suffix)))
                count += 1
    return count


def artifact_controls(root, probe):
    """A rehashed real PDF byte mutation must fail exact encoding, not file SHA."""
    control_root = Path(__file__).resolve().parent.parent / "workspace/target/vmb-design/20261015/column-page-labels/independent-convergence/artifact-controls"
    control_root.mkdir(parents=True, exist_ok=True)
    leaf = probe["pdf"]
    descriptor = leaf["assembly"]["pdf"]
    original = artifact(root, descriptor)
    check(original.startswith(b"%PDF-"), "actual PDF control payload")
    changed = bytes([original[0] ^ 1]) + original[1:]
    with tempfile.TemporaryDirectory(prefix="actual-", dir=control_root) as temporary:
        target = Path(temporary)
        for path in root.iterdir():
            if path.is_file():
                shutil.copy2(path, target / path.name)
        name = "changed-pdf-" + sha(changed) + ".pdf"
        (target / name).write_bytes(changed)
        bad = copy.deepcopy(leaf)
        bad["assembly"]["pdf"] = dict(file=name, sha256=sha(changed), length=len(changed))
        earlier = bad["structural_probe"]["relations"]["navigation"]["assembly"]
        earlier.update(pdf=name, hash=list(digest(changed)), bytes=len(changed))
        # The artifact descriptor and both structural raw-byte witnesses are
        # internally consistent. The unchanged kernel must reject the bytes.
        strict_equal(artifact(target, bad["assembly"]["pdf"]), changed,
                     "rehashed byte control passes artifact integrity itself")
        bad["assembly"]["fingerprint"] = pdf.reported_assembly_fingerprint(bad, changed)
        reject(lambda: verify_attempt_pdf(target, bad), "SHA-and-fingerprint-resealed actual PDF bytes",
               "every independently encoded final PDF byte")
        count = 1
        if leaf["parent"]["programs"]["fonts"]:
            bad = copy.deepcopy(leaf)
            program = bad["parent"]["programs"]["fonts"][0]
            data = bytearray(artifact(root, program["program"]))
            entries = [struct.unpack(">4sIII", data[at:at + 16])
                       for at in range(12, 12 + 16 * int.from_bytes(data[4:6], "big"), 16)]
            hmtx = next(e for e in entries if e[0] == b"hmtx")
            check(hmtx[3] >= 2, "actual source-owned subset advance control")
            data[hmtx[2] + 1] ^= 1
            head = next(e[2] for e in entries if e[0] == b"head")
            data[head + 8:head + 12] = bytes(4)
            for index, (_, _, offset, length) in enumerate(entries):
                data[16 + 16 * index:20 + 16 * index] = resources.checksum(
                    bytes(data[offset:offset + length])).to_bytes(4, "big")
            data[head + 8:head + 12] = ((0xB1B0AFBA - resources.checksum(bytes(data)))
                                      & 0xFFFFFFFF).to_bytes(4, "big")
            check(resources.checksum(bytes(data)) == 0xB1B0AFBA,
                  "actual changed subset preserves whole SFNT checksum")
            name = "changed-subset-" + sha(data) + ".bin"
            (target / name).write_bytes(data)
            program["program"] = dict(file=name, sha256=sha(data), length=len(data))
            program["sha256"] = sha(data)
            strict_equal(artifact(target, program["program"]), bytes(data),
                         "changed resource passes artifact descriptor integrity")
            reject(lambda: verify_attempt_pdf(target, bad),
                   "checksum-valid subset original hmtx bytes", "exact original hmtx bytes")
            count += 1
    return count


def source_snapshot():
    tools = Path(__file__).resolve().parent
    return {str(p.relative_to(tools.parent)): sha(p.read_bytes())
            for p in sorted(tools.glob("*.py")) if p.name != Path(__file__).name}


def file_inventory(base):
    base = base.resolve()
    check(base.is_dir(), "explicit existing input root for file epoch")
    result = {}
    for path in sorted(base.rglob("*")):
        check(not path.is_symlink(), "input epoch does not follow a mutable symlink")
        if path.is_file():
            result[str(path.relative_to(base))] = dict(
                sha256=sha(path.read_bytes()), bytes=path.stat().st_size)
    return result


def file_epoch(directories, pdf_environment):
    """Bind loaded-source dependencies and every input file, not just JSON rows."""
    tools = Path(__file__).resolve().parent
    return dict(
        tools={str(p.relative_to(tools.parent)): dict(sha256=sha(p.read_bytes()),
                                                    bytes=p.stat().st_size)
               for p in sorted(tools.glob("*.py"))},
        inputs=[dict(root=str(Path(directory).resolve()),
                     files=file_inventory(Path(directory))) for directory in directories],
        pypdf=pdf_environment,
    )


def pdf_static_environment(site):
    environment = pdf.configure_pdf_site(site)
    # The unchanged loader already excludes __pycache__. Exclude legacy
    # standalone bytecode too, so ordinary lazy imports are not file mutations.
    environment["files"] = {name: value for name, value in environment["files"].items()
                            if Path(name).suffix not in (".pyc", ".pyo")}
    environment["guard_policy"] = "all static package files; excludes __pycache__, .pyc and .pyo"
    return environment


def protected_output_roots(directories, environment):
    return [Path(__file__).resolve().parent, Path(environment["base"]).resolve(),
            *[Path(d).resolve() for d in directories]]


def load_inputs(directories):
    entries, cases = [], set()
    for directory in directories:
        directory = Path(directory).resolve()
        check(directory.is_dir(), "explicit convergence probe directory")
        paths = sorted(directory.glob("*.json"))
        check(paths, "actual captured convergence JSON inputs")
        for path in paths:
            raw = path.read_bytes()
            probe = json.loads(raw)
            check(probe["wire"] in (SCHEMA, OBSERVED_SCHEMA), "actual versioned convergence input family")
            check(path.name == probe["case"] + "-" + sha(raw) + ".json",
                  "raw original capture filename binds every JSON byte")
            check(probe["case"] not in cases, "unique actual convergence fixture")
            cases.add(probe["case"])
            entries.append((path, raw, probe))
    check(entries, "actual convergence inputs required")
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directories", nargs="+", type=Path,
                        help="Explicit actual column-page-label-convergence directory.")
    parser.add_argument("--self-test", action="store_true", help="Run bounded actual-input rejection controls.")
    parser.add_argument("--pdf-site", type=Path, help="Existing pinned pypdf6.10.0 site; no installation.")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    before, self_sha = source_snapshot(), sha(Path(__file__).read_bytes())
    strict_equal(before, ORIGINAL_TOOLS, "all preexisting Python tools byte-immutable")
    environment = pdf_static_environment(args.pdf_site)
    protected_roots = protected_output_roots(args.directories, environment)
    if args.output:
        check(all(not args.output.resolve().is_relative_to(root) for root in protected_roots),
              "report destination must exclude all protected input/tool/pypdf roots")
    epoch_before = file_epoch(args.directories, environment)
    entries = load_inputs(args.directories)
    totals, controls, rows = Counter(), Counter(), []
    for path, raw, probe in entries:
        result = verify(path.parent, probe)
        totals["fixtures"] += 1
        totals["attempts"] += result["attempts"]
        totals["page_owners"] += result["page_owners"]
        totals.update(result["complete_attempt_totals"])
        if args.self_test:
            CONTROL_CONTEXT.clear()
            CONTROL_CONTEXT.update(case=probe["case"], category="outer")
            controls["outer_rejections"] += outer_controls(path.parent, probe)
            CONTROL_CONTEXT["category"] = "generated_source"
            controls["generated_label_rejections"] += label_controls(path.parent, probe)
            for attempt in probe["attempts"]:
                CONTROL_CONTEXT.update(category="full_nested_adapter", attempt=attempt["index"])
                controls["nested_rejections"] += nested_controls(path.parent, attempt)
                CONTROL_CONTEXT["category"] = "rehashed_payload"
                controls["rehashed_payload_rejections"] += artifact_controls(path.parent, attempt)
            CONTROL_CONTEXT.pop("attempt", None)
        rows.append(dict(path=str(path), sha256=sha(raw), case=probe["case"], **result))
        print("PASS " + probe["case"] + " " + str(result["attempts"]) + " actual attempts", file=sys.stderr)
    strict_equal(source_snapshot(), before, "all historical Python tools unchanged after complete verification")
    strict_equal(sha(Path(__file__).read_bytes()), self_sha, "new checker source stable across run")
    epoch_after = file_epoch(args.directories, pdf_static_environment(args.pdf_site))
    strict_equal(epoch_after, epoch_before,
                 "all input/tool/pypdf files unchanged, with no additions or removals")
    result = dict(wire="typaxis.column-convergence-independent-result/1", status="pass",
                  checker_sha256=self_sha, old_tools=before, environment=environment,
                  report_destination=None if args.output is None else str(args.output.resolve()),
                  protected_output_roots=[str(root) for root in protected_roots],
                  file_epoch=dict(begin=epoch_before, end=epoch_after,
                      begin_sha256=sha(canonical(epoch_before).encode()),
                      end_sha256=sha(canonical(epoch_after).encode())),
                  totals=dict(totals), negative_tests=dict(controls), inputs=rows,
                  control_inventory=dict(count=len(CONTROL_RECORDS),
                      ordered_descriptors_sha256=sha(canonical(CONTROL_RECORDS).encode()),
                      descriptors=CONTROL_RECORDS),
                  evidence_limits=EVIDENCE_LIMITS + [
                      "Source preparation/copy requested bytes, original shaping and earlier physical pass counters are linked issuer witnesses, not replayed allocator or shaping traces.",
                      "Normal runs fully replay each complete nested command/PDF payload. Bounded selftests report only actually rejected controls.",
                      "No historical public callback is relabeled. No failed-attempt receipt, whole-book, physical heap, PDF/UA or original CFF positivity claim."])
    encoded = json.dumps(result, sort_keys=True, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded)
    print(encoded, end="")


ORIGINAL_TOOLS = {
    "tools/generate_cff_subr_diagnostic_fixture.py": "aa7875cedb916fc2130041a8f47dd3fdfa84071a1c5adc0ca6a93ce647101eb4",
    "tools/generate_production_list_fonts.py": "6765719ff9afd45116fe395db0921e02929c2ae697a04b6755a0bac411bea3c6",
    "tools/generate_unicode_linebreak.py": "bb4ab6211f3e3fc9efabe85448455ce3665b1ebc5c1af4e19e7dc0febd932dc4",
    "tools/inspect_harano_cff_outlines.py": "52b4c639df0505f2e93679b44642693acf02ac970ccb99b4cceb6e64bc3eebf3",
    "tools/inspect_harano_cff_program.py": "058d891e9b276fe9b07d2421af054e96b228f64593e1ada41ed4c0e260130ab2",
    "tools/inspect_harano_cff_tables.py": "3bf01fe17558f0271f6028fa9ba5a5609644cdd3a703c183738aae9c74fdc4d1",
    "tools/matterhorn_protocol.py": "06515f9881d41f2b6ae42ad402e1897e1f9a894b223933a1b660947a3acd128a",
    "tools/prepare_vmb_distinct_probe.py": "1ceccb6b2aeec8136834689fd9780ce0e12ccd751a8ab35e7f9a7a1da08f23a2",
    "tools/release.py": "e857e4d00839a956da061d80dfceb67925d5b75ea802121f4c6427145f8607b3",
    "tools/test_machine_profile.py": "a5344830814b110020999a8523e3523f9293986a03923d9b6cba36e5a0375e34",
    "tools/test_pdf_differential.py": "25def7d00e1d034215f7c809deb8c1e7ff96bc8217b3a39b72784cca6172f8aa",
    "tools/test_pdf_structure.py": "e07ae7e1099ae639a5cba940e7e3699ec14b42afecac5d77bb321768e2dc14fb",
    "tools/test_precomposed_vector.py": "b76e6afe621f4f611c57bf9a9f00a4674f4ab073fa51f787076273ef5766231a",
    "tools/test_release.py": "4590e132e4d54c3e897ba57996c36dd3bff145ef111ae0f0a5bea1bc05f155a0",
    "tools/test_vmb_scale_probe.py": "89dc5cb7f58c1bb17e332aaa1cf43f36c6ae6afef39723a23b01cb9c6e7c4494",
    "tools/verify_book_v2_block_width_pdf.py": "5aef5709ec9423d56a548006c4178bdfebc2abe560c282f5197db53f647ee7c6",
    "tools/verify_book_v2_cff_diagnostics.py": "bce8bf75a7c4c3b7337942ef3a51e8f2e64919844865ade70d7870fec78b083e",
    "tools/verify_book_v2_cff_subset_diagnostics.py": "a3054923d2750af10e9860eb1f16591c2f7f2417f5168de5d6a9449d13c0ad53",
    "tools/verify_book_v2_column_balance.py": "7478b540448558812593cf806db0dec8b0fed70c5e96438cb866ca7ff457fccf",
    "tools/verify_book_v2_column_display.py": "cd3f9275cb4374af7cb4ba76cadd7901173c122239b70b8bd70563ef79026075",
    "tools/verify_book_v2_column_marked.py": "6a98e77fe1abf6ef88c6f0a91655bdf8d07a36c5081717b087ee94b9d864b1a5",
    "tools/verify_book_v2_column_navigation.py": "8c91153396b4ac9ecdec2af17608a5725b69203bdfedc45fd4951589c6991e2c",
    "tools/verify_book_v2_column_paint_images.py": "aa857c74fcc58f7e32d94af288efb56df5852a36373de8916eec236c8c2b0978",
    "tools/verify_book_v2_column_paints.py": "722863663d1477183c243b980a24a540ec6d303b7f505f01e0fa585c3886d051",
    "tools/verify_book_v2_column_pdf.py": "e00fba88fb4615c6cf5ccf8afa09fd7f08d168b5654d9ea9e2e18aef9819190a",
    "tools/verify_book_v2_column_relations.py": "b29cb6435841fb359fea42337a7e1be1c488dde41cb9a0fbfc727f6064db6a82",
    "tools/verify_book_v2_column_resources.py": "3f802ac049afe7469c451e17b4ae2dafd2b1bddfd2189878f873d5e9bc33fc21",
    "tools/verify_book_v2_column_running_regions.py": "be25eef861d0db1e72a90f2ca3b46e7c9e9aac54dcb9ce8c2744657fe7cc50e4",
    "tools/verify_book_v2_definition_table_pdf.py": "934f953e3260e8b476590ca3349b338db0df9e9e754f42c8bb4b54e4e7fe344b",
    "tools/verify_book_v2_font_objects.py": "a6fbd928ee5f9580c1dadc69e98bd005cd6b0b17e665c3c701ca6facc61a9a1c",
    "tools/verify_book_v2_header_pdfs.py": "a201aa264aa8e29fc88fc57b1427d2b4b38cd95abb8ca59479ff6f3066cb3c34",
    "tools/verify_book_v2_header_resources.py": "99dc6d1f0a03690c8315545bee3a8f6e8c3ed592d39e2841e3889e1e9e3fcb32",
    "tools/verify_book_v2_horizontal_origins_pdf.py": "fc0f9238390f060477ddbf8a96fe553e262b0ca4e0c1938aadc89d866fc97599",
    "tools/verify_book_v2_image_commands.py": "175f37a1bbda2c6b6daed62ff90e37cb124a6697083f907e3fcc4771b363f3d3",
    "tools/verify_book_v2_image_objects.py": "9281590c4fbfe8248702ea677fb2e7b68f604c3763701c3a3fd0f2df33a567fa",
    "tools/verify_book_v2_marked_content.py": "e8cc8e80d1bccf0efc857a2eeea724d5fc8d401be34ced4028cec4b9c9eff1f5",
    "tools/verify_book_v2_marked_scopes.py": "b8b0c41e9337aa76c767edace88371561937976f21cbb5ce50fd443ca135ddfb",
    "tools/verify_book_v2_named_footnotes.py": "7aba0f068c9b2d7e8a2a0eab4806c71264dc503a07ddd7a40ab8dda0d8346477",
    "tools/verify_book_v2_named_pages_pdf.py": "ba2655c1ed4d2a0fd704b3ab415e01c8c4c516c340220e35eacf8853ba0fd2e3",
    "tools/verify_book_v2_natural_cell_pdf.py": "11ef72fe8ed3a85a161b296dcec9e3f1d3678a6d29d2f9d5a0f8942e4564fe96",
    "tools/verify_book_v2_navigation_geometry.py": "91ddd639dea6f5e55e2327446c3d1437fd15411fbfc362bb77015c8e35919070",
    "tools/verify_book_v2_nested_caption_pdf.py": "9c27c619eb927ddedbc1a6911f7bcfc6036208fed779e798753d5e4a7115b0e0",
    "tools/verify_book_v2_nested_header_pdf.py": "1d9630fa3edbdea6da5b35d1bbe5af341c1a81368cc06e2113969a4dc706a1a4",
    "tools/verify_book_v2_nested_keep_pdf.py": "28d63d5b07d1bc013c8ef167afa019e680647763e8e9520912fc031e0d1b17d3",
    "tools/verify_book_v2_nested_span_pdf.py": "38f5c881df5b21084c1103ff189646101ca87058afeb97be8dbdaf75c1d8f7a7",
    "tools/verify_book_v2_nested_table_pdf.py": "1d78287072b25855b05fa3e2d42dca5e3e8fcdfba5872a64ab08de2b4a524043",
    "tools/verify_book_v2_page_frames_pdf.py": "d5ada66a9bdf2e71f31b520ecb1eedbf7a2aa4f150e3033953bafc9c6395f96f",
    "tools/verify_book_v2_page_region_pdfs.py": "70536252f3916d2797a2135cc01f55d4085fbe0913848f4c01767d5663c74062",
    "tools/verify_book_v2_pdf_assembly.py": "ce25d0fc00d0df47e6170513da4e61c47d8437032e4fca4fa0d207e3564d7e4f",
    "tools/verify_book_v2_raster_programs.py": "3affbd5538166b374784b53f2d573c73a933a859115cc5cbd0aeba9a6ab22829",
    "tools/verify_book_v2_source_structure.py": "3fa6f61c55ff09514b5c336c265adcff75ccd4cd2173f4dc9037bcaf43ee79bc",
    "tools/verify_book_v2_structure_relations.py": "79352f8d35920383a5483112db2703df0152aa944275519bcadc846a6daece66",
    "tools/verify_book_v2_table_alignment_pdf.py": "9eb14e8e3176c824eb24d0947230f10edc16228cee94f66f14d28a6ec1e111b0",
    "tools/verify_book_v2_table_block_source_pdf.py": "a5bf7834eb37925eee238ce84d7e28456cbc5a9d857a67997a998ea98bf5667c",
    "tools/verify_book_v2_table_caption_break_pdf.py": "07876a522d32c6c3495524722e892ce4b0ef00280e3797577076240d94ef929e",
    "tools/verify_book_v2_table_caption_pdf.py": "fafae3fc2dddbeefe529ded1f84b4b6bbf7fb5d2bc2bfe0842a72687cbfa6cbf",
    "tools/verify_book_v2_table_cell_break_pdf.py": "7c4d4c511bb13ebc73d0ab33a8aadceb8d2e8a68e240a2b5fcceaf99bed4ba01",
    "tools/verify_book_v2_table_header_break_pdf.py": "aefe8a545b19b06bb35e5e87b636b40145162941a898421c7300047aba4217e8",
    "tools/verify_book_v2_table_name_transitions.py": "d2090c483de17832bf30841dbb678c91faf818499dccaaceaf8cadd0b22acfa0",
    "tools/verify_book_v2_table_rowspan_break_pdf.py": "2b3a74c5bb043a067025811f25eb205416b183f5f9d8ce4c2df72e405ffe5f28",
    "tools/verify_book_v2_table_source_pdf.py": "e648451cb19f0763dc875e30c0c264aa0f2656a5a9f14ad8a90a848e01d74954",
    "tools/verify_book_v2_table_width_pdf.py": "2d448115f9627401a3ffd6e28574928f7cd1186dc247fbfba1ae3f44770fd9ac",
    "tools/verify_book_v2_text_commands.py": "f1693dfbd57f0b35b7244a278f5122a1be204a9da87e16bb9763f0e1ee6aa086",
    "tools/verify_book_v2_uniform_table_pages.py": "c60bab45fa436c4bf5b3e46cc7e0d9729ef0009b12996bfe17bed8fbe74cb649",
    "tools/verify_book_v2_variable_width_pdf.py": "ce060bf9f8059b57792a292d51f6ddd0eb6e944cd0a1ed27a0eb41cbb160b963",
    "tools/verify_book_v2_vector_programs.py": "920f635d2e8d23a9f6a857344b0d37f324ed39a288e15b389aaf746129d04ba8",
    "tools/verify_harano_cff_subset.py": "e889a0920a5f0544008032b0d0245ac37596a7a6adc05ec2e2b2af57078939c5",
    "tools/verify_legacy_cff_diagnostics.py": "b4785d8b7fe26e9d936fc48f4f11cfbeac5006ef09a3176ba9273c7dd77cf3b4",
    "tools/verify_machine_profile.py": "4113ac8f6ba1df2f427cff4a8607b65180cf378514fa8b1552009796d468c441",
    "tools/verify_pdf_differential.py": "029fb1beef07049ea78758290acdd99628ff85d05d30540254203262ca4e2100",
    "tools/verify_pdf_structure.py": "bdc78e098d7b6b7ff069841701abc383b94042ecf73915bc3fcc80c52a74462d",
    "tools/verify_precomposed_vector.py": "f1a4f7dd683310543142762408d4310b4a4f64526b764c64f25330dcc860709b",
    "tools/verify_production_body_probe.py": "68001b56aa1df483013dafadec1c2f1bb657d45c43797ac8e4374c9d72062ddc",
    "tools/verify_production_list_probe.py": "6a86eba6c24fe6601fe54906986486adefd72c24f211542a8ee3ce23b0bd0c97",
    "tools/verify_production_navigation_probe.py": "38ee240df3918903719557c1b2cd1a4d75136a6ce236335b019b6617286ab75f",
    "tools/verify_production_number_probe.py": "5b85205089520f8394873e2b9e452a9c6b5a5219197df22ea5dbe1aa81845a5c",
    "tools/verify_production_raster_probe.py": "bff20e50cff852cb159d2921ef5a90d2c7240034c917f0a29f2f637b59d6d3a5",
    "tools/verify_public_cff_diagnostics.py": "34f526787f0c1cdc9eeac7d52564c864c800e19aaec84363c61d97dada03a729",
    "tools/verify_reproducibility.py": "1e44ac63815eb1f287c58871563f8aaf341f1363e6873f80e0f1e9ec9c1551f1",
    "tools/verify_vmb_description_pdf.py": "9300264b8a57e67d9c3d77e2f9d4f5c83b03a28fd27c6ad6ca8b1ca364bac677",
    "tools/verify_vmb_font_diagnostics.py": "1e57d8b29e41605ef960fe315d45f9223a7ffc571a4777bed4f1237e4d671267",
    "tools/verify_vmb_number_pdf.py": "2904850524e493a28e67e94d75f02c25e853a12d4e1ef9bdad7b29a409d2ab80",
    "tools/verify_vmb_scale_pdf.py": "9209af8209dc56058ecb80aaf5e5edaf855c4c51738c01a323270436c952f02e",
    "tools/verify_vmb_table_alignment_pdf.py": "2444805972d11e1c3833901e8985662570b9486056fc3f35842f1673b0ae5ee5"
}


if __name__ == "__main__":
    main()

