#![forbid(unsafe_code)]

use typaxis_core::{
    AdmittedResourceFingerprint, DocumentFingerprint, FontFaceId, FontInstanceId, NodeId,
    ReferenceFingerprint, StyleFingerprint,
};
use typaxis_resource_admission::{
    AdmittedFontInstanceRef, AdmittedFontInstanceTable, AdmittedResourceLedgerToken,
};
use typaxis_style::{ResolvedTextStyle, StyleValidationError};
use typaxis_syntax::{PackageComputedStyle, PackageGeneratedTextBinding, ValidatedParsedPackage};

/// Exact identity of all validated inputs that can affect one layout state.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct LayoutEpoch {
    document: DocumentFingerprint,
    style: StyleFingerprint,
    admitted_resources: AdmittedResourceFingerprint,
    references: ReferenceFingerprint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutEpochError {
    AdmittedResourceDocumentMismatch,
    PackageEpochMismatch,
}

impl LayoutEpoch {
    pub fn from_validated_inputs(
        generated_text: PackageGeneratedTextBinding<'_>,
        admitted_resources: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, LayoutEpochError> {
        let package = generated_text.package();
        if !admitted_resources
            .ledger()
            .matches_declarations(&package.package().resources)
        {
            return Err(LayoutEpochError::AdmittedResourceDocumentMismatch);
        }
        Ok(Self {
            document: package.epoch_identity().document(),
            style: package.epoch_identity().style(),
            admitted_resources: admitted_resources.fingerprint(),
            references: generated_text.generated_text().reference_fingerprint(),
        })
    }

    pub const fn document(self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style(self) -> StyleFingerprint {
        self.style
    }
    pub const fn admitted_resources(self) -> AdmittedResourceFingerprint {
        self.admitted_resources
    }
    pub const fn references(self) -> ReferenceFingerprint {
        self.references
    }

    /// Reissues the epoch for the next pagination pass while preserving the
    /// stable document, style, and admitted-resource identities. The new
    /// reference identity is accepted only from the package-owned generated
    /// text validation boundary.
    pub fn with_generated_text(
        self,
        generated_text: PackageGeneratedTextBinding<'_>,
    ) -> Result<Self, LayoutEpochError> {
        let package = generated_text.package();
        if self.document != package.epoch_identity().document()
            || self.style != package.epoch_identity().style()
        {
            return Err(LayoutEpochError::PackageEpochMismatch);
        }
        Ok(Self {
            document: self.document,
            style: self.style,
            admitted_resources: self.admitted_resources,
            references: generated_text.generated_text().reference_fingerprint(),
        })
    }

    /// Returns whether two states share every pagination input other than the
    /// generated reference overlay.
    pub fn same_stable_inputs(self, other: Self) -> bool {
        self.document == other.document
            && self.style == other.style
            && self.admitted_resources == other.admitted_resources
    }
}

/// Text style after package/style identity and the admitted resource set have
/// all been checked at one trust boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedLayoutTextStyle {
    owner: NodeId,
    style_owner: NodeId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    admitted_resources: AdmittedResourceFingerprint,
    resolved: ResolvedTextStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutTextStyleError {
    PackageStyleMismatch,
    AdmittedResourceDocumentMismatch,
    InvalidStyle(StyleValidationError),
}

impl ResolvedLayoutTextStyle {
    pub fn new(
        package: &ValidatedParsedPackage,
        computed: &PackageComputedStyle,
        admitted: AdmittedResourceLedgerToken<'_>,
    ) -> Result<Self, LayoutTextStyleError> {
        if computed.document_fingerprint() != package.epoch_identity().document()
            || computed.style_fingerprint() != package.epoch_identity().style()
        {
            return Err(LayoutTextStyleError::PackageStyleMismatch);
        }
        if !admitted
            .ledger()
            .matches_declarations(&package.package().resources)
        {
            return Err(LayoutTextStyleError::AdmittedResourceDocumentMismatch);
        }
        let resolved = ResolvedTextStyle::try_from_computed(computed.computed(), admitted)
            .map_err(LayoutTextStyleError::InvalidStyle)?;
        Ok(Self {
            owner: computed.owner(),
            style_owner: computed.style_owner(),
            document: computed.document_fingerprint(),
            style: computed.style_fingerprint(),
            admitted_resources: admitted.fingerprint(),
            resolved,
        })
    }

    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn style_owner(&self) -> NodeId {
        self.style_owner
    }
    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }
    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }
    pub const fn admitted_resource_fingerprint(&self) -> AdmittedResourceFingerprint {
        self.admitted_resources
    }
    pub fn matches_epoch(&self, epoch: LayoutEpoch) -> bool {
        self.document == epoch.document()
            && self.style == epoch.style()
            && self.admitted_resources == epoch.admitted_resources()
    }
    pub const fn resolved(&self) -> &ResolvedTextStyle {
        &self.resolved
    }
}

