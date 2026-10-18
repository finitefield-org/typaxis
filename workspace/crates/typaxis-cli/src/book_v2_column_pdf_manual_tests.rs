use super::*;

fn copy_artifacts(value: &Value, source: &std::path::Path, target: &std::path::Path) {
    match value {
        Value::Object(object) => {
            if let (Some(name), Some(hash)) = (
                object.get("file").and_then(Value::as_str),
                object.get("sha256").and_then(Value::as_str),
            ) {
                assert_eq!(std::path::Path::new(name).file_name().unwrap(), name);
                let bytes = fs::read(source.join(name)).unwrap();
                assert_eq!(probes::hex(&typaxis_core::sha256(&bytes)), hash);
                if let Some(length) = object.get("length").and_then(Value::as_u64) {
                    assert_eq!(bytes.len() as u64, length);
                }
                if target.join(name).exists() {
                    assert_eq!(fs::read(target.join(name)).unwrap(), bytes);
                } else {
                    fs::write(target.join(name), &bytes).unwrap();
                }
            } else {
                for child in object.values() {
                    copy_artifacts(child, source, target);
                }
            }
        }
        Value::Array(values) => {
            for child in values {
                copy_artifacts(child, source, target);
            }
        }
        _ => {}
    }
}

fn manual_pdf(
    case: &str,
    observed: Option<BookV2PdfConvergenceObservation>,
    scope: &str,
    display: &BookV2ColumnDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    caps: &M4EffectiveResourceLimits,
) {
    let mut constructors = [[0; 4]; 14];
    let mut fonts = BookV2ColumnFontSelectionBuilder::new(display, caps, MAXIMUM, 0, 0, 0).unwrap();
    let units = fonts.reservation_sizes();
    constructors[0] = [
        fonts.record_charge(),
        fonts.spool_charge(),
        0,
        fonts.work_steps(),
    ];
    let selection = fonts.build().unwrap();
    let closed = fonts.prepare_font_closures(&selection).unwrap();
    let programs = fonts.write_font_programs(&closed).unwrap();
    let cids = fonts.plan_cids(&programs).unwrap();
    macro_rules! stage {($slot:expr,$builder:ident,$source:expr $(,$arg:expr)?)=>{{let mut b=$builder::new($source,$($arg,)?caps,MAXIMUM,0,0,0,0).unwrap();constructors[$slot]=[b.record_charge(),b.spool_charge(),b.output_charge(),b.work_steps()];b.build().unwrap()}};}
    let streams = stage!(1, BookV2ColumnFontStreamBuilder, &cids);
    let objects = stage!(2, BookV2ColumnFontObjectBuilder, &streams, 4);
    let text = stage!(3, BookV2ColumnTextCommandBuilder, &objects);
    let mut images = fonts
        .next_stage_after_counted(
            text.record_charge(),
            text.spool_charge(),
            text.work_steps(),
            &mut 0,
            &mut 0,
            &mut 0,
        )
        .unwrap();
    constructors[4] = [
        images.record_charge(),
        images.spool_charge(),
        text.output_charge(),
        images.work_steps(),
    ];
    let selected = images.select_images().unwrap();
    let raster = images.write_raster_programs(&selected).unwrap();
    let mut vb = BookV2ColumnVectorProgramBuilder::new(
        &raster,
        caps,
        MAXIMUM,
        0,
        0,
        text.output_charge(),
        0,
    )
    .unwrap();
    constructors[5] = [
        vb.record_charge(),
        vb.spool_charge(),
        vb.output_charge(),
        vb.work_steps(),
    ];
    let vectors = vb.build().unwrap();
    let objects = stage!(
        6,
        BookV2ColumnImageObjectBuilder,
        &vectors,
        objects.next_object()
    );
    let commands = stage!(7, BookV2ColumnImageCommandBuilder, &objects);
    let scopes = stage!(8, BookV2ColumnMarkedScopeBuilder, &commands);
    let mut cb =
        BookV2ColumnMarkedContentBuilder::new(&scopes, &text, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[9] = [
        cb.record_charge(),
        cb.spool_charge(),
        cb.output_charge(),
        cb.work_steps(),
    ];
    let content = cb.build().unwrap();
    let source = stage!(10, BookV2ColumnSourceStructureBuilder, &content);
    let mut builder = Relations::new(&source, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[11] = [
        builder.record_charge(),
        builder.spool_charge(),
        builder.output_charge(),
        builder.work_steps(),
    ];
    let tree = builder.build().unwrap();
    let facts = inspect_relations(&tree);
    if scope == "retained_source_cut" {
        assert!(facts.headers > 0);
        assert_eq!((facts.notes, facts.edges), (0, 0));
    } else {
        assert_eq!((facts.headers, facts.notes, facts.edges), (0, 1, 1));
    }
    assert_eq!(facts.notes, facts.relocated + facts.unpainted);
    let mut navigation = Navigation::new(&tree, caps, MAXIMUM, 0, 0, 0, 0).unwrap();
    constructors[12] = [
        navigation.record_charge(),
        navigation.spool_charge(),
        navigation.output_charge(),
        navigation.work_steps(),
    ];
    let n = navigation.build().unwrap();
    inspect_navigation(&n);
    navigation_probes::emit(
        case,
        Some(&n),
        &tree,
        None,
        caps,
        navigation_probes::Context {
            scope,
            parent_scope: scope,
            maximum: MAXIMUM,
            constructors: std::array::from_fn(|i| constructors[i]),
            font_units: units,
            first_bounds_size: navigation.reservation_sizes()[0],
            encoded: observed,
            marked: observed,
            relations: None,
            ancestor: observed,
            parent_caller_before: None,
            caller_before: None,
            observed: None,
            build_after: [
                n.record_charge(),
                n.spool_charge(),
                n.output_charge(),
                n.work_steps(),
            ],
        },
    );
    let mut assembly = ColumnAssembly::new_counted(
        &n, caps, MAXIMUM, 0, 0, 0, 0, &mut 0, &mut 0, &mut 0, &mut 0,
    )
    .unwrap();
    constructors[13] = [
        assembly.record_charge(),
        assembly.spool_charge(),
        assembly.output_charge(),
        assembly.work_steps(),
    ];
    assert_eq!(
        constructors[13],
        [
            n.record_charge() + 1,
            n.spool_charge(),
            n.output_charge(),
            n.work_steps()
        ]
    );
    let sizes = assembly.reservation_sizes();
    let pdf = assembly.build().unwrap();
    assert!(std::ptr::eq(pdf.navigation(), &n));
    let credit = text.source().bytes().len() + objects.bytes().len() + content.byte_length();
    assert_eq!(
        pdf.output_charge(),
        n.output_charge() - credit as u64 + pdf.bytes().len() as u64
    );
    assert!(pdf.bytes().starts_with(b"%PDF-1.7\n"));
    if let Some(root) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
        let root = std::path::PathBuf::from(root);
        let old_directory = root.join("column-navigation");
        const OLD_SHA: &str = "cdd301ba55aee91884088ab8b273848a4e1bb7dcd77b9543352b21ee4fc9eb0c";
        let old = old_directory.join(format!("{case}-{OLD_SHA}.json"));
        let bytes = fs::read(&old).unwrap_or_else(|e| {
            panic!("authentic retained Navigation snapshot differs: {old:?}: {e}")
        });
        assert_eq!(probes::hex(&typaxis_core::sha256(&bytes)), OLD_SHA);
        let parent: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parent["observed"]["scope"], "retained_source_cut");
        assert!(parent["observed"]["caller_after"].is_null());
        let target = root.join("column-pdf");
        fs::create_dir_all(&target).unwrap();
        copy_artifacts(&parent, &old_directory, &target);
        pdf_probes::emit_retained(case, &pdf, caps, sizes, constructors, parent);
    }
}

