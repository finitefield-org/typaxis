use super::*;
use typaxis_display_list::book_v2::{BookV2BodyDisplay, BookV2ImagePaint};
use typaxis_resources::book_v2::{
    BookV2FontSelectionBuilder as Images, BookV2FontSelectionError as IE, BookV2RasterError as RE,
};
use typaxis_resources::{AdmittedImageMediaKind as Media, ImageEncoding};
fn image_key(image: &typaxis_resources::AdmittedImage) -> (String, [u8; 32], Option<[u8; 32]>) {
    (
        format!("{:?}", image.media_kind()),
        image.content_hash(),
        image.admitted_safe_vector().map(|v| v.fingerprint()),
    )
}
pub(super) fn check(
    text: &typaxis_pdf::book_v2::BookV2TextCommands<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
    base_records: u64,
    base_spool: u64,
    base_output: u64,
    base_work: u64,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let mut oracle = BTreeMap::new();
    let mut count = 0;
    for (i, paint) in display.paints().iter().enumerate() {
        let usage = display.image_use(i).unwrap();
        let expected = match *paint {
            Paint::Image(_) => true,
            Paint::Math(j) => matches!(
                display.math().draws()[j].paint(),
                BookV2MathPaint::Vector(_)
            ),
            _ => false,
        };
        assert_eq!(usage.is_some(), expected);
        let Some(usage) = usage else { continue };
        count += 1;
        assert_eq!(usage.paint(), *paint);
        assert!(std::ptr::eq(
            usage.image(),
            display.admitted().image(usage.image().image_id()).unwrap()
        ));
        match *paint {
            Paint::Image(j) => {
                let draw = &display.images().draws()[j];
                assert!(std::ptr::eq(usage.image(), draw.image()));
                assert_eq!(usage.owner(), draw.source().owner());
                assert_eq!(usage.page_index(), draw.fragment().fragment().page_index());
                assert_eq!(usage.geometry(), draw.paint());
            }
            Paint::Math(j) => {
                let draw = &display.math().draws()[j];
                let BookV2MathPaint::Vector(v) = draw.paint() else {
                    panic!()
                };
                assert_eq!(usage.owner(), draw.terminal().source().owner());
                assert_eq!(usage.page_index(), draw.terminal().page_index());
                assert_eq!(
                    usage.geometry(),
                    BookV2ImagePaint::Vector {
                        content_key: v.content_key(),
                        viewport: v.viewport(),
                        scale_raw: v.scale_raw(),
                        matrix: v.matrix(),
                        color: v.color()
                    }
                );
            }
            _ => panic!(),
        }
        oracle
            .entry(image_key(usage.image()))
            .and_modify(|id: &mut u32| *id = (*id).min(usage.image().image_id().get()))
            .or_insert(usage.image().image_id().get());
    }
    assert!(display.image_use(usize::MAX).unwrap().is_none());
    let run = |records: u64, spool: u64, work, prior_work: u64, capture: bool| -> Result<_, RE> {
        let mut builder = Images::new(
            display,
            limits,
            work,
            records.max(base_records),
            spool.max(base_spool),
            prior_work.max(base_work),
        )?;
        let selection = builder.select_images()?;
        assert!(std::ptr::eq(selection.display(), display));
        assert_eq!(selection.uses().len(), count);
        assert_eq!(selection.images().len(), oracle.len());
        let mut seen = BTreeSet::new();
        for image in selection.images() {
            assert!(seen.insert(image_key(image.image())));
            assert_eq!(
                oracle[&image_key(image.image())],
                image.image().image_id().get()
            );
        }
        for usage in selection.uses() {
            let expected = display.image_use(usage.paint_index()).unwrap().unwrap();
            assert!(std::ptr::eq(usage.usage().image(), expected.image()));
            assert_eq!(usage.usage().geometry(), expected.geometry());
            assert_eq!(usage.usage().owner(), expected.owner());
            assert_eq!(
                image_key(selection.images()[usage.resource_index()].image()),
                image_key(expected.image())
            );
            assert!(std::ptr::eq(
                selection.for_paint(usage.paint_index()).unwrap(),
                usage
            ));
        }
        for i in 0..display.paints().len() {
            assert_eq!(
                selection.for_paint(i).is_some(),
                display.image_use(i).unwrap().is_some()
            );
        }
        assert!(selection.for_paint(usize::MAX).is_none());
        let stats = builder.cff_program_statistics();
        let programs = builder.write_raster_programs(&selection)?;
        assert!(std::ptr::eq(programs.source(), &selection));
        assert_eq!(builder.cff_program_statistics(), stats);
        assert_eq!(
            (
                programs.record_charge(),
                programs.spool_charge(),
                programs.work_steps()
            ),
            (
                builder.record_charge(),
                builder.spool_charge(),
                builder.work_steps()
            )
        );
        assert!(programs.program(usize::MAX).is_none());
        for (i, image) in selection.images().iter().enumerate() {
            let program = programs.program(i);
            if matches!(
                image.image().media_kind(),
                Media::SafeVector | Media::SafeVector2
            ) {
                assert!(program.is_none());
                continue;
            }
            let p = program.unwrap();
            assert!(std::ptr::eq(p.source(), image));
            assert_eq!(p.width(), image.image().width().get());
            assert_eq!(p.height(), image.image().height().get());
            assert_eq!(p.sha256(), typaxis_core::sha256(p.bytes()));
            match image.image().media_kind() {
                Media::Png => {
                    assert_eq!(p.encoding(), ImageEncoding::Flate);
                    assert!(p.jpeg().is_none());
                    if let Some(alpha) = p.alpha_mask() {
                        assert_eq!(alpha.width.get(), p.width());
                        assert_eq!(alpha.height.get(), p.height());
                        assert_eq!(alpha.bits_per_component, 8);
                        assert_eq!(alpha.encoding, ImageEncoding::Flate);
                    }
                }
                Media::JpegBaseline => {
                    assert_eq!(p.encoding(), ImageEncoding::Jpeg);
                    assert!(p.alpha_mask().is_none());
                    let j = image.image().jpeg_attestation().unwrap();
                    assert!(std::ptr::eq(p.jpeg().unwrap(), j));
                    assert!(std::ptr::eq(p.bytes(), j.normalized_bytes()));
                    assert_eq!(p.sha256(), j.normalized_sha256());
                }
                _ => unreachable!(),
            }
            if capture {
                if let Ok(directory) = std::env::var("TYPAXIS_BOOK_RASTER_PROGRAMS_PROBE") {
                    let key = p
                        .fingerprint()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>();
                    let path = std::path::Path::new(&directory);
                    std::fs::create_dir_all(path).unwrap();
                    std::fs::write(path.join(format!("{key}.source")), image.image().bytes())
                        .unwrap();
                    std::fs::write(path.join(format!("{key}.color")), p.bytes()).unwrap();
                    if let Some(alpha) = p.alpha_mask() {
                        std::fs::write(path.join(format!("{key}.alpha")), &alpha.encoded_bytes)
                            .unwrap();
                    }
                    std::fs::write(path.join(format!("{key}.json")),serde_json::to_vec(&serde_json::json!({"width":p.width(),"height":p.height(),"color_space":format!("{:?}",p.color_space()),"encoding":format!("{:?}",p.encoding()),"source_sha256":image.image().content_hash().iter().map(|b|format!("{b:02x}")).collect::<String>(),"payload_sha256":p.sha256().iter().map(|b|format!("{b:02x}")).collect::<String>(),"alpha":p.alpha_mask().is_some()})).unwrap()).unwrap();
                }
            }
        }
        if capture {
            vectors::check(text, &programs, limits, base_output);
        }
        Ok((
            builder.record_charge(),
            builder.spool_charge(),
            builder.work_steps(),
            selection.fingerprint(),
            programs.fingerprint(),
        ))
    };
    let (records, spool, work, selection_fp, fp) = run(0, 0, MAX_WORK, 0, true).unwrap();
    assert_eq!(
        run(0, 0, work, 0, false).unwrap(),
        (records, spool, work, selection_fp, fp)
    );
    let pr = limits.base().get().max_fragments - (records - base_records);
    let ps = limits.base().get().max_spool_bytes - (spool - base_spool);
    assert_eq!(
        run(pr, ps, work, 0, false).unwrap(),
        (
            limits.base().get().max_fragments,
            limits.base().get().max_spool_bytes,
            work,
            selection_fp,
            fp
        )
    );
    assert!(matches!(
        run(pr + 1, ps, work, 0, false),
        Err(RE::Budget(IE::Records))
    ));
    assert!(matches!(
        run(pr, ps + 1, work, 0, false),
        Err(RE::Budget(IE::Spool))
    ));
    assert!(matches!(
        run(0, 0, work - 1, 0, false),
        Err(RE::Budget(IE::Work))
    ));
    assert_eq!(
        run(0, 0, work + 17, base_work + 17, false).unwrap(),
        (records, spool, work + 17, selection_fp, fp)
    );
    let mut failed = Images::new(
        display,
        limits,
        work - 1,
        base_records,
        base_spool,
        base_work,
    )
    .unwrap();
    let selection = failed.select_images().unwrap();
    assert!(matches!(
        failed.write_raster_programs(&selection),
        Err(RE::Budget(IE::Work))
    ));
    assert_eq!(failed.work_steps(), work - 1);
    let used = (failed.record_charge(), failed.spool_charge());
    assert!(matches!(
        failed.write_raster_programs(&selection),
        Err(RE::Budget(IE::Work))
    ));
    assert!(failed.record_charge() >= used.0 && failed.spool_charge() >= used.1);
    let mut repeated = Images::new(
        display,
        limits,
        MAX_WORK,
        base_records,
        base_spool,
        base_work,
    )
    .unwrap();
    let selection = repeated.select_images().unwrap();
    let one = repeated.write_raster_programs(&selection).unwrap();
    let two = repeated.write_raster_programs(&selection).unwrap();
    assert_eq!(one.fingerprint(), two.fingerprint());
    for i in 0..selection.images().len() {
        match (one.program(i), two.program(i)) {
            (Some(a), Some(b)) => {
                assert_eq!(a.bytes(), b.bytes());
                assert_eq!(
                    a.alpha_mask().map(|m| &m.encoded_bytes),
                    b.alpha_mask().map(|m| &m.encoded_bytes)
                );
            }
            (None, None) => {}
            _ => panic!(),
        }
    }
    let mut foreign_builder = typaxis_display_list::book_v2::BookV2MathDisplayBuilder::new(
        display.source(),
        display.admitted(),
        limits,
        MAX_WORK,
        0,
        0,
    )
    .unwrap();
    let foreign = foreign_builder.build_body().unwrap();
    assert_eq!(foreign.fingerprint(), display.fingerprint());
    let mut different = Images::new(
        &foreign,
        limits,
        MAX_WORK,
        base_records,
        base_spool,
        base_work,
    )
    .unwrap();
    assert!(matches!(
        different.write_raster_programs(&selection),
        Err(RE::Budget(IE::Identity))
    ));
}

#[path = "book_v2_vector_program_tests.rs"]
mod vectors;
