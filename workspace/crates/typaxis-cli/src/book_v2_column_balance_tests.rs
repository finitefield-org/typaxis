use super::*;
use crate::book_v2_resources::with_budgeted_book_v2_column_sources as source_drive;
#[path = "book_v2_column_balance_probes.rs"]
mod probes;
const LIST_FONT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../samples/machine-package/staging/production-book-1/vmb-book/list-fonts/body-list-visible.ttf"));

fn balanced_input(
    root: &Root,
    mut data: Value,
    text: &str,
    font: Option<&[u8]>,
    caps: &M4EffectiveResourceLimits,
) -> PreparedBookV2Resources {
    if let Some(font) = font {
        data["resources"]["font_faces"][0]["media_type"] = if font.starts_with(b"OTTO") {
            "sfnt-cff1"
        } else {
            "sfnt-truetype-glyf"
        }
        .into();
        data["resources"]["font_faces"][0]["expected_sha256"] = typaxis_core::sha256(font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
            .into();
    }
    let body = body_with_source(root, data, text.as_bytes(), caps);
    if let Some(font) = font {
        fs::write(root.0.join("body.bin"), font).unwrap();
    }
    prepare_book_v2_resources(
        body,
        &root.context(),
        &config_with_extension(caps.base().get().clone(), caps.extension().get().clone()),
        caps,
    )
    .unwrap()
}

fn balance_spool_limits(
    caps: &M4EffectiveResourceLimits,
    maximum: u64,
) -> M4EffectiveResourceLimits {
    let mut base = caps.base().get().clone();
    base.max_spool_bytes = maximum;
    let mut extension = caps.extension().get().clone();
    extension.max_font_subset_bytes = extension.max_font_subset_bytes.min(maximum);
    M4EffectiveResourceLimits::new(ValidatedResourceLimits::new(base).unwrap(), extension).unwrap()
}

fn balanced_caps() -> M4EffectiveResourceLimits {
    changed_caps(&caps(), |c| {
        c.max_column_balance_candidates = 128;
        c.max_layout_passes = 512;
        c.max_line_reshape_passes = 512;
        c.max_footnote_reflows_per_page = 512;
    })
}

fn balanced_body(text: &str, count: usize, notes: bool) -> Value {
    let mut data = plain_data(text, notes, false);
    let paragraph = data["document"]["blocks"][1].clone();
    let blocks = data["document"]["blocks"].as_array_mut().unwrap();
    blocks.resize(count, paragraph);
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"] = json!({"x":17*65536,"y":10*65536,"width":210*65536,"height":256*65536});
        master["column_layout"] =
            json!({"count":2,"gap":10*65536,"fill":"sequential","balance":"last_page"});
        if notes {
            master["footnote"] =
                json!({"x":23*65536,"y":300*65536,"width":300*65536,"height":120*65536});
        }
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

#[derive(Clone, Debug, PartialEq)]
struct BodyBalance {
    target: i64,
    positive_extent: i64,
    first_target: i64,
    units: Vec<usize>,
    used: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq)]
struct BalanceTrace {
    extent: i64,
    full: i64,
    first: i64,
    targets: Vec<i64>,
    table_certificates: usize,
    zero_table_observations: usize,
    skipped_table_capacities: u64,
}

fn balance_u64(bytes: &[u8], at: usize) -> u64 {
    u64::from_be_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn balance_i64(bytes: &[u8], at: usize) -> i64 {
    i64::from_be_bytes(bytes[at..at + 8].try_into().unwrap())
}

fn ordinary_capacity_prefix(
    candidate: &BookV2ColumnBalanceCandidate,
    ranges: &[std::ops::Range<usize>],
    extent: i64,
) {
    let bytes = candidate.capacity_canonical_bytes();
    let mut at = 48;
    for (column, range) in ranges.iter().enumerate() {
        assert_eq!(
            u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap()) as usize,
            column
        );
        at += 2;
        for item in [range.start, range.end] {
            assert_eq!(balance_u64(bytes, at) as usize, item);
            assert!(
                bytes[at + 8..at + 83].iter().all(|byte| *byte == 0),
                "foreign table cursor in ordinary capacity prefix"
            );
            at += 83;
        }
        let used = extent * range.len() as i64;
        assert_eq!(balance_i64(bytes, at), used);
        at += 8;
        assert_eq!(bytes[at], u8::from(!range.is_empty()));
        at += 1;
        assert_eq!(balance_i64(bytes, at), 0);
        at += 8;
        assert_eq!(balance_u64(bytes, at), u64::from(!range.is_empty()));
        at += 8;
        if !range.is_empty() {
            assert_eq!(balance_i64(bytes, at), 0);
            assert_eq!(balance_i64(bytes, at + 8), used);
            at += 16;
            assert_eq!(bytes[at], 1);
            at += 1;
            assert_eq!(balance_u64(bytes, at) as usize, range.start);
            assert_eq!(balance_u64(bytes, at + 8) as usize, range.end);
            at += 16;
            assert!(bytes[at..at + 33].iter().all(|byte| *byte == 0));
            at += 33;
        }
        assert_eq!(
            bytes[at], 1,
            "each rejected ordinary column retains its blocker"
        );
        at += 1;
    }
    assert_eq!(at, bytes.len());
}

fn balance_trace(page: &BookV2ColumnPageSelection<'_, '_, '_, '_, '_>) -> Option<BalanceTrace> {
    let receipt = page.balance()?;
    let bytes = receipt.canonical_bytes();
    assert_eq!(receipt.fingerprint(), typaxis_core::sha256(bytes));
    assert_eq!(
        &bytes[..32],
        &typaxis_core::sha256(BOOK_V2_COLUMN_BALANCE_ALGORITHM.as_bytes())
    );
    assert_eq!(&bytes[32..64], &receipt.input_fingerprint());
    assert_eq!(
        receipt.input_fingerprint(),
        typaxis_core::sha256(receipt.input_canonical_bytes())
    );
    assert_eq!(
        balance_i64(bytes, 64),
        receipt.selected_height().get().raw()
    );
    assert_eq!(balance_u64(bytes, 72) as usize, receipt.candidates().len());
    assert_eq!(receipt.page_index(), page.page_index());
    assert_eq!(
        receipt.full_height(),
        page.candidate().frames().body().height()
    );
    let count = i64::from(page.candidate().frames().column_count());
    let first = ((receipt.initial_extent().raw() + count - 1) / count)
        .clamp(1, receipt.full_height().get().raw());
    assert_eq!(receipt.initial_height().get().raw(), first);
    assert_eq!(receipt.candidates()[0].target(), receipt.initial_height());
    let mut at = 80;
    let mut targets = Vec::new();
    let mut rejection_states = BTreeSet::new();
    let mut table_certificates = 0;
    let mut zero_table_observations = 0;
    let mut skipped_table_capacities = 0;
    for (index, candidate) in receipt.candidates().iter().enumerate() {
        assert_eq!(balance_i64(bytes, at), candidate.target().get().raw());
        assert_eq!(candidate.target_height(), candidate.target());
        at += 8;
        assert_eq!(bytes[at], u8::from(candidate.terminal()));
        at += 1;
        assert_eq!(&bytes[at..at + 32], &candidate.rejection_fingerprint());
        at += 32;
        let rejection_start = at;
        assert_eq!(
            balance_u64(bytes, at) as usize,
            candidate.rejections().len()
        );
        at += 8;
        for rejection in candidate.rejections() {
            assert_eq!(
                u16::from_be_bytes(bytes[at..at + 2].try_into().unwrap()),
                rejection.column_index()
            );
            at += 2;
            assert_eq!(
                u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap()),
                rejection.owner().get()
            );
            at += 4;
            assert_eq!(
                balance_u64(bytes, at) as usize,
                rejection.source_items().start
            );
            at += 8;
            assert_eq!(
                balance_u64(bytes, at) as usize,
                rejection.source_items().end
            );
            at += 8;
            assert_eq!(
                balance_i64(bytes, at),
                rejection.required_height().get().raw()
            );
            at += 8;
            assert_eq!(balance_i64(bytes, at), rejection.deficit().get().raw());
            at += 8;
            assert_eq!(
                bytes[at],
                u8::from(rejection.table_group_fingerprint().is_some())
            );
            at += 1;
            if let Some(fingerprint) = rejection.table_group_fingerprint() {
                assert_eq!(&bytes[at..at + 32], &fingerprint);
                at += 32;
                let proofs = rejection.table_group_certificates().collect::<Vec<_>>();
                assert!(!proofs.is_empty());
                assert_eq!(typaxis_core::sha256(proofs.last().unwrap()), fingerprint);
                let mut prior_chain = [0; 32];
                let mut prior_group = [0; 32];
                for proof in proofs {
                    table_certificates += 1;
                    if proof[..32] == typaxis_core::sha256(b"typaxis.book-2-table-capacity-group/1")
                    {
                        assert!(proof.len() >= 155);
                        let current = balance_i64(proof, 115);
                        let required = balance_i64(proof, 123);
                        assert!(required > current);
                        let trials = balance_u64(proof, 139);
                        let skipped = balance_u64(proof, 147);
                        assert!(trials >= 2);
                        assert_eq!(trials - 2 + skipped, (required - current) as u64);
                        skipped_table_capacities += skipped;
                        prior_group = typaxis_core::sha256(proof);
                    } else if proof[..32]
                        == typaxis_core::sha256(b"typaxis.book-2-table-capacity-observation/1")
                    {
                        assert!(proof.len() >= 145);
                        let capacity = balance_i64(proof, 115);
                        let maximum = balance_i64(proof, 123);
                        let used = balance_i64(proof, 131);
                        assert!(
                            capacity >= 0 && capacity <= maximum && used >= 0 && used <= capacity
                        );
                        assert!(proof[139] <= 1 && proof[144] <= 1);
                        if capacity == 0 {
                            assert_eq!(used, 0);
                            zero_table_observations += 1;
                        }
                        prior_group = typaxis_core::sha256(proof);
                    } else {
                        assert_eq!(
                            &proof[..32],
                            &typaxis_core::sha256(b"typaxis.book-2-column-table-keep-group/1")
                        );
                        assert_eq!(proof.len(), 128);
                        assert_eq!(&proof[32..64], &prior_chain);
                        assert_eq!(&proof[64..96], &prior_group);
                        assert!(balance_i64(proof, 120) >= balance_i64(proof, 112));
                        prior_chain = typaxis_core::sha256(proof);
                    }
                }
            } else {
                assert_eq!(rejection.table_group_certificates().len(), 0);
            }
            assert!(rejection.column_index() < page.candidate().frames().column_count());
            assert!(!rejection.source_items().is_empty());
            assert_eq!(
                rejection.required_height().get().raw() - candidate.target().get().raw(),
                rejection.deficit().get().raw()
            );
            assert!(rejection.deficit().get().raw() > 0);
        }
        let rejection_end = at;
        assert_eq!(&bytes[at..at + 32], &candidate.capacity_fingerprint());
        at += 32;
        let capacity_size = balance_u64(bytes, at) as usize;
        at += 8;
        assert_eq!(capacity_size, candidate.capacity_canonical_bytes().len());
        assert_eq!(
            &bytes[at..at + capacity_size],
            candidate.capacity_canonical_bytes()
        );
        at += capacity_size;
        let terminal = index + 1 == receipt.candidates().len();
        assert_eq!(candidate.terminal(), terminal);
        if terminal {
            assert!(candidate.rejections().is_empty());
            assert_eq!(candidate.rejection_fingerprint(), [0; 32]);
            assert_eq!(candidate.target(), receipt.selected_height());
            assert!(candidate.capacity_canonical_bytes().is_empty());
            assert_eq!(candidate.capacity_fingerprint(), [0; 32]);
        } else {
            assert_eq!(
                candidate.rejection_fingerprint(),
                typaxis_core::sha256(&bytes[rejection_start..rejection_end])
            );
            let capacity = candidate.capacity_canonical_bytes();
            assert_eq!(
                candidate.capacity_fingerprint(),
                typaxis_core::sha256(capacity)
            );
            assert_eq!(
                &capacity[..32],
                &typaxis_core::sha256(b"typaxis.book-2-column-capacity-trace/1")
            );
            assert_eq!(balance_i64(capacity, 32), candidate.target().get().raw());
            assert_eq!(balance_u64(capacity, 40), count as u64);
            assert!(rejection_states.insert(candidate.rejection_fingerprint()));
            let least = candidate
                .rejections()
                .iter()
                .min_by_key(|rejection| {
                    (
                        rejection.deficit().get().raw(),
                        rejection.column_index(),
                        rejection.source_items().start,
                    )
                })
                .unwrap();
            let next = receipt.candidates()[index + 1].target().get().raw();
            assert_eq!(
                next,
                candidate.target().get().raw() + least.deficit().get().raw()
            );
            assert!(
                next > candidate.target().get().raw() && next <= receipt.full_height().get().raw()
            );
        }
        targets.push(candidate.target().get().raw());
    }
    assert_eq!(at, bytes.len());
    assert!(page
        .candidate()
        .columns()
        .iter()
        .all(|column| column.bounds().height() == receipt.selected_height()));
    Some(BalanceTrace {
        extent: receipt.initial_extent().raw(),
        full: receipt.full_height().get().raw(),
        first,
        targets,
        table_certificates,
        zero_table_observations,
        skipped_table_capacities,
    })
}

