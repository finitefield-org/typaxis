use crate::descriptor::MachineProfileDescriptor;
use typaxis_core::{
    push_jcs_string, DocumentPackageContractId, MachineInputLimitBounds, MachinePdfProfileId,
    COORDINATE_UNIT, PRODUCT_NAME,
};
use typaxis_diagnostics::MAX_MACHINE_DIAGNOSTICS;
use typaxis_syntax::machine_profile_boundary::{
    AtomicFilePublicationCapabilityToken, MachineInputCapabilityToken,
    ResourceAdmissionCapabilityToken, MAX_HOST_READ_CANDIDATES, MAX_RESOURCE_ROOTS,
};

/// Target-derived host facts required by a machine-PDF profile.
///
/// Construction is intentionally limited to sealed compile-time tokens from
/// the package, resource, and publication boundaries. Neither configuration
/// nor the filesystem can alter these booleans or fixed host limits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HostCapabilityDescriptor {
    atomic_file_publish: bool,
    contained_package_open: bool,
    contained_resource_open: bool,
    max_read_candidates: usize,
    max_resource_roots: usize,
}

impl HostCapabilityDescriptor {
    pub const fn compiled() -> Self {
        Self::compose(
            MachineInputCapabilityToken::compiled(),
            ResourceAdmissionCapabilityToken::compiled(),
            AtomicFilePublicationCapabilityToken::compiled(),
        )
    }

    /// Compose the three sealed owner tokens. There is deliberately no
    /// constructor accepting raw booleans.
    pub const fn compose(
        machine_input: MachineInputCapabilityToken,
        resource_admission: ResourceAdmissionCapabilityToken,
        atomic_publication: AtomicFilePublicationCapabilityToken,
    ) -> Self {
        Self {
            atomic_file_publish: atomic_publication.available(),
            contained_package_open: machine_input.contained_package_open(),
            contained_resource_open: resource_admission.contained_resource_open(),
            max_read_candidates: MAX_HOST_READ_CANDIDATES,
            max_resource_roots: MAX_RESOURCE_ROOTS,
        }
    }

    pub const fn atomic_file_publish(self) -> bool {
        self.atomic_file_publish
    }

    pub const fn contained_package_open(self) -> bool {
        self.contained_package_open
    }

    pub const fn contained_resource_open(self) -> bool {
        self.contained_resource_open
    }

    pub const fn max_read_candidates(self) -> usize {
        self.max_read_candidates
    }

    pub const fn max_resource_roots(self) -> usize {
        self.max_resource_roots
    }

    pub const fn profile_available(self, profile: MachinePdfProfileId) -> bool {
        match profile {
            MachinePdfProfileId::BasicDocument1
            | MachinePdfProfileId::Paragraph1
            | MachinePdfProfileId::Table1 => {
                self.atomic_file_publish
                    && self.contained_package_open
                    && self.contained_resource_open
            }
        }
    }

    #[cfg(test)]
    pub(crate) const fn contained_open_unavailable_for_test() -> Self {
        Self {
            contained_package_open: false,
            ..Self::compiled()
        }
    }

    #[cfg(test)]
    pub(crate) const fn atomic_publication_unavailable_for_test() -> Self {
        Self {
            atomic_file_publish: false,
            ..Self::compiled()
        }
    }
}

