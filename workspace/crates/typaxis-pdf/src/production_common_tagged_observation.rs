//! Observations issued only after the common serializer has verified its graph.
use super::*;
use std::fmt::Write;

#[derive(Debug)]
pub struct ProductionCommonVectorMarkedObservation {
    usage_id: u32,
    structure_node_id: u32,
    page_index: u32,
    mcid: u32,
    paint_ordinal: u32,
    semantic_fragment_ordinal: u32,
    page_object: u32,
    content_object: u32,
    structure_object: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}
impl ProductionCommonVectorMarkedObservation {
    pub fn usage_id(&self) -> u32 {
        self.usage_id
    }
    pub fn structure_node_id(&self) -> u32 {
        self.structure_node_id
    }
    pub fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn mcid(&self) -> u32 {
        self.mcid
    }
    pub fn paint_ordinal(&self) -> u32 {
        self.paint_ordinal
    }
    pub fn semantic_fragment_ordinal(&self) -> u32 {
        self.semantic_fragment_ordinal
    }
    pub fn page_object(&self) -> u32 {
        self.page_object
    }
    pub fn content_object(&self) -> u32 {
        self.content_object
    }
    pub fn structure_object(&self) -> u32 {
        self.structure_object
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Debug)]
pub struct ProductionCommonTaggedObservation {
    vector_records: Vec<ProductionCommonVectorMarkedObservation>,
    vector_artifacts: Vec<(u32, String)>,
    marked_content_jcs: String,
    marked_content_sha256: [u8; 32],
    selected_binding_sha256: [u8; 32],
    canonical_jcs: String,
    fingerprint: [u8; 32],
    pub(super) record_charge: u64,
    pub(super) spool_charge: u64,
}
impl ProductionCommonTaggedObservation {
    pub fn vector_records(&self) -> &[ProductionCommonVectorMarkedObservation] {
        &self.vector_records
    }
    pub fn vector_record(&self, usage_id: u32) -> Option<&ProductionCommonVectorMarkedObservation> {
        self.vector_records
            .binary_search_by_key(&usage_id, |r| r.usage_id)
            .ok()
            .map(|i| &self.vector_records[i])
    }
    pub fn vector_is_artifact(&self, usage_id: u32) -> bool {
        self.vector_artifacts
            .binary_search_by_key(&usage_id, |(id, _)| *id)
            .is_ok()
    }
    pub fn vector_artifact_count(&self) -> usize {
        self.vector_artifacts.len()
    }
    pub fn marked_content_jcs(&self) -> &str {
        &self.marked_content_jcs
    }
    pub fn marked_content_sha256(&self) -> [u8; 32] {
        self.marked_content_sha256
    }
    pub fn selected_binding_sha256(&self) -> [u8; 32] {
        self.selected_binding_sha256
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

// Private: the caller has completed pdf.verify and all final PDF closures.
// The source, selected groups, object numbers and bytes are taken from that
// same immutable assembly; callers cannot submit substitute observations.
pub(super) fn observe_common_tagged_graph(
    pdf: &ProductionFootnotePdfAssembly<
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
        '_,
    >,
    limits: &M4EffectiveResourceLimits,
    previous_records: u64,
    previous_spool: u64,
) -> Result<ProductionCommonTaggedObservation, E> {
    let marked = pdf.source.structure_objects().annotations().marked();
    let structure = marked.structure();
    let vector_count = pdf.vector_final_writer.usages().len() as u64;
    let artifact_count = pdf
        .vector_final_writer
        .usages()
        .iter()
        .filter(|usage| {
            structure
                .display()
                .table_draw_role(usage.paint_ordinal() as usize)
                .is_some_and(|role| role.repeated_header())
        })
        .count() as u64;
    let record_charge = previous_records
        .checked_add(vector_count.checked_mul(2).ok_or(E::RecordLimit)?)
        .and_then(|v| v.checked_add(4))
        .ok_or(E::RecordLimit)?;
    let extra = vector_count
        .checked_mul(2048)
        .and_then(|v| v.checked_add((pdf.observations.len() as u64).checked_mul(256)?))
        .and_then(|v| v.checked_add((marked.pages().len() as u64).checked_mul(256)?))
        .and_then(|v| v.checked_add(4096))
        .ok_or(E::SpoolLimit)?;
    let spool_charge = previous_spool.checked_add(extra).ok_or(E::SpoolLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::RecordLimit);
    }
    if spool_charge > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    let mut marked_content_jcs =
        String::from("{\"algorithm\":\"typaxis.production-common-marked-content/1\",\"pages\":[");
    for (i, page) in marked.pages().iter().enumerate() {
        if i > 0 {
            marked_content_jcs.push(',');
        }
        write!(
            marked_content_jcs,
            "{{\"byte_length\":{},\"content_sha256\":",
            page.content().len()
        )
        .unwrap();
        push_observed_hash(&mut marked_content_jcs, sha256(page.content()));
        write!(
            marked_content_jcs,
            ",\"page_index\":{}}}",
            page.page_index()
        )
        .unwrap();
    }
    marked_content_jcs.push_str("],\"selected_binding_sha256\":");
    push_observed_hash(&mut marked_content_jcs, structure.fingerprint());
    marked_content_jcs.push('}');
    let marked_content_sha256 = sha256(marked_content_jcs.as_bytes());
    let mut vector_records = Vec::new();
    vector_records
        .try_reserve_exact(
            usize::try_from(vector_count - artifact_count).map_err(|_| E::RecordLimit)?,
        )
        .map_err(|_| E::AllocationFailure)?;
    for group in structure.groups() {
        let Some(usage_id) = group.vector_usage_id() else {
            continue;
        };
        if vector_records
            .last()
            .is_some_and(|r: &ProductionCommonVectorMarkedObservation| r.usage_id >= usage_id)
        {
            return Err(E::ReceiptMismatch);
        }
        let page_object = pdf
            .object_number(R::Page(group.page_index()))
            .ok_or(E::ReceiptMismatch)?;
        let content_object = pdf
            .object_number(R::PageContent(group.page_index()))
            .ok_or(E::ReceiptMismatch)?;
        let structure_object = pdf
            .object_number(R::StructureNode(group.node()))
            .ok_or(E::ReceiptMismatch)?;
        let paint_ordinal = u32::try_from(group.draws().start).map_err(|_| E::RecordLimit)?;
        let usage = pdf
            .vector_final_writer
            .usages()
            .get(usage_id as usize)
            .ok_or(E::ReceiptMismatch)?;
        if usage.page_index() != group.page_index()
            || usage.paint_ordinal() != paint_ordinal
            || usage.page_object_number() != page_object
            || usage.page_content_object_number() != content_object
        {
            return Err(E::ReceiptMismatch);
        }
        let mut canonical_jcs =
            format!("{{\"content_object\":{content_object},\"marked_content_sha256\":");
        push_observed_hash(&mut canonical_jcs, marked_content_sha256);
        write!(canonical_jcs, ",\"mcid\":{},\"page_index\":{},\"page_object\":{page_object},\"paint_ordinal\":{paint_ordinal},\"semantic_fragment_ordinal\":{},\"structure_node_id\":{},\"structure_object\":{structure_object},\"usage_id\":{usage_id}}}",
            group.mcid(), group.page_index(), group.semantic_fragment_ordinal(), group.node().get()).unwrap();
        let fingerprint = sha256(canonical_jcs.as_bytes());
        vector_records.push(ProductionCommonVectorMarkedObservation {
            usage_id,
            structure_node_id: group.node().get(),
            page_index: group.page_index(),
            mcid: group.mcid(),
            paint_ordinal,
            semantic_fragment_ordinal: group.semantic_fragment_ordinal(),
            page_object,
            content_object,
            structure_object,
            canonical_jcs,
            fingerprint,
        });
    }
    let display = structure.display();
    let mut vector_artifacts = Vec::new();
    vector_artifacts
        .try_reserve_exact(usize::try_from(artifact_count).map_err(|_| E::RecordLimit)?)
        .map_err(|_| E::AllocationFailure)?;
    // The precharged two records per physical usage cover the semantic/copy
    // record and its retained collection slot. Visit actual writer usages so
    // every Form occurrence has exactly one semantic or artifact observation.
    for usage in pdf.vector_final_writer.usages() {
        let id = usage.usage_id();
        let paint = usage.paint_ordinal() as usize;
        let semantic = vector_records
            .binary_search_by_key(&id, |r| r.usage_id)
            .is_ok();
        let role = display
            .table_draw_role(paint)
            .filter(|r| r.repeated_header());
        if let Some(role) = role {
            if semantic {
                return Err(E::ReceiptMismatch);
            }
            let vector = display
                .draws()
                .get(paint)
                .and_then(|d| d.vector_paint())
                .ok_or(E::ReceiptMismatch)?;
            let page = vector.page_index();
            let page_object = pdf.object_number(R::Page(page)).ok_or(E::ReceiptMismatch)?;
            let content_object = pdf
                .object_number(R::PageContent(page))
                .ok_or(E::ReceiptMismatch)?;
            if page != usage.page_index()
                || page_object != usage.page_object_number()
                || content_object != usage.page_content_object_number()
            {
                return Err(E::ReceiptMismatch);
            }
            let mut record = format!("{{\"artifact\":\"table_header_copy\",\"content_object\":{content_object},\"form_object\":{},\"marked_content_sha256\":", usage.form_absolute_object_number());
            push_observed_hash(&mut record, marked_content_sha256);
            write!(record, ",\"owner\":{},\"page_index\":{page},\"page_object\":{page_object},\"paint_ordinal\":{paint},\"source_cell\":{},\"usage_id\":{id}}}", vector.owner().get(), role.owner().get()).unwrap();
            vector_artifacts
                .try_reserve(1)
                .map_err(|_| E::AllocationFailure)?;
            vector_artifacts.push((id, record));
        } else if !semantic {
            return Err(E::ReceiptMismatch);
        }
    }
    if (vector_records.len() + vector_artifacts.len()) as u64 != vector_count {
        return Err(E::ReceiptMismatch);
    }
    let mut canonical_jcs = String::from(if vector_artifacts.is_empty() {
        "{\"algorithm\":\"typaxis.production-common-tagged-pdf-observation/1\",\"marked_content_sha256\":"
    } else {
        "{\"algorithm\":\"typaxis.production-common-tagged-pdf-observation/2\",\"marked_content_sha256\":"
    });
    push_observed_hash(&mut canonical_jcs, marked_content_sha256);
    canonical_jcs.push_str(",\"objects\":[");
    for (i, object) in pdf.observations.iter().enumerate() {
        if i > 0 {
            canonical_jcs.push(',');
        }
        write!(
            canonical_jcs,
            "{{\"byte_length\":{},\"number\":{},\"offset\":{},\"sha256\":",
            object.byte_length(),
            object.number(),
            object.offset()
        )
        .unwrap();
        push_observed_hash(&mut canonical_jcs, object.sha256());
        canonical_jcs.push('}');
    }
    canonical_jcs.push_str("],\"pdf_sha256\":");
    push_observed_hash(&mut canonical_jcs, pdf.content_hash());
    canonical_jcs.push_str(",\"selected_binding_sha256\":");
    push_observed_hash(&mut canonical_jcs, structure.fingerprint());
    canonical_jcs.push_str(",\"structure_registry_sha256\":");
    push_observed_hash(&mut canonical_jcs, structure.registry().fingerprint());
    if !vector_artifacts.is_empty() {
        canonical_jcs.push_str(",\"vector_artifacts\":[");
        for (index, (_, record)) in vector_artifacts.iter().enumerate() {
            if index > 0 {
                canonical_jcs.push(',');
            }
            canonical_jcs.push_str(record);
        }
        canonical_jcs.push(']');
    }
    canonical_jcs.push_str(",\"vector_records\":[");
    for (i, record) in vector_records.iter().enumerate() {
        if i > 0 {
            canonical_jcs.push(',');
        }
        canonical_jcs.push_str(record.canonical_jcs());
    }
    canonical_jcs.push_str("]}");
    let retained_bytes = vector_records.iter().try_fold(
        (canonical_jcs.len() as u64)
            .checked_add(marked_content_jcs.len() as u64)
            .ok_or(E::SpoolLimit)?,
        |n, r| {
            n.checked_add(r.canonical_jcs.len() as u64)
                .ok_or(E::SpoolLimit)
        },
    )?;
    let retained_bytes = vector_artifacts
        .iter()
        .try_fold(retained_bytes, |n, (_, r)| {
            n.checked_add(r.len() as u64).ok_or(E::SpoolLimit)
        })?;
    if retained_bytes > extra {
        return Err(E::SpoolLimit);
    }
    Ok(ProductionCommonTaggedObservation {
        vector_records,
        vector_artifacts,
        marked_content_jcs,
        marked_content_sha256,
        selected_binding_sha256: structure.fingerprint(),
        fingerprint: sha256(canonical_jcs.as_bytes()),
        canonical_jcs,
        record_charge,
        spool_charge,
    })
}
fn push_observed_hash(out: &mut String, value: [u8; 32]) {
    out.push('"');
    for byte in value {
        write!(out, "{byte:02x}").unwrap();
    }
    out.push('"');
}
