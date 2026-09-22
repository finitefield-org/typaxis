use super::*;
use crate::book_v2_resources::with_converged_book_v2_pdf;
use typaxis_display_list::book_v2::BookV2BodyPaintIndex as Paint;
use typaxis_pdf::book_v2::BookV2MarkedArtifact as Artifact;

fn pdf_data(text: &str, empty: bool, overflow: bool) -> Value {
    let mut data = region_data(text);
    let base = data["document"]["blocks"][0]["blocks"][0].clone();
    let mut paragraphs = Vec::new();
    let mut next = 2u32;
    let first = text.chars().next().unwrap().len_utf8();
    for _ in 0..8 {
        let mut p = base.clone();
        p["node_id"] = next.into();
        next += 1;
        p["children"][0]["node_id"] = next.into();
        next += 1;
        p["children"][0]["text_span"]["end_byte"] = first.into();
        paragraphs.push(p);
    }
    data["document"]["blocks"][0]["blocks"] = paragraphs.into();
    let master = &mut data["page_masters"]["masters"][0];
    master["body"]["height"] = (48 * 65536).into();
    if overflow {
        master["footer"]["height"] = (16 * 65536 - 1).into();
    }
    for role in ["header_content", "footer_content"] {
        let region = &mut master[role];
        region["node_id"] = next.into();
        next += 1;
        for block in region["blocks"].as_array_mut().unwrap() {
            block["node_id"] = next.into();
            next += 1;
            if empty {
                block["children"] = json!([]);
            }
            for item in block["children"].as_array_mut().unwrap() {
                item["node_id"] = next.into();
                next += 1;
            }
        }
    }
    data
}
#[test]
fn book_v2_page_region_pdf_emits_each_page_artifact_and_selects_region_only_glyphs() {
    check(None, false, false);
}
#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_page_region_pdf_embeds_original_harano_artifact_glyphs() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    check(Some(&font), false, false);
}
#[test]
fn book_v2_page_region_pdf_preserves_empty_regions_without_semantic_nodes() {
    check(None, true, false);
}
#[test]
fn book_v2_page_region_pdf_rejects_overflow_before_pdf_callback() {
    check(None, false, true);
}
fn check(font: Option<&[u8]>, empty: bool, overflow: bool) {
    let root = Root::new();
    let base = limits();
    let mut caps = base.base().get().clone();
    caps.max_line_reshape_passes = 128;
    caps.max_layout_passes = 128;
    let limits = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(caps).unwrap(),
        base.extension().get().clone(),
    )
    .unwrap();
    let text = if font.is_some() {
        "本文の柱"
    } else {
        "Result"
    };
    let data = pdf_data(text, empty, overflow);
    let input = if let Some(font) = font {
        vector_tests::vector_input_with_font(&root, data.clone(), &limits, font, text.as_bytes())
    } else {
        prepared(&root, data.clone(), text.as_bytes(), &limits)
    };
    let result = with_converged_book_v2_pdf(
        &input,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, observed| {
            assert!(!overflow, "overflow must not issue a PDF");
            let registry = pdf.navigation().source().source();
            let marked = registry.source();
            let scopes = marked.source();
            let display = scopes.display();
            let pages = marked.pages().len();
            assert!(pages >= 2);
            assert_eq!(display.page_regions().len(), 2 * pages);
            let mut unjoined = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                display.source(),
                input.resources(),
                &limits,
                100_000_000,
                0,
                0,
            )
            .unwrap();
            assert!(
                unjoined
                    .build_body()
                    .unwrap()
                    .with_page_regions(Vec::new(), &limits, 100_000_000, 0, 0)
                    .is_err(),
                "even empty authored regions must be attached"
            );
            let navigation = display
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .source_flow()
                .navigation();
            // Check all source language owners, including the blank paragraphs
            // whose regions contain no paints. Body language nodes remain required.
            for language in navigation.languages() {
                assert_eq!(
                    registry
                        .nodes()
                        .iter()
                        .any(|n| n.source().key().owner() == language.node_id()),
                    language.page_region().is_none()
                );
            }
            let mut body_gids = std::collections::BTreeSet::new();
            let mut region_gids = std::collections::BTreeSet::new();
            for (index, paint) in display.paints().iter().enumerate() {
                let usage = display.font_use(index, 0).unwrap().unwrap();
                let target = if matches!(paint, Paint::PageRegionText { .. }) {
                    &mut region_gids
                } else {
                    &mut body_gids
                };
                for i in 0..usage.glyphs().len() {
                    target.insert(usage.glyphs().get(i).unwrap());
                }
            }
            if !empty {
                assert!(region_gids.difference(&body_gids).next().is_some());
            }
            let mut selector = typaxis_resources::book_v2::BookV2FontSelectionBuilder::new(
                display,
                &limits,
                display.work_steps() + 10_000_000,
                0,
                0,
                0,
            )
            .unwrap();
            let selected = selector.build().unwrap();
            let selected_gids: std::collections::BTreeSet<_> = (0..selected.fonts().len())
                .flat_map(|i| selected.glyphs(i).unwrap())
                .collect();
            assert!(region_gids.is_subset(&selected_gids));
            let mut proof_regions = Vec::new();
            for (i, region) in display.page_regions().iter().enumerate() {
                assert_eq!(region.page_index(), (i / 2) as u32);
                assert_eq!(
                    region.kind(),
                    if i % 2 == 0 {
                        Kind::Header
                    } else {
                        Kind::Footer
                    }
                );
                let actual: String = region.draws().iter().map(|d| d.exact_text()).collect();
                assert_eq!(
                    actual,
                    if empty {
                        String::new()
                    } else if i % 2 == 0 {
                        text.repeat(2)
                    } else {
                        text.into()
                    }
                );
                let artifact = if i % 2 == 0 {
                    Artifact::RunningHeader
                } else {
                    Artifact::RunningFooter
                };
                let groups: Vec<_> = scopes
                    .page_groups(region.page_index())
                    .unwrap()
                    .iter()
                    .filter(|g| g.artifact() == Some(artifact))
                    .collect();
                assert_eq!(groups.is_empty(), empty);
                for g in groups {
                    assert_eq!(g.mcid(), None);
                }
                for draw in region.draws() {
                    assert!(registry
                        .nodes()
                        .iter()
                        .all(|n| n.source().key().owner() != draw.owner()));
                    assert!(std::ptr::eq(
                        draw.font_instance().ledger(),
                        input.resources()
                    ));
                }
                proof_regions.push(json!({"page":region.page_index(),"role":region.kind().as_str(),"owner":region.owner().get(),"text":actual,
                "glyphs":region.draws().iter().flat_map(|d|d.glyphs().iter().map(move |g|json!({"gid":g.original_gid().get(),"x":g.x().raw(),"y":g.y().raw(),"text":d.exact_text()}))).collect::<Vec<_>>()}));
            }
            for page in 0..pages {
                let bytes = marked.page_bytes(page).unwrap();
                for role in ["Header", "Footer"] {
                    let needle = format!("/Artifact << /Type /Pagination /Subtype /{role} >> BDC");
                    assert_eq!(
                        bytes.windows(needle.len()).any(|v| v == needle.as_bytes()),
                        !empty
                    );
                }
            }
            if let Ok(folder) = std::env::var("TYPAXIS_BOOK_V2_PAGE_REGION_PROBE") {
                let folder = std::path::Path::new(&folder);
                fs::create_dir_all(folder).unwrap();
                let name = if empty {
                    "empty"
                } else if font.is_some() {
                    "harano"
                } else {
                    "truetype"
                };
                let path = folder.join(format!("{name}.pdf"));
                use std::io::Write;
                fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&path)
                    .unwrap()
                    .write_all(pdf.bytes())
                    .unwrap();
                // Export the actual PDF pipeline's resources, not a second
                // selection whose correspondence to emitted bytes is assumed.
                let cids = marked.text().source().source().source();
                let programs = cids.source();
                let closures = programs.source();
                let selection = closures.selection();
                let resource_dir = folder.join("resources");
                fs::create_dir_all(&resource_dir).unwrap();
                let hex =
                    |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
                let mut fonts = Vec::new();
                for (index, program) in programs.fonts().iter().enumerate() {
                    let original = selection.fonts()[index].instance().font();
                    let source_name = format!("{name}-{index}-source.sfnt");
                    let subset_name = format!("{name}-{index}-subset.sfnt");
                    fs::write(resource_dir.join(&source_name), original.bytes()).unwrap();
                    fs::write(resource_dir.join(&subset_name), program.bytes()).unwrap();
                    fonts.push(json!({"source":source_name,"subset":subset_name,
                        "source_sha256":hex(original.content_hash()),"subset_sha256":hex(program.sha256()),
                        "mapping":closures.fonts()[index].glyphs().map(|g|[g.get(),program.subset_gid(g).unwrap().get()]).collect::<Vec<_>>(),
                        "selected":selection.glyphs(index).unwrap().map(|g|g.get()).collect::<Vec<_>>(),
                        "bindings":cids.bindings(index).unwrap().iter().map(|b|json!({"gid":b.original_gid().get(),"cid":b.cid().get(),"subset":b.subset_gid().get(),"width":b.width_1000(),"unicode":b.unicode().map(|c|c.to_string())})).collect::<Vec<_>>() }));
                }
                let uses = cids.uses().iter().enumerate().map(|(i,u)| {
                    use typaxis_display_list::book_v2::{BookV2FontUseGlyphs,BookV2FontUseText};
                    let usage=u.source().usage();
                    let text = |t| match t { BookV2FontUseText::Text(s) => s.to_owned(), BookV2FontUseText::Scalar(c) => c.to_string() };
                    let (page,role) = match usage.paint() {
                        Paint::PageRegionText{region,..} => (display.page_regions()[region].page_index(),Some(display.page_regions()[region].kind().as_str())),
                        Paint::Text(index) => (display.text().draws()[index].page_index(),None),
                        _ => panic!("plain source fixture"),
                    };
                    let BookV2FontUseGlyphs::Cluster(glyphs)=usage.glyphs() else { panic!("plain source fixture") };
                    json!({"font":u.font_index(),"page":page,"role":role,"size":usage.size().get().raw(),
                        "positions":glyphs.iter().map(|g|[g.x().raw(),g.y().raw()]).collect::<Vec<_>>(),
                        "gids":glyphs.iter().map(|g|g.original_gid().get()).collect::<Vec<_>>(),
                        "cids":cids.cids(i).unwrap().iter().map(|c|c.get()).collect::<Vec<_>>(),
                        "text":text(usage.text()),"actual_text":u.actual_text().map(text)})
                }).collect::<Vec<_>>();
                let resources = json!({"fonts":fonts,"uses":uses,"repeated_only_gid":null});
                fs::write(
                    resource_dir.join(format!("{name}.json")),
                    serde_json::to_vec_pretty(&resources).unwrap(),
                )
                .unwrap();
                let proof = json!({"pdf":path,"pages":pages,"regions":proof_regions,"wire":data,"font_hash":typaxis_core::sha256(font.unwrap_or(FONT)),"source_text":text});
                fs::write(
                    folder.join(format!("{name}.json")),
                    serde_json::to_vec_pretty(&proof).unwrap(),
                )
                .unwrap();
            }
            eprintln!(
                "page-region PDF: font={} empty={empty} pages={pages} bytes={} line_passes={}",
                if font.is_some() { "Harano" } else { "TT" },
                pdf.bytes().len(),
                observed.line_reshape_passes()
            );
        },
    );
    if overflow {
        assert!(result.is_err());
    } else {
        result.unwrap();
    }
}