fn body_balance(
    input: &PreparedBookV2Resources,
    data: &Value,
    caps: &M4EffectiveResourceLimits,
) -> BodyBalance {
    let count = data["document"]["blocks"].as_array().unwrap().len();
    let mut budget = BookV2ColumnPageBudget::new(caps, 100_000_000);
    source_drive(
        input,
        caps,
        MODE,
        &mut budget,
        |closed, search, observed| {
            assert_eq!(observed.record_charge(), search.record_charge());
            source_closure::coverage(closed, search, data);
            probes::record("ordinary-body", closed, search, data, caps);
            let pages = closed.geometry().pages();
            assert_eq!(pages.len(), 1);
            let page = &pages[0];
            assert_eq!(page.columns().len(), 2);
            let original = search.source_lines();
            assert_eq!(original.paragraphs().len(), count);
            // These paragraphs have no authored spacing or markers. Their actual
            // selected line metrics are the complete original positive extents.
            let extents = original
                .paragraphs()
                .iter()
                .map(|p| {
                    let selected = p.selected().unwrap();
                    assert_eq!(selected.lines().len(), 1);
                    selected.lines()[0]
                        .line()
                        .metrics()
                        .line_height()
                        .get()
                        .raw()
                })
                .collect::<Vec<_>>();
            assert!(extents.iter().all(|extent| *extent == extents[0]));
            let positive_extent = extents.iter().sum::<i64>();
            let first_target = (positive_extent + 1) / 2;
            let target = extents[0] * count.div_ceil(2) as i64;
            let trace = balance_trace(page.selection()).unwrap();
            assert_eq!(trace.extent, positive_extent);
            assert_eq!(trace.first, first_target);
            assert_eq!(
                trace.targets,
                if count == 6 {
                    vec![first_target]
                } else {
                    vec![first_target, target]
                }
            );
            if count != 6 {
                ordinary_capacity_prefix(
                    &page.selection().balance().unwrap().candidates()[0],
                    &if count == 5 {
                        vec![0..2, 2..4]
                    } else {
                        vec![0..0, 0..0]
                    },
                    extents[0],
                );
                let rejections = page.selection().balance().unwrap().candidates()[0].rejections();
                assert_eq!(rejections.len(), 2);
                for (column, rejection) in rejections.iter().enumerate() {
                    assert_eq!(rejection.column_index() as usize, column);
                    let start = if count == 5 { 2 + column * 2 } else { 0 };
                    assert_eq!(rejection.source_items(), start..start + 1);
                    assert_eq!(
                        rejection.owner().get(),
                        data["document"]["blocks"][start]["node_id"]
                            .as_u64()
                            .unwrap() as u32
                    );
                    assert_eq!(rejection.required_height().get().raw(), target);
                    assert_eq!(rejection.deficit().get().raw(), target - first_target);
                }
            }
            let mut units = Vec::new();
            let mut used = Vec::new();
            for (index, column) in page.columns().iter().enumerate() {
                let authored = page
                    .selection()
                    .candidate()
                    .frames()
                    .column(index as u16)
                    .unwrap();
                assert_eq!(
                    (
                        column.bounds().x(),
                        column.bounds().y(),
                        column.bounds().width()
                    ),
                    (authored.x(), authored.y(), authored.width())
                );
                assert_eq!(authored.height().get().raw(), 256 * 65536);
                assert_eq!(column.bounds().height().get().raw(), target);
                let selection = &page.selection().candidate().columns()[index];
                assert_eq!(selection.bounds(), column.bounds());
                let original_units = selection
                    .parts()
                    .iter()
                    .map(|part| part.items().unwrap().len())
                    .sum::<usize>();
                units.push(original_units);
                used.push(selection.used_height().raw());
                for fragment in column.fragments() {
                    inside(
                        column.bounds(),
                        page.fragments()[fragment].fragment().bounds(),
                    );
                }
            }
            assert_eq!(units, vec![count.div_ceil(2), count / 2]);
            assert_eq!(
                used,
                units
                    .iter()
                    .map(|count| extents[0] * *count as i64)
                    .collect::<Vec<_>>()
            );
            BodyBalance {
                target,
                positive_extent,
                first_target,
                units,
                used,
            }
        },
    )
    .unwrap()
}

#[test]
fn book_v2_column_balance_derives_equal_and_odd_body_targets_from_original_extents() {
    let caps = balanced_caps();
    for count in [6, 5, 1] {
        let root = Root::new();
        let data = balanced_body("Pro", count, false);
        let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
        let selected = body_balance(&input, &data, &caps);
        assert_eq!(selected.target == selected.first_target, count == 6);
        assert_eq!(body_balance(&input, &data, &caps), selected);
    }
}

