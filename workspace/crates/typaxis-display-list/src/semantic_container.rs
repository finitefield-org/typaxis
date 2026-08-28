use typaxis_core::{push_jcs_string, sha256, NodeId, SourceSpan};
use typaxis_document::SemanticContainerKind;
use typaxis_layout::{StagingSemanticContainerFragment, StagingSemanticContainerSelectedLayout};
use typaxis_syntax::{StagingSemanticContainerProfileView, ValidatedStagingSemanticPackage};

const DISPLAY_RECEIPT_ALGORITHM: &str = "typaxis.semantic-container-display/1";
const RASTER_OBSERVATION_ALGORITHM: &str = "typaxis.semantic-container-raster-observation/1";

/// Typed future structure-tree input. PDF consumes this enum and never
/// reinterprets the wire semantic-kind string.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum StagingSemanticStructureRole {
    Result,
    Proof,
    Exercise,
}

impl StagingSemanticStructureRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Result => "Result",
            Self::Proof => "Proof",
            Self::Exercise => "Exercise",
        }
    }

    pub const fn role_map_target(self) -> &'static str {
        "Div"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticChildPaint {
    owner: NodeId,
    sequence: u32,
    x: i64,
    y: i64,
    width: i64,
    height: i64,
    fingerprint: [u8; 32],
}