/// Sealed selection proof consumed by shaping. It is impossible to construct
/// this receipt from a caller-selected instance ID, hash, or raw font bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShapeFontSelectionReceipt<'a> {
    epoch: LayoutEpoch,
    style: ResolvedLayoutTextStyle,
    font: AdmittedFontInstanceRef<'a>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeFontSelectionError {
    LayoutStyle(LayoutTextStyleError),
    EpochMismatch,
    FontInstanceLedgerMismatch,
    MissingFontInstance(FontFaceId),
    DuplicateFontFaceInstance(FontFaceId),
}

impl<'a> ShapeFontSelectionReceipt<'a> {
    pub fn new(
        package: &ValidatedParsedPackage,
        computed: &PackageComputedStyle,
        admitted: AdmittedResourceLedgerToken<'a>,
        instances: &'a AdmittedFontInstanceTable,
        epoch: LayoutEpoch,
    ) -> Result<Self, ShapeFontSelectionError> {
        let style = ResolvedLayoutTextStyle::new(package, computed, admitted)
            .map_err(ShapeFontSelectionError::LayoutStyle)?;
        if !style.matches_epoch(epoch) {
            return Err(ShapeFontSelectionError::EpochMismatch);
        }
        let selected_face = style.resolved().font_face_id();
        let font_instance_id = select_font_instance(
            admitted.fingerprint(),
            instances.ledger_fingerprint(),
            selected_face,
            instances
                .instances()
                .iter()
                .map(|instance| (instance.font_instance_id(), instance.font_face_id())),
        )?;
        let font = instances
            .resolve(font_instance_id, admitted.ledger())
            .ok_or(ShapeFontSelectionError::FontInstanceLedgerMismatch)?;
        if font.ledger_fingerprint() != admitted.fingerprint()
            || font.font_face_id() != selected_face
        {
            return Err(ShapeFontSelectionError::FontInstanceLedgerMismatch);
        }
        Ok(Self { epoch, style, font })
    }

    pub const fn epoch(&self) -> LayoutEpoch {
        self.epoch
    }
    pub const fn style(&self) -> &ResolvedLayoutTextStyle {
        &self.style
    }
    pub const fn owner(&self) -> NodeId {
        self.style.style_owner()
    }
    pub const fn admitted_font(&self) -> AdmittedFontInstanceRef<'a> {
        self.font
    }
    pub fn matches_epoch(&self, epoch: LayoutEpoch) -> bool {
        self.epoch == epoch
    }
}

fn select_font_instance(
    expected_ledger: AdmittedResourceFingerprint,
    table_ledger: AdmittedResourceFingerprint,
    selected_face: FontFaceId,
    instances: impl IntoIterator<Item = (FontInstanceId, FontFaceId)>,
) -> Result<FontInstanceId, ShapeFontSelectionError> {
    if table_ledger != expected_ledger {
        return Err(ShapeFontSelectionError::FontInstanceLedgerMismatch);
    }
    let mut matching = instances
        .into_iter()
        .filter(|(_, face)| *face == selected_face);
    let selected = matching
        .next()
        .ok_or(ShapeFontSelectionError::MissingFontInstance(selected_face))?;
    if matching.next().is_some() {
        return Err(ShapeFontSelectionError::DuplicateFontFaceInstance(
            selected_face,
        ));
    }
    Ok(selected.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::admitted_resource_fingerprint_from_jcs;

    #[test]
    fn font_selection_rejects_a_table_from_another_ledger() {
        let expected = admitted_resource_fingerprint_from_jcs("{\"ledger\":0}");
        let other = admitted_resource_fingerprint_from_jcs("{\"ledger\":1}");
        assert_eq!(
            select_font_instance(
                expected,
                other,
                FontFaceId::new(0),
                [(FontInstanceId::new(0), FontFaceId::new(0))],
            ),
            Err(ShapeFontSelectionError::FontInstanceLedgerMismatch)
        );
    }

    #[test]
    fn font_selection_rejects_wrong_or_duplicate_faces() {
        let ledger = admitted_resource_fingerprint_from_jcs("{\"ledger\":0}");
        assert_eq!(
            select_font_instance(
                ledger,
                ledger,
                FontFaceId::new(0),
                [(FontInstanceId::new(0), FontFaceId::new(1))],
            ),
            Err(ShapeFontSelectionError::MissingFontInstance(
                FontFaceId::new(0)
            ))
        );
        assert_eq!(
            select_font_instance(
                ledger,
                ledger,
                FontFaceId::new(0),
                [
                    (FontInstanceId::new(0), FontFaceId::new(0)),
                    (FontInstanceId::new(1), FontFaceId::new(0)),
                ],
            ),
            Err(ShapeFontSelectionError::DuplicateFontFaceInstance(
                FontFaceId::new(0)
            ))
        );
    }
}
