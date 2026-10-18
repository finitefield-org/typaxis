use super::*;

fn catalog() -> ResourceCatalog {
    ResourceCatalog {
        font_faces: vec![FontFaceDeclaration {
            font_face_id: FontFaceId::new(0),
            family: "本文-Body".into(),
            uri: PortablePath::new("font.bin").unwrap(),
            face_index: 0,
            expected_sha256: Some(sha256(b"authentic font bytes")),
        }],
        images: vec![ImageDeclaration {
            image_id: ImageResourceId::new(0),
            uri: PortablePath::new("image.bin").unwrap(),
            expected_sha256: Some(sha256(b"authentic image bytes")),
        }],
    }
}

#[test]
fn borrowed_catalog_keeps_the_original_rows_and_authentic_reads() {
    let tree = TempTree::new("borrowed-catalog");
    fs::write(tree.path().join("font.bin"), b"authentic font bytes").unwrap();
    fs::write(tree.path().join("image.bin"), b"authentic image bytes").unwrap();
    let declarations = catalog();
    let context = host_context(tree.path(), &[]);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let borrowed =
        HostResourceAdmissionSession::new_borrowed(&context, &config, &declarations).unwrap();
    let owned = HostResourceAdmissionSession::new(&context, &config, &declarations).unwrap();
    assert!(std::ptr::eq(borrowed.declarations, &declarations));
    assert!(!std::ptr::eq(&owned.declarations, &declarations));
    let font = borrowed.open_font(FontFaceId::new(0)).unwrap();
    let image = borrowed.open_image(ImageResourceId::new(0)).unwrap();
    let PendingResourceDeclaration::Font(font_row) = font.declaration else {
        panic!("wrong declaration kind")
    };
    let PendingResourceDeclaration::Image(image_row) = image.declaration else {
        panic!("wrong declaration kind")
    };
    assert!(std::ptr::eq(font_row, &declarations.font_faces[0]));
    assert!(std::ptr::eq(image_row, &declarations.images[0]));
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(&declarations, config.limits(), borrowed.roots())
            .unwrap();
    let pending_font = resolver.read_font(font).unwrap();
    let pending_image = resolver.read_image(image).unwrap();
    assert_eq!(pending_font.bytes(), b"authentic font bytes");
    assert_eq!(pending_image.bytes(), b"authentic image bytes");
    assert_eq!(pending_font.content_hash(), sha256(pending_font.bytes()));
    assert_eq!(pending_image.content_hash(), sha256(pending_image.bytes()));
    assert_eq!(
        borrowed
            .read_ledger_token()
            .unwrap()
            .candidate_attempt_count(),
        2
    );
    assert_eq!(
        borrowed
            .read_ledger_token()
            .unwrap()
            .stored_opened_identity_count(),
        2
    );
    assert!(matches!(
        borrowed.open_image(ImageResourceId::new(1)),
        Err(ResourceAdmissionError::MissingLogicalResource)
    ));
}

#[test]
fn borrowed_catalog_still_checks_declaration_and_session_identity() {
    let tree = TempTree::new("borrowed-catalog-identity");
    fs::write(tree.path().join("font.bin"), b"authentic font bytes").unwrap();
    let declarations = catalog();
    let mut changed = declarations.clone();
    changed.font_faces[0].family.push_str(" changed");
    let context = host_context(tree.path(), &[]);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let host =
        HostResourceAdmissionSession::new_borrowed(&context, &config, &declarations).unwrap();
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(&changed, config.limits(), host.roots()).unwrap();
    assert!(matches!(
        resolver.read_font(host.open_font(FontFaceId::new(0)).unwrap()),
        Err(ResourceAdmissionError::ReceiptIdentityMismatch)
    ));
    let foreign =
        HostResourceAdmissionSession::new_borrowed(&context, &config, &declarations).unwrap();
    let mut resolver =
        AdmittedResourceResolver::new_with_roots(&declarations, config.limits(), host.roots())
            .unwrap();
    assert!(matches!(
        resolver.read_font(foreign.open_font(FontFaceId::new(0)).unwrap()),
        Err(ResourceAdmissionError::RootSetMismatch)
    ));
}

#[test]
fn borrowed_catalog_registers_in_the_shared_ledger_and_preserves_constructor_errors() {
    let tree = TempTree::new("borrowed-shared-catalog");
    let declarations = catalog();
    let context = host_context(tree.path(), &[]);
    let config = effective_config(vec![ConfigResourceRoot::ProjectRoot]);
    let ledger = HostReadIdentityLedger::new();
    let old = ledger.token().unwrap();
    let host = HostResourceAdmissionSession::new_borrowed_with_read_ledger(
        &context,
        &config,
        &declarations,
        &ledger,
    )
    .unwrap();
    assert!(matches!(
        old.revalidate(),
        Err(HostAdmissionError::ReadIdentityMismatch)
    ));
    let current = ledger.token().unwrap();
    assert_eq!(current.candidate_attempt_count(), 2);
    assert_eq!(
        host.read_ledger_token().unwrap().candidate_attempt_count(),
        2
    );
    assert!(matches!(
        host.open_font(FontFaceId::new(0)),
        Err(ResourceAdmissionError::MissingResourceCandidate)
    ));
    current.revalidate().unwrap();
    let mut invalid = declarations.clone();
    invalid.font_faces[0].font_face_id = FontFaceId::new(4);
    let owned = HostResourceAdmissionSession::new(&context, &config, &invalid).unwrap_err();
    assert_eq!(
        HostResourceAdmissionSession::new_borrowed(&context, &config, &invalid).unwrap_err(),
        owned
    );
    let aliased = host_context(tree.path(), &[tree.path()]);
    assert_eq!(
        HostResourceAdmissionSession::new_borrowed(&aliased, &config, &declarations).unwrap_err(),
        HostResourceAdmissionSession::new(&aliased, &config, &declarations).unwrap_err()
    );
}
