use std::collections::BTreeSet;
use typaxis_core::{push_jcs_string, sha256, NodeId, ValidatedResourceLimits};
use typaxis_display_list::{
    StagingSemanticContainerDisplay, StagingSemanticContainerPaint, StagingSemanticStructureRole,
};
use typaxis_syntax::{StagingSemanticContainerProfileView, ValidatedStagingSemanticPackage};

const PDF_CLOSURE_ALGORITHM: &str = "typaxis.semantic-container-pdf-closure/1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerStructureInput {
    owner: NodeId,
    role: StagingSemanticStructureRole,
    role_map_target: &'static str,
    fragment_paint_fingerprints: Vec<[u8; 32]>,
    logical_child_owners: Vec<NodeId>,
}

impl StagingSemanticContainerStructureInput {
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn role(&self) -> StagingSemanticStructureRole {
        self.role
    }
    pub const fn role_map_target(&self) -> &'static str {
        self.role_map_target
    }
    pub fn fragment_paint_fingerprints(&self) -> &[[u8; 32]] {
        &self.fragment_paint_fingerprints
    }
    pub fn logical_child_owners(&self) -> &[NodeId] {
        &self.logical_child_owners
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerPdfPageObservation {
    page_index: u32,
    owner: NodeId,
    fragment_index: u32,
    role: StagingSemanticStructureRole,
    child_count: u32,
    display_paint_fingerprint: [u8; 32],
    raster_fingerprint: [u8; 32],
    content_stream_fingerprint: [u8; 32],
}

impl StagingSemanticContainerPdfPageObservation {
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn owner(&self) -> NodeId {
        self.owner
    }
    pub const fn fragment_index(&self) -> u32 {
        self.fragment_index
    }
    pub const fn role(&self) -> StagingSemanticStructureRole {
        self.role
    }
    pub const fn child_count(&self) -> u32 {
        self.child_count
    }
    pub const fn display_paint_fingerprint(&self) -> [u8; 32] {
        self.display_paint_fingerprint
    }
    pub const fn raster_fingerprint(&self) -> [u8; 32] {
        self.raster_fingerprint
    }
    pub const fn content_stream_fingerprint(&self) -> [u8; 32] {
        self.content_stream_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerPdfClosureReceipt {
    selected_layout_fingerprint: [u8; 32],
    display_fingerprint: [u8; 32],
    page_count: u32,
    pdf_sha256: [u8; 32],
    fingerprint: [u8; 32],
    canonical_jcs: String,
}

impl StagingSemanticContainerPdfClosureReceipt {
    pub const fn selected_layout_fingerprint(&self) -> [u8; 32] {
        self.selected_layout_fingerprint
    }
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn page_count(&self) -> u32 {
        self.page_count
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSemanticContainerPdf {
    bytes: Vec<u8>,
    pages: Vec<StagingSemanticContainerPdfPageObservation>,
    structure_inputs: Vec<StagingSemanticContainerStructureInput>,
    receipt: StagingSemanticContainerPdfClosureReceipt,
}

impl StagingSemanticContainerPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn pages(&self) -> &[StagingSemanticContainerPdfPageObservation] {
        &self.pages
    }
    pub fn structure_inputs(&self) -> &[StagingSemanticContainerStructureInput] {
        &self.structure_inputs
    }
    pub const fn receipt(&self) -> &StagingSemanticContainerPdfClosureReceipt {
        &self.receipt
    }

    pub fn verify(
        &self,
        package: &ValidatedStagingSemanticPackage,
        profile: &StagingSemanticContainerProfileView,
        display: &StagingSemanticContainerDisplay,
    ) -> Result<(), StagingSemanticContainerPdfError> {
        display
            .verify(package, profile)
            .map_err(|_| StagingSemanticContainerPdfError::DisplayMismatch)?;
        let expected = serialize_pdf(display, profile.limits())?;
        enforce_pdf_limits(profile.limits(), &expected)?;
        if self.bytes != expected.bytes
            || self.pages != expected.pages
            || self.structure_inputs != expected.structure_inputs
            || self.receipt.selected_layout_fingerprint
                != display.receipt().selected_layout_fingerprint()
            || self.receipt.display_fingerprint != display.receipt().fingerprint()
            || usize::try_from(self.receipt.page_count) != Ok(self.pages.len())
            || self.receipt.pdf_sha256 != sha256(&self.bytes)
        {
            return Err(StagingSemanticContainerPdfError::ReceiptMismatch);
        }
        let canonical = encode_closure(
            display.receipt().selected_layout_fingerprint(),
            display.receipt().fingerprint(),
            &self.pages,
            &self.structure_inputs,
            self.receipt.pdf_sha256,
        );
        if canonical != self.receipt.canonical_jcs
            || sha256(canonical.as_bytes()) != self.receipt.fingerprint
        {
            return Err(StagingSemanticContainerPdfError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSemanticContainerPdfError {
    DisplayMismatch,
    InvalidDisplayPage,
    ObjectLimit,
    OutputLimit,
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSemanticContainerPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DisplayMismatch => {
                formatter.write_str("I9190: semantic Display receipt mismatch")
            }
            Self::InvalidDisplayPage => {
                formatter.write_str("I9190: semantic Display page is invalid")
            }
            Self::ObjectLimit => formatter.write_str("L5100: semantic PDF object limit exceeded"),
            Self::OutputLimit => formatter.write_str("L5100: semantic PDF byte limit exceeded"),
            Self::ArithmeticOverflow => {
                formatter.write_str("L5100: semantic PDF arithmetic overflow")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: semantic PDF receipt mismatch"),
            Self::AllocationFailure => formatter.write_str("L5100: semantic PDF allocation failed"),
        }
    }
}

impl std::error::Error for StagingSemanticContainerPdfError {}

pub fn write_staging_semantic_container_pdf(
    package: &ValidatedStagingSemanticPackage,
    profile: &StagingSemanticContainerProfileView,
    display: &StagingSemanticContainerDisplay,
) -> Result<StagingSemanticContainerPdf, StagingSemanticContainerPdfError> {
    display
        .verify(package, profile)
        .map_err(|_| StagingSemanticContainerPdfError::DisplayMismatch)?;
    let serialized = serialize_pdf(display, profile.limits())?;
    enforce_pdf_limits(profile.limits(), &serialized)?;
    let bytes = serialized.bytes;
    let pages = serialized.pages;
    let structure_inputs = serialized.structure_inputs;
    let pdf_sha256 = sha256(&bytes);
    let canonical_jcs = encode_closure(
        display.receipt().selected_layout_fingerprint(),
        display.receipt().fingerprint(),
        &pages,
        &structure_inputs,
        pdf_sha256,
    );
    let pdf = StagingSemanticContainerPdf {
        receipt: StagingSemanticContainerPdfClosureReceipt {
            selected_layout_fingerprint: display.receipt().selected_layout_fingerprint(),
            display_fingerprint: display.receipt().fingerprint(),
            page_count: u32::try_from(pages.len())
                .map_err(|_| StagingSemanticContainerPdfError::ObjectLimit)?,
            pdf_sha256,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        bytes,
        pages,
        structure_inputs,
    };
    pdf.verify(package, profile, display)?;
    Ok(pdf)
}

struct SerializedSemanticPdf {
    bytes: Vec<u8>,
    pages: Vec<StagingSemanticContainerPdfPageObservation>,
    structure_inputs: Vec<StagingSemanticContainerStructureInput>,
}

fn enforce_pdf_limits(
    limits: &ValidatedResourceLimits,
    pdf: &SerializedSemanticPdf,
) -> Result<(), StagingSemanticContainerPdfError> {
    let object_count = 2usize
        .checked_add(
            pdf.pages
                .len()
                .checked_mul(2)
                .ok_or(StagingSemanticContainerPdfError::ObjectLimit)?,
        )
        .ok_or(StagingSemanticContainerPdfError::ObjectLimit)?;
    if u32::try_from(object_count).map_err(|_| StagingSemanticContainerPdfError::ObjectLimit)?
        > limits.get().max_pdf_objects
    {
        return Err(StagingSemanticContainerPdfError::ObjectLimit);
    }
    if u64::try_from(pdf.bytes.len()).map_err(|_| StagingSemanticContainerPdfError::OutputLimit)?
        > limits.get().max_output_bytes
    {
        return Err(StagingSemanticContainerPdfError::OutputLimit);
    }
    Ok(())
}

fn serialize_pdf(
    display: &StagingSemanticContainerDisplay,
    limits: &ValidatedResourceLimits,
) -> Result<SerializedSemanticPdf, StagingSemanticContainerPdfError> {
    let page_count = display.pages().len();
    let object_count = 2usize
        .checked_add(
            page_count
                .checked_mul(2)
                .ok_or(StagingSemanticContainerPdfError::ObjectLimit)?,
        )
        .ok_or(StagingSemanticContainerPdfError::ObjectLimit)?;
    if u32::try_from(object_count).map_err(|_| StagingSemanticContainerPdfError::ObjectLimit)?
        > limits.get().max_pdf_objects
    {
        return Err(StagingSemanticContainerPdfError::ObjectLimit);
    }
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(object_count)
        .map_err(|_| StagingSemanticContainerPdfError::AllocationFailure)?;
    objects.resize_with(object_count, Vec::<u8>::new);
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    let mut kids = String::from("[");
    for index in 0..page_count {
        if index > 0 {
            kids.push(' ');
        }
        let page_object = 3usize + index * 2;
        kids.push_str(&format!("{page_object} 0 R"));
    }
    kids.push(']');
    objects[1] = format!("<< /Type /Pages /Count {page_count} /Kids {kids} >>").into_bytes();

    let mut observations = Vec::new();
    observations
        .try_reserve_exact(page_count)
        .map_err(|_| StagingSemanticContainerPdfError::AllocationFailure)?;
    let mut structure_inputs: Vec<StagingSemanticContainerStructureInput> = Vec::new();
    structure_inputs
        .try_reserve_exact(page_count)
        .map_err(|_| StagingSemanticContainerPdfError::AllocationFailure)?;
    let mut structure_owners = BTreeSet::new();
    for (index, page) in display.pages().iter().enumerate() {
        if usize::try_from(page.page_index()) != Ok(index) || page.paints().len() != 1 {
            return Err(StagingSemanticContainerPdfError::InvalidDisplayPage);
        }
        let paint = &page.paints()[0];
        let content = encode_content_stream(paint);
        let page_object_index = 2 + index * 2;
        let content_object_index = page_object_index + 1;
        let content_object_number = content_object_index + 1;
        objects[page_object_index] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] /Resources << >> /Contents {content_object_number} 0 R >>"
        )
        .into_bytes();
        let mut stream = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
        stream.extend_from_slice(content.as_bytes());
        stream.extend_from_slice(b"\nendstream");
        objects[content_object_index] = stream;
        observations.push(StagingSemanticContainerPdfPageObservation {
            page_index: page.page_index(),
            owner: paint.owner(),
            fragment_index: paint.fragment_index(),
            role: paint.structure_role(),
            child_count: u32::try_from(paint.child_paints().len())
                .map_err(|_| StagingSemanticContainerPdfError::ObjectLimit)?,
            display_paint_fingerprint: paint.fingerprint(),
            raster_fingerprint: page.raster_observation().raster_fingerprint(),
            content_stream_fingerprint: sha256(content.as_bytes()),
        });
        if let Some(structure) = structure_inputs
            .last_mut()
            .filter(|structure| structure.owner == paint.owner())
        {
            if structure.role != paint.structure_role()
                || structure.role_map_target != paint.structure_role().role_map_target()
            {
                return Err(StagingSemanticContainerPdfError::InvalidDisplayPage);
            }
            structure
                .fragment_paint_fingerprints
                .push(paint.fingerprint());
            structure
                .logical_child_owners
                .extend(paint.child_paints().iter().map(|child| child.owner()));
        } else {
            if !structure_owners.insert(paint.owner()) {
                return Err(StagingSemanticContainerPdfError::InvalidDisplayPage);
            }
            structure_inputs.push(StagingSemanticContainerStructureInput {
                owner: paint.owner(),
                role: paint.structure_role(),
                role_map_target: paint.structure_role().role_map_target(),
                fragment_paint_fingerprints: vec![paint.fingerprint()],
                logical_child_owners: paint
                    .child_paints()
                    .iter()
                    .map(|child| child.owner())
                    .collect(),
            });
        }
    }
    Ok(SerializedSemanticPdf {
        bytes: serialize_objects(&objects, limits.get().max_output_bytes)?,
        pages: observations,
        structure_inputs,
    })
}

fn encode_content_stream(paint: &StagingSemanticContainerPaint) -> String {
    let mut output = format!(
        "% typaxis-semantic /{} /{} owner={} fragment={}\nq\n0 0 0 rg\n",
        paint.structure_role().as_str(),
        paint.structure_role().role_map_target(),
        paint.owner().get(),
        paint.fragment_index()
    );
    for child in paint.child_paints() {
        output.push_str(&format!(
            "{} {} {} {} re f\n",
            child.x(),
            child.y(),
            child.width(),
            child.height()
        ));
    }
    output.push('Q');
    output
}

fn serialize_objects(
    objects: &[Vec<u8>],
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingSemanticContainerPdfError> {
    fn append(
        output: &mut Vec<u8>,
        bytes: &[u8],
        max_output_bytes: u64,
    ) -> Result<(), StagingSemanticContainerPdfError> {
        let next_len = output
            .len()
            .checked_add(bytes.len())
            .ok_or(StagingSemanticContainerPdfError::OutputLimit)?;
        if u64::try_from(next_len).map_err(|_| StagingSemanticContainerPdfError::OutputLimit)?
            > max_output_bytes
        {
            return Err(StagingSemanticContainerPdfError::OutputLimit);
        }
        output
            .try_reserve(bytes.len())
            .map_err(|_| StagingSemanticContainerPdfError::AllocationFailure)?;
        output.extend_from_slice(bytes);
        Ok(())
    }

    let mut output = Vec::new();
    append(
        &mut output,
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n",
        max_output_bytes,
    )?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| StagingSemanticContainerPdfError::AllocationFailure)?;
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        let number = index + 1;
        append(
            &mut output,
            format!("{number} 0 obj\n").as_bytes(),
            max_output_bytes,
        )?;
        append(&mut output, object, max_output_bytes)?;
        append(&mut output, b"\nendobj\n", max_output_bytes)?;
    }
    let xref = output.len();
    append(
        &mut output,
        format!("xref\n0 {}\n", objects.len() + 1).as_bytes(),
        max_output_bytes,
    )?;
    append(&mut output, b"0000000000 65535 f \n", max_output_bytes)?;
    for offset in offsets {
        append(
            &mut output,
            format!("{offset:010} 00000 n \n").as_bytes(),
            max_output_bytes,
        )?;
    }
    append(
        &mut output,
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
        max_output_bytes,
    )?;
    Ok(output)
}

fn encode_closure(
    selected: [u8; 32],
    display: [u8; 32],
    pages: &[StagingSemanticContainerPdfPageObservation],
    structures: &[StagingSemanticContainerStructureInput],
    pdf_sha256: [u8; 32],
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, PDF_CLOSURE_ALGORITHM);
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"pages\":[");
    for (index, page) in pages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"child_count\":");
        output.push_str(&page.child_count.to_string());
        output.push_str(",\"content_stream_fingerprint\":");
        push_hash(&mut output, page.content_stream_fingerprint);
        output.push_str(",\"display_paint_fingerprint\":");
        push_hash(&mut output, page.display_paint_fingerprint);
        output.push_str(",\"fragment_index\":");
        output.push_str(&page.fragment_index.to_string());
        output.push_str(",\"owner\":");
        output.push_str(&page.owner.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"raster_fingerprint\":");
        push_hash(&mut output, page.raster_fingerprint);
        output.push_str(",\"role\":");
        push_jcs_string(&mut output, page.role.as_str());
        output.push('}');
    }
    output.push_str("],\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
    output.push_str(",\"selected_layout_fingerprint\":");
    push_hash(&mut output, selected);
    output.push_str(",\"structure_inputs\":[");
    for (index, structure) in structures.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"fragment_paint_fingerprints\":[");
        for (fragment_index, fingerprint) in structure
            .fragment_paint_fingerprints
            .iter()
            .copied()
            .enumerate()
        {
            if fragment_index > 0 {
                output.push(',');
            }
            push_hash(&mut output, fingerprint);
        }
        output.push_str("],\"logical_child_owners\":[");
        for (child_index, owner) in structure.logical_child_owners.iter().enumerate() {
            if child_index > 0 {
                output.push(',');
            }
            output.push_str(&owner.get().to_string());
        }
        output.push_str("],\"owner\":");
        output.push_str(&structure.owner.get().to_string());
        output.push_str(",\"role\":");
        push_jcs_string(&mut output, structure.role.as_str());
        output.push_str(",\"role_map_target\":");
        push_jcs_string(&mut output, structure.role_map_target);
        output.push('}');
    }
    output.push_str("]}");
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
    use typaxis_display_list::build_staging_semantic_container_display_fixture;
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
    fn semantic_container_pdf_observation_matches_display_raster_and_structure_input() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let display =
            build_staging_semantic_container_display_fixture(&package, &profile, 2).unwrap();
        let pdf = write_staging_semantic_container_pdf(&package, &profile, &display).unwrap();
        assert!(pdf.bytes().starts_with(b"%PDF-1.7"));
        assert_eq!(pdf.pages().len(), display.pages().len());
        for (observed, page) in pdf.pages().iter().zip(display.pages()) {
            assert_eq!(
                observed.raster_fingerprint(),
                page.raster_observation().raster_fingerprint()
            );
        }
        assert_eq!(
            pdf.structure_inputs()[0].role(),
            StagingSemanticStructureRole::Result
        );
        assert_eq!(pdf.structure_inputs().len(), 3);
        assert_eq!(pdf.structure_inputs()[0].owner(), NodeId::new(1));
        assert_eq!(
            pdf.structure_inputs()[0]
                .fragment_paint_fingerprints()
                .len(),
            2
        );
        assert_eq!(
            pdf.structure_inputs()[0].logical_child_owners(),
            &[NodeId::new(2), NodeId::new(4), NodeId::new(7)]
        );
        assert!(!pdf
            .bytes()
            .windows(b"/StructTreeRoot".len())
            .any(|window| window == b"/StructTreeRoot"));
    }

    #[test]
    fn semantic_container_pdf_detects_serialized_byte_tamper() {
        let limits = ValidatedResourceLimits::new(ResourceLimits::default()).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let profile = test_profile(&package, &limits);
        let display =
            build_staging_semantic_container_display_fixture(&package, &profile, 2).unwrap();
        let mut pdf = write_staging_semantic_container_pdf(&package, &profile, &display).unwrap();
        pdf.bytes.push(b' ');
        assert_eq!(
            pdf.verify(&package, &profile, &display),
            Err(StagingSemanticContainerPdfError::ReceiptMismatch)
        );
    }

    #[test]
    fn semantic_container_pdf_enforces_receipted_object_and_output_limits() {
        fn write_with_limits(
            update: impl FnOnce(&mut ResourceLimits),
        ) -> Result<StagingSemanticContainerPdf, StagingSemanticContainerPdfError> {
            let mut raw_limits = ResourceLimits::default();
            update(&mut raw_limits);
            let limits = ValidatedResourceLimits::new(raw_limits).unwrap();
            let decoded = StagingSemanticDocumentPackageDecoder::new()
                .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
                .unwrap();
            let package = StagingSemanticPackageParser::new()
                .parse(decoded, &limits)
                .unwrap();
            let profile = test_profile(&package, &limits);
            let display =
                build_staging_semantic_container_display_fixture(&package, &profile, 2).unwrap();
            write_staging_semantic_container_pdf(&package, &profile, &display)
        }

        assert_eq!(
            write_with_limits(|limits| limits.max_pdf_objects = 9),
            Err(StagingSemanticContainerPdfError::ObjectLimit)
        );
        assert!(write_with_limits(|limits| limits.max_pdf_objects = 10).is_ok());
        let exact_output = u64::try_from(write_with_limits(|_| {}).unwrap().bytes().len()).unwrap();
        assert!(write_with_limits(|limits| limits.max_output_bytes = exact_output).is_ok());
        assert_eq!(
            write_with_limits(|limits| limits.max_output_bytes = exact_output - 1),
            Err(StagingSemanticContainerPdfError::OutputLimit)
        );
    }
}
