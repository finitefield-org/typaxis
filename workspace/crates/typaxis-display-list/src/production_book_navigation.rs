//! Book-selection inputs projected from the actual joint display/navigation.
use super::*;
use typaxis_resource_admission::AdmittedResourceLedger;

pub struct ProductionFootnoteBookNavigationInputs<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'i crate::ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    pages: Vec<BookNavigationSelectedPage>,
    destinations: Vec<BookNavigationDestinationBinding>,
    links: Vec<BookInternalLink>,
    language_paints: Vec<BookLanguagePaintV2>,
    vector_paints: Vec<BookVectorLanguagePaintV2>,
    child_language_paints: Vec<BookLanguagePaintV2>,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionFootnoteBookNavigationInputs<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn pages(&self) -> &[BookNavigationSelectedPage] {
        &self.pages
    }
    pub fn destinations(&self) -> &[BookNavigationDestinationBinding] {
        &self.destinations
    }
    pub fn links(&self) -> &[BookInternalLink] {
        &self.links
    }
    pub fn language_paints(&self) -> &[BookLanguagePaintV2] {
        &self.language_paints
    }
    /// Equation-number children use child-registry fingerprints, not ordinary
    /// owner records. Their parent relationship remains in the source registry.
    pub fn child_language_paints(&self) -> &[BookLanguagePaintV2] {
        &self.child_language_paints
    }
    pub fn vector_paints(&self) -> &[BookVectorLanguagePaintV2] {
        &self.vector_paints
    }
    pub fn record_charge(&self) -> u64 {
        self.record_charge
    }
    pub fn spool_charge(&self) -> u64 {
        self.spool_charge
    }
    pub fn verify(
        &self,
        source: &crate::ProductionFootnoteNavigation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_>,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookNavigationSelectedError> {
        if !std::ptr::eq(self.source, source) {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        source
            .structure()
            .verify(source.structure().display(), admitted, limits)
            .map_err(|_| BookNavigationSelectedError::ReceiptMismatch)
    }
}

/// `record_base` and `spool_base` carry all already retained downstream work.
/// The common driver supplies the actual complete PDF assembly charges.
pub fn project_production_footnote_book_navigation<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    source: &'i crate::ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
    record_base: u64,
    spool_base: u64,
) -> Result<
    ProductionFootnoteBookNavigationInputs<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookNavigationSelectedError,
