//! Independent controls on the observed running-region path. Requested storage
//! and accepted prefixes are measured; allocator capacity and shaping internals
//! are not inferred from these ledgers. All resources enter through the host.
use super::super::super as driver;
use super::*;
use typaxis_core::NodeId;
use typaxis_layout::book_v2::{
    prepare_book_v2_page_region_inlines_counted,
    prepare_book_v2_page_region_inlines_with_observed_source_counted,
    with_budgeted_book_v2_page_region_lines, with_observed_book_v2_page_region_lines,
    BookV2ObservedPageRegionLineBudget, BookV2PageRegionLineBudget, BookV2PageRegionLines,
};
use typaxis_shaping::book_v2::{
    shape_book_v2_page_region_text_counted,
    shape_book_v2_page_region_text_with_observed_source_counted,
    with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as A,
    BookV2ObservedSourceAccounting, BookV2ObservedVerificationError as V,
    BookV2PageRegionTextShape,
};
use typaxis_syntax::book_v2::{
    prepare_book_v2_page_region_text_flow, prepare_book_v2_page_region_text_flow_observed_counted,
    BookV2ObservedSourceFlowObservation as SourceObservation, BookV2PageRegionKind as Kind,
    BookV2PageRegionTextFlow,
};

const PRIOR: [u64; 3] = [7, 11, 13];
const EPOCH: [u8; 32] = [43; 32];
const CONTEXT_FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"));

fn region_data(text: &str, empty: bool, narrow: bool) -> Value {
    let mut d = data(false);
    let end = text.len();
    let span = json!({"source_id":0,"start_byte":0,"end_byte":end});
    let text_node = json!({"kind":"text","node_id":0,"span":span,
        "text_span":{"text_id":0,"start_byte":0,"end_byte":end}});
    let p = |children: Vec<Value>| {
        json!({"kind":"paragraph","node_id":0,
        "classes":[],"span":span,"children":children})
    };
    d["text_buffers"] = json!([{"text_id":0,"utf8":text,"mappings":[{
        "kind":"identity","source_span":span,"text_range":{"start_byte":0,"end_byte":end}}]}]);
    d["document"]["blocks"] = vec![p(vec![text_node.clone()]); 8].into();
    let master = &mut d["page_masters"]["masters"][0];
    master["body"] = json!({"x":20*65536,"y":120*65536,"width":360*65536,"height":48*65536});
    master["header"] =
        json!({"x":10*65536,"y":0,"width":if narrow {14*65536}else{200*65536},"height":96*65536});
    master["footer"] = json!({"x":10*65536,"y":500*65536,"width":80*65536,"height":60*65536});
    master["header_content"] = json!({"node_id":0,"span":span,"blocks":[p(if empty {vec![]} else {vec![
        text_node.clone(),json!({"kind":"soft_break","node_id":0,"span":span}),
        text_node.clone(),json!({"kind":"hard_break","node_id":0,"span":span})]})]});
    master["footer_content"] = json!({"node_id":0,"span":span,"blocks":[{
        "kind":"heading","node_id":0,"span":span,"classes":[],"level":2,
        "anchor_id":null,"children":if empty {vec![]} else {vec![text_node]}}]});
    let mut next = 0;
    renumber(&mut d["document"], &mut next);
    for master in d["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            renumber(&mut master[role], &mut next);
        }
    }
    d
}

