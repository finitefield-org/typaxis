use crate::descriptor::{
    MachineBlockKind, MachineInlineKind, MachinePageFrame, MachinePageValue,
    MachineProfileDescriptor, MachineReferenceFormat,
};
use crate::HostCapabilityDescriptor;
use typaxis_core::{
    sha256, DocumentFingerprint, FontFaceId, JsonPointer, MachineInputFingerprint,
    MachinePdfProfileId, NodeId, PortablePath, StyleFingerprint,
};
use typaxis_diagnostics::{
    Diagnostic, DiagnosticBuilder, DiagnosticCode, DiagnosticLocation, GlobalDiagnosticScope,
    LayoutErrorSubject, MachineDiagnosticBudgetError, MachineDiagnosticLender,
    MachineDiagnosticPhase, MasterErrorSubject, PublicMachineError, ResourceErrorSubject, Severity,
    StyleErrorSubject, StylePropertyName, L5100, L5101, R7100,
};
use typaxis_syntax::machine_profile_boundary::{
    Block, FootnoteDefinition, Inline, MachineInputSessionIdentity, PageMaster, PageMasterRule,
    ReferenceFormat, StyleRule, StyleValue,
};
use typaxis_syntax::ValidatedMachinePackage;

const UNSUPPORTED_CONTENT_MESSAGE: &str =
    "content is not supported by the selected machine PDF profile";
const UNSUPPORTED_STYLE_MESSAGE: &str =
    "style is not supported by the selected machine PDF profile";
const UNSUPPORTED_MASTER_MESSAGE: &str =
    "page master is not supported by the selected machine PDF profile";
const UNSUPPORTED_RESOURCE_MESSAGE: &str =
    "resource is not supported by the selected machine PDF profile";
const MISSING_TEXT_FONT_MESSAGE: &str =
    "text-producing content requires a declared machine PDF font";
const HOST_UNAVAILABLE_MESSAGE: &str = "required compiled host capability is unavailable";
pub const BASIC_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.basic-profile-receipt/1";
pub const FOOTNOTE_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.footnote-profile-receipt/1";
pub const TABLE_PROFILE_RECEIPT_ALGORITHM: &str = "typaxis.table-profile-receipt/1";

/// Failure returned before PACKAGE admission when a compiled host primitive is
/// unavailable. The emitted `I9110` is fatal and therefore terminates the
/// command-wide diagnostic budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostCapabilityPreflightError {
    WrongDiagnosticPhase,
    DiagnosticBudget(MachineDiagnosticBudgetError),
    Unavailable,
}

impl std::fmt::Display for HostCapabilityPreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongDiagnosticPhase => formatter.write_str("host diagnostic phase is required"),
            Self::DiagnosticBudget(_) => {
                formatter.write_str("host diagnostic budget cannot accept the result")
            }
            Self::Unavailable => formatter.write_str(HOST_UNAVAILABLE_MESSAGE),
        }
    }
}

impl std::error::Error for HostCapabilityPreflightError {}

impl HostCapabilityDescriptor {
    /// Validate contained-open availability before any PACKAGE read using the
    /// same facts used by capability encoding. Atomic-publication availability
    /// is intentionally handled earlier by publication-context construction,
    /// where no sidecar can safely be promised.
    pub fn preflight(
        self,
        _profile: MachinePdfProfileId,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<(), HostCapabilityPreflightError> {
        if diagnostics.phase() != MachineDiagnosticPhase::Host {
            return Err(HostCapabilityPreflightError::WrongDiagnosticPhase);
        }
        if self.contained_package_open() && self.contained_resource_open() {
            return Ok(());
        }
        let error = PublicMachineError::CompiledHostUnavailable;
        let diagnostic = DiagnosticBuilder::global(
            error.code(),
            Severity::Fatal,
            HOST_UNAVAILABLE_MESSAGE,
            GlobalDiagnosticScope::Io,
        )
        .expect("the static host capability message is canonical")
        .build();
        let _ = diagnostics
            .emit(diagnostic)
            .map_err(HostCapabilityPreflightError::DiagnosticBudget)?;
        Err(HostCapabilityPreflightError::Unavailable)
    }
}

/// Why a capability gate did not issue a success receipt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachinePdfPreflightFailure {
    WrongDiagnosticPhase,
    DiagnosticBudget(MachineDiagnosticBudgetError),
    Unsupported {
        violation_count: u64,
        primary_code: DiagnosticCode,
    },
}