#[derive(Clone, Debug, PartialEq)]
struct PageShape {
    page: u32,
    named: Option<String>,
    original_heights: Vec<i64>,
    heights: Vec<i64>,
    widths: Vec<i64>,
    used: Vec<i64>,
    owners: Vec<Vec<u32>>,
    notes: usize,
    forced: Option<u32>,
    balance: Option<BalanceTrace>,
}

fn shapes(
    closed: &BookV2ColumnSourceClosure<'_, '_, '_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    data: &Value,
    caps: &M4EffectiveResourceLimits,
) -> Vec<PageShape> {
    source_closure::coverage_retained(closed, search, data);
    probes::record("selected-shapes", closed, search, data, caps);
    closed
        .geometry()
        .pages()
        .iter()
        .map(|page| {
            let selected = page.selection();
            let mut original_heights = Vec::new();
            let mut heights = Vec::new();
            let mut widths = Vec::new();
            let mut used = Vec::new();
            let mut owners = Vec::new();
            for (index, column) in page.columns().iter().enumerate() {
                let original = selected.candidate().frames().column(index as u16).unwrap();
                assert_eq!(
                    (
                        column.bounds().x(),
                        column.bounds().y(),
                        column.bounds().width()
                    ),
                    (original.x(), original.y(), original.width())
                );
                original_heights.push(original.height().get().raw());
                heights.push(column.bounds().height().get().raw());
                widths.push(column.bounds().width().get().raw());
                used.push(selected.candidate().columns()[index].used_height().raw());
                let mut source = Vec::new();
                for index in column.fragments() {
                    let fragment = page.fragments()[index];
                    inside(column.bounds(), fragment.fragment().bounds());
                    source.push(fragment.fragment().owner().get());
                }
                owners.push(source);
            }
            PageShape {
                page: selected.page_index(),
                named: selected.named_page().map(str::to_owned),
                original_heights,
                heights,
                widths,
                used,
                owners,
                notes: page
                    .fragments()
                    .iter()
                    .filter(|f| f.definition_index().is_some())
                    .count(),
                forced: selected.forced_break().map(|owner| owner.get()),
                balance: balance_trace(selected),
            }
        })
        .collect()
}

fn source_shapes(data: &Value, text: &str, caps: &M4EffectiveResourceLimits) -> Vec<PageShape> {
    let root = Root::new();
    let input = balanced_input(&root, data.clone(), text, None, caps);
    let mut budget = BookV2ColumnPageBudget::new(caps, 1_000_000_000);
    source_drive(&input, caps, MODE, &mut budget, |closed, search, _| {
        shapes(closed, search, data, caps)
    })
    .unwrap()
}

#[test]
fn book_v2_column_balance_preserves_original_empty_anchor_line_extents() {
    let caps = balanced_caps();
    for visible in [false, true] {
        let mut data = balanced_body("Pro", 5, false);
        for (index, paragraph) in data["document"]["blocks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .enumerate()
        {
            if visible && index == 2 {
                continue;
            }
            paragraph["children"] = json!([{"kind":"anchor","node_id":0,"span":paragraph["span"],
                "anchor_id":format!("balanced-empty-{index}")}]);
        }
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"],
            &mut 0,
        );
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let result = shapes(closed, search, &data, &caps);
            assert_eq!(result.len(), 1);
            let extents = search
                .source_items(None)
                .unwrap()
                .iter()
                .map(|item| item.consumed_height().unwrap().raw())
                .collect::<Vec<_>>();
            assert_eq!(extents.len(), 5);
            let original = search.source_lines();
            for (index, paragraph) in original.paragraphs().iter().enumerate() {
                let line = &paragraph.selected().unwrap().lines()[0].line();
                assert_eq!(extents[index], line.metrics().line_height().get().raw());
                assert!(extents[index] > 0);
                assert_eq!(line.start_unit() == line.end_unit(), !visible || index != 2);
            }
            let extent = extents.iter().sum::<i64>();
            let target = (0..=5)
                .map(|cut| {
                    extents[..cut]
                        .iter()
                        .sum::<i64>()
                        .max(extents[cut..].iter().sum::<i64>())
                })
                .min()
                .unwrap();
            let trace = result[0].balance.as_ref().unwrap();
            assert_eq!(trace.extent, extent);
            assert_eq!(trace.first, (extent + 1) / 2);
            assert_eq!(trace.targets.last(), Some(&target));
            assert_eq!(result[0].heights, [target; 2]);
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_balance_preserves_leading_consecutive_and_trailing_forced_blank_pages() {
    let caps = balanced_caps();
    let mut data = balanced_body("Pro", 5, false);
    let paragraphs = data["document"]["blocks"].as_array().unwrap().clone();
    let command =
        json!({"kind":"page_break","node_id":0,"classes":[],"span":paragraphs[0]["span"]});
    let mut blocks = vec![command.clone(), command.clone()];
    blocks.extend(paragraphs);
    blocks.extend([command.clone(), command]);
    data["document"]["blocks"] = blocks.into();
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    let result = source_shapes(&data, "Pro", &caps);
    assert_eq!(result.len(), 5);
    assert_eq!(
        result.iter().filter(|page| page.forced.is_some()).count(),
        4
    );
    for page in &result {
        assert_eq!(page.original_heights, vec![256 * 65536; 2]);
        if page.page == 2 {
            assert!(page.balance.is_some());
            assert_eq!(page.owners.iter().map(Vec::len).collect::<Vec<_>>(), [3, 2]);
            assert_eq!(page.heights, vec![page.used[0]; 2]);
            assert!(page.heights[0] < page.original_heights[0]);
        } else {
            assert!(page.balance.is_none());
            assert!(page.owners.iter().all(Vec::is_empty));
            assert_eq!(page.heights, page.original_heights);
        }
    }
    assert_eq!(source_shapes(&data, "Pro", &caps), result);

    let mut empty = balanced_body("Pro", 1, false);
    let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":empty["document"]["blocks"][0]["span"]});
    empty["document"]["blocks"] = json!([command, command]);
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut empty["document"],
        &mut 0,
    );
    let root = Root::new();
    let input = balanced_input(&root, empty.clone(), "Pro", None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
    source_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        |closed, search, observed| {
            let result = shapes(closed, search, &empty, &caps);
            assert_eq!(result.len(), 3);
            assert!(result
                .iter()
                .all(|page| page.owners.iter().all(Vec::is_empty)
                    && page.heights == page.original_heights
                    && page.balance.is_none()));
            assert_eq!(closed.semantic_fragments(), 0);
            assert!(search
                .source_items(None)
                .unwrap()
                .iter()
                .all(|item| item.source().is_none() && item.consumed_height().unwrap().raw() == 0));
            assert_eq!(observed.balance_candidates(), 0);
            assert_eq!(search.balance_candidates(), 0);
        },
    )
    .unwrap();
}

#[test]
fn book_v2_column_balance_uses_the_global_final_nonempty_page_for_notes_only_endings() {
    let caps = balanced_caps();
    let mut data = balanced_body("Pro", 1, true);
    let paragraph = data["document"]["footnotes"][0]["blocks"][0].clone();
    data["document"]["footnotes"][0]["blocks"] = vec![paragraph; 7].into();
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["footnote"]["height"] =
            (24 * 65536 + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW).into();
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    let result = source_shapes(&data, "Pro", &caps);
    assert!(result.len() > 1);
    assert_eq!(result[0].owners.iter().map(Vec::len).sum::<usize>(), 1);
    assert!(result.last().unwrap().notes > 0);
    for page in &result {
        assert!(page.balance.is_none());
        assert_eq!(page.heights, page.original_heights);
        if page.page > 0 {
            assert!(page.owners.iter().all(Vec::is_empty));
        }
    }
    assert_eq!(source_shapes(&data, "Pro", &caps), result);
}

#[test]
fn book_v2_column_balance_retains_one_original_body_and_note_endpoint() {
    let caps = balanced_caps();
    for reference_at in [0, 4] {
        let mut data = balanced_body("Pro", 5, true);
        if reference_at != 0 {
            let reference = data["document"]["blocks"][0]["children"]
                .as_array_mut()
                .unwrap()
                .pop()
                .unwrap();
            assert_eq!(reference["kind"], "footnote_reference");
            data["document"]["blocks"][reference_at]["children"]
                .as_array_mut()
                .unwrap()
                .push(reference);
            crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
                &mut data["document"],
                &mut 0,
            );
        }
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let pages = shapes(closed, search, &data, &caps);
            assert_eq!(pages.len(), 1);
            let items = search.source_items(None).unwrap();
            assert_eq!(items.len(), 5);
            assert!(items.iter().all(|item| !item.keep_with_next()
                && item.space_before().raw() == 0
                && item.space_after().raw() == 0
                && item.leading().raw() == 0
                && item.trailing().raw() == 0));
            let extents = items
                .iter()
                .map(|item| item.consumed_height().unwrap().raw())
                .collect::<Vec<_>>();
            let extent = extents.iter().sum::<i64>();
            // Enumerate every legal original paragraph boundary independently.
            let target = (0..=extents.len())
                .map(|cut| {
                    extents[..cut]
                        .iter()
                        .sum::<i64>()
                        .max(extents[cut..].iter().sum::<i64>())
                })
                .min()
                .unwrap();
            let trace = pages[0].balance.as_ref().unwrap();
            assert_eq!(trace.extent, extent);
            assert_eq!(trace.first, (extent + 1) / 2);
            assert_eq!(trace.targets.last(), Some(&target));
            assert_eq!(pages[0].heights, [target; 2]);
            assert_eq!(pages[0].notes, 2);
            assert_eq!(closed.unreferenced_definitions(), 1);
            let page = &closed.geometry().pages()[0];
            let selected = page.selection();
            let original_note = selected.candidate().frames().footnote().unwrap();
            let selected_note = selected.candidate().footnote_bounds().unwrap();
            assert_eq!(selected_note.x(), original_note.x());
            assert_eq!(selected_note.width(), original_note.width());
            assert_eq!(
                selected_note.y().raw() + selected_note.height().get().raw(),
                original_note.y().raw() + original_note.height().get().raw()
            );
            assert_eq!(
                selected_note.height().get().raw(),
                search
                    .source_items(Some(0))
                    .unwrap()
                    .iter()
                    .map(|item| item.consumed_height().unwrap().raw())
                    .sum::<i64>()
                    + typaxis_layout::FOOTNOTE_SEPARATOR_BAND_RAW
            );
            assert_eq!(page.footnote_markers().len(), 1);
            let state = selected.next_state().source_state();
            assert_eq!(state.next_item(), 5);
            assert_eq!(
                state.definition_status(0),
                Some(typaxis_pagination::ProductionFootnoteDemandStatus::Complete)
            );
            assert_eq!(
                state.definition_status(1),
                Some(typaxis_pagination::ProductionFootnoteDemandStatus::Unreferenced)
            );
            assert!(state.pending_definitions().is_empty());
            for note in page
                .fragments()
                .iter()
                .filter(|item| item.definition_index().is_some())
            {
                assert_eq!(note.definition_index(), Some(0));
                inside(
                    selected.candidate().frames().footnote().unwrap(),
                    note.fragment().bounds(),
                );
            }
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_balance_retains_measured_keep_groups_and_widow_orphan_cuts() {
    let caps = balanced_caps();
    for keep in [true, false] {
        let mut data = balanced_body("Pro", if keep { 4 } else { 1 }, false);
        if keep {
            for paragraph in &mut data["document"]["blocks"].as_array_mut().unwrap()[..2] {
                paragraph["classes"] = json!(["balanced-keep"]);
            }
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":"balanced-keep","selector":"paragraph.balanced-keep","source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
        } else {
            let paragraph = &mut data["document"]["blocks"][0];
            let text = paragraph["children"][0].clone();
            let command = json!({"kind":"hard_break","node_id":0,"span":paragraph["span"]});
            paragraph["children"] = json!([text, command, text, command, text, command, text]);
        }
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"],
            &mut 0,
        );
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let selected = shapes(closed, search, &data, &caps);
            assert_eq!(selected.len(), 1);
            let lines = search.source_lines();
            let extents = lines
                .paragraphs()
                .iter()
                .flat_map(|p| p.selected().unwrap().lines())
                .map(|line| line.line().metrics().line_height().get().raw())
                .collect::<Vec<_>>();
            assert_eq!(extents.len(), 4);
            assert!(extents.iter().all(|extent| *extent == extents[0]));
            let target = extents[0] * if keep { 3 } else { 2 };
            let trace = selected[0].balance.as_ref().unwrap();
            assert_eq!(trace.extent, extents.iter().sum::<i64>());
            assert_eq!(
                trace.targets,
                if keep {
                    vec![2 * extents[0], 3 * extents[0]]
                } else {
                    vec![2 * extents[0]]
                }
            );
            assert_eq!(selected[0].heights, vec![target; 2]);
            let expected = if keep { vec![3, 1] } else { vec![2, 2] };
            assert_eq!(
                selected[0].owners.iter().map(Vec::len).collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                selected[0].used,
                expected
                    .into_iter()
                    .map(|count| extents[0] * count as i64)
                    .collect::<Vec<_>>()
            );
            if keep {
                // The two authored keeps tie the first three original leaves.
                let original = data["document"]["blocks"].as_array().unwrap()[..3]
                    .iter()
                    .map(|p| p["node_id"].as_u64().unwrap() as u32)
                    .collect::<Vec<_>>();
                assert_eq!(selected[0].owners[0], original);
            } else {
                // Both cuts leave at least two original lines on either side.
                assert_eq!(selected[0].owners[0], selected[0].owners[1]);
            }
        })
        .unwrap();
    }
}