impl StagingSemanticChildPaint {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn sequence(&self) -> u32 {
        self.sequence
    }
    pub const fn x(&self) -> i64 {
        self.x
    }
    pub const fn y(&self) -> i64 {
        self.y
    }
    pub const fn width(&self) -> i64 {
        self.width
    }
    pub const fn height(&self) -> i64 {
        self.height
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerPaint {
    owner: NodeId,
    semantic_kind: SemanticContainerKind,
    structure_role: StagingSemanticStructureRole,
    fragment_index: u32,
    first: bool,
    last: bool,
    source_span: SourceSpan,
    style_fingerprint: [u8; 32],
    selected_fragment_fingerprint: [u8; 32],
    child_paints: Vec<StagingSemanticChildPaint>,
    fingerprint: [u8; 32],
}

impl StagingSemanticContainerPaint {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn semantic_kind(&self) -> SemanticContainerKind {
        self.semantic_kind
    }
    pub const fn structure_role(&self) -> StagingSemanticStructureRole {
        self.structure_role
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn is_first(&self) -> bool {
        self.first
    }
    pub const fn is_last(&self) -> bool {
        self.last
    }
    pub const fn source_span(&self) -> SourceSpan {
        self.source_span
    }
    pub const fn style_fingerprint(&self) -> [u8; 32] {
        self.style_fingerprint
    }
    pub const fn selected_fragment_fingerprint(&self) -> [u8; 32] {
        self.selected_fragment_fingerprint
    }
    pub fn child_paints(&self) -> &[StagingSemanticChildPaint] {
        &self.child_paints
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticRasterObservation {
    page_index: u32,
    paint_fingerprint: [u8; 32],
    raster_fingerprint: [u8; 32],
    painted_child_count: u32,
}

impl StagingSemanticRasterObservation {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn paint_fingerprint(&self) -> [u8; 32] {
        self.paint_fingerprint
    }
    pub const fn raster_fingerprint(&self) -> [u8; 32] {
        self.raster_fingerprint
    }
    pub const fn painted_child_count(&self) -> u32 {
        self.painted_child_count
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerDisplayPage {
    page_index: u32,
    paints: Vec<StagingSemanticContainerPaint>,
    raster_observation: StagingSemanticRasterObservation,
}

impl StagingSemanticContainerDisplayPage {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn paints(&self) -> &[StagingSemanticContainerPaint] {
        &self.paints
    }
    pub const fn raster_observation(&self) -> &StagingSemanticRasterObservation {
        &self.raster_observation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerDisplayReceipt {
    selected_layout_fingerprint: [u8; 32],
    page_count: u32,
    paint_count: u32,
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl StagingSemanticContainerDisplayReceipt {
    pub const fn selected_layout_fingerprint(&self) -> [u8; 32] {
        self.selected_layout_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn paint_count(&self) -> u32 {
        self.paint_count
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerDisplay {
    selected: StagingSemanticContainerSelectedLayout,
    pages: Vec<StagingSemanticContainerDisplayPage>,
    receipt: StagingSemanticContainerDisplayReceipt,
}

impl StagingSemanticContainerDisplay {
    pub fn pages(&self) -> &[StagingSemanticContainerDisplayPage] {
        &self.pages
    }
    pub const fn receipt(&self) -> &StagingSemanticContainerDisplayReceipt {
        &self.receipt
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSemanticContainerProfileView,
    ) -> Result<(), StagingSemanticContainerDisplayError> {
        self.selected
            .verify(package, profile)
            .map_err(|_| StagingSemanticContainerDisplayError::SelectedLayoutMismatch)?;
        if self.receipt.selected_layout_fingerprint != self.selected.receipt().fingerprint()
            || usize::try_from(self.receipt.page_count) != Ok(self.pages.len())
            || usize::try_from(self.receipt.paint_count)
                != Ok(self.pages.iter().map(|page| page.paints.len()).sum())
            || !pages_are_closed(&self.pages, self.selected.fragments())
        {
            return Err(StagingSemanticContainerDisplayError::ReceiptMismatch);
        }
        let canonical = encode_display(self.selected.receipt().fingerprint(), &self.pages);
        if canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.fingerprint
        {
            return Err(StagingSemanticContainerDisplayError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerDisplayError {
    SelectedLayoutMismatch,
    InvalidGeometry(NodeId),
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSemanticContainerDisplayError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelectedLayoutMismatch => {
                formatter.write_str("I9190: semantic selected-layout mismatch")
            }
            Self::InvalidGeometry(owner) => write!(
                formatter,
                "L5101: invalid semantic display geometry at node {}",
                owner.get()
            ),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5101: semantic display arithmetic overflow")
            }
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: semantic display receipt mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5101: semantic display allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingSemanticContainerDisplayError {}

pub fn build_staging_semantic_container_display(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerProfileView,
    selected: &StagingSemanticContainerSelectedLayout,
) -> Result<StagingSemanticContainerDisplay, StagingSemanticContainerDisplayError> {
    selected
        .verify(package, profile)
        .map_err(|_| StagingSemanticContainerDisplayError::SelectedLayoutMismatch)?;
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(selected.fragments().len())
        .map_err(|_| StagingSemanticContainerDisplayError::AllocationFailure)?;
    for fragment in selected.fragments() {
        let paint = paint_fragment(fragment)?;
        let raster_observation = raster_observation(fragment.page_index(), &paint)?;
        pages.push(StagingSemanticContainerDisplayPage {
            page_index: fragment.page_index(),
            paints: vec![paint],
            raster_observation,
        });
    }
    let canonical_jcs = encode_display(selected.receipt().fingerprint(), &pages);
    let display = StagingSemanticContainerDisplay {
        selected: selected.clone(),
        receipt: StagingSemanticContainerDisplayReceipt {
            selected_layout_fingerprint: selected.receipt().fingerprint(),
            page_count: u32::try_from(pages.len())
                .map_err(|_| StagingSemanticContainerDisplayError::ArithmeticOverflow)?,
            paint_count: u32::try_from(pages.iter().map(|page| page.paints.len()).sum::<usize>())
                .map_err(|_| StagingSemanticContainerDisplayError::ArithmeticOverflow)?,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        pages,
    };
    display.verify(package, profile)?;
    Ok(display)
}

#[cfg(feature = "staging-fixtures")]
pub fn build_staging_semantic_container_display_fixture(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerProfileView,
    fragment_item_capacity: u32,
) -> Result<StagingSemanticContainerDisplay, StagingSemanticContainerDisplayError> {
    let selected = typaxis_layout::layout_staging_semantic_containers(
        package,
        profile,
        fragment_item_capacity,
    )
    .map_err(|_| StagingSemanticContainerDisplayError::SelectedLayoutMismatch)?;
    build_staging_semantic_container_display(package, profile, &selected)
}

fn paint_fragment(
    fragment: &StagingSemanticContainerFragment,
) -> Result<StagingSemanticContainerPaint, StagingSemanticContainerDisplayError> {
    let style = fragment.computed_style().block_style();
    let x = style.start_indent().get().raw();
    let width = 100i64
        .checked_sub(x)
        .and_then(|value| value.checked_sub(style.end_indent().get().raw()))
        .filter(|value| *value > 0)
        .ok_or(StagingSemanticContainerDisplayError::InvalidGeometry(
            fragment.owner(),
        ))?;
    let mut child_paints = Vec::new();
    let fragment_space_before = if fragment.is_first() {
        style.space_before().get().raw()
    } else {
        0
    };
    for (sequence, owner) in fragment.child_owners().iter().enumerate() {
        let sequence = u32::try_from(sequence)
            .map_err(|_| StagingSemanticContainerDisplayError::ArithmeticOverflow)?;
        let y = i64::from(sequence)
            .checked_mul(12)
            .and_then(|value| value.checked_add(fragment_space_before))
            .ok_or(StagingSemanticContainerDisplayError::ArithmeticOverflow)?;
        let mut child = StagingSemanticChildPaint {
            owner: *owner,
            sequence,
            x,
            y,
            width,
            height: 10,
            fingerprint: [0; 32],
        };
        child.fingerprint = sha256(encode_child_paint(&child).as_bytes());
        child_paints.push(child);
    }
    let structure_role = match fragment.semantic_kind() {
        SemanticContainerKind::Result => StagingSemanticStructureRole::Result,
        SemanticContainerKind::Proof => StagingSemanticStructureRole::Proof,
        SemanticContainerKind::Exercise => StagingSemanticStructureRole::Exercise,
    };
    let mut paint = StagingSemanticContainerPaint {
        owner: fragment.owner(),
        semantic_kind: fragment.semantic_kind(),
        structure_role,
        fragment_index: fragment.fragment_index(),
        first: fragment.is_first(),
        last: fragment.is_last(),
        source_span: fragment.source_span(),
        style_fingerprint: fragment.style_fingerprint(),
        selected_fragment_fingerprint: fragment.fingerprint(),
        child_paints,
        fingerprint: [0; 32],
    };
    paint.fingerprint = sha256(encode_paint(&paint).as_bytes());
    Ok(paint)
}

fn raster_observation(
    page_index: u32,
    paint: &StagingSemanticContainerPaint,
) -> Result<StagingSemanticRasterObservation, StagingSemanticContainerDisplayError> {
    let canonical = encode_raster_observation(page_index, paint);
    Ok(StagingSemanticRasterObservation {
        page_index,
        paint_fingerprint: paint.fingerprint,
        raster_fingerprint: sha256(canonical.as_bytes()),
        painted_child_count: u32::try_from(paint.child_paints.len())
            .map_err(|_| StagingSemanticContainerDisplayError::ArithmeticOverflow)?,
    })
}

fn pages_are_closed(
    pages: &[StagingSemanticContainerDisplayPage],
    fragments: &[StagingSemanticContainerFragment],
) -> bool {
    if pages.len() != fragments.len() {
        return false;
    }
    pages.iter().zip(fragments).all(|(page, fragment)| {
        let Ok(expected_paint) = paint_fragment(fragment) else {
            return false;
        };
        let Ok(expected_raster) = raster_observation(page.page_index, &expected_paint) else {
            return false;
        };
        page.page_index == fragment.page_index()
            && page.paints.as_slice() == std::slice::from_ref(&expected_paint)
            && page.raster_observation == expected_raster
    })
}

fn encode_child_paint(paint: &StagingSemanticChildPaint) -> String {
    format!(
        "{{\"height\":{},\"owner\":{},\"sequence\":{},\"width\":{},\"x\":{},\"y\":{}}}",
        paint.height,
        paint.owner.get(),
        paint.sequence,
        paint.width,
        paint.x,
        paint.y
    )
}

fn encode_paint(paint: &StagingSemanticContainerPaint) -> String {
    let mut output = String::from("{\"child_paints\":[");
    for (index, child) in paint.child_paints.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(&encode_child_paint(child));
    }
    output.push_str("],\"first\":");
    output.push_str(if paint.first { "true" } else { "false" });
    output.push_str(",\"fragment_index\":");
    output.push_str(&paint.fragment_index.to_string());
    output.push_str(",\"kind\":");
    push_jcs_string(&mut output, paint.semantic_kind.as_str());
    output.push_str(",\"last\":");
    output.push_str(if paint.last { "true" } else { "false" });
    output.push_str(",\"owner\":");
    output.push_str(&paint.owner.get().to_string());
    output.push_str(",\"selected_fragment_fingerprint\":");
    push_hash(&mut output, paint.selected_fragment_fingerprint);
    output.push_str(",\"source_span\":{\"end_byte\":");
    output.push_str(&paint.source_span.end_byte().get().to_string());
    output.push_str(",\"source_id\":");
    output.push_str(&paint.source_span.source_id().get().to_string());
    output.push_str(",\"start_byte\":");
    output.push_str(&paint.source_span.start_byte().get().to_string());
    output.push_str("},\"structure_role\":");
    push_jcs_string(&mut output, paint.structure_role.as_str());
    output.push_str(",\"style_fingerprint\":");
    push_hash(&mut output, paint.style_fingerprint);
    output.push('}');
    output
}

fn encode_raster_observation(page_index: u32, paint: &StagingSemanticContainerPaint) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, RASTER_OBSERVATION_ALGORITHM);
    output.push_str(",\"page_index\":");
    output.push_str(&page_index.to_string());
    output.push_str(",\"paint\":");
    output.push_str(&encode_paint(paint));
    output.push('}');
    output
}

fn encode_display(
    selected_layout_fingerprint: [u8; 32],
    pages: &[StagingSemanticContainerDisplayPage],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, DISPLAY_RECEIPT_ALGORITHM);
    output.push_str(",\"pages\":[");
    for (page_index, page) in pages.iter().enumerate() {
        if page_index > 0 {
            output.push(',');
        }
        output.push_str("{\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"paints\":[");
        for (paint_index, paint) in page.paints.iter().enumerate() {
            if paint_index > 0 {
                output.push(',');
            }
            output.push_str(&encode_paint(paint));
        }
        output.push_str("],\"raster_fingerprint\":");
        push_hash(&mut output, page.raster_observation.raster_fingerprint);
        output.push('}');
    }
    output.push_str("],\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected_layout_fingerprint);
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

#[cfg(test)]
mod tests {
    use super::*;
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};
    use typaxis_layout::layout_staging_semantic_containers;
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::StagingSemanticPackageParser;

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/semantic-container/job/document-package.json"
    ));

    fn test_profile(
        package: &ValidatedStagingSemanticPackage,
        limits: &ValidatedResourceLimits,
    ) -> StagingSemanticContainerProfileView {
        StagingSemanticContainerProfileView::new(package, limits).unwrap()
    }

    #[test]
    fn semantic_container_display_binds_selected_children_kind_source_and_style() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        let display =
            build_staging_semantic_container_display(&package, &profile, &selected).unwrap();
        assert_eq!(display.pages().len(), selected.fragments().len());
        assert_eq!(
            display.pages()[0].paints()[0].structure_role(),
            StagingSemanticStructureRole::Result
        );
        assert_eq!(display.pages()[0].paints()[0].child_paints().len(), 2);
        assert_eq!(display.pages()[0].paints()[0].child_paints()[0].y(), 7);
        assert_eq!(display.pages()[1].paints()[0].child_paints()[0].y(), 0);
        display.verify(&package, &profile).unwrap();
    }

    #[test]
    fn semantic_container_display_detects_child_paint_tamper() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        let mut display =
            build_staging_semantic_container_display(&package, &profile, &selected).unwrap();
        display.pages[0].paints[0].child_paints[0].owner = NodeId::new(99);
        assert_eq!(
            display.verify(&package, &profile),
            Err(StagingSemanticContainerDisplayError::ReceiptMismatch)
        );
    }

    #[test]
    fn semantic_container_display_rederives_typed_role_after_self_consistent_tamper() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let selected = layout_staging_semantic_containers(&package, &profile, 2).unwrap();
        let mut display =
            build_staging_semantic_container_display(&package, &profile, &selected).unwrap();
        display.pages[0].paints[0].structure_role = StagingSemanticStructureRole::Proof;
        display.pages[0].paints[0].fingerprint =
            sha256(encode_paint(&display.pages[0].paints[0]).as_bytes());
        display.pages[0].raster_observation =
            raster_observation(0, &display.pages[0].paints[0]).unwrap();
        let canonical = encode_display(selected.receipt().fingerprint(), &display.pages);
        display.receipt.fingerprint = sha256(canonical.as_bytes());
        display.receipt.canonical_jcs = canonical;
        assert_eq!(
            display.verify(&package, &profile),
            Err(StagingSemanticContainerDisplayError::ReceiptMismatch)
        );
    }
}