impl MachinePdfPreflightFailure {
    pub const fn violation_count(self) -> u64 {
        match self {
            Self::Unsupported {
                violation_count, ..
            } => violation_count,
            Self::WrongDiagnosticPhase | Self::DiagnosticBudget(_) => 0,
        }
    }
}

impl std::fmt::Display for MachinePdfPreflightFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongDiagnosticPhase => {
                formatter.write_str("capability diagnostic phase is required")
            }
            Self::DiagnosticBudget(_) => {
                formatter.write_str("capability diagnostic budget cannot accept the result")
            }
            Self::Unsupported {
                violation_count, ..
            } => write!(
                formatter,
                "machine PDF profile rejected {violation_count} capability violation(s)"
            ),
        }
    }
}

impl std::error::Error for MachinePdfPreflightFailure {}

/// A capability receipt issued only after complete `paragraph-1` inspection.
///
/// The session field is deliberately private and has no accessor. Downstream
/// owners can ask this receipt to verify a package, but cannot reconstruct its
/// opaque admission binding from public fingerprint bytes.
///
/// ```compile_fail
/// use typaxis_machine_profile::MachinePdfPreflightReceipt;
/// fn require_clone<T: Clone>() {}
/// require_clone::<MachinePdfPreflightReceipt>();
/// ```
#[derive(Debug)]
pub struct MachinePdfPreflightReceipt {
    profile: MachinePdfProfileId,
    document: DocumentFingerprint,
    style: StyleFingerprint,
    package_input: MachineInputFingerprint,
    session: MachineInputSessionIdentity,
    profile_receipt_sha256: [u8; 32],
}

impl MachinePdfPreflightReceipt {
    pub const fn profile(&self) -> MachinePdfProfileId {
        self.profile
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn document(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn style(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn machine_input_fingerprint(&self) -> MachineInputFingerprint {
        self.package_input
    }

    pub const fn package_input(&self) -> MachineInputFingerprint {
        self.package_input
    }

    /// Stable digest bound into machine manifests between capability
    /// preflight and layout.
    pub const fn profile_receipt_sha256(&self) -> [u8; 32] {
        self.profile_receipt_sha256
    }

    pub fn verify(
        &self,
        profile: MachinePdfProfileId,
        package: &ValidatedMachinePackage,
    ) -> Result<(), MachinePdfReceiptMismatch> {
        if self.profile != profile {
            return Err(MachinePdfReceiptMismatch::Profile);
        }
        let epoch = package.package().epoch_identity();
        if self.document != epoch.document() {
            return Err(MachinePdfReceiptMismatch::Document);
        }
        if self.style != epoch.style() {
            return Err(MachinePdfReceiptMismatch::Style);
        }
        if self.package_input != package.provenance().fingerprint() {
            return Err(MachinePdfReceiptMismatch::MachineInput);
        }
        if self.session != *package.provenance().session_identity() {
            return Err(MachinePdfReceiptMismatch::Session);
        }
        if self.profile_receipt_sha256 != profile_receipt_fingerprint(profile, package) {
            return Err(MachinePdfReceiptMismatch::ProfileReceipt);
        }
        Ok(())
    }

    pub fn matches(&self, profile: MachinePdfProfileId, package: &ValidatedMachinePackage) -> bool {
        self.verify(profile, package).is_ok()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MachinePdfReceiptMismatch {
    Profile,
    Document,
    Style,
    MachineInput,
    Session,
    ProfileReceipt,
}

impl std::fmt::Display for MachinePdfReceiptMismatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Profile => "machine PDF profile receipt mismatch",
            Self::Document => "machine PDF document fingerprint mismatch",
            Self::Style => "machine PDF style fingerprint mismatch",
            Self::MachineInput => "machine PDF input fingerprint mismatch",
            Self::Session => "machine PDF admission session mismatch",
            Self::ProfileReceipt => "machine PDF profile receipt fingerprint mismatch",
        })
    }
}

impl std::error::Error for MachinePdfReceiptMismatch {}

/// Deterministic, pre-resource/pre-layout gate for a closed machine profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachinePdfPreflight {
    descriptor: MachineProfileDescriptor,
}

impl MachinePdfPreflight {
    pub const BASIC_DOCUMENT_1: Self = Self::new(MachineProfileDescriptor::BASIC_DOCUMENT_1);
    pub const FOOTNOTE_1: Self = Self::new(MachineProfileDescriptor::FOOTNOTE_1);
    pub const PARAGRAPH_1: Self = Self::new(MachineProfileDescriptor::PARAGRAPH_1);
    pub const TABLE_1: Self = Self::new(MachineProfileDescriptor::TABLE_1);

