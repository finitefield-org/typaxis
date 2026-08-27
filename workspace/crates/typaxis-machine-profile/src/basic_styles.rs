use typaxis_core::{DocumentFingerprint, JsonPointer, NodeId, StyleFingerprint};
use typaxis_diagnostics::{
    DiagnosticBuilder, DiagnosticCode, DiagnosticLocation, MachineDiagnosticBudgetError,
    MachineDiagnosticLender, MachineDiagnosticPhase, PublicMachineError, Severity,
    StyleErrorSubject, StylePropertyName, L5101,
};
use typaxis_syntax::machine_profile_boundary::{
    BasicBlockStylePropertyDescriptor, BasicStyleBlockKind, BasicStyleProperty, Block,
    BASIC_BLOCK_STYLE_PROPERTIES, BASIC_BLOCK_STYLE_REGISTRY_VERSION,
};
use typaxis_syntax::ValidatedStagingStylePackage;

pub const BASIC_DOCUMENT_PROFILE_ID: &str = "typaxis.machine-pdf/basic-document-1";
pub const FOOTNOTE_PROFILE_ID: &str = "typaxis.machine-pdf/footnote-1";
pub const TABLE_PROFILE_ID: &str = "typaxis.machine-pdf/table-1";

/// Closed style component of the public basic-document descriptor. The
/// historical `STAGING` constant remains for focused MI2 slice tests only.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentStyleDescriptor {
    table: bool,
    footnote: bool,
}

impl BasicDocumentStyleDescriptor {
    pub const STAGING: Self = Self {
        table: false,
        footnote: false,
    };
    pub const FOOTNOTE_1: Self = Self {
        table: false,
        footnote: true,
    };
    pub const TABLE_1: Self = Self {
        table: true,
        footnote: false,
    };

    pub const fn profile_id(self) -> &'static str {
        if self.table {
            TABLE_PROFILE_ID
        } else if self.footnote {
            FOOTNOTE_PROFILE_ID
        } else {
            BASIC_DOCUMENT_PROFILE_ID
        }
    }

    pub const fn registry_version(self) -> &'static str {
        BASIC_BLOCK_STYLE_REGISTRY_VERSION
    }

    pub const fn additive_properties(self) -> &'static [BasicBlockStylePropertyDescriptor] {
        BASIC_BLOCK_STYLE_PROPERTIES
    }

    pub const fn accepts(self, block: BasicStyleBlockKind, property: BasicStyleProperty) -> bool {
        property.applies_to(block)
    }

    pub const fn accepts_table(self, property: BasicStyleProperty) -> bool {
        self.table && property.applies_to_table()
    }
}

#[derive(Debug)]
struct BasicDocumentStyleBinding;

/// Sealed receipt proving that every style declaration passed the staging
/// descriptor before any layout consumer was invoked.
#[derive(Debug)]
pub struct BasicDocumentStylePreflightReceipt {
    package: [u8; 32],
    document: DocumentFingerprint,
    style: StyleFingerprint,
    profile_id: &'static str,
    registry_version: &'static str,
    _binding: BasicDocumentStyleBinding,
}

impl BasicDocumentStylePreflightReceipt {
    pub const fn package_fingerprint(&self) -> [u8; 32] {
        self.package
    }

    pub const fn document_fingerprint(&self) -> DocumentFingerprint {
        self.document
    }

    pub const fn style_fingerprint(&self) -> StyleFingerprint {
        self.style
    }

