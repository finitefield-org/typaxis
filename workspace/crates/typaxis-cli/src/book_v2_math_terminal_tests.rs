use typaxis_core::sha256;
use typaxis_layout::ProductionPlacedInline;
use typaxis_pagination::book_v2::{
    BookV2BodyMathSource, BookV2BodyMathTerminals, BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM,
};
use typaxis_pagination::ProductionBodyFragmentSource;

/// Decode the fixed wire offsets independently and compare each record against
/// its actual selected source/physical fragment, including local inline origin.
pub(super) fn assert_math_terminals(value: &BookV2BodyMathTerminals<'_, '_, '_, '_, '_, '_, '_>) {
    let data = value.canonical_bytes();
    assert_eq!(
        &data[..32],
        &sha256(BOOK_V2_BODY_MATH_TERMINAL_ALGORITHM.as_bytes())
    );
    assert_eq!(value.fingerprint(), sha256(data));
    let geometry = value.source().geometry();
    let u64_at = |offset| u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
    assert_eq!(u64_at(224) as usize, geometry.pages().len());
    assert_eq!(u64_at(232) as usize, value.source().semantic_math());
    assert_eq!(u64_at(240) as usize, value.source().repeated_math());
    assert_eq!(
        u64_at(248) as usize,
        value.source().unreferenced_definitions()
    );
    let numbers = geometry
        .pages()
        .iter()
        .flat_map(|p| p.equation_numbers())
        .collect::<Vec<_>>();
    assert_eq!(u64_at(256) as usize, numbers.len());
    let fragments = geometry
        .pages()
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| {
            p.fragments_with_roles()
                .enumerate()
                .map(move |(fi, (placed, role, repeated))| {
                    (
                        placed,
                        role,
                        repeated,
                        value.source().fragment_flow(pi, fi).unwrap(),
                    )
                })
        })
        .collect::<Vec<_>>();
    let mut semantic = 0;
    for (index, terminal) in value.terminals().iter().enumerate() {
        let (placed, role, repeated, flow) = fragments[terminal.fragment_index()];
        let fragment = placed.fragment();
        assert_eq!(terminal.page_index(), fragment.page_index());
        assert_eq!(terminal.definition_index(), placed.definition_index());
        assert_eq!(terminal.item_index(), placed.item_index());
        assert_eq!(terminal.cell_owner(), role.map(|r| r.owner()));
        assert_eq!(terminal.repeated_header(), repeated);
        semantic += usize::from(!terminal.repeated_header());
        let start = 264 + index * 126;
        let record = &data[start..start + 126];
        assert_eq!(
            record[0],
            u8::from(matches!(terminal.source(), BookV2BodyMathSource::Native(_)))
        );
        assert_eq!(
            u32::from_be_bytes(record[1..5].try_into().unwrap()),
            terminal.source().owner().get()
        );
        assert_eq!(
            u32::from_be_bytes(record[5..9].try_into().unwrap()),
            terminal.page_index()
        );
        assert_eq!(
            u64::from_be_bytes(record[9..17].try_into().unwrap()) as usize,
            terminal.fragment_index()
        );
        assert_eq!(record[17], u8::from(terminal.definition_index().is_some()));
        assert_eq!(
            u64::from_be_bytes(record[18..26].try_into().unwrap()) as usize,
            terminal.definition_index().unwrap_or(0)
        );
        assert_eq!(
            u64::from_be_bytes(record[26..34].try_into().unwrap()) as usize,
            terminal.item_index()
        );
        assert_eq!(record[34], u8::from(terminal.inline_index().is_some()));
        assert_eq!(
            u32::from_be_bytes(record[35..39].try_into().unwrap()),
            terminal.inline_index().unwrap_or(0)
        );
        assert_eq!(record[39], u8::from(terminal.cell_owner().is_some()));
        assert_eq!(
            u32::from_be_bytes(record[40..44].try_into().unwrap()),
            terminal.cell_owner().map_or(0, |c| c.get())
        );
        assert_eq!(record[44], u8::from(terminal.repeated_header()));
        assert_eq!(
            i64::from_be_bytes(record[45..53].try_into().unwrap()),
            terminal.origin_x().raw()
        );
        assert_eq!(
            i64::from_be_bytes(record[53..61].try_into().unwrap()),
            terminal.baseline().raw()
        );
        assert_eq!(record[61], u8::from(terminal.viewport().is_some()));
        if let Some(rect) = terminal.viewport() {
            for (i, expected) in [rect.x(), rect.y(), rect.width().get(), rect.height().get()]
                .into_iter()
                .enumerate()
            {
                assert_eq!(
                    i64::from_be_bytes(record[62 + i * 8..70 + i * 8].try_into().unwrap()),
                    expected.raw()
                );
            }
        } else {
            assert_eq!(&record[62..94], &[0; 32]);
        }
        assert_eq!(&record[94..126], &terminal.source().fingerprint());
        if let Some(inline) = terminal.inline_index() {
            let ProductionBodyFragmentSource::ParagraphLine {
                paragraph_index,
                line_index,
            } = fragment.source()
            else {
                panic!("inline terminal without a paragraph");
            };
            let selected = &flow.lines().paragraphs()[paragraph_index as usize].lines()
                [line_index as usize]
                .items()[inline as usize];
            match (terminal.source(), selected) {
                (
                    BookV2BodyMathSource::Vector(binding),
                    ProductionPlacedInline::Vector(selected),
                ) => {
                    assert_eq!(binding.node_id(), selected.occurrence().item().node_id());
                    let local = selected.geometry();
                    assert_eq!(
                        terminal.origin_x(),
                        fragment
                            .bounds()
                            .x()
                            .checked_add(local.pen_origin_x())
                            .unwrap()
                    );
                    assert_eq!(
                        terminal.baseline(),
                        fragment
                            .bounds()
                            .y()
                            .checked_add(local.line_baseline_y())
                            .unwrap()
                    );
                    let viewport = terminal.viewport().unwrap();
                    assert_eq!(
                        viewport.x(),
                        fragment
                            .bounds()
                            .x()
                            .checked_add(local.viewport().x())
                            .unwrap()
                    );
                    assert_eq!(
                        viewport.y(),
                        fragment
                            .bounds()
                            .y()
                            .checked_add(local.viewport().y())
                            .unwrap()
                    );
                    assert_eq!(viewport.width(), local.viewport().width());
                    assert_eq!(viewport.height(), local.viewport().height());
                }
                (
                    BookV2BodyMathSource::Native(receipt),
                    ProductionPlacedInline::BookV2Math(selected),
                ) => {
                    assert!(std::ptr::eq(receipt, selected.receipt()));
                    assert_eq!(
                        terminal.origin_x(),
                        fragment.bounds().x().checked_add(selected.pen_x()).unwrap()
                    );
                    assert_eq!(
                        terminal.baseline(),
                        fragment
                            .bounds()
                            .y()
                            .checked_add(selected.baseline())
                            .unwrap()
                    );
                }
                _ => panic!("terminal source kind differs from selected line"),
            }
        } else {
            assert_eq!(terminal.source().owner(), fragment.owner());
            assert_eq!(Some(terminal.baseline()), fragment.baseline());
            match terminal.source() {
                BookV2BodyMathSource::Vector(binding) => {
                    let typaxis_layout::PrecomposedVectorPlacementInput::MathVectorBlock(input) =
                        binding.placement()
                    else {
                        panic!("not a formula block");
                    };
                    let viewport = fragment.viewport().unwrap();
                    assert_eq!(terminal.viewport(), Some(viewport));
                    assert_eq!(
                        terminal.origin_x(),
                        viewport
                            .x()
                            .checked_sub(input.metrics().origin_x())
                            .unwrap()
                    );
                }
                BookV2BodyMathSource::Native(receipt) => {
                    let ProductionBodyFragmentSource::NativeMathBlock { block_index } =
                        fragment.source()
                    else {
                        panic!("not native display");
                    };
                    let native = flow.lines().prepared().native_math().unwrap();
                    assert!(std::ptr::eq(
                        receipt,
                        native.receipt(fragment.owner()).unwrap()
                    ));
                    assert_eq!(
                        terminal.origin_x(),
                        fragment
                            .viewport()
                            .unwrap()
                            .x()
                            .checked_sub(native.display_blocks()[block_index as usize].left())
                            .unwrap()
                    );
                }
            }
        }
    }
    assert_eq!(semantic, value.source().semantic_math());
    assert_eq!(
        value.terminals().len() - semantic,
        value.source().repeated_math()
    );
    let offset = 264 + value.terminals().len() * 126;
    for (index, number) in numbers.iter().enumerate() {
        let record = &data[offset + index * 81..offset + (index + 1) * 81];
        let n = number.geometry();
        for (i, expected) in [
            n.owner().get(),
            n.parent_owner().get(),
            n.fragment_index(),
            n.page_index(),
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(
                u32::from_be_bytes(record[i * 4..i * 4 + 4].try_into().unwrap()),
                expected
            );
        }
        let rect = n.bounds();
        for (i, expected) in [rect.x(), rect.y(), rect.width().get(), rect.height().get()]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                i64::from_be_bytes(record[16 + i * 8..24 + i * 8].try_into().unwrap()),
                expected.raw()
            );
        }
        assert_eq!(&record[48..80], &n.shape_fingerprint());
        assert_eq!(record[80], u8::from(number.repeated_header()));
    }
    let mut offset = offset + numbers.len() * 81;
    if value.source().has_header_variants() {
        assert_eq!(
            &data[offset..offset + 32],
            &sha256(b"typaxis.book-2-math-terminal-header-owners/1")
        );
        assert_eq!(
            u64_at(offset + 32) as usize,
            value.source().header_variant_fragments()
        );
        offset += 40;
        for page in geometry.pages() {
            for variant in page.header_variants() {
                assert_eq!(
                    u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()),
                    page.selection().page_index()
                );
                assert_eq!(u64_at(offset + 4) as usize, variant.fragment_index());
                assert_eq!(u64_at(offset + 12) as usize, variant.global_item_index());
                let flow = variant.measurements().flow();
                let prepared = flow.lines().prepared();
                for (i, hash) in [
                    variant.header().fingerprint(),
                    flow.lines().fingerprint(),
                    flow.blocks().map_or([0; 32], |b| b.fingerprint()),
                    prepared
                        .vector_bindings()
                        .map_or([0; 32], |b| b.fingerprint()),
                    prepared.native_math().map_or([0; 32], |n| n.fingerprint()),
                ]
                .iter()
                .enumerate()
                {
                    assert_eq!(&data[offset + 20 + i * 32..offset + 52 + i * 32], hash);
                }
                offset += 180;
            }
        }
    }
    assert_eq!(data.len(), offset);
}