fn list_body(data: &mut Value, ordered: bool) {
    let blocks = data["document"]["blocks"].as_array().unwrap();
    let span = blocks[0]["span"].clone();
    let items = blocks
        .iter()
        .map(|p| json!({"node_id":0,"span":span,"blocks":[p]}))
        .collect::<Vec<_>>();
    data["document"]["blocks"] = json!([{"kind":"list","node_id":0,"span":span,"classes":[],"ordered":ordered,
        "start":if ordered {json!(1)} else {Value::Null},"items":items}]);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    let mut rule = rules
        .iter()
        .find(|rule| {
            rule["selector"] == "paragraph"
                && rule["declarations"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|d| d["name"] == "font_family")
        })
        .unwrap()
        .clone();
    rule["selector"] = "list".into();
    rule["style_id"] = "balanced-list-font".into();
    rule["source_order"] = (rules
        .iter()
        .filter_map(|rule| rule["source_order"].as_u64())
        .max()
        .unwrap()
        + 1)
    .into();
    rules.push(rule);
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
}

#[test]
fn book_v2_column_balance_derives_spaced_hard_group_costs_from_original_metrics() {
    let caps = balanced_caps();
    for keep_indices in [[0, 2], [1, 3]] {
        let mut data = balanced_body("Pro", 5, false);
        for index in 0..5 {
            data["document"]["blocks"][index]["classes"] = json!([format!("balanced-gap-{index}")]);
            let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
            rules.push(json!({"style_id":format!("balanced-gap-{index}"),"selector":format!("paragraph.balanced-gap-{index}"),
                "source_order":rules.len(),"extends":null,"declarations":[
                    {"name":"space_before","important":false,"value":{"kind":"length","value":([9,1,13,2,17][index]*65536)}},
                    {"name":"space_after","important":false,"value":{"kind":"length","value":([2,8,1,14,5][index]*65536)}},
                    {"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":keep_indices.contains(&index)}}
                ]}));
        }
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let selected = shapes(closed, search, &data, &caps);
            assert_eq!(selected.len(), 1);
            let items = search.source_items(None).unwrap();
            assert_eq!(items.len(), 5);
            assert_eq!(
                items
                    .iter()
                    .enumerate()
                    .filter_map(|(index, item)| item.keep_with_next().then_some(index))
                    .collect::<Vec<_>>(),
                keep_indices
            );
            let occupancy = |range: std::ops::Range<usize>| {
                items[range.clone()]
                    .iter()
                    .map(|item| item.consumed_height().unwrap().raw())
                    .sum::<i64>()
                    + range
                        .clone()
                        .skip(1)
                        .map(|index| {
                            items[index - 1].space_after().raw() + items[index].space_before().raw()
                        })
                        .sum::<i64>()
            };
            let extent = occupancy(0..5);
            let target = (0..=5)
                .filter(|cut| *cut == 0 || !items[*cut - 1].keep_with_next())
                .map(|cut| occupancy(0..cut).max(occupancy(cut..5)))
                .min()
                .unwrap();
            let trace = selected[0].balance.as_ref().unwrap();
            assert_eq!(trace.extent, extent);
            assert_eq!(trace.first, (extent + 1) / 2);
            assert_eq!(trace.targets.last(), Some(&target));
            assert_eq!(selected[0].heights, [target; 2]);
            assert!(
                target > trace.first,
                "unequal original gaps must require a measured deficit"
            );
            let page = &closed.geometry().pages()[0];
            let mut end = 0;
            for (index, column) in page.selection().candidate().columns().iter().enumerate() {
                let ranges = column
                    .parts()
                    .iter()
                    .map(|part| part.items().unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(ranges.len(), 1);
                let range = ranges[0].clone();
                assert_eq!(range.start, end);
                assert_eq!(column.used_height().raw(), occupancy(range.clone()));
                if range.end < 5 {
                    assert!(
                        !items[range.end - 1].keep_with_next(),
                        "hard keep column {index}"
                    );
                }
                end = range.end;
            }
            assert_eq!(end, 5);
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_balance_keeps_original_list_markers_in_the_selected_columns() {
    let caps = balanced_caps();
    for ordered in [false, true] {
        let mut data = balanced_body("Pro", 5, false);
        list_body(&mut data, ordered);
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", Some(LIST_FONT), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let selected = shapes(closed, search, &data, &caps);
            assert_eq!(selected.len(), 1);
            assert_eq!(
                selected[0].owners.iter().map(Vec::len).collect::<Vec<_>>(),
                [3, 2]
            );
            let extent = search.source_lines().paragraphs()[0]
                .selected()
                .unwrap()
                .lines()[0]
                .line()
                .metrics()
                .line_height()
                .get()
                .raw();
            assert_eq!(selected[0].heights, vec![3 * extent; 2]);
            let page = &closed.geometry().pages()[0];
            assert_eq!(page.list_markers().len(), 5);
            let mut marker_columns = Vec::new();
            for marker in page.list_markers() {
                let fragment = marker.fragment_index() as usize;
                let column = page.fragment_column(fragment).unwrap();
                marker_columns.push(column);
                inside(
                    page.columns()[usize::from(column)].bounds(),
                    marker.bounds(),
                );
                assert_eq!(
                    marker.baseline(),
                    page.fragments()[fragment].fragment().baseline().unwrap()
                );
            }
            assert_eq!(marker_columns, [0, 0, 0, 1, 1]);
        })
        .unwrap();
    }
}