#[test]
fn book_v2_page_region_pdf_unselected_regions_preserve_join_budgets() {
    let root = Root::new();
    let limits = limits();
    let mut data = pdf_data("Result", false, false);
    let mut unused = data["page_masters"]["masters"][0].clone();
    unused["master_id"] = "unused".into();
    data["page_masters"]["masters"][0]["header_content"] = Value::Null;
    data["page_masters"]["masters"][0]["footer_content"] = Value::Null;
    data["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .push(unused);
    let input = prepared(&root, data, b"Result", &limits);
    with_converged_book_v2_pdf(
        &input,
        &limits,
        typaxis_linebreak::JapaneseLineBreakMode::Normal,
        100_000_000,
        |pdf, _| {
            let registry = pdf.navigation().source().source();
            let display = registry.source().source().display();
            assert!(display.page_regions().is_empty());
            assert!(registry
                .source()
                .source()
                .groups()
                .iter()
                .all(|g| g.artifact().is_none()));
            let navigation = display
                .source()
                .source()
                .flow()
                .lines()
                .prepared()
                .source_flow()
                .navigation();
            assert!(navigation
                .languages()
                .iter()
                .any(|l| l.page_region().is_some()));
            for language in navigation.languages() {
                assert_eq!(
                    registry
                        .nodes()
                        .iter()
                        .any(|n| n.source().key().owner() == language.node_id()),
                    language.page_region().is_none()
                );
            }
            let fresh = || {
                let mut builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
                    display.source(),
                    input.resources(),
                    &limits,
                    100_000_000,
                    0,
                    0,
                )
                .unwrap();
                builder.build_body().unwrap()
            };
            let before = fresh();
            let fingerprint = before.fingerprint();
            let records = before.record_charge() + 7;
            let work = before.work_steps() + 11;
            let joined = before
                .with_page_regions(Vec::new(), &limits, 100_000_000, records, work)
                .unwrap();
            assert_eq!(joined.record_charge(), records);
            assert!(joined.work_steps() > work);
            assert_eq!(joined.fingerprint(), fingerprint);
            let exact = joined.work_steps();
            assert!(fresh()
                .with_page_regions(Vec::new(), &limits, exact, records, work)
                .is_ok());
            assert!(fresh()
                .with_page_regions(Vec::new(), &limits, exact - 1, records, work)
                .is_err());
            let mut observed_records = 0;
            let mut observed_work = 0;
            assert!(fresh()
                .with_page_regions_counted(
                    Vec::new(), &limits, exact - 1, records, work,
                    &mut observed_records, &mut observed_work,
                )
                .is_err());
            assert_eq!(observed_records, records);
            assert_eq!(observed_work, exact - 1);
            let counted = fresh().with_page_regions_counted(
                Vec::new(), &limits, exact, records, work,
                &mut observed_records, &mut observed_work,
            ).unwrap();
            assert_eq!(observed_records, counted.record_charge());
            assert_eq!(observed_work, counted.work_steps());
            assert_eq!(counted.fingerprint(), fingerprint);
            assert!(fresh()
                .with_page_regions(
                    Vec::new(),
                    &limits,
                    100_000_000,
                    limits.base().get().max_fragments + 1,
                    work
                )
                .is_err());
            assert!(fresh()
                .with_page_regions(Vec::new(), &limits, 100_000_000, u64::MAX, work)
                .is_err());
        },
    )
    .unwrap();
}

#[path = "book_v2_region_failure_budget_tests.rs"]
mod failure_budget;
