use super::*;
use crate::{FontFaceDeclaration, FontFaceId, ImageDeclaration, ImageResourceId};
#[derive(Default)]
struct Trace {
    requests: Vec<HostReadRequest>,
    refuse: bool,
}
impl HostReadBudget for Trace {
    type Error = ();
    const OBSERVE_DECLARATIONS: bool = true;
    fn charge(&mut self, request: HostReadRequest) -> Result<(), ()> {
        if self.refuse {
            return Err(());
        }
        self.requests.push(request);
        Ok(())
    }
}

#[test]
fn borrowed_declaration_comparison_preserves_every_identity_field() {
    let font = FontFaceDeclaration {
        font_face_id: FontFaceId::new(0),
        family: "和文".repeat(8192),
        uri: PortablePath::new("fonts/body.otf").unwrap(),
        face_index: 0,
        expected_sha256: Some([91; 32]),
    };
    let image = ImageDeclaration {
        image_id: ImageResourceId::new(0),
        uri: PortablePath::new("images/式.svg").unwrap(),
        expected_sha256: None,
    };
    let subject = ResourceErrorSubject::FontFace(font.font_face_id);
    for field in 0..6 {
        let mut candidate = font.clone();
        match field {
            0 => (),
            1 => candidate.font_face_id = FontFaceId::new(1),
            2 => candidate.family.push('別'),
            3 => candidate.uri = PortablePath::new("fonts/other.otf").unwrap(),
            4 => candidate.face_index = 1,
            _ => candidate.expected_sha256 = None,
        }
        let source = PendingResourceDeclaration::Font(&candidate);
        let expected = PendingResourceDeclaration::Font(&font);
        let mut trace = Trace::default();
        assert_eq!(
            matches(source, expected, subject.clone(), &mut trace).unwrap(),
            field == 0
        );
        assert_eq!(trace.requests.len(), 1);
        assert_eq!(
            trace.requests[0].work,
            64 + 2
                * (candidate.family.len()
                    + candidate.uri.as_str().len()
                    + font.family.len()
                    + font.uri.as_str().len()) as u64
        );
        assert_eq!(trace.requests[0].buffer_bytes, 0);
        let mut short = Trace {
            refuse: true,
            ..Trace::default()
        };
        assert!(
            matches!(matches(source, expected, subject.clone(), &mut short), Err(ResourceReadError::Budget(f)) if f.subject == subject && f.failure.request == trace.requests[0])
        );
        assert!(short.requests.is_empty());
    }
    let mut trace = Trace::default();
    let i = PendingResourceDeclaration::Image(&image);
    assert!(matches(
        i,
        i,
        ResourceErrorSubject::Image(image.image_id),
        &mut trace
    )
    .unwrap());
    assert!(!matches(
        i,
        PendingResourceDeclaration::Font(&font),
        subject,
        &mut trace
    )
    .unwrap());
}

#[test]
fn pending_uri_is_independently_owned_with_one_exact_reservation() {
    for count in [1, 63, 64, 65, 8192, 1_048_576] {
        let text = "a".repeat(count);
        let uri = PortablePath::new(text).unwrap();
        let subject = ResourceErrorSubject::Image(ImageResourceId::new(3));
        let mut trace = Trace::default();
        let copied = pending_uri(&uri, subject.clone(), &mut trace).unwrap();
        assert_eq!(copied, uri);
        assert_ne!(copied.as_str().as_ptr(), uri.as_str().as_ptr());
        assert_eq!(
            trace.requests,
            [HostReadRequest {
                stage: HostReadStage::DeclarationUri,
                file_offset: 0,
                buffer_bytes: count as u64,
                work: 16 * count as u64 + 64
            }]
        );
        let mut short = Trace {
            refuse: true,
            ..Trace::default()
        };
        assert!(
            matches!(pending_uri(&uri, subject.clone(), &mut short), Err(ResourceReadError::Budget(f)) if f.subject == subject && f.failure.request == trace.requests[0])
        );
        assert_eq!(uri.as_str().len(), count);
    }
    let uri = PortablePath::new("日本語/式.svg").unwrap();
    let copied = pending_uri(
        &uri,
        ResourceErrorSubject::Image(ImageResourceId::new(0)),
        &mut Trace::default(),
    )
    .unwrap();
    assert_eq!(copied, uri);
}
