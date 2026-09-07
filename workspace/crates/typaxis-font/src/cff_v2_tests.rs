use super::*;

#[test]
fn cff_v2_fdselect_formats_have_exact_glyph_coverage() {
    let zero = [0, 0, 0, 1, 1];
    let three = [3, 0, 2, 0, 0, 0, 0, 2, 1, 0, 4];
    assert_eq!(fd_select(&zero, 0, 4, 2).unwrap().0, [0, 0, 1, 1]);
    assert_eq!(fd_select(&three, 0, 4, 2).unwrap().0, [0, 0, 1, 1]);
    for (position, value) in [(4, 1), (7, 0), (8, 2), (10, 3), (2, 0)] {
        let mut bad = three;
        bad[position] = value;
        assert_eq!(
            fd_select(&bad, 0, 4, 2).unwrap_err().kind,
            K::InvalidFdSelect
        );
    }
    assert_eq!(fd_select(&zero, 0, 4, 1).unwrap_err().table_offset, 3);
}

#[test]
fn cff_v2_charset_cids_are_not_sids_and_ranges_cannot_overlap() {
    for payload in [
        vec![0, 0, 10, 0, 11, 0, 12],
        vec![1, 0, 10, 2],
        vec![2, 0, 10, 0, 2],
    ] {
        let mut b = vec![0; 4];
        b.extend(payload);
        assert_eq!(charset(&b, 4, 4).unwrap().0, [0, 10, 11, 12]);
    }
    for payload in [
        vec![0, 0, 0],
        vec![0, 0, 10, 0, 10],
        vec![2, 255, 255, 0, 1],
        vec![1, 0, 10, 3],
    ] {
        let mut b = vec![0; 4];
        b.extend(payload);
        assert_eq!(charset(&b, 4, 3).unwrap_err().kind, K::InvalidCharset);
    }
    assert_eq!(
        charset(&[0, 0, 0], 0, 1).unwrap_err().kind,
        K::InvalidCharset
    );
}

#[test]
fn cff_v2_index_retains_ranges_and_checks_count_before_offsets() {
    let b = encode_cff_index(&[vec![14], vec![139, 14]]).unwrap();
    let parsed = index(&b, 0, 2, K::SubroutineLimit).unwrap();
    assert_eq!(parsed.objects[0].bytes(&b), [14]);
    assert_eq!(parsed.objects[1].bytes(&b), [139, 14]);
    assert_eq!(parsed.span.end, b.len());
    let e = index(&[0, 2], 0, 1, K::SubroutineLimit).unwrap_err();
    assert_eq!(
        (e.kind, e.limit, e.observed),
        (K::SubroutineLimit, Some(1), Some(2))
    );
    for i in 3..=5 {
        let mut bad = b.clone();
        bad[i] = 0;
        assert!(index(&bad, 0, 2, K::SubroutineLimit).is_err());
    }
}

#[test]
fn cff_v2_dict_preflights_operand_stack_and_duplicate_operators() {
    let mut b = vec![139; 49];
    b.push(5);
    let e = dict(
        &b,
        ProgramSpan {
            start: 0,
            end: b.len(),
        },
    )
    .unwrap_err();
    assert_eq!(
        (e.kind, e.limit, e.observed),
        (K::OperandLimit, Some(48), Some(49))
    );
    let b = [139, 20, 139, 20];
    assert!(dict(
        &b,
        ProgramSpan {
            start: 0,
            end: b.len()
        }
    )
    .is_err());
}

