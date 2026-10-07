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
    let mut seen_children = BTreeMap::new();
    let mut occurrences = BTreeMap::<NodeId, u32>::new();
    let mut physical_vector_usage = 0u32;
    for (index, draw) in display.draws().iter().enumerate() {
        let usage_id = physical_vector_usage;
        if draw.vector_paint().is_some() {
            physical_vector_usage = physical_vector_usage
                .checked_add(1)
                .ok_or(E::FragmentLimit)?;
        }
        // Artifact copies have no reading language or semantic destination.
        // Retain physical vector usage IDs for later semantic occurrences.
        if display
            .table_draw_role(index)
            .is_some_and(|role| role.repeated_header())
        {
            continue;
        }
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
                    if child.parent_language_record_fingerprint != parent.record_fingerprint {
                        return Err(E::InvalidLanguagePaint);
                    }
                    let occurrence = (text.page_index(), text.fragment_index());
                    if let Some(previous) = seen_children.get(&child.node_id) {
                        if *previous != occurrence {
                            return Err(E::InvalidLanguagePaint);
                        }
                        // A shaped equation number may contain several text
                        // clusters in one atomic selected fragment. Its language
                        // child is observed once, at the first actual draw.
                        continue;
                    }
                    seen_children.insert(child.node_id, occurrence);
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
                    usage_id,
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
            crate::ProductionBodyDraw::Math(math) => {
                let record = navigation
                    .languages()
                    .record(math.owner())
                    .ok_or(E::InvalidLanguagePaint)?;
                if !matches!(
                    record.node_kind,
                    StagingComputedLanguageOwnerKindV2::InlineMath
                        | StagingComputedLanguageOwnerKindV2::DisplayMath
                ) {
                    return Err(E::InvalidLanguagePaint);
                }
                if record.effective_language.as_ref() != navigation.languages().document_language()
                {
                    charge_string(record.effective_language.as_ref())?;
                    let occurrence = occurrences.entry(math.owner()).or_default();
                    language_inputs.push(BookLanguagePaintInputV2 {
                        owner_node_id: math.owner(),
                        occurrence: *occurrence,
                        page_index: math.page_index(),
                        paint_ordinal: ordinal,
                    });
                    *occurrence = occurrence.checked_add(1).ok_or(E::FragmentLimit)?;
                }
            }
            crate::ProductionBodyDraw::Raster(_) | crate::ProductionBodyDraw::SvgFigure(_) => (),
        }
    }
    if seen_children.len() != navigation.languages().child_records().len() {
        return Err(E::InvalidLanguagePaint);
    }
    let language_paints = validate_paints_v2(navigation, &pages, &language_inputs)?;
    validate_stored_vector_paints_with_sparse_usages_v2(
        navigation,
        &pages,
        &language_paints,
        &vector_paints,
        true,
    )?;
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

/// Selected book facts remain bound to the actual joint navigation. The legacy
/// Display verifier is not used to authorize this alternative source owner.
pub struct ProductionFootnoteBookNavigation<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    source: &'i crate::ProductionFootnoteNavigation<'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    selected: BookNavigationSelectedReceiptV2,
    child_language_paints: Vec<BookLanguagePaintV2>,
    record_charge: u64,
    spool_charge: u64,
}
impl ProductionFootnoteBookNavigation<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn selected(&self) -> &BookNavigationSelectedReceiptV2 {
        &self.selected
    }
    /// Actual one-based destination pages for Page-format references, in source
    /// traversal order. This borrows the sealed selection and allocates nothing.
    /// A caller retaining these rows must charge and sort them before supplying
    /// another generated-text candidate. Matching labels alone do not prove
    /// convergence of the surrounding line and page layout.
    pub fn resolved_page_references(
        &self,
    ) -> impl Iterator<Item = Result<(NodeId, u32), BookNavigationSelectedError>> + '_ {
        let flow = self
            .source
            .structure()
            .display()
            .source()
            .line_layout()
            .source_flow();
        flow.paragraphs()
            .iter()
            .flat_map(|p| p.items())
            .filter_map(|site| {
                let Some(typaxis_syntax::ProductionInlineReference::Anchor {
                    target,
                    target_owner,
                    format: typaxis_syntax::ProductionReferenceFormat::Page,
                }) = site.reference()
                else {
                    return None;
                };
                Some((|| {
                    let index = self
                        .selected
                        .destinations
                        .binary_search_by(|binding| {
                            binding.destination.anchor_id.as_str().cmp(target)
                        })
                        .map_err(|_| BookNavigationSelectedError::DestinationMismatch)?;
                    let binding = &self.selected.destinations[index];
                    if binding.source_node_id != target_owner {
                        return Err(BookNavigationSelectedError::DestinationMismatch);
                    }
                    let page = binding
                        .destination
                        .page_index
                        .checked_add(1)
                        .ok_or(BookNavigationSelectedError::DestinationOutOfBounds)?;
                    Ok((site.owner(), page))
                })())
            })
    }
    pub fn child_language_paints(&self) -> &[BookLanguagePaintV2] {
        &self.child_language_paints
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
        profile: &StagingBookNavigationProfileAuthorizationV2,
        admitted: &AdmittedResourceLedger,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), BookNavigationSelectedError> {
        if !std::ptr::eq(self.source, source)
            || self.selected.profile_sha256 != profile.profile_receipt_fingerprint()
            || self.selected.limits_sha256 != limits.fingerprint()
        {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        source
            .structure()
            .verify(source.structure().display(), admitted, limits)
            .map_err(|_| BookNavigationSelectedError::ReceiptMismatch)
    }
}