#[test]
fn book_v2_column_pdf_retained_end_two_contextual_header_gid71_keeps_original_roots() {
    with_retained_cut_display(|display, caps| {
        let semantic = display
            .text()
            .draws()
            .iter()
            .filter(|d| !d.repeated_header())
            .flat_map(|d| d.glyphs())
            .map(|g| g.original_gid().get())
            .collect::<BTreeSet<_>>();
        let repeated = display
            .text()
            .draws()
            .iter()
            .filter(|d| d.repeated_header())
            .flat_map(|d| d.glyphs())
            .map(|g| g.original_gid().get())
            .collect::<BTreeSet<_>>();
        assert!(semantic.contains(&59) && !semantic.contains(&71) && repeated.contains(&71));
        manual_pdf(
            "navigation-context-retained-cut",
            None,
            "retained_source_cut",
            display,
            caps,
        );
    });
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_pdf_callback_continuation_extra_build_application_error_retains_all_charges() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(2, false, false), None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let (mut original, mut accepted) = ([0; 4], [0; 4]);
    let result: Result<Result<(), &str>, _> = pdf_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |bundle, continuation, observed| {
            original = pdf_quartet(observed);
            assert!(std::ptr::eq(continuation.navigation(), bundle.navigation()));
            let again = continuation.build().unwrap();
            assert_eq!(again.bytes(), bundle.pdf().bytes());
            accepted = [
                continuation.record_charge(),
                continuation.spool_charge(),
                continuation.output_charge(),
                continuation.work_steps(),
            ];
            assert_eq!(accepted[2], original[2] + again.bytes().len() as u64);
            for axis in [0, 1, 3] {
                assert!(accepted[axis] > original[axis]);
            }
            // This forgets only the borrowed facade reference. The driver still
            // owns the original builder and reads its real accepted ledger.
            std::mem::forget(continuation);
            Err("continuation application refusal")
        },
    );
    let after = pdf_quartet(budget.observation());
    println!("column PDF continuation successful rebuild original={original:?} accepted={accepted:?} caller_after={after:?} result={result:?}");
    assert_eq!(result.unwrap(), Err("continuation application refusal"));
    assert_eq!(after, accepted);
}