#[test]
fn book_v2_column_balance_follows_named_parity_counts_and_actual_residual_line_widths() {
    let caps = balanced_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro Pro";
    for blanks in [1, 2] {
        let mut data = balanced_body(text, 5, false);
        for paragraph in data["document"]["blocks"].as_array_mut().unwrap() {
            paragraph["classes"] = json!(["balanced-appendix"]);
        }
        let command = json!({"kind":"page_break","node_id":0,"classes":[],"span":data["document"]["blocks"][0]["span"]});
        for _ in 0..blanks {
            data["document"]["blocks"]
                .as_array_mut()
                .unwrap()
                .insert(0, command.clone());
        }
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        rules.push(json!({"style_id":"balanced-appendix","selector":"paragraph.balanced-appendix","source_order":rules.len(),"extends":null,
            "declarations":[{"name":"page","important":false,"value":{"kind":"string","value":"balanced-appendix"}}]}));
        let template = data["page_masters"]["masters"][0].clone();
        for (parity, count) in [("left", 3), ("right", 2)] {
            let mut master = template.clone();
            master["master_id"] = format!("z-balanced-appendix-{parity}").into();
            master["body"]["width"] = ((count * 100 + (count - 1) * 10) * 65536 + 1).into();
            master["body"]["height"] = (768 * 65536).into();
            master["column_layout"]["count"] = count.into();
            data["page_masters"]["masters"]
                .as_array_mut()
                .unwrap()
                .push(master);
        }
        data["page_masters"]["selection_rules"] = json!([
            {"master_id":"z-balanced-appendix-left","parity":"even","first":null,"named_page":"balanced-appendix","source_order":0},
            {"master_id":"z-balanced-appendix-right","parity":"odd","first":null,"named_page":"balanced-appendix","source_order":1}
        ]);
        crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
            &mut data["document"],
            &mut 0,
        );
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), text, None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
        source_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            |closed, search, observed| {
                let result = shapes(closed, search, &data, &caps);
                assert_eq!(result.len(), blanks + 1);
                let final_page = result.last().unwrap();
                assert!(final_page.balance.is_some());
                assert_eq!(final_page.page as usize, blanks);
                assert_eq!(final_page.named.as_deref(), Some("balanced-appendix"));
                let count = if blanks == 1 { 3 } else { 2 };
                assert_eq!(final_page.heights.len(), count);
                assert_eq!(final_page.original_heights, vec![768 * 65536; count]);
                assert!(final_page
                    .heights
                    .iter()
                    .all(|height| *height == final_page.heights[0] && *height < 768 * 65536));
                let mut expected_widths = vec![100 * 65536; count];
                *expected_widths.last_mut().unwrap() += 1;
                assert_eq!(final_page.widths, expected_widths);
                assert!(result[..blanks]
                    .iter()
                    .all(|page| page.heights == page.original_heights
                        && page.owners.iter().all(Vec::is_empty)
                        && page.balance.is_none()));
                assert!(observed.width_feedback_passes() >= 2);
                let page = closed.geometry().pages().last().unwrap();
                let mut selected_widths = BTreeSet::new();
                for (fragment_index, placed) in page.fragments().iter().enumerate() {
                    if let Source::ParagraphLine {
                        paragraph_index,
                        line_index,
                    } = placed.fragment().source()
                    {
                        let column = page.fragment_column(fragment_index).unwrap();
                        let selected = &search.source_lines().paragraphs()
                            [paragraph_index as usize]
                            .selected()
                            .unwrap()
                            .lines()[line_index as usize];
                        assert_eq!(
                            selected.inline_size(),
                            page.columns()[usize::from(column)].bounds().width()
                        );
                        selected_widths.insert(selected.inline_size().get().raw());
                    }
                }
                assert_eq!(
                    selected_widths,
                    BTreeSet::from([100 * 65536, 100 * 65536 + 1])
                );
            },
        )
        .unwrap();
    }
}

#[test]
fn book_v2_column_balance_retains_table_nested_rowspan_caption_and_header_source_owners() {
    let caps = balanced_caps();
    let text = "Pro Pro Pro Pro Pro Pro Pro";
    for (mode, height) in [
        ("common", 256),
        ("forced", 256),
        ("span", 256),
        ("nested-body-span", 544),
        ("nested-body-caption", 512),
    ] {
        let mut data = super::super::super::super::data(false, mode, text);
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["body"]["height"] = (height * 65536).into();
            master["column_layout"]["balance"] = "last_page".into();
        }
        let selected = source_shapes(&data, text, &caps);
        assert!(selected.len() > 1, "{mode}: {selected:?}");
        let final_page = selected.last().unwrap();
        assert!(final_page.balance.is_some());
        if matches!(mode, "span" | "nested-body-caption") {
            let trace = final_page.balance.as_ref().unwrap();
            assert!(
                trace.table_certificates > 0,
                "{mode}: no measured table event"
            );
            assert!(
                trace.skipped_table_capacities > 0,
                "{mode}: no proved raw capacities skipped"
            );
        }
        assert!(final_page.owners.iter().any(|owners| !owners.is_empty()));
        assert_eq!(final_page.heights.len(), 2);
        assert!(final_page
            .heights
            .iter()
            .all(|height| *height == final_page.heights[0]));
        assert!(final_page
            .heights
            .iter()
            .zip(&final_page.original_heights)
            .all(|(selected, original)| selected <= original));
        assert!(selected[..selected.len() - 1]
            .iter()
            .all(|page| page.heights == page.original_heights && page.balance.is_none()));
        if mode == "forced" {
            let owner = data["document"]["blocks"][0]["body"][0]["cells"][0]["blocks"][2]["node_id"]
                .as_u64()
                .unwrap() as u32;
            assert_eq!(
                selected
                    .iter()
                    .filter_map(|page| page.forced)
                    .collect::<Vec<_>>(),
                [owner]
            );
        }
        let mut natural = data.clone();
        for master in natural["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"]["balance"] = "none".into();
        }
        let natural = source_shapes(&natural, text, &caps);
        assert!(natural.iter().all(|page| page.balance.is_none()));
        assert_eq!(
            final_page.balance.as_ref().unwrap().extent,
            natural.last().unwrap().used.iter().sum::<i64>()
        );
        assert_eq!(natural.len(), selected.len());
        assert_eq!(
            &natural[..natural.len() - 1],
            &selected[..selected.len() - 1]
        );
        // The full-height final page is a separate source selection. Balancing
        // changes its cuts while preserving the complete original sequence.
        let original_owners = natural
            .last()
            .unwrap()
            .owners
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        let balanced_owners = final_page
            .owners
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(
            balanced_owners, original_owners,
            "last-page source endpoint changed: {mode}"
        );
    }
}

