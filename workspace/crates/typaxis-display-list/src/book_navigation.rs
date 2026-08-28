use std::collections::{BTreeMap, BTreeSet};

use typaxis_core::{
    push_jcs_string, sha256, AnchorId, NodeId, ValidatedResourceLimits, JSON_SAFE_INTEGER_MAX,
};
use typaxis_document::StagingLanguageNodeKind;
use typaxis_syntax::{StagingBookNavigationProfileAuthorization, ValidatedStagingBookNavigation};

use crate::{DestinationView, NamedDestination};

pub const BOOK_NAVIGATION_SELECTED_ALGORITHM: &str = "typaxis.book-navigation-selected/1";
pub const BOOK_DESTINATION_REGISTRY_ALGORITHM: &str = "typaxis.book-destination-registry/1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BookNavigationSelectedPage {
    pub page_index: u32,
    pub width_raw: i64,
    pub height_raw: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationDestinationBinding {
    pub source_node_id: NodeId,
    pub frame_id: u32,
    pub destination: NamedDestination,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookLanguagePaintInput {
    pub owner_node_id: NodeId,
    pub occurrence: u32,
    pub page_index: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookInternalLinkInput {
    pub owner_node_id: NodeId,
    pub page_index: u32,
    pub destination: AnchorId,
    pub x_raw: i64,
    pub y_raw: i64,
    pub width_raw: i64,
    pub height_raw: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationSelectedEntry {
    outline_id: u32,
    parent_outline_id: Option<u32>,
    level: u8,
    label: String,
    source_node_id: NodeId,
    source_language: String,
    destination: NamedDestination,
    frame_id: u32,
}

impl BookNavigationSelectedEntry {
    pub const fn outline_id(&self) -> u32 {
        self.outline_id
    }
    pub const fn parent_outline_id(&self) -> Option<u32> {
        self.parent_outline_id
    }
    pub const fn level(&self) -> u8 {
        self.level
    }
    pub fn label(&self) -> &str {
        &self.label
    }
    pub const fn source_node_id(&self) -> NodeId {
        self.source_node_id
    }
    pub fn source_language(&self) -> &str {
        &self.source_language
    }
    pub const fn destination(&self) -> &NamedDestination {
        &self.destination
    }
    pub const fn frame_id(&self) -> u32 {
        self.frame_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookLanguagePaint {
    occurrence: u32,
    owner_node_id: NodeId,
    page_index: u32,
    language: String,
}

impl BookLanguagePaint {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub fn language(&self) -> &str {
        &self.language
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookInternalLink {
    owner_node_id: NodeId,
    page_index: u32,
    destination: AnchorId,
    x_raw: i64,
    y_raw: i64,
    width_raw: i64,
    height_raw: i64,
}

impl BookInternalLink {
    pub const fn owner_node_id(&self) -> NodeId {
        self.owner_node_id
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn destination(&self) -> &AnchorId {
        &self.destination
    }
    pub const fn x_raw(&self) -> i64 {
        self.x_raw
    }
    pub const fn y_raw(&self) -> i64 {
        self.y_raw
    }
    pub const fn width_raw(&self) -> i64 {
        self.width_raw
    }
    pub const fn height_raw(&self) -> i64 {
        self.height_raw
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookNavigationSelectedReceipt {
    metadata_sha256: [u8; 32],
    language_sha256: [u8; 32],
    outline_sha256: [u8; 32],
    profile_sha256: [u8; 32],
    limits_sha256: [u8; 32],
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    destination_registry_sha256: [u8; 32],
    pages: Vec<BookNavigationSelectedPage>,
    destinations: Vec<BookNavigationDestinationBinding>,
    entries: Vec<BookNavigationSelectedEntry>,
    language_paints: Vec<BookLanguagePaint>,
    links: Vec<BookInternalLink>,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl BookNavigationSelectedReceipt {
    pub const fn metadata_sha256(&self) -> [u8; 32] {
        self.metadata_sha256
    }
    pub const fn language_sha256(&self) -> [u8; 32] {
        self.language_sha256
    }
    pub const fn outline_sha256(&self) -> [u8; 32] {
        self.outline_sha256
    }
    pub const fn profile_sha256(&self) -> [u8; 32] {
        self.profile_sha256
    }
    pub const fn limits_sha256(&self) -> [u8; 32] {
        self.limits_sha256
    }
    pub const fn selected_layout_sha256(&self) -> [u8; 32] {
        self.selected_layout_sha256
    }
    pub const fn selected_layout_fragment_count(&self) -> u64 {
        self.selected_layout_fragment_count
    }
    pub const fn destination_registry_sha256(&self) -> [u8; 32] {
        self.destination_registry_sha256
    }
    pub fn pages(&self) -> &[BookNavigationSelectedPage] {
        &self.pages
    }
    pub fn destinations(&self) -> &[BookNavigationDestinationBinding] {
        &self.destinations
    }
    pub fn entries(&self) -> &[BookNavigationSelectedEntry] {
        &self.entries
    }
    pub fn language_paints(&self) -> &[BookLanguagePaint] {
        &self.language_paints
    }
    pub fn links(&self) -> &[BookInternalLink] {
        &self.links
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }

    pub fn verify(
        &self,
        navigation: &ValidatedStagingBookNavigation,
        profile: &StagingBookNavigationProfileAuthorization,
        limits: &ValidatedResourceLimits,
    ) -> Result<(), BookNavigationSelectedError> {
        if self.metadata_sha256 != navigation.metadata().fingerprint()
            || self.language_sha256 != navigation.languages().fingerprint()
            || self.outline_sha256 != navigation.outline().fingerprint()
            || self.profile_sha256 != profile.profile_receipt_fingerprint()
            || self.limits_sha256 != profile.limits_sha256()
            || navigation.limits() != limits
            || profile.metadata_sha256() != self.metadata_sha256
            || profile.language_sha256() != self.language_sha256
            || profile.outline_sha256() != self.outline_sha256
        {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        validate_pages(&self.pages, limits)?;
        let registry = validate_destinations(navigation, &self.destinations, &self.pages)?;
        if self.destination_registry_sha256
            != sha256(encode_destination_registry(&self.destinations).as_bytes())
        {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        let expected_entries = resolve_entries(
            navigation,
            &registry,
            self.selected_layout_fragment_count,
            limits,
        )?;
        if self.entries != expected_entries {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        validate_paints(
            navigation,
            &self.pages,
            &self
                .language_paints
                .iter()
                .map(|paint| BookLanguagePaintInput {
                    owner_node_id: paint.owner_node_id,
                    occurrence: paint.occurrence,
                    page_index: paint.page_index,
                })
                .collect::<Vec<_>>(),
        )
        .and_then(|value| {
            if value == self.language_paints {
                Ok(value)
            } else {
                Err(BookNavigationSelectedError::ReceiptMismatch)
            }
        })?;
        validate_links(
            navigation,
            &self
                .links
                .iter()
                .map(|link| BookInternalLinkInput {
                    owner_node_id: link.owner_node_id,
                    page_index: link.page_index,
                    destination: link.destination.clone(),
                    x_raw: link.x_raw,
                    y_raw: link.y_raw,
                    width_raw: link.width_raw,
                    height_raw: link.height_raw,
                })
                .collect::<Vec<_>>(),
            &self.pages,
            &registry,
        )
        .and_then(|value| {
            if value == self.links {
                Ok(value)
            } else {
                Err(BookNavigationSelectedError::ReceiptMismatch)
            }
        })?;
        let canonical = encode_selected(self);
        if canonical != self.canonical_jcs || sha256(canonical.as_bytes()) != self.fingerprint {
            return Err(BookNavigationSelectedError::ReceiptMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookNavigationSelectedError {
    ProfileMismatch,
    NonCanonicalPage,
    DestinationMismatch,
    DestinationOutOfBounds,
    FragmentLimit,
    InvalidLanguagePaint,
    InvalidLink,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for BookNavigationSelectedError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ProfileMismatch => formatter.write_str("L5100: book-navigation profile mismatch"),
            Self::NonCanonicalPage => formatter.write_str("L5100: selected page geometry mismatch"),
            Self::DestinationMismatch => {
                formatter.write_str("L5100: selected destination mismatch")
            }
            Self::DestinationOutOfBounds => {
                formatter.write_str("L5100: selected destination is outside its page")
            }
            Self::FragmentLimit => {
                formatter.write_str("L5110: selected outline fragment limit exceeded")
            }
            Self::InvalidLanguagePaint => {
                formatter.write_str("L5100: selected language paint mismatch")
            }
            Self::InvalidLink => formatter.write_str("L5100: selected internal link mismatch"),
            Self::ReceiptMismatch => {
                formatter.write_str("I9190: selected book-navigation receipt mismatch")
            }
            Self::AllocationFailure => {
                formatter.write_str("L5110: selected book-navigation allocation failed")
            }
        }
    }
}

impl std::error::Error for BookNavigationSelectedError {}

#[allow(clippy::too_many_arguments)]
pub fn select_staging_book_navigation(
    navigation: &ValidatedStagingBookNavigation,
    profile: &StagingBookNavigationProfileAuthorization,
    limits: &ValidatedResourceLimits,
    selected_layout_sha256: [u8; 32],
    selected_layout_fragment_count: u64,
    pages: &[BookNavigationSelectedPage],
    destinations: &[BookNavigationDestinationBinding],
    language_paints: &[BookLanguagePaintInput],
    links: &[BookInternalLinkInput],
) -> Result<BookNavigationSelectedReceipt, BookNavigationSelectedError> {
    if navigation.limits() != limits
        || profile.metadata_sha256() != navigation.metadata().fingerprint()
        || profile.language_sha256() != navigation.languages().fingerprint()
        || profile.outline_sha256() != navigation.outline().fingerprint()
    {
        return Err(BookNavigationSelectedError::ProfileMismatch);
    }
    validate_pages(pages, limits)?;
    let registry = validate_destinations(navigation, destinations, pages)?;
    let entries = resolve_entries(
        navigation,
        &registry,
        selected_layout_fragment_count,
        limits,
    )?;
    let language_paints = validate_paints(navigation, pages, language_paints)?;
    let links = validate_links(navigation, links, pages, &registry)?;
    let destination_registry_jcs = encode_destination_registry(destinations);
    let mut receipt = BookNavigationSelectedReceipt {
        metadata_sha256: navigation.metadata().fingerprint(),
        language_sha256: navigation.languages().fingerprint(),
        outline_sha256: navigation.outline().fingerprint(),
        profile_sha256: profile.profile_receipt_fingerprint(),
        limits_sha256: profile.limits_sha256(),
        selected_layout_sha256,
        selected_layout_fragment_count,
        destination_registry_sha256: sha256(destination_registry_jcs.as_bytes()),
        pages: pages.to_vec(),
        destinations: destinations.to_vec(),
        entries,
        language_paints,
        links,
        canonical_jcs: String::new(),
        fingerprint: [0; 32],
    };
    receipt.canonical_jcs = encode_selected(&receipt);
    receipt.fingerprint = sha256(receipt.canonical_jcs.as_bytes());
    receipt.verify(navigation, profile, limits)?;
    Ok(receipt)
}

fn validate_pages(
    pages: &[BookNavigationSelectedPage],
    limits: &ValidatedResourceLimits,
) -> Result<(), BookNavigationSelectedError> {
    if pages.is_empty()
        || u32::try_from(pages.len()).is_err()
        || u32::try_from(pages.len()).is_ok_and(|count| count > limits.get().max_pages)
        || pages.iter().enumerate().any(|(index, page)| {
            usize::try_from(page.page_index) != Ok(index)
                || page.width_raw <= 0
                || page.height_raw <= 0
                || page.width_raw > JSON_SAFE_INTEGER_MAX
                || page.height_raw > JSON_SAFE_INTEGER_MAX
        })
    {
        return Err(BookNavigationSelectedError::NonCanonicalPage);
    }
    Ok(())
}

fn validate_destinations<'a>(
    navigation: &ValidatedStagingBookNavigation,
    destinations: &'a [BookNavigationDestinationBinding],
    pages: &[BookNavigationSelectedPage],
) -> Result<BTreeMap<&'a AnchorId, &'a BookNavigationDestinationBinding>, BookNavigationSelectedError>
{
    let mut registry = BTreeMap::new();
    let mut previous: Option<&AnchorId> = None;
    let mut owners = BTreeSet::new();
    for binding in destinations {
        let destination = &binding.destination;
        if previous.is_some_and(|prior| prior >= &destination.anchor_id)
            || registry.insert(&destination.anchor_id, binding).is_some()
            || !owners.insert(binding.source_node_id)
            || navigation.anchor_owner(&destination.anchor_id) != Some(binding.source_node_id)
        {
            return Err(BookNavigationSelectedError::DestinationMismatch);
        }
        previous = Some(&destination.anchor_id);
        let page = pages
            .get(destination.page_index as usize)
            .ok_or(BookNavigationSelectedError::DestinationMismatch)?;
        if !matches!(destination.view, DestinationView::Xyz { .. })
            || !view_within_page(&destination.view, page)
        {
            return Err(BookNavigationSelectedError::DestinationOutOfBounds);
        }
    }
    if registry.len() != navigation.anchors().len()
        || navigation.anchors().iter().any(|(anchor, owner)| {
            registry.get(anchor).map(|binding| binding.source_node_id) != Some(*owner)
        })
    {
        return Err(BookNavigationSelectedError::DestinationMismatch);
    }
    Ok(registry)
}

fn view_within_page(view: &DestinationView, page: &BookNavigationSelectedPage) -> bool {
    match view {
        DestinationView::Xyz { point } => {
            (0..=page.width_raw).contains(&point.x.raw())
                && (0..=page.height_raw).contains(&point.y.raw())
        }
        DestinationView::FitPage => true,
        DestinationView::FitWidth { top } => top
            .as_ref()
            .map_or(true, |top| (0..=page.height_raw).contains(&top.raw())),
    }
}

fn resolve_entries(
    navigation: &ValidatedStagingBookNavigation,
    registry: &BTreeMap<&AnchorId, &BookNavigationDestinationBinding>,
    selected_layout_fragment_count: u64,
    limits: &ValidatedResourceLimits,
) -> Result<Vec<BookNavigationSelectedEntry>, BookNavigationSelectedError> {
    let outline_count = u64::try_from(navigation.outline().entries().len())
        .map_err(|_| BookNavigationSelectedError::FragmentLimit)?;
    if selected_layout_fragment_count
        .checked_add(outline_count)
        .map_or(true, |count| count > limits.get().max_fragments)
    {
        return Err(BookNavigationSelectedError::FragmentLimit);
    }
    let mut output = Vec::new();
    output
        .try_reserve_exact(navigation.outline().entries().len())
        .map_err(|_| BookNavigationSelectedError::AllocationFailure)?;
    for entry in navigation.outline().entries() {
        let binding = registry
            .get(&entry.destination)
            .copied()
            .ok_or(BookNavigationSelectedError::DestinationMismatch)?;
        if binding.source_node_id != entry.source.node_id {
            return Err(BookNavigationSelectedError::DestinationMismatch);
        }
        output.push(BookNavigationSelectedEntry {
            outline_id: entry.outline_id,
            parent_outline_id: entry.parent_outline_id,
            level: entry.level,
            label: entry.label.clone(),
            source_node_id: entry.source.node_id,
            source_language: entry.source.computed_language.clone(),
            destination: binding.destination.clone(),
            frame_id: binding.frame_id,
        });
    }
    Ok(output)
}

fn validate_paints(
    navigation: &ValidatedStagingBookNavigation,
    pages: &[BookNavigationSelectedPage],
    inputs: &[BookLanguagePaintInput],
) -> Result<Vec<BookLanguagePaint>, BookNavigationSelectedError> {
    let mut output = Vec::new();
    let mut owners = BTreeSet::new();
    let mut next_occurrence = BTreeMap::new();
    let required_owners = navigation
        .languages()
        .records()
        .iter()
        .filter(|record| {
            record.effective_language.as_ref() != navigation.languages().document_language()
                && matches!(
                    record.node_kind,
                    StagingLanguageNodeKind::Text
                        | StagingLanguageNodeKind::Reference
                        | StagingLanguageNodeKind::FootnoteReference
                        | StagingLanguageNodeKind::InlineMath
                        | StagingLanguageNodeKind::DisplayMath
                )
        })
        .map(|record| record.node_id)
        .collect::<BTreeSet<_>>();
    let mut previous = None;
    for input in inputs {
        let record = navigation
            .languages()
            .record(input.owner_node_id)
            .ok_or(BookNavigationSelectedError::InvalidLanguagePaint)?;
        let expected_occurrence = next_occurrence.entry(input.owner_node_id).or_insert(0u32);
        if input.page_index as usize >= pages.len()
            || record.effective_language.as_ref() == navigation.languages().document_language()
            || input.occurrence != *expected_occurrence
            || previous.is_some_and(|prior| {
                prior >= (input.page_index, input.owner_node_id, input.occurrence)
            })
        {
            return Err(BookNavigationSelectedError::InvalidLanguagePaint);
        }
        *expected_occurrence = expected_occurrence
            .checked_add(1)
            .ok_or(BookNavigationSelectedError::FragmentLimit)?;
        owners.insert(input.owner_node_id);
        previous = Some((input.page_index, input.owner_node_id, input.occurrence));
        output.push(BookLanguagePaint {
            occurrence: input.occurrence,
            owner_node_id: input.owner_node_id,
            page_index: input.page_index,
            language: record.effective_language.to_string(),
        });
    }
    if owners != required_owners {
        return Err(BookNavigationSelectedError::InvalidLanguagePaint);
    }
    Ok(output)
}

fn validate_links(
    navigation: &ValidatedStagingBookNavigation,
    inputs: &[BookInternalLinkInput],
    pages: &[BookNavigationSelectedPage],
    registry: &BTreeMap<&AnchorId, &BookNavigationDestinationBinding>,
) -> Result<Vec<BookInternalLink>, BookNavigationSelectedError> {
    let mut output = Vec::new();
    let mut owners = BTreeSet::new();
    let mut previous = None;
    for input in inputs {
        let page = pages
            .get(input.page_index as usize)
            .ok_or(BookNavigationSelectedError::InvalidLink)?;
        let right = input.x_raw.checked_add(input.width_raw);
        let bottom = input.y_raw.checked_add(input.height_raw);
        owners.insert(input.owner_node_id);
        if !registry.contains_key(&input.destination)
            || navigation.internal_link_target(input.owner_node_id) != Some(&input.destination)
            || previous.is_some_and(|prior| {
                prior
                    >= (
                        input.page_index,
                        input.owner_node_id,
                        input.x_raw,
                        input.y_raw,
                    )
            })
            || input.width_raw <= 0
            || input.height_raw <= 0
            || input.x_raw < 0
            || input.y_raw < 0
            || right.map_or(true, |value| value > page.width_raw)
            || bottom.map_or(true, |value| value > page.height_raw)
        {
            return Err(BookNavigationSelectedError::InvalidLink);
        }
        previous = Some((
            input.page_index,
            input.owner_node_id,
            input.x_raw,
            input.y_raw,
        ));
        output.push(BookInternalLink {
            owner_node_id: input.owner_node_id,
            page_index: input.page_index,
            destination: input.destination.clone(),
            x_raw: input.x_raw,
            y_raw: input.y_raw,
            width_raw: input.width_raw,
            height_raw: input.height_raw,
        });
    }
    if owners
        != navigation
            .internal_links()
            .iter()
            .map(|(owner, _)| *owner)
            .collect::<BTreeSet<_>>()
    {
        return Err(BookNavigationSelectedError::InvalidLink);
    }
    Ok(output)
}

fn encode_destination_registry(destinations: &[BookNavigationDestinationBinding]) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, BOOK_DESTINATION_REGISTRY_ALGORITHM);
    output.push_str(",\"destinations\":[");
    for (index, binding) in destinations.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"anchor_id\":");
        push_jcs_string(&mut output, binding.destination.anchor_id.as_str());
        output.push_str(",\"frame_id\":");
        output.push_str(&binding.frame_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&binding.destination.page_index.to_string());
        output.push_str(",\"source_node_id\":");
        output.push_str(&binding.source_node_id.get().to_string());
        output.push_str(",\"view\":");
        push_view(&mut output, &binding.destination.view);
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn encode_selected(value: &BookNavigationSelectedReceipt) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, BOOK_NAVIGATION_SELECTED_ALGORITHM);
    output.push_str(",\"destination_registry_sha256\":");
    push_hash(&mut output, value.destination_registry_sha256);
    output.push_str(",\"entries\":[");
    for (index, entry) in value.entries.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, entry.destination.anchor_id.as_str());
        output.push_str(",\"frame_id\":");
        output.push_str(&entry.frame_id.to_string());
        output.push_str(",\"label\":");
        push_jcs_string(&mut output, &entry.label);
        output.push_str(",\"level\":");
        output.push_str(&entry.level.to_string());
        output.push_str(",\"outline_id\":");
        output.push_str(&entry.outline_id.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&entry.destination.page_index.to_string());
        output.push_str(",\"parent_outline_id\":");
        if let Some(parent) = entry.parent_outline_id {
            output.push_str(&parent.to_string());
        } else {
            output.push_str("null");
        }
        output.push_str(",\"source_language\":");
        push_jcs_string(&mut output, &entry.source_language);
        output.push_str(",\"source_node_id\":");
        output.push_str(&entry.source_node_id.get().to_string());
        output.push_str(",\"view\":");
        push_view(&mut output, &entry.destination.view);
        output.push('}');
    }
    output.push_str("],\"language_paints\":[");
    for (index, paint) in value.language_paints.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"language\":");
        push_jcs_string(&mut output, &paint.language);
        output.push_str(",\"occurrence\":");
        output.push_str(&paint.occurrence.to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&paint.owner_node_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&paint.page_index.to_string());
        output.push('}');
    }
    output.push_str("],\"language_sha256\":");
    push_hash(&mut output, value.language_sha256);
    output.push_str(",\"limits_sha256\":");
    push_hash(&mut output, value.limits_sha256);
    output.push_str(",\"links\":[");
    for (index, link) in value.links.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"destination\":");
        push_jcs_string(&mut output, link.destination.as_str());
        output.push_str(",\"height\":");
        output.push_str(&link.height_raw.to_string());
        output.push_str(",\"owner_node_id\":");
        output.push_str(&link.owner_node_id.get().to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&link.page_index.to_string());
        output.push_str(",\"width\":");
        output.push_str(&link.width_raw.to_string());
        output.push_str(",\"x\":");
        output.push_str(&link.x_raw.to_string());
        output.push_str(",\"y\":");
        output.push_str(&link.y_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"metadata_sha256\":");
    push_hash(&mut output, value.metadata_sha256);
    output.push_str(",\"outline_sha256\":");
    push_hash(&mut output, value.outline_sha256);
    output.push_str(",\"pages\":[");
    for (index, page) in value.pages.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push_str("{\"height\":");
        output.push_str(&page.height_raw.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&page.page_index.to_string());
        output.push_str(",\"width\":");
        output.push_str(&page.width_raw.to_string());
        output.push('}');
    }
    output.push_str("],\"profile_sha256\":");
    push_hash(&mut output, value.profile_sha256);
    output.push_str(",\"selected_layout_fragment_count\":");
    output.push_str(&value.selected_layout_fragment_count.to_string());
    output.push_str(",\"selected_layout_sha256\":");
    push_hash(&mut output, value.selected_layout_sha256);
    output.push('}');
    output
}

fn push_view(output: &mut String, view: &DestinationView) {
    match view {
        DestinationView::Xyz { point } => {
            output.push_str("{\"kind\":\"xyz\",\"x\":");
            output.push_str(&point.x.raw().to_string());
            output.push_str(",\"y\":");
            output.push_str(&point.y.raw().to_string());
            output.push('}');
        }
        DestinationView::FitPage => output.push_str("{\"kind\":\"fit_page\"}"),
        DestinationView::FitWidth { top } => {
            output.push_str("{\"kind\":\"fit_width\",\"top\":");
            if let Some(top) = top {
                output.push_str(&top.raw().to_string());
            } else {
                output.push_str("null");
            }
            output.push('}');
        }
    }
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
    use typaxis_core::{Length, Point, ResourceLimits};
    use typaxis_syntax::machine_profile_boundary::wire::{
        DocumentPackageDecodePolicy, StagingSemanticDocumentPackageDecoder,
    };
    use typaxis_syntax::{
        validate_staging_book_navigation, StagingSemanticPackageParser,
        ValidatedStagingSemanticPackage,
    };

    const FIXTURE: &[u8] = include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../samples/machine-package/staging/production-book-1/book-navigation/job/document-package.json"
    ));
    const SCALE: i64 = 65_536;

    struct SelectionFixture {
        navigation: ValidatedStagingBookNavigation,
        profile: StagingBookNavigationProfileAuthorization,
        limits: ValidatedResourceLimits,
        pages: Vec<BookNavigationSelectedPage>,
        destinations: Vec<BookNavigationDestinationBinding>,
        paints: Vec<BookLanguagePaintInput>,
        links: Vec<BookInternalLinkInput>,
        _package: ValidatedStagingSemanticPackage,
    }

    fn fixture(max_fragments: u64) -> SelectionFixture {
        let raw_limits = ResourceLimits {
            max_fragments,
            ..ResourceLimits::default()
        };
        let limits = ValidatedResourceLimits::new(raw_limits).unwrap();
        let decoded = StagingSemanticDocumentPackageDecoder::new()
            .decode(FIXTURE, &DocumentPackageDecodePolicy::new(&limits))
            .unwrap();
        let package = StagingSemanticPackageParser::new()
            .parse(decoded, &limits)
            .unwrap();
        let navigation = validate_staging_book_navigation(&package, &limits).unwrap();
        let profile = StagingBookNavigationProfileAuthorization::bind_profile_receipt(
            typaxis_syntax::StagingBookNavigationProfileView::new(&package, &navigation, &limits)
                .unwrap(),
            sha256(b"test-book-navigation-profile"),
            &package,
            &navigation,
            &limits,
        )
        .unwrap();
        let pages = vec![
            BookNavigationSelectedPage {
                page_index: 0,
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            },
            BookNavigationSelectedPage {
                page_index: 1,
                width_raw: 1_000 * SCALE,
                height_raw: 800 * SCALE,
            },
        ];
        let destination =
            |anchor: &str, source: u32, frame_id: u32, page_index: u32, x: i64, y: i64| {
                BookNavigationDestinationBinding {
                    source_node_id: NodeId::new(source),
                    frame_id,
                    destination: NamedDestination {
                        anchor_id: AnchorId::new(anchor).unwrap(),
                        page_index,
                        view: DestinationView::Xyz {
                            point: Point {
                                x: Length::from_raw(x * SCALE).unwrap(),
                                y: Length::from_raw(y * SCALE).unwrap(),
                            },
                        },
                    },
                }
            };
        SelectionFixture {
            navigation,
            profile,
            limits,
            pages,
            destinations: vec![
                destination("chapter-1", 2, 1, 0, 100, 700),
                destination("exercise-1", 7, 2, 1, 100, 700),
                destination("part-1", 1, 0, 0, 0, 800),
            ],
            paints: vec![
                BookLanguagePaintInput {
                    owner_node_id: NodeId::new(3),
                    occurrence: 0,
                    page_index: 0,
                },
                BookLanguagePaintInput {
                    owner_node_id: NodeId::new(6),
                    occurrence: 0,
                    page_index: 0,
                },
                BookLanguagePaintInput {
                    owner_node_id: NodeId::new(9),
                    occurrence: 0,
                    page_index: 1,
                },
            ],
            links: vec![BookInternalLinkInput {
                owner_node_id: NodeId::new(5),
                page_index: 0,
                destination: AnchorId::new("chapter-1").unwrap(),
                x_raw: 100 * SCALE,
                y_raw: 650 * SCALE,
                width_raw: 60 * SCALE,
                height_raw: 20 * SCALE,
            }],
            _package: package,
        }
    }

    fn select(
        fixture: &SelectionFixture,
        selected_layout_fragment_count: u64,
    ) -> Result<BookNavigationSelectedReceipt, BookNavigationSelectedError> {
        select_staging_book_navigation(
            &fixture.navigation,
            &fixture.profile,
            &fixture.limits,
            sha256(b"selected-book-layout"),
            selected_layout_fragment_count,
            &fixture.pages,
            &fixture.destinations,
            &fixture.paints,
            &fixture.links,
        )
    }

    #[test]
    fn book_navigation_selected_fragment_limit_is_combined_and_inclusive() {
        let fixture = fixture(6);
        assert!(select(&fixture, 3).is_ok());
        assert_eq!(
            select(&fixture, 4),
            Err(BookNavigationSelectedError::FragmentLimit)
        );
    }

    #[test]
    fn book_navigation_selected_registries_are_bidirectionally_closed() {
        let mut missing_destination = fixture(6);
        missing_destination.destinations.pop();
        assert_eq!(
            select(&missing_destination, 3),
            Err(BookNavigationSelectedError::DestinationMismatch)
        );

        let mut missing_paint = fixture(6);
        missing_paint.paints.pop();
        assert_eq!(
            select(&missing_paint, 3),
            Err(BookNavigationSelectedError::InvalidLanguagePaint)
        );

        let mut unbound_view = fixture(6);
        unbound_view.destinations[0].destination.view = DestinationView::FitPage;
        assert_eq!(
            select(&unbound_view, 3),
            Err(BookNavigationSelectedError::DestinationOutOfBounds)
        );

        let mut oversized_page = fixture(6);
        oversized_page.pages[0].width_raw = JSON_SAFE_INTEGER_MAX + 1;
        assert_eq!(
            select(&oversized_page, 3),
            Err(BookNavigationSelectedError::NonCanonicalPage)
        );
    }
}