fn fixed_dict_int(v: i32, out: &mut Vec<u8>) {
    out.push(29);
    out.extend(v.to_be_bytes());
}
fn op(values: &[i32], operator: u16, out: &mut Vec<u8>) {
    for v in values {
        fixed_dict_int(*v, out);
    }
    if operator > 255 {
        out.push(12);
    }
    out.push(operator as u8);
}
struct CidFixture {
    bytes: Vec<u8>,
    fd_dicts: Vec<ProgramSpan>,
    private: usize,
    local: usize,
}
fn cid_fixture(shared: bool) -> CidFixture {
    let name = encode_cff_index(&[b"TypaxisCIDTest".to_vec()]).unwrap();
    let strings = encode_cff_index(&[b"Adobe".to_vec(), b"Identity".to_vec()]).unwrap();
    let globals = encode_cff_index(&[vec![11]]).unwrap();
    let chars = encode_cff_index(&[vec![14], vec![14], vec![14]]).unwrap();
    let local = encode_cff_index(&[vec![11], vec![11]]).unwrap();
    let mut private = Vec::new();
    op(&[600], 20, &mut private);
    op(&[0], 21, &mut private);
    op(&[18], 19, &mut private);
    assert_eq!(private.len(), 18);
    let mut positions = [0usize; 6];
    for _ in 0..3 {
        let mut top = Vec::new();
        op(&[391, 392, 0], 0x0c1e, &mut top);
        op(&[20], 0x0c22, &mut top);
        for (i, operator) in [15, 17, 0x0c25, 0x0c24].iter().enumerate() {
            op(&[positions[i] as i32], *operator, &mut top);
        }
        let top = encode_cff_index(&[top]).unwrap();
        let mut d0 = Vec::new();
        op(&[18, positions[4] as i32], 18, &mut d0);
        let mut d1 = Vec::new();
        op(&[18, positions[5] as i32], 18, &mut d1);
        let fds = encode_cff_index(&[d0, d1]).unwrap();
        let prefix = 4 + name.len() + top.len() + strings.len() + globals.len();
        let next = [
            prefix,
            prefix + 5,
            prefix + 5 + chars.len(),
            prefix + 5 + chars.len() + 4,
            prefix + 5 + chars.len() + 4 + fds.len(),
            prefix
                + 5
                + chars.len()
                + 4
                + fds.len()
                + if shared {
                    0
                } else {
                    private.len() + local.len()
                },
        ];
        if positions == next {
            let mut bytes = vec![1, 0, 4, 4];
            bytes.extend(&name);
            bytes.extend(&top);
            bytes.extend(&strings);
            bytes.extend(&globals);
            bytes.extend([0, 0, 10, 0, 11]);
            bytes.extend(&chars);
            bytes.extend([0, 0, 1, 1]);
            bytes.extend(&fds);
            bytes.extend(&private);
            bytes.extend(&local);
            if !shared {
                bytes.extend(&private);
                bytes.extend(&local);
            }
            let fd_dicts = index(&bytes, positions[3], 256, K::FontDictLimit)
                .unwrap()
                .objects;
            return CidFixture {
                bytes,
                fd_dicts,
                private: positions[4],
                local: positions[4] + private.len(),
            };
        }
        positions = next;
    }
    panic!("fixed-size DICT offsets must settle");
}

#[test]
fn cff_v2_complete_multifd_program_counts_shared_subrs_per_declaration() {
    for shared in [false, true] {
        let f = cid_fixture(shared);
        let source: Arc<[u8]> = f.bytes.into();
        let parsed = inspect_cff1_program_v2(source.clone(), 3, 1000, 5).unwrap();
        assert!(Arc::ptr_eq(&source, &parsed.program.source));
        assert_eq!(parsed.font_dict_count(), 2);
        assert_eq!(parsed.global_subroutine_count(), 1);
        assert_eq!(parsed.fd_for_gid(1), Some(1));
        assert_eq!(parsed.cid_for_gid(1), Some(10));
        assert_eq!(parsed.width_defaults(1), Some((600 * 65536, 0)));
        let e = inspect_cff1_program_v2(source, 3, 1000, 4).unwrap_err();
        assert_eq!(
            (e.kind, e.fd, e.limit, e.observed),
            (K::SubroutineLimit, Some(1), Some(4), Some(5))
        );
    }
}

#[test]
fn cff_v2_partial_private_overlap_and_fd_matrix_are_rejected() {
    let mut f = cid_fixture(false);
    let at = f.fd_dicts[1].start + 6;
    f.bytes[at..at + 4].copy_from_slice(&((f.private + 1) as i32).to_be_bytes());
    assert!(inspect_cff1_program_v2(f.bytes.into(), 3, 1000, 10).is_err());
    let f = cid_fixture(false);
    assert_eq!(
        inspect_cff1_program_v2(f.bytes.into(), 3, 2000, 10)
            .unwrap_err()
            .kind,
        K::UnsupportedFontMatrix
    );
    let b = [139, 12, 7];
    let d = dict(&b, ProgramSpan { start: 0, end: 3 }).unwrap();
    assert_eq!(
        matrix(&d, 1000, true, 0).unwrap_err().kind,
        K::UnsupportedFontMatrix
    );
}

#[test]
fn cff_v2_subroutine_offset_cannot_point_inside_its_private_dict() {
    let mut f = cid_fixture(false);
    assert_eq!(f.local, f.private + 18);
    f.bytes[f.private + 13..f.private + 17].copy_from_slice(&1i32.to_be_bytes());
    let e = inspect_cff1_program_v2(f.bytes.into(), 3, 1000, 10).unwrap_err();
    assert_eq!((e.kind, e.fd), (K::OverlappingStructures, Some(0)));
}

