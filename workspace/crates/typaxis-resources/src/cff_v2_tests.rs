use super::*;
use typaxis_core::{
    BidiLevel, Length, M4ResourceLimits, OpenTypeTag, ResourceLimits, TextBufferId, TextSpan,
    Utf8ByteOffset, ValidatedResourceLimits,
};
use typaxis_font::admit_sfnt_cff1_v2;
use typaxis_shaping::{shape_cff1_run_v2, Cff1ShapeInputV2};

fn source(id: u32, start: u32, len: usize) -> ShapeSourceSpan {
    ShapeSourceSpan::Parsed(
        TextSpan::new(
            TextBufferId::new(id),
            Utf8ByteOffset::new(start),
            Utf8ByteOffset::new(start + len as u32),
        )
        .unwrap(),
    )
}
fn input(text: &str) -> Cff1ShapeInputV2<'_> {
    Cff1ShapeInputV2 {
        run_id: GlyphRunId::new(1),
        font: FontInstanceId::new(1),
        source: source(9, 31, text.len()),
        utf8: text,
        font_size: PositiveLength::new(Length::from_raw(11 * 65536).unwrap()).unwrap(),
        bidi_level: BidiLevel::LTR,
        script: OpenTypeTag::new(*b"Hani").unwrap(),
        language: Some("ja"),
        pre_context: None,
        post_context: None,
    }
}
fn original() -> Cff1AdmissionV2 {
    let bytes: std::sync::Arc<[u8]> = std::fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap())
        .unwrap()
        .into();
    assert_eq!(
        sha256(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let limits = M4EffectiveResourceLimits::new(
        ValidatedResourceLimits::new(ResourceLimits::default()).unwrap(),
        M4ResourceLimits::default(),
    )
    .unwrap();
    admit_sfnt_cff1_v2(bytes, 0, &limits).unwrap()
}
#[test]
fn source_identity_includes_namespace_buffer_and_offsets() {
    let encode = |span| {
        let mut s = String::new();
        encode_source(&mut s, span);
        s
    };
    assert_eq!(
        encode(source(9, 31, 1)),
        "{\"end_byte\":32,\"kind\":\"parsed\",\"start_byte\":31,\"text_id\":9}"
    );
    assert_ne!(encode(source(9, 31, 1)), encode(source(8, 31, 1)));
    assert_ne!(encode(source(9, 31, 1)), encode(source(9, 32, 1)));
}
#[test]
fn empty_plan_set_has_no_font_work() {
    assert!(freeze_cff1_pdf_fonts_v2(&[]).unwrap().is_empty());
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_pdf_plan_original_extraction_identity_and_instance_union() {
    let admission = original();
    // Find an actual default variant: base and base+VS select the same GID,
    // while their exact source text is different.
    let (base, vs) = (0x4e00..=0x9fff)
        .filter_map(char::from_u32)
        .find_map(|base| {
            let gid = admission.cmap().glyph_for_sequence(base, None)?;
            (0xe0100..=0xe010f)
                .filter_map(char::from_u32)
                .find(|&vs| admission.cmap().glyph_for_sequence(base, Some(vs)) == Some(gid))
                .map(|vs| (base, vs))
        })
        .unwrap();
    let text = format!("A{base}{base}{vs}");
    let shaped = shape_cff1_run_v2(&admission, input(&text)).unwrap();
    let runs = [&shaped];
    let make = |runs| Cff1PdfFontInputV2 {
        font_face_id: FontFaceId::new(1),
        font_instance_id: FontInstanceId::new(1),
        admission: &admission,
        runs,
    };
    let plans = freeze_cff1_pdf_fonts_v2(&[make(&runs)]).unwrap();
    let plan = &plans[0];
    assert_eq!(plan.clusters().len(), 3);
    assert!(!plan.clusters()[0].requires_actual_text());
    assert!(plan.clusters()[1].requires_actual_text());
    assert!(plan.clusters()[2].requires_actual_text());
    assert_eq!(plan.clusters()[1].cids(), plan.clusters()[2].cids());
    assert_eq!(
        plan.clusters()
            .iter()
            .map(|c| c.exact_text())
            .collect::<String>(),
        text
    );
    for cluster in plan.clusters() {
        let extracted = if cluster.requires_actual_text() {
            cluster.exact_text().to_owned()
        } else {
            cluster
                .cids()
                .iter()
                .flat_map(|cid| {
                    plan.bindings()[usize::from(cid.get()) - 1]
                        .unicode
                        .iter()
                        .map(|v| v.get())
                })
                .collect()
        };
        assert_eq!(extracted, cluster.exact_text());
    }
    assert_eq!(plan.indirect_object_blueprint().len(), 6);
    assert_eq!(plan.dense_widths_1000().len(), plan.bindings().len() + 1);
    assert_eq!(
        plan.fingerprint(),
        freeze_cff1_pdf_fonts_v2(&[make(&runs)]).unwrap()[0].fingerprint()
    );
    let mut moved = input(&text);
    moved.source = source(10, 31, text.len());
    let moved = shape_cff1_run_v2(&admission, moved).unwrap();
    let moved_runs = [&moved];
    let moved_plan = freeze_cff1_pdf_fonts_v2(&[make(&moved_runs)]).unwrap();
    assert_eq!(plan.subset().sha256(), moved_plan[0].subset().sha256());
    assert_ne!(plan.fingerprint(), moved_plan[0].fingerprint());
    let mut second = input("B");
    second.font = FontInstanceId::new(2);
    let second = shape_cff1_run_v2(&admission, second).unwrap();
    let second_runs = [&second];
    let inputs = [
        Cff1PdfFontInputV2 {
            font_face_id: FontFaceId::new(1),
            font_instance_id: FontInstanceId::new(2),
            admission: &admission,
            runs: &second_runs,
        },
        make(&runs),
    ];
    let union = freeze_cff1_pdf_fonts_v2(&inputs).unwrap();
    assert_eq!(
        union
            .iter()
            .map(|p| p.font_instance_id().get())
            .collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(union[0].fingerprint(), plan.fingerprint());
    assert_eq!(union[1].clusters()[0].exact_text(), "B");
    assert!(matches!(
        freeze_cff1_pdf_fonts_v2(&[make(&runs), make(&runs)]),
        Err(E::InvalidInput)
    ));
    let duplicate = [&shaped, &shaped];
    assert!(matches!(
        freeze_cff1_pdf_fonts_v2(&[make(&duplicate)]),
        Err(E::InvalidInput)
    ));
    assert!(matches!(
        freeze_cff1_pdf_fonts_v2(&[make(&second_runs)]),
        Err(E::IdentityMismatch)
    ));
    let mut wrong_size = input("A");
    wrong_size.run_id = GlyphRunId::new(2);
    wrong_size.font_size = PositiveLength::new(Length::from_raw(12 * 65536).unwrap()).unwrap();
    let wrong_size = shape_cff1_run_v2(&admission, wrong_size).unwrap();
    let mixed_sizes = [&shaped, &wrong_size];
    let mixed = freeze_cff1_pdf_fonts_v2(&[make(&mixed_sizes)]).unwrap();
    assert_eq!(mixed[0].subset().sha256(), plan.subset().sha256());
    assert_eq!(mixed[0].bindings(), plan.bindings());
    assert_eq!(
        mixed[0].clusters().last().unwrap().font_size(),
        wrong_size.font_size()
    );
    assert_ne!(
        mixed[0].clusters()[0].font_size(),
        mixed[0].clusters().last().unwrap().font_size()
    );
    assert_ne!(mixed[0].fingerprint(), plan.fingerprint());
}

#[test]
fn generated_source_identity_preserves_owner_and_generation_kind() {
    use typaxis_core::{
        AnchorId, GeneratedBufferKey, GenerationKind, NodeId, SourceId, SourceSpan,
    };
    use typaxis_document::{Block, Document, Inline, ReferenceFormat, ValidatedDocumentNodeIndex};
    use typaxis_text::{GeneratedBufferDraft, GeneratedTextStore, TextStore};
    let provenance = |owner: u32, format: ReferenceFormat, kind: GenerationKind| {
        let span = SourceSpan::new(
            SourceId::new(0),
            Utf8ByteOffset::new(0),
            Utf8ByteOffset::new(0),
        )
        .unwrap();
        let mut children = Vec::new();
        for id in 2..owner {
            children.push(Inline::Anchor {
                node_id: NodeId::new(id),
                span,
                anchor_id: AnchorId::new(format!("padding-{id}")).unwrap(),
            });
        }
        children.push(Inline::Reference {
            node_id: NodeId::new(owner),
            span,
            target: AnchorId::new("target").unwrap(),
            format,
        });
        let index = ValidatedDocumentNodeIndex::new(&Document {
            node_id: NodeId::new(0),
            blocks: vec![Block::Paragraph {
                node_id: NodeId::new(1),
                span,
                classes: vec![],
                children,
            }],
            footnotes: vec![],
        })
        .unwrap();
        let key = GeneratedBufferKey::new(NodeId::new(owner), kind, 0);
        let draft = GeneratedBufferDraft::new(&index, key, "12".to_owned()).unwrap();
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let store = GeneratedTextStore::new(
            vec![draft],
            &index,
            &limits,
            &TextStore::new(vec![]).unwrap(),
        )
        .unwrap();
        store
            .provenance(key, Utf8ByteOffset::new(0), Utf8ByteOffset::new(2))
            .unwrap()
    };
    let encode = |p| {
        let mut s = String::new();
        encode_source(&mut s, ShapeSourceSpan::Generated(p));
        s
    };
    let counter = provenance(2, ReferenceFormat::Number, GenerationKind::Counter);
    let another = provenance(3, ReferenceFormat::Number, GenerationKind::Counter);
    let reference = provenance(2, ReferenceFormat::Page, GenerationKind::PageReference);
    assert_eq!(counter.text_span(), another.text_span());
    assert_eq!(counter.text_span(), reference.text_span());
    assert_ne!(encode(counter), encode(another));
    assert_ne!(encode(counter), encode(reference));
    assert_eq!(encode(counter),"{\"end_byte\":2,\"generation_kind\":\"counter\",\"kind\":\"generated\",\"owner\":2,\"owner_local_ordinal\":0,\"start_byte\":0,\"text_id\":0}");
}
