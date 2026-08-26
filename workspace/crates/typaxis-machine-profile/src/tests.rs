use super::*;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use typaxis_core::{sha256, DocumentPackageContractId, ResourceLimits, ValidatedResourceLimits};
use typaxis_diagnostics::{
    DiagnosticSubject, MachineDiagnosticBudget, MachineDiagnosticPhase, Severity, I9110, L5100,
    L5101, MAX_MACHINE_DIAGNOSTICS, R7100,
};
use typaxis_syntax::machine_profile_boundary::{
    wire, AtomicFilePublicationCapabilityToken, HostMachineInputSession,
    MachineInputCapabilityToken, MachineInputHostOptions, ResourceAdmissionCapabilityToken,
};
use typaxis_syntax::{
    DocumentPackageParser, MachineParseOutcome, PackageValidationPolicy, ValidatedMachinePackage,
};

static NEXT_FIXTURE_ROOT: AtomicU64 = AtomicU64::new(0);

struct FixtureRoot(PathBuf);

impl FixtureRoot {
    fn new(label: &str) -> Self {
        let ordinal = NEXT_FIXTURE_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "typaxis-machine-profile-{label}-{}-{ordinal}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("fixture root must be creatable");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for FixtureRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn source_span() -> wire::WireSourceSpan {
    wire::WireSourceSpan {
        source_id: 0,
        start_byte: 0,
        end_byte: 0,
    }
}

fn default_master(id: &str) -> wire::WirePageMaster {
    wire::WirePageMaster {
        master_id: id.to_owned(),
        width: 100,
        height: 100,
        body: wire::WireRect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        },
        header: None,
        footer: None,
        footnote: None,
    }
}

fn base_wire() -> wire::WireDocumentPackage {
    wire::WireDocumentPackage {
        contract: DocumentPackageContractId::V1_0,
        coordinate_unit: wire::WireCoordinateUnit::PdfPoint1_65536,
        sources: vec![wire::WireSource {
            source_id: 0,
            uri: "input.tsf".to_owned(),
            utf8_byte_length: 0,
            sha256: sha256(&[]),
        }],
        text_buffers: Vec::new(),
        document: wire::WireDocument {
            node_id: 0,
            blocks: Vec::new(),
            footnotes: Vec::new(),
        },
        style_sheet: wire::WireStyleSheet { rules: Vec::new() },
        page_masters: wire::WirePageMasterSet {
            default_master_id: "default".to_owned(),
            masters: vec![default_master("default")],
            selection_rules: Vec::new(),
        },
        resources: wire::WireResourceCatalog {
            font_faces: Vec::new(),
            images: Vec::new(),
        },
    }
}

fn add_text_buffer(package: &mut wire::WireDocumentPackage) {
    package.text_buffers = vec![wire::WireTextBuffer {
        text_id: 0,
        utf8: "x".to_owned(),
        mappings: vec![wire::WireTextMapSegment {
            text_range: wire::WireByteRange {
                start_byte: 0,
                end_byte: 1,
            },
            kind: wire::WireTextMapKind::Inserted,
            source_span: None,
        }],
    }];
}

fn text_inline(node_id: u32) -> wire::WireInline {
    wire::WireInline::Text {
        node_id,
        span: source_span(),
        text_span: wire::WireTextSpan {
            text_id: 0,
            start_byte: 0,
            end_byte: 1,
        },
    }
}

fn paragraph(node_id: u32, children: Vec<wire::WireInline>) -> wire::WireBlock {
    wire::WireBlock::Paragraph {
        node_id,
        span: source_span(),
        classes: Vec::new(),
        children,
    }
}

