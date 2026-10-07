use super::*;
use typaxis_linebreak::{
    break_production_inline, ProductionInlineLogicalUnit as U, ProductionLineBreakBudget,
    ProductionNativeMathInlineItem,
};

#[test]
fn production_native_math_kernel_keeps_real_computation_atomic() {
    let fixture = crate::math::staging_math_layout_fixture().unwrap();
    let receipt = fixture
        .layout
        .receipts()
        .iter()
        .find(|r| r.kind() == typaxis_math::MathNodeKind::Inline)
        .unwrap();
    let source = fixture.package.math_node(receipt.node_id()).unwrap();
    let paragraph = NodeId::new(0);
    let math = ProductionNativeMathInlineItem::from_computation(
        receipt.node_id(),
        paragraph,
        source.domain().span,
        receipt.key().bytes(),
        receipt.computation(),
    )
    .unwrap();
    assert_eq!(
        math.computation_sha256(),
        receipt.computation().fingerprint()
    );
    assert_eq!(math.receipt_sha256(), receipt.key().bytes());
    assert_eq!(
        math.advance().get().raw(),
        receipt.computation().dimensions().advance()
    );
    let width = math
        .right()
        .max(math.advance().get())
        .checked_sub(math.left().min(Length::ZERO))
        .unwrap();
    let size = PositiveLength::new(width).unwrap();
    let height = PositiveLength::new(Length::from_raw(1).unwrap()).unwrap();
    let p = ProductionInlineParagraph::itemize_with_breaks(
        paragraph,
        vec![U::Math(math)],
        vec![],
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let mut budget = ProductionLineBreakBudget::new(1, 1);
    let selected = break_production_inline(&p, size, height, &mut budget).unwrap();
    assert_eq!(selected.lines().len(), 1);
    let line = &selected.lines()[0];
    assert_eq!((line.line().start_unit(), line.line().end_unit()), (0, 1));
    assert_eq!(line.required_inline_size().get(), width);
    assert_eq!(line.line().metrics().content_ascent(), math.ascent());
    assert_eq!(line.line().metrics().content_descent(), math.descent());
    assert!(
        line.line().occurrences().is_empty(),
        "native math must not issue an SVG occurrence"
    );
    assert_eq!(budget.remaining_steps(), 0);
    assert_eq!(budget.remaining_lines(), 0);
    let narrow =
        PositiveLength::new(width.checked_sub(Length::from_raw(1).unwrap()).unwrap()).unwrap();
    assert!(matches!(
        break_production_inline(
            &p,
            narrow,
            height,
            &mut ProductionLineBreakBudget::new(10, 10)
        ),
        Err(AtomicVectorInlineError::NoFeasibleLine)
    ));
    assert!(matches!(
        break_production_inline(&p, size, height, &mut ProductionLineBreakBudget::new(0, 10)),
        Err(AtomicVectorInlineError::CandidateLimit)
    ));
    let text = AtomicVectorTextUnit::new(
        'A',
        NonNegativeLength::new(width).unwrap(),
        NonNegativeLength::ZERO,
        NonNegativeLength::ZERO,
    );
    let control = |id| {
        U::Break(
            ProductionExplicitBreak::new(NodeId::new(id), source.domain().span, BreakKind::Allowed)
                .unwrap(),
        )
    };
    let p = ProductionInlineParagraph::itemize_with_breaks(
        paragraph,
        vec![
            U::Text(text),
            control(1000),
            U::Math(math),
            control(1001),
            U::Text(text),
        ],
        vec![
            ProductionTextClusterRange {
                start_unit: 0,
                end_unit: 1,
            },
            ProductionTextClusterRange {
                start_unit: 4,
                end_unit: 5,
            },
        ],
        JapaneseLineBreakMode::Normal,
    )
    .unwrap();
    let selected = break_production_inline(
        &p,
        size,
        height,
        &mut ProductionLineBreakBudget::new(100, 10),
    )
    .unwrap();
    assert_eq!(selected.lines().len(), 3);
    assert!(selected.lines()[1].unit_pen_x(2).is_some());
    assert!(selected.lines()[0].unit_pen_x(2).is_none());
    assert!(selected.lines()[2].unit_pen_x(2).is_none());
    assert!(ProductionInlineParagraph::itemize_with_breaks(
        paragraph,
        vec![U::Math(math), U::Math(math)],
        vec![],
        JapaneseLineBreakMode::Normal
    )
    .is_err());
    assert!(ProductionInlineParagraph::itemize_with_breaks(
        NodeId::new(9999),
        vec![U::Math(math)],
        vec![],
        JapaneseLineBreakMode::Normal
    )
    .is_err());
    let other = ProductionNativeMathInlineItem::from_computation(
        receipt.node_id(),
        paragraph,
        source.domain().span,
        sha256(b"another-receipt"),
        receipt.computation(),
    )
    .unwrap();
    assert_ne!(math.fingerprint(), other.fingerprint());
    let display = fixture
        .layout
        .receipts()
        .iter()
        .find(|r| r.kind() == typaxis_math::MathNodeKind::Display)
        .unwrap();
    assert!(ProductionNativeMathInlineItem::from_computation(
        display.node_id(),
        paragraph,
        source.domain().span,
        display.key().bytes(),
        display.computation()
    )
    .is_err());
}
