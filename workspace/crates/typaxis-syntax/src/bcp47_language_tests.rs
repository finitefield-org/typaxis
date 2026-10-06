use super::*;

#[test]
fn fixed_storage_keeps_canonical_grammar_and_all_grandfathered_tags() {
    for (raw, expected) in [
        (
            "EN-latn-us-u-CA-gregory-a-foo",
            "en-Latn-US-a-foo-u-ca-gregory",
        ),
        ("en-z-BB-0-AA-a-CC-x-PrIvAtE", "en-0-aa-a-cc-z-bb-x-private"),
        ("zh-CMN-yue-Hans-cn-1901", "zh-cmn-yue-Hans-CN-1901"),
        ("es-419", "es-419"),
        ("X-A-B-C", "x-a-b-c"),
        ("abcdefgh-Latn-US-1234-abcde", "abcdefgh-Latn-US-1234-abcde"),
    ] {
        assert_eq!(canonicalize(raw, 255).unwrap().as_str(), expected);
    }
    for expected in GRANDFATHERED {
        assert_eq!(
            canonicalize(&expected.to_ascii_uppercase(), 255)
                .unwrap()
                .as_str(),
            *expected
        );
    }
    for raw in [
        "",
        "en--US",
        "-en",
        "en-",
        "en_US",
        "日本語",
        "e",
        "abcdefghi",
        "x",
        "x-abcdefghi",
        "en-a",
        "en-a-x-private",
        "en-a-foo-A-bar",
        "en-ABCDE-abcde",
        "en-aaa-bbb-ccc-ddd",
        "en-US-Latn",
        "en-abcde-abcde",
    ] {
        assert!(
            matches!(canonicalize(raw, 255), Err(Error::Invalid)),
            "{raw}"
        );
    }
}

#[test]
fn extension_groups_are_sorted_for_every_permutation_including_digit_singletons() {
    fn visit(parts: &mut [&str], offset: usize) {
        if offset == parts.len() {
            let raw = format!("EN-{}-x-PrIvAtE", parts.join("-"));
            assert_eq!(
                canonicalize(&raw, 255).unwrap().as_str(),
                "en-0-aa-9-bb-a-cc-b-dd-u-ee-z-ff-x-private"
            );
        } else {
            for i in offset..parts.len() {
                parts.swap(i, offset);
                visit(parts, offset + 1);
                parts.swap(i, offset);
            }
        }
    }
    visit(&mut ["0-AA", "9-BB", "A-CC", "B-DD", "U-EE", "Z-FF"], 0);
}

#[test]
fn fixed_and_configured_limits_keep_exact_boundaries_and_error_precedence() {
    let longest = format!("X{}", "-A".repeat(127));
    assert_eq!(longest.len(), 255);
    assert_eq!(
        canonicalize(&longest, 255).unwrap().as_str(),
        longest.to_ascii_lowercase()
    );
    assert!(matches!(canonicalize(&longest, 254), Err(Error::TextLimit)));
    assert!(matches!(
        canonicalize(&(longest + "-a"), 1),
        Err(Error::Invalid)
    ));
    assert!(matches!(canonicalize("en--US", 1), Err(Error::Invalid)));
    assert!(matches!(canonicalize("en-a", 3), Err(Error::TextLimit)));
    assert!(matches!(canonicalize("en-a", 4), Err(Error::Invalid)));
    assert_eq!(canonicalize("en-US", 5).unwrap().as_str(), "en-US");
    let mut groups = String::from("en");
    for singleton in "0123456789abcdefghijklmnopqrstuvwyz".chars().rev() {
        groups.push_str(&format!("-{singleton}-aa"));
    }
    let canonical = canonicalize(&groups, 255).unwrap();
    assert!(canonical.as_str().starts_with("en-0-aa-1-aa"));
    assert!(canonical.as_str().ends_with("-y-aa-z-aa"));
    assert_eq!(canonical.as_str().len(), groups.len());
}
