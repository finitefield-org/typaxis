//! Generated synthetic CID CFF branch controls. These do not replace the pinned
//! original Harano gate and do not assert all-CFF, public or whole-book closure.
use super::*;
use std::{collections::BTreeSet, sync::Arc};
use typaxis_core::{FontFaceId, FontInstanceId};
use typaxis_display_list::book_v2::BookV2ResourceDisplay;
use typaxis_font::{
    admit_sfnt_cff1, admit_sfnt_cff1_v2, Cff1Error, Cff1SubsetSession, CffSelectionFailureV2,
    OriginalGlyphId,
};
use typaxis_resources::book_v2::{
    BookV2CffProgramError, BookV2CffProgramStatistics, BookV2FontClosureKind,
    BookV2FontCommandSession,
};

const ORIGINAL_HEX: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/cff-media/typaxis-cff-fixture.otf.hex"));
const SYNTHETIC_TEXT: &str = "AB";
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn statistics(s: BookV2CffProgramStatistics) -> [u64; 3] {
    [
        s.operations_used,
        s.outline_segments_used,
        s.cached_glyphs as u64,
    ]
}
fn evidence(case: &str, mut facts: Value, font: &[u8], pdf: Option<&[u8]>) {
    let directory = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/vmb-design/20261015/column-cff-session-controls/actual-probes")
        })
        .join("column-cff-session-controls");
    fs::create_dir_all(&directory).unwrap();
    let font_name = format!("generated-cid-{}.otf", hex(&typaxis_core::sha256(font)));
    fs::write(directory.join(&font_name), font).unwrap();
    facts["wire"] = "typaxis.generated-synthetic-cid-cff-control/1".into();
    facts["case"] = case.into();
    facts["generated_font"] =
        json!({"file":font_name,"sha256":hex(&typaxis_core::sha256(font)),"bytes":font.len()});
    facts["source_fixture_sha256"] =
        "16da5a41c10414b308aa62db39066e0540e460f3e2d0f9c2811f0b32907d94f0".into();
    facts["scope"] =
        "generated synthetic CID carrier branch only; original Harano gate remains unexecuted"
            .into();
    if let Some(pdf) = pdf {
        let name = format!("{case}-{}.pdf", hex(&typaxis_core::sha256(pdf)));
        fs::write(directory.join(&name), pdf).unwrap();
        facts["pdf"] =
            json!({"file":name,"sha256":hex(&typaxis_core::sha256(pdf)),"bytes":pdf.len()});
    }
    let raw = serde_json::to_vec_pretty(&facts).unwrap();
    fs::write(
        directory.join(format!("{case}-{}.json", hex(&typaxis_core::sha256(&raw)))),
        raw,
    )
    .unwrap();
    eprintln!("Synthetic CID CFF {case}: {facts}");
}
fn u16_at(b: &[u8], at: usize) -> usize {
    u16::from_be_bytes(b[at..at + 2].try_into().unwrap()) as usize
}
fn u32_at(b: &[u8], at: usize) -> usize {
    u32::from_be_bytes(b[at..at + 4].try_into().unwrap()) as usize
}
fn sfnt_checksum(b: &[u8]) -> u32 {
    b.chunks(4)
        .map(|part| {
            let mut word = [0; 4];
            word[..part.len()].copy_from_slice(part);
            u32::from_be_bytes(word)
        })
        .fold(0u32, u32::wrapping_add)
}
fn make_index(objects: &[Vec<u8>]) -> Vec<u8> {
    let mut b = (objects.len() as u16).to_be_bytes().to_vec();
    if objects.is_empty() {
        return b;
    }
    let end = 1 + objects.iter().map(Vec::len).sum::<usize>();
    assert!(end < 256);
    b.push(1);
    let mut offset = 1;
    b.push(offset as u8);
    for o in objects {
        offset += o.len();
        b.push(offset as u8);
    }
    for o in objects {
        b.extend_from_slice(o);
    }
    b
}
fn dict_op(values: &[i32], operator: u16, b: &mut Vec<u8>) {
    for v in values {
        b.push(29);
        b.extend_from_slice(&v.to_be_bytes());
    }
    if operator > 255 {
        b.push(12);
    }
    b.push(operator as u8);
}
fn type2_integer(v: i32, b: &mut Vec<u8>) {
    assert!((-32768..=32767).contains(&v));
    b.push(28);
    b.extend_from_slice(&(v as i16).to_be_bytes());
}
fn fresh_source(scaffold: &[u8]) -> Vec<u8> {
    // New, fully authored tiny test source. The legacy writer is only SFNT
    // scaffolding; its CFF/private/charstrings and old subset name are replaced.
    // Values below are source data, not modified admission receipts or limits.
    let ps = b"TypaxisCIDControl";
    let outlines: Vec<Vec<(u8, Vec<i32>)>> = vec![
        vec![(21, vec![50, 0]), (5, vec![500, 0, 0, 700, -500, 0])],
        vec![(21, vec![50, 0]), (5, vec![250, 700, 250, -700])],
        vec![
            (21, vec![50, 0]),
            (5, vec![0, 700]),
            (8, vec![450, 0, 0, -350, -450, 0, 450, 0, 0, -350, -450, 0]),
        ],
    ];
    let chars = outlines
        .iter()
        .map(|ops| {
            let mut b = Vec::new();
            for (op, values) in ops {
                for v in values {
                    type2_integer(*v, &mut b);
                }
                b.push(*op);
            }
            b.push(14);
            b
        })
        .collect::<Vec<_>>();
    let name = make_index(&[ps.to_vec()]);
    let strings = make_index(&[b"Adobe".to_vec(), b"Identity".to_vec(), ps.to_vec()]);
    let globals = make_index(&[]);
    let char_index = make_index(&chars);
    let charset = [0, 0, 1, 0, 2];
    let fd_select = [0, 0, 0, 0];
    let mut private = Vec::new();
    dict_op(&[600], 20, &mut private);
    dict_op(&[0], 21, &mut private);
    let mut offsets = [0usize; 5];
    let mut cff = Vec::new();
    for _ in 0..4 {
        let mut top = Vec::new();
        dict_op(&[391, 392, 0], 0x0c1e, &mut top);
        dict_op(&[50, 0, 550, 700], 5, &mut top);
        dict_op(&[3], 0x0c22, &mut top);
        for (i, op) in [15, 17, 0x0c25, 0x0c24].iter().enumerate() {
            dict_op(&[offsets[i] as i32], *op, &mut top);
        }
        let top = make_index(&[top]);
        let mut fd = Vec::new();
        dict_op(&[private.len() as i32, offsets[4] as i32], 18, &mut fd);
        dict_op(&[393], 0x0c26, &mut fd);
        let fd = make_index(&[fd]);
        let base = 4 + name.len() + top.len() + strings.len() + globals.len();
        let next = [
            base,
            base + charset.len(),
            base + charset.len() + char_index.len(),
            base + charset.len() + char_index.len() + fd_select.len(),
            base + charset.len() + char_index.len() + fd_select.len() + fd.len(),
        ];
        if offsets == next {
            cff.extend_from_slice(&[1, 0, 4, 4]);
            for part in [
                &name[..],
                &top,
                &strings,
                &globals,
                &charset,
                &char_index,
                &fd_select,
                &fd,
                &private,
            ] {
                cff.extend_from_slice(part);
            }
            break;
        }
        offsets = next;
    }
    assert_eq!(cff.len(), offsets[4] + private.len());
    let program =
        typaxis_font::inspect_cff1_program_v2(Arc::from(cff.clone()), 3, 1000, 100000).unwrap();
    assert_eq!(program.width_defaults(0), Some((600 * 65536, 0)));
    assert_eq!(program.font_dict_count(), 1);
    let mut tables = std::collections::BTreeMap::<[u8; 4], Vec<u8>>::new();
    for i in 0..u16_at(scaffold, 4) {
        let p = 12 + 16 * i;
        let tag = scaffold[p..p + 4].try_into().unwrap();
        let at = u32_at(scaffold, p + 8);
        let len = u32_at(scaffold, p + 12);
        tables.insert(tag, scaffold[at..at + len].to_vec());
    }
    let inherited = tables
        .iter()
        .map(|(t, b)| {
            (
                String::from_utf8(t.to_vec()).unwrap(),
                hex(&typaxis_core::sha256(b)),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    tables.insert(*b"CFF ", cff.clone());
    let names = [
        (1, "Typaxis CID Control"),
        (2, "Regular"),
        (3, "Typaxis CID Control source 1"),
        (4, "Typaxis CID Control Regular"),
        (6, "TypaxisCIDControl"),
    ];
    let mut name = Vec::new();
    name.extend_from_slice(&0u16.to_be_bytes());
    name.extend_from_slice(&(names.len() as u16).to_be_bytes());
    name.extend_from_slice(&((6 + 12 * names.len()) as u16).to_be_bytes());
    let mut storage = Vec::new();
    for (id, text) in names {
        let encoded = text
            .encode_utf16()
            .flat_map(u16::to_be_bytes)
            .collect::<Vec<_>>();
        for v in [
            3u16,
            1,
            0x409,
            id,
            encoded.len() as u16,
            storage.len() as u16,
        ] {
            name.extend_from_slice(&v.to_be_bytes());
        }
        storage.extend(encoded);
    }
    name.extend(storage);
    tables.insert(*b"name", name);
    let mut fmt4 = Vec::new();
    for v in [
        4u16,
        40,
        0,
        6,
        4,
        1,
        2,
        65,
        66,
        65535,
        0,
        65,
        66,
        65535,
        (-64i16) as u16,
        (-64i16) as u16,
        1,
        0,
        0,
        0,
    ] {
        fmt4.extend_from_slice(&v.to_be_bytes());
    }
    assert_eq!(fmt4.len(), 40);
    let mut cmap = Vec::new();
    for v in [0u16, 1, 3, 1] {
        cmap.extend_from_slice(&v.to_be_bytes());
    }
    cmap.extend_from_slice(&12u32.to_be_bytes());
    cmap.extend(fmt4);
    tables.insert(*b"cmap", cmap);
    let mut head = tables[b"head"].clone();
    head[8..12].fill(0);
    head[18..20].copy_from_slice(&1000u16.to_be_bytes());
    for (i, v) in [50i16, 0, 550, 700].iter().enumerate() {
        head[36 + 2 * i..38 + 2 * i].copy_from_slice(&v.to_be_bytes());
    }
    tables.insert(*b"head", head);
    let mut hhea = tables[b"hhea"].clone();
    for (at, v) in [
        (4, 800i16),
        (6, -200),
        (8, 0),
        (10, 600),
        (12, 50),
        (14, 50),
        (16, 550),
    ] {
        hhea[at..at + 2].copy_from_slice(&v.to_be_bytes());
    }
    hhea[34..36].copy_from_slice(&3u16.to_be_bytes());
    tables.insert(*b"hhea", hhea);
    let mut hmtx = Vec::new();
    for _ in 0..3 {
        hmtx.extend_from_slice(&600u16.to_be_bytes());
        hmtx.extend_from_slice(&50i16.to_be_bytes());
    }
    tables.insert(*b"hmtx", hmtx);
    let mut maxp = 0x00005000u32.to_be_bytes().to_vec();
    maxp.extend_from_slice(&3u16.to_be_bytes());
    tables.insert(*b"maxp", maxp);
    let count = tables.len();
    let power = 1usize << (usize::BITS - 1 - count.leading_zeros());
    let mut bytes = b"OTTO".to_vec();
    for v in [
        count as u16,
        (power * 16) as u16,
        power.ilog2() as u16,
        ((count - power) * 16) as u16,
    ] {
        bytes.extend_from_slice(&v.to_be_bytes());
    }
    bytes.resize(12 + 16 * count, 0);
    let mut head_at = 0;
    let mut output_tables = Vec::new();
    for (i, (tag, body)) in tables.iter().enumerate() {
        while bytes.len() % 4 != 0 {
            bytes.push(0);
        }
        let at = bytes.len();
        let p = 12 + 16 * i;
        bytes[p..p + 4].copy_from_slice(tag);
        bytes[p + 4..p + 8].copy_from_slice(&sfnt_checksum(body).to_be_bytes());
        bytes[p + 8..p + 12].copy_from_slice(&(at as u32).to_be_bytes());
        bytes[p + 12..p + 16].copy_from_slice(&(body.len() as u32).to_be_bytes());
        bytes.extend_from_slice(body);
        if tag == b"head" {
            head_at = at;
        }
        output_tables.push(json!({"tag":String::from_utf8(tag.to_vec()).unwrap(),"offset":at,"bytes":body.len(),"sha256":hex(&typaxis_core::sha256(body)),"inherited_unchanged":inherited[&String::from_utf8(tag.to_vec()).unwrap()]==hex(&typaxis_core::sha256(body))}));
    }
    while bytes.len() % 4 != 0 {
        bytes.push(0);
    }
    let adj = 0xB1B0AFBAu32.wrapping_sub(sfnt_checksum(&bytes));
    bytes[head_at + 8..head_at + 12].copy_from_slice(&adj.to_be_bytes());
    assert_eq!(sfnt_checksum(&bytes), 0xB1B0AFBA);
    evidence(
        "fresh-authored-source",
        json!({"stage":"NEW fully authored bounded synthetic CID source, normal /2 admission follows","not_unchanged_legacy_writer":true,"scaffold_sha256":hex(&typaxis_core::sha256(scaffold)),"authored":{"postscript_name":"TypaxisCIDControl","ROS":["Adobe","Identity",0],"glyph_count":3,"CIDCount":3,"fd_select":[0,0,0],"charset":[0,1,2],"defaultWidthX":600,"nominalWidthX":0,"upem":1000,"bbox":[50,0,550,700],"hhea":[800,-200,0],"hmtx":[[600,50],[600,50],[600,50]],"cmap":[[65,1],[66,2]],"outlines_type2_ops":outlines,"charstrings":chars,"cff_bytes":cff,"tables":output_tables}}),
        &bytes,
        None,
    );
    bytes
}
fn generated_cid(caps: &M4EffectiveResourceLimits) -> Vec<u8> {
    let original = ORIGINAL_HEX.split_whitespace().collect::<String>();
    let original = (0..original.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&original[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        hex(&typaxis_core::sha256(&original)),
        "16da5a41c10414b308aa62db39066e0540e460f3e2d0f9c2811f0b32907d94f0"
    );
    let source = admit_sfnt_cff1(&original, 0, caps).unwrap();
    let selected = BTreeSet::from([OriginalGlyphId::new(1), OriginalGlyphId::new(2)]);
    let mut writer = Cff1SubsetSession::from_admission(&source);
    let written = writer
        .subset(
            &source,
            FontFaceId::new(0),
            FontInstanceId::new(0),
            &selected,
            10,
        )
        .unwrap();
    let bytes = fresh_source(written.bytes());
    let admitted = admit_sfnt_cff1_v2(Arc::from(bytes.clone()), 0, caps).unwrap();
    assert_eq!(admitted.glyph_count(), 3);
    assert_eq!(admitted.program().font_dict_count(), 1);
    assert_eq!(admitted.cmap().glyph_for_sequence('A', None), Some(1));
    assert_eq!(admitted.cmap().glyph_for_sequence('B', None), Some(2));
    assert_eq!(admitted.source(), bytes);
    evidence(
        "fresh-authored-admitted",
        json!({"stage":"actual normal admit_sfnt_cff1_v2 succeeded","glyphs":admitted.glyph_count(),"fds":admitted.program().font_dict_count()}),
        &bytes,
        None,
    );
    bytes
}
fn prepared(
    root: &Root,
    caps: &M4EffectiveResourceLimits,
    bytes: &[u8],
) -> PreparedBookV2Resources {
    let mut value = data(false);
    let span = json!({"source_id":0,"start_byte":0,"end_byte":SYNTHETIC_TEXT.len()});
    value["text_buffers"] = json!([{"text_id":0,"utf8":SYNTHETIC_TEXT,"mappings":[{"kind":"identity","source_span":span,"text_range":{"start_byte":0,"end_byte":SYNTHETIC_TEXT.len()}}]}]);
    value["sources"][0]["sha256"] = hex(&typaxis_core::sha256(SYNTHETIC_TEXT.as_bytes())).into();
    value["sources"][0]["utf8_byte_length"] = SYNTHETIC_TEXT.len().into();
    value["document"]["blocks"] = json!([{"kind":"paragraph","node_id":1,"span":span,"classes":[],"children":[{"kind":"text","node_id":2,"span":span,"text_span":{"text_id":0,"start_byte":0,"end_byte":SYNTHETIC_TEXT.len()}}]}]);
    value["resources"]["font_faces"] = json!([{"font_face_id":0,"family":"Body","uri":"body.bin","face_index":0,"media_type":"sfnt-cff1","expected_sha256":hex(&typaxis_core::sha256(bytes))}]);
    value["resources"]["images"] = json!([]);
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    fs::write(root.0.join("input.tsf"), SYNTHETIC_TEXT).unwrap();
    fs::write(root.0.join("body.bin"), bytes).unwrap();
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            None,
        ),
        caps.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(caps.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, caps.base()).unwrap();
    let body =
        prepare_admitted_book_v2_body(session.finish(raw, decoded, sources).unwrap(), caps.base())
            .unwrap();
    let prepared = crate::book_v2_resources::prepare_book_v2_resources(
        body,
        &root.context(),
        &config(caps),
        caps,
    )
    .unwrap();
    assert_eq!(prepared.resources().fonts().len(), 1);
    let typaxis_resources::AdmittedProductionFontV3::Cff1V2(actual) =
        &prepared.resources().fonts()[0]
    else {
        panic!("not genuine CID /2 admission")
    };
    assert_eq!(actual.admission().source(), bytes);
    prepared
}
fn loan<'a, D: BookV2ResourceDisplay<'a>>(
    session: &mut BookV2FontCommandSession,
    display: &D,
    caps: &M4EffectiveResourceLimits,
    application_error: bool,
    full: bool,
) -> Value {
    let (mut r, mut s, mut w) = (
        session.record_charge(),
        session.spool_charge(),
        session.work_steps(),
    );
    let before = [r, s, w];
    let before_cff = statistics(session.cff_program_statistics());
    let mut fact = Value::Null;
    let outcome=session.with_builder_counted(display,caps,r,s,w,&mut r,&mut s,&mut w,|builder| {
        let selection=builder.build().unwrap();let closures=builder.prepare_font_closures(&selection).unwrap();
        assert!(!closures.fonts().is_empty());assert!(closures.fonts().iter().all(|f|f.kind()==BookV2FontClosureKind::Cff1V2));
        let selected=closures.fonts().iter().flat_map(|f|f.glyphs().map(|g|g.get())).collect::<BTreeSet<_>>();
        assert_eq!(selected,BTreeSet::from([0,1,2]));
        let outcome=builder.prepare_cff_programs(&closures);
        let error=outcome.as_ref().err().map(|e|format!("{e:?}"));
        let typed=match &outcome {Err(BookV2CffProgramError::Cff(CffSelectionFailureV2::Glyph(e)))=>Some(json!({"kind":format!("{:?}",e.kind),"gid":e.gid,"fd":e.fd,"operator":e.operator,"context":format!("{:?}",e.font_context)})),_=>None};
        let positive=outcome.is_ok();let mut program_hashes=Vec::new();
        if positive && full {
            let stats=builder.cff_program_statistics();let programs=builder.write_font_programs(&closures).unwrap();
            let cids=builder.plan_cids(&programs).unwrap();assert_eq!(cids.fonts().len(),1);
            program_hashes=programs.fonts().iter().map(|f|hex(&f.sha256())).collect();
            assert_eq!(builder.cff_program_statistics(),stats);
            let (mut rr,mut ss,mut ww)=(builder.record_charge(),builder.spool_charge(),builder.work_steps());
            builder.next_stage_after_counted(rr,ss,ww,&mut rr,&mut ss,&mut ww).unwrap();
            let images=builder.select_images().unwrap();let _rasters=builder.write_raster_programs(&images).unwrap();
            assert_eq!(builder.cff_program_statistics(),stats);
        }
        fact=json!({"before":before,"before_cff":before_cff,"positive":positive,"error":error,"typed":typed,"selected":selected,"program_hashes":program_hashes,"inside":[builder.record_charge(),builder.spool_charge(),builder.work_steps()],"inside_cff":statistics(builder.cff_program_statistics())});
        if application_error || !positive {Err("synthetic CFF callback sentinel")} else {Ok(())}
    });
    let callback = match outcome {
        Ok(r) => json!({"called":true,"application_ok":r.is_ok()}),
        Err(e) => json!({"called":false,"constructor_error":format!("{e:?}")}),
    };
    json!({"facts":fact,"callback":callback,"after":[session.record_charge(),session.spool_charge(),session.work_steps()],"observed":[r,s,w],"after_cff":statistics(session.cff_program_statistics())})
}
#[test]
fn book_v2_column_synthetic_cid_cff_carrier_executes_caches_and_retains_application_error() {
    let caps = caps();
    let bytes = generated_cid(&caps);
    let root = Root::new();
    let input = prepared(&root, &caps, &bytes);
    let mut page_budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    crate::book_v2_resources::with_budgeted_book_v2_column_display(
        &input,
        &caps,
        MODE,
        &mut page_budget,
        |display, _, _| {
            let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
            let first = loan(&mut session, display, &caps, false, true);
            assert_eq!(first["facts"]["positive"], true);
            assert_eq!(first["callback"]["application_ok"], true);
            let stats = session.cff_program_statistics();
            assert!(stats.operations_used > 0 && stats.outline_segments_used > 0);
            assert_eq!(stats.cached_glyphs, 3);
            let second = loan(&mut session, display, &caps, true, true);
            assert_eq!(second["callback"]["application_ok"], false);
            assert_eq!(session.cff_program_statistics(), stats);
            assert_eq!(second["after"], second["facts"]["inside"]);
            assert_eq!(second["after"], second["observed"]);
            let third = loan(&mut session, display, &caps, false, true);
            assert_eq!(third["callback"]["application_ok"], true);
            assert_eq!(session.cff_program_statistics(), stats);
            assert!(third["after"][2].as_u64().unwrap() > second["after"][2].as_u64().unwrap());
            evidence(
                "carrier-positive",
                json!({"runs":[first,second,third]}),
                &bytes,
                None,
            );
        },
    )
    .unwrap();
}
#[test]
fn book_v2_column_synthetic_cid_cff_public_pdf_has_real_cff_program_and_persistent_retry_cache() {
    let caps = caps();
    let bytes = generated_cid(&caps);
    let root = Root::new();
    let input = prepared(&root, &caps, &bytes);
    let mut budget =
        crate::book_v2_resources::BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let mut called = 0;
    let first = crate::book_v2_resources::with_budgeted_converged_book_v2_column_pdf(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |roots, observed| {
            called += 1;
            assert!(roots.pdf().page_references().is_empty());
            let programs = roots.navigation_bundle().marked().paints().cids().source();
            assert_eq!(programs.fonts().len(), 1);
            assert_eq!(programs.fonts()[0].kind(), BookV2FontClosureKind::Cff1V2);
            (
                roots.pdf().bytes().to_vec(),
                [
                    observed.record_charge(),
                    observed.spool_charge(),
                    observed.output_charge(),
                    observed.work_steps(),
                ],
            )
        },
    )
    .unwrap();
    assert_eq!(called, 1);
    assert!(first.0.starts_with(b"%PDF-"));
    assert!(first
        .0
        .windows(b"/CIDFontType0".len())
        .any(|w| w == b"/CIDFontType0"));
    assert!(first
        .0
        .windows(b"/FontFile3".len())
        .any(|w| w == b"/FontFile3"));
    let stats = budget.cff_program_statistics();
    assert!(stats.operations_used > 0 && stats.outline_segments_used > 0);
    assert_eq!(stats.cached_glyphs, 3);
    let second = crate::book_v2_resources::with_budgeted_converged_book_v2_column_pdf(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |roots, _| {
            called += 1;
            Err::<(), _>(("application sentinel", roots.pdf().bytes().to_vec()))
        },
    )
    .unwrap();
    assert_eq!(called, 2);
    assert!(second.is_err());
    assert_eq!(second.err().unwrap().1, first.0);
    assert_eq!(budget.cff_program_statistics(), stats);
    evidence(
        "public-pdf-positive",
        json!({"callback_count":called,"first_quartet":first.1,"after":[budget.observation().record_charge(),budget.observation().spool_charge(),budget.observation().output_charge(),budget.observation().work_steps()],"cff":statistics(stats)}),
        &bytes,
        Some(&first.0),
    );
}

fn quota_caps(
    base: &M4EffectiveResourceLimits,
    operations: u64,
    segments: u64,
) -> M4EffectiveResourceLimits {
    let mut extension = base.extension().get().clone();
    extension.max_cff_charstring_operations = operations;
    extension.max_cff_outline_segments = segments;
    M4EffectiveResourceLimits::new(base.base().clone(), extension).unwrap()
}
#[test]
fn book_v2_column_synthetic_cid_cff_exact_quota_and_one_short_retries_keep_accepted_type2_work() {
    let defaults = caps();
    let bytes = generated_cid(&defaults);
    let root = Root::new();
    let input = prepared(&root, &defaults, &bytes);
    let mut pages = BookV2ColumnPageBudget::new(&defaults, MAXIMUM);
    let measured = crate::book_v2_resources::with_budgeted_book_v2_column_display(
        &input,
        &defaults,
        MODE,
        &mut pages,
        |display, _, _| {
            let mut session = BookV2FontCommandSession::new(&defaults, MAXIMUM);
            let run = loan(&mut session, display, &defaults, false, false);
            assert_eq!(run["facts"]["positive"], true);
            statistics(session.cff_program_statistics())
        },
    )
    .unwrap();
    assert!(measured[0] > 1 && measured[1] > 1);
    for (name, operations, segments, success, kind) in [
        ("exact", measured[0], measured[1], true, ""),
        (
            "one-short-operations",
            measured[0] - 1,
            measured[1],
            false,
            "CharstringOperationLimit",
        ),
        (
            "one-short-segments",
            measured[0],
            measured[1] - 1,
            false,
            "OutlineSegmentLimit",
        ),
    ] {
        let caps = quota_caps(&defaults, operations, segments);
        // Generation was done under defaults. The unchanged actual CID bytes
        // are re-admitted with each genuine new limit fingerprint.
        let root = Root::new();
        let input = prepared(&root, &caps, &bytes);
        let mut pages = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
        crate::book_v2_resources::with_budgeted_book_v2_column_display(&input, &caps, MODE, &mut pages, |display, _, _| {
            let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
            let first = loan(&mut session, display, &caps, false, false);
            assert_eq!(first["facts"]["positive"], success);
            if !success { assert_eq!(first["facts"]["typed"]["kind"], kind); }
            let spent = session.cff_program_statistics();
            assert!(spent.operations_used > 0 && spent.operations_used <= operations);
            assert!(spent.outline_segments_used <= segments);
            let before = [session.record_charge(), session.spool_charge(), session.work_steps()];
            let second = loan(&mut session, display, &caps, false, false);
            assert_eq!(second["facts"]["positive"], success);
            let after = session.cff_program_statistics();
            assert!(after.operations_used >= spent.operations_used && after.operations_used <= operations);
            assert!(after.outline_segments_used >= spent.outline_segments_used && after.outline_segments_used <= segments);
            assert!(after.cached_glyphs >= spent.cached_glyphs);
            assert_eq!(second["after"], second["observed"]);
            assert_eq!(second["after"], second["facts"]["inside"]);
            for i in 0..3 { assert!(second["after"][i].as_u64().unwrap() >= before[i]); }
            if success { assert_eq!(statistics(after), measured); assert_eq!(after, spent); }
            else { assert!(after.cached_glyphs < measured[2] as usize); }
            evidence(name, json!({"measured":measured,"limits":[operations,segments],"runs":[first,second]}), &bytes, None);
        }).unwrap();
    }
}
fn glyph_failure(error: &(dyn std::error::Error + 'static)) -> Option<String> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(glyph) = error.downcast_ref::<typaxis_font::CffGlyphFailureV2>() {
            return Some(format!("{:?}", glyph.kind));
        }
        current = error.source();
    }
    None
}
#[test]
fn book_v2_column_synthetic_cid_cff_public_pdf_quota_refusal_never_delivers_or_resets_retry_prefix()
{
    let defaults = caps();
    let bytes = generated_cid(&defaults);
    let caps = quota_caps(
        &defaults,
        1,
        defaults.extension().get().max_cff_outline_segments,
    );
    let root = Root::new();
    let input = prepared(&root, &caps, &bytes);
    let mut budget =
        crate::book_v2_resources::BookV2ColumnPdfConvergenceBudget::new(&caps, MAXIMUM);
    let before = budget.observation();
    let mut outcomes = Vec::new();
    for attempt in 0..2 {
        let prior = budget.observation();
        let prior_cff = budget.cff_program_statistics();
        let error = crate::book_v2_resources::with_budgeted_converged_book_v2_column_pdf(
            &input,
            &caps,
            MODE,
            &mut budget,
            4,
            |_, _| panic!("quota-refused synthetic CFF reached PDF callback"),
        )
        .unwrap_err();
        assert_eq!(
            glyph_failure(&error),
            Some(format!("{:?}", Cff1Error::CharstringOperationLimit))
        );
        let now = budget.observation();
        let cff = budget.cff_program_statistics();
        assert_eq!(cff.operations_used, 1);
        assert_eq!(cff.cached_glyphs, 0);
        assert!(cff.outline_segments_used >= prior_cff.outline_segments_used);
        assert!(
            now.record_charge() >= prior.record_charge()
                && now.spool_charge() >= prior.spool_charge()
                && now.work_steps() >= prior.work_steps()
        );
        assert_eq!(now.output_charge(), prior.output_charge());
        outcomes.push(json!({"attempt":attempt,"error":format!("{error:?}"),"prior":[prior.record_charge(),prior.spool_charge(),prior.output_charge(),prior.work_steps()],"after":[now.record_charge(),now.spool_charge(),now.output_charge(),now.work_steps()],"cff":statistics(cff)}));
    }
    assert!(budget.observation().record_charge() > before.record_charge());
    evidence(
        "public-pdf-quota-refusal",
        json!({"outcomes":outcomes}),
        &bytes,
        None,
    );
}
#[test]
fn book_v2_column_synthetic_cid_cff_carrier_rejects_distinct_admission_and_changed_limits_without_type2_reset(
) {
    let caps = caps();
    let bytes = generated_cid(&caps);
    let root = Root::new();
    let input = prepared(&root, &caps, &bytes);
    let other_root = Root::new();
    let foreign = prepared(&other_root, &caps, &bytes);
    assert!(input.resources().session_identity() != foreign.resources().session_identity());
    let mut session = BookV2FontCommandSession::new(&caps, MAXIMUM);
    let mut pages = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    crate::book_v2_resources::with_budgeted_book_v2_column_display(
        &input,
        &caps,
        MODE,
        &mut pages,
        |display, _, _| {
            assert_eq!(
                loan(&mut session, display, &caps, false, false)["facts"]["positive"],
                true
            );
        },
    )
    .unwrap();
    let stats = session.cff_program_statistics();
    let before = [
        session.record_charge(),
        session.spool_charge(),
        session.work_steps(),
    ];
    let changed = quota_caps(
        &caps,
        caps.extension().get().max_cff_charstring_operations - 1,
        caps.extension().get().max_cff_outline_segments,
    );
    let mut other_pages = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let foreign_outcome = crate::book_v2_resources::with_budgeted_book_v2_column_display(
        &foreign,
        &caps,
        MODE,
        &mut other_pages,
        |display, _, _| loan(&mut session, display, &caps, false, false),
    )
    .unwrap();
    assert_eq!(foreign_outcome["callback"]["called"], false);
    assert_eq!(foreign_outcome["callback"]["constructor_error"], "Identity");
    let mut own_pages = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let limits_outcome = crate::book_v2_resources::with_budgeted_book_v2_column_display(
        &input,
        &caps,
        MODE,
        &mut own_pages,
        |display, _, _| loan(&mut session, display, &changed, false, false),
    )
    .unwrap();
    assert_eq!(limits_outcome["callback"]["called"], false);
    assert_eq!(limits_outcome["callback"]["constructor_error"], "Identity");
    assert_eq!(session.cff_program_statistics(), stats);
    assert_eq!(
        [
            session.record_charge(),
            session.spool_charge(),
            session.work_steps()
        ],
        before
    );
    evidence(
        "admission-identity",
        json!({"foreign":foreign_outcome,"limits":limits_outcome,"cff":statistics(stats)}),
        &bytes,
        None,
    );
}
