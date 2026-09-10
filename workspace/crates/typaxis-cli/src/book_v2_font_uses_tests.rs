use super::*;
use typaxis_display_list::book_v2::{
    BookV2FontUseGlyphs as G, BookV2FontUseSource as S, BookV2FontUseText as T,
};
use typaxis_display_list::ProductionNativeMathPaint;

pub(super) fn check(display: &BookV2BodyDisplay<'_, '_, '_, '_, '_, '_, '_, '_>) {
    let instances = display
        .source()
        .source()
        .flow()
        .lines()
        .prepared()
        .shaped()
        .font_instances();
    for (paint_index, paint) in display.paints().iter().enumerate() {
        let mut expected = Vec::new();
        match *paint {
            P::Text(i) => {
                let d = &display.text().draws()[i];
                expected.push(Some((
                    S::Text(d.text_span()),
                    T::Text(d.exact_text()),
                    G::Cluster(d.glyphs()),
                    d.font().face_id(),
                    d.font().size(),
                    instances.fingerprint(),
                )));
            }
            P::Marker(i) => {
                let d = &display.markers().draws()[i];
                for c in d.clusters() {
                    expected.push(Some((
                        S::Text(c.text_span()),
                        T::Text(c.exact_text()),
                        G::Cluster(c.glyphs()),
                        d.source().font().face_id(),
                        d.source().font().size(),
                        instances.fingerprint(),
                    )));
                }
            }
            P::EquationNumber(i) => {
                let d = &display.numbers().draws()[i];
                for c in d.clusters() {
                    expected.push(Some((
                        S::Text(c.text_span()),
                        T::Text(c.exact_text()),
                        G::Cluster(c.glyphs()),
                        d.font().face_id(),
                        d.font().size(),
                        instances.fingerprint(),
                    )));
                }
            }
            P::Math(i) => {
                let d = &display.math().draws()[i];
                if let BookV2MathPaint::Native(n) = d.paint() {
                    let BookV2BodyMathSource::Native(receipt) = d.terminal().source() else {
                        panic!()
                    };
                    let table = display
                        .source()
                        .source()
                        .flow()
                        .lines()
                        .prepared()
                        .native_math()
                        .unwrap()
                        .font_instances()
                        .fingerprint();
                    for (slot, p) in n.paints().iter().enumerate() {
                        expected.push(match *p {
                            ProductionNativeMathPaint::Rule(_) => None,
                            ProductionNativeMathPaint::Glyph {
                                original_gid,
                                unicode,
                                logical_ordinal,
                                font_size,
                                ..
                            } => Some((
                                S::NativeMath {
                                    owner: receipt.node_id(),
                                    receipt: receipt.fingerprint(),
                                    computation: receipt.computation().fingerprint(),
                                    paint_index: slot as u32,
                                    logical_ordinal,
                                },
                                T::Scalar(unicode),
                                G::Native(original_gid),
                                receipt.font_face_id(),
                                font_size,
                                table,
                            )),
                        });
                    }
                }
            }
            P::Image(_) | P::FootnoteSeparator(_) => (),
        }
        assert_eq!(display.font_slot_count(paint_index), Some(expected.len()));
        assert!(display
            .font_use(paint_index, expected.len())
            .unwrap()
            .is_none());
        for (slot, expected) in expected.into_iter().enumerate() {
            let actual = display.font_use(paint_index, slot).unwrap();
            let Some((source, text, glyphs, face, size, table)) = expected else {
                assert!(actual.is_none());
                continue;
            };
            let actual = actual.unwrap();
            assert_eq!(actual.paint(), *paint);
            assert_eq!(actual.slot(), slot);
            assert_eq!(actual.source(), source);
            assert_eq!(actual.text(), text);
            assert_eq!(actual.size(), size);
            assert!(std::ptr::eq(actual.instance().ledger(), display.admitted()));
            assert!(std::ptr::eq(
                actual.instance().font(),
                display.admitted().font(face).unwrap()
            ));
            assert_eq!(actual.instance().table_fingerprint(), table);
            assert_eq!(actual.glyphs().len(), glyphs.len());
            assert!(!actual.glyphs().is_empty());
            match (actual.glyphs(), glyphs) {
                (G::Cluster(a), G::Cluster(b)) => assert!(std::ptr::eq(a, b)),
                (G::Native(a), G::Native(b)) => assert_eq!(a, b),
                _ => panic!("font selection changed glyph source kind"),
            }
            for i in 0..glyphs.len() {
                assert_eq!(actual.glyphs().get(i), glyphs.get(i));
            }
            assert!(actual.glyphs().get(glyphs.len()).is_none());
        }
    }
    assert_eq!(display.font_slot_count(display.paints().len()), None);
    assert!(display
        .font_use(display.paints().len(), 0)
        .unwrap()
        .is_none());
}