pub fn seal_production_footnote_book_navigation<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(
    inputs: ProductionFootnoteBookNavigationInputs<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    profile: &StagingBookNavigationProfileAuthorizationV2,
    admitted: &AdmittedResourceLedger,
    limits: &M4EffectiveResourceLimits,
) -> Result<
    ProductionFootnoteBookNavigation<'i, 'n, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    BookNavigationSelectedError,
> {
    use BookNavigationSelectedError as E;
    inputs.verify(inputs.source, admitted, limits)?;
    let display = inputs.source.structure().display();
    let navigation = display.source().line_layout().source_flow().navigation();
    if navigation.limits() != limits
        || profile.metadata_sha256() != navigation.metadata().fingerprint()
        || profile.language_sha256() != navigation.languages().fingerprint()
        || profile.outline_sha256() != navigation.outline().fingerprint()
        || profile.limits_sha256() != limits.fingerprint()
    {
        return Err(E::ProfileMismatch);
    }
    let record_charge = inputs
        .record_charge
        .checked_add(
            (navigation.outline().entries().len() as u64)
                .checked_mul(4)
                .ok_or(E::FragmentLimit)?,
        )
        .and_then(|n| n.checked_add((inputs.destinations.len() as u64).checked_mul(2)?))
        .and_then(|n| n.checked_add(8))
        .ok_or(E::FragmentLimit)?;
    if record_charge > limits.base().get().max_fragments {
        return Err(E::FragmentLimit);
    }
    let mut spool_charge = inputs.spool_charge;
    for entry in navigation.outline().entries() {
        for value in [
            entry.label.as_str(),
            entry.destination.as_str(),
            entry.source.computed_language.as_str(),
        ] {
            spool_charge = spool_charge
                .checked_add(value.len() as u64)
                .ok_or(E::SpoolLimit)?;
            if spool_charge > limits.base().get().max_spool_bytes {
                return Err(E::SpoolLimit);
            }
        }
    }
    let fragment_count = display
        .source()
        .geometry()
        .pages()
        .iter()
        .try_fold(0u64, |n, p| n.checked_add(p.fragments().len() as u64))
        .ok_or(E::FragmentLimit)?;
    let registry = validate_destinations_v2(navigation, &inputs.destinations, &inputs.pages)?;
    let entries = resolve_entries_v2(navigation, &registry, fragment_count, limits.base())?;
    let destinations = encode_book_bounded(&mut spool_charge, limits, |out| {
        write_destination_registry(out, &inputs.destinations)
    })?;
    let mut selected = BookNavigationSelectedReceiptV2 {
        metadata_sha256: navigation.metadata().fingerprint(),
        language_sha256: navigation.languages().fingerprint(),
        outline_sha256: navigation.outline().fingerprint(),
        profile_sha256: profile.profile_receipt_fingerprint(),
        limits_sha256: limits.fingerprint(),
        selected_layout_sha256: display.fingerprint(),
        selected_layout_fragment_count: fragment_count,
        destination_registry_sha256: sha256(destinations.as_bytes()),
        vector_display_sha256: display.fingerprint(),
        pages: inputs.pages,
        destinations: inputs.destinations,
        entries,
        language_paints: inputs.language_paints,
        vector_paints: inputs.vector_paints,
        links: inputs.links,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    let hashes = [
        sha256(
            encode_book_bounded(&mut spool_charge, limits, |out| {
                write_entries_v2(out, &selected.entries)
            })?
            .as_bytes(),
        ),
        sha256(
            encode_book_bounded(&mut spool_charge, limits, |out| {
                write_language_paints_v2(out, &selected.language_paints)
            })?
            .as_bytes(),
        ),
        sha256(
            encode_book_bounded(&mut spool_charge, limits, |out| {
                write_links_v2(out, &selected.links)
            })?
            .as_bytes(),
        ),
        sha256(
            encode_book_bounded(&mut spool_charge, limits, |out| {
                write_pages_v2(out, &selected.pages)
            })?
            .as_bytes(),
        ),
        sha256(
            encode_book_bounded(&mut spool_charge, limits, |out| {
                write_vector_paints_v2(out, &selected.vector_paints)
            })?
            .as_bytes(),
        ),
    ];
    selected.canonical_jcs = encode_book_bounded(&mut spool_charge, limits, |out| {
        write_selected_v2(out, &selected, &hashes)
    })?;
    selected.fingerprint = sha256(selected.canonical_jcs.as_bytes());
    Ok(ProductionFootnoteBookNavigation {
        source: inputs.source,
        selected,
        child_language_paints: inputs.child_language_paints,
        record_charge,
        spool_charge,
    })
}
