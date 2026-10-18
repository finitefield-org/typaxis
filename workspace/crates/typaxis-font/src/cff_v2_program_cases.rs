fn encode_cff_index(objects: &[Vec<u8>]) -> Result<Vec<u8>, ()> {
    let mut out = (objects.len() as u16).to_be_bytes().to_vec();
    if objects.is_empty() {
        return Ok(out);
    }
    let end = objects.iter().map(Vec::len).sum::<usize>() + 1;
    let size = if end <= 255 {
        1
    } else if end <= 65535 {
        2
    } else if end <= 16777215 {
        3
    } else {
        4
    };
    out.push(size as u8);
    let mut at = 1u32;
    for object in objects.iter().chain(std::iter::once(&Vec::new())) {
        out.extend(&at.to_be_bytes()[4 - size..]);
        at += object.len() as u32;
    }
    for object in objects {
        out.extend(object);
    }
    Ok(out)
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
pub fn cid_fixture(shared: bool) -> Vec<u8> {
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
            return bytes;
        }
        positions = next;
    }
    panic!("fixed-size DICT offsets must settle");
}

pub fn cases() -> Vec<(String, Vec<u8>, u16, u32)> {
    let mut out = Vec::new();
    for shared in [false, true] {
        let b = cid_fixture(shared);
        out.push((format!("shared-{shared}"), b.clone(), 3, 5));
        for i in 0..b.len() {
            for v in [0, 1, 28, 29, 30, 31, 139, 255] {
                if b[i] == v {
                    continue;
                }
                let mut data = b.clone();
                data[i] = v;
                out.push((format!("shared-{shared}-byte-{i}-{v}"), data, 3, 5));
            }
        }
        for end in 0..b.len() {
            out.push((
                format!("shared-{shared}-prefix-{end}"),
                b[..end].to_vec(),
                3,
                5,
            ));
        }
        for g in [0, 1, 2, 3, 4, 65535] {
            for limit in [0, 1, 2, 3, 4, 5, 6, 65535] {
                out.push((
                    format!("shared-{shared}-glyphs-{g}-limit-{limit}"),
                    b.clone(),
                    g,
                    limit,
                ));
            }
        }
    }
    out
}