fn check_ordinary_table_group(mode: &str) {
    let caps = balanced_caps();
    let mut data = super::super::super::super::data(false, "common", "Pro");
    let mut table = data["document"]["blocks"][0].clone();
    for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
        cell["blocks"].as_array_mut().unwrap().truncate(2);
    }
    let mut paragraph = table["body"][0]["cells"][0]["blocks"][0].clone();
    paragraph["classes"] = json!([]);
    let mut successor = if mode == "table-to-table" {
        table.clone()
    } else {
        paragraph.clone()
    };
    successor["classes"] = json!([]);
    if mode == "ordinary-to-table" {
        paragraph["classes"] = json!(["balance-hard-keep"]);
        data["document"]["blocks"] = json!([paragraph, table]);
    } else {
        table["classes"] = json!(["balance-hard-keep"]);
        data["document"]["blocks"] = json!([table, successor]);
    }
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for selector in ["paragraph.balance-hard-keep", "table.balance-hard-keep"] {
        rules.push(json!({"style_id":format!("{mode}-{}", rules.len()),"selector":selector,"source_order":rules.len(),"extends":null,
                "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    }
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"]["height"] = (256 * 65536).into();
        master["column_layout"]["balance"] = "last_page".into();
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    let root = Root::new();
    let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
    source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
        let result = shapes(closed, search, &data, &caps);
        assert_eq!(result.len(), 1, "{mode}: {result:?}");
        assert!(result[0].balance.is_some(), "{mode}");
        assert!(
            result[0].balance.as_ref().unwrap().table_certificates > 0,
            "{mode}: final-page table keep must carry original table capacity certificates"
        );
        let page = &closed.geometry().pages()[0];
        assert_eq!(
            page.selection()
                .next_state()
                .source_state()
                .next_table_index(),
            if mode == "table-to-table" { 2 } else { 1 }
        );
        let column_for = |owner: u32| {
            let columns = page
                .fragments_with_roles()
                .enumerate()
                .filter(|(_, (placed, _, repeated))| {
                    !repeated && placed.fragment().owner().get() == owner
                })
                .map(|(index, _)| page.fragment_column(index).unwrap())
                .collect::<BTreeSet<_>>();
            assert_eq!(columns.len(), 1, "keep owner split across columns: {mode}");
            *columns.first().unwrap()
        };
        let blocks = &data["document"]["blocks"];
        let first_owner = if mode == "ordinary-to-table" {
            blocks[0]["node_id"].as_u64().unwrap() as u32
        } else {
            blocks[0]["body"][0]["cells"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["blocks"]
                .as_array()
                .unwrap()
                .last()
                .unwrap()["node_id"]
                .as_u64()
                .unwrap() as u32
        };
        let second_owner = if mode == "table-to-ordinary" {
            blocks[1]["node_id"].as_u64().unwrap() as u32
        } else {
            blocks[1]["body"][0]["cells"][0]["blocks"][0]["node_id"]
                .as_u64()
                .unwrap() as u32
        };
        assert_eq!(
            column_for(first_owner),
            column_for(second_owner),
            "hard keep crossed a selected column: {mode}"
        );
        assert!(result[0].heights[0] <= result[0].original_heights[0]);
    })
    .unwrap();
}

#[test]
fn book_v2_column_balance_keeps_ordinary_to_table_hard_groups() {
    check_ordinary_table_group("ordinary-to-table");
}
#[test]
fn book_v2_column_balance_keeps_table_to_ordinary_hard_groups() {
    check_ordinary_table_group("table-to-ordinary");
}
#[test]
fn book_v2_column_balance_keeps_table_to_table_hard_groups() {
    check_ordinary_table_group("table-to-table");
}

fn zero_table_data(successor: Option<&str>) -> Value {
    let mut data = super::super::super::super::data(false, "common", "Pro");
    let mut positive_table = data["document"]["blocks"][0].clone();
    for cell in positive_table["body"][0]["cells"].as_array_mut().unwrap() {
        cell["blocks"].as_array_mut().unwrap().truncate(2);
    }
    let mut paragraph = positive_table["body"][0]["cells"][0]["blocks"][0].clone();
    paragraph["classes"] = json!([]);
    let mut zero = positive_table.clone();
    for section in ["head", "body", "foot"] {
        if let Some(rows) = zero.get_mut(section).and_then(Value::as_array_mut) {
            for row in rows {
                for cell in row["cells"].as_array_mut().unwrap() {
                    cell["blocks"] = json!([]);
                }
            }
        }
    }
    zero["classes"] = json!(["balanced-zero-keep"]);
    data["document"]["blocks"] = match successor {
        Some("ordinary") => json!([zero, paragraph]),
        Some("table") => json!([zero, positive_table]),
        None => json!([zero]),
        _ => unreachable!(),
    };
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"balanced-zero-keep","selector":"table.balanced-zero-keep","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
    for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
        master["body"]["height"] = (256 * 65536).into();
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    data
}

fn check_zero_table_group(successor: Option<&str>) {
    let caps = balanced_caps();
    let data = zero_table_data(successor);
    let evaluate = |balance: bool| {
        let mut authored = data.clone();
        for master in authored["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"]["balance"] = if balance { "last_page" } else { "none" }.into();
        }
        let root = Root::new();
        let input = balanced_input(&root, authored.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(
            &input,
            &caps,
            MODE,
            &mut budget,
            |closed, search, observed| {
                let selected = shapes(closed, search, &authored, &caps);
                assert_eq!(selected.len(), 1, "zero table baseline: {successor:?}");
                let original = search.source_table(0).unwrap();
                assert_eq!(
                    original.height().raw(),
                    0,
                    "authored empty cells must retain original zero measured height"
                );
                assert!(original
                    .cells()
                    .iter()
                    .all(|cell| cell.natural_height().raw() == 0 && cell.content().is_empty()));
                assert!(original.rows().iter().all(|row| row.height().raw() == 0));
                assert!(original.keep_with_next());
                assert_eq!(
                    closed.geometry().pages()[0]
                        .selection()
                        .next_state()
                        .source_state()
                        .next_table_index(),
                    if successor == Some("table") { 2 } else { 1 }
                );
                if successor.is_none() {
                    assert_eq!(closed.semantic_fragments(), 0);
                    assert_eq!(selected[0].used, [0, 0]);
                    assert!(selected[0].balance.is_none());
                    assert_eq!(selected[0].heights, selected[0].original_heights);
                    assert_eq!(observed.balance_candidates(), 0);
                } else {
                    assert!(closed.semantic_fragments() > 0);
                    assert_eq!(selected[0].balance.is_some(), balance);
                }
                selected
            },
        )
        .unwrap()
    };
    let natural = evaluate(false);
    let selected = evaluate(true);
    assert_eq!(
        natural[0].owners.iter().flatten().collect::<BTreeSet<_>>(),
        selected[0].owners.iter().flatten().collect::<BTreeSet<_>>()
    );
    if successor.is_some() {
        let extent = natural[0].used.iter().sum::<i64>();
        assert!(extent > 0);
        let trace = selected[0].balance.as_ref().unwrap();
        assert!(
            trace.zero_table_observations > 0,
            "zero hard table group must bind actual source observation at capacity zero"
        );
        assert_eq!(trace.extent, extent);
        assert_eq!(trace.first, (extent + 1) / 2);
        assert_eq!(selected[0].heights[0], *trace.targets.last().unwrap());
        if successor == Some("ordinary") {
            assert_eq!(selected[0].heights, [extent; 2]);
            assert_eq!(trace.targets, [(extent + 1) / 2, extent]);
        }
    } else {
        assert_eq!(natural, selected);
    }
}

#[test]
fn book_v2_column_balance_keeps_zero_table_to_positive_ordinary_source_groups() {
    check_zero_table_group(Some("ordinary"));
}
#[test]
fn book_v2_column_balance_keeps_zero_table_to_positive_table_source_groups() {
    check_zero_table_group(Some("table"));
}
#[test]
fn book_v2_column_balance_skips_all_zero_empty_table_without_capacity_candidates() {
    check_zero_table_group(None);
}

#[test]
fn book_v2_column_balance_retains_original_positive_spacing_when_zero_tables_use_zero_height() {
    let caps = balanced_caps();
    let mut data = zero_table_data(None);
    let mut first = data["document"]["blocks"][0].clone();
    let mut second = first.clone();
    first["classes"] = json!(["zero-after"]);
    second["classes"] = json!(["zero-before"]);
    data["document"]["blocks"] = json!([first, second]);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    for (selector, name, amount) in [
        ("table.zero-after", "space_after", 7),
        ("table.zero-before", "space_before", 9),
    ] {
        rules.push(json!({"style_id":format!("zero-spacing-{name}"),"selector":selector,"source_order":rules.len(),"extends":null,
            "declarations":[{"name":name,"important":false,"value":{"kind":"length","value":amount*65536}}]}));
    }
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    let evaluate = |balance: bool| {
        let mut authored = data.clone();
        for master in authored["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"]["balance"] = if balance { "last_page" } else { "none" }.into();
        }
        let root = Root::new();
        let input = balanced_input(&root, authored.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let selected = shapes(closed, search, &authored, &caps);
            assert_eq!(selected.len(), 1);
            assert_eq!(closed.semantic_fragments(), 0);
            for index in 0..2 {
                let table = search.source_table(index).unwrap();
                assert_eq!(table.height().raw(), 0);
                assert!(!table.keep_with_next());
            }
            let original_gap = search.source_table(0).unwrap().space_after().raw()
                + search.source_table(1).unwrap().space_before().raw();
            assert_eq!(original_gap, 16 * 65536);
            assert_eq!(
                closed.geometry().pages()[0]
                    .selection()
                    .next_state()
                    .source_state()
                    .next_table_index(),
                2
            );
            if balance {
                let trace = selected[0]
                    .balance
                    .as_ref()
                    .expect("positive original occupancy lost mandatory balance receipt");
                assert_eq!(trace.extent, original_gap);
                assert_eq!(trace.first, original_gap / 2);
                assert_eq!(trace.targets, [original_gap / 2]);
                assert_eq!(selected[0].used, [0, 0]);
                assert_eq!(selected[0].heights, [original_gap / 2; 2]);
            } else {
                assert!(selected[0].balance.is_none());
                assert_eq!(selected[0].used.iter().sum::<i64>(), original_gap);
                assert_eq!(selected[0].heights, selected[0].original_heights);
            }
            selected
        })
        .unwrap()
    };
    let original = evaluate(false);
    let balanced = evaluate(true);
    assert_eq!(
        balanced[0].balance.as_ref().unwrap().extent,
        original[0].used.iter().sum::<i64>()
    );
}

