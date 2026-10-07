use super::*;
#[path = "book_v2_font_object_tests.rs"]
mod objects;
use typaxis_pdf::book_v2::{
    BookV2FontAuxiliaryKind as Auxiliary, BookV2FontStreamBuilder as Streams,
    BookV2FontStreamError as Error,
};
use typaxis_resources::book_v2::BookV2CidPlans;

fn decode(token: &str, bom: bool) -> String {
    assert!(token.starts_with('<') && token.ends_with('>'));
    let hex = &token[1..token.len() - 1];
    assert_eq!(hex.len() % 4, 0);
    let mut units = (0..hex.len())
        .step_by(4)
        .map(|i| u16::from_str_radix(&hex[i..i + 4], 16).unwrap());
    if bom {
        assert_eq!(units.next(), Some(0xfeff));
    }
    String::from_utf16(&units.collect::<Vec<_>>()).unwrap()
}
// Parse the emitted CMap independently of the production encoder and CID
// plan. Check declared block lengths, duplicate keys and code-space bounds.
fn cmap(bytes: &[u8]) -> BTreeMap<u16, String> {
    let source = std::str::from_utf8(bytes).unwrap();
    assert!(source.contains("1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n"));
    assert!(source.ends_with("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n"));
    let mut lines = source.lines();
    let mut result = BTreeMap::new();
    while let Some(line) = lines.next() {
        if let Some(count) = line.strip_suffix(" beginbfchar") {
            let count: usize = count.parse().unwrap();
            assert!((1..=100).contains(&count));
            for _ in 0..count {
                let fields = lines.next().unwrap().split_whitespace().collect::<Vec<_>>();
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].len(), 6);
                let cid = u16::from_str_radix(&fields[0][1..5], 16).unwrap();
                assert_ne!(cid, 0);
                assert!(result.insert(cid, decode(fields[1], false)).is_none());
            }
            assert_eq!(lines.next(), Some("endbfchar"));
        }
    }
    result
}
pub(super) fn check(
    source: &BookV2CidPlans<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
    limits: &M4EffectiveResourceLimits,
) {
    const MAX_WORK: u64 = 1_000_000_000;
    let mut object_builder = Streams::new(source, limits, MAX_WORK, 0, 0, 0, 0).unwrap();
    let object_streams = object_builder.build().unwrap();
    objects::check(&object_streams, limits);
    let mut different_base = limits.base().get().clone();
    different_base.max_output_bytes -= 1;
    let different_limits = M4EffectiveResourceLimits::new(
        typaxis_core::ValidatedResourceLimits::new(different_base).unwrap(),
        limits.extension().get().clone(),
    )
    .unwrap();
    assert!(matches!(
        Streams::new(source, &different_limits, MAX_WORK, 0, 0, 0, 0),
        Err(Error::Identity)
    ));
    let run = |records, spool, output, max_work, prior_work| -> Result<_, Error> {
        let mut builder =
            Streams::new(source, limits, max_work, records, spool, output, prior_work)?;
        let first = builder.build()?;
        assert!(std::ptr::eq(first.source(), source));
        assert_eq!(first.fonts().len(), source.fonts().len());
        let mut maps = Vec::new();
        let mut bytes = 0;
        for (index, font) in first.fonts().iter().enumerate() {
            assert_eq!(font.font_index(), index);
            let bindings = source.bindings(index).unwrap();
            let to_unicode = first.to_unicode(index).unwrap();
            let map = cmap(to_unicode);
            let expected = bindings
                .iter()
                .filter_map(|b| b.unicode().map(|c| (b.cid().get(), c.to_string())))
                .collect::<BTreeMap<_, _>>();
            assert_eq!(map, expected);
            maps.push(map);
            let auxiliary = first.auxiliary(index).unwrap();
            match source.fonts()[index].source().kind() {
                K::TrueType => {
                    assert_eq!(font.auxiliary_kind(), Auxiliary::TrueTypeCidToGid);
                    let gids = auxiliary
                        .chunks_exact(2)
                        .map(|b| u16::from_be_bytes([b[0], b[1]]))
                        .collect::<Vec<_>>();
                    assert_eq!(auxiliary.len(), (bindings.len() + 1) * 2);
                    assert_eq!(gids[0], 0);
                    for binding in bindings {
                        assert_eq!(
                            gids[binding.cid().get() as usize],
                            binding.subset_gid().get()
                        );
                    }
                }
                K::Cff1V2 => {
                    assert_eq!(font.auxiliary_kind(), Auxiliary::CffCidSet);
                    let count = source.fonts()[index].source().source().glyphs().len();
                    assert_eq!(auxiliary.len(), count.div_ceil(8));
                    for bit in 0..auxiliary.len() * 8 {
                        assert_eq!(auxiliary[bit / 8] & (0x80 >> (bit % 8)) != 0, bit < count);
                    }
                }
            }
            bytes += to_unicode.len() + auxiliary.len();
        }
        for (index, usage) in source.uses().iter().enumerate() {
            let mut extracted = source
                .cids(index)
                .unwrap()
                .iter()
                .filter_map(|cid| maps[usage.font_index()].get(&cid.get()))
                .cloned()
                .collect::<String>();
            if let Some(token) = first.actual_text(index) {
                assert!(usage.requires_actual_text());
                extracted = decode(std::str::from_utf8(token).unwrap(), true);
                assert_eq!(extracted, original(usage.actual_text().unwrap()));
                bytes += token.len();
            } else {
                assert!(!usage.requires_actual_text());
            }
            assert_eq!(extracted, original(usage.source().usage().text()));
        }
        assert_eq!(bytes, first.byte_length());
        assert_eq!(first.output_charge(), output + bytes as u64);
        assert!(first.to_unicode(usize::MAX).is_none());
        assert!(first.auxiliary(usize::MAX).is_none());
        assert!(first.actual_text(usize::MAX).is_none());
        assert_eq!(
            (
                first.record_charge(),
                first.spool_charge(),
                first.output_charge(),
                first.work_steps()
            ),
            (
                builder.record_charge(),
                builder.spool_charge(),
                builder.output_charge(),
                builder.work_steps()
            )
        );
        Ok((
            first.record_charge(),
            first.spool_charge(),
            first.output_charge(),
            first.work_steps(),
            first.fingerprint(),
        ))
    };
    let (records, spool, output, work, fp) = run(0, 0, 0, MAX_WORK, 0).unwrap();
    assert_eq!(
        run(0, 0, 0, work, 0).unwrap(),
        (records, spool, output, work, fp)
    );
    let base = limits.base().get();
    let prior_records = base.max_fragments - (records - source.record_charge());
    let prior_spool = base.max_spool_bytes - (spool - source.spool_charge());
    let prior_output = base.max_output_bytes - output;
    assert_eq!(
        run(prior_records, prior_spool, prior_output, work, 0).unwrap(),
        (
            base.max_fragments,
            base.max_spool_bytes,
            base.max_output_bytes,
            work,
            fp
        )
    );
    assert!(matches!(
        run(prior_records + 1, prior_spool, prior_output, work, 0),
        Err(Error::Records)
    ));
    assert!(matches!(
        run(prior_records, prior_spool + 1, prior_output, work, 0),
        Err(Error::Spool)
    ));
    // Even the no-font/no-text owner must honor an already exceeded budget.
    assert!(matches!(
        run(prior_records, prior_spool, prior_output + 1, work, 0),
        Err(Error::Output)
    ));
    assert!(matches!(run(0, 0, 0, work - 1, 0), Err(Error::Work)));
    assert_eq!(
        run(0, 0, 0, work + 17, source.work_steps() + 17).unwrap(),
        (records, spool, output, work + 17, fp)
    );
    let mut failed = Streams::new(source, limits, work - 1, 0, 0, 0, 0).unwrap();
    assert!(matches!(failed.build(), Err(Error::Work)));
    // A font-free pass has no encoding loop to consume work before its next
    // owner reservation. That retained reservation is not refunded on failure.
    let retry_records = records + u64::from(source.fonts().is_empty() && source.uses().is_empty());
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.output_charge(),
            failed.work_steps()
        ),
        (records, spool, output, work - 1)
    );
    assert!(matches!(failed.build(), Err(Error::Work)));
    assert_eq!(
        (
            failed.record_charge(),
            failed.spool_charge(),
            failed.output_charge(),
            failed.work_steps()
        ),
        (retry_records, spool, output, work - 1)
    );
    // Locate the first work boundary after the complete allocation reservation.
    let mut lower = source.work_steps();
    let mut upper = work - 1;
    while lower < upper {
        let middle = lower + (upper - lower) / 2;
        let mut probe = Streams::new(source, limits, middle, 0, 0, 0, 0).unwrap();
        assert!(matches!(probe.build(), Err(Error::Work)));
        if probe.record_charge() == records {
            upper = middle;
        } else {
            assert_eq!(probe.record_charge(), source.record_charge() + 1);
            lower = middle + 1;
        }
    }
    let mut early = Streams::new(source, limits, lower, 0, 0, 0, 0).unwrap();
    assert!(matches!(early.build(), Err(Error::Work)));
    assert_eq!(
        (
            early.record_charge(),
            early.spool_charge(),
            early.output_charge()
        ),
        (records, spool, output)
    );
    if lower > source.work_steps() {
        let mut before = Streams::new(source, limits, lower - 1, 0, 0, 0, 0).unwrap();
        assert!(matches!(before.build(), Err(Error::Work)));
        assert_eq!(
            (
                before.record_charge(),
                before.spool_charge(),
                before.output_charge()
            ),
            (source.record_charge() + 1, source.spool_charge(), 0)
        );
    }
    let mut repeated = Streams::new(source, limits, MAX_WORK, 0, 0, 0, 0).unwrap();
    let one = repeated.build().unwrap();
    let two = repeated.build().unwrap();
    assert_eq!(one.fingerprint(), two.fingerprint());
    assert_eq!(two.output_charge(), 2 * output);
    assert_eq!(
        two.spool_charge() - one.spool_charge(),
        spool - source.spool_charge()
    );
}