> {
    use BookNavigationSelectedError as E;
    let structure = source.structure();
    structure
        .verify(structure.display(), admitted, limits)
        .map_err(|_| E::ReceiptMismatch)?;
    source.verify(structure).map_err(|_| E::ReceiptMismatch)?;
    if record_base
        < source
            .record_base()
            .checked_add(source.additional_records())
            .ok_or(E::FragmentLimit)?
        || spool_base < structure.spool_charge()
    {
        return Err(E::ReceiptMismatch);
    }
    let display = structure.display();
    let navigation = display.source().line_layout().source_flow().navigation();
    let geometry = display.source().block_layout().page_geometry();
    let count = display
        .source()
        .geometry()
        .pages()
        .len()
        .checked_add(source.destinations().len())
        .and_then(|n| n.checked_add(source.links().len()))
        .and_then(|n| n.checked_add(display.draws().len()))
        .and_then(|n| n.checked_add(navigation.languages().records().len()))
        .ok_or(E::FragmentLimit)?;
    // Rows, temporary input rows and validation maps/sets are charged up front.
    let record_charge = record_base
        .checked_add((count as u64).checked_mul(8).ok_or(E::FragmentLimit)?)
        .and_then(|n| n.checked_add(1))
        .ok_or(E::FragmentLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::FragmentLimit);
    }
    if spool_base > limits.base().get().max_spool_bytes {
        return Err(E::SpoolLimit);
    }
    let mut spool_charge = spool_base;
    let mut charge_string = |text: &str| -> Result<(), E> {
        spool_charge = spool_charge
            .checked_add(text.len() as u64)
            .ok_or(E::SpoolLimit)?;
        if spool_charge > limits.base().get().max_spool_bytes {
            return Err(E::SpoolLimit);
        }
        Ok(())
    };
    let mut pages = Vec::new();
    pages
        .try_reserve_exact(display.source().geometry().pages().len())
        .map_err(|_| E::AllocationFailure)?;
    for page in 0..display.source().geometry().pages().len() {
        pages.push(BookNavigationSelectedPage {
            page_index: u32::try_from(page).map_err(|_| E::FragmentLimit)?,
            width_raw: geometry.page_width().get().raw(),
            height_raw: geometry.page_height().get().raw(),
        });
    }
    validate_pages(&pages, limits.base())?;
    let mut destinations = Vec::new();
    destinations
        .try_reserve_exact(source.destinations().len())
        .map_err(|_| E::AllocationFailure)?;
    for destination in source.destinations() {
        let name = source
            .destination_name(destination.anchor_index())
            .ok_or(E::DestinationMismatch)?;
        charge_string(name.as_str())?;
        destinations.push(BookNavigationDestinationBinding {
            source_node_id: destination.owner(),
            frame_id: destination.fragment_index(),
            destination: NamedDestination {
                anchor_id: name.clone(),
                page_index: destination.page_index(),
                view: DestinationView::Xyz {
                    point: typaxis_core::Point {
                        x: destination.x(),
                        y: destination.y(),
                    },
                },
            },
        });
    }
    let registry = validate_destinations_v2(navigation, &destinations, &pages)?;
    let mut link_inputs = Vec::new();
    link_inputs
        .try_reserve_exact(source.links().len())
        .map_err(|_| E::AllocationFailure)?;
    for link in source.links() {
        let Some(index) = link.destination_index() else {
            continue;
        };
        let name = source.destination_name(index).ok_or(E::InvalidLink)?;
        charge_string(name.as_str())?;
        charge_string(name.as_str())?;
        let bounds = link.bounds();
        link_inputs.push(BookInternalLinkInput {
            owner_node_id: link.owner(),
            page_index: link.page_index(),
            destination: name.clone(),
            x_raw: bounds.x().raw(),
            y_raw: bounds.y().raw(),
            width_raw: bounds.width().get().raw(),
            height_raw: bounds.height().get().raw(),
        });
    }
    link_inputs
        .sort_unstable_by_key(|link| (link.page_index, link.owner_node_id, link.x_raw, link.y_raw));
    let links = validate_links_v2(navigation, &link_inputs, &pages, &registry)?;
    let mut language_inputs = Vec::new();
    let mut vector_paints = Vec::new();
    language_inputs
        .try_reserve_exact(display.draws().len())
        .map_err(|_| E::AllocationFailure)?;
    vector_paints
        .try_reserve_exact(display.draws().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut child_language_paints = Vec::new();
    child_language_paints
        .try_reserve_exact(navigation.languages().child_records().len())
        .map_err(|_| E::AllocationFailure)?;
    let mut seen_children = BTreeSet::new();
    let mut occurrences = BTreeMap::<NodeId, u32>::new();
    for (index, draw) in display.draws().iter().enumerate() {
        let ordinal = u32::try_from(index).map_err(|_| E::FragmentLimit)?;
        match draw {
            crate::ProductionBodyDraw::Text(text) => {
                if text.equation_number().is_some() {
                    let child = navigation
                        .languages()
                        .child_record(text.owner())
                        .ok_or(E::InvalidLanguagePaint)?;
                    let parent = navigation
                        .languages()
                        .record(child.parent_owner_node_id)
                        .ok_or(E::InvalidLanguagePaint)?;
                    if child.parent_language_record_fingerprint != parent.record_fingerprint
                        || !seen_children.insert(child.node_id)
                    {
                        return Err(E::InvalidLanguagePaint);
                    }
                    charge_string(child.effective_language.as_ref())?;
                    child_language_paints.push(BookLanguagePaintV2 {
                        occurrence: 0,
                        owner_node_id: child.node_id,
                        page_index: text.page_index(),
                        paint_ordinal: ordinal,
                        language: child.effective_language.to_string(),
                        language_record_fingerprint: child.record_fingerprint,
                    });
                    continue;
                }

                let record = navigation
                    .languages()
                    .record(text.owner())
                    .ok_or(E::InvalidLanguagePaint)?;
                if record.effective_language.as_ref() == navigation.languages().document_language()
                    || !matches!(
                        record.node_kind,
                        StagingComputedLanguageOwnerKindV2::Text
                            | StagingComputedLanguageOwnerKindV2::Reference
                            | StagingComputedLanguageOwnerKindV2::FootnoteReference
                            | StagingComputedLanguageOwnerKindV2::InlineMath
                            | StagingComputedLanguageOwnerKindV2::DisplayMath
                    )
                {
                    continue;
                }
                charge_string(record.effective_language.as_ref())?;
                let occurrence = occurrences.entry(text.owner()).or_default();
                language_inputs.push(BookLanguagePaintInputV2 {
                    owner_node_id: text.owner(),
                    occurrence: *occurrence,
                    page_index: text.page_index(),
                    paint_ordinal: ordinal,
                });
                *occurrence = occurrence.checked_add(1).ok_or(E::FragmentLimit)?;
            }
            crate::ProductionBodyDraw::Vector(vector) => {
                let owner = vector.binding().node_id();
                let source_index = navigation
                    .languages()
                    .records()
                    .binary_search_by_key(&owner, |r| r.node_id)
                    .map_err(|_| E::InvalidLanguagePaint)?;
                let record = &navigation.languages().records()[source_index];
                charge_string(record.effective_language.as_ref())?;
                vector_paints.push(BookVectorLanguagePaintV2 {
                    usage_id: u32::try_from(vector_paints.len()).map_err(|_| E::FragmentLimit)?,
                    owner_node_id: owner,
                    kind: vector.binding().kind(),
                    source_owner_ordinal: u32::try_from(source_index)
                        .map_err(|_| E::FragmentLimit)?,
                    page_index: vector.page_index(),
                    paint_ordinal: ordinal,
                    language: record.effective_language.to_string(),
                    language_record_fingerprint: record.record_fingerprint,
                    display_command_fingerprint: vector.fingerprint(),
                    requires_paint_language: record.effective_language.as_ref()
                        != navigation.languages().document_language(),
                });
            }
            crate::ProductionBodyDraw::Raster(_) => (),
        }
    }
    if seen_children.len() != navigation.languages().child_records().len() {
        return Err(E::InvalidLanguagePaint);
    }
    let language_paints = validate_paints_v2(navigation, &pages, &language_inputs)?;
    validate_stored_vector_paints_v2(navigation, &pages, &language_paints, &vector_paints)?;
    Ok(ProductionFootnoteBookNavigationInputs {
        source,
        pages,
        destinations,
        links,
        language_paints,
        vector_paints,
        child_language_paints,
        record_charge,
        spool_charge,
    })
}