#[test]
fn book_v2_column_balance_rejects_unknown_table_carriers_before_pipeline_trials() {
    let caps = balanced_caps();
    for balance in ["none", "last_page"] {
        let mut data = zero_table_data(Some("ordinary"));
        data["document"]["blocks"][0]["foot"] = Value::Null;
        for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"]["balance"] = balance.into();
        }
        data["sources"][0]["sha256"] = typaxis_core::sha256(b"Pro")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into();
        data["sources"][0]["utf8_byte_length"] = 3.into();
        let root = Root::new();
        fs::write(
            root.0.join("document-package.json"),
            serde_json::to_vec(&data).unwrap(),
        )
        .unwrap();
        fs::write(root.0.join("input.tsf"), b"Pro").unwrap();
        let budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        let (session, raw) = HostBookV2InputSession::open(
            MachineInputHostOptions::new(
                HostPath::new(root.0.join("document-package.json")).unwrap(),
                None,
            ),
            caps.base(),
        )
        .unwrap();
        let error = session
            .decode_and_bind(&raw, &DocumentPackageDecodePolicy::new(caps.base()))
            .err()
            .expect("unknown table carrier was admitted");
        assert!(matches!(
            error.kind(),
            typaxis_machine_input::book_v2::BookV2InputErrorKind::Decode(_)
        ));
        assert!(error
            .to_string()
            .contains("unchanged contract-1.3 carrier is invalid"));
        assert!(format!("{:?}", error.progress()).contains("RawPackageAdmitted"));
        assert_eq!(budget.observation().balance_candidates(), 0);
        assert_eq!(budget.observation().work_steps(), 0);
        assert_eq!(budget.observation().record_charge(), 0);
        assert_eq!(budget.observation().spool_charge(), 0);
    }
}

#[test]
fn book_v2_column_balance_preserves_original_forced_zero_table_and_trailing_blank_debt() {
    let caps = balanced_caps();
    let mut data = zero_table_data(None);
    let table = &mut data["document"]["blocks"][0];
    table["head"] = json!([]);
    table["classes"] = json!(["zero-forced-space"]);
    for cell in table["body"][0]["cells"].as_array_mut().unwrap() {
        cell["blocks"] =
            json!([{"kind":"page_break","node_id":0,"classes":[],"span":cell["span"]}]);
    }
    let paragraph = balanced_body("Pro", 1, false)["document"]["blocks"][0].clone();
    data["document"]["blocks"]
        .as_array_mut()
        .unwrap()
        .insert(0, paragraph);
    let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
    rules.push(json!({"style_id":"zero-forced-space","selector":"table.zero-forced-space","source_order":rules.len(),"extends":null,
        "declarations":[{"name":"space_before","important":false,"value":{"kind":"length","value":16*65536}}]}));
    crate::book_v2_resources::tests::shaping_tests::table_caption_breaks::renumber(
        &mut data["document"],
        &mut 0,
    );
    let forced_owner = data["document"]["blocks"][1]["body"][0]["cells"][0]["blocks"][0]["node_id"]
        .as_u64()
        .unwrap() as u32;
    let evaluate = |balance: bool| {
        let mut authored = data.clone();
        for master in authored["page_masters"]["masters"].as_array_mut().unwrap() {
            master["column_layout"]["balance"] = if balance { "last_page" } else { "none" }.into();
        }
        let root = Root::new();
        let input = balanced_input(&root, authored.clone(), "Pro", None, &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let selected = shapes(closed, search, &authored, &caps);
            assert_eq!(selected.len(), 2);
            let table = search.source_table(0).unwrap();
            assert_eq!(table.height().raw(), 0);
            assert_eq!(table.space_before().raw(), 16 * 65536);
            assert!(search
                .source_items(None)
                .unwrap()
                .iter()
                .filter(|item| item.source().is_none())
                .all(|item| item.consumed_height().unwrap().raw() == 0));
            assert_eq!(selected[0].forced, Some(forced_owner));
            assert_eq!(selected[0].balance.is_some(), balance);
            assert!(selected[1].balance.is_none());
            assert!(selected[1].owners.iter().all(Vec::is_empty));
            assert_eq!(selected[1].used, [0, 0]);
            assert_eq!(selected[1].heights, selected[1].original_heights);
            assert_eq!(closed.semantic_fragments(), 1);
            selected
        })
        .unwrap()
    };
    let natural = evaluate(false);
    let balanced = evaluate(true);
    let trace = balanced[0].balance.as_ref().unwrap();
    assert_eq!(trace.extent, natural[0].used.iter().sum::<i64>());
    assert_eq!(trace.first, (trace.extent + 1) / 2);
    assert_eq!(balanced[0].forced, natural[0].forced);
    assert_eq!(balanced[1], natural[1]);
}

#[test]
fn book_v2_column_balance_binds_native_vector_header_math_and_equation_provenance() {
    use crate::book_v2_resources::tests::shaping_tests::native_tests::native_input;
    use crate::book_v2_resources::with_budgeted_book_v2_column_math as math_drive;
    let caps = balanced_caps();
    for vector in [false, true] {
        for nested in [false, true] {
            let root = Root::new();
            let mut data = math_terminals::header_data(vector, false, nested);
            for master in data["page_masters"]["masters"].as_array_mut().unwrap() {
                master["column_layout"]["balance"] = "last_page".into();
            }
            let input = if vector {
                vector_input(&root, data.clone(), &caps)
            } else {
                native_input(&root, data.clone(), &caps)
            };
            let evaluate = || {
                let mut budget = BookV2ColumnPageBudget::new(&caps, 1_000_000_000);
                let result = math_drive(
                    &input,
                    &caps,
                    MODE,
                    &mut budget,
                    |terminals, search, observed| {
                        assert_eq!(terminals.record_charge(), search.record_charge());
                        assert_eq!(observed.record_charge(), search.record_charge());
                        let result = shapes(terminals.source(), search, &data, &caps);
                        assert!(result.len() > 1);
                        let final_page = result.last().unwrap();
                        assert!(final_page.balance.is_some());
                        assert!(final_page
                            .heights
                            .iter()
                            .all(|height| *height == final_page.heights[0]));
                        assert!(terminals.source().has_header_variants());
                        assert!(terminals.source().repeated_math() > 0);
                        assert_eq!(terminals.source().semantic_math(), 2);
                        assert_eq!(
                            terminals
                                .terminals()
                                .iter()
                                .filter(|terminal| terminal.repeated_header())
                                .count(),
                            terminals.source().repeated_math()
                        );
                        let numbers = terminals
                            .source()
                            .geometry()
                            .pages()
                            .iter()
                            .map(|page| page.equation_numbers().len())
                            .sum::<usize>();
                        assert_eq!(numbers > 0, vector);
                        (math_terminals::inspect(terminals), result)
                    },
                )
                .unwrap();
                (result, budget.observation())
            };
            assert_eq!(
                evaluate(),
                evaluate(),
                "balanced terminal replay changed: {vector}/{nested}"
            );
        }
    }
}

