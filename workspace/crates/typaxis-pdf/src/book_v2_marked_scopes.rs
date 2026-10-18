//! Source-owned marked-content boundaries over actual ordered book-2 paints.
//! These boundaries do not replace the structure tree or paint commands.
use super::*;
use typaxis_core::NodeId;
use typaxis_display_list::book_v2::{
    BookV2BodyDisplay, BookV2BodyPaintIndex, BookV2ColumnDisplay, BookV2ColumnPaintIndex,
    BookV2ResourceDisplay, BookV2SemanticDisplay,
};

pub const BOOK_V2_MARKED_SCOPES_ALGORITHM: &str = "typaxis.book-2-marked-scopes/1";
pub use typaxis_display_list::book_v2::{
    BookV2SemanticArtifact as BookV2MarkedArtifact, BookV2SemanticKind as BookV2MarkedRole,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Key {
    owner: Option<NodeId>,
    page: u32,
    fragment: usize,
    role: BookV2MarkedRole,
    artifact: Option<BookV2MarkedArtifact>,
}
struct Properties<'a> {
    key: Key,
    alternative: Option<&'a str>,
    actual: Option<&'a str>,
}
fn properties<'borrow, 'a, D: BookV2SemanticDisplay<'a>>(
    display: &'borrow D,
    index: usize,
) -> Result<Properties<'borrow>, E> {
    let p = display
        .semantic_paint(index)
        .map_err(|_| E::Identity)?
        .ok_or(E::Identity)?;
    if p.kind() == BookV2MarkedRole::Formula && p.actual_text().is_none() {
        return Err(E::Identity);
    }
    Ok(Properties {
        key: Key {
            owner: p.owner(),
            page: p.page_index(),
            fragment: p.fragment_index(),
            role: p.kind(),
            artifact: p.artifact(),
        },
        alternative: p.alternative(),
        actual: p.actual_text(),
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BookV2MarkedScope {
    key: Key,
    paints: Range<usize>,
    mcid: Option<u32>,
    begin: Range<usize>,
}
impl BookV2MarkedScope {
    pub fn owner(&self) -> Option<NodeId> {
        self.key.owner
    }
    pub fn page_index(&self) -> u32 {
        self.key.page
    }
    pub fn fragment_index(&self) -> usize {
        self.key.fragment
    }
    pub fn role(&self) -> BookV2MarkedRole {
        self.key.role
    }
    pub fn artifact(&self) -> Option<BookV2MarkedArtifact> {
        self.key.artifact
    }
    pub fn mcid(&self) -> Option<u32> {
        self.mcid
    }
    pub fn paints(&self) -> Range<usize> {
        self.paints.clone()
    }
}
// All group walking is linear. Text clusters may coalesce only within the
// same actual owner/fragment/role; formula and number never share a scope.
fn visit<'a, D: BookV2SemanticDisplay<'a>>(
    display: &D,
    budget: &mut Budget,
    mut emit: impl FnMut(&mut Budget, Properties<'_>, Range<usize>, Option<u32>) -> Result<(), E>,
) -> Result<usize, E> {
    let pages = display.page_count();
    let mut start = 0;
    let mut previous_page = None;
    let mut next_mcid = 0u32;
    let mut count = 0usize;
    while start < display.paint_count() {
        budget.step(1)?;
        let p = properties(display, start)?;
        if p.key.page as usize >= pages || previous_page.is_some_and(|page| page > p.key.page) {
            return Err(E::Identity);
        }
        if previous_page != Some(p.key.page) {
            next_mcid = 0;
            previous_page = Some(p.key.page);
        }
        let mut end = start + 1;
        if p.key.role == BookV2MarkedRole::Text {
            while end < display.paint_count() {
                budget.step(1)?;
                if properties(display, end)?.key != p.key {
                    break;
                }
                end += 1;
            }
        }
        let mcid = if p.key.artifact.is_some() {
            None
        } else {
            let id = next_mcid;
            next_mcid = next_mcid.checked_add(1).ok_or(E::Records)?;
            Some(id)
        };
        emit(budget, p, start..end, mcid)?;
        start = end;
        count = count.checked_add(1).ok_or(E::Records)?;
    }
    Ok(count)
}
fn begin(p: &Properties<'_>, mcid: Option<u32>, out: &mut Encoder<'_>) -> Result<(), E> {
    if let Some(artifact) = p.key.artifact {
        return out.extend(match artifact {
            BookV2MarkedArtifact::RepeatedHeader | BookV2MarkedArtifact::RunningHeader => {
                b"/Artifact << /Type /Pagination /Subtype /Header >> BDC\n"
            }
            BookV2MarkedArtifact::FootnoteSeparator => b"/Artifact << /Type /Layout >> BDC\n",
            BookV2MarkedArtifact::RunningFooter => {
                b"/Artifact << /Type /Pagination /Subtype /Footer >> BDC\n"
            }
        });
    }
    out.extend(match p.key.role {
        BookV2MarkedRole::Text | BookV2MarkedRole::EquationNumber => b"/Span << /MCID ",
        BookV2MarkedRole::Label => b"/Lbl << /MCID ",
        BookV2MarkedRole::Formula => b"/Formula << /MCID ",
        BookV2MarkedRole::Figure => b"/Figure << /MCID ",
        BookV2MarkedRole::Separator => return Err(E::Identity),
    })?;
    out.unsigned(u64::from(mcid.ok_or(E::Identity)?))?;
    out.extend(b" >> BDC\n")?;
    if let Some(actual) = p.actual {
        // Keep the semantic Formula/Figure and its MCID on the outer scope.
        // Poppler applies replacement text to Span, including invisible vector
        // anchors, but ignores the same property on Formula/Figure tags.
        out.extend(b"/Span << /ActualText ")?;
        font_encoding::utf16(out, actual.chars(), true)?;
        out.extend(b" >> BDC\n")?;
    }
    Ok(())
}
pub struct BookV2MarkedScopesData<
    'm,
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    D: BookV2SemanticDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'m BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>,
    groups: Vec<BookV2MarkedScope>,
    pages: Vec<Range<usize>>,
    bytes: Vec<u8>,
    fingerprint: [u8; 32],
    budget: Budget,
}
pub type BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2MarkedScopesData<
        'm,
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2BodyPaintIndex,
    >;
/// Column semantic ownership stays in every parent.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2MarkedScopes, BookV2ColumnMarkedScopes};
/// fn body<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(value: BookV2ColumnMarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> BookV2MarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { value }
/// ```
pub type BookV2ColumnMarkedScopes<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2MarkedScopesData<
        'm,
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2ColumnPaintIndex,
    >;
impl<'m, 'k, 'j, 'r, 'i, 'v, 'a, D: BookV2SemanticDisplay<'a, Paint = P>, P: Copy + Eq>
    BookV2MarkedScopesData<'m, 'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    pub fn source(&self) -> &'m BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P> {
        self.source
    }
    pub fn display(&self) -> &'v D {
        self.source.source().source().source().source().display()
    }
    pub fn groups(&self) -> &[BookV2MarkedScope] {
        &self.groups
    }
    pub fn page_groups(&self, page: u32) -> Option<&[BookV2MarkedScope]> {
        self.groups.get(self.pages.get(page as usize)?.clone())
    }
    pub fn for_paint(&self, paint: usize) -> Option<&BookV2MarkedScope> {
        let i = self.groups.partition_point(|g| g.paints.end <= paint);
        self.groups.get(i).filter(|g| g.paints.contains(&paint))
    }
    pub fn begin_bytes(&self, group: usize) -> Option<&[u8]> {
        self.bytes.get(self.groups.get(group)?.begin.clone())
    }
    pub fn end_bytes(&self, group: usize) -> Option<&'static [u8]> {
        self.groups.get(group)?;
        Some(if self.actual_text(group).is_some() {
            b"EMC\nEMC\n"
        } else {
            b"EMC\n"
        })
    }
    /// Source Alt belongs to the eventual StructElem, never to a shared Form.
    /// Repeated display-only copies have no semantic alternative or replacement.
    pub fn alternative(&self, group: usize) -> Option<&str> {
        let g = self.groups.get(group)?;
        if g.artifact().is_some() {
            return None;
        }
        properties(
            self.source.source().source().source().source().display(),
            g.paints.start,
        )
        .ok()?
        .alternative
    }
    pub fn actual_text(&self, group: usize) -> Option<&str> {
        let g = self.groups.get(group)?;
        if g.artifact().is_some() {
            return None;
        }
        properties(
            self.source.source().source().source().source().display(),
            g.paints.start,
        )
        .ok()?
        .actual
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
}
pub struct BookV2MarkedScopeBuilderData<
    'm,
    'k,
    'j,
    'r,
    'i,
    'v,
    'a,
    D: BookV2SemanticDisplay<'a>,
    P: Copy = <D as BookV2ResourceDisplay<'a>>::Paint,
