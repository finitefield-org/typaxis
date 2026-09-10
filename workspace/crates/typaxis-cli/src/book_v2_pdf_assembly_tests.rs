use super::*;
fn rectangle(r: typaxis_core::Rect) -> [i64; 4] {
    [
        r.x().raw(),
        r.y().raw(),
        r.width().get().raw(),
        r.height().get().raw(),
    ]
}
use typaxis_pdf::book_v2::{
    BookV2FontStreamError as PE, BookV2PdfAssemblyBuilder as Assembly, BookV2PdfAssemblyError as AE,
};
pub(super) fn check(
    source: &typaxis_pdf::book_v2::BookV2NavigationGeometry<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    limits: &M4EffectiveResourceLimits,
) -> serde_json::Value {
    let marked = source.source().source().source();
    let display = marked.source().display();
    let lines = display.source().source().flow().lines();
    let flow = lines.prepared().source_flow();
    let mut choice_work = 0;
    let mut expected = None;
    for page in 0..marked.pages().len() {
        let selected = typaxis_syntax::book_v2::select_book_v2_page_master(
            flow.body(),
            page as u32,
            display.source().source().geometry().pages()[page]
                .selection()
                .named_page(),
            &mut choice_work,
            u64::MAX,
        )
        .unwrap();
        let advanced = selected.advanced();
        if advanced.header_content.is_some()
            || advanced.footer_content.is_some()
            || advanced.column_layout.is_some()
        {
            expected = Some(AE::UnsupportedPageMaster);
            break;
        }
        let m = &selected.master().body;
        if lines.frames().is_none_or(|_f| {
            let b = display.source().source().geometry().pages()[page]
                .selection()
                .body_bounds();
            [
                b.x().raw(),
                b.y().raw(),
                b.width().get().raw(),
                b.height().get().raw(),
            ] != [m.x, m.y, m.width, m.height]
        }) {
            expected = Some(AE::PageFrameMismatch);
            break;
        }
    }
    // Input work is cumulative; this allowance funds the verification builds.
    let work_limit = source.work_steps().checked_add(100_000_000).unwrap();
    if let Some(expected) = expected {
        let mut b = Assembly::new(source, limits, work_limit, 0, 0, 0, 0).unwrap();
        let before = b.work_steps();
        assert_eq!(b.build().err().unwrap(), expected);
        let first = b.work_steps();
        assert!(first > before);
        assert_eq!(b.build().err().unwrap(), expected);
        assert!(b.work_steps() > first);
        let mut pipeline = typaxis_pdf::book_v2::BookV2PdfPipeline::new(
            display,
            marked.text().source().first_object(),
            limits,
            work_limit,
            0,
            0,
            0,
            0,
        )
        .unwrap();
        for _ in 0..2 {
            let before = pipeline.work_steps();
            let error = pipeline
                .with_pdf(|_| panic!("failed assembly reached callback"))
                .unwrap_err();
            assert!(
                matches!(error, typaxis_pdf::book_v2::BookV2PdfPipelineError::Assembly(e) if e == expected)
            );
            assert!(pipeline.work_steps() > before);
        }
        return serde_json::json!({"error":format!("{expected:?}"),"frame":lines.frames().map(|f|rectangle(f.body()))});
    }
    let run = |records, spool, output, work, prior_work, capture| -> Result<_, AE> {
        let mut b = Assembly::new(source, limits, work, records, spool, output, prior_work)?;
        let pdf = b.build()?;
        let fonts = marked.text().source();
        let images = marked.source().source().source();
        for object in fonts.objects() {
            assert_eq!(
                pdf.object_bytes(object.id().get()),
                Some(&fonts.bytes()[object.byte_range()])
            );
        }
        for object in images.objects() {
            assert_eq!(
                pdf.object_bytes(object.id().get()),
                Some(&images.bytes()[object.byte_range()])
            );
        }
        for object in marked.anchor_objects() {
            assert_eq!(
                pdf.object_bytes(object.id().get()),
                Some(&marked.anchor_object_bytes()[object.byte_range()])
            );
        }
        assert!(pdf.bytes().starts_with(b"%PDF-1.7\n"));
        assert!(pdf.bytes().ends_with(b"%%EOF\n"));
        assert!(pdf.object_bytes(pdf.catalog_object()).is_some());
        assert!(pdf.object_bytes(0).is_none());
        assert!(pdf.object_bytes(u32::MAX).is_none());
        assert!(pdf.page_object(marked.pages().len()).is_none());
        let mut result = serde_json::Value::Null;
        if capture {
            pipeline::check(display, fonts.first_object(), limits, pdf.bytes());
            let path = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE").map(|root| {
                let root = std::path::PathBuf::from(root);
                std::fs::create_dir_all(&root).unwrap();
                let hash = pdf
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let path = root.join(format!("assembly-{hash}.pdf"));
                std::fs::write(&path, pdf.bytes()).unwrap();
                path.to_string_lossy().into_owned()
            });
            result = serde_json::json!({"pdf":path,"hash":typaxis_core::sha256(pdf.bytes()),"bytes":pdf.bytes().len(),"catalog":pdf.catalog_object(),"pages":(0..marked.pages().len()).map(|p|pdf.page_object(p).unwrap()).collect::<Vec<_>>(),"frame":lines.frames().map(|f|rectangle(f.body()))});
            result["page_names"] = serde_json::json!(display
                .source()
                .source()
                .geometry()
                .pages()
                .iter()
                .map(|p| p.selection().named_page())
                .collect::<Vec<_>>());
            result["page_frames"] = serde_json::json!(display
                .source()
                .source()
                .geometry()
                .pages()
                .iter()
                .map(|p| rectangle(p.selection().body_bounds()))
                .collect::<Vec<_>>());
            result["footnote_regions"] = serde_json::json!(display
                .source()
                .source()
                .geometry()
                .pages()
                .iter()
                .map(|p| p.selection().declared_footnote_region().map(rectangle))
                .collect::<Vec<_>>());
            result["candidates"] = serde_json::json!(flow
                .page_reference_values()
                .unwrap_or(&[])
                .iter()
                .map(|(owner, page)| [owner.get(), *page])
                .collect::<Vec<_>>());
            result["references"] = serde_json::json!(pdf.page_references().iter().map(|r| serde_json::json!({"owner":r.owner().get(),"candidate":r.candidate_page(),"target":r.target_page(),"placed":r.is_placed()})).collect::<Vec<_>>());
            result["labels_match"] = pdf.page_reference_labels_match().into();
            let again = b.build()?;
            assert_eq!(pdf.bytes(), again.bytes());
            assert_eq!(pdf.fingerprint(), again.fingerprint());
            assert!(again.record_charge() > pdf.record_charge());
            assert!(again.spool_charge() > pdf.spool_charge());
            assert_eq!(
                again.output_charge(),
                pdf.output_charge() + pdf.bytes().len() as u64
            );
        }
        Ok((
            pdf.record_charge(),
            pdf.spool_charge(),
            pdf.output_charge(),
            pdf.work_steps(),
            pdf.fingerprint(),
            result,
        ))
    };
    let full = run(0, 0, 0, work_limit, 0, true).unwrap();
    let base = limits.base().get();
    let pr = base.max_fragments - (full.0 - source.record_charge());
    let ps = base.max_spool_bytes - (full.1 - source.spool_charge());
    let po = base.max_output_bytes - (full.2 - source.output_charge());
    let exact = run(pr, ps, po, full.3, 0, false).unwrap();
    assert_eq!(
        (exact.0, exact.1, exact.2, exact.3, exact.4),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            full.3,
            full.4
        )
    );
    for (r, s, o, w, error) in [
        (pr + 1, 0, 0, work_limit, PE::Records),
        (0, ps + 1, 0, work_limit, PE::Spool),
        (0, 0, po + 1, work_limit, PE::Output),
        (0, 0, 0, full.3 - 1, PE::Work),
    ] {
        assert_eq!(
            run(r, s, o, w, 0, false).err().unwrap(),
            AE::Resource(error)
        );
    }
    let prior = run(0, 0, 0, work_limit, source.work_steps() + 17, false).unwrap();
    assert_eq!(prior.3, full.3 + 17);
    assert_eq!(prior.4, full.4);
    let mut late = Assembly::new(source, limits, full.3 - 1, 0, 0, 0, 0).unwrap();
    assert_eq!(late.build().err().unwrap(), AE::Resource(PE::Work));
    let retained = (
        late.record_charge(),
        late.spool_charge(),
        late.output_charge(),
    );
    assert_eq!(late.build().err().unwrap(), AE::Resource(PE::Work));
    assert_eq!(
        retained,
        (
            late.record_charge(),
            late.spool_charge(),
            late.output_charge()
        )
    );
    full.5
}

#[path = "book_v2_pipeline_tests.rs"]
mod pipeline;