#[test]
fn book_v2_column_balance_candidate_work_and_record_limits_are_cumulative_and_inclusive() {
    let data = balanced_body("Pro", 5, false);
    let base = balanced_caps();
    let evaluate = |caps: &M4EffectiveResourceLimits, maximum, retry: bool| {
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "Pro", None, caps);
        let mut budget = BookV2ColumnPageBudget::new(caps, maximum);
        let mut called = false;
        let mut delivered = None;
        let result = source_drive(
            &input,
            caps,
            MODE,
            &mut budget,
            |closed, search, observed| {
                called = true;
                delivered = Some(observed);
                assert!(observed.balance_candidates() >= search.balance_candidates());
                shapes(closed, search, &data, &caps)
            },
        )
        .map_err(|error| format!("{error:?}"));
        if let Some(delivered) = delivered {
            assert_eq!(delivered, budget.observation());
        }
        if retry {
            assert!(result.is_err() && !called);
            let prefix = budget.observation();
            assert!(
                source_drive(&input, caps, MODE, &mut budget, |_, _, _| panic!(
                    "failed balance delivered fallback pages"
                ))
                .is_err()
            );
            assert!(budget.observation().work_steps() >= prefix.work_steps());
            assert!(budget.observation().record_charge() >= prefix.record_charge());
            assert!(budget.observation().balance_candidates() >= prefix.balance_candidates());
            assert!(budget.observation().spool_charge() >= prefix.spool_charge());
            assert!(
                budget.observation().balance_candidates()
                    <= u64::from(caps.base().get().max_column_balance_candidates)
            );
            assert!(budget.observation().work_steps() <= maximum);
            assert!(budget.observation().record_charge() <= caps.base().get().max_fragments);
            assert!(budget.observation().spool_charge() <= caps.base().get().max_spool_bytes);
        }
        (result, budget.observation(), called)
    };
    let full = evaluate(&base, 100_000_000, false);
    assert!(full.0.is_ok() && full.2);
    assert!(
        full.1.balance_candidates() > 2,
        "source and physical repetitions must share the counter"
    );
    assert_eq!(evaluate(&base, full.1.work_steps(), false), full);
    let short_work = evaluate(&base, full.1.work_steps() - 1, true);
    assert!(short_work.0.is_err() && !short_work.2);
    let exact_records = changed_caps(&base, |c| c.max_fragments = full.1.record_charge());
    assert_eq!(evaluate(&exact_records, 100_000_000, false).0, full.0);
    let short_records = changed_caps(&base, |c| c.max_fragments = full.1.record_charge() - 1);
    let refused = evaluate(&short_records, 100_000_000, true);
    assert!(refused.0.is_err() && !refused.2);
    let exact_candidates = changed_caps(&base, |c| {
        c.max_column_balance_candidates = full.1.balance_candidates().try_into().unwrap()
    });
    let exact = evaluate(&exact_candidates, 100_000_000, false);
    assert_eq!(exact.0, full.0);
    assert_eq!(exact.1.balance_candidates(), full.1.balance_candidates());
    let short_candidates = changed_caps(&base, |c| {
        c.max_column_balance_candidates = (full.1.balance_candidates() - 1).try_into().unwrap()
    });
    let refused = evaluate(&short_candidates, 100_000_000, true);
    assert!(refused.0.is_err() && !refused.2);
    assert!(refused.0.as_ref().unwrap_err().contains("BalanceLimit"));
    assert_eq!(
        refused.1.balance_candidates(),
        u64::from(short_candidates.base().get().max_column_balance_candidates)
    );
    assert!(full.1.spool_charge() > 0);
    let exact_spool = balance_spool_limits(&base, full.1.spool_charge());
    let exact = evaluate(&exact_spool, 100_000_000, false);
    assert_eq!(exact.0, full.0);
    assert_eq!(exact.1.spool_charge(), full.1.spool_charge());
    let short_spool = balance_spool_limits(&base, full.1.spool_charge() - 1);
    let refused = evaluate(&short_spool, 100_000_000, true);
    assert!(refused.0.is_err() && !refused.2);
    assert!(refused.0.as_ref().unwrap_err().contains("Spool"));
}

#[test]
fn book_v2_column_balance_retains_callback_trials_in_the_driver_budget() {
    let caps = balanced_caps();
    let data = balanced_body("Pro", 5, false);
    let root = Root::new();
    let input = balanced_input(&root, data.clone(), "Pro", None, &caps);
    let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
    let (delivered, delta) = source_drive(
        &input,
        &caps,
        MODE,
        &mut budget,
        |closed, search, observed| {
            let original = shapes(closed, search, &data, &caps);
            let before = (
                search.balance_candidates(),
                search.work_steps(),
                search.record_charge(),
                search.terminal_spool_charge(),
            );
            let extra = search.select_balanced_pages().unwrap();
            let receipt = extra.pages().last().unwrap().balance().unwrap();
            assert_eq!(receipt.candidates().len(), 2);
            assert_eq!(
                receipt.selected_height().get().raw(),
                original[0].heights[0]
            );
            let delta = (
                search.balance_candidates() - before.0,
                search.work_steps() - before.1,
                search.record_charge() - before.2,
                search.terminal_spool_charge() - before.3,
            );
            assert_eq!(delta.0, 2);
            assert!(delta.1 > 0 && delta.2 > 0 && delta.3 > 0);
            (observed, delta)
        },
    )
    .unwrap();
    let retained = budget.observation();
    assert_eq!(
        retained.balance_candidates(),
        delivered.balance_candidates() + delta.0
    );
    assert_eq!(retained.work_steps(), delivered.work_steps() + delta.1);
    assert_eq!(
        retained.record_charge(),
        delivered.record_charge() + delta.2
    );
    assert_eq!(retained.spool_charge(), delivered.spool_charge() + delta.3);
}

#[test]
fn book_v2_column_balance_receipts_keep_the_exact_search_and_source_owners() {
    let caps = balanced_caps();
    let root = Root::new();
    let input = balanced_input(&root, balanced_body("Pro", 5, false), "Pro", None, &caps);
    let navigation = prepare_book_v2_navigation(input.body().styled()).unwrap();
    let flow = prepare_book_v2_text_flow(input.body().styled(), &navigation).unwrap();
    let plan = prepare_book_v2_column_frame_plan(&flow, &mut 0, 100_000_000, 0, 0).unwrap();
    let policy = prepare_book_v2_resource_policy(input.body(), &caps).unwrap();
    let bindings = bind_book_v2_vectors(&policy, input.resources(), &caps).unwrap();
    let mut allowance = BookV2BodyLineBudget::new(100_000_000, 512);
    with_budgeted_book_v2_column_lines(&policy, &flow, input.resources(), &bindings, &caps, MODE, None, &mut allowance, &plan, None, |lines| {
        let mut records = lines.retained_record_charge();
        let source = prepare_book_v2_column_flow_counted(&lines, None, &caps, records, &mut records).unwrap();
        let measured = prepare_book_v2_column_table_measurements_counted(source, &caps, &mut records).unwrap();
        let mut owner = prepare_book_v2_column_page_search_counted(&measured, &caps, 100_000_000, records, &mut 0, &mut 0).unwrap();
        let mut foreign = prepare_book_v2_column_page_search_counted(&measured, &caps, 100_000_000, records, &mut 0, &mut 0).unwrap();
        let stable = owner.select_stable_column_pages_counted(512, &mut 0).unwrap();
        assert!(owner.balance_candidates() >= 4);
        let placed = owner.place_column_pages(stable.sequence()).unwrap();
        let before = (foreign.work_steps(), foreign.record_charge(), foreign.balance_candidates(), foreign.terminal_spool_charge());
        assert!(matches!(foreign.verify_sequence(stable.sequence()), Err(e) if e.kind == E::ReceiptMismatch));
        assert!(matches!(foreign.place_column_pages(stable.sequence()), Err(e) if e.kind == E::ReceiptMismatch));
        assert!(matches!(foreign.close_column_page_sources(&stable, &placed), Err(e) if e.kind == E::ReceiptMismatch));
        let closed = owner.close_column_page_sources(&stable, &placed).unwrap();
        assert!(matches!(foreign.finalize_column_page_math(closed, &caps, 0), Err(e) if e.kind == E::ReceiptMismatch));
        assert_eq!(before, (foreign.work_steps(), foreign.record_charge(), foreign.balance_candidates(), foreign.terminal_spool_charge()));
    }).unwrap();
}

#[test]
#[ignore = "requires explicit original TYPAXIS_HARANO_FONT"]
fn book_v2_column_balance_preserves_original_harano_glyphs_lists_and_notes() {
    let font = fs::read(std::env::var("TYPAXIS_HARANO_FONT").unwrap()).unwrap();
    assert_eq!(
        typaxis_core::sha256(&font)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>(),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    let caps = balanced_caps();
    for notes in [false, true] {
        let mut data = balanced_body("本文", 5, notes);
        list_body(&mut data, true);
        let root = Root::new();
        let input = balanced_input(&root, data.clone(), "本文", Some(&font), &caps);
        let mut budget = BookV2ColumnPageBudget::new(&caps, 100_000_000);
        source_drive(&input, &caps, MODE, &mut budget, |closed, search, _| {
            let result = shapes(closed, search, &data, &caps);
            assert_eq!(result.len(), 1);
            assert!(result[0].heights[0] < result[0].original_heights[0]);
            assert_eq!(closed.geometry().pages()[0].list_markers().len(), 5);
            assert_eq!(
                !closed.geometry().pages()[0].footnote_markers().is_empty(),
                notes
            );
            for paragraph in search.source_lines().paragraphs() {
                for line in paragraph.lines() {
                    for inline in line.items() {
                        if let typaxis_layout::ProductionPlacedInline::Text(cluster) = inline {
                            for glyph in cluster.glyphs() {
                                assert!(glyph.glyph().original_gid.get() > 0);
                                assert!(std::ptr::eq(
                                    glyph.glyph(),
                                    &cluster.run().glyph_run().glyphs[glyph.glyph_index() as usize]
                                ));
                            }
                        }
                    }
                }
            }
        })
        .unwrap();
    }
}