> {
    source: &'m BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>,
    pub(super) budget: Budget,
}
pub type BookV2MarkedScopeBuilder<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2MarkedScopeBuilderData<
        'm,
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2BodyPaintIndex,
    >;
/// Column semantic ownership stays in every parent.
///
/// ```compile_fail
/// use typaxis_pdf::book_v2::{BookV2MarkedScopeBuilder, BookV2ColumnMarkedScopeBuilder};
/// fn body<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>(value: BookV2ColumnMarkedScopeBuilder<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a>) -> BookV2MarkedScopeBuilder<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> { value }
/// ```
pub type BookV2ColumnMarkedScopeBuilder<'m, 'k, 'j, 'r, 'i, 'v, 'd, 'g, 'q, 'b, 'f, 's, 'p, 'a> =
    BookV2MarkedScopeBuilderData<
        'm,
        'k,
        'j,
        'r,
        'i,
        'v,
        'a,
        BookV2ColumnDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
        BookV2ColumnPaintIndex,
    >;
impl<'m, 'k, 'j, 'r, 'i, 'v, 'a, D: BookV2SemanticDisplay<'a, Paint = P>, P: Copy + Eq>
    BookV2MarkedScopeBuilderData<'m, 'k, 'j, 'r, 'i, 'v, 'a, D, P>
{
    pub fn new(
        source: &'m BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
    ) -> Result<Self, E> {
        Self::new_counted(
            source,
            limits,
            max_work,
            prior_records,
            prior_spool,
            prior_output,
            prior_work,
            &mut 0,
            &mut 0,
            &mut 0,
            &mut 0,
        )
    }
    /// Keep every accepted constructor reservation visible even on failure.
    pub fn new_counted(
        source: &'m BookV2ImageCommandsData<'k, 'j, 'r, 'i, 'v, 'a, D, P>,
        limits: &M4EffectiveResourceLimits,
        max_work: u64,
        prior_records: u64,
        prior_spool: u64,
        prior_output: u64,
        prior_work: u64,
        observed_records: &mut u64,
        observed_spool: &mut u64,
        observed_output: &mut u64,
        observed_work: &mut u64,
    ) -> Result<Self, E> {
        *observed_records = prior_records.max(source.record_charge());
        *observed_spool = prior_spool.max(source.spool_charge());
        *observed_output = prior_output.max(source.output_charge());
        *observed_work = prior_work.max(source.work_steps());

        let inherited = BookV2ImageCommandBuilderData::new_counted(
            source.source(),
            limits,
            max_work,
            prior_records.max(source.record_charge()),
            prior_spool.max(source.spool_charge()),
            prior_output.max(source.output_charge()),
            prior_work.max(source.work_steps()),
            observed_records,
            observed_spool,
            observed_output,
            observed_work,
        )?;
        Ok(Self {
            source,
            budget: inherited.budget,
        })
    }
    pub fn record_charge(&self) -> u64 {
        self.budget.records
    }
    pub fn spool_charge(&self) -> u64 {
        self.budget.spool
    }
    pub fn output_charge(&self) -> u64 {
        self.budget.output
    }
    pub fn work_steps(&self) -> u64 {
        self.budget.work
    }
    pub fn build(&mut self) -> Result<BookV2MarkedScopesData<'m, 'k, 'j, 'r, 'i, 'v, 'a, D, P>, E> {
        let display = self.source.source().source().source().source().display();
        let mut closing = 0usize;
        let count = visit(display, &mut self.budget, |_, p, _, _| {
            closing = closing
                .checked_add(if p.key.artifact.is_none() && p.actual.is_some() {
                    8
                } else {
                    4
                })
                .ok_or(E::Output)?;
            Ok(())
        })?;
        let npages = display.page_count();
        let records = count
            .checked_add(npages)
            .and_then(|n| n.checked_add(1))
            .ok_or(E::Records)?;
        let metadata = count
            .checked_mul(std::mem::size_of::<BookV2MarkedScope>())
            .and_then(|n| n.checked_add(npages.checked_mul(std::mem::size_of::<Range<usize>>())?))
            .ok_or(E::Spool)?;
        // Each closing EMC will be emitted once even though its static bytes
        // need no retained allocation in this boundary contribution.
        let output_limit = (self.budget.max_output - self.budget.output)
            .checked_sub(closing as u64)
            .ok_or(E::Output)?;
        let spool_limit = (self.budget.max_spool - self.budget.spool)
            .checked_sub(metadata as u64)
            .ok_or(E::Spool)?;
        let mut length = 0usize;
        visit(display, &mut self.budget, |budget, p, _, mcid| {
            let mut out = Encoder {
                budget,
                bytes: None,
                length,
                output_limit,
                spool_limit,
            };
            begin(&p, mcid, &mut out)?;
            length = out.length;
            Ok(())
        })?;
        self.budget.reserve(
            records,
            metadata.checked_add(length).ok_or(E::Spool)?,
            length.checked_add(closing).ok_or(E::Output)?,
        )?;
        self.budget.step(1)?;
        let mut groups = Vec::new();
        let mut pages = Vec::new();
        let mut bytes = Vec::new();
        groups.try_reserve_exact(count).map_err(|_| E::Allocation)?;
        pages.try_reserve_exact(npages).map_err(|_| E::Allocation)?;
        bytes.try_reserve_exact(length).map_err(|_| E::Allocation)?;
        visit(display, &mut self.budget, |budget, p, paints, mcid| {
            let start = bytes.len();
            let mut out = Encoder {
                budget,
                bytes: Some(&mut bytes),
                length: start,
                output_limit: length as u64,
                spool_limit: length as u64,
            };
            begin(&p, mcid, &mut out)?;
            groups.push(BookV2MarkedScope {
                key: p.key,
                paints,
                mcid,
                begin: start..out.length,
            });
            Ok(())
        })?;
        if groups.len() != count || bytes.len() != length {
            return Err(E::Identity);
        }
        let mut cursor = 0;
        for page in 0..npages {
            self.budget.step(1)?;
            let start = cursor;
            while groups
                .get(cursor)
                .is_some_and(|g| g.key.page as usize == page)
            {
                self.budget.step(1)?;
                cursor += 1;
            }
            pages.push(start..cursor);
        }
        if cursor != count {
            return Err(E::Identity);
        }
        self.budget.step(length.div_ceil(64) + 1)?;
        let mut fingerprint = sha256(BOOK_V2_MARKED_SCOPES_ALGORITHM.as_bytes());
        fingerprint = self.budget.fold(fingerprint, &self.source.fingerprint())?;
        fingerprint = self.budget.fold(fingerprint, &sha256(&bytes))?;
        for g in &groups {
            let mut b = [0u8; 51];
            b[0] = u8::from(g.key.owner.is_some());
            b[1..5].copy_from_slice(&g.key.owner.map_or(0, |n| n.get()).to_be_bytes());
            b[5..9].copy_from_slice(&g.key.page.to_be_bytes());
            b[9..17].copy_from_slice(&(g.key.fragment as u64).to_be_bytes());
            b[17] = g.key.role as u8;
            b[18] = g.key.artifact.map_or(0, |a| 1 + a as u8);
            b[19..27].copy_from_slice(&(g.paints.start as u64).to_be_bytes());
            b[27..35].copy_from_slice(&(g.paints.end as u64).to_be_bytes());
            b[35..43].copy_from_slice(&(g.begin.start as u64).to_be_bytes());
            b[43..51].copy_from_slice(&(g.begin.end as u64).to_be_bytes());
            fingerprint = self.budget.fold(fingerprint, &b)?;
        }
        fingerprint = self
            .budget
            .fold(fingerprint, &(npages as u64).to_be_bytes())?;
        Ok(BookV2MarkedScopesData {
            source: self.source,
            groups,
            pages,
            bytes,
            fingerprint,
            budget: self.budget,
        })
    }
}