#[test]
#[ignore = "requires explicit TYPAXIS_HARANO_FONT pointing to the unchanged original font"]
fn cff_v2_original_harano_program_has_all_eighteen_fd_contexts() {
    let path = std::env::var("TYPAXIS_HARANO_FONT").expect("explicit font path");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        sha256(&bytes),
        [
            0x66, 0xef, 0x32, 0x70, 0xe6, 0x86, 0x90, 0x61, 0x2e, 0x8b, 0xf9, 0x82, 0xac, 0xfa,
            0xd0, 0xe8, 0xb4, 0x02, 0x12, 0xce, 0x64, 0x66, 0x1c, 0xce, 0x2b, 0xb6, 0xd3, 0xa9,
            0x8a, 0xc8, 0x47, 0x17
        ]
    );
    let font = FontRef::new(&bytes).unwrap();
    let cff = font
        .data_for_tag(read_fonts::types::Tag::new(b"CFF "))
        .unwrap();
    let maxp = font.maxp().unwrap();
    let head = font.head().unwrap();
    let p = inspect_cff1_program_v2(
        Arc::from(cff.as_bytes()),
        maxp.num_glyphs(),
        head.units_per_em(),
        100_000,
    )
    .unwrap();
    assert_eq!(p.glyph_count(), 23060);
    assert_eq!(p.font_dict_count(), 18);
    assert_eq!(p.local_subroutine_count(12), Some(21626));
    let fds = p.program.fd_by_gid.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        fds,
        [0, 1, 3, 4, 5, 7, 8, 10, 12, 13, 14, 17]
            .into_iter()
            .collect()
    );
    // Independently derived from original bytes with FontTools 4.51.0.
    let hex = |bytes: &[u8]| {
        sha256(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    };
    assert_eq!(
        hex(&p.program.fd_by_gid),
        "83ca997d6773a631791dad08bf2448ed15566a7f51b052d7776a8a364cc344e1"
    );
    let cid_bytes = p
        .program
        .cid_by_gid
        .iter()
        .flat_map(|cid| cid.to_be_bytes())
        .collect::<Vec<_>>();
    assert_eq!(
        hex(&cid_bytes),
        "22a2721ffa80fc5fe1da53fe8a78f0a4a0c59a4f73a32ee62f30177f25ce04aa"
    );
    assert_eq!(p.global_subroutine_count(), 1600);
    assert_eq!(p.width_defaults(14), Some((767 * 65536, 634 * 65536)));
    for gid in 0..p.glyph_count() {
        assert!(!p.charstring(gid).unwrap().is_empty());
    }
    eprintln!(
        "Harano original bytes={} glyphs={} FD={} global_subrs={} local_subrs={:?}",
        bytes.len(),
        p.glyph_count(),
        p.font_dict_count(),
        p.global_subroutine_count(),
        (0..18)
            .map(|fd| p.local_subroutine_count(fd).unwrap())
            .collect::<Vec<_>>()
    );
}

#[test]
fn cff_v2_fd_count_limit_rejects_257_before_index_allocation() {
    let e = index(&[1, 1], 0, 256, K::FontDictLimit).unwrap_err();
    assert_eq!(
        (e.kind, e.limit, e.observed),
        (K::FontDictLimit, Some(256), Some(257))
    );
}

#[test]
fn cff_v2_cannot_alias_charstrings_with_font_dict_index() {
    let mut f = cid_fixture(false);
    let names = index(&f.bytes, 4, 1, K::InvalidStructure).unwrap();
    let top_index = index(&f.bytes, names.span.end, 1, K::InvalidStructure).unwrap();
    let top = dict(&f.bytes, top_index.objects[0]).unwrap();
    let fd_offset = offset(&top, 0x0c24, 0).unwrap();
    let operator = top
        .iter()
        .find(|e| e.operator == 17)
        .unwrap()
        .operator_offset;
    f.bytes[operator - 4..operator].copy_from_slice(&(fd_offset as i32).to_be_bytes());
    // Two FD DICT objects are nonempty byte strings. They must not be accepted
    // simultaneously as the two glyph programs, even without executing glyphs.
    let e = inspect_cff1_program_v2(f.bytes.into(), 2, 1000, 10).unwrap_err();
    assert_eq!(e.kind, K::OverlappingStructures);
    assert_eq!(e.table_offset, fd_offset);
}
