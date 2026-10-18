#[derive(Clone, Debug)]
pub struct Case {
    pub label: String,
    pub bytes: Vec<u8>,
    pub glyphs: u16,
}
pub fn words(v: &[u16]) -> Vec<u8> {
    v.iter().flat_map(|n| n.to_be_bytes()).collect()
}
fn put(v: &mut [u8], at: usize, n: u16) {
    v[at..at + 2].copy_from_slice(&n.to_be_bytes());
}
fn append(v: &mut Vec<u8>, field: usize, tail: &[u8]) {
    let n = v.len() as u16;
    put(v, field, n);
    v.extend(tail);
}
fn coverage(g: u16) -> Vec<u8> {
    words(&[1, 1, g])
}
fn class(g: u16, c: u16) -> Vec<u8> {
    words(&[1, g, 1, c])
}
pub fn lookup(kind: u16, sub: &[u8]) -> Vec<u8> {
    let mut v = words(&[kind, 0, 1, 8]);
    v.extend(sub);
    v
}
fn single(format: u16, g: u16) -> Vec<u8> {
    let mut v = if format == 1 {
        words(&[1, 0, 0])
    } else {
        words(&[2, 0, 0, 0])
    };
    append(&mut v, 2, &coverage(g));
    v
}
fn pair1(count: u16, second: u16) -> Vec<u8> {
    let mut v = words(&[1, 0, 0, 0, count]);
    v.resize(10 + count as usize * 2, 0);
    append(&mut v, 2, &coverage(0));
    for i in 0..count {
        append(&mut v, 10 + i as usize * 2, &words(&[1, second]));
    }
    v
}
fn pair2(class_format: u16) -> Vec<u8> {
    let mut v = words(&[2, 0, 0, 0, 0, 0, 0, 0]);
    append(&mut v, 2, &coverage(0));
    let c = if class_format == 1 {
        class(0, 1)
    } else {
        words(&[2, 1, 0, 0, 1])
    };
    append(&mut v, 8, &c);
    append(&mut v, 10, &c);
    v
}
fn mark(g: u16) -> Vec<u8> {
    let mut v = words(&[1, 0, 0, 0, 0, 0]);
    append(&mut v, 2, &coverage(0));
    append(&mut v, 4, &coverage(g));
    v
}
fn context1(chain: bool, input: u16) -> Vec<u8> {
    let rule = if chain {
        words(&[1, 0, 2, input, 1, 0, 0])
    } else {
        words(&[2, 0, input])
    };
    let mut set = words(&[1, 4]);
    set.extend(rule);
    let mut v = words(&[1, 0, 1, 0]);
    append(&mut v, 2, &coverage(0));
    append(&mut v, 6, &set);
    v
}
fn context2(chain: bool, c: u16, class_format: u16) -> Vec<u8> {
    let rule = if chain {
        words(&[1, c, 2, c, 1, c, 0])
    } else {
        words(&[2, 0, c])
    };
    let mut set = words(&[1, 4]);
    set.extend(rule);
    let d = if class_format == 1 {
        class(0, c)
    } else {
        words(&[2, 1, 0, 0, c])
    };
    let mut v = if chain {
        words(&[2, 0, 0, 0, 0, 2, 0, 0])
    } else {
        words(&[2, 0, 0, 2, 0, 0])
    };
    append(&mut v, 2, &coverage(0));
    if chain {
        for field in [4, 6, 8] {
            append(&mut v, field, &d);
        }
        append(&mut v, 12, &set);
        append(&mut v, 14, &set);
    } else {
        append(&mut v, 4, &d);
        append(&mut v, 8, &set);
        append(&mut v, 10, &set);
    }
    v
}
fn context3(chain: bool, g: u16) -> Vec<u8> {
    let mut v = if chain {
        words(&[3, 1, 0, 1, 0, 1, 0, 0])
    } else {
        words(&[3, 1, 0, 0])
    };
    if chain {
        for field in [4, 8, 12] {
            append(&mut v, field, &coverage(g));
        }
    } else {
        append(&mut v, 6, &coverage(g));
    }
    v
}
pub fn lookup_seeds() -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    for g in [0, 1, 63] {
        for f in [1, 2] {
            out.push((format!("single-{f}-{g}"), lookup(1, &single(f, g))));
        }
    }
    for (count, second) in [(0, 0), (1, 0), (1, 1), (17, 0)] {
        out.push((
            format!("pair1-{count}-{second}"),
            lookup(2, &pair1(count, second)),
        ));
    }
    for f in [1, 2] {
        out.push((format!("pair2-{f}"), lookup(2, &pair2(f))));
    }
    let mut cursive = words(&[1, 0, 0]);
    append(&mut cursive, 2, &coverage(0));
    out.push(("cursive".into(), lookup(3, &cursive)));
    for kind in [4, 5, 6] {
        for g in [0, 1] {
            out.push((format!("mark-{kind}-{g}"), lookup(kind, &mark(g))));
        }
    }
    for chain in [false, true] {
        let kind = if chain { 8 } else { 7 };
        for g in [0, 1] {
            out.push((
                format!("context1-{chain}-{g}"),
                lookup(kind, &context1(chain, g)),
            ));
            out.push((
                format!("context3-{chain}-{g}"),
                lookup(kind, &context3(chain, g)),
            ));
        }
        for c in [0, 1] {
            for f in [1, 2] {
                out.push((
                    format!("context2-{chain}-{c}-{f}"),
                    lookup(kind, &context2(chain, c, f)),
                ));
            }
        }
    }
    let direct = out.clone();
    for (label, bytes) in direct {
        let kind = u16::from_be_bytes([bytes[0], bytes[1]]);
        let mut ext = words(&[1, kind, 0, 8]);
        ext.extend(&bytes[8..]);
        out.push((format!("extension-{label}"), lookup(9, &ext)));
    }
    for hit in [true, false] {
        let mut v = words(&[1, 0, 2, 10, 65535]);
        v.extend(single(1, u16::from(!hit)));
        out.push((format!("short-circuit-{hit}"), v));
    }
    let mut ranges = words(&[1, 0, 0]);
    append(&mut ranges, 2, &words(&[2, 3, 5, 4, 0, 0, 0, 0, 2, 1, 0]));
    out.push(("coverage-reversed-and-overlap".into(), lookup(1, &ranges)));
    let mut v = pair1(2, 0);
    let at = v.len() as u16;
    put(&mut v, 2, at);
    v.extend(words(&[2, 2, 3, 2, 0, 0, 0, 0]));
    out.push(("pair-empty-range-iterator".into(), lookup(2, &v)));
    out.push(("unknown-lookup".into(), words(&[10, 0, 0])));
    out.push(("empty-direct".into(), words(&[1, 0, 0])));
    out.push(("empty-extension".into(), words(&[9, 0, 0])));
    out
}
pub fn jstf(lookup: &[u8]) -> Vec<u8> {
    let mut v = words(&[1, 0, 1]);
    v.extend(b"latn");
    v.extend(words(&[12, 0, 6, 0, 1, 4]));
    let priority = v.len();
    v.resize(priority + 20, 0);
    put(&mut v, priority + 8, 20);
    v.extend(words(&[1, 4]));
    v.extend(lookup);
    v
}
fn expand(seeds: Vec<(String, Vec<u8>)>) -> Vec<Case> {
    let mut out = Vec::new();
    for (label, bytes) in seeds {
        for glyphs in [0, 1, 3, 64] {
            out.push(Case {
                label: format!("{label}-glyphs-{glyphs}"),
                bytes: bytes.clone(),
                glyphs,
            });
        }
        for len in 0..bytes.len() {
            out.push(Case {
                label: format!("{label}-prefix-{len}"),
                bytes: bytes[..len].to_vec(),
                glyphs: 1,
            });
        }
        for at in 0..bytes.len() {
            for byte in [0, 1, 255] {
                if bytes[at] != byte {
                    let mut v = bytes.clone();
                    v[at] = byte;
                    out.push(Case {
                        label: format!("{label}-byte-{at}-{byte}"),
                        bytes: v,
                        glyphs: 1,
                    });
                }
            }
        }
    }
    out
}
pub fn lookup_cases() -> Vec<Case> {
    expand(lookup_seeds())
}
pub fn table_cases() -> Vec<Case> {
    let mut seeds: Vec<_> = lookup_seeds()
        .into_iter()
        .map(|(n, v)| (n, jstf(&v)))
        .collect();
    seeds.push(("empty-table".into(), words(&[1, 0, 0])));
    let mut ext = words(&[1, 0, 1]);
    ext.extend(b"latn");
    ext.extend(words(&[12, 6, 0, 0, 3, 0, 1, 2]));
    seeds.push(("extenders".into(), ext));
    for field in 0..10 {
        let mut v = jstf(&lookup(1, &single(1, 0)));
        put(&mut v, 22 + 8, 0);
        put(&mut v, 22 + field * 2, 20);
        seeds.push((format!("priority-field-{field}"), v));
    }
    expand(seeds)
}