    pub const fn profile_id(&self) -> &'static str {
        self.profile_id
    }

    pub const fn registry_version(&self) -> &'static str {
        self.registry_version
    }

    pub fn verifies(&self, package: &ValidatedStagingStylePackage) -> bool {
        self.verifies_for(package, BasicDocumentStyleDescriptor::STAGING)
    }

    pub fn verifies_for(
        &self,
        package: &ValidatedStagingStylePackage,
        descriptor: BasicDocumentStyleDescriptor,
    ) -> bool {
        self.package == package.package_fingerprint().into_bytes()
            && self.document == package.package().epoch_identity().document()
            && self.style == package.package().epoch_identity().style()
            && self.profile_id == descriptor.profile_id()
            && self.registry_version == BASIC_BLOCK_STYLE_REGISTRY_VERSION
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BasicDocumentStylePreflightFailure {
    WrongDiagnosticPhase,
    DiagnosticBudget(MachineDiagnosticBudgetError),
    Unsupported {
        violation_count: u64,
        primary_code: DiagnosticCode,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BasicDocumentStylePreflight {
    descriptor: BasicDocumentStyleDescriptor,
}

impl BasicDocumentStylePreflight {
    pub const STAGING: Self = Self {
        descriptor: BasicDocumentStyleDescriptor::STAGING,
    };
    pub const TABLE_1: Self = Self {
        descriptor: BasicDocumentStyleDescriptor::TABLE_1,
    };
    pub const FOOTNOTE_1: Self = Self {
        descriptor: BasicDocumentStyleDescriptor::FOOTNOTE_1,
    };

    pub fn run(
        self,
        package: &ValidatedStagingStylePackage,
        diagnostics: &mut MachineDiagnosticLender<'_>,
    ) -> Result<BasicDocumentStylePreflightReceipt, BasicDocumentStylePreflightFailure> {
        if diagnostics.phase() != MachineDiagnosticPhase::Capability {
            return Err(BasicDocumentStylePreflightFailure::WrongDiagnosticPhase);
        }
        let parsed = package.package().package();
        let mut violation_count = 0u64;
        let mut primary_code = None;
        for rule in &parsed.style_sheet.rules {
            let selector = rule.selector.split('.').next().unwrap_or_default();
            let block = BasicStyleBlockKind::from_str(selector);
            for (ordinal, declaration) in rule.declarations.iter().enumerate() {
                let property = BasicStyleProperty::from_str(&declaration.name);
                let accepted = property.is_some_and(|property| {
                    if selector == "table" {
                        self.descriptor.accepts_table(property)
                    } else {
                        block.is_some_and(|block| self.descriptor.accepts(block, property))
                    }
                });
                if accepted {
                    continue;
                }
                violation_count = violation_count.saturating_add(1);
                primary_code.get_or_insert(L5101);
                let pointer = package
                    .locations()
                    .style_declaration(rule.style_id.as_str(), 0, ordinal)
                    .unwrap_or_else(|| JsonPointer::root().child("style_sheet"));
                let property = StylePropertyName::new(declaration.name.clone());
                let subject = StyleErrorSubject::new(
                    parsed.document.node_id,
                    Some(rule.style_id.clone()),
                    property,
                );
                let error = PublicMachineError::UnsupportedStyle(subject);
                let uri = parsed
                    .sources
                    .records()
                    .first()
                    .expect("staging syntax enforces one source")
                    .uri()
                    .clone();
                let diagnostic = DiagnosticBuilder::located(
                    error.code(),
                    Severity::Error,
                    "style is not supported by the selected machine PDF profile",
                    DiagnosticLocation::package_json(uri, pointer, None),
                )
                .expect("the static staging style diagnostic is canonical")
                .subject(error.subject().expect("unsupported style has a subject"))
                .build();
                let _ = diagnostics
                    .emit_error_with(|| diagnostic)
                    .map_err(BasicDocumentStylePreflightFailure::DiagnosticBudget)?;
            }
        }
        if violation_count != 0 {
            return Err(BasicDocumentStylePreflightFailure::Unsupported {
                violation_count,
                primary_code: primary_code.expect("a violation records its code"),
            });
        }
        for node_id in figure_node_ids(&parsed.document.blocks) {
            let has_width = package
                .figure_has_required_width(node_id)
                .is_ok_and(|has_width| has_width);
            if has_width {
                continue;
            }
            violation_count = violation_count.saturating_add(1);
            primary_code.get_or_insert(L5101);
            let pointer = package
                .locations()
                .node(node_id.get(), 0)
                .unwrap_or_else(|| JsonPointer::root().child("document"));
            let subject = StyleErrorSubject::new(
                node_id,
                None,
                StylePropertyName::new(BasicStyleProperty::Width.as_str().to_owned()),
            );
            let error = PublicMachineError::UnsupportedStyle(subject);
            let uri = parsed
                .sources
                .records()
                .first()
                .expect("staging syntax enforces one source")
                .uri()
                .clone();
            let diagnostic = DiagnosticBuilder::located(
                error.code(),
                Severity::Error,
                "a figure requires a positive computed width in the selected machine PDF profile",
                DiagnosticLocation::package_json(uri, pointer, None),
            )
            .expect("the static figure-width diagnostic is canonical")
            .subject(error.subject().expect("unsupported style has a subject"))
            .build();
            let _ = diagnostics
                .emit_error_with(|| diagnostic)
                .map_err(BasicDocumentStylePreflightFailure::DiagnosticBudget)?;
        }
        if violation_count != 0 {
            return Err(BasicDocumentStylePreflightFailure::Unsupported {
                violation_count,
                primary_code: primary_code.expect("a violation records its code"),
            });
        }
        let epoch = package.package().epoch_identity();
        Ok(BasicDocumentStylePreflightReceipt {
            package: package.package_fingerprint().into_bytes(),
            document: epoch.document(),
            style: epoch.style(),
            profile_id: self.descriptor.profile_id(),
            registry_version: self.descriptor.registry_version(),
            _binding: BasicDocumentStyleBinding,
        })
    }
}

fn figure_node_ids(blocks: &[Block]) -> Vec<NodeId> {
    let mut figures = Vec::new();
    let mut pending: Vec<&Block> = blocks.iter().rev().collect();
    while let Some(block) = pending.pop() {
        match block {
            Block::Figure {
                node_id, caption, ..
            } => {
                figures.push(*node_id);
                pending.extend(caption.iter().rev());
            }
            Block::List { items, .. } => {
                pending.extend(items.iter().rev().flat_map(|item| item.blocks.iter().rev()));
            }
            Block::Table { head, body, .. } => pending.extend(
                body.iter()
                    .rev()
                    .chain(head.iter().rev())
                    .flat_map(|row| row.cells.iter().rev())
                    .flat_map(|cell| cell.blocks.iter().rev()),
            ),
            Block::Paragraph { .. } | Block::Heading { .. } | Block::PageBreak { .. } => {}
        }
    }
    figures
}
