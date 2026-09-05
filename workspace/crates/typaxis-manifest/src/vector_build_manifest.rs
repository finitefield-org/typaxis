use crate::{
    StagingBookNavigationManifestV2, StagingMathVectorManifest, StagingSafeVectorManifestV2,
    StagingTaggedPdfManifestV2,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingVectorBuildStatus {
    Built,
    Failed,
}

/// Exact vector-related members staged for the production root build manifest.
///
/// This is deliberately not a separately versioned aggregate record and has no
/// fingerprint of its own. The production build-manifest owner serializes
/// these eight members directly into its canonical root object.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingProductionBuildManifestVectorFields {
    status: StagingVectorBuildStatus,
    book_navigation_record: Option<String>,
    book_navigation_fingerprint: Option<[u8; 32]>,
    safe_vector_record: Option<String>,
    safe_vector_fingerprint: Option<[u8; 32]>,
    math_vector_record: Option<String>,
    math_vector_fingerprint: Option<[u8; 32]>,
    tagged_pdf_record: Option<String>,
    tagged_pdf_fingerprint: Option<[u8; 32]>,
}

impl StagingProductionBuildManifestVectorFields {
    pub fn built(
        book: &StagingBookNavigationManifestV2,
        safe: &StagingSafeVectorManifestV2,
        math: &StagingMathVectorManifest,
        tagged: &StagingTaggedPdfManifestV2,
    ) -> Result<Self, StagingProductionBuildManifestVectorFieldsError> {
        Self::assemble(
            StagingVectorBuildStatus::Built,
            Some(book),
            Some(safe),
            Some(math),
            Some(tagged),
        )
    }
    pub fn failed(
        book: Option<&StagingBookNavigationManifestV2>,
        safe: Option<&StagingSafeVectorManifestV2>,
        math: Option<&StagingMathVectorManifest>,
        tagged: Option<&StagingTaggedPdfManifestV2>,
    ) -> Result<Self, StagingProductionBuildManifestVectorFieldsError> {
        Self::assemble(StagingVectorBuildStatus::Failed, book, safe, math, tagged)
    }
    fn assemble(
        status: StagingVectorBuildStatus,
        book: Option<&StagingBookNavigationManifestV2>,
        safe: Option<&StagingSafeVectorManifestV2>,
        math: Option<&StagingMathVectorManifest>,
        tagged: Option<&StagingTaggedPdfManifestV2>,
    ) -> Result<Self, StagingProductionBuildManifestVectorFieldsError> {
        if status == StagingVectorBuildStatus::Built
            && [
                book.is_some(),
                safe.is_some(),
                math.is_some(),
                tagged.is_some(),
            ]
            .contains(&false)
        {
            return Err(StagingProductionBuildManifestVectorFieldsError::MissingBuiltRecord);
        }
        if math.is_some_and(|value| {
            safe.map(StagingSafeVectorManifestV2::fingerprint)
                != Some(value.safe_vector_manifest_fingerprint())
        }) {
            return Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch);
        }
        if tagged.is_some_and(|value| {
            safe.map(StagingSafeVectorManifestV2::fingerprint)
                != Some(value.safe_vector_manifest_fingerprint())
                || math.map(StagingMathVectorManifest::fingerprint)
                    != Some(value.math_vector_manifest_fingerprint())
        }) {
            return Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch);
        }
        if !present_hashes_match([
            book.map(StagingBookNavigationManifestV2::semantic_sha256),
            safe.map(StagingSafeVectorManifestV2::package_fingerprint),
            math.map(StagingMathVectorManifest::package_fingerprint),
        ]) {
            return Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch);
        }
        if !present_hashes_match([
            book.map(StagingBookNavigationManifestV2::package_sha256),
            tagged.map(StagingTaggedPdfManifestV2::package_sha256),
        ]) {
            return Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch);
        }
        if !present_hashes_match([
            book.map(StagingBookNavigationManifestV2::final_pdf_sha256),
            safe.map(StagingSafeVectorManifestV2::final_pdf_sha256),
            tagged.map(StagingTaggedPdfManifestV2::final_pdf_sha256),
        ]) {
            return Err(StagingProductionBuildManifestVectorFieldsError::PdfMismatch);
        }
        let book_navigation_record = book.map(|value| value.canonical_jcs().to_owned());
        let book_navigation_fingerprint = book.map(StagingBookNavigationManifestV2::fingerprint);
        let safe_vector_record = safe.map(|value| value.canonical_jcs().to_owned());
        let safe_vector_fingerprint = safe.map(StagingSafeVectorManifestV2::fingerprint);
        let math_vector_record = math.map(|value| value.canonical_jcs().to_owned());
        let math_vector_fingerprint = math.map(StagingMathVectorManifest::fingerprint);
        let tagged_pdf_record = tagged.map(|value| value.canonical_jcs().to_owned());
        let tagged_pdf_fingerprint = tagged.map(StagingTaggedPdfManifestV2::fingerprint);
        Ok(Self {
            status,
            book_navigation_record,
            book_navigation_fingerprint,
            safe_vector_record,
            safe_vector_fingerprint,
            math_vector_record,
            math_vector_fingerprint,
            tagged_pdf_record,
            tagged_pdf_fingerprint,
        })
    }
    pub const fn status(&self) -> StagingVectorBuildStatus {
        self.status
    }
    pub fn book_navigation_record(&self) -> Option<&str> {
        self.book_navigation_record.as_deref()
    }
    pub const fn book_navigation_fingerprint(&self) -> Option<[u8; 32]> {
        self.book_navigation_fingerprint
    }
    pub fn safe_vector_record(&self) -> Option<&str> {
        self.safe_vector_record.as_deref()
    }
    pub const fn safe_vector_fingerprint(&self) -> Option<[u8; 32]> {
        self.safe_vector_fingerprint
    }
    pub fn math_vector_record(&self) -> Option<&str> {
        self.math_vector_record.as_deref()
    }
    pub const fn math_vector_fingerprint(&self) -> Option<[u8; 32]> {
        self.math_vector_fingerprint
    }
    pub fn tagged_pdf_record(&self) -> Option<&str> {
        self.tagged_pdf_record.as_deref()
    }
    pub const fn tagged_pdf_fingerprint(&self) -> Option<[u8; 32]> {
        self.tagged_pdf_fingerprint
    }

    /// Canonical root-object projection consumed by the production manifest
    /// owner. These members remain an unversioned part of the owner build
    /// manifest rather than a separately publishable public record.
    pub fn canonical_root_projection(&self) -> String {
        encode_root_projection(self)
    }
}

