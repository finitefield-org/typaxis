//! Original /1 program positions and detailed selected-glyph failure. The broad
//! entry points retain their historical error type and successful receipts.
use super::*;
use std::cell::Cell;

pub(super) fn program_offsets(objects: &[Vec<u8>], start: usize) -> Result<Vec<usize>, Cff1Error> {
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| Cff1Error::InvalidCff)?;
    let mut cursor = start;
    for object in objects {
        offsets.push(cursor);
        cursor = cursor
            .checked_add(object.len())
            .ok_or(Cff1Error::InvalidCff)?;
    }
    Ok(offsets)
}

struct ObservedGlyph<'a> {
    admission: &'a Cff1Admission,
    context: Cell<FontFailureContext>,
}
impl ObservedGlyph<'_> {
    fn position(&self, kind: ProgramKind, position: usize, end: bool, operator: Option<u16>) {
        let offsets = match kind {
            ProgramKind::Glyph(_) => &self.admission.program.charstring_offsets,
            ProgramKind::Local(_) => &self.admission.program.local_subr_offsets,
            ProgramKind::Global(_) => &self.admission.program.global_subr_offsets,
        };
        let index = match kind {
            ProgramKind::Glyph(gid) => usize::from(gid),
            ProgramKind::Local(index) | ProgramKind::Global(index) => index,
        };
        let mut context = self.context.get();
        context.clear_position();
        context.operator = operator;
        if let (Some(start), Ok(bytes)) = (offsets.get(index), self.program_bytes(kind)) {
            if (end && position == bytes.len()) || (!end && position < bytes.len()) {
                if let Some(offset) = start.checked_add(position) {
                    context.at(offset);
                    context.position_is_exact = !end;
                    context.position_is_end = end;
                }
            }
        }
        self.context.set(context);
    }
}
impl Type2ProgramAccess for ObservedGlyph<'_> {
    fn observe(&self, kind: ProgramKind, position: usize, operator: Option<u16>) {
        self.position(kind, position, false, operator);
    }
    fn observe_end(&self, kind: ProgramKind, position: usize) {
        self.position(kind, position, true, None);
    }
    fn reject_operator(&self, reason: Type2OperatorRejection) {
        let mut context = self.context.get();
        context.reason = match reason {
            Type2OperatorRejection::Unsupported => FontFailureReason::UnsupportedCffOperator,
            Type2OperatorRejection::Reserved => FontFailureReason::ReservedCffOperator,
        };
        self.context.set(context);
    }
    fn program_bytes(&self, kind: ProgramKind) -> Result<&[u8], Cff1Error> {
        program_bytes(&self.admission.program, kind)
    }
    fn local_subroutine_count(&self) -> usize {
        self.admission.program.local_subrs.len()
    }
    fn global_subroutine_count(&self) -> usize {
        self.admission.program.global_subrs.len()
    }
    fn validate_width(&self, operand: Option<i32>) -> Result<(), Cff1Error> {
        validate_source_width(self.admission, operand).inspect_err(|_| {
            let mut context = self.context.get();
            context.reason = FontFailureReason::InvalidCharstringWidth;
            self.context.set(context);
        })
    }
}

impl Cff1Admission {
    fn selection_failure(&self, kind: Cff1Error, phase: FontFailurePhase) -> Cff1Failure {
        let mut context = self.glyph_context;
        context.phase = phase;
        context.clear_position();
        context.reason = match kind {
            Cff1Error::ReceiptMismatch | Cff1Error::InvalidGlyphClosure => {
                FontFailureReason::Invariant
            }
            Cff1Error::SelectedGlyphLimit | Cff1Error::SubsetByteLimit => {
                FontFailureReason::BudgetExceeded
            }
            Cff1Error::InvalidSelectedGlyph => FontFailureReason::InvalidSelectedGlyph,
            _ => FontFailureReason::InvalidTable,
        };
        // No source field is blamed for selection or output-encoding failures.
        context.table_tag = None;
        Cff1Failure { kind, context }
    }
}
impl Cff1SubsetSession {
    pub fn prepare_face_detailed(
        &mut self,
        admission: &Cff1Admission,
        face: FontFaceId,
        selected: &BTreeSet<OriginalGlyphId>,
    ) -> Result<(), Cff1Failure> {
        self.require_admission(admission)
            .map_err(|e| admission.selection_failure(e, FontFailurePhase::Subset))?;
        self.evaluate_face_gid_detailed(admission, face, OriginalGlyphId::new(0))?;
        for gid in selected {
            self.evaluate_face_gid_detailed(admission, face, *gid)?;
        }
        Ok(())
    }
    pub fn subset_detailed(
        &mut self,
        admission: &Cff1Admission,
        face: FontFaceId,
        instance: FontInstanceId,
        selected: &BTreeSet<OriginalGlyphId>,
        max_cids_per_font: u16,
    ) -> Result<Cff1Subset, Cff1Failure> {
        self.require_admission(admission)
            .map_err(|e| admission.selection_failure(e, FontFailurePhase::Subset))?;
        let closure =
            Self::close_instance_selection(admission, face, instance, selected, max_cids_per_font)
                .map_err(|e| admission.selection_failure(e, FontFailurePhase::Subset))?;
        for gid in closure.source_gids() {
            self.evaluate_face_gid_detailed(admission, face, *gid)?;
        }
        let mut context = admission
            .selection_failure(Cff1Error::InvalidSubset, FontFailurePhase::Subset)
            .context;
        write_subset(
            admission,
            closure,
            &self.evaluated,
            self.limits.max_font_subset_bytes,
            &mut context,
        )
        .map_err(|kind| {
            context.reason = admission
                .selection_failure(kind, FontFailurePhase::Subset)
                .context
                .reason;
            Cff1Failure { kind, context }
        })
    }
    fn evaluate_face_gid_detailed(
        &mut self,
        admission: &Cff1Admission,
        face: FontFaceId,
        gid: OriginalGlyphId,
    ) -> Result<(), Cff1Failure> {
        let mut context = admission.glyph_context;
        context.gid = Some(u32::from(gid.get()));
        if u32::from(gid.get()) >= admission.glyph_count {
            context.reason = FontFailureReason::InvalidSelectedGlyph;
            return Err(Cff1Failure {
                kind: Cff1Error::InvalidSelectedGlyph,
                context,
            });
        }
        let key = (face, admission.source_sha256, gid.get());
        if !self.evaluated.contains_key(&key) {
            let observed = ObservedGlyph {
                admission,
                context: Cell::new(context),
            };
            let outline = evaluate_type2(&observed, gid.get(), self).map_err(|kind| {
                let mut context = observed.context.get();
                let budget = match kind {
                    Cff1Error::CharstringOperationLimit => Some((
                        self.limits.max_cff_charstring_operations,
                        self.operations_used,
                    )),
                    Cff1Error::OutlineSegmentLimit => Some((
                        self.limits.max_cff_outline_segments,
                        self.outline_segments_used,
                    )),
                    _ => None,
                };
                if let Some((limit, used)) = budget {
                    context.reason = FontFailureReason::BudgetExceeded;
                    context.limit = Some(limit);
                    context.observed = used.checked_add(1);
                }
                Cff1Failure { kind, context }
            })?;
            self.evaluated.insert(key, outline);
        }
        Ok(())
    }
}
