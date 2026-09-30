use super::*;

fn parsed() -> ParsedMathReceipt {
    parse_math_source(
        "\\frac{α+1}{y}^{2}",
        MathParseLimits::new(1_000, 64).unwrap(),
    )
    .unwrap()
}
fn reseal_cache(value: &mut ParsedMathReceipt) {
    value.ast_fingerprint = sha256(value.ast_jcs.as_bytes());
    value.receipt_jcs = encode_parsed_receipt(
        value.source_sha256,
        value.ast_fingerprint,
        value.ast_node_count,
        value.ast_depth,
        &value.canonical_source,
    );
    value.receipt_fingerprint = sha256(value.receipt_jcs.as_bytes());
}

#[test]
fn canonical_revalidation_rejects_self_hashed_ast_source_and_receipt_cache_changes() {
    let original = parsed();
    original.verify().unwrap();
    let changes: [fn(&mut String); 3] = [
        |text| {
            text.pop();
        },
        |text| text.push(' '),
        |text| {
            *text = text.replacen('α', "β", 1);
        },
    ];
    for alter in changes {
        let mut ast = original.clone();
        alter(&mut ast.ast_jcs);
        reseal_cache(&mut ast);
        assert_eq!(
            ast.verify().unwrap_err().kind(),
            MathSourceErrorKind::ReceiptMismatch
        );
        assert_eq!(
            required_math_layout_units(&ast).unwrap_err(),
            MathComputationError::ParsedReceipt
        );
        let mut source = original.clone();
        alter(&mut source.canonical_source);
        reseal_cache(&mut source);
        assert_eq!(
            source.verify().unwrap_err().kind(),
            MathSourceErrorKind::ReceiptMismatch
        );
        let mut receipt = original.clone();
        alter(&mut receipt.receipt_jcs);
        receipt.receipt_fingerprint = sha256(receipt.receipt_jcs.as_bytes());
        assert_eq!(
            receipt.verify().unwrap_err().kind(),
            MathSourceErrorKind::ReceiptMismatch
        );
    }
    let mut source = original.clone();
    source.source_sha256[0] ^= 1;
    reseal_cache(&mut source);
    assert_eq!(
        source.verify().unwrap_err().kind(),
        MathSourceErrorKind::ReceiptMismatch
    );
    let mut accounting = original.clone();
    accounting.ast_depth += 1;
    reseal_cache(&mut accounting);
    assert_eq!(
        accounting.verify().unwrap_err().kind(),
        MathSourceErrorKind::ReceiptMismatch
    );
}

#[test]
fn canonical_comparison_requires_all_bytes_at_every_utf8_prefix_boundary() {
    let value = parsed();
    for cut in value.ast_jcs.char_indices().map(|(n, _)| n) {
        assert!(!canonical_matches(&value.ast_jcs[..cut], |out| write_ast(
            &value.ast, out
        )));
    }
    for cut in value.canonical_source.char_indices().map(|(n, _)| n) {
        assert!(!canonical_matches(&value.canonical_source[..cut], |out| {
            write_formatted_row(&value.ast.root, out)
        }));
    }
    for cut in value.receipt_jcs.char_indices().map(|(n, _)| n) {
        assert!(!canonical_matches(&value.receipt_jcs[..cut], |out| {
            write_parsed_receipt(
                value.source_sha256,
                value.ast_fingerprint,
                value.ast_node_count,
                value.ast_depth,
                &value.canonical_source,
                out,
            )
        }));
    }
    let mut extended = value.ast_jcs.clone();
    extended.push(' ');
    assert!(!canonical_matches(&extended, |out| write_ast(
        &value.ast, out
    )));
    let mut extended = value.canonical_source.clone();
    extended.push(' ');
    assert!(!canonical_matches(&extended, |out| write_formatted_row(
        &value.ast.root,
        out
    )));
    assert!(canonical_matches(&value.ast_jcs, |out| write_ast(
        &value.ast, out
    )));
    assert!(canonical_matches(&value.canonical_source, |out| {
        write_formatted_row(&value.ast.root, out)
    }));
}