/// Canonically encode the machine-input capabilities artifact.
///
/// The result is JCS-compatible by construction: object keys and every
/// descriptor slice are already in lexical order, numbers stay inside the JSON
/// safe integer range, and all strings use the shared JCS escaper.
pub fn encode_capabilities_canonical(host: HostCapabilityDescriptor) -> String {
    let default_profile = MachineProfileDescriptor::PARAGRAPH_1;
    let profiles = [
        MachineProfileDescriptor::BASIC_DOCUMENT_1,
        MachineProfileDescriptor::PARAGRAPH_1,
        MachineProfileDescriptor::TABLE_1,
    ];
    let mut output = String::with_capacity(3_200);
    output.push_str("{\"contract\":");
    push_jcs_string(&mut output, DocumentPackageContractId::CURRENT.as_str());
    output.push_str(",\"engine\":{\"name\":");
    push_jcs_string(&mut output, PRODUCT_NAME);
    output.push_str(",\"version\":");
    push_jcs_string(&mut output, env!("CARGO_PKG_VERSION"));
    output.push_str("},\"machine_input\":{\"coordinate_units\":[");
    push_jcs_string(&mut output, COORDINATE_UNIT);
    output.push_str("],\"default_profile\":");
    push_jcs_string(&mut output, default_profile.id().as_str());
    output.push_str(",\"document_package_contracts\":[");
    push_jcs_string(&mut output, DocumentPackageContractId::V1_0.as_str());
    output.push(',');
    push_jcs_string(&mut output, DocumentPackageContractId::V1_1.as_str());
    output.push(',');
    push_jcs_string(&mut output, DocumentPackageContractId::V1_2.as_str());
    output.push_str("],\"host_features\":{\"atomic_file_publish\":");
    push_bool(&mut output, host.atomic_file_publish());
    output.push_str(",\"contained_package_open\":");
    push_bool(&mut output, host.contained_package_open());
    output.push_str(",\"contained_resource_open\":");
    push_bool(&mut output, host.contained_resource_open());
    output.push_str("},\"host_limits\":{\"max_read_candidates\":");
    output.push_str(&host.max_read_candidates().to_string());
    output.push_str(",\"max_resource_roots\":");
    output.push_str(&host.max_resource_roots().to_string());
    output.push_str("},\"limits\":{\"max_document_package_bytes\":{\"default\":");
    output.push_str(&MachineInputLimitBounds::DEFAULT_MAX_DOCUMENT_PACKAGE_BYTES.to_string());
    output.push_str(",\"maximum\":");
    output.push_str(&MachineInputLimitBounds::HARD_MAX_DOCUMENT_PACKAGE_BYTES.to_string());
    output.push_str("},\"max_json_nesting_depth\":{\"default\":");
    output.push_str(&MachineInputLimitBounds::DEFAULT_MAX_JSON_NESTING_DEPTH.to_string());
    output.push_str(",\"maximum\":");
    output.push_str(&MachineInputLimitBounds::HARD_MAX_JSON_NESTING_DEPTH.to_string());
    output.push_str("}},\"max_diagnostics\":");
    output.push_str(&MAX_MACHINE_DIAGNOSTICS.to_string());
    output.push_str(",\"profiles\":[");
    for (index, profile) in profiles.into_iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_profile(&mut output, profile, host);
    }
    output.push_str("]}}");
    output
}

fn push_profile(
    output: &mut String,
    profile: MachineProfileDescriptor,
    host: HostCapabilityDescriptor,
) {
    output.push_str("{\"available\":");
    push_bool(output, host.profile_available(profile.id()));
    output.push_str(",\"blocks\":");
    push_named_values(output, profile.accepted_blocks(), |value| value.as_str());
    output.push_str(",\"font_formats\":");
    push_named_values(output, profile.accepted_font_formats(), |value| {
        value.as_str()
    });
    output.push_str(",\"footnotes\":");
    push_bool(
        output,
        profile.footnotes().definitions() || profile.footnotes().references(),
    );
    output.push_str(",\"id\":");
    push_jcs_string(output, profile.id().as_str());
    output.push_str(",\"image_formats\":");
    push_named_values(output, profile.accepted_image_formats(), |value| {
        value.as_str()
    });
    output.push_str(",\"inlines\":{\"kinds\":");
    push_named_values(output, profile.accepted_inlines(), |value| value.as_str());
    output.push_str(",\"reference_formats\":");
    push_named_values(output, profile.accepted_reference_formats(), |value| {
        value.as_str()
    });
    output.push_str("},\"page_master\":{\"count\":");
    output.push_str(&profile.page_master().count().to_string());
    output.push_str(",\"optional_frames\":");
    push_named_values(output, profile.page_master().optional_frames(), |value| {
        value.as_str()
    });
    output.push_str(",\"selection_rules\":");
    push_bool(output, profile.page_master().selection_rules());
    output.push_str("},\"page_values\":");
    push_named_values(output, profile.accepted_page_values(), |value| {
        value.as_str()
    });
    output.push_str(",\"pdf_features\":");
    push_named_values(output, profile.pdf_features(), |value| value.as_str());
    output.push_str(",\"source_closure\":");
    push_jcs_string(output, profile.source_closure().as_str());
    output.push_str(",\"source_count\":{\"maximum\":");
    output.push_str(&profile.source_count().maximum().to_string());
    output.push_str(",\"minimum\":");
    output.push_str(&profile.source_count().minimum().to_string());
    output.push_str("},\"style_block_types\":");
    push_named_values(output, profile.style_block_types(), |value| value.as_str());
    output.push_str(",\"style_properties\":");
    push_named_values(output, profile.accepted_style_properties(), |value| {
        value.as_str()
    });
    output.push_str(",\"style_selectors\":");
    push_named_values(output, profile.accepted_style_selectors(), |value| {
        value.as_str()
    });
    output.push_str(",\"unsupported_pdf_features\":");
    push_named_values(output, profile.unsupported_pdf_features(), |value| {
        value.as_str()
    });
    output.push('}');
}

fn push_bool(output: &mut String, value: bool) {
    output.push_str(if value { "true" } else { "false" });
}

fn push_named_values<T>(output: &mut String, values: &[T], name: impl Fn(&T) -> &'static str) {
    output.push('[');
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        push_jcs_string(output, name(value));
    }
    output.push(']');
}