fn present_hashes_match<const N: usize>(values: [Option<[u8; 32]>; N]) -> bool {
    let mut present = values.into_iter().flatten();
    let Some(first) = present.next() else {
        return true;
    };
    present.all(|value| value == first)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingProductionBuildManifestVectorFieldsError {
    MissingBuiltRecord,
    DependencyMismatch,
    PdfMismatch,
    ReceiptMismatch,
}
impl std::fmt::Display for StagingProductionBuildManifestVectorFieldsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I9190: vector build-manifest closure {:?}", self)
    }
}
impl std::error::Error for StagingProductionBuildManifestVectorFieldsError {}

fn encode_root_projection(value: &StagingProductionBuildManifestVectorFields) -> String {
    let mut out = String::from("{\"book_navigation_manifest\":");
    push_record(&mut out, value.book_navigation_record());
    out.push_str(",\"book_navigation_manifest_fingerprint\":");
    push_optional_hash(&mut out, value.book_navigation_fingerprint());
    out.push_str(",\"math_vector_manifest\":");
    push_record(&mut out, value.math_vector_record());
    out.push_str(",\"math_vector_manifest_fingerprint\":");
    push_optional_hash(&mut out, value.math_vector_fingerprint());
    out.push_str(",\"safe_vector_manifest\":");
    push_record(&mut out, value.safe_vector_record());
    out.push_str(",\"safe_vector_manifest_fingerprint\":");
    push_optional_hash(&mut out, value.safe_vector_fingerprint());
    out.push_str(",\"status\":\"");
    out.push_str(match value.status() {
        StagingVectorBuildStatus::Built => "built",
        StagingVectorBuildStatus::Failed => "failed",
    });
    out.push_str("\",\"tagged_pdf_manifest\":");
    push_record(&mut out, value.tagged_pdf_record());
    out.push_str(",\"tagged_pdf_manifest_fingerprint\":");
    push_optional_hash(&mut out, value.tagged_pdf_fingerprint());
    out.push('}');
    out
}
fn push_record(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => out.push_str(value),
        None => out.push_str("null"),
    }
}
fn push_optional_hash(out: &mut String, value: Option<[u8; 32]>) {
    match value {
        Some(value) => push_hash(out, value),
        None => out.push_str("null"),
    }
}
fn push_hash(out: &mut String, value: [u8; 32]) {
    const H: &[u8; 16] = b"0123456789abcdef";
    out.push('"');
    for byte in value {
        out.push(char::from(H[usize::from(byte >> 4)]));
        out.push(char::from(H[usize::from(byte & 15)]))
    }
    out.push('"')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_v2_fixture::{build_vector_v2_manifests, manifest_vector_v2_fixture};

    #[test]
    fn vector_build_manifest_closure_requires_nonnull_built_pairs_and_preserves_failed_nulls() {
        let fixture = manifest_vector_v2_fixture().unwrap();
        let products = build_vector_v2_manifests(&fixture).unwrap();
        let built = StagingProductionBuildManifestVectorFields::built(
            &products.book,
            &products.safe,
            &products.math,
            &products.tagged,
        )
        .unwrap();
        assert_eq!(built.status(), StagingVectorBuildStatus::Built);
        let built_root = built.canonical_root_projection();
        for member in [
            "book_navigation_manifest",
            "book_navigation_manifest_fingerprint",
            "math_vector_manifest",
            "math_vector_manifest_fingerprint",
            "safe_vector_manifest",
            "safe_vector_manifest_fingerprint",
            "tagged_pdf_manifest",
            "tagged_pdf_manifest_fingerprint",
        ] {
            assert!(!built_root.contains(&format!("\"{member}\":null")));
        }
        assert!(built_root.contains(
            "\"book_navigation_manifest\":{\"algorithm\":\"typaxis.book-navigation-manifest/2\""
        ));
        assert!(built_root.contains(
            "\"safe_vector_manifest\":{\"algorithm\":\"typaxis.safe-vector-manifest/2\""
        ));
        assert!(built_root.contains(
            "\"math_vector_manifest\":{\"algorithm\":\"typaxis.math-vector-manifest/1\""
        ));
        assert!(built_root.contains("\"tagged_pdf_manifest\":{\"accessibility_profile\":"));
        let failed =
            StagingProductionBuildManifestVectorFields::failed(None, None, None, None).unwrap();
        assert_eq!(failed.status(), StagingVectorBuildStatus::Failed);
        assert_eq!(
            failed.canonical_root_projection().matches(":null").count(),
            8
        );
    }

    #[test]
    fn vector_build_manifest_closure_rejects_completed_child_without_its_owner_dependency() {
        let fixture = manifest_vector_v2_fixture().unwrap();
        let products = build_vector_v2_manifests(&fixture).unwrap();
        assert_eq!(
            StagingProductionBuildManifestVectorFields::failed(
                None,
                None,
                Some(&products.math),
                None
            ),
            Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch)
        );
        assert_eq!(
            StagingProductionBuildManifestVectorFields::failed(
                None,
                Some(&products.safe),
                None,
                Some(&products.tagged)
            ),
            Err(StagingProductionBuildManifestVectorFieldsError::DependencyMismatch)
        );

        let failed_after_tagged = StagingProductionBuildManifestVectorFields::failed(
            None,
            Some(&products.safe),
            Some(&products.math),
            Some(&products.tagged),
        )
        .unwrap();
        assert_eq!(
            failed_after_tagged.book_navigation_record(),
            None,
            "book-navigation is an independent manifest owner"
        );
        assert!(failed_after_tagged.tagged_pdf_record().is_some());
    }
}