    pub const fn new(descriptor: MachineProfileDescriptor) -> Self {
        Self { descriptor }
    }

    pub const fn descriptor(self) -> MachineProfileDescriptor {
        self.descriptor
    }

    /// Inspect all bounded syntax work even after the command-wide diagnostic
    /// materialization budget starts omitting records.
    pub fn run(
        self,
        package: &ValidatedMachinePackage,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<MachinePdfPreflightReceipt, MachinePdfPreflightFailure> {
        if diagnostics.phase() != MachineDiagnosticPhase::Capability {
            return Err(MachinePdfPreflightFailure::WrongDiagnosticPhase);
        }

        let mut violations = ViolationEmitter {
            package,
            diagnostics,
            count: 0,
            first_code: None,
        };
        self.inspect_source_closure(package, &mut violations)?;
        if self.descriptor.id() == MachinePdfProfileId::TABLE_1 {
            self.inspect_table_domain(package, &mut violations)?;
        }
        if self.descriptor.id() == MachinePdfProfileId::FOOTNOTE_1 {
            self.inspect_footnote_domain(package, &mut violations)?;
        }
        let first_text_node = self.inspect_document(package, &mut violations)?;
        self.inspect_styles(package, &mut violations)?;
        self.inspect_page_masters(package, &mut violations)?;
        self.inspect_resources(package, first_text_node, &mut violations)?;

        if violations.count != 0 {
            return Err(MachinePdfPreflightFailure::Unsupported {
                violation_count: violations.count,
                primary_code: violations
                    .first_code
                    .expect("a counted violation records its primary code"),
            });
        }

        let epoch = package.package().epoch_identity();
        Ok(MachinePdfPreflightReceipt {
            profile: self.descriptor.id(),
            document: epoch.document(),
            style: epoch.style(),
            package_input: package.provenance().fingerprint(),
            session: package.provenance().session_identity().clone(),
            profile_receipt_sha256: profile_receipt_fingerprint(self.descriptor.id(), package),
        })
    }

    pub fn preflight(
        self,
        package: &ValidatedMachinePackage,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<MachinePdfPreflightReceipt, MachinePdfPreflightFailure> {
        self.run(package, diagnostics)
    }

    fn inspect_source_closure(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let parsed = package.package();
        let source_count = parsed.package().sources.records().len();
        if !self.descriptor.source_count().permits(source_count)
            || !parsed.include_graph().edges().is_empty()
        {
            violations.content(parsed.package().document.node_id, None)?;
        }
        Ok(())
    }

    fn inspect_document(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<Option<NodeId>, MachinePdfPreflightFailure> {
        let document = &package.package().package().document;
        let mut stack = Vec::new();
        stack.extend(document.footnotes.iter().rev().map(WorkItem::Footnote));
        stack.extend(document.blocks.iter().rev().map(WorkItem::Block));
        let mut previous_node_id = document.node_id;
        let mut first_text_node = None;

        while let Some(item) = stack.pop() {
            let node_id = item.node_id();
            debug_assert!(previous_node_id < node_id);
            previous_node_id = node_id;
            match item {
                WorkItem::Block(block) => {
                    self.inspect_block(block, violations)?;
                    push_block_children(&mut stack, block);
                }
                WorkItem::Inline(inline) => {
                    if first_text_node.is_none() && inline_is_text_producing(inline) {
                        first_text_node = Some(node_id);
                    }
                    self.inspect_inline(inline, violations)?;
                    push_inline_children(&mut stack, inline);
                }
                WorkItem::Footnote(footnote) => {
                    if !self.descriptor.footnotes().definitions() {
                        violations.content(footnote.node_id, None)?;
                    }
                    stack.extend(footnote.blocks.iter().rev().map(WorkItem::Block));
                }
            }
        }
        Ok(first_text_node)
    }

    fn inspect_table_domain(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let blocks = &package.package().package().document.blocks;
        for block in blocks {
            if let Block::Table { head, body, .. } = block {
                for row in head.iter().chain(body) {
                    for cell in &row.cells {
                        for block in &cell.blocks {
                            match block {
                                Block::Paragraph { children, .. } => {
                                    inspect_table_cell_inlines(children, violations)?;
                                }
                                _ => violations.content(block_node_id(block), None)?,
                            }
                            reject_nested_tables(block, violations)?;
                        }
                    }
                }
            } else {
                reject_nested_tables(block, violations)?;
            }
        }
        Ok(())
    }

    fn inspect_footnote_domain(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let document = &package.package().package().document;
        let mut referenced = std::collections::BTreeSet::new();
        for block in &document.blocks {
            collect_footnote_body_targets(block, &mut referenced);
        }
        for definition in &document.footnotes {
            let mut text_producing = false;
            let definition_invalid =
                definition.blocks.is_empty() || !referenced.contains(&definition.footnote_id);
            for block in &definition.blocks {
                let children = match block {
                    Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
                        children.as_slice()
                    }
                    _ => {
                        violations.content(block_node_id(block), None)?;
                        continue;
                    }
                };
                inspect_footnote_definition_inlines(
                    children,
                    false,
                    &mut text_producing,
                    violations,
                )?;
            }
            if definition_invalid || !text_producing {
                violations.content(definition.node_id, None)?;
            }
        }
        Ok(())
    }

    fn inspect_block(
        self,
        block: &Block,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let kind = block_kind(block);
        if !self.descriptor.accepts_block(kind) {
            violations.content(block_node_id(block), None)?;
        }
        if self.descriptor.accepted_image_formats().is_empty() {
            if let Block::Figure {
                node_id, image_id, ..
            } = block
            {
                violations.resource_at_node(*node_id, ResourceErrorSubject::Image(*image_id))?;
            }
        }
        Ok(())
    }

    fn inspect_inline(
        self,
        inline: &Inline,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let kind = inline_kind(inline);
        let node_id = inline_node_id(inline);
        let text_span = match inline {
            Inline::Text { text_span, .. } => Some(*text_span),
            _ => None,
        };
        let accepted = self.descriptor.accepts_inline(kind)
            && (!matches!(inline, Inline::FootnoteReference { .. })
                || self.descriptor.footnotes().references());
        if !accepted {
            violations.content(node_id, text_span)?;
        }
        if let Inline::Reference { format, .. } = inline {
            let format = reference_format(*format);
            if !self.descriptor.accepts_reference_format(format) {
                violations.content(node_id, None)?;
            }
        }
        Ok(())
    }

    fn inspect_styles(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let parsed = package.package().package();
        for rule in &parsed.style_sheet.rules {
            if !self.descriptor.accepts_style_selector(&rule.selector) {
                violations.style(rule, None, None)?;
            }
            for (declaration_ordinal, declaration) in rule.declarations.iter().enumerate() {
                let property = StylePropertyName::new(declaration.name.clone());
                if !self.descriptor.accepts_style_property(&declaration.name) {
                    violations.style(rule, property, Some(declaration_ordinal))?;
                    continue;
                }
                if declaration.name == "page" {
                    let page_value = match &declaration.value {
                        StyleValue::Keyword(value) if value == "auto" => MachinePageValue::Auto,
                        _ => MachinePageValue::Named,
                    };
                    if !self.descriptor.accepts_page_value(page_value) {
                        violations.style(rule, property, Some(declaration_ordinal))?;
                    }
                }
            }
        }
        Ok(())
    }

    fn inspect_page_masters(
        self,
        package: &ValidatedMachinePackage,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let page_masters = &package.package().package().page_masters;
        let expected_count = self.descriptor.page_master().count() as usize;
        let mut rules: Vec<&PageMasterRule> = if self.descriptor.page_master().selection_rules() {
            Vec::new()
        } else {
            page_masters.selection_rules.iter().collect()
        };
        rules.sort_by(|left, right| {
            left.master_id
                .cmp(&right.master_id)
                .then(left.source_order.cmp(&right.source_order))
        });
        let mut next_rule = 0;
        for master in &page_masters.masters {
            if page_masters.masters.len() != expected_count {
                violations.master(master)?;
            }
            for frame in self.descriptor.page_master().rejected_optional_frames() {
                if master_has_frame(master, *frame) {
                    violations.master(master)?;
                }
            }
            if self.descriptor.id() == MachinePdfProfileId::FOOTNOTE_1
                && !valid_footnote_master_geometry(master)
            {
                violations.master(master)?;
            }
            while next_rule < rules.len() && rules[next_rule].master_id == master.master_id {
                violations.master_rule(rules[next_rule])?;
                next_rule += 1;
            }
        }
        debug_assert_eq!(next_rule, rules.len());
        Ok(())
    }

    fn inspect_resources(
        self,
        package: &ValidatedMachinePackage,
        first_text_node: Option<NodeId>,
        violations: &mut ViolationEmitter<'_, '_, '_>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let resources = &package.package().package().resources;
        if let Some(node_id) = first_text_node {
            if resources.font_faces.len() < self.descriptor.minimum_fonts_for_text() as usize {
                violations.missing_text_font(node_id)?;
            }
        }
        if self.descriptor.accepted_image_formats().is_empty() {
            for image in &resources.images {
                violations.image(image.image_id)?;
            }
        }
        Ok(())
    }
}

fn profile_receipt_fingerprint(
    profile: MachinePdfProfileId,
    package: &ValidatedMachinePackage,
) -> [u8; 32] {
    let epoch = package.package().epoch_identity();
    let mut bytes = Vec::with_capacity(192);
    let algorithm = match profile {
        MachinePdfProfileId::Footnote1 => FOOTNOTE_PROFILE_RECEIPT_ALGORITHM,
        MachinePdfProfileId::Table1 => TABLE_PROFILE_RECEIPT_ALGORITHM,
        MachinePdfProfileId::BasicDocument1 | MachinePdfProfileId::Paragraph1 => {
            BASIC_PROFILE_RECEIPT_ALGORITHM
        }
    };
    bytes.extend_from_slice(algorithm.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(package.contract().as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(profile.as_str().as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&epoch.document().bytes());
    bytes.extend_from_slice(&epoch.style().bytes());
    bytes.extend_from_slice(&package.provenance().fingerprint().bytes());
    sha256(&bytes)
}

enum WorkItem<'a> {
    Block(&'a Block),
    Inline(&'a Inline),
    Footnote(&'a FootnoteDefinition),
}

impl WorkItem<'_> {
    fn node_id(&self) -> NodeId {
        match self {
            Self::Block(block) => block_node_id(block),
            Self::Inline(inline) => inline_node_id(inline),
            Self::Footnote(footnote) => footnote.node_id,
        }
    }
}

fn push_block_children<'a>(stack: &mut Vec<WorkItem<'a>>, block: &'a Block) {
    match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            stack.extend(children.iter().rev().map(WorkItem::Inline));
        }
        Block::List { items, .. } => {
            for item in items.iter().rev() {
                stack.extend(item.blocks.iter().rev().map(WorkItem::Block));
            }
        }
        Block::Table { head, body, .. } => {
            for row in body.iter().rev().chain(head.iter().rev()) {
                for cell in row.cells.iter().rev() {
                    stack.extend(cell.blocks.iter().rev().map(WorkItem::Block));
                }
            }
        }
        Block::Figure { caption, .. } => {
            stack.extend(caption.iter().rev().map(WorkItem::Block));
        }
        Block::PageBreak { .. } => {}
    }
}

fn push_inline_children<'a>(stack: &mut Vec<WorkItem<'a>>, inline: &'a Inline) {
    match inline {
        Inline::Emphasis { children, .. }
        | Inline::Strong { children, .. }
        | Inline::Link { children, .. } => {
            stack.extend(children.iter().rev().map(WorkItem::Inline));
        }
        Inline::Text { .. }
        | Inline::Anchor { .. }
        | Inline::Reference { .. }
        | Inline::FootnoteReference { .. }
        | Inline::SoftBreak { .. }
        | Inline::HardBreak { .. } => {}
    }
}

fn inspect_table_cell_inlines(
    inlines: &[Inline],
    violations: &mut ViolationEmitter<'_, '_, '_>,
) -> Result<(), MachinePdfPreflightFailure> {
    for inline in inlines {
        match inline {
            Inline::Text { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
            Inline::Emphasis {
                node_id, children, ..
            }
            | Inline::Strong {
                node_id, children, ..
            }
            | Inline::Link {
                node_id, children, ..
            } => {
                violations.content(*node_id, None)?;
                inspect_table_cell_inlines(children, violations)?;
            }
            Inline::Anchor { node_id, .. }
            | Inline::Reference { node_id, .. }
            | Inline::FootnoteReference { node_id, .. } => {
                violations.content(*node_id, None)?;
            }
        }
    }
    Ok(())
}

fn collect_footnote_body_targets(
    block: &Block,
    targets: &mut std::collections::BTreeSet<typaxis_core::FootnoteId>,
) {
    match block {
        Block::Paragraph { children, .. } | Block::Heading { children, .. } => {
            collect_footnote_inline_targets(children, targets)
        }
        Block::List { items, .. } => {
            for child in items.iter().flat_map(|item| &item.blocks) {
                collect_footnote_body_targets(child, targets);
            }
        }
        Block::Table { head, body, .. } => {
            for child in head
                .iter()
                .chain(body)
                .flat_map(|row| &row.cells)
                .flat_map(|cell| &cell.blocks)
            {
                collect_footnote_body_targets(child, targets);
            }
        }
        Block::Figure { caption, .. } => {
            for child in caption {
                collect_footnote_body_targets(child, targets);
            }
        }
        Block::PageBreak { .. } => {}
    }
}

fn collect_footnote_inline_targets(
    inlines: &[Inline],
    targets: &mut std::collections::BTreeSet<typaxis_core::FootnoteId>,
) {
    for inline in inlines {
        match inline {
            Inline::FootnoteReference { footnote_id, .. } => {
                targets.insert(footnote_id.clone());
            }
            Inline::Emphasis { children, .. }
            | Inline::Strong { children, .. }
            | Inline::Link { children, .. } => collect_footnote_inline_targets(children, targets),
            Inline::Text { .. }
            | Inline::Anchor { .. }
            | Inline::Reference { .. }
            | Inline::SoftBreak { .. }
            | Inline::HardBreak { .. } => {}
        }
    }
}

fn inspect_footnote_definition_inlines(
    inlines: &[Inline],
    inside_link: bool,
    text_producing: &mut bool,
    violations: &mut ViolationEmitter<'_, '_, '_>,
) -> Result<(), MachinePdfPreflightFailure> {
    for inline in inlines {
        match inline {
            Inline::Text {
                node_id, text_span, ..
            } => {
                if text_span.start_byte() < text_span.end_byte() {
                    *text_producing = true;
                } else {
                    let _ = node_id;
                }
            }
            Inline::Reference {
                format: ReferenceFormat::Page,
                ..
            } => *text_producing = true,
            Inline::Link {
                node_id, children, ..
            } if !inside_link => {
                let mut link_text = false;
                inspect_footnote_definition_inlines(children, true, &mut link_text, violations)?;
                if link_text {
                    *text_producing = true;
                } else {
                    violations.content(*node_id, None)?;
                }
            }
            Inline::Anchor { .. } | Inline::SoftBreak { .. } | Inline::HardBreak { .. } => {}
            Inline::Emphasis { children, .. } | Inline::Strong { children, .. } => {
                // The profile descriptor emits the canonical rejection for
                // these globally unsupported inline kinds. Continue only to
                // avoid a redundant definition-level "empty" diagnostic.
                inspect_footnote_definition_inlines(
                    children,
                    inside_link,
                    text_producing,
                    violations,
                )?;
            }
            Inline::Link {
                node_id, children, ..
            } => {
                // Link is otherwise part of the M2 inline set, so nesting is
                // the footnote-context violation owned by this inspection.
                violations.content(*node_id, None)?;
                inspect_footnote_definition_inlines(children, true, text_producing, violations)?;
            }
            Inline::Reference { .. } => {
                // The descriptor emits the canonical format rejection.
            }
            Inline::FootnoteReference { node_id, .. } => {
                violations.content(*node_id, None)?;
            }
        }
    }
    Ok(())
}

fn valid_footnote_master_geometry(master: &PageMaster) -> bool {
    let Some(footnote) = master.footnote else {
        return false;
    };
    let Some(body_end) = master.body.y().checked_add(master.body.height().get()) else {
        return false;
    };
    let Some(footnote_end) = footnote.y().checked_add(footnote.height().get()) else {
        return false;
    };
    footnote.x() == master.body.x()
        && footnote.width() == master.body.width()
        && footnote_end == body_end
        && footnote.height().get().raw() < master.body.height().get().raw()
}

fn reject_nested_tables(
    block: &Block,
    violations: &mut ViolationEmitter<'_, '_, '_>,
) -> Result<(), MachinePdfPreflightFailure> {
    match block {
        Block::Table {
            node_id,
            head,
            body,
            ..
        } => {
            violations.content(*node_id, None)?;
            for row in head.iter().chain(body) {
                for cell in &row.cells {
                    for child in &cell.blocks {
                        reject_nested_tables(child, violations)?;
                    }
                }
            }
        }
        Block::List { items, .. } => {
            for child in items.iter().flat_map(|item| &item.blocks) {
                reject_nested_tables(child, violations)?;
            }
        }
        Block::Figure { caption, .. } => {
            for child in caption {
                reject_nested_tables(child, violations)?;
            }
        }
        Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
    }
    Ok(())
}

fn block_kind(block: &Block) -> MachineBlockKind {
    match block {
        Block::Paragraph { .. } => MachineBlockKind::Paragraph,
        Block::Heading { .. } => MachineBlockKind::Heading,
        Block::List { .. } => MachineBlockKind::List,
        Block::Table { .. } => MachineBlockKind::Table,
        Block::Figure { .. } => MachineBlockKind::Figure,
        Block::PageBreak { .. } => MachineBlockKind::PageBreak,
    }
}

fn block_node_id(block: &Block) -> NodeId {
    match block {
        Block::Paragraph { node_id, .. }
        | Block::Heading { node_id, .. }
        | Block::List { node_id, .. }
        | Block::Table { node_id, .. }
        | Block::Figure { node_id, .. }
        | Block::PageBreak { node_id, .. } => *node_id,
    }
}

fn inline_kind(inline: &Inline) -> MachineInlineKind {
    match inline {
        Inline::Text { .. } => MachineInlineKind::Text,
        Inline::Emphasis { .. } => MachineInlineKind::Emphasis,
        Inline::Strong { .. } => MachineInlineKind::Strong,
        Inline::Link { .. } => MachineInlineKind::Link,
        Inline::Anchor { .. } => MachineInlineKind::Anchor,
        Inline::Reference { .. } => MachineInlineKind::Reference,
        Inline::FootnoteReference { .. } => MachineInlineKind::FootnoteReference,
        Inline::SoftBreak { .. } => MachineInlineKind::SoftBreak,
        Inline::HardBreak { .. } => MachineInlineKind::HardBreak,
    }
}

fn inline_node_id(inline: &Inline) -> NodeId {
    match inline {
        Inline::Text { node_id, .. }
        | Inline::Emphasis { node_id, .. }
        | Inline::Strong { node_id, .. }
        | Inline::Link { node_id, .. }
        | Inline::Anchor { node_id, .. }
        | Inline::Reference { node_id, .. }
        | Inline::FootnoteReference { node_id, .. }
        | Inline::SoftBreak { node_id, .. }
        | Inline::HardBreak { node_id, .. } => *node_id,
    }
}

fn inline_is_text_producing(inline: &Inline) -> bool {
    matches!(
        inline,
        Inline::Text { .. }
            | Inline::Reference {
                format: ReferenceFormat::Page,
                ..
            }
    )
}

fn reference_format(format: ReferenceFormat) -> MachineReferenceFormat {
    match format {
        ReferenceFormat::Text => MachineReferenceFormat::Text,
        ReferenceFormat::Page => MachineReferenceFormat::Page,
        ReferenceFormat::Number => MachineReferenceFormat::Number,
    }
}

fn master_has_frame(master: &PageMaster, frame: MachinePageFrame) -> bool {
    match frame {
        MachinePageFrame::Footer => master.footer.is_some(),
        MachinePageFrame::Footnote => master.footnote.is_some(),
        MachinePageFrame::Header => master.header.is_some(),
    }
}

struct ViolationEmitter<'package, 'loan, 'budget> {
    package: &'package ValidatedMachinePackage,
    diagnostics: &'loan mut MachineDiagnosticLender<'budget>,
    count: u64,
    first_code: Option<DiagnosticCode>,
}

impl ViolationEmitter<'_, '_, '_> {
    fn content(
        &mut self,
        node_id: NodeId,
        text_span: Option<typaxis_core::TextSpan>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        self.emit(L5100, move || {
            let subject = LayoutErrorSubject::new(node_id, text_span);
            let error = PublicMachineError::UnsupportedContent(subject);
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_CONTENT_MESSAGE,
                node_location(package, node_id),
            )
            .expect("the static unsupported-content message is canonical")
            .subject(error.subject().expect("unsupported content has a subject"))
            .build()
        })
    }

    fn style(
        &mut self,
        rule: &StyleRule,
        property: Option<StylePropertyName>,
        declaration_ordinal: Option<usize>,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        let document_node = package.package().package().document.node_id;
        let style_id = rule.style_id.clone();
        let source_order = rule.source_order;
        self.emit(L5101, move || {
            let pointer = declaration_ordinal
                .and_then(|ordinal| {
                    package.provenance().locations().style_declaration(
                        style_id.as_str(),
                        0,
                        ordinal,
                    )
                })
                .or_else(|| {
                    package
                        .provenance()
                        .locations()
                        .style_rule_by_source_order(source_order, 0)
                })
                .unwrap_or_else(|| JsonPointer::root().child("style_sheet"));
            let subject = StyleErrorSubject::new(document_node, Some(style_id), property);
            let error = PublicMachineError::UnsupportedStyle(subject);
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_STYLE_MESSAGE,
                package_location(package, pointer),
            )
            .expect("the static unsupported-style message is canonical")
            .subject(error.subject().expect("unsupported style has a subject"))
            .build()
        })
    }

    fn master(&mut self, master: &PageMaster) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        let master_id = master.master_id.clone();
        self.emit(L5101, move || {
            let pointer = package
                .provenance()
                .locations()
                .page_master(master_id.as_str(), 0)
                .unwrap_or_else(|| JsonPointer::root().child("page_masters"));
            let subject = MasterErrorSubject::new(master_id, None);
            let error = PublicMachineError::UnsupportedMaster(subject);
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_MASTER_MESSAGE,
                package_location(package, pointer),
            )
            .expect("the static unsupported-master message is canonical")
            .subject(error.subject().expect("unsupported master has a subject"))
            .build()
        })
    }

    fn master_rule(&mut self, rule: &PageMasterRule) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        let master_id = rule.master_id.clone();
        let source_order = rule.source_order;
        self.emit(L5101, move || {
            let pointer = package
                .provenance()
                .locations()
                .page_master_rule_by_source_order(source_order, 0)
                .unwrap_or_else(|| {
                    JsonPointer::root()
                        .child("page_masters")
                        .child("selection_rules")
                });
            let subject = MasterErrorSubject::new(master_id, Some(source_order));
            let error = PublicMachineError::UnsupportedMaster(subject);
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_MASTER_MESSAGE,
                package_location(package, pointer),
            )
            .expect("the static unsupported-master message is canonical")
            .subject(error.subject().expect("unsupported master has a subject"))
            .build()
        })
    }

    fn resource_at_node(
        &mut self,
        node_id: NodeId,
        subject: ResourceErrorSubject,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        self.emit(R7100, move || {
            let error = PublicMachineError::UnsupportedResource(subject);
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_RESOURCE_MESSAGE,
                node_location(package, node_id),
            )
            .expect("the static unsupported-resource message is canonical")
            .subject(error.subject().expect("unsupported resource has a subject"))
            .build()
        })
    }

    fn image(
        &mut self,
        image_id: typaxis_core::ImageResourceId,
    ) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        self.emit(R7100, move || {
            let pointer = package
                .provenance()
                .locations()
                .image(image_id.get(), 0)
                .unwrap_or_else(|| JsonPointer::root().child("resources").child("images"));
            let error =
                PublicMachineError::UnsupportedResource(ResourceErrorSubject::Image(image_id));
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                UNSUPPORTED_RESOURCE_MESSAGE,
                package_location(package, pointer),
            )
            .expect("the static unsupported-resource message is canonical")
            .subject(error.subject().expect("unsupported resource has a subject"))
            .build()
        })
    }

    fn missing_text_font(&mut self, node_id: NodeId) -> Result<(), MachinePdfPreflightFailure> {
        let package = self.package;
        self.emit(R7100, move || {
            let pointer = JsonPointer::root().child("resources").child("font_faces");
            let error = PublicMachineError::UnsupportedResource(ResourceErrorSubject::FontFace(
                FontFaceId::new(0),
            ));
            DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                MISSING_TEXT_FONT_MESSAGE,
                package_location(package, pointer),
            )
            .expect("the static missing-font message is canonical")
            .subject(
                error
                    .subject()
                    .expect("missing font has a resource subject"),
            )
            .located_note("first text-producing site", node_location(package, node_id))
            .expect("the static text-site note is canonical")
            .build()
        })
    }

    fn emit(
        &mut self,
        code: DiagnosticCode,
        build: impl FnOnce() -> Diagnostic,
    ) -> Result<(), MachinePdfPreflightFailure> {
        self.count = self.count.saturating_add(1);
        self.first_code.get_or_insert(code);
        self.diagnostics
            .emit_error_with(build)
            .map(|_| ())
            .map_err(MachinePdfPreflightFailure::DiagnosticBudget)
    }
}

fn node_location(package: &ValidatedMachinePackage, node_id: NodeId) -> DiagnosticLocation {
    let pointer = package
        .provenance()
        .locations()
        .node(node_id.get(), 0)
        .unwrap_or_else(|| JsonPointer::root().child("document"));
    package_location(package, pointer)
}

fn package_location(package: &ValidatedMachinePackage, pointer: JsonPointer) -> DiagnosticLocation {
    DiagnosticLocation::package_json(package_uri(package).clone(), pointer, None)
}

fn package_uri(package: &ValidatedMachinePackage) -> &PortablePath {
    package
        .provenance()
        .progress()
        .package()
        .expect("validated machine packages retain admitted PACKAGE facts")
        .uri()
}
