//! Private successor resource policy. This gate authorizes declared resource
//! admission only; it does not advertise a public profile or issue layout authority.
use typaxis_core::{sha256, M4EffectiveResourceLimits};
use typaxis_syntax::book_v2::SourceAdmittedBookV2Body;

pub const BOOK_V2_RESOURCE_POLICY_ALGORITHM: &str = "typaxis.book-2-resource-policy/1";
pub const BOOK_V2_RESOURCE_SET: &str = "typaxis.production-book-resource-set/3";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2ResourcePolicyError {
    OwnerOrLimits,
    MediaDeclaration,
}
impl std::fmt::Display for BookV2ResourcePolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "book-2 resource policy: {self:?}")
    }
}
impl std::error::Error for BookV2ResourcePolicyError {}

/// Borrowed source owner plus immutable resource policy. No caller-supplied
/// hash can construct this authorization, and equal input bytes do not allow
/// a different body owner to replace it.
#[derive(Debug)]
pub struct BookV2ResourcePolicy<'a> {
    body: &'a SourceAdmittedBookV2Body,
    limits: M4EffectiveResourceLimits,
    canonical: String,
    fingerprint: [u8; 32],
}
impl<'a> BookV2ResourcePolicy<'a> {
    pub const fn body(&self) -> &'a SourceAdmittedBookV2Body {
        self.body
    }
    pub const fn limits(&self) -> &M4EffectiveResourceLimits {
        &self.limits
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical
    }
    pub fn verify_for(
        &self,
        body: &SourceAdmittedBookV2Body,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookV2ResourcePolicyError> {
        if !std::ptr::eq(self.body, body) || self.limits != *limits {
            return Err(BookV2ResourcePolicyError::OwnerOrLimits);
        }
        Ok(())
    }
}

pub fn prepare_book_v2_resource_policy<'a>(
    body: &'a SourceAdmittedBookV2Body,
    limits: &M4EffectiveResourceLimits,
) -> Result<BookV2ResourcePolicy<'a>, BookV2ResourcePolicyError> {
    use BookV2ResourcePolicyError as E;
    if body.styled().body().limits() != limits.base() || body.provenance().limits() != limits.base()
    {
        return Err(E::OwnerOrLimits);
    }
    // Wire media is unchanged; the /3 resource set selects CFF /2 admission.
    super::semantic_container::validate_media_declarations(body.styled().body().resources(), true)
        .map_err(|_| E::MediaDeclaration)?;
    let input = body
        .provenance()
        .progress()
        .fingerprint()
        .ok_or(E::OwnerOrLimits)?;
    let hex = |h: [u8; 32]| h.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let canonical = format!(
        "{{\"algorithm\":\"{}\",\"input_sha256\":\"{}\",\"limits_sha256\":\"{}\",\"package_sha256\":\"{}\",\"resource_set\":\"{}\"}}",
        BOOK_V2_RESOURCE_POLICY_ALGORITHM, hex(input.bytes()), hex(limits.fingerprint()),
        hex(body.styled().body().canonical_jcs_sha256()), BOOK_V2_RESOURCE_SET,
    );
    Ok(BookV2ResourcePolicy {
        body,
        limits: limits.clone(),
        fingerprint: sha256(canonical.as_bytes()),
        canonical,
    })
}
