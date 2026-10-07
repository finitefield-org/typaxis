use super::*;
use std::collections::{BTreeMap, BTreeSet};
use typaxis_layout::book_v2::BookV2RebuiltBodyLineVariants;
use typaxis_pagination::book_v2::{
    prepare_book_v2_table_header_variant, BookV2TableMeasurements, BookV2TableWidthSource,
};

pub(super) fn paragraph_owners(v: &Value, owners: &mut BTreeSet<u32>) {
    if v["kind"] == "paragraph" {
        owners.insert(v["node_id"].as_u64().unwrap() as u32);
    }
    if let Some(a) = v.as_array() {
        for v in a {
            paragraph_owners(v, owners);
        }
    } else if let Some(o) = v.as_object() {
        for v in o.values() {
            paragraph_owners(v, owners);
        }
    }
}

pub(super) fn nest_header(data: &mut Value, notes: bool) {
    let table = if notes {
        &mut data["document"]["footnotes"][0]["blocks"][0]
    } else {
        &mut data["document"]["blocks"][0]
    };
    let mut child = table.clone();
    for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
        cell["blocks"].as_array_mut().unwrap().truncate(1);
    }
    let p = child["body"][0]["cells"][0]["blocks"][0].clone();
    child["caption"] = json!([p,{"kind":"page_break","node_id":0,"span":p["span"],"classes":[]},p]);
    table["caption"] = json!([p]);
    table["head"][0]["cells"][0]["blocks"] = json!([child]);
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
}

pub(super) fn check(
    set: &BookV2RebuiltBodyLineVariants<'_, '_, '_>,
    measurements: &[BookV2TableMeasurements<'_, '_, '_, '_>],
    limits: &M4EffectiveResourceLimits,
    expected: &BTreeSet<u32>,
    units: u32,
) {
    let base = &measurements[0];
    let build = |index, work, prior| {
        prepare_book_v2_table_header_variant(
            set,
            base,
            &measurements[index],
            0,
            limits,
            work,
            prior,
        )
    };
    let narrow = build(0, 1_000_000, 0).unwrap();
    let wide = build(1, 1_000_000, 0).unwrap();
    let copy = build(2, 1_000_000, 0).unwrap();
    assert!(narrow.height() > wide.height());
    assert_eq!(narrow.height(), copy.height());
    assert_ne!(narrow.leaves().len(), wide.leaves().len());
    assert_ne!(narrow.fingerprint(), wide.fingerprint());
    assert_eq!(narrow.fingerprint(), copy.fingerprint());
    narrow.verify(base, base, limits).unwrap();
    assert!(narrow.verify(base, &measurements[2], limits).is_err());
    assert!(wide
        .verify(&measurements[2], &measurements[1], limits)
        .is_err());
    for (i, plan) in [&narrow, &wide, &copy].into_iter().enumerate() {
        assert!(std::ptr::eq(plan.variant(), &measurements[i]));
        assert_eq!(plan.table_index(), 0);
        assert_eq!(
            plan.definition_index(),
            base.flow().table_source_definition(0).unwrap()
        );
        let mut coverage = BTreeMap::<u32, u32>::new();
        let mut local_items = BTreeSet::new();
        let mut captions = 0;
        for leaf in plan.leaves() {
            assert!(local_items.insert(leaf.variant_item_index()));
            let item = plan.variant().item(leaf.variant_item_index()).unwrap();
            let BookV2TableWidthSource::Paragraph {
                owner,
                units: range,
            } = leaf.source()
            else {
                panic!("text fixture")
            };
            assert_eq!(*owner, item.owner());
            assert_eq!(range.start, *coverage.get(&owner.get()).unwrap_or(&0));
            coverage.insert(owner.get(), range.end);
            assert!(leaf.top() >= Length::ZERO);
            assert!(
                leaf.top()
                    .checked_add(item.consumed_height().unwrap())
                    .unwrap()
                    <= plan.height()
            );
            if leaf.cell_owner().is_none() {
                captions += 1;
            }
        }
        // Expected owners come from the authored root header JSON, including
        // complete nested tables/captions, excluding the root caption and body.
        assert_eq!(coverage.keys().copied().collect::<BTreeSet<_>>(), *expected);
        assert!(coverage.values().all(|n| *n == units));
        assert_eq!(captions > 0, base.tables().len() > 1);
        assert_eq!(plan.remaining_height(plan.height()).unwrap(), Length::ZERO);
        assert!(plan
            .remaining_height(Length::from_raw(plan.height().raw() - 1).unwrap())
            .is_err());
        assert_eq!(
            plan.remaining_height(Length::from_raw(plan.height().raw() + 1).unwrap())
                .unwrap()
                .raw(),
            1
        );
        let exact = build(i, plan.work_steps(), 0).unwrap();
        assert_eq!(exact.fingerprint(), plan.fingerprint());
        assert!(build(i, plan.work_steps() - 1, 0).is_err());
        let raised = limits.base().get().max_fragments / 2;
        assert!(plan.record_charge() < raised);
        let added = build(i, plan.work_steps(), raised).unwrap().record_charge() - raised;
        let prior = limits.base().get().max_fragments - added;
        assert_eq!(
            build(i, plan.work_steps(), prior).unwrap().record_charge(),
            limits.base().get().max_fragments
        );
        assert!(build(i, plan.work_steps(), prior + 1).is_err());
    }
    assert!(prepare_book_v2_table_header_variant(
        set,
        base,
        &measurements[1],
        base.tables().len(),
        limits,
        1_000_000,
        0
    )
    .is_err());
    eprintln!("table header variants: tables={},definition={:?},height={}/{},leaves={}/{},work={},records={}",base.tables().len(),wide.definition_index(),narrow.height().raw(),wide.height().raw(),narrow.leaves().len(),wide.leaves().len(),wide.work_steps(),wide.record_charge());
}
