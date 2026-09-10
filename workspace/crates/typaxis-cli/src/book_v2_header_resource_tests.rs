use super::*;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_resources::book_v2::*;

fn text(t: BookV2FontUseText<'_>) -> String {
    match t {
        BookV2FontUseText::Text(s) => s.to_owned(),
        BookV2FontUseText::Scalar(c) => c.to_string(),
    }
}

pub(super) fn verify(
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    repeated_only_gid: Option<u16>,
) {
    display
        .verify_resource_selection(display.admitted(), limits)
        .unwrap();
    if let Some(gid) = repeated_only_gid {
        let contains = |repeated| {
            display
                .text()
                .draws()
                .iter()
                .filter(|d| d.repeated_header() == repeated)
                .flat_map(|d| d.glyphs())
                .any(|g| g.original_gid().get() == gid)
        };
        assert!(
            !contains(false),
            "context glyph must be absent from every original text paint"
        );
        assert!(
            contains(true),
            "context glyph must occur in actual repeated header paint"
        );
    }
    let mut expected = BTreeMap::<_, BTreeSet<_>>::new();
    let mut positions = Vec::new();
    let mut image_positions = Vec::new();
    let mut image_payloads = BTreeSet::new();
    for paint in 0..display.paints().len() {
        for slot in 0..display.font_slot_count(paint).unwrap() {
            if let Some(usage) = display.font_use(paint, slot).unwrap() {
                positions.push((paint, slot));
                let glyphs = expected
                    .entry(usage.instance().font_instance_id())
                    .or_default();
                for i in 0..usage.glyphs().len() {
                    glyphs.insert(usage.glyphs().get(i).unwrap());
                }
            }
        }
        if let Some(image) = display.image_use(paint).unwrap() {
            image_positions.push(paint);
            image_payloads.insert((
                image.image().content_hash(),
                format!("{:?}", image.image().media_kind()),
            ));
        }
    }
    let run = |work, records, spool, prior_work| -> Result<_, Box<dyn std::error::Error>> {
        let mut builder =
            BookV2FontSelectionBuilder::new(display, limits, work, records, spool, prior_work)?;
        let selected = builder.build()?;
        assert_eq!(selected.uses().len(), positions.len());
        assert_eq!(selected.fonts().len(), expected.len());
        for ((&id, glyphs), (i, font)) in expected.iter().zip(selected.fonts().iter().enumerate()) {
            assert_eq!(id, font.instance().font_instance_id());
            assert_eq!(
                selected.glyphs(i).unwrap().collect::<BTreeSet<_>>(),
                *glyphs
            );
            assert!(std::ptr::eq(font.instance().ledger(), display.admitted()));
        }
        for (actual, &(paint, slot)) in selected.uses().iter().zip(&positions) {
            let original = display.font_use(paint, slot)?.unwrap();
            assert_eq!((actual.paint_index(), actual.usage().slot()), (paint, slot));
            assert_eq!(actual.usage().source(), original.source());
            assert_eq!(actual.usage().text(), original.text());
            assert_eq!(
                actual.usage().instance().table_fingerprint(),
                original.instance().table_fingerprint()
            );
            assert!(std::ptr::eq(
                actual.usage().instance().font(),
                original.instance().font()
            ));
        }
        if let Some(gid) = repeated_only_gid {
            assert!(selected.glyphs(0).unwrap().any(|g| g.get() == gid));
        }
        let closures = builder.prepare_font_closures(&selected)?;
        let programs = builder.write_font_programs(&closures)?;
        let cids = builder.plan_cids(&programs)?;
        for (i, font) in programs.fonts().iter().enumerate() {
            let closed = &closures.fonts()[i];
            assert!(std::ptr::eq(font.source(), closed));
            assert_eq!(font.sha256(), typaxis_core::sha256(font.bytes()));
            assert_eq!(closed.glyphs().next().unwrap().get(), 0);
            for gid in selected.glyphs(i).unwrap() {
                assert!(closed.subset_gid(gid).is_some());
                assert_eq!(font.subset_gid(gid), closed.subset_gid(gid));
                assert!(font.advance(gid).is_some());
            }
            assert!(match font.kind() {
                BookV2FontClosureKind::TrueType => font.bytes().starts_with(&[0, 1, 0, 0]),
                BookV2FontClosureKind::Cff1V2 => font.bytes().starts_with(b"OTTO"),
            });
            let bindings = cids.bindings(i).unwrap();
            assert_eq!(bindings.len(), selected.glyphs(i).unwrap().len());
            for (binding, gid) in bindings.iter().zip(selected.glyphs(i).unwrap()) {
                assert_eq!(binding.original_gid(), gid);
                assert_eq!(Some(binding.subset_gid()), closed.subset_gid(gid));
                let units = u64::from(
                    selected.fonts()[i]
                        .instance()
                        .font()
                        .metadata()
                        .units_per_em,
                );
                assert_eq!(
                    u64::from(binding.width_1000()),
                    (u64::from(font.advance(gid).unwrap()) * 1000 + units / 2) / units
                );
            }
        }
        assert_eq!(cids.uses().len(), selected.uses().len());
        for (i, (usage, original)) in cids.uses().iter().zip(selected.uses()).enumerate() {
            assert!(std::ptr::eq(usage.source(), original));
            assert_eq!(
                cids.usage_index(original.paint_index(), original.usage().slot()),
                Some(i)
            );
            let ids = cids.cids(i).unwrap();
            assert_eq!(ids.len(), original.usage().glyphs().len());
            let bindings = cids.bindings(usage.font_index()).unwrap();
            let mut extracted = String::new();
            for (j, cid) in ids.iter().enumerate() {
                let binding = &bindings[cid.get() as usize - 1];
                assert_eq!(
                    binding.original_gid(),
                    original.usage().glyphs().get(j).unwrap()
                );
                if let Some(c) = binding.unicode() {
                    extracted.push(c);
                }
            }
            if usage.requires_actual_text() {
                assert_eq!(usage.actual_text().unwrap(), original.usage().text());
                extracted = text(usage.actual_text().unwrap());
            }
            assert_eq!(extracted, text(original.usage().text()));
        }
        if work == 1_000_000_000 && records == 0 && spool == 0 && prior_work == 0 {
            if let Some(root) = std::env::var_os("TYPAXIS_BOOK_V2_HEADER_RESOURCE_PROBE") {
                let root = std::path::PathBuf::from(root);
                assert!(root.is_absolute());
                fs::create_dir_all(&root).unwrap();
                let prefix = display
                    .fingerprint()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>();
                let mut fonts = Vec::new();
                for (index, program) in programs.fonts().iter().enumerate() {
                    let original = selected.fonts()[index].instance().font();
                    let source_name = format!("{prefix}-{index}-source.sfnt");
                    let subset_name = format!("{prefix}-{index}-subset.sfnt");
                    fs::write(root.join(&source_name), original.bytes()).unwrap();
                    fs::write(root.join(&subset_name), program.bytes()).unwrap();
                    fonts.push(json!({"source":source_name,"subset":subset_name,
                        "source_sha256":original.content_hash().iter().map(|b|format!("{b:02x}")).collect::<String>(),
                        "subset_sha256":program.sha256().iter().map(|b|format!("{b:02x}")).collect::<String>(),
                        "mapping":closures.fonts()[index].glyphs().map(|g|[g.get(),program.subset_gid(g).unwrap().get()]).collect::<Vec<_>>(),
                        "selected":selected.glyphs(index).unwrap().map(|g|g.get()).collect::<Vec<_>>(),
                        "bindings":cids.bindings(index).unwrap().iter().map(|b|json!({"gid":b.original_gid().get(),"cid":b.cid().get(),"subset":b.subset_gid().get(),"width":b.width_1000(),"unicode":b.unicode().map(|c|c.to_string())})).collect::<Vec<_>>() }));
                }
                let uses = cids.uses().iter().enumerate().map(|(i,u)|json!({"font":u.font_index(),
                    "gids":(0..u.source().usage().glyphs().len()).map(|j|u.source().usage().glyphs().get(j).unwrap().get()).collect::<Vec<_>>(),
                    "cids":cids.cids(i).unwrap().iter().map(|c|c.get()).collect::<Vec<_>>(),
                    "text":text(u.source().usage().text()),"actual_text":u.actual_text().map(text)})).collect::<Vec<_>>();
                fs::write(
                    root.join(format!("{prefix}.json")),
                    serde_json::to_vec_pretty(
                        &json!({"fonts":fonts,"uses":uses,"repeated_only_gid":repeated_only_gid}),
                    )
                    .unwrap(),
                )
                .unwrap();
            }
        }
        let images = builder.select_images()?;
        assert_eq!(images.uses().len(), image_positions.len());
        for (actual, &paint) in images.uses().iter().zip(&image_positions) {
            let original = display.image_use(paint)?.unwrap();
            assert_eq!(actual.paint_index(), paint);
            assert_eq!(actual.usage().owner(), original.owner());
            assert_eq!(actual.usage().page_index(), original.page_index());
            assert_eq!(actual.usage().geometry(), original.geometry());
            assert!(std::ptr::eq(actual.usage().image(), original.image()));
            assert!(std::ptr::eq(
                images.images()[actual.resource_index()].image(),
                original.image()
            ));
        }
        assert_eq!(images.images().len(), image_payloads.len());
        Ok((
            builder.work_steps(),
            builder.record_charge(),
            builder.spool_charge(),
            [
                selected.fingerprint(),
                closures.fingerprint(),
                programs.fingerprint(),
                cids.fingerprint(),
                images.fingerprint(),
            ],
            programs
                .fonts()
                .iter()
                .map(|f| f.sha256())
                .collect::<Vec<_>>(),
        ))
    };
    let full = run(1_000_000_000, 0, 0, 0).unwrap();
    assert_eq!(run(full.0, 0, 0, 0).unwrap(), full);
    assert!(run(full.0 - 1, 0, 0, 0).is_err());
    let prior = limits.base().get().max_fragments - (full.1 - display.record_charge());
    let spool = limits.base().get().max_spool_bytes - (full.2 - display.source().spool_charge());
    let exact = run(full.0, prior, spool, 0).unwrap();
    assert_eq!(
        (exact.1, exact.2),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes
        )
    );
    assert_eq!((exact.3, &exact.4), (full.3, &full.4));
    assert!(run(full.0, prior + 1, spool, 0).is_err());
    assert!(run(full.0, prior, spool + 1, 0).is_err());
    let shifted = run(full.0 + 17, 0, 0, display.work_steps() + 17).unwrap();
    assert_eq!(
        (shifted.0, shifted.1, shifted.2),
        (full.0 + 17, full.1, full.2)
    );
    assert_eq!((shifted.3, &shifted.4), (full.3, &full.4));
    eprintln!(
        "header resources: work={},records={},spool={},fonts={},uses={},images={}",
        full.0,
        full.1,
        full.2,
        expected.len(),
        positions.len(),
        image_positions.len()
    );
}