fn font_face(id: u32, family: &str, face_index: u32) -> wire::WireFontFace {
    wire::WireFontFace {
        font_face_id: id,
        family: family.to_owned(),
        uri: format!("font-{id}.ttf"),
        face_index,
        expected_sha256: None,
    }
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
fn parse_fixture(label: &str, package: &wire::WireDocumentPackage) -> Box<ValidatedMachinePackage> {
    let root = FixtureRoot::new(label);
    let bytes = wire::DocumentPackageEncoder::default()
        .to_jcs_vec(package)
        .expect("fixture must canonically encode");
    let package_path = root.path().join("document-package.json");
    fs::write(&package_path, bytes).expect("PACKAGE fixture must be writable");
    fs::write(root.path().join("input.tsf"), []).expect("source fixture must be writable");

    let limits = ValidatedResourceLimits::new(ResourceLimits::default())
        .expect("default limits must validate");
    let options = MachineInputHostOptions::new(
        typaxis_core::HostPath::new(package_path).expect("fixture path must be absolute"),
        None,
    );
    let (session, raw) =
        HostMachineInputSession::open(options, &limits).expect("PACKAGE must be admitted");
    let decoded = session
        .decode_and_bind(
            &raw,
            &wire::StrictDocumentPackageDecoder::new(),
            &wire::DocumentPackageDecodePolicy::new(&limits),
        )
        .expect("PACKAGE fixture must decode");
    let sources = session
        .admit_sources(&decoded, &limits)
        .expect("source fixture must be admitted");
    let admitted = session
        .finish(raw, decoded, sources)
        .expect("admission receipts must bind");
    let allowed_schemes = ["http", "https", "mailto", "tel"].map(str::to_owned);
    let policy = PackageValidationPolicy::new(&limits, &allowed_schemes)
        .expect("default URI schemes must validate");
    match DocumentPackageParser::new().parse(admitted, &policy) {
        MachineParseOutcome::Parsed { package } => package,
        MachineParseOutcome::Failed { failure, .. } => {
            panic!("fixture must cross syntax validation: {failure}")
        }
    }
}

fn run_preflight(
    package: &ValidatedMachinePackage,
) -> (
    Result<MachinePdfPreflightReceipt, MachinePdfPreflightFailure>,
    typaxis_diagnostics::MachineDiagnostics,
) {
    let mut budget = MachineDiagnosticBudget::new();
    let result = {
        let mut lender = budget
            .lend(MachineDiagnosticPhase::Capability)
            .expect("fresh budget must lend capability phase");
        MachinePdfPreflight::PARAGRAPH_1.run(package, &mut lender)
    };
    (result, budget.finish())
}

#[test]
fn descriptor_is_a_closed_exhaustive_paragraph_1_contract() {
    let descriptor = MachineProfileDescriptor::PARAGRAPH_1;
    assert_eq!(descriptor.id(), MachinePdfProfileId::PARAGRAPH_1);
    assert_eq!(
        descriptor.accepted_blocks(),
        &[MachineBlockKind::Heading, MachineBlockKind::Paragraph]
    );
    assert_eq!(
        descriptor.rejected_blocks(),
        &[
            MachineBlockKind::Figure,
            MachineBlockKind::List,
            MachineBlockKind::PageBreak,
            MachineBlockKind::Table,
        ]
    );
    assert_eq!(
        descriptor.accepted_font_formats(),
        &[
            MachineFontFormat::SfntTrueTypeGlyf,
            MachineFontFormat::TtcTrueTypeGlyf,
        ]
    );
    assert_eq!(descriptor.minimum_fonts_for_text(), 1);
    assert!(descriptor.accepted_image_formats().is_empty());
    assert!(!descriptor.footnotes().definitions());
    assert!(!descriptor.footnotes().references());
    assert_eq!(
        descriptor.unsupported_pdf_features(),
        &[
            MachinePdfFeature::HeadingSemantics,
            MachinePdfFeature::LinkAnnotations,
            MachinePdfFeature::Outlines,
            MachinePdfFeature::TaggedPdf,
        ]
    );

    assert_partition(
        descriptor.accepted_blocks(),
        descriptor.rejected_blocks(),
        &[
            MachineBlockKind::Figure,
            MachineBlockKind::Heading,
            MachineBlockKind::List,
            MachineBlockKind::PageBreak,
            MachineBlockKind::Paragraph,
            MachineBlockKind::Table,
        ],
    );
    assert_partition(
        descriptor.accepted_inlines(),
        descriptor.rejected_inlines(),
        &[
            MachineInlineKind::Anchor,
            MachineInlineKind::Emphasis,
            MachineInlineKind::FootnoteReference,
            MachineInlineKind::HardBreak,
            MachineInlineKind::Link,
            MachineInlineKind::Reference,
            MachineInlineKind::SoftBreak,
            MachineInlineKind::Strong,
            MachineInlineKind::Text,
        ],
    );
    assert_partition(
        descriptor.accepted_reference_formats(),
        descriptor.rejected_reference_formats(),
        &[
            MachineReferenceFormat::Number,
            MachineReferenceFormat::Page,
            MachineReferenceFormat::Text,
        ],
    );
    assert_partition(
        descriptor.accepted_style_selectors(),
        descriptor.rejected_style_selectors(),
        &[
            MachineBlockKind::Figure,
            MachineBlockKind::Heading,
            MachineBlockKind::List,
            MachineBlockKind::PageBreak,
            MachineBlockKind::Paragraph,
            MachineBlockKind::Table,
        ],
    );
    assert_partition(
        descriptor.accepted_style_properties(),
        descriptor.rejected_style_properties(),
        &[
            MachineStyleProperty::FontFamily,
            MachineStyleProperty::FontSize,
            MachineStyleProperty::LineHeight,
            MachineStyleProperty::Page,
        ],
    );
    assert_partition(
        descriptor.accepted_page_values(),
        descriptor.rejected_page_values(),
        &[MachinePageValue::Auto, MachinePageValue::Named],
    );
    assert_partition(
        descriptor.page_master().optional_frames(),
        descriptor.page_master().rejected_optional_frames(),
        &[
            MachinePageFrame::Footer,
            MachinePageFrame::Footnote,
            MachinePageFrame::Header,
        ],
    );
    assert_partition(
        descriptor.accepted_font_formats(),
        descriptor.rejected_font_formats(),
        &[
            MachineFontFormat::OpenTypeCff,
            MachineFontFormat::SfntTrueTypeGlyf,
            MachineFontFormat::TtcOpenTypeCff,
            MachineFontFormat::TtcTrueTypeGlyf,
            MachineFontFormat::Woff2,
        ],
    );
    assert_partition(
        descriptor.accepted_image_formats(),
        descriptor.rejected_image_formats(),
        &[
            MachineImageFormat::Jpeg,
            MachineImageFormat::Png,
            MachineImageFormat::Svg,
            MachineImageFormat::Vector,
        ],
    );
    assert_partition(
        descriptor.pdf_features(),
        descriptor.unsupported_pdf_features(),
        &[
            MachinePdfFeature::HeadingSemantics,
            MachinePdfFeature::LinkAnnotations,
            MachinePdfFeature::NamedDestinations,
            MachinePdfFeature::Outlines,
            MachinePdfFeature::TaggedPdf,
            MachinePdfFeature::TextExtraction,
        ],
    );
}

fn assert_partition<T>(accepted: &[T], rejected: &[T], domain: &[T])
where
    T: Copy + std::fmt::Debug + Ord,
{
    let accepted: BTreeSet<T> = accepted.iter().copied().collect();
    let rejected: BTreeSet<T> = rejected.iter().copied().collect();
    assert!(accepted.is_disjoint(&rejected));
    assert_eq!(
        accepted.union(&rejected).copied().collect::<BTreeSet<_>>(),
        domain.iter().copied().collect()
    );
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn capabilities_are_the_exact_canonical_descriptor_projection() {
    let encoded = encode_capabilities_canonical(HostCapabilityDescriptor::compiled());
    let expected = concat!(
        "{\"contract\":\"typaxis.contract/1.1\",",
        "\"engine\":{\"name\":\"typaxis\",\"version\":\"0.1.0\"},",
        "\"machine_input\":{",
        "\"coordinate_units\":[\"pdf_point_1_65536\"],",
        "\"default_profile\":\"typaxis.machine-pdf/paragraph-1\",",
        "\"document_package_contracts\":[\"typaxis.contract/1.0\",\"typaxis.contract/1.1\"],",
        "\"host_features\":{\"atomic_file_publish\":true,\"contained_package_open\":true,\"contained_resource_open\":true},",
        "\"host_limits\":{\"max_read_candidates\":131072,\"max_resource_roots\":64},",
        "\"limits\":{\"max_document_package_bytes\":{\"default\":134217728,\"maximum\":9007199254740991},",
        "\"max_json_nesting_depth\":{\"default\":256,\"maximum\":256}},",
        "\"max_diagnostics\":256,",
        "\"profiles\":[{",
        "\"available\":true,",
        "\"blocks\":[\"heading\",\"paragraph\"],",
        "\"font_formats\":[\"sfnt-truetype-glyf\",\"ttc-truetype-glyf\"],",
        "\"footnotes\":false,",
        "\"id\":\"typaxis.machine-pdf/paragraph-1\",",
        "\"image_formats\":[],",
        "\"inlines\":{\"kinds\":[\"anchor\",\"hard_break\",\"reference\",\"soft_break\",\"text\"],\"reference_formats\":[\"page\"]},",
        "\"page_master\":{\"count\":1,\"optional_frames\":[],\"selection_rules\":false},",
        "\"page_values\":[\"auto\"],",
        "\"pdf_features\":[\"named-destinations\",\"text-extraction\"],",
        "\"source_closure\":\"entry_only\",",
        "\"source_count\":{\"maximum\":1,\"minimum\":1},",
        "\"style_block_types\":[\"heading\",\"paragraph\"],",
        "\"style_properties\":[\"font_family\",\"font_size\",\"line_height\",\"page\"],",
        "\"style_selectors\":[\"heading\",\"paragraph\"],",
        "\"unsupported_pdf_features\":[\"heading-semantics\",\"link-annotations\",\"outlines\",\"tagged-pdf\"]",
        "}]}}"
    );
    assert_eq!(encoded, expected);
    assert_eq!(
        encoded,
        compact_json_fixture(include_str!(
            "../../../../samples/conformance/machine-capabilities.json"
        ))
    );
}

fn compact_json_fixture(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_string = false;
    let mut escaped = false;
    for character in input.chars() {
        if in_string {
            output.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
        } else if character == '"' {
            in_string = true;
            output.push(character);
        } else if !character.is_ascii_whitespace() {
            output.push(character);
        }
    }
    output
}

#[test]
fn unavailable_host_uses_i9110_from_the_compiled_descriptor_gate() {
    let host = HostCapabilityDescriptor::contained_open_unavailable_for_test();
    assert!(!host.profile_available(MachinePdfProfileId::PARAGRAPH_1));
    let encoded = encode_capabilities_canonical(host);
    assert!(encoded.contains("\"available\":false"));

    let mut budget = MachineDiagnosticBudget::new();
    let result = {
        let mut lender = budget
            .lend(MachineDiagnosticPhase::Host)
            .expect("fresh budget must lend host phase");
        host.preflight(MachinePdfProfileId::PARAGRAPH_1, &mut lender)
    };
    assert_eq!(result, Err(HostCapabilityPreflightError::Unavailable));
    let diagnostics = budget.finish();
    assert_eq!(diagnostics.diagnostics().len(), 1);
    assert_eq!(*diagnostics.diagnostics()[0].code(), I9110);
    assert_eq!(diagnostics.diagnostics()[0].severity(), Severity::Fatal);
}

#[test]
fn compiled_host_descriptor_is_the_composition_of_owner_tokens() {
    assert_eq!(
        HostCapabilityDescriptor::compiled(),
        HostCapabilityDescriptor::compose(
            MachineInputCapabilityToken::compiled(),
            ResourceAdmissionCapabilityToken::compiled(),
            AtomicFilePublicationCapabilityToken::compiled(),
        )
    );
}

#[test]
fn atomic_unavailability_is_reported_but_does_not_promise_an_i9110_sidecar() {
    let host = HostCapabilityDescriptor::atomic_publication_unavailable_for_test();
    assert!(!host.profile_available(MachinePdfProfileId::PARAGRAPH_1));

    let mut budget = MachineDiagnosticBudget::new();
    let result = {
        let mut lender = budget
            .lend(MachineDiagnosticPhase::Host)
            .expect("fresh budget must lend host phase");
        host.preflight(MachinePdfProfileId::PARAGRAPH_1, &mut lender)
    };
    assert_eq!(result, Ok(()));
    assert!(budget.finish().diagnostics().is_empty());
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum AdvertisedFixture {
    SourceEntryOnly,
    Block(MachineBlockKind),
    Inline(MachineInlineKind),
    Reference(MachineReferenceFormat),
    StyleBlock(MachineBlockKind),
    StyleProperty(MachineStyleProperty),
    StyleSelector(MachineBlockKind),
    PageValue(MachinePageValue),
    DefaultPageMaster,
    Font(MachineFontFormat),
    Pdf(MachinePdfFeature),
}

const SINGLE_FIXTURES: &[AdvertisedFixture] = &[
    AdvertisedFixture::SourceEntryOnly,
    AdvertisedFixture::Block(MachineBlockKind::Heading),
    AdvertisedFixture::Block(MachineBlockKind::Paragraph),
    AdvertisedFixture::Inline(MachineInlineKind::Anchor),
    AdvertisedFixture::Inline(MachineInlineKind::HardBreak),
    AdvertisedFixture::Inline(MachineInlineKind::Reference),
    AdvertisedFixture::Inline(MachineInlineKind::SoftBreak),
    AdvertisedFixture::Inline(MachineInlineKind::Text),
    AdvertisedFixture::Reference(MachineReferenceFormat::Page),
    AdvertisedFixture::StyleBlock(MachineBlockKind::Heading),
    AdvertisedFixture::StyleBlock(MachineBlockKind::Paragraph),
    AdvertisedFixture::StyleProperty(MachineStyleProperty::FontFamily),
    AdvertisedFixture::StyleProperty(MachineStyleProperty::FontSize),
    AdvertisedFixture::StyleProperty(MachineStyleProperty::LineHeight),
    AdvertisedFixture::StyleProperty(MachineStyleProperty::Page),
    AdvertisedFixture::StyleSelector(MachineBlockKind::Heading),
    AdvertisedFixture::StyleSelector(MachineBlockKind::Paragraph),
    AdvertisedFixture::PageValue(MachinePageValue::Auto),
    AdvertisedFixture::DefaultPageMaster,
    AdvertisedFixture::Font(MachineFontFormat::SfntTrueTypeGlyf),
    AdvertisedFixture::Font(MachineFontFormat::TtcTrueTypeGlyf),
    AdvertisedFixture::Pdf(MachinePdfFeature::NamedDestinations),
    AdvertisedFixture::Pdf(MachinePdfFeature::TextExtraction),
];

fn advertised_fixture_keys(descriptor: MachineProfileDescriptor) -> BTreeSet<AdvertisedFixture> {
    let mut keys = BTreeSet::from([
        AdvertisedFixture::SourceEntryOnly,
        AdvertisedFixture::DefaultPageMaster,
    ]);
    keys.extend(
        descriptor
            .accepted_blocks()
            .iter()
            .copied()
            .map(AdvertisedFixture::Block),
    );
    keys.extend(
        descriptor
            .accepted_inlines()
            .iter()
            .copied()
            .map(AdvertisedFixture::Inline),
    );
    keys.extend(
        descriptor
            .accepted_reference_formats()
            .iter()
            .copied()
            .map(AdvertisedFixture::Reference),
    );
    keys.extend(
        descriptor
            .style_block_types()
            .iter()
            .copied()
            .map(AdvertisedFixture::StyleBlock),
    );
    keys.extend(
        descriptor
            .accepted_style_properties()
            .iter()
            .copied()
            .map(AdvertisedFixture::StyleProperty),
    );
    keys.extend(
        descriptor
            .accepted_style_selectors()
            .iter()
            .copied()
            .map(AdvertisedFixture::StyleSelector),
    );
    keys.extend(
        descriptor
            .accepted_page_values()
            .iter()
            .copied()
            .map(AdvertisedFixture::PageValue),
    );
    keys.extend(
        descriptor
            .accepted_font_formats()
            .iter()
            .copied()
            .map(AdvertisedFixture::Font),
    );
    keys.extend(
        descriptor
            .pdf_features()
            .iter()
            .copied()
            .map(AdvertisedFixture::Pdf),
    );
    keys
}

#[test]
fn descriptor_mutation_cannot_escape_single_fixture_coverage() {
    let advertised = advertised_fixture_keys(MachineProfileDescriptor::PARAGRAPH_1);
    let fixtures: BTreeSet<_> = SINGLE_FIXTURES.iter().copied().collect();
    assert_eq!(advertised, fixtures);

    for feature in SINGLE_FIXTURES {
        let mut mutated = fixtures.clone();
        assert!(mutated.remove(feature));
        assert_ne!(advertised, mutated, "removing {feature:?} must be detected");
    }
}

fn wire_for_single_fixture(feature: AdvertisedFixture) -> wire::WireDocumentPackage {
    let mut package = base_wire();
    match feature {
        AdvertisedFixture::SourceEntryOnly | AdvertisedFixture::DefaultPageMaster => {}
        AdvertisedFixture::Block(MachineBlockKind::Heading) => {
            package.document.blocks.push(wire::WireBlock::Heading {
                node_id: 1,
                span: source_span(),
                classes: Vec::new(),
                level: 1,
                anchor_id: None,
                children: Vec::new(),
            });
        }
        AdvertisedFixture::Block(MachineBlockKind::Paragraph) => {
            package.document.blocks.push(paragraph(1, Vec::new()));
        }
        AdvertisedFixture::Inline(kind) => {
            let inline = match kind {
                MachineInlineKind::Anchor => wire::WireInline::Anchor {
                    node_id: 2,
                    span: source_span(),
                    anchor_id: "target".to_owned(),
                },
                MachineInlineKind::HardBreak => wire::WireInline::HardBreak {
                    node_id: 2,
                    span: source_span(),
                },
                MachineInlineKind::Reference => {
                    package
                        .resources
                        .font_faces
                        .push(font_face(0, "Fixture", 0));
                    package.document.blocks.push(paragraph(
                        1,
                        vec![wire::WireInline::Anchor {
                            node_id: 2,
                            span: source_span(),
                            anchor_id: "target".to_owned(),
                        }],
                    ));
                    package.document.blocks.push(paragraph(
                        3,
                        vec![wire::WireInline::Reference {
                            node_id: 4,
                            span: source_span(),
                            target: "target".to_owned(),
                            format: wire::WireReferenceFormat::Page,
                        }],
                    ));
                    return package;
                }
                MachineInlineKind::SoftBreak => wire::WireInline::SoftBreak {
                    node_id: 2,
                    span: source_span(),
                },
                MachineInlineKind::Text => {
                    package
                        .resources
                        .font_faces
                        .push(font_face(0, "Fixture", 0));
                    add_text_buffer(&mut package);
                    text_inline(2)
                }
                _ => unreachable!("only advertised inline fixtures are constructed"),
            };
            package.document.blocks.push(paragraph(1, vec![inline]));
        }
        AdvertisedFixture::Reference(MachineReferenceFormat::Page) => {
            return wire_for_single_fixture(AdvertisedFixture::Inline(
                MachineInlineKind::Reference,
            ));
        }
        AdvertisedFixture::StyleBlock(kind) | AdvertisedFixture::StyleSelector(kind) => {
            package.style_sheet.rules.push(wire::WireStyleRule {
                style_id: "fixture".to_owned(),
                extends: None,
                selector: kind.as_str().to_owned(),
                source_order: 0,
                declarations: Vec::new(),
            });
        }
        AdvertisedFixture::StyleProperty(property) => {
            let value = match property {
                MachineStyleProperty::FontFamily => {
                    package
                        .resources
                        .font_faces
                        .push(font_face(0, "Fixture", 0));
                    wire::WireStyleValue::FontFamilyList {
                        families: vec!["Fixture".to_owned()],
                    }
                }
                MachineStyleProperty::FontSize | MachineStyleProperty::LineHeight => {
                    wire::WireStyleValue::Length { value: 12 }
                }
                MachineStyleProperty::Page => wire::WireStyleValue::Keyword {
                    value: "auto".to_owned(),
                },
            };
            let name = match property {
                MachineStyleProperty::FontFamily => wire::WireDeclarationName::FontFamily,
                MachineStyleProperty::FontSize => wire::WireDeclarationName::FontSize,
                MachineStyleProperty::LineHeight => wire::WireDeclarationName::LineHeight,
                MachineStyleProperty::Page => wire::WireDeclarationName::Page,
            };
            package.style_sheet.rules.push(wire::WireStyleRule {
                style_id: "fixture".to_owned(),
                extends: None,
                selector: "paragraph".to_owned(),
                source_order: 0,
                declarations: vec![wire::WireDeclaration {
                    name,
                    value,
                    important: false,
                }],
            });
        }
        AdvertisedFixture::PageValue(MachinePageValue::Auto) => {
            return wire_for_single_fixture(AdvertisedFixture::StyleProperty(
                MachineStyleProperty::Page,
            ));
        }
        AdvertisedFixture::Font(format) => {
            let face_index = match format {
                MachineFontFormat::SfntTrueTypeGlyf => 0,
                MachineFontFormat::TtcTrueTypeGlyf => 1,
                _ => unreachable!("only advertised font fixtures are constructed"),
            };
            package
                .resources
                .font_faces
                .push(font_face(0, "Fixture", face_index));
        }
        AdvertisedFixture::Pdf(MachinePdfFeature::NamedDestinations) => {
            package.document.blocks.push(paragraph(
                1,
                vec![wire::WireInline::Anchor {
                    node_id: 2,
                    span: source_span(),
                    anchor_id: "target".to_owned(),
                }],
            ));
        }
        AdvertisedFixture::Pdf(MachinePdfFeature::TextExtraction) => {
            package
                .resources
                .font_faces
                .push(font_face(0, "Fixture", 0));
            add_text_buffer(&mut package);
            package
                .document
                .blocks
                .push(paragraph(1, vec![text_inline(2)]));
        }
        AdvertisedFixture::Block(_)
        | AdvertisedFixture::Reference(_)
        | AdvertisedFixture::PageValue(_)
        | AdvertisedFixture::Pdf(_) => {
            unreachable!("rejected descriptor values have no positive fixture")
        }
    }
    package
}

fn combined_wire(heading_level: u8) -> wire::WireDocumentPackage {
    let mut package = base_wire();
    add_text_buffer(&mut package);
    package.document.blocks = vec![
        wire::WireBlock::Heading {
            node_id: 1,
            span: source_span(),
            classes: Vec::new(),
            level: heading_level,
            anchor_id: Some("heading".to_owned()),
            children: vec![
                wire::WireInline::Anchor {
                    node_id: 2,
                    span: source_span(),
                    anchor_id: "inline".to_owned(),
                },
                text_inline(3),
                wire::WireInline::Reference {
                    node_id: 4,
                    span: source_span(),
                    target: "inline".to_owned(),
                    format: wire::WireReferenceFormat::Page,
                },
                wire::WireInline::SoftBreak {
                    node_id: 5,
                    span: source_span(),
                },
                wire::WireInline::HardBreak {
                    node_id: 6,
                    span: source_span(),
                },
            ],
        },
        paragraph(7, Vec::new()),
    ];
    package.resources.font_faces =
        vec![font_face(0, "Standalone", 0), font_face(1, "Collection", 1)];
    package.style_sheet.rules = vec![
        wire::WireStyleRule {
            style_id: "heading_style".to_owned(),
            extends: None,
            selector: "heading".to_owned(),
            source_order: 0,
            declarations: vec![
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontFamily,
                    value: wire::WireStyleValue::FontFamilyList {
                        families: vec!["Standalone".to_owned()],
                    },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::FontSize,
                    value: wire::WireStyleValue::Length { value: 12 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::LineHeight,
                    value: wire::WireStyleValue::Length { value: 14 },
                    important: false,
                },
                wire::WireDeclaration {
                    name: wire::WireDeclarationName::Page,
                    value: wire::WireStyleValue::Keyword {
                        value: "auto".to_owned(),
                    },
                    important: false,
                },
            ],
        },
        wire::WireStyleRule {
            style_id: "paragraph_style".to_owned(),
            extends: None,
            selector: "paragraph".to_owned(),
            source_order: 1,
            declarations: Vec::new(),
        },
    ];
    package
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn every_advertised_item_and_the_combined_fixture_pass_preflight() {
    for (ordinal, feature) in SINGLE_FIXTURES.iter().copied().enumerate() {
        let package = parse_fixture(
            &format!("single-{ordinal}"),
            &wire_for_single_fixture(feature),
        );
        let (result, diagnostics) = run_preflight(&package);
        assert!(result.is_ok(), "single fixture failed for {feature:?}");
        assert!(diagnostics.diagnostics().is_empty());
    }

    let package = parse_fixture("combined", &combined_wire(3));
    let (result, diagnostics) = run_preflight(&package);
    let receipt = result.expect("combined advertised fixture must pass");
    assert!(diagnostics.diagnostics().is_empty());
    assert!(receipt.matches(MachinePdfProfileId::PARAGRAPH_1, &package));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn heading_level_changes_document_fingerprint_without_advertising_semantics() {
    let first = parse_fixture("heading-one", &combined_wire(1));
    let second = parse_fixture("heading-two", &combined_wire(2));
    let (first_result, _) = run_preflight(&first);
    let (second_result, _) = run_preflight(&second);
    let first_receipt = first_result.expect("level one must pass");
    let second_receipt = second_result.expect("level two must pass");
    assert_ne!(
        first_receipt.document_fingerprint(),
        second_receipt.document_fingerprint()
    );
    assert!(MachineProfileDescriptor::PARAGRAPH_1
        .unsupported_pdf_features()
        .contains(&MachinePdfFeature::HeadingSemantics));
}

fn rejected_wire() -> wire::WireDocumentPackage {
    let mut package = base_wire();
    add_text_buffer(&mut package);
    package.document.blocks = vec![
        paragraph(
            1,
            vec![
                wire::WireInline::Anchor {
                    node_id: 2,
                    span: source_span(),
                    anchor_id: "target".to_owned(),
                },
                wire::WireInline::Emphasis {
                    node_id: 3,
                    span: source_span(),
                    children: vec![text_inline(4)],
                },
                wire::WireInline::Strong {
                    node_id: 5,
                    span: source_span(),
                    children: vec![text_inline(6)],
                },
                wire::WireInline::Link {
                    node_id: 7,
                    span: source_span(),
                    target: wire::WireLinkTarget::Uri {
                        uri: "https://example.com".to_owned(),
                    },
                    children: vec![text_inline(8)],
                },
                wire::WireInline::Reference {
                    node_id: 9,
                    span: source_span(),
                    target: "target".to_owned(),
                    format: wire::WireReferenceFormat::Text,
                },
                wire::WireInline::Reference {
                    node_id: 10,
                    span: source_span(),
                    target: "target".to_owned(),
                    format: wire::WireReferenceFormat::Number,
                },
                wire::WireInline::FootnoteReference {
                    node_id: 11,
                    span: source_span(),
                    footnote_id: "note".to_owned(),
                },
            ],
        ),
        wire::WireBlock::Figure {
            node_id: 12,
            span: source_span(),
            classes: Vec::new(),
            image_id: 1,
            alt: "fixture".to_owned(),
            caption: Vec::new(),
        },
    ];
    package.document.footnotes.push(wire::WireFootnote {
        footnote_id: "note".to_owned(),
        node_id: 13,
        span: source_span(),
        blocks: vec![paragraph(14, Vec::new())],
    });
    package.style_sheet.rules = vec![
        wire::WireStyleRule {
            style_id: "list_style".to_owned(),
            extends: None,
            selector: "list".to_owned(),
            source_order: 0,
            declarations: vec![wire::WireDeclaration {
                name: wire::WireDeclarationName::Page,
                value: wire::WireStyleValue::String {
                    value: "special".to_owned(),
                },
                important: false,
            }],
        },
        wire::WireStyleRule {
            style_id: "class_style".to_owned(),
            extends: None,
            selector: "paragraph.fixture".to_owned(),
            source_order: 1,
            declarations: Vec::new(),
        },
    ];
    let mut master_a = default_master("a");
    master_a.header = Some(wire::WireRect {
        x: 0,
        y: 0,
        width: 100,
        height: 10,
    });
    package.page_masters = wire::WirePageMasterSet {
        default_master_id: "a".to_owned(),
        masters: vec![master_a, default_master("b")],
        selection_rules: vec![wire::WirePageMasterRule {
            master_id: "a".to_owned(),
            parity: wire::WirePageParity::Any,
            first: None,
            named_page: None,
            source_order: 0,
        }],
    };
    package.resources.images = vec![
        wire::WireImage {
            image_id: 0,
            uri: "first.png".to_owned(),
            expected_sha256: None,
        },
        wire::WireImage {
            image_id: 1,
            uri: "second.png".to_owned(),
            expected_sha256: None,
        },
    ];
    package
        .resources
        .font_faces
        .push(font_face(0, "Fixture", 0));
    package
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn unsupported_domains_are_rejected_in_node_then_global_order() {
    let package = parse_fixture("rejected", &rejected_wire());
    let (result, diagnostics) = run_preflight(&package);
    assert_eq!(
        result.unwrap_err(),
        MachinePdfPreflightFailure::Unsupported {
            violation_count: 18,
            primary_code: L5100,
        }
    );
    let codes: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .map(|diagnostic| *diagnostic.code())
        .collect();
    assert_eq!(
        codes,
        vec![
            L5100, L5100, L5100, L5100, L5100, L5100, L5100, R7100, L5100, L5101, L5101, L5101,
            L5101, L5101, L5101, L5101, R7100, R7100,
        ]
    );
    let content_nodes: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| match diagnostic.subject() {
            Some(DiagnosticSubject::Layout(subject)) => Some(subject.node_id().get()),
            _ => None,
        })
        .collect();
    assert_eq!(content_nodes, vec![3, 5, 7, 9, 10, 11, 12, 13]);
    let style_ids: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| match diagnostic.subject() {
            Some(DiagnosticSubject::Style(subject)) => {
                subject.style_id().map(|style_id| style_id.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(style_ids, vec!["list_style", "list_style", "class_style"]);
    let master_ids: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| match diagnostic.subject() {
            Some(DiagnosticSubject::Master(subject)) => Some(subject.master_id().as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(master_ids, vec!["a", "a", "a", "b"]);
    let image_ids: Vec<_> = diagnostics
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| match diagnostic.subject() {
            Some(DiagnosticSubject::Resource(
                typaxis_diagnostics::ResourceErrorSubject::Image(image_id),
            )) => Some(image_id.get()),
            _ => None,
        })
        .collect();
    assert_eq!(image_ids, vec![1, 0, 1]);
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn all_ast_work_completes_after_the_shared_budget_starts_omitting() {
    let mut wire = base_wire();
    wire.document.blocks = (1..=300)
        .map(|node_id| wire::WireBlock::PageBreak {
            node_id,
            span: source_span(),
            classes: Vec::new(),
        })
        .collect();
    let package = parse_fixture("diagnostic-budget", &wire);
    let (result, diagnostics) = run_preflight(&package);
    assert_eq!(
        result.unwrap_err(),
        MachinePdfPreflightFailure::Unsupported {
            violation_count: 300,
            primary_code: L5100,
        }
    );
    assert_eq!(diagnostics.diagnostics().len(), MAX_MACHINE_DIAGNOSTICS);
    assert_eq!(diagnostics.omitted_count(), 44);
    assert_eq!(
        diagnostics
            .diagnostics()
            .last()
            .and_then(|diagnostic| diagnostic.notes().last())
            .and_then(|note| note.omitted_count()),
        Some(44)
    );
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn zero_fonts_are_allowed_only_without_a_text_producing_site() {
    let blank = parse_fixture("zero-font-blank", &base_wire());
    assert!(run_preflight(&blank).0.is_ok());

    let mut text = base_wire();
    add_text_buffer(&mut text);
    text.document
        .blocks
        .push(paragraph(1, vec![text_inline(2)]));
    let text = parse_fixture("zero-font-text", &text);
    let (result, diagnostics) = run_preflight(&text);
    assert_eq!(
        result.unwrap_err(),
        MachinePdfPreflightFailure::Unsupported {
            violation_count: 1,
            primary_code: R7100,
        }
    );
    assert_eq!(*diagnostics.diagnostics()[0].code(), R7100);
    assert!(matches!(
        diagnostics.diagnostics()[0].subject(),
        Some(DiagnosticSubject::Resource(
            typaxis_diagnostics::ResourceErrorSubject::FontFace(font_face_id)
        )) if *font_face_id == typaxis_core::FontFaceId::new(0)
    ));
}

#[cfg(any(target_os = "android", target_os = "linux", target_os = "macos"))]
#[test]
fn receipt_binds_the_opaque_admission_session_even_for_identical_bytes() {
    let wire = combined_wire(2);
    let first = parse_fixture("session-first", &wire);
    let second = parse_fixture("session-second", &wire);
    let (result, _) = run_preflight(&first);
    let receipt = result.expect("first package must pass");
    assert_eq!(
        receipt.verify(MachinePdfProfileId::PARAGRAPH_1, &second),
        Err(MachinePdfReceiptMismatch::Session)
    );
}
