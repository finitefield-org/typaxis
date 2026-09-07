//! FD-bound Type2 execution over the original program spans. These inspection
//! results are not selected-glyph closure, font admission, or PDF authorization.
use super::*;
use std::cell::Cell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffGlyphFailureReasonV2 {
    Execution,
    InvalidWidth,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CffGlyphFailureV2 {
    pub kind: Cff1Error,
    pub reason: CffGlyphFailureReasonV2,
    pub gid: u16,
    pub fd: Option<u8>,
    pub table_offset: Option<usize>,
    pub operator: Option<u16>,
}
impl std::fmt::Display for CffGlyphFailureV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CFF /2 {:?}: GID {}, FD {:?}, {:?}",
            self.reason, self.gid, self.fd, self.kind
        )
    }
}
impl std::error::Error for CffGlyphFailureV2 {}
#[derive(Clone, Copy)]
enum ProgramKindV2 {
    Glyph(u16),
    Local { fd: u8, index: usize },
    Global { index: usize },
}
struct SelectedFontDict<'a> {
    program: &'a CffProgramV2,
    fd: u8,
    current: Cell<Option<(usize, u16)>>,
    invalid_width: Cell<bool>,
    source_width: Cell<Option<i32>>,
}
impl SelectedFontDict<'_> {
    fn span(&self, kind: ProgramKind) -> Option<ProgramSpan> {
        let kind = match kind {
            ProgramKind::Glyph(gid) => ProgramKindV2::Glyph(gid),
            ProgramKind::Local(index) => ProgramKindV2::Local { fd: self.fd, index },
            ProgramKind::Global(index) => ProgramKindV2::Global { index },
        };
        match kind {
            ProgramKindV2::Glyph(gid) => self.program.charstrings.get(usize::from(gid)),
            ProgramKindV2::Local { fd, index } => self
                .program
                .font_dicts
                .get(usize::from(fd))?
                .local_subrs
                .get(index),
            ProgramKindV2::Global { index } => self.program.global_subrs.get(index),
        }
        .copied()
    }
}
impl Type2ProgramAccess for SelectedFontDict<'_> {
    fn accepts_subroutine_endchar(&self) -> bool {
        true
    }
    fn observe(&self, kind: ProgramKind, position: usize, operator: u16) {
        self.current.set(
            self.span(kind)
                .and_then(|s| s.start.checked_add(position))
                .map(|p| (p, operator)),
        );
    }
    fn program_bytes(&self, kind: ProgramKind) -> Result<&[u8], Cff1Error> {
        self.span(kind)
            .map(|s| s.bytes(&self.program.source))
            .ok_or(Cff1Error::InvalidCharstring)
    }
    fn local_subroutine_count(&self) -> usize {
        self.program.font_dicts[usize::from(self.fd)]
            .local_subrs
            .len()
    }
    fn global_subroutine_count(&self) -> usize {
        self.program.global_subrs.len()
    }
    fn validate_width(&self, operand: Option<i32>) -> Result<(), Cff1Error> {
        let fd = &self.program.font_dicts[usize::from(self.fd)];
        let width = match operand {
            Some(v) => fd.nominal_width_x.checked_add(v),
            None => Some(fd.default_width_x),
        };
        let Some(width) = width else {
            self.invalid_width.set(true);
            return Err(Cff1Error::InvalidCharstring);
        };
        // OpenType advance is hmtx; the CFF/PostScript width can differ.
        self.source_width.set(Some(width));
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CffOutlineCommandV2 {
    Move(i32, i32),
    Line(i32, i32),
    Cubic(i32, i32, i32, i32, i32, i32),
    Close,
}
/// Coordinates and control-point bounds are 16.16 font units. Bounds are the
/// control-point hull used by the existing evaluator, not tight Bezier extrema.
#[derive(Debug)]
pub struct CffEvaluatedGlyphV2 {
    gid: u16,
    fd: u8,
    advance: u16,
    outline: EvaluatedGlyph,
    source_width_fixed: i32,
}
impl CffEvaluatedGlyphV2 {
    pub const fn gid(&self) -> u16 {
        self.gid
    }
    pub const fn fd(&self) -> u8 {
        self.fd
    }
    pub const fn source_width_fixed(&self) -> i32 {
        self.source_width_fixed
    }
    pub const fn advance(&self) -> u16 {
        self.advance
    }
    pub const fn control_bounds(&self) -> Option<[i32; 4]> {
        self.outline.bbox
    }
    pub fn commands(&self) -> impl ExactSizeIterator<Item = CffOutlineCommandV2> + '_ {
        self.outline.segments.iter().map(|s| match *s {
            OutlineSegment::Move(x, y) => CffOutlineCommandV2::Move(x, y),
            OutlineSegment::Line(x, y) => CffOutlineCommandV2::Line(x, y),
            OutlineSegment::Cubic(a, b, c, d, e, f) => CffOutlineCommandV2::Cubic(a, b, c, d, e, f),
            OutlineSegment::Close => CffOutlineCommandV2::Close,
        })
    }
}
/// One finite inspection-work budget across all evaluated glyphs. No implicit
/// per-glyph reset or uncharged cache. A later admitted subset owner must bind
/// source/face/profile identity and selected-glyph caching before publication.
#[derive(Debug)]
pub struct CffProgramEvaluationSessionV2 {
    limits: M4ResourceLimits,
    operations: u64,
    segments: u64,
}
impl CffProgramEvaluationSessionV2 {
    pub fn new(limits: &M4EffectiveResourceLimits) -> Self {
        Self {
            limits: *limits.extension().get(),
            operations: 0,
            segments: 0,
        }
    }
    pub const fn operations_used(&self) -> u64 {
        self.operations
    }
    pub const fn outline_segments_used(&self) -> u64 {
        self.segments
    }
    pub fn evaluate(
        &mut self,
        inspection: &CffProgramInspectionV2,
        gid: u16,
        advance: u16,
    ) -> Result<CffEvaluatedGlyphV2, CffGlyphFailureV2> {
        let program = &inspection.program;
        let fd = program
            .fd_by_gid
            .get(usize::from(gid))
            .copied()
            .ok_or(CffGlyphFailureV2 {
                kind: Cff1Error::InvalidSelectedGlyph,
                reason: CffGlyphFailureReasonV2::Execution,
                gid,
                fd: None,
                table_offset: None,
                operator: None,
            })?;
        let selected = SelectedFontDict {
            program,
            fd,
            current: Cell::new(None),
            invalid_width: Cell::new(false),
            source_width: Cell::new(None),
        };
        let outline = evaluate_type2(&selected, gid, self).map_err(|kind| CffGlyphFailureV2 {
            kind,
            reason: if selected.invalid_width.get() {
                CffGlyphFailureReasonV2::InvalidWidth
            } else {
                CffGlyphFailureReasonV2::Execution
            },
            gid,
            fd: Some(fd),
            table_offset: selected.current.get().map(|v| v.0),
            operator: selected.current.get().map(|v| v.1),
        })?;
        let source_width_fixed = selected.source_width.get().ok_or(CffGlyphFailureV2 {
            kind: Cff1Error::InvalidCharstring,
            reason: CffGlyphFailureReasonV2::InvalidWidth,
            gid,
            fd: Some(fd),
            table_offset: selected.current.get().map(|v| v.0),
            operator: selected.current.get().map(|v| v.1),
        })?;
        Ok(CffEvaluatedGlyphV2 {
            source_width_fixed,
            gid,
            fd,
            advance,
            outline,
        })
    }
}
impl Type2WorkBudget for CffProgramEvaluationSessionV2 {
    fn charge_operation(&mut self) -> Result<(), Cff1Error> {
        let next = self
            .operations
            .checked_add(1)
            .filter(|v| *v <= self.limits.max_cff_charstring_operations)
            .ok_or(Cff1Error::CharstringOperationLimit)?;
        self.operations = next;
        Ok(())
    }
    fn charge_segment(&mut self) -> Result<(), Cff1Error> {
        let next = self
            .segments
            .checked_add(1)
            .filter(|v| *v <= self.limits.max_cff_outline_segments)
            .ok_or(Cff1Error::OutlineSegmentLimit)?;
        self.segments = next;
        Ok(())
    }
}