fn region_input(
    root: &Root,
    mut d: Value,
    limits: &M4EffectiveResourceLimits,
    font: &[u8],
) -> PreparedBookV2Resources {
    let source = d["text_buffers"][0]["utf8"]
        .as_str()
        .unwrap()
        .as_bytes()
        .to_vec();
    let hex = |bytes: &[u8]| {
        typaxis_core::sha256(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    d["sources"][0]["sha256"] = hex(&source).into();
    d["sources"][0]["utf8_byte_length"] = source.len().into();
    d["resources"]["font_faces"][0]["expected_sha256"] = hex(font).into();
    fs::write(
        root.0.join("document-package.json"),
        serde_json::to_vec(&d).unwrap(),
    )
    .unwrap();
    fs::write(root.0.join("input.tsf"), &source).unwrap();
    fs::write(root.0.join("body.bin"), font).unwrap();
    fs::write(root.0.join("collection.bin"), COLLECTION).unwrap();
    fs::write(root.0.join("cover.bin"), PNG).unwrap();
    let (session, raw) = HostBookV2InputSession::open(
        MachineInputHostOptions::new(
            HostPath::new(root.0.join("document-package.json")).unwrap(),
            None,
        ),
        limits.base(),
    )
    .unwrap();
    let decoded = session
        .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(limits.base()))
        .unwrap();
    let sources = session.admit_sources(&decoded, limits.base()).unwrap();
    let body = prepare_admitted_book_v2_body(
        session.finish(raw, decoded, sources).unwrap(),
        limits.base(),
    )
    .unwrap();
    let input = crate::book_v2_resources::prepare_book_v2_resources(
        body,
        &root.context(),
        &config(limits),
        limits,
    )
    .unwrap();
    assert_eq!(input.body().styled().body().limits(), limits.base());
    assert_eq!(
        input.body().styled().body().wire().text_buffers()[0]
            .utf8
            .as_bytes(),
        source
    );
    assert_eq!(input.resources().fonts()[0].bytes(), font);
    let canonical = typaxis_document_package::book_v2::BookV2DocumentPackageEncoder::new()
        .encode(input.body().styled().body().wire())
        .unwrap();
    assert_eq!(
        input.body().styled().body().canonical_jcs_sha256(),
        typaxis_core::sha256(canonical.as_bytes())
    );
    input
}
fn loan<'cells, R>(
    cells: &'cells mut [u64; 3],
    limits: &'cells ValidatedResourceLimits,
    max_r: u64,
    max_w: u64,
    inspect: impl for<'id> FnOnce(&mut BookV2ObservedSourceAccounting<'id, 'cells>) -> R,
) -> R {
    let [r, s, w] = cells;
    with_book_v2_observed_source_accounting(r, s, w, limits, max_r, max_w, inspect)
}
fn triple(o: SourceObservation) -> [u64; 3] {
    [o.record_charge(), o.spool_charge(), o.work_steps()]
}
fn source_equal(a: &BookV2PageRegionTextFlow<'_>, b: &BookV2PageRegionTextFlow<'_>) {
    assert!(std::ptr::eq(a.source(), b.source()));
    assert!(std::ptr::eq(a.body(), b.body()));
    assert!(std::ptr::eq(a.navigation(), b.navigation()));
    assert_eq!(a.master_id(), b.master_id());
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.fingerprint(), b.fingerprint());
    assert_eq!(a.record_charge(), b.record_charge());
    assert_eq!(a.text_flow().paragraphs(), b.text_flow().paragraphs());
    assert_eq!(a.text_flow().events(), b.text_flow().events());
    assert_eq!(a.text_flow().tables(), b.text_flow().tables());
    assert!(a.text_flow().footnote_definitions().is_empty());
    assert!(a.text_flow().lists().is_empty());
}
fn shape_facts(shape: &BookV2PageRegionTextShape<'_>) -> Value {
    json!({"fingerprint":shape.fingerprint(),"context":shape.line_context_fingerprint(),
        "records":shape.output_records(),"epoch":shape.binding_epoch(),
        "paragraphs":shape.paragraphs().iter().map(|p|json!({"owner":p.owner().get(),
            "font":p.font().map(|f|json!([f.face_id().get(),f.content_hash(),f.face_index(),f.size().get().raw(),f.ascender().raw(),f.descender().raw(),f.line_gap().raw()])),
            "runs":p.runs().iter().map(|r|json!({"owner":r.owner().get(),"language":r.language(),
                "source":format!("{:?}",r.glyph_run().source_span),"font":r.glyph_run().font.get(),
                "glyphs":r.glyph_run().glyphs.iter().map(|g|json!([g.original_gid.get(),g.advance_x.raw(),g.advance_y.raw(),g.offset_x.raw(),g.offset_y.raw()])).collect::<Vec<_>>() })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()})
}
fn rsw(o: BookV2PdfConvergenceObservation) -> [u64; 3] {
    [o.records, o.spool, o.work]
}

fn delivery_data(case: u8) -> Value {
    let mut d = region_data("Result", case == 5, false);
    let span = d["document"]["blocks"][0]["span"].clone();
    if case == 1 {
        let original = d["document"]["blocks"][0].clone();
        d["document"]["blocks"] = vec![original; 14].into();
        for p in d["document"]["blocks"].as_array_mut().unwrap() {
            p["classes"] = json!(["region-named"]);
        }
        let rules = d["style_sheet"]["rules"].as_array_mut().unwrap();
        rules.push(json!({"style_id":"region-named","selector":"paragraph.region-named","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"region-named"}}]}));
        let template = d["page_masters"]["masters"][0].clone();
        for (name, x) in [("named-first", 13), ("named-even", 17), ("named-odd", 19)] {
            let mut m = template.clone();
            m["master_id"] = name.into();
            m["header"]["x"] = (x * 65536).into();
            d["page_masters"]["masters"].as_array_mut().unwrap().push(m);
        }
        d["page_masters"]["selection_rules"] = json!([
            {"master_id":"named-first","parity":"any","first":true,"named_page":"region-named","source_order":0},
            {"master_id":"named-even","parity":"even","first":null,"named_page":"region-named","source_order":1},
            {"master_id":"named-odd","parity":"odd","first":null,"named_page":"region-named","source_order":2}]);
    }
    if case == 2 {
        let blocks = d["document"]["blocks"].as_array_mut().unwrap();
        blocks.insert(
            0,
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
        );
        blocks.extend([
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
            json!({"kind":"page_break","node_id":0,"classes":[],"span":span}),
        ]);
    }
    if case == 3 || case == 4 {
        let mut unused = d["page_masters"]["masters"][0].clone();
        unused["master_id"] = "never-selected-running-owner".into();
        d["page_masters"]["masters"][0]["header_content"] = Value::Null;
        d["page_masters"]["masters"][0]["footer_content"] = Value::Null;
        if case == 4 {
            d["page_masters"]["masters"]
                .as_array_mut()
                .unwrap()
                .push(unused);
        }
    }
    let mut next = 0;
    renumber(&mut d["document"], &mut next);
    for m in d["page_masters"]["masters"].as_array_mut().unwrap() {
        for role in ["header_content", "footer_content"] {
            renumber(&mut m[role], &mut next);
        }
    }
    d["page_masters"]["masters"]
        .as_array_mut()
        .unwrap()
        .sort_by(|left, right| {
            left["master_id"]
                .as_str()
                .unwrap()
                .cmp(right["master_id"].as_str().unwrap())
        });
    d
}

macro_rules! pdf_facts {
    ($roots:expr)=>{{
        use typaxis_display_list::book_v2::BookV2SemanticDisplay;
        let roots=$roots;let display=roots.display();let marked=roots.navigation_bundle().marked();
        let scopes=marked.content().source();
        let regions=display.page_regions().iter().map(|r|json!({"owner":r.owner().get(),"page":r.page_index(),
            "kind":r.kind().as_str(),"frame":format!("{:?}",r.frame()),"layout":r.layout_fingerprint(),"fp":r.fingerprint(),
            "draws":r.draws().iter().map(|d|json!({"owner":d.owner().get(),"paragraph":d.paragraph_owner().get(),
                "line":d.line_index(),"text":d.exact_text(),"span":format!("{:?}",d.source_span()),
                "font":format!("{:?}",d.font()),"bounds":format!("{:?}",d.logical_bounds()),
                "glyphs":d.glyphs().iter().map(|g|json!([g.original_gid().get(),g.x().raw(),g.y().raw()])).collect::<Vec<_>>() })).collect::<Vec<_>>() })).collect::<Vec<_>>();
        let mut running=0;
        for (paint,index) in display.paints().iter().enumerate(){
            if matches!(index,typaxis_display_list::book_v2::BookV2ColumnPaintIndex::PageRegionText{..}) {
                let group=scopes.for_paint(paint).unwrap();assert!(group.artifact().is_some());assert_eq!(group.mcid(),None);
                let gi=scopes.groups().iter().position(|g|std::ptr::eq(g,group)).unwrap();
                assert!(!marked.structure().bindings().iter().any(|b|b.group_index()==gi));
                let s=display.semantic_paint(paint).unwrap().unwrap();assert!(s.artifact().is_some());
                running+=1;
            }
        }
        assert_eq!(running,display.page_regions().iter().map(|r|r.draws().len()).sum::<usize>());
        (roots.pdf().bytes().to_vec(),display.fingerprint(),json!(regions),running)
    }};
}

// An actual typed-kernel companion on the very same issued Display. This is
// test orchestration, not an encoder/counter reimplementation or fake payment.
// The real old helpers publish their own accepted prefixes and typed O phase.
#[allow(clippy::too_many_arguments)]
fn lower_prefix_oracle<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    display: &typaxis_display_list::book_v2::BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    limits: &M4EffectiveResourceLimits,
    maximum: u64,
    before: BookV2PdfConvergenceObservation,
) -> (Result<(), E>, BookV2PdfConvergenceObservation) {
    use driver::{marked_driver, navigation_driver, paints_driver, pdf_driver, relations_driver};
    let mut session = typaxis_resources::book_v2::BookV2FontCommandSession::new(limits, maximum);
    let mut producer = before;
    let mut phase = paints_driver::BookV2ColumnOutputPhase::default();
    let result = (|| {
        paints_driver::with_column_paints_from_session(
            display,
            &mut session,
            limits,
            maximum,
            4,
            [before.records, before.spool, before.output, before.work],
            &mut producer,
            &mut phase,
            |paints, _, encoded, phase, producer| {
                marked_driver::with_column_marked_from_paints(
                    paints,
                    limits,
                    maximum,
                    encoded,
                    producer,
                    phase,
                    |marked, _, observed, phase, producer| {
                        relations_driver::with_column_relations_from_marked(
                            marked,
                            limits,
                            maximum,
                            observed,
                            producer,
                            phase,
                            |relations, _, observed, phase, producer| {
                                navigation_driver::with_column_navigation_from_relations(
                                    relations,
                                    limits,
                                    maximum,
                                    observed,
                                    producer,
                                    phase,
                                    |navigation, _, observed, phase, producer| {
                                        pdf_driver::with_column_pdf_from_navigation(
                                            navigation,
                                            limits,
                                            maximum,
                                            observed,
                                            before,
                                            producer,
                                            phase,
                                            |_, _, _, _, _| Ok(()),
                                        )?
                                    },
                                )?
                            },
                        )?
                    },
                )?
            },
        )?
    })();
    producer.output = phase.accepted_output().unwrap_or(producer.output);
    (result, producer)
}

#[test]
fn book_v2_column_observed_regions_same_readonly_display_reaches_real_pdf_and_keeps_descendant_fees(
) {
    for case in 0..6 {
        let limits = caps();
        let root = Root::new();
        let prepared = region_input(&root, delivery_data(case), &limits, FONT);
        let mut old = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
        let expected = driver::pdf_driver::with_budgeted_book_v2_column_pdf(
            &prepared,
            &limits,
            MODE,
            &mut old,
            4,
            |roots, _, _| pdf_facts!(roots),
        )
        .unwrap();
        let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
        let mut work_boundary = None;
        for callback in 0..3 {
            let mut upstream = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            let mut retained = None;
            let result=driver::with_budgeted_book_v2_column_math(&prepared,&limits,MODE,&mut upstream,|terminals,_,entry|{
                let mut current=entry;
                let mut command=driver::ObservedColumnRegionPdfCommand::new(&limits,MAXIMUM);
                let result=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|a,rest|{
                    let mut view=driver::ColumnProgressView{accounting:a,rest};
                    driver::with_observed_column_display_from_math(terminals,&policy,&prepared,&limits,MODE,&mut view,|display,view|{
                        if case==1 {
                            let original=display.source().original_graph().lines().prepared().source_flow();
                            let pages=display.source().source().geometry().pages();assert!(pages.len()>=3);
                            let mut selected_ids=std::collections::BTreeSet::new();
                            for (index,page) in pages.iter().enumerate(){
                                assert_eq!(page.selection().named_page(),Some("region-named"));
                                let selected=select_book_v2_page_master(original.body(),index as u32,page.selection().named_page(),&mut 0,MAXIMUM).unwrap();
                                let id=selected.master().master_id.as_str();
                                assert_eq!(id,if index==0 {"named-first"}else if (index+1)%2==0 {"named-even"}else{"named-odd"});
                                selected_ids.insert(id.to_owned());
                                for region in display.page_regions().iter().filter(|r|r.page_index()==index as u32){
                                    let (source,frame)=match region.kind(){
                                        Kind::Header=>(selected.advanced().header_content.as_ref().unwrap(),selected.master().header.unwrap()),
                                        Kind::Footer=>(selected.advanced().footer_content.as_ref().unwrap(),selected.master().footer.unwrap()),
                                    };
                                    assert_eq!(region.owner().get(),source.node_id);
                                    assert_eq!([region.frame().x().raw(),region.frame().y().raw(),region.frame().width().get().raw(),region.frame().height().get().raw()],
                                        [frame.x,frame.y,frame.width,frame.height]);
                                }
                            }
                            assert_eq!(selected_ids.len(),3);
                        }
                        let after_attach=view.snapshot();let mut last_pdf=None;
                        let mut wrong=driver::ObservedColumnRegionPdfCommand::new(&limits,MAXIMUM-1);
                        let refused: Result<(),E>=driver::with_observed_column_pdf_from_display(display,&mut wrong,&limits,4,view,|_|panic!("wrong real command maximum cannot issue PDF"));
                        assert!(matches!(refused,Err(E::Identity)));assert_eq!(view.snapshot(),after_attach);
                        for retry in 0..2 {
                            let actual=driver::with_observed_column_pdf_from_display(display,&mut command,&limits,4,view,|roots|{
                                assert!(std::ptr::eq(roots.display(),display));
                                last_pdf=Some([roots.pdf().record_charge(),roots.pdf().spool_charge(),roots.pdf().output_charge(),roots.pdf().work_steps()]);
                                Ok(pdf_facts!(roots))
                            })?;
                            assert_eq!(actual,expected);
                            let issued=last_pdf.unwrap();assert_eq!([view.snapshot().records,view.snapshot().spool,view.snapshot().output,view.snapshot().work],issued);
                            if callback==0 && retry==0 {work_boundary=Some((after_attach.work,issued[3]));}
                            assert!(view.snapshot().records>after_attach.records && view.snapshot().spool>after_attach.spool && view.snapshot().work>after_attach.work);
                            eprintln!("region observed same-root PDF case={case} retry={retry} quartet={issued:?}");
                        }
                        let mut issued_error=None;
                        let bridge_error=driver::with_observed_column_pdf_from_display(display,&mut command,&limits,4,view,|roots|{
                            issued_error=Some([roots.pdf().record_charge(),roots.pdf().spool_charge(),roots.pdf().output_charge(),roots.pdf().work_steps()]);
                            Err::<(),E>(E::Limit("closed region PDF consumer sentinel"))
                        });
                        assert!(matches!(bridge_error,Err(E::Limit("closed region PDF consumer sentinel"))));
                        assert_eq!([view.snapshot().records,view.snapshot().spool,view.snapshot().output,view.snapshot().work],issued_error.unwrap());
                        assert_eq!((view.snapshot().candidates,view.snapshot().balance_candidates,view.snapshot().width_passes,view.snapshot().width_refinements,view.snapshot().page_passes),
                            (after_attach.candidates,after_attach.balance_candidates,after_attach.width_passes,after_attach.width_refinements,after_attach.page_passes));
                        if !display.page_regions().is_empty() {
                            let original=display.source().original_graph().lines().prepared().source_flow();
                            let named=display.source().source().geometry().pages()[0].selection().named_page();
                            let selected=select_book_v2_page_master(original.body(),0,named,&mut 0,MAXIMUM).unwrap();
                            let before=view.snapshot();let prior=rsw(before);let mut observed=SourceObservation::default();
                            prepare_book_v2_page_region_text_flow_observed_counted(selected,Kind::Header,original.navigation(),
                                prior[0],prior[1],prior[2],view.accounting.maximum_records(),MAXIMUM,&mut observed).unwrap();
                            view.accounting.prepare_region_source(selected,Kind::Header,original.navigation()).unwrap();
                            assert_eq!(view.accounting.counters(),triple(observed));assert_eq!(view.snapshot().output,before.output);
                        }
                        retained=Some(view.snapshot());
                        if callback==2 {let borrowed=&mut *view;std::mem::forget(borrowed);}
                        if callback==1 {Err(E::Limit("region display application sentinel"))}else{Ok(())}
                    })
                });
                assert_eq!(current,retained.unwrap());result
            }).unwrap();
            assert_eq!(result.is_ok(), callback != 1);
        }
        if case == 0 {
            let (attach_work, pdf_work) = work_boundary.unwrap();
            assert!(pdf_work > attach_work);
            let maximum = pdf_work - 1;
            let mut upstream = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
            driver::with_budgeted_book_v2_column_math(&prepared,&limits,MODE,&mut upstream,|terminals,_,entry|{
                assert!(entry.work<=maximum);let mut current=entry;let mut expected_after=None;
                let result=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,maximum,|a,rest|{
                    let mut view=driver::ColumnProgressView{accounting:a,rest};
                    driver::with_observed_column_display_from_math(terminals,&policy,&prepared,&limits,MODE,&mut view,|display,view|{
                        assert_eq!(view.snapshot().work,attach_work);
                        let before=view.snapshot();let (oracle,expected)=lower_prefix_oracle(display,&limits,maximum,before);
                        let oracle=oracle.err().expect("real one-short lower producer refusal");
                        let mut command=driver::ObservedColumnRegionPdfCommand::new(&limits,maximum);
                        let actual: Result<(),E>=driver::with_observed_column_pdf_from_display(display,&mut command,&limits,4,view,
                            |_|panic!("one-short producer cannot issue PDF"));
                        let actual=actual.err().unwrap();assert_eq!(format!("{actual:?}"),format!("{oracle:?}"));
                        assert!(format!("{actual:?}").to_lowercase().contains("work"));
                        assert_eq!(view.snapshot(),expected);expected_after=Some(expected);
                        assert!(expected.records>=before.records && expected.spool>=before.spool && expected.work>=before.work);
                        eprintln!("region genuine lower Work refusal before={before:?} accepted={expected:?} primary={actual:?}");
                        Ok(())
                    })
                });result.unwrap();assert_eq!(current,expected_after.unwrap());
            }).unwrap();
        }
    }
}

#[test]
fn book_v2_column_observed_regions_source_matches_frozen_syntax_all_three_and_pointer_verification()
{
    let limits = caps();
    for (text, empty) in [("Result", false), ("É日本🙂", false), ("Result", true)] {
        let root = Root::new();
        let input = region_input(&root, region_data(text, empty, false), &limits, FONT);
        let body = input.body().styled();
        let nav = prepare_book_v2_navigation(body).unwrap();
        let other = prepare_book_v2_navigation(body).unwrap();
        let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
        for kind in [Kind::Header, Kind::Footer] {
            let mut observation = SourceObservation::default();
            let expected = prepare_book_v2_page_region_text_flow_observed_counted(
                selected,
                kind,
                &nav,
                PRIOR[0],
                PRIOR[1],
                PRIOR[2],
                limits.base().get().max_fragments,
                MAXIMUM,
                &mut observation,
            )
            .unwrap();
            let mut cells = PRIOR;
            loan(
                &mut cells,
                limits.base(),
                limits.base().get().max_fragments,
                MAXIMUM,
                |a| {
                    let actual = a.prepare_region_source(selected, kind, &nav).unwrap();
                    source_equal(&actual, &expected);
                    let legacy =
                        prepare_book_v2_page_region_text_flow(selected, kind, &nav, PRIOR[0])
                            .unwrap();
                    source_equal(&actual, &legacy);
                    let logical = 1 + actual
                        .text_flow()
                        .paragraphs()
                        .iter()
                        .map(|p| 4 + p.items().len() as u64)
                        .sum::<u64>();
                    assert_eq!(actual.record_charge(), PRIOR[0] + logical);
                    assert_eq!(observation.logical_source_record_charge(), logical);
                    assert!(a.counters()[0] > actual.record_charge());
                    assert_eq!(a.counters(), triple(observation));
                    let before = a.counters();
                    actual.verify_for(body, &nav).unwrap();
                    assert!(actual.verify_for(body, &other).is_err());
                    assert_eq!(a.counters(), before);
                    for p in actual.text_flow().paragraphs() {
                        for item in p.items() {
                            if let typaxis_syntax::ProductionInlineContent::Text { utf8, span } =
                                item.content()
                            {
                                let original = &body.body().wire().text_buffers()
                                    [span.text_id().get() as usize]
                                    .utf8;
                                assert_eq!(
                                    utf8.as_ptr(),
                                    original[span.start_byte().get() as usize..].as_ptr()
                                );
                            }
                        }
                    }
                },
            );
            assert_eq!(cells, triple(observation));
        }
    }
}

#[test]
fn book_v2_column_observed_regions_source_exact_short_and_failed_retry_match_real_oracle() {
    let limits = caps();
    let root = Root::new();
    let input = region_input(&root, region_data("Result", false, false), &limits, FONT);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let other_root = Root::new();
    let other_input = region_input(
        &other_root,
        region_data("Result", false, false),
        &limits,
        FONT,
    );
    let other = prepare_book_v2_navigation(other_input.body().styled()).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let mut complete = SourceObservation::default();
    prepare_book_v2_page_region_text_flow_observed_counted(
        selected,
        Kind::Header,
        &nav,
        PRIOR[0],
        PRIOR[1],
        PRIOR[2],
        limits.base().get().max_fragments,
        MAXIMUM,
        &mut complete,
    )
    .unwrap();
    let final_cells = triple(complete);
    let max_s = limits.base().get().max_spool_bytes;
    for (navigation, prior, max_r, max_w) in [
        (&nav, PRIOR, final_cells[0], MAXIMUM),
        (&nav, PRIOR, final_cells[0] - 1, MAXIMUM),
        (
            &nav,
            [PRIOR[0], max_s - (final_cells[1] - PRIOR[1]), PRIOR[2]],
            limits.base().get().max_fragments,
            MAXIMUM,
        ),
        (
            &nav,
            [PRIOR[0], max_s - (final_cells[1] - PRIOR[1]) + 1, PRIOR[2]],
            limits.base().get().max_fragments,
            MAXIMUM,
        ),
        (
            &nav,
            PRIOR,
            limits.base().get().max_fragments,
            final_cells[2],
        ),
        (
            &nav,
            PRIOR,
            limits.base().get().max_fragments,
            final_cells[2] - 1,
        ),
        (&other, PRIOR, limits.base().get().max_fragments, MAXIMUM),
        (
            &nav,
            [
                limits.base().get().max_fragments + 1,
                max_s + 1,
                MAXIMUM + 1,
            ],
            limits.base().get().max_fragments,
            MAXIMUM,
        ),
    ] {
        let mut expected = SourceObservation::default();
        let oracle = prepare_book_v2_page_region_text_flow_observed_counted(
            selected,
            Kind::Header,
            navigation,
            prior[0],
            prior[1],
            prior[2],
            max_r,
            max_w,
            &mut expected,
        );
        let mut cells = prior;
        loan(&mut cells, limits.base(), max_r, max_w, |a| {
            let actual = a.prepare_region_source(selected, Kind::Header, navigation);
            assert_eq!(
                actual.as_ref().err().cloned(),
                oracle.as_ref().err().copied().map(V::Source)
            );
            assert_eq!(a.counters(), triple(expected));
            if let (Ok(actual), Ok(expected)) = (&actual, &oracle) {
                source_equal(actual, expected);
            }
        });
        assert_eq!(cells, triple(expected));
    }
    let mut cells = PRIOR;
    loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| {
            for _ in 0..2 {
                let before = a.counters();
                let mut expected = SourceObservation::default();
                let err = prepare_book_v2_page_region_text_flow_observed_counted(
                    selected,
                    Kind::Footer,
                    &other,
                    before[0],
                    before[1],
                    before[2],
                    a.maximum_records(),
                    a.maximum_work(),
                    &mut expected,
                )
                .err()
                .unwrap();
                assert_eq!(
                    a.prepare_region_source(selected, Kind::Footer, &other)
                        .err(),
                    Some(V::Source(err))
                );
                assert_eq!(a.counters(), triple(expected));
                assert!(a.counters()[2] > before[2]);
            }
        },
    );
}

#[test]
fn book_v2_column_observed_regions_shape_uses_actual_original_glyphs_and_fresh_output_limit() {
    let limits = caps();
    let root = Root::new();
    let input = region_input(&root, region_data("Result", false, false), &limits, FONT);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let flow = prepare_book_v2_page_region_text_flow(selected, Kind::Header, &nav, 0).unwrap();
    let mut old_r = 0;
    let old = shape_book_v2_page_region_text_counted(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        None,
        &mut old_r,
    )
    .unwrap();
    assert_eq!(old.output_records(), old_r);
    assert!(old_r > 0);
    for maximum in [
        PRIOR[0] + old_r + input.resources().fonts().len() as u64,
        PRIOR[0] + old_r + input.resources().fonts().len() as u64 - 1,
    ] {
        let mut cells = PRIOR;
        loan(&mut cells, limits.base(), maximum, MAXIMUM, |accounting| {
            accounting.with_zero_intrinsic_records(|a, window| {
                let mut actual_r = 0;
                let actual = shape_book_v2_page_region_text_with_observed_source_counted(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    EPOCH,
                    None,
                    a,
                    window,
                    &mut actual_r,
                );
                assert!(actual_r <= old_r);
                let after_shape = a.counters();
                assert_eq!(
                    after_shape[0],
                    PRIOR[0] + actual_r + input.resources().fonts().len() as u64
                );
                assert!(after_shape[1] > PRIOR[1] && after_shape[2] > PRIOR[2]);
                if maximum == PRIOR[0] + old_r + input.resources().fonts().len() as u64 {
                    let actual = actual.unwrap();
                    assert_eq!(shape_facts(&actual), shape_facts(&old));
                    assert!(std::ptr::eq(actual.flow(), &flow));
                    actual
                        .verify(&flow, input.resources(), &limits, EPOCH)
                        .unwrap();
                    assert!(actual
                        .paragraphs()
                        .iter()
                        .flat_map(|p| p.runs())
                        .flat_map(|r| &r.glyph_run().glyphs)
                        .all(|g| g.original_gid.get() != 0));
                } else {
                    assert!(matches!(
                        actual,
                        Err(
                            typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError::Shape(
                                typaxis_shaping::ProductionTextShapeError {
                                    kind:
                                        typaxis_shaping::ProductionTextShapeErrorKind::OutputLimit,
                                    ..
                                }
                            )
                        )
                    ));
                }
                window.seal();
                assert_eq!(a.intrinsic_limit(window), Err(A::ClosedWindow));
            });
        });
    }
    // Earlier genuine affiliation failures precede the new accounting domain
    // and bad floors. This is the actual legacy kernel's first-cause order.
    let other_root = Root::new();
    let other = region_input(
        &other_root,
        region_data("Result", false, false),
        &limits,
        FONT,
    );
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    for (policy, epoch) in [(&other_policy, EPOCH), (&policy, [0; 32])] {
        let prior = [
            limits.base().get().max_fragments + 1,
            limits.base().get().max_spool_bytes + 1,
            MAXIMUM + 1,
        ];
        let mut cells = prior;
        loan(
            &mut cells,
            limits.base(),
            limits.base().get().max_fragments,
            MAXIMUM,
            |a| {
                a.with_zero_intrinsic_records(|a, w| {
                    let mut report = 99;
                    let error = shape_book_v2_page_region_text_with_observed_source_counted(
                        policy,
                        &flow,
                        input.resources(),
                        &limits,
                        epoch,
                        None,
                        a,
                        w,
                        &mut report,
                    )
                    .err()
                    .unwrap();
                    assert!(matches!(
                        error,
                        typaxis_shaping::book_v2::BookV2ObservedPageRegionShapeError::Shape(
                            typaxis_shaping::ProductionTextShapeError {
                                kind:
                                    typaxis_shaping::ProductionTextShapeErrorKind::ReceiptMismatch,
                                ..
                            }
                        )
                    ));
                    assert_eq!(report, 0);
                    assert_eq!(a.counters(), prior);
                });
            },
        );
    }
}

#[test]
fn book_v2_column_observed_regions_two_phase_feedback_preserves_guard_and_consumption_order() {
    use typaxis_linebreak::{
        BreakError, LineLayoutContext, LineLayoutStateFingerprint, LineReshapeFeedback,
        LineReshapeObservation,
    };
    let limits = caps();
    let initial = LineLayoutStateFingerprint::from_canonical_bytes(b"{\"state\":0}").unwrap();
    let changed = LineLayoutStateFingerprint::from_canonical_bytes(b"{\"state\":1}").unwrap();
    let mut old = LineReshapeFeedback::new(initial);
    let mut new = LineReshapeFeedback::new(initial);
    let mut old_b = LineLayoutContext::from_limits(limits.base())
        .take_budget()
        .unwrap();
    let mut new_b = LineLayoutContext::from_limits(limits.base())
        .take_budget()
        .unwrap();
    for (index, outcome) in [(1, changed), (2, changed)] {
        let p = old.begin_pass(&mut old_b).unwrap();
        let old_outcome = p.complete(outcome).unwrap();
        let request = new.prepare_pass_request(&mut new_b).unwrap();
        assert_eq!(request.requested_logical_target().unwrap(), index);
        let p = request.reserve().unwrap();
        assert_eq!(p.pass_index() as usize, index);
        assert_eq!(p.complete(outcome).unwrap(), old_outcome);
        assert_eq!(old, new);
        assert_eq!(
            old_b.remaining_reshape_passes(),
            new_b.remaining_reshape_passes()
        );
        assert_eq!(
            old_outcome,
            if index == 1 {
                LineReshapeObservation::RebreakRequired
            } else {
                LineReshapeObservation::Stable
            }
        );
    }
    let remaining = new_b.remaining_reshape_passes();
    assert_eq!(
        new.prepare_pass_request(&mut new_b).err(),
        Some(BreakError::ReshapeTerminal)
    );
    assert_eq!(new_b.remaining_reshape_passes(), remaining);
    // A payment refusal is deliberate allowance arithmetic. It proves that
    // the actual controller's begun pass survives; it is not an allocator test.
    let mut feedback = LineReshapeFeedback::new(initial);
    let mut budget = LineLayoutContext::from_limits(limits.base())
        .take_budget()
        .unwrap();
    let remaining = budget.remaining_reshape_passes();
    {
        let request = feedback.prepare_pass_request(&mut budget).unwrap();
        let target = request.requested_logical_target().unwrap();
        assert_eq!(target, 1);
        let mut cells = PRIOR;
        loan(&mut cells, limits.base(), PRIOR[0], MAXIMUM, |a| {
            assert!(matches!(
                a.debit(
                    NodeId::new(0),
                    [
                        1,
                        (target * std::mem::size_of::<typaxis_linebreak::LineReshapePassRecord>())
                            as u64,
                        1
                    ]
                ),
                Err(A::Records { .. })
            ));
            assert_eq!(a.counters(), PRIOR);
        });
        drop(request);
    }
    assert_eq!(budget.remaining_reshape_passes(), remaining - 1);
    assert!(feedback.records().is_empty());
    let request = feedback.prepare_pass_request(&mut budget).unwrap();
    let permit = request.reserve().unwrap();
    assert_eq!(permit.pass_index(), 2);
    drop(permit);
    let remaining = budget.remaining_reshape_passes();
    assert_eq!(
        feedback.prepare_pass_request(&mut budget).err(),
        Some(BreakError::ReshapePassInFlight)
    );
    assert_eq!(budget.remaining_reshape_passes(), remaining);
}

#[test]
fn book_v2_column_observed_regions_inline_local_records_match_legacy_and_charge_only_current_graph()
{
    let limits = caps();
    let root = Root::new();
    let input = region_input(&root, region_data("Result", false, false), &limits, FONT);
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let flow = prepare_book_v2_page_region_text_flow(selected, Kind::Header, &nav, 0).unwrap();
    let mut shaped_r = 0;
    let shaped = shape_book_v2_page_region_text_counted(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        None,
        &mut shaped_r,
    )
    .unwrap();
    let mut old_r = 0;
    let old = prepare_book_v2_page_region_inlines_counted(
        &flow,
        &shaped,
        input.resources(),
        &limits,
        EPOCH,
        MODE,
        0,
        &mut old_r,
    )
    .unwrap();
    assert_eq!(old.record_charge(), old_r);
    assert!(old_r > shaped_r);
    for max_r in [PRIOR[0] + old_r, PRIOR[0] + old_r - 1] {
        let mut cells = PRIOR;
        loan(&mut cells, limits.base(), max_r, MAXIMUM, |a| {
            a.with_zero_intrinsic_records(|a, window| {
                let mut reported = 0;
                let result = prepare_book_v2_page_region_inlines_with_observed_source_counted(
                    &flow,
                    &shaped,
                    input.resources(),
                    &limits,
                    EPOCH,
                    MODE,
                    a,
                    window,
                    &mut reported,
                );
                assert_eq!(a.counters(), [PRIOR[0] + reported, PRIOR[1], PRIOR[2]]);
                assert!(reported >= shaped_r && reported <= old_r);
                if max_r == PRIOR[0] + old_r {
                    let actual = result.unwrap();
                    assert_eq!(actual.fingerprint(), old.fingerprint());
                    assert_eq!(actual.record_charge(), old_r);
                    actual.verify(&flow, &shaped).unwrap();
                    assert!(std::ptr::eq(actual.flow(), &flow));
                    assert!(std::ptr::eq(actual.shaped(), &shaped));
                    assert_eq!(actual.paragraphs().len(), old.paragraphs().len());
                    for (a, b) in actual.paragraphs().iter().zip(old.paragraphs()) {
                        assert_eq!(a.owner(), b.owner());
                        assert_eq!(a.line_height(), b.line_height());
                        assert_eq!(
                            a.items().map(|i| i.fingerprint()),
                            b.items().map(|i| i.fingerprint())
                        );
                        assert_eq!(a.anchors(), b.anchors());
                        assert_eq!(
                            format!("{:?}", a.glyph_clusters()),
                            format!("{:?}", b.glyph_clusters())
                        );
                    }
                    // A smaller local output after this genuine staged payment
                    // never refunds its already accepted larger prefix.
                    let before = a.counters();
                    a.retain_intrinsic_output(window, NodeId::new(flow.source().node_id), shaped_r)
                        .unwrap();
                    assert_eq!(a.counters(), before);
                    assert_eq!(window.paid_records(), old_r);
                } else {
                    assert!(result.is_err());
                    assert!(reported < old_r);
                }
            });
        });
    }
    // A stale epoch is an affiliation failure before any observed debit.
    let mut cells = PRIOR;
    loan(&mut cells, limits.base(), PRIOR[0], MAXIMUM, |a| {
        a.with_zero_intrinsic_records(|a, w| {
            let mut report = 99;
            let error = prepare_book_v2_page_region_inlines_with_observed_source_counted(
                &flow,
                &shaped,
                input.resources(),
                &limits,
                [0; 32],
                MODE,
                a,
                w,
                &mut report,
            )
            .err()
            .unwrap();
            assert!(format!("{error:?}").contains("ReceiptMismatch"));
            assert_eq!(report, 0);
            assert_eq!(a.counters(), PRIOR);
        });
    });
}

fn selected_facts(selected: Option<&typaxis_linebreak::ProductionInlineBreak>) -> Option<Value> {
    selected.map(|selected| {
        json!({
            "canonical": selected.canonical_jcs(),
            "fingerprint": selected.fingerprint(),
            "paragraph": selected.paragraph_fingerprint(),
            "lines": selected.lines().iter().map(|line| {
                let actual = line.line();
                let metrics = actual.metrics();
                json!({
                    "width": line.inline_size().get().raw(),
                    "shift": line.origin_shift().get().raw(),
                    "required": line.required_inline_size().get().raw(),
                    "index": actual.line_index(),
                    "start": actual.start_unit(), "end": actual.end_unit(),
                    "advance": actual.logical_advance().get().raw(),
                    "left": actual.visual_left().map(|x| x.raw()),
                    "right": actual.visual_right().map(|x| x.raw()),
                    "ascent": metrics.content_ascent().get().raw(),
                    "descent": metrics.content_descent().get().raw(),
                    "leading_before": metrics.leading_before().get().raw(),
                    "leading_after": metrics.leading_after().get().raw(),
                    "height": metrics.line_height().get().raw(),
                    "break_kind": format!("{:?}", actual.break_kind()),
                    "penalty": actual.break_penalty(), "demerits": actual.break_demerits(),
                    "pens": (actual.start_unit()..actual.end_unit())
                        .map(|unit| line.unit_pen_x(unit).map(|x| x.raw())).collect::<Vec<_>>(),
                    "occurrences": actual.occurrences().iter().map(|item| json!({
                        "unit": item.unit_index(), "item": format!("{:?}", item.item()),
                        "pen": item.pen_x().raw(), "before": item.spacing_before().get().raw(),
                        "after": item.spacing_after().get().raw()
                    })).collect::<Vec<_>>()
                })
            }).collect::<Vec<_>>()
        })
    })
}

fn line_facts(lines: &BookV2PageRegionLines<'_, '_>) -> Value {
    json!({"fingerprint":lines.fingerprint(),"shape":shape_facts(lines.prepared().shaped()),
        "inline":lines.prepared().fingerprint(),"frame":format!("{:?}",lines.frame()),
        "height":lines.content_height().get().raw(),"origins":format!("{:?}",lines.origins()),
        "paragraphs":lines.paragraphs().iter().map(|p|json!({"owner":p.owner().get(),
            "width":p.inline_size().get().raw(),"selected":selected_facts(p.selected()),
            "lines":format!("{:?}",p.lines()),"anchors":format!("{:?}",p.anchors())})).collect::<Vec<_>>()})
}

#[test]
fn book_v2_column_observed_regions_real_rebreak_and_every_axis_keep_current_prefixes() {
    let limits = caps();
    let root = Root::new();
    let input = region_input(
        &root,
        region_data("f i", false, true),
        &limits,
        CONTEXT_FONT,
    );
    let body = input.body().styled();
    let nav = prepare_book_v2_navigation(body).unwrap();
    let selected = select_book_v2_page_master(body, 0, None, &mut 0, MAXIMUM).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
    let flow = prepare_book_v2_page_region_text_flow(selected, Kind::Header, &nav, 0).unwrap();
    let mut legacy =
        BookV2PageRegionLineBudget::new(MAXIMUM, limits.base().get().max_line_reshape_passes);
    let expected = with_budgeted_book_v2_page_region_lines(
        &policy,
        &flow,
        input.resources(),
        &limits,
        EPOCH,
        MODE,
        selected,
        &mut legacy,
        0,
        |stable| {
            assert!(
                stable.passes().len() >= 2,
                "genuine original contextual-font rebreak"
            );
            let glyphs = stable
                .lines()
                .paragraphs()
                .iter()
                .flat_map(|p| p.lines())
                .flat_map(|l| l.items())
                .filter_map(|i| {
                    if let typaxis_layout::ProductionPlacedInline::Text(c) = i {
                        Some(c)
                    } else {
                        None
                    }
                })
                .flat_map(|c| c.glyphs())
                .map(|g| g.glyph().original_gid.get())
                .collect::<Vec<_>>();
            assert!(glyphs.contains(&71));
            assert!(!glyphs.contains(&59));
            (line_facts(stable.lines()), format!("{:?}", stable.passes()))
        },
    )
    .unwrap();
    let mut local = BookV2ObservedPageRegionLineBudget::new(
        MAXIMUM,
        limits.base().get().max_line_reshape_passes,
    );
    let mut complete = PRIOR;
    loan(
        &mut complete,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| {
            let actual = with_observed_book_v2_page_region_lines(
                &policy,
                &flow,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                selected,
                &mut local,
                a,
                |stable, _| (line_facts(stable.lines()), format!("{:?}", stable.passes())),
            )
            .unwrap();
            assert_eq!(actual, expected);
        },
    );
    assert_eq!(local.candidate_steps(), legacy.candidate_steps());
    assert_eq!(local.reshape_passes(), legacy.reshape_passes());
    assert!(complete.iter().zip(PRIOR).all(|(a, b)| *a > b));
    let delta = [
        complete[0] - PRIOR[0],
        complete[1] - PRIOR[1],
        complete[2] - PRIOR[2],
    ];
    let max_s = limits.base().get().max_spool_bytes;
    for (axis, short) in (0..3).flat_map(|axis| [(axis, false), (axis, true)]) {
        let mut cells = PRIOR;
        let max_r = if axis == 0 {
            complete[0] - u64::from(short)
        } else {
            limits.base().get().max_fragments
        };
        if axis == 1 {
            cells[1] = max_s - delta[1] + u64::from(short);
        }
        let max_w = if axis == 2 {
            complete[2] - u64::from(short)
        } else {
            MAXIMUM
        };
        let prior = cells;
        let mut local = BookV2ObservedPageRegionLineBudget::new(
            MAXIMUM,
            limits.base().get().max_line_reshape_passes,
        );
        let result = loan(&mut cells, limits.base(), max_r, max_w, |a| {
            with_observed_book_v2_page_region_lines(
                &policy,
                &flow,
                input.resources(),
                &limits,
                EPOCH,
                MODE,
                selected,
                &mut local,
                a,
                |stable, _| line_facts(stable.lines()),
            )
        });
        if short {
            assert!(result.is_err(), "axis {axis} genuine one-short must refuse");
        } else {
            assert_eq!(result.unwrap(), expected.0);
            assert_eq!(
                cells,
                [
                    prior[0] + delta[0],
                    prior[1] + delta[1],
                    prior[2] + delta[2]
                ]
            );
        }
        assert!(cells.iter().zip(prior).all(|(a, b)| *a >= b));
        assert!(cells[0] <= max_r && cells[1] <= max_s && cells[2] <= max_w);
        eprintln!("region-lines axis={axis} short={short} before={prior:?} accepted={cells:?} passes={} candidates={}",local.reshape_passes(),local.candidate_steps());
    }
    for short in [false, true] {
        let mut local = BookV2ObservedPageRegionLineBudget::new(
            legacy.candidate_steps() - u64::from(short),
            legacy.reshape_passes(),
        );
        let mut cells = PRIOR;
        let result = loan(
            &mut cells,
            limits.base(),
            limits.base().get().max_fragments,
            MAXIMUM,
            |a| {
                with_observed_book_v2_page_region_lines(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    EPOCH,
                    MODE,
                    selected,
                    &mut local,
                    a,
                    |stable, _| line_facts(stable.lines()),
                )
            },
        );
        assert_eq!(result.is_ok(), !short);
        assert_eq!(local.remaining_steps(), 0);
        if !short {
            assert_eq!(result.unwrap(), expected.0);
        }
    }
    eprintln!("region-lines original=context-font genuine-rebreak={} candidates={} before={PRIOR:?} after={complete:?}",legacy.reshape_passes(),legacy.candidate_steps());
    // The callback runs after the graph window is closed and spends on the
    // same actual three cells. Its descendant source fees survive all Results.
    for mode in 0..3 {
        let mut cells = PRIOR;
        let mut expected_after = None;
        let outcome = loan(
            &mut cells,
            limits.base(),
            limits.base().get().max_fragments,
            MAXIMUM,
            |a| {
                let mut local = BookV2ObservedPageRegionLineBudget::new(
                    MAXIMUM,
                    limits.base().get().max_line_reshape_passes,
                );
                with_observed_book_v2_page_region_lines(
                    &policy,
                    &flow,
                    input.resources(),
                    &limits,
                    EPOCH,
                    MODE,
                    selected,
                    &mut local,
                    a,
                    |stable, a| {
                        let before = a.counters();
                        assert_eq!(line_facts(stable.lines()), expected.0);
                        let mut observed = SourceObservation::default();
                        prepare_book_v2_page_region_text_flow_observed_counted(
                            selected,
                            Kind::Footer,
                            &nav,
                            before[0],
                            before[1],
                            before[2],
                            a.maximum_records(),
                            a.maximum_work(),
                            &mut observed,
                        )
                        .unwrap();
                        a.prepare_region_source(selected, Kind::Footer, &nav)
                            .unwrap();
                        assert_eq!(a.counters(), triple(observed));
                        expected_after = Some(a.counters());
                        if mode == 2 {
                            let reference = &mut *a;
                            std::mem::forget(reference);
                        }
                        if mode == 1 {
                            Err(E::Limit("region callback sentinel"))
                        } else {
                            Ok(())
                        }
                    },
                )
            },
        )
        .unwrap();
        assert_eq!(outcome.is_ok(), mode != 1);
        assert_eq!(cells, expected_after.unwrap());
    }
    let overflow_root = Root::new();
    let mut d = region_data("f i", false, true);
    d["page_masters"]["masters"][0]["header"]["height"] = (1 * 65536).into();
    let overflow_input = region_input(&overflow_root, d, &limits, CONTEXT_FONT);
    let b = overflow_input.body().styled();
    let n = prepare_book_v2_navigation(b).unwrap();
    let selected = select_book_v2_page_master(b, 0, None, &mut 0, MAXIMUM).unwrap();
    let f = prepare_book_v2_page_region_text_flow(selected, Kind::Header, &n, 0).unwrap();
    let p = prepare_book_v2_resource_policy(overflow_input.body(), &limits).unwrap();
    let mut local = BookV2ObservedPageRegionLineBudget::new(
        MAXIMUM,
        limits.base().get().max_line_reshape_passes,
    );
    let mut cells = PRIOR;
    let result: Result<(), _> = loan(
        &mut cells,
        limits.base(),
        limits.base().get().max_fragments,
        MAXIMUM,
        |a| {
            with_observed_book_v2_page_region_lines(
                &p,
                &f,
                overflow_input.resources(),
                &limits,
                EPOCH,
                MODE,
                selected,
                &mut local,
                a,
                |_, _| panic!("height overflow must not issue stable view"),
            )
        },
    );
    assert!(matches!(
        result,
        Err(
            typaxis_layout::book_v2::BookV2ObservedPageRegionLayoutError::Region(
                typaxis_layout::book_v2::BookV2PageRegionLayoutError::Overflow { .. }
            )
        )
    ));
    assert!(local.reshape_passes() >= 2);
    assert!(cells.iter().zip(PRIOR).all(|(a, b)| *a > b));
}

#[test]
fn book_v2_column_observed_regions_attach_empty_fastpath_checks_domain_then_floors_without_new_work(
) {
    use typaxis_display_list::book_v2::{BookV2ColumnDisplayBuilder, BookV2ResourceDisplay};
    let limits = caps();
    let root = Root::new();
    let prepared = input(&root, &data(false), &limits);
    let policy = prepare_book_v2_resource_policy(prepared.body(), &limits).unwrap();
    let other_root = Root::new();
    let other = input(&other_root, &data(false), &limits);
    let other_policy = prepare_book_v2_resource_policy(other.body(), &limits).unwrap();
    let mut parent = BookV2ColumnPageBudget::new(&limits, MAXIMUM);
    driver::with_budgeted_book_v2_column_math(&prepared,&limits,MODE,&mut parent,|terminals,_,entry|{
        for axis in 0..3 {
            let actual=[terminals.record_charge(),terminals.spool_charge(),terminals.work_steps()];
            if actual[axis]==0 {continue;}
            let mut current=entry;
            match axis {0=>current.records=actual[0]-1,1=>current.spool=actual[1]-1,_=>current.work=actual[2]-1};
            let unpaid=current;
            let result: Result<(),E>=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|a,rest|{
                let mut view=driver::ColumnProgressView{accounting:a,rest};
                driver::with_observed_column_display_from_math(terminals,&policy,&prepared,&limits,MODE,&mut view,
                    |_,_|panic!("unpaid genuine terminal cannot construct Display"))
            });
            assert!(matches!(result,Err(E::Identity)));assert_eq!(current,unpaid);
        }
        let fresh=||BookV2ColumnDisplayBuilder::new(terminals,prepared.resources(),&limits,MAXIMUM,entry.records,entry.work)
            .unwrap().build().unwrap();
        let display=fresh();assert!(display.page_regions().is_empty());
        let fp=display.fingerprint();let paint_count=display.paints().len();
        let mut current=BookV2PdfConvergenceObservation{records:display.record_charge(),work:display.work_steps(),..entry};
        let before=current;
        driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|accounting,rest|{
            let mut view=driver::ColumnProgressView{accounting,rest};
            let display=driver::column_page_region_driver::attach_observed(display,&policy,&limits,MODE,&mut view).unwrap();
            assert_eq!(display.fingerprint(),fp);assert_eq!(display.paints().len(),paint_count);
            assert!(display.page_regions().is_empty());assert_eq!(view.snapshot(),before);
        });assert_eq!(current,before);
        for axis in 0..3 {
            let display=fresh();let actual=[display.record_charge(),display.source_spool_charge(),display.work_steps()];
            if actual[axis]==0 {continue;}
            let mut current=before;
            match axis {0=>current.records=actual[0]-1,1=>current.spool=actual[1]-1,_=>current.work=actual[2]-1};
            let unpaid=current;
            let result=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|a,rest|{
                let mut view=driver::ColumnProgressView{accounting:a,rest};
                driver::column_page_region_driver::attach_observed(display,&policy,&limits,MODE,&mut view)
            });
            assert!(matches!(result,Err(E::Identity)));assert_eq!(current,unpaid);
        }
        for bad_floor in [false,true] {
            let display=fresh();let mut current=before;
            if bad_floor {current.records=limits.base().get().max_fragments+1;}
            let invalid=current;
            let error=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|a,rest|{
                let mut view=driver::ColumnProgressView{accounting:a,rest};
                driver::column_page_region_driver::attach_observed(display,&other_policy,&limits,MODE,&mut view)
            }).err().unwrap();
            assert!(matches!(error,E::Identity));assert_eq!(current,invalid);
        }
        for mask in 1..8u8 {
            let display=fresh();let mut current=before;
            if mask&1!=0 {current.records=limits.base().get().max_fragments+1;}
            if mask&2!=0 {current.spool=limits.base().get().max_spool_bytes+1;}
            if mask&4!=0 {current.work=MAXIMUM+1;}
            let invalid=current;
            let result=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments,MAXIMUM,|a,rest|{
                let mut view=driver::ColumnProgressView{accounting:a,rest};
                driver::column_page_region_driver::attach_observed(display,&policy,&limits,MODE,&mut view)
            });
            let error=result.err().unwrap();
            assert!(matches!(error,E::ObservedAccounting(e) if e==if mask&1!=0 {A::Records{owner:NodeId::new(0)}} else if mask&2!=0 {A::Spool{owner:NodeId::new(0)}} else {A::Work{owner:NodeId::new(0)}}));
            assert_eq!(current,invalid);
        }
        // Full authenticated record domain is checked even on no-region input,
        // before the joint zero debit can diagnose already-invalid counters.
        let display=fresh();let mut current=before;current.records=limits.base().get().max_fragments+1;
        let invalid=current;
        let result=driver::with_observed_column_progress(&mut current,limits.base(),limits.base().get().max_fragments-1,MAXIMUM,|a,rest|{
            let mut view=driver::ColumnProgressView{accounting:a,rest};
            driver::column_page_region_driver::attach_observed(display,&policy,&limits,MODE,&mut view)
        });
        assert!(matches!(result,Err(E::ObservedAccounting(A::LimitsMismatch))));assert_eq!(current,invalid);
    }).unwrap();
}
