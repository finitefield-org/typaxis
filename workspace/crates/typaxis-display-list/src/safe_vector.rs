use typaxis_core::{
    push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits, NodeId, Rect,
};
use typaxis_document::StagingM4FigurePlacement;
use typaxis_layout::StagingSafeVectorSelectedLayout;
use typaxis_syntax::{
    StagingM4PageGeometry, StagingPrecomposedVectorProfileAuthorization,
    StagingSafeVectorProfileView, ValidatedStagingSemanticPackage,
};

pub const STAGING_DRAW_VECTOR_ALGORITHM: &str = "typaxis.draw-vector-display/1";

/// Type-level proof that structure properties are owned by page-level
/// DrawVector usages. Reusable Form streams are downstream of this boundary
/// and receive no MCID, Alt, ActualText, or Lang occurrence from Display.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorFormStructureIsolationReceiptV2 {
    vector_display_sha256: [u8; 32],
    form_count: u32,
    page_do_usage_count: u32,
    form_mcid_count: u32,
    form_structure_property_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl VectorFormStructureIsolationReceiptV2 {
    pub const fn vector_display_sha256(&self) -> [u8; 32] {
        self.vector_display_sha256
    }
    pub const fn form_count(&self) -> u32 {
        self.form_count
    }
    pub const fn page_do_usage_count(&self) -> u32 {
        self.page_do_usage_count
    }
    pub const fn form_mcid_count(&self) -> u32 {
        self.form_mcid_count
    }
    pub const fn form_structure_property_count(&self) -> u32 {
        self.form_structure_property_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        display: &crate::StagingPrecomposedVectorDisplay,
    ) -> Result<(), StagingSafeVectorDisplayError> {
        display
            .verify_resource_closure()
            .map_err(|_| StagingSafeVectorDisplayError::ReceiptMismatch)?;
        let canonical = encode_form_structure_isolation_v2(
            display.receipt().fingerprint(),
            display.receipt().content_key_count(),
            display.receipt().command_count(),
        );
        if self.vector_display_sha256 != display.receipt().fingerprint()
            || self.form_count != display.receipt().content_key_count()
            || self.page_do_usage_count != display.receipt().command_count()
            || self.form_mcid_count != 0
            || self.form_structure_property_count != 0
            || self.canonical_jcs != canonical
            || self.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
}

pub fn prove_vector_form_structure_isolation_v2(
    display: &crate::StagingPrecomposedVectorDisplay,
) -> Result<VectorFormStructureIsolationReceiptV2, StagingSafeVectorDisplayError> {
    display
        .verify_resource_closure()
        .map_err(|_| StagingSafeVectorDisplayError::ReceiptMismatch)?;
    let canonical_jcs = encode_form_structure_isolation_v2(
        display.receipt().fingerprint(),
        display.receipt().content_key_count(),
        display.receipt().command_count(),
    );
    let receipt = VectorFormStructureIsolationReceiptV2 {
        vector_display_sha256: display.receipt().fingerprint(),
        form_count: display.receipt().content_key_count(),
        page_do_usage_count: display.receipt().command_count(),
        form_mcid_count: 0,
        form_structure_property_count: 0,
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
    };
    receipt.verify(display)?;
    Ok(receipt)
}

fn encode_form_structure_isolation_v2(
    vector_display_sha256: [u8; 32],
    form_count: u32,
    page_do_usage_count: u32,
) -> String {
    let mut output = String::from("{\"form_count\":");
    output.push_str(&form_count.to_string());
    output.push_str(
        ",\"form_mcid_count\":0,\"form_structure_property_count\":0,\"marked_content_plan_algorithm\":",
    );
    push_jcs_string(
        &mut output,
        crate::tagged_structure::MARKED_CONTENT_PLAN_ALGORITHM_V2,
    );
    output.push_str(",\"page_do_usage_count\":");
    output.push_str(&page_do_usage_count.to_string());
    output.push_str(",\"vector_display_sha256\":");
    push_hash(&mut output, vector_display_sha256);
    output.push('}');
    output
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingDrawVector {
    occurrence: u32,
    owner: NodeId,
    image_id: ImageResourceId,
    ir_fingerprint: [u8; 32],
    admitted_sha256: [u8; 32],
    page_index: u32,
    frame_index: u32,
    bounds: Rect,
    scale: i32,
    placement: StagingM4FigurePlacement,
    selected_placement_fingerprint: [u8; 32],
    fingerprint: [u8; 32],
}

impl StagingDrawVector {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn frame_index(&self) -> u32 {
        self.frame_index
    }
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }
    pub const fn scale_raw(&self) -> i32 {
        self.scale
    }
    pub const fn placement(&self) -> StagingM4FigurePlacement {
        self.placement
    }
    pub const fn selected_placement_fingerprint(&self) -> [u8; 32] {
        self.selected_placement_fingerprint
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorDisplayPage {
    page_index: u32,
    commands: Vec<StagingDrawVector>,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn commands(&self) -> &[StagingDrawVector] {
        &self.commands
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorDisplayReceipt {
    package_fingerprint: [u8; 32],
    profile_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    selected_layout_fingerprint: [u8; 32],
    page_geometry_fingerprint: [u8; 32],
    command_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorDisplayReceipt {
    pub const fn algorithm(&self) -> &'static str {
        STAGING_DRAW_VECTOR_ALGORITHM
    }

    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn selected_layout_fingerprint(&self) -> [u8; 32] {
        self.selected_layout_fingerprint
    }
    pub const fn page_geometry_fingerprint(&self) -> [u8; 32] {
        self.page_geometry_fingerprint
    }
    pub const fn command_count(&self) -> u32 {
        self.command_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorDisplay {
    pages: Vec<StagingSafeVectorDisplayPage>,
    page_geometry: StagingM4PageGeometry,
    receipt: StagingSafeVectorDisplayReceipt,
}

impl StagingSafeVectorDisplay {
    pub fn pages(&self) -> &[StagingSafeVectorDisplayPage] {
        &self.pages
    }
    pub fn commands(&self) -> impl Iterator<Item = &StagingDrawVector> {
        self.pages.iter().flat_map(|page| &page.commands)
    }
    pub const fn receipt(&self) -> &StagingSafeVectorDisplayReceipt {
        &self.receipt
    }
    pub const fn page_geometry(&self) -> &StagingM4PageGeometry {
        &self.page_geometry
    }

    pub fn verify_resource_closure(&self) -> Result<(), StagingSafeVectorDisplayError> {
        let commands: Vec<_> = self.commands().collect();
        let canonical = encode_display(
            self.receipt.selected_layout_fingerprint,
            &self.pages,
            &self.page_geometry,
        );
        if self.pages.is_empty()
            || usize::try_from(self.receipt.command_count) != Ok(commands.len())
            || self.receipt.page_geometry_fingerprint != self.page_geometry.fingerprint()
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
        }
        for (index, command) in commands.into_iter().enumerate() {
            if usize::try_from(command.occurrence) != Ok(index)
                || command.fingerprint != sha256(encode_command(command).as_bytes())
            {
                return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
            }
        }
        for (index, page) in self.pages.iter().enumerate() {
            if usize::try_from(page.page_index) != Ok(index)
                || page
                    .commands
                    .iter()
                    .any(|command| command.page_index != page.page_index)
                || page.fingerprint != sha256(encode_page(page).as_bytes())
            {
                return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
            }
        }
        Ok(())
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSafeVectorProfileView,
        limits: &M4EffectiveResourceLimits,
        selected: &StagingSafeVectorSelectedLayout,
    ) -> Result<(), StagingSafeVectorDisplayError> {
        selected
            .verify_downstream(package, profile, limits)
            .map_err(|_| StagingSafeVectorDisplayError::SelectedMismatch)?;
        self.verify_resource_closure()?;
        let commands: Vec<_> = self.commands().collect();
        let canonical = encode_display(
            selected.receipt().fingerprint(),
            &self.pages,
            &self.page_geometry,
        );
        let expected_page_count = selected
            .placements()
            .last()
            .map_or(Some(1u32), |placement| {
                placement.page_index().checked_add(1)
            })
            .ok_or(StagingSafeVectorDisplayError::PageLimit)?;
        if self.receipt.package_fingerprint != package.semantic_fingerprint()
            || self.receipt.profile_fingerprint != profile.profile_fingerprint()
            || self.receipt.limits_fingerprint != limits.fingerprint()
            || self.receipt.selected_layout_fingerprint != selected.receipt().fingerprint()
            || self.page_geometry != *selected.page_geometry()
            || self.receipt.page_geometry_fingerprint != self.page_geometry.fingerprint()
            || usize::try_from(self.receipt.command_count) != Ok(commands.len())
            || commands.len() != selected.placements().len()
            || usize::try_from(expected_page_count) != Ok(self.pages.len())
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
        {
            return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
        }
        for (index, (command, placement)) in
            commands.into_iter().zip(selected.placements()).enumerate()
        {
            if usize::try_from(command.occurrence) != Ok(index)
                || command.owner != placement.owner()
                || command.image_id != placement.image_id()
                || command.ir_fingerprint != placement.ir_fingerprint()
                || command.admitted_sha256 != placement.admitted_sha256()
                || command.page_index != placement.page_index()
                || command.frame_index != placement.frame_index()
                || command.bounds != placement.bounds()
                || command.scale != placement.scale_raw()
                || command.placement != placement.placement()
                || command.selected_placement_fingerprint != placement.fingerprint()
                || command.fingerprint != sha256(encode_command(command).as_bytes())
            {
                return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
            }
        }
        for (index, page) in self.pages.iter().enumerate() {
            if usize::try_from(page.page_index) != Ok(index)
                || page
                    .commands
                    .iter()
                    .any(|command| command.page_index != page.page_index)
                || page.fingerprint != sha256(encode_page(page).as_bytes())
            {
                return Err(StagingSafeVectorDisplayError::ReceiptMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorDisplayError {
    SelectedMismatch,
    PageLimit,
    CommandLimit,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSafeVectorDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedMismatch => {
                formatter.write_str("I9190: SafeVector selected layout mismatch")
            }
            Self::PageLimit => formatter.write_str("L5100: SafeVector Display page limit exceeded"),
            Self::CommandLimit => formatter.write_str("L5110: DrawVector command limit exceeded"),
            Self::ReceiptMismatch => formatter.write_str("I9190: DrawVector receipt mismatch"),
            Self::AllocationFailure => formatter.write_str("L5100: DrawVector allocation failed"),
        }
    }
}

impl std::error::Error for StagingSafeVectorDisplayError {}

pub fn build_staging_safe_vector_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSafeVectorProfileView,
    limits: &M4EffectiveResourceLimits,
    selected: &StagingSafeVectorSelectedLayout,
) -> Result<StagingSafeVectorDisplay, StagingSafeVectorDisplayError> {
    selected
        .verify_downstream(package, profile, limits)
        .map_err(|_| StagingSafeVectorDisplayError::SelectedMismatch)?;
    let page_count = selected
        .placements()
        .last()
        .map_or(Some(1u32), |placement| {
            placement.page_index().checked_add(1)
        })
        .ok_or(StagingSafeVectorDisplayError::PageLimit)?;
    if page_count > limits.base().get().max_pages {
        return Err(StagingSafeVectorDisplayError::PageLimit);
    }
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(page_count as usize)
        .map_err(|_| StagingSafeVectorDisplayError::AllocationFailure)?;
    for page_index in 0..page_count {
        pages.push(StagingSafeVectorDisplayPage {
            page_index,
            commands: Vec::new(),
            fingerprint: [0; 32],
        });
    }
    for placement in selected.placements() {
        let mut command = StagingDrawVector {
            occurrence: placement.occurrence(),
            owner: placement.owner(),
            image_id: placement.image_id(),
            ir_fingerprint: placement.ir_fingerprint(),
            admitted_sha256: placement.admitted_sha256(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            bounds: placement.bounds(),
            scale: placement.scale_raw(),
            placement: placement.placement(),
            selected_placement_fingerprint: placement.fingerprint(),
            fingerprint: [0; 32],
        };
        command.fingerprint = sha256(encode_command(&command).as_bytes());
        pages
            .get_mut(command.page_index as usize)
            .ok_or(StagingSafeVectorDisplayError::ReceiptMismatch)?
            .commands
            .push(command);
    }
    for page in &mut pages {
        page.fingerprint = sha256(encode_page(page).as_bytes());
    }
    let page_geometry = selected.page_geometry().clone();
    let canonical_jcs = encode_display(selected.receipt().fingerprint(), &pages, &page_geometry);
    let display = StagingSafeVectorDisplay {
        receipt: StagingSafeVectorDisplayReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            selected_layout_fingerprint: selected.receipt().fingerprint(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            command_count: u32::try_from(selected.placements().len())
                .map_err(|_| StagingSafeVectorDisplayError::CommandLimit)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        pages,
        page_geometry,
    };
    display.verify(package, profile, limits, selected)?;
    Ok(display)
}

/// Build the Figure/SafeVector-1 compatibility Display inside the complete
/// production profile. Empty trailing pages are retained so the Figure and
/// producer-composed Display receipts share the same selected page tuple.
pub fn build_production_safe_vector_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingPrecomposedVectorProfileAuthorization,
    limits: &M4EffectiveResourceLimits,
    admitted: &typaxis_resource_admission::AdmittedResourceLedger,
    selected: &StagingSafeVectorSelectedLayout,
    page_count: u32,
) -> Result<StagingSafeVectorDisplay, StagingSafeVectorDisplayError> {
    selected
        .verify_production(package, profile, limits, admitted)
        .map_err(|_| StagingSafeVectorDisplayError::SelectedMismatch)?;
    if page_count == 0
        || page_count > limits.base().get().max_pages
        || selected
            .placements()
            .iter()
            .any(|placement| placement.page_index() >= page_count)
    {
        return Err(StagingSafeVectorDisplayError::PageLimit);
    }
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(page_count as usize)
        .map_err(|_| StagingSafeVectorDisplayError::AllocationFailure)?;
    for page_index in 0..page_count {
        pages.push(StagingSafeVectorDisplayPage {
            page_index,
            commands: Vec::new(),
            fingerprint: [0; 32],
        });
    }
    for placement in selected.placements() {
        let mut command = StagingDrawVector {
            occurrence: placement.occurrence(),
            owner: placement.owner(),
            image_id: placement.image_id(),
            ir_fingerprint: placement.ir_fingerprint(),
            admitted_sha256: placement.admitted_sha256(),
            page_index: placement.page_index(),
            frame_index: placement.frame_index(),
            bounds: placement.bounds(),
            scale: placement.scale_raw(),
            placement: placement.placement(),
            selected_placement_fingerprint: placement.fingerprint(),
            fingerprint: [0; 32],
        };
        command.fingerprint = sha256(encode_command(&command).as_bytes());
        pages
            .get_mut(command.page_index as usize)
            .ok_or(StagingSafeVectorDisplayError::ReceiptMismatch)?
            .commands
            .push(command);
    }
    for page in &mut pages {
        page.fingerprint = sha256(encode_page(page).as_bytes());
    }
    let page_geometry = selected.page_geometry().clone();
    let canonical_jcs = encode_display(selected.receipt().fingerprint(), &pages, &page_geometry);
    let display = StagingSafeVectorDisplay {
        receipt: StagingSafeVectorDisplayReceipt {
            package_fingerprint: package.semantic_fingerprint(),
            profile_fingerprint: profile.profile_receipt_fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            selected_layout_fingerprint: selected.receipt().fingerprint(),
            page_geometry_fingerprint: page_geometry.fingerprint(),
            command_count: u32::try_from(selected.placements().len())
                .map_err(|_| StagingSafeVectorDisplayError::CommandLimit)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        pages,
        page_geometry,
    };
    display.verify_resource_closure()?;
    Ok(display)
}

fn encode_display(
    selected_layout_fingerprint: [u8; 32],
    pages: &[StagingSafeVectorDisplayPage],
    page_geometry: &StagingM4PageGeometry,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_DRAW_VECTOR_ALGORITHM);
    output.push_str(",\"page_geometry\":");
    output.push_str(page_geometry.canonical_jcs());
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_page(page));
    }
    output.push_str("],\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected_layout_fingerprint);
    output.push('}');
    output
}

fn encode_page(page: &StagingSafeVectorDisplayPage) -> String {
    let mut output = String::from("{\"commands\":[");
    for (index, command) in page.commands.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_command(command));
    }
    output.push_str("],\"page_index\":");
    output.push_str(&page.page_index.to_string());
    output.push('}');
    output
}

fn encode_command(value: &StagingDrawVector) -> String {
    let mut output = String::from("{\"admitted_sha256\":");
    push_hash(&mut output, value.admitted_sha256);
    output.push_str(",\"bounds\":{");
    output.push_str("\"height\":");
    output.push_str(&value.bounds.height().get().raw().to_string());
    output.push_str(",\"width\":");
    output.push_str(&value.bounds.width().get().raw().to_string());
    output.push_str(",\"x\":");
    output.push_str(&value.bounds.x().raw().to_string());
    output.push_str(",\"y\":");
    output.push_str(&value.bounds.y().raw().to_string());
    output.push_str("},\"frame_index\":");
    output.push_str(&value.frame_index.to_string());
    output.push_str(",\"image_id\":");
    output.push_str(&value.image_id.get().to_string());
    output.push_str(",\"ir_fingerprint\":");
    push_hash(&mut output, value.ir_fingerprint);
    output.push_str(",\"occurrence\":");
    output.push_str(&value.occurrence.to_string());
    output.push_str(",\"op\":\"draw_vector\",\"owner\":");
    output.push_str(&value.owner.get().to_string());
    output.push_str(",\"page_index\":");
    output.push_str(&value.page_index.to_string());
    output.push_str(",\"placement\":");
    push_jcs_string(&mut output, value.placement.as_str());
    output.push_str(",\"scale\":");
    output.push_str(&value.scale.to_string());
    output.push_str(",\"selected_placement_fingerprint\":");
    push_hash(&mut output, value.selected_placement_fingerprint);
    output.push('}');
    output
}

fn push_hash(output: &mut String, value: [u8; 32]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    output.push('"');
    for byte in value {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output.push('"');
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub struct StagingSafeVectorDisplayFixture {
    pub layout: typaxis_layout::StagingSafeVectorLayoutFixture,
    pub display: StagingSafeVectorDisplay,
}

#[cfg(any(test, feature = "staging-fixtures"))]
pub fn staging_safe_vector_display_fixture(
) -> Result<StagingSafeVectorDisplayFixture, Box<dyn std::error::Error>> {
    let layout = typaxis_layout::staging_safe_vector_layout_fixture()?;
    let display = build_staging_safe_vector_display(
        &layout.package,
        &layout.profile,
        &layout.limits,
        &layout.selected,
    )?;
    Ok(StagingSafeVectorDisplayFixture { layout, display })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draw_vector_v1_frozen_canonical_bytes() {
        let fixture = staging_safe_vector_display_fixture().unwrap();
        assert_eq!(
            fixture.display.receipt().algorithm(),
            STAGING_DRAW_VECTOR_ALGORITHM
        );
        assert_eq!(
            fixture.display.receipt().fingerprint(),
            [
                0xe5, 0x31, 0xff, 0xb5, 0x9b, 0xf2, 0x70, 0x92, 0x80, 0x67, 0xb7, 0xa1, 0xd5, 0x07,
                0x1d, 0xb1, 0x51, 0xf3, 0x16, 0xe1, 0xbd, 0x32, 0xdb, 0xac, 0x3c, 0xbf, 0x6c, 0x9a,
                0x06, 0x08, 0xa1, 0x86,
            ]
        );
        assert_eq!(
            sha256(fixture.display.receipt().canonical_jcs().as_bytes()),
            fixture.display.receipt().fingerprint()
        );
        assert!(fixture
            .display
            .receipt()
            .canonical_jcs()
            .contains("\"algorithm\":\"typaxis.draw-vector-display/1\""));
        assert!(!fixture
            .display
            .receipt()
            .canonical_jcs()
            .contains("content_key"));
    }

    #[test]
    fn vector_display_emits_one_closed_draw_vector_per_selected_use() {
        let fixture = staging_safe_vector_display_fixture().unwrap();
        assert_eq!(fixture.display.receipt().command_count(), 1);
        let command = fixture.display.commands().next().unwrap();
        assert_eq!(command.image_id(), ImageResourceId::new(0));
        assert_eq!(
            command.ir_fingerprint(),
            fixture.layout.selected.placements()[0].ir_fingerprint()
        );
        fixture
            .display
            .verify(
                &fixture.layout.package,
                &fixture.layout.profile,
                &fixture.layout.limits,
                &fixture.layout.selected,
            )
            .unwrap();
        assert!(fixture
            .display
            .receipt()
            .canonical_jcs()
            .contains("\"selected_layout_fingerprint\""));
    }

    #[test]
    fn vector_marked_content_v2_rejects_form_structure_injection() {
        let fixture = crate::staging_precomposed_vector_display_fixture().unwrap();
        let receipt = prove_vector_form_structure_isolation_v2(&fixture.display).unwrap();
        assert_eq!(receipt.form_count(), 1);
        assert_eq!(receipt.page_do_usage_count(), 4);

        let mut mcid_injection = receipt.clone();
        mcid_injection.form_mcid_count = 1;
        let error = mcid_injection.verify(&fixture.display).unwrap_err();
        assert_eq!(error, StagingSafeVectorDisplayError::ReceiptMismatch);
        assert!(error.to_string().starts_with("I9190:"));

        let mut property_injection = receipt;
        property_injection.form_structure_property_count = 1;
        let error = property_injection.verify(&fixture.display).unwrap_err();
        assert_eq!(error, StagingSafeVectorDisplayError::ReceiptMismatch);
        assert!(error.to_string().starts_with("I9190:"));
    }
}
