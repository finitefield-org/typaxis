use super::*;
/// Save the actual callback PDF separately from the independently constructed
/// per-component probe. Matching uses the selected display, not object numbers.
pub(crate) fn record_driver_pdf(
    pdf: &typaxis_pdf::book_v2::BookV2PdfAssembly<
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
        '_,
    >,
    observation: crate::book_v2_resources::BookV2PdfConvergenceObservation,
    limits: &M4EffectiveResourceLimits,
) {
    let marked = pdf.navigation().source().source().source();
    let display = marked.source().display();
    if display.source().source().has_header_variants() {
        assert_book_v2_body_resources(display.source(), display.admitted(), limits);
    } else {
        assert_math_display(display.source(), display.admitted(), limits);
    }
    let Ok(root) = std::env::var("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let name = pdf
        .fingerprint()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let root = PathBuf::from(root);
    let path = root.join(format!("driver-{name}.pdf"));
    fs::write(&path, pdf.bytes()).unwrap();
    let flow = display
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .source_flow();
    let frame = display
        .source()
        .source()
        .flow()
        .lines()
        .frames()
        .unwrap()
        .body();
    let rect = |r: typaxis_core::Rect| {
        [
            r.x().raw(),
            r.y().raw(),
            r.width().get().raw(),
            r.height().get().raw(),
        ]
    };
    let pages = display.source().source().geometry().pages();
    let probe = json!({
        "display":display.fingerprint(),
        "passes":[observation.candidate_passes(),observation.line_reshape_passes(),observation.page_passes()],
        "charges":[observation.record_charge(),observation.spool_charge(),observation.output_charge(),observation.work_steps()],
        "assembly": {
            "pdf":path.to_string_lossy(),"hash":typaxis_core::sha256(pdf.bytes()),"bytes":pdf.bytes().len(),
            "catalog":pdf.catalog_object(),"pages":(0..marked.pages().len()).map(|p|pdf.page_object(p).unwrap()).collect::<Vec<_>>(),
            "frame":[frame.x().raw(),frame.y().raw(),frame.width().get().raw(),frame.height().get().raw()],
            "page_names":pages.iter().map(|p|p.selection().named_page()).collect::<Vec<_>>(),
            "page_frames":pages.iter().map(|p|rect(p.selection().body_bounds())).collect::<Vec<_>>(),
            "footnote_regions":pages.iter().map(|p|p.selection().declared_footnote_region().map(rect)).collect::<Vec<_>>(),
            "candidates":flow.page_reference_values().unwrap_or(&[]).iter().map(|(owner,page)|[owner.get(),*page]).collect::<Vec<_>>(),
            "references":pdf.page_references().iter().map(|r|json!({"owner":r.owner().get(),"candidate":r.candidate_page(),"target":r.target_page(),"placed":r.is_placed()})).collect::<Vec<_>>(),
            "labels_match":pdf.page_reference_labels_match(),
        }
    });
    fs::write(
        root.join(format!("driver-{name}.driver")),
        serde_json::to_vec(&probe).unwrap(),
    )
    .unwrap();
}