#[test]
#[allow(forgetting_references)]
fn book_v2_column_pdf_callback_continuation_failed_build_and_forgotten_loan_retain_all_charges() {
    let caps = display_caps();
    let root = Root::new();
    let input = display_input(&root, region_data(2, false, false), None, &caps);
    let mut full = BookV2ColumnPageBudget::new(&caps, MAXIMUM);
    let mut successful_work = 0;
    pdf_drive(&input, &caps, MODE, &mut full, 4, |_, continuation, _| {
        continuation.build().unwrap();
        successful_work = continuation.work_steps();
    })
    .unwrap();

    let maximum = successful_work - 1;
    let mut budget = BookV2ColumnPageBudget::new(&caps, maximum);
    let (mut original, mut accepted) = ([0; 4], [0; 4]);
    pdf_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        4,
        |_, continuation, observed| {
            original = pdf_quartet(observed);
            assert_eq!(
                continuation.build().err(),
                Some(PdfError::Resource(PdfResourceError::Work))
            );
            accepted = [
                continuation.record_charge(),
                continuation.spool_charge(),
                continuation.output_charge(),
                continuation.work_steps(),
            ];
            for axis in [0, 1, 3] {
                assert!(accepted[axis] > original[axis]);
            }
            assert!(
                accepted[2] > original[2],
                "late work refusal follows accepted output reservation"
            );
            assert!(accepted[3] <= maximum);
            std::mem::forget(continuation);
        },
    )
    .unwrap();
    assert_eq!(pdf_quartet(budget.observation()), accepted);
    println!("column PDF continuation late failure original={original:?} accepted={accepted:?} caller_after={:?}", pdf_quartet(budget.observation()));

    let before_retry = budget.observation();
    assert!(pdf_drive(&input, &caps, MODE, &mut budget, 4, |_, _, _| {
        panic!("exhausted continuation retry delivered PDF")
    })
    .is_err());
    let after_retry = budget.observation();
    for axis in [0, 1, 3] {
        assert!(pdf_quartet(after_retry)[axis] >= pdf_quartet(before_retry)[axis]);
    }
    assert_eq!(after_retry.output_charge(), before_retry.output_charge());
}
