//! Number-stage storage and observed backend scratch share the Source ledger.
use super::*;
use crate::shared_shape_storage as shared;
use std::fmt::{self, Write};

struct NumberShapeStorage<'call, 'id, 'cells> {
    accounting: &'call mut BookV2ObservedSourceAccounting<'id, 'cells>,
    owner: NodeId,
    error: Option<BookV2ObservedAccountingError>,
}
impl NumberShapeStorage<'_, '_, '_> {
    fn accepted(
        &mut self,
        result: Result<(), BookV2ObservedAccountingError>,
    ) -> Result<(), LinkedShaperError> {
        result.map_err(|error| {
            self.error.get_or_insert(error);
            LinkedShaperError::AllocationFailure
        })
    }
}
impl shared::Storage for NumberShapeStorage<'_, '_, '_> {
    fn reserve<T>(&mut self, _: shared::Site, count: usize) -> Result<(), LinkedShaperError> {
        let bytes = count
            .checked_mul(std::mem::size_of::<T>())
            .and_then(|n| u64::try_from(n).ok());
        let result = match bytes {
            Some(bytes) => self.accounting.debit(self.owner, [0, bytes, count as u64]),
            None => Err(BookV2ObservedAccountingError::Spool { owner: self.owner }),
        };
        self.accepted(result)
    }
    fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
        let result = self.accounting.debit_records(self.owner, count);
        self.accepted(result)
    }
    fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
        let result = self.accounting.debit_work(self.owner, count);
        self.accepted(result)
    }
}

pub(super) fn select_font_observed<'a>(
    fonts: BodyFonts<'a>,
    families: &[String],
    owner: NodeId,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(FontFaceId, crate::production_text::inputs::BodyFont<'a>), BookV2EquationNumberError> {
    let mut storage = NumberShapeStorage {
        accounting,
        owner,
        error: None,
    };
    let result = fonts.select_with_storage(families, &mut storage);
    if let Some(error) = storage.error {
        return Err(failure(
            owner,
            BookV2EquationNumberErrorKind::ObservedAccounting(error),
        ));
    }
    result.map_err(|error| {
        failure(
            owner,
            match error {
                ProductionTextShapeErrorKind::MissingSelectedFont => {
                    BookV2EquationNumberErrorKind::MissingSelectedFont
                }
                ProductionTextShapeErrorKind::Backend(error) => {
                    BookV2EquationNumberErrorKind::Shape(StagingEquationNumberShapeError::Backend(
                        error,
                    ))
                }
                _ => BookV2EquationNumberErrorKind::ReceiptMismatch,
            },
        )
    })
}

pub(super) fn coverage_observed(
    font: crate::production_text::inputs::BodyFont<'_>,
    text: &str,
    owner: NodeId,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<(), BookV2EquationNumberError> {
    let mut storage = NumberShapeStorage {
        accounting,
        owner,
        error: None,
    };
    let result = font.coverage_with_storage(text, &mut storage);
    if let Some(error) = storage.error {
        return Err(failure(
            owner,
            BookV2EquationNumberErrorKind::ObservedAccounting(error),
        ));
    }
    result.map_err(|e| failure(owner, BookV2EquationNumberErrorKind::Coverage(e)))
}

pub(super) fn shape_runs_observed(
    input: EquationNumberRunInput<'_>,
    owner: NodeId,
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<
    (
        BidiLevel,
        Vec<StagingEquationNumberGlyphRun>,
        PositiveLength,
    ),
    BookV2EquationNumberError,
> {
    let mut storage = NumberShapeStorage {
        accounting,
        owner,
        error: None,
    };
    let result = shared::shape_equation_number_runs_with_storage(input, &mut storage);
    if let Some(error) = storage.error {
        return Err(failure(
            owner,
            BookV2EquationNumberErrorKind::ObservedAccounting(error),
        ));
    }
    result.map_err(|error| failure(owner, BookV2EquationNumberErrorKind::Shape(error)))
}

fn failure(owner: NodeId, kind: BookV2EquationNumberErrorKind) -> BookV2EquationNumberError {
    BookV2EquationNumberError { owner, kind }
}
pub(super) fn pay(
    accounting: &mut Option<&mut BookV2ObservedSourceAccounting<'_, '_>>,
    owner: NodeId,
    request: [u64; 3],
) -> Result<(), BookV2EquationNumberError> {
    if let Some(a) = accounting.as_deref_mut() {
        a.debit(owner, request)
            .map_err(|e| failure(owner, BookV2EquationNumberErrorKind::ObservedAccounting(e)))?;
    }
    Ok(())
}
pub(super) fn pay_vector<T>(
    accounting: &mut Option<&mut BookV2ObservedSourceAccounting<'_, '_>>,
    owner: NodeId,
    count: usize,
) -> Result<(), BookV2EquationNumberError> {
    if accounting.is_none() {
        return Ok(());
    }
    let bytes = count
        .checked_mul(std::mem::size_of::<T>())
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| failure(owner, BookV2EquationNumberErrorKind::SpoolLimit))?;
    pay(accounting, owner, [0, bytes, count as u64])
}

struct Count(usize);
impl Write for Count {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.checked_add(text.len()).ok_or(fmt::Error)?;
        Ok(())
    }
}
struct Reserved<'a> {
    text: &'a mut String,
    length: usize,
}
impl Write for Reserved<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        let next = self.text.len().checked_add(text.len()).ok_or(fmt::Error)?;
        if next > self.length || next > self.text.capacity() {
            return Err(fmt::Error);
        }
        self.text.push_str(text);
        Ok(())
    }
}

pub(super) fn glyph_digest(
    owner: NodeId,
    runs: &[StagingEquationNumberGlyphRun],
    accounting: &mut BookV2ObservedSourceAccounting<'_, '_>,
) -> Result<[u8; 32], BookV2EquationNumberError> {
    let mismatch = || failure(owner, BookV2EquationNumberErrorKind::ReceiptMismatch);
    // The caller has paid the run/glyph/cluster traversal before this counting
    // pass. Integer formatting uses fmt's stack storage, not to_string buffers.
    let mut count = Count(0);
    write_receipt(&mut count, runs)
        .map_err(|_| failure(owner, BookV2EquationNumberErrorKind::SpoolLimit))?;
    let bytes = u64::try_from(count.0)
        .map_err(|_| failure(owner, BookV2EquationNumberErrorKind::SpoolLimit))?;
    let work = bytes
        .checked_mul(2)
        .ok_or_else(|| failure(owner, BookV2EquationNumberErrorKind::OutputLimit))?;
    accounting
        .debit(owner, [0, bytes, work])
        .map_err(|e| failure(owner, BookV2EquationNumberErrorKind::ObservedAccounting(e)))?;
    let mut canonical = String::new();
    canonical
        .try_reserve_exact(count.0)
        .map_err(|_| failure(owner, BookV2EquationNumberErrorKind::AllocationFailure))?;
    write_receipt(
        &mut Reserved {
            text: &mut canonical,
            length: count.0,
        },
        runs,
    )
    .map_err(|_| mismatch())?;
    if canonical.len() != count.0 {
        return Err(mismatch());
    }
    Ok(sha256(canonical.as_bytes()))
}

fn text_span(output: &mut impl Write, value: TextSpan) -> fmt::Result {
    write!(
        output,
        "{{\"end_byte\":{},\"start_byte\":{},\"text_id\":{}}}",
        value.end_byte().get(),
        value.start_byte().get(),
        value.text_id().get()
    )
}
fn source_span(output: &mut impl Write, value: ShapeSourceSpan) -> fmt::Result {
    match value {
        ShapeSourceSpan::Parsed(value) => text_span(output, value),
        ShapeSourceSpan::Generated(_) => output.write_str("null"),
    }
}
fn jcs_string(output: &mut impl Write, value: &str) -> fmt::Result {
    output.write_char('"')?;
    for c in value.chars() {
        match c {
            '"' => output.write_str("\\\"")?,
            '\\' => output.write_str("\\\\")?,
            '\u{8}' => output.write_str("\\b")?,
            '\u{9}' => output.write_str("\\t")?,
            '\u{a}' => output.write_str("\\n")?,
            '\u{c}' => output.write_str("\\f")?,
            '\u{d}' => output.write_str("\\r")?,
            c if c < '\u{20}' => write!(output, "\\u{:04x}", u32::from(c))?,
            c => output.write_char(c)?,
        }
    }
    output.write_char('"')
}
fn write_receipt(output: &mut impl Write, runs: &[StagingEquationNumberGlyphRun]) -> fmt::Result {
    output.write_str("{\"algorithm\":")?;
    jcs_string(output, EQUATION_NUMBER_GLYPH_RECEIPT_ALGORITHM)?;
    output.write_str(",\"runs\":[")?;
    for (ri, run) in runs.iter().enumerate() {
        if ri != 0 {
            output.write_char(',')?;
        }
        write!(
            output,
            "{{\"bidi_level\":{},\"clusters\":[",
            run.bidi_level.get()
        )?;
        for (ci, cluster) in run.clusters.iter().enumerate() {
            if ci != 0 {
                output.write_char(',')?;
            }
            write!(
                output,
                "{{\"glyph_end\":{},\"glyph_start\":{},\"source_span\":",
                cluster.glyph_end, cluster.glyph_start
            )?;
            source_span(output, cluster.source_span)?;
            output.write_char('}')?;
        }
        output.write_str("],\"glyphs\":[")?;
        for (gi, glyph) in run.glyphs.iter().enumerate() {
            if gi != 0 {
                output.write_char(',')?;
            }
            write!(output, "{{\"advance_x\":{},\"advance_y\":{},\"offset_x\":{},\"offset_y\":{},\"original_gid\":{}}}",
                glyph.advance_x.raw(), glyph.advance_y.raw(), glyph.offset_x.raw(),
                glyph.offset_y.raw(), glyph.original_gid.get())?;
        }
        write!(output, "],\"run_id\":{},\"script\":", run.run_id.get())?;
        // Real itemized runs use the four ASCII bytes of a Unicode script tag.
        let script = run.script.bytes();
        jcs_string(
            output,
            std::str::from_utf8(&script).map_err(|_| fmt::Error)?,
        )?;
        output.write_str(",\"source_span\":")?;
        text_span(output, run.source_span)?;
        output.write_char('}')?;
    }
    output.write_str("]}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_output_source_number_keeps_owner_and_atomic_prefix_at_each_work_request() {
        struct Fees(Vec<[u64; 3]>);
        impl shared::Storage for Fees {
            fn reserve<T>(&mut self, _: shared::Site, n: usize) -> Result<(), LinkedShaperError> {
                self.0
                    .push([0, (n * std::mem::size_of::<T>()) as u64, n as u64]);
                Ok(())
            }
            fn records(&mut self, n: u64) -> Result<(), LinkedShaperError> {
                self.0.push([n, 0, 0]);
                Ok(())
            }
            fn work(&mut self, n: u64) -> Result<(), LinkedShaperError> {
                self.0.push([0, 0, n]);
                Ok(())
            }
        }
        const FONT: &[u8] = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
        ));
        let input = || shared::tests::input(FONT, "Result");
        let mut fees = Fees(Vec::new());
        let ordinary = shape_equation_number_runs(input()).unwrap();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            ordinary
        );
        let owner = NodeId::new(91);
        let limits = ValidatedResourceLimits::new(typaxis_core::ResourceLimits::default()).unwrap();
        let run = |maximum_work| {
            let [mut r, mut s, mut w] = [7, 11, 13];
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                100_000,
                maximum_work,
                |a| shape_runs_observed(input(), owner, a),
            );
            (result, [r, s, w])
        };
        let full = run(1_000_000);
        assert_eq!(full.0.unwrap(), ordinary);
        let mut prefix = [7, 11, 13];
        let mut cutoffs = 0;
        for fee in fees.0 {
            if fee[0] == 0 && fee[1] == 0 && fee[2] > 0 {
                let actual = run(prefix[2]);
                assert_eq!(
                    actual.0.unwrap_err(),
                    BookV2EquationNumberError {
                        owner,
                        kind: BookV2EquationNumberErrorKind::ObservedAccounting(
                            BookV2ObservedAccountingError::Work { owner }
                        )
                    }
                );
                assert_eq!(actual.1, prefix);
                cutoffs += 1;
            }
            for (total, delta) in prefix.iter_mut().zip(fee) {
                *total += delta;
            }
        }
        assert_eq!(prefix, full.1);
        eprintln!(
            "real number Source work cutoffs={cutoffs}, owner=91, full={:?}",
            full.1
        );
    }
    use typaxis_core::{ResourceLimits, TextBufferId, Utf8ByteOffset, ValidatedResourceLimits};

    #[test]
    fn number_storage_shared_kernels_exact_short_axes_retain_live_producer_records() {
        #[derive(Default)]
        struct Fees(Vec<[u64; 3]>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                _: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0
                    .push([0, (count * std::mem::size_of::<T>()) as u64, count as u64]);
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push([count, 0, 0]);
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push([0, 0, count]);
                Ok(())
            }
        }
        let fonts: [&[u8]; 2] = [
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/profiles/production-book-1/combined/job/body.ttf"
            )),
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../samples/machine-package/staging/production-book-1/vmb-book/header-context-font/header-context.ttf"
            )),
        ];
        for (text_index, text) in ["(123) abc", "العربية हिन्दी 한글 កម្ពុជា ᬩ"]
            .into_iter()
            .enumerate()
        {
            let mut prior_font: Option<[u64; 3]> = None;
            for (font_index, font) in fonts.into_iter().enumerate() {
                let face = harfrust::FontRef::from_index(font, 0).unwrap();
                let input = || EquationNumberRunInput {
                    exact_text: text,
                    text_span: TextSpan::new(
                        TextBufferId::new(42),
                        Utf8ByteOffset::new(100),
                        Utf8ByteOffset::new(100 + text.len() as u32),
                    )
                    .unwrap(),
                    font_bytes: font,
                    face_index: 0,
                    units_per_em: face.head().unwrap().units_per_em(),
                    glyph_count: u32::from(face.maxp().unwrap().num_glyphs()),
                    font_instance: FontInstanceId::new(1),
                    font_size: PositiveLength::new(Length::from_raw(12 * 65536).unwrap()).unwrap(),
                    language: Some("ja"),
                    maximum_records: 65_536,
                    total_records: Some(100_000),
                };
                let legacy = shape_equation_number_runs(input()).unwrap();
                let mut fees = Fees::default();
                assert_eq!(
                    shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
                    legacy
                );
                let mut prefixes = vec![[7, 11, 13]];
                for fee in fees.0 {
                    let last = *prefixes.last().unwrap();
                    prefixes.push([last[0] + fee[0], last[1] + fee[1], last[2] + fee[2]]);
                }
                // A rejected combined spool/work debit publishes neither axis.
                // Compare the exact accepted invoice prefix, including cases
                // where the final work refusal leaves a spool request unpaid.
                let prefix_at = |axis: usize, cap: u64| {
                    prefixes[prefixes.partition_point(|prefix| prefix[axis] <= cap) - 1]
                };
                let owner = NodeId::new(91);
                let run = |maximum: [u64; 3]| {
                    let raw = ResourceLimits {
                        max_spool_bytes: maximum[1],
                        ..ResourceLimits::default()
                    };
                    let limits = ValidatedResourceLimits::new(raw).unwrap();
                    let (mut r, mut s, mut w) = (7, 11, 13);
                    let result = with_book_v2_observed_source_accounting(
                        &mut r,
                        &mut s,
                        &mut w,
                        &limits,
                        maximum[0],
                        maximum[2],
                        |a| shape_runs_observed(input(), owner, a),
                    );
                    (result, [r, s, w])
                };
                let full = run([100_000, 1_000_000, 1_000_000]);
                assert_eq!(full.1, *prefixes.last().unwrap());
                if let Some(previous) = prior_font {
                    if text_index == 0 {
                        assert_eq!(full.1[0], previous[0]);
                        assert!(full.1[1] > previous[1] && full.1[2] > previous[2]);
                    }
                }
                prior_font = Some(full.1);
                assert_eq!(full.0.as_ref().unwrap(), &legacy);
                let records = legacy
                    .1
                    .iter()
                    .map(|r| 1 + r.glyphs.len() as u64 + r.clusters.len() as u64)
                    .sum::<u64>();
                assert_eq!(full.1[0], 7 + records);
                assert!(full.1[1] > 11 && full.1[2] > 13);
                assert_eq!(run(full.1), full);
                for axis in 0..3 {
                    let mut maximum = full.1;
                    maximum[axis] -= 1;
                    let failed = run(maximum);
                    let expected = match axis {
                        0 => BookV2ObservedAccountingError::Records { owner },
                        1 => BookV2ObservedAccountingError::Spool { owner },
                        _ => BookV2ObservedAccountingError::Work { owner },
                    };
                    assert_eq!(
                        failed.0.unwrap_err().kind,
                        BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                    );
                    assert!(failed.1.iter().zip(maximum).all(|(n, m)| *n <= m));
                    assert!(failed.1.iter().zip([7, 11, 13]).all(|(n, m)| *n >= m));
                    assert_eq!(failed.1, prefix_at(axis, maximum[axis]));
                    if axis > 0 {
                        assert_eq!(failed.1[0], full.1[0]);
                    }
                }
                // Exercise every smaller spool/work ceiling, including refusals inside
                // the dependency's class, paragraph and level buffers before any run
                // exists. The actual loan must preserve its owner and error axis.
                let mut cuts = 0;
                for axis in 1..3 {
                    let prior = [7, 11, 13];
                    for cap in prior[axis]..full.1[axis] {
                        let mut maximum = full.1;
                        maximum[axis] = cap;
                        let failed = run(maximum);
                        let expected = if axis == 1 {
                            BookV2ObservedAccountingError::Spool { owner }
                        } else {
                            BookV2ObservedAccountingError::Work { owner }
                        };
                        assert_eq!(
                            failed.0.unwrap_err().kind,
                            BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                        );
                        assert!(failed.1.iter().zip(maximum).all(|(n, m)| *n <= m));
                        assert!(failed.1.iter().zip(prior).all(|(n, m)| *n >= m));
                        assert_eq!(failed.1, prefix_at(axis, cap));
                        cuts += 1;
                    }
                }
                eprintln!(
                "number backend font={font_index} live-loan spool/work cuts={cuts}, full={:?}, text_case={text_index}",
                full.1
            );
            }
        }
    }

    #[test]
    fn number_storage_live_glyph_growth_keeps_typed_owner_and_exact_debit_prefix() {
        #[derive(Default)]
        struct Fees(Vec<(Option<shared::Site>, [u64; 3])>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                site: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0.push((
                    Some(site),
                    [0, (count * std::mem::size_of::<T>()) as u64, count as u64],
                ));
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [count, 0, 0]));
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [0, 0, count]));
                Ok(())
            }
        }
        let font = crate::shared_shape_storage::tests::glyph_growth_font();
        let face = harfrust::FontRef::from_index(&font, 0).unwrap();
        let input = || EquationNumberRunInput {
            exact_text: "RRR",
            text_span: TextSpan::new(
                TextBufferId::new(42),
                Utf8ByteOffset::new(100),
                Utf8ByteOffset::new(103),
            )
            .unwrap(),
            font_bytes: &font,
            face_index: 0,
            units_per_em: face.head().unwrap().units_per_em(),
            glyph_count: u32::from(face.maxp().unwrap().num_glyphs()),
            font_instance: FontInstanceId::new(1),
            font_size: PositiveLength::new(Length::from_raw(12 * 65536).unwrap()).unwrap(),
            language: Some("ja"),
            maximum_records: 65_536,
            total_records: Some(100_000),
        };
        let legacy = shape_equation_number_runs(input()).unwrap();
        assert_eq!(legacy.1[0].glyphs.len(), 384);
        let mut fees = Fees::default();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            legacy
        );
        let owner = NodeId::new(91);
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let (mut r, mut s, mut w) = (7, 11, 13);
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |accounting| shape_runs_observed(input(), owner, accounting),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        assert_eq!(full.0.as_ref().unwrap(), &legacy);
        assert_eq!(run(full.1), full);
        let mut prefix = [7, 11, 13];
        let mut controls = 0;
        let mut growth_requests = 0;
        let mut initialization = Vec::new();
        let mut control_rows = Vec::new();
        for (index, (site, fee)) in fees.0.iter().enumerate() {
            if matches!(
                site,
                Some(shared::Site::Harfrust(
                    harfrust::scratch::Site::GlyphInfos | harfrust::scratch::Site::GlyphPositions
                ))
            ) {
                growth_requests += 1;
                for axis in [1, 2] {
                    assert!(fee[axis] > 0);
                    let mut maximum = full.1;
                    maximum[axis] = prefix[axis] + fee[axis] - 1;
                    let failed = run(maximum);
                    let error = if axis == 1 {
                        BookV2ObservedAccountingError::Spool { owner }
                    } else {
                        BookV2ObservedAccountingError::Work { owner }
                    };
                    let failure = failed.0.unwrap_err().kind;
                    assert_eq!(
                        failure,
                        BookV2EquationNumberErrorKind::ObservedAccounting(error)
                    );
                    assert_eq!(failed.1, prefix);
                    control_rows.push((index, axis, maximum, failed.1, format!("{failure:?}")));
                    controls += 1;
                }
                if matches!(
                    site,
                    Some(shared::Site::Harfrust(
                        harfrust::scratch::Site::GlyphPositions
                    ))
                ) {
                    initialization.extend([index + 1, index + 2]);
                }
            }
            if initialization.contains(&index) {
                assert!(site.is_none() && fee[0] == 0 && fee[1] == 0 && fee[2] > 0);
                let mut maximum = full.1;
                maximum[2] = prefix[2] + fee[2] - 1;
                let failed = run(maximum);
                let failure = failed.0.unwrap_err().kind;
                assert_eq!(
                    failure,
                    BookV2EquationNumberErrorKind::ObservedAccounting(
                        BookV2ObservedAccountingError::Work { owner }
                    )
                );
                assert_eq!(failed.1, prefix);
                control_rows.push((index, 2, maximum, failed.1, format!("{failure:?}")));
                controls += 1;
            }
            for axis in 0..3 {
                prefix[axis] += fee[axis];
            }
        }
        assert_eq!(prefix, full.1);
        assert_eq!(growth_requests, 14);
        assert_eq!(controls, 42);
        if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
            use std::fmt::Write as _;
            let output = std::path::PathBuf::from(directory).join("number-glyph-growth");
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("derived-multiple.ttf"), &font).unwrap();
            std::fs::write(
                output.join("glyphs.json"),
                encode_equation_number_glyph_receipt(&legacy.1),
            )
            .unwrap();
            let mut json = String::from("{\"schema\":\"typaxis.number-glyph-growth-source-probe/1\",\"fixture_kind\":\"derived-test-font\",\"owner\":91,\"source_text\":\"RRR\",\"glyphs\":384,\"initial\":[7,11,13],\"full\":");
            write!(
                json,
                "{:?},\"glyph_info_bytes\":{},\"glyph_position_bytes\":{},\"events\":[",
                full.1,
                std::mem::size_of::<harfrust::GlyphInfo>(),
                std::mem::size_of::<harfrust::GlyphPosition>()
            )
            .unwrap();
            for (index, (site, fee)) in fees.0.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push_str("{\"site\":");
                typaxis_core::push_jcs_string(
                    &mut json,
                    &site.map(|site| format!("{site:?}")).unwrap_or_default(),
                );
                write!(json, ",\"delta\":{fee:?}}}").unwrap();
            }
            json.push_str("],\"controls\":[");
            for (index, (event, axis, maximum, observed, error)) in control_rows.iter().enumerate()
            {
                if index > 0 {
                    json.push(',');
                }
                write!(json, "{{\"event\":{event},\"axis\":{axis},\"maximum\":{maximum:?},\"observed\":{observed:?},\"error\":").unwrap();
                typaxis_core::push_jcs_string(&mut json, error);
                json.push('}');
            }
            json.push_str("]}\n");
            std::fs::write(output.join("source-fees.json"), json).unwrap();
        }
        eprintln!("number glyph growth live-loan: glyphs=384, growth reservations={growth_requests}, typed cutoff cases={controls}, full={:?}",full.1);
    }

    #[test]
    fn number_storage_aat_maps_keep_typed_source_errors_and_every_smaller_spool_work_prefix() {
        #[derive(Default)]
        struct Fees(Vec<(Option<shared::Site>, [u64; 3])>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                site: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0.push((
                    Some(site),
                    [0, (count * std::mem::size_of::<T>()) as u64, count as u64],
                ));
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [count, 0, 0]));
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [0, 0, count]));
                Ok(())
            }
        }
        let font = shared::tests::aat_font();
        let input = || shared::tests::input(&font, "111");
        let legacy = shape_equation_number_runs(input()).unwrap();
        assert!(legacy.1[0]
            .glyphs
            .iter()
            .all(|glyph| glyph.original_gid.get() == 19));
        let mut fees = Fees::default();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            legacy
        );
        let owner = NodeId::new(91);
        let initial = [7u64, 11, 13];
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let [mut r, mut s, mut w] = initial;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |accounting| shape_runs_observed(input(), owner, accounting),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        assert_eq!(full.0.as_ref().unwrap(), &legacy);
        assert_eq!(run(full.1), full);
        let mut paid = initial;
        for (_, fee) in &fees.0 {
            for axis in 0..3 {
                paid[axis] += fee[axis];
            }
        }
        assert_eq!(paid, full.1);
        let mut controls = Vec::new();
        for axis in [1, 2] {
            for cap in initial[axis]..full.1[axis] {
                let mut maximum = full.1;
                maximum[axis] = cap;
                let mut prefix = initial;
                let mut rejected = None;
                for (index, (_, fee)) in fees.0.iter().enumerate() {
                    let proposed = [prefix[0] + fee[0], prefix[1] + fee[1], prefix[2] + fee[2]];
                    if (0..3).any(|i| proposed[i] > maximum[i]) {
                        rejected = Some(index);
                        break;
                    }
                    prefix = proposed;
                }
                let event = rejected.unwrap();
                let failed = run(maximum);
                let failure = failed.0.unwrap_err().kind;
                let expected = if axis == 1 {
                    BookV2ObservedAccountingError::Spool { owner }
                } else {
                    BookV2ObservedAccountingError::Work { owner }
                };
                assert_eq!(
                    failure,
                    BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                );
                assert_eq!(failed.1, prefix, "axis={axis}, cap={cap}, event={event}");
                controls.push((event, axis, maximum, failed.1, format!("{failure:?}")));
            }
        }
        let growth = fees
            .0
            .iter()
            .filter(|(site, _)| {
                matches!(
                    site,
                    Some(shared::Site::Harfrust(
                        harfrust::scratch::Site::AatFeatureEvents
                            | harfrust::scratch::Site::AatChainFlags
                            | harfrust::scratch::Site::AatRangeFlags
                    ))
                )
            })
            .count();
        assert_eq!(growth, 5);
        if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
            use std::fmt::Write as _;
            let output = std::path::PathBuf::from(directory).join("aat-number-source");
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("derived-aat.ttf"), &font).unwrap();
            std::fs::write(
                output.join("glyphs.json"),
                encode_equation_number_glyph_receipt(&legacy.1),
            )
            .unwrap();
            let mut json=String::from("{\"schema\":\"typaxis.number-aat-source-probe/1\",\"fixture_kind\":\"derived-test-font\",\"owner\":91,\"source_text\":\"111\",\"glyphs\":3,\"initial\":[7,11,13],\"full\":");
            write!(json, "{:?},\"events\":[", full.1).unwrap();
            for (index, (site, fee)) in fees.0.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push_str("{\"site\":");
                typaxis_core::push_jcs_string(
                    &mut json,
                    &site.map(|site| format!("{site:?}")).unwrap_or_default(),
                );
                write!(json, ",\"delta\":{fee:?}}}").unwrap();
            }
            json.push_str("],\"controls\":[");
            for (index, (event, axis, maximum, observed, error)) in controls.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(json,"{{\"event\":{event},\"axis\":{axis},\"maximum\":{maximum:?},\"observed\":{observed:?},\"error\":").unwrap();
                typaxis_core::push_jcs_string(&mut json, error);
                json.push('}');
            }
            json.push_str("]}\n");
            std::fs::write(output.join("source-fees.json"), json).unwrap();
        }
        eprintln!(
            "number AAT live-loan: reservations={growth}, typed spool/work cutoffs={}, full={:?}",
            controls.len(),
            full.1
        );
    }
    #[test]
    fn number_storage_match_positions_keep_typed_source_errors_and_every_smaller_spool_work_prefix()
    {
        #[derive(Default)]
        struct Fees(Vec<(Option<shared::Site>, [u64; 3])>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                site: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0.push((
                    Some(site),
                    [0, (count * std::mem::size_of::<T>()) as u64, count as u64],
                ));
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [count, 0, 0]));
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [0, 0, count]));
                Ok(())
            }
        }
        let font = shared::tests::context_font();
        let input = || shared::tests::input(&font, "11111");
        let legacy = shape_equation_number_runs(input()).unwrap();
        assert_eq!(legacy.1[0].glyphs.len(), 12);
        assert_eq!(
            legacy.1[0]
                .glyphs
                .iter()
                .filter(|g| g.original_gid.get() == 19)
                .count(),
            7
        );
        assert_eq!(
            legacy.1[0]
                .glyphs
                .iter()
                .filter(|g| g.original_gid.get() == 20)
                .count(),
            1
        );
        let mut fees = Fees::default();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            legacy
        );
        let owner = NodeId::new(91);
        let initial = [7u64, 11, 13];
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let [mut r, mut s, mut w] = initial;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |accounting| shape_runs_observed(input(), owner, accounting),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        assert_eq!(full.0.as_ref().unwrap(), &legacy);
        assert_eq!(run(full.1), full);
        let mut paid = initial;
        for (_, fee) in &fees.0 {
            for axis in 0..3 {
                paid[axis] += fee[axis];
            }
        }
        assert_eq!(paid, full.1);
        let mut controls = Vec::new();
        for axis in [1, 2] {
            for cap in initial[axis]..full.1[axis] {
                let mut maximum = full.1;
                maximum[axis] = cap;
                let mut prefix = initial;
                let mut rejected = None;
                for (index, (_, fee)) in fees.0.iter().enumerate() {
                    let proposed = [prefix[0] + fee[0], prefix[1] + fee[1], prefix[2] + fee[2]];
                    if (0..3).any(|i| proposed[i] > maximum[i]) {
                        rejected = Some(index);
                        break;
                    }
                    prefix = proposed;
                }
                let event = rejected.unwrap();
                let failed = run(maximum);
                let failure = failed.0.unwrap_err().kind;
                let expected = if axis == 1 {
                    BookV2ObservedAccountingError::Spool { owner }
                } else {
                    BookV2ObservedAccountingError::Work { owner }
                };
                assert_eq!(
                    failure,
                    BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                );
                assert_eq!(failed.1, prefix, "axis={axis}, cap={cap}, event={event}");
                controls.push((event, axis, maximum, failed.1, format!("{failure:?}")));
            }
        }
        let growth = fees
            .0
            .iter()
            .filter(|(site, _)| {
                matches!(
                    site,
                    Some(shared::Site::Harfrust(
                        harfrust::scratch::Site::MatchPositions
                    ))
                )
            })
            .count();
        assert_eq!(growth, 2);
        if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
            use std::fmt::Write as _;
            let output = std::path::PathBuf::from(directory).join("match-number-source");
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("derived-contextual.ttf"), &font).unwrap();
            std::fs::write(
                output.join("glyphs.json"),
                encode_equation_number_glyph_receipt(&legacy.1),
            )
            .unwrap();
            let mut json=String::from("{\"schema\":\"typaxis.number-match-source-probe/1\",\"fixture_kind\":\"derived-test-font\",\"owner\":91,\"source_text\":\"11111\",\"glyphs\":12,\"initial\":[7,11,13],\"full\":");
            write!(json, "{:?},\"events\":[", full.1).unwrap();
            for (index, (site, fee)) in fees.0.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push_str("{\"site\":");
                typaxis_core::push_jcs_string(
                    &mut json,
                    &site.map(|site| format!("{site:?}")).unwrap_or_default(),
                );
                write!(json, ",\"delta\":{fee:?}}}").unwrap();
            }
            json.push_str("],\"controls\":[");
            for (index, (event, axis, maximum, observed, error)) in controls.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(json,"{{\"event\":{event},\"axis\":{axis},\"maximum\":{maximum:?},\"observed\":{observed:?},\"error\":").unwrap();
                typaxis_core::push_jcs_string(&mut json, error);
                json.push('}');
            }
            json.push_str("]}\n");
            std::fs::write(output.join("source-fees.json"), json).unwrap();
        }
        eprintln!(
            "number match positions live-loan: reservations={growth}, typed spool/work cutoffs={}, full={:?}",
            controls.len(),
            full.1
        );
    }
    #[test]
    fn number_storage_indic_sort_keep_typed_source_errors_and_every_smaller_spool_work_prefix() {
        #[derive(Default)]
        struct Fees(Vec<(Option<shared::Site>, [u64; 3])>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                site: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0.push((
                    Some(site),
                    [0, (count * std::mem::size_of::<T>()) as u64, count as u64],
                ));
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [count, 0, 0]));
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [0, 0, count]));
                Ok(())
            }
        }
        let font = shared::tests::indic_font();
        let input = || shared::tests::input(&font, "किकि");
        let legacy = shape_equation_number_runs(input()).unwrap();
        assert_eq!(
            legacy.1[0]
                .glyphs
                .iter()
                .map(|g| g.original_gid.get())
                .collect::<Vec<_>>(),
            [65, 23, 65, 23]
        );
        let mut fees = Fees::default();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            legacy
        );
        let owner = NodeId::new(91);
        let initial = [7u64, 11, 13];
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let [mut r, mut s, mut w] = initial;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |accounting| shape_runs_observed(input(), owner, accounting),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        assert_eq!(full.0.as_ref().unwrap(), &legacy);
        assert_eq!(run(full.1), full);
        let mut paid = initial;
        for (_, fee) in &fees.0 {
            for axis in 0..3 {
                paid[axis] += fee[axis];
            }
        }
        assert_eq!(paid, full.1);
        let mut controls = Vec::new();
        for axis in [1, 2] {
            for cap in initial[axis]..full.1[axis] {
                let mut maximum = full.1;
                maximum[axis] = cap;
                let mut prefix = initial;
                let mut rejected = None;
                for (index, (_, fee)) in fees.0.iter().enumerate() {
                    let proposed = [prefix[0] + fee[0], prefix[1] + fee[1], prefix[2] + fee[2]];
                    if (0..3).any(|i| proposed[i] > maximum[i]) {
                        rejected = Some(index);
                        break;
                    }
                    prefix = proposed;
                }
                let event = rejected.unwrap();
                let failed = run(maximum);
                let failure = failed.0.unwrap_err().kind;
                let expected = if axis == 1 {
                    BookV2ObservedAccountingError::Spool { owner }
                } else {
                    BookV2ObservedAccountingError::Work { owner }
                };
                assert_eq!(
                    failure,
                    BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                );
                assert_eq!(failed.1, prefix, "axis={axis}, cap={cap}, event={event}");
                controls.push((event, axis, maximum, failed.1, format!("{failure:?}")));
            }
        }
        let growth = fees
            .0
            .iter()
            .filter(|(site, _)| {
                matches!(
                    site,
                    Some(shared::Site::Harfrust(
                        harfrust::scratch::Site::IndicSortRecords
                    ))
                )
            })
            .count();
        assert_eq!(growth, 2);
        if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
            use std::fmt::Write as _;
            let output = std::path::PathBuf::from(directory).join("indic-number-source");
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("derived-indic.ttf"), &font).unwrap();
            std::fs::write(
                output.join("glyphs.json"),
                encode_equation_number_glyph_receipt(&legacy.1),
            )
            .unwrap();
            let mut json=String::from("{\"schema\":\"typaxis.number-indic-source-probe/1\",\"fixture_kind\":\"derived-test-font\",\"owner\":91,\"source_text\":\"किकि\",\"glyphs\":4,\"initial\":[7,11,13],\"full\":");
            write!(json, "{:?},\"events\":[", full.1).unwrap();
            for (index, (site, fee)) in fees.0.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push_str("{\"site\":");
                typaxis_core::push_jcs_string(
                    &mut json,
                    &site.map(|site| format!("{site:?}")).unwrap_or_default(),
                );
                write!(json, ",\"delta\":{fee:?}}}").unwrap();
            }
            json.push_str("],\"controls\":[");
            for (index, (event, axis, maximum, observed, error)) in controls.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(json,"{{\"event\":{event},\"axis\":{axis},\"maximum\":{maximum:?},\"observed\":{observed:?},\"error\":").unwrap();
                typaxis_core::push_jcs_string(&mut json, error);
                json.push('}');
            }
            json.push_str("]}\n");
            std::fs::write(output.join("source-fees.json"), json).unwrap();
        }
        eprintln!(
            "number Indic sort live-loan: reservations={growth}, typed spool/work cutoffs={}, full={:?}",
            controls.len(),
            full.1
        );
    }

    fn exercise_buffer_sort_source(font: Vec<u8>, text: &str, owner_value: u32, case: &str) {
        #[derive(Default)]
        struct Fees(Vec<(Option<shared::Site>, [u64; 3])>);
        impl shared::Storage for Fees {
            fn reserve<T>(
                &mut self,
                site: shared::Site,
                count: usize,
            ) -> Result<(), LinkedShaperError> {
                self.0.push((
                    Some(site),
                    [0, (count * std::mem::size_of::<T>()) as u64, count as u64],
                ));
                Ok(())
            }
            fn records(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [count, 0, 0]));
                Ok(())
            }
            fn work(&mut self, count: u64) -> Result<(), LinkedShaperError> {
                self.0.push((None, [0, 0, count]));
                Ok(())
            }
        }
        let input = || shared::tests::input(&font, text);
        let legacy = shape_equation_number_runs(input()).unwrap();
        assert_eq!(
            legacy.1.iter().map(|run| run.glyphs.len()).sum::<usize>(),
            text.chars().count()
        );
        assert!(legacy
            .1
            .iter()
            .flat_map(|run| &run.glyphs)
            .all(|glyph| glyph.original_gid.get() != 0));
        let mut fees = Fees::default();
        assert_eq!(
            shared::shape_equation_number_runs_with_storage(input(), &mut fees).unwrap(),
            legacy
        );
        let owner = NodeId::new(owner_value);
        let initial = [7u64, 11, 13];
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let [mut r, mut s, mut w] = initial;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |accounting| shape_runs_observed(input(), owner, accounting),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        assert_eq!(full.0.as_ref().unwrap(), &legacy);
        assert_eq!(run(full.1), full);
        let mut paid = initial;
        for (_, fee) in &fees.0 {
            for axis in 0..3 {
                paid[axis] += fee[axis];
            }
        }
        assert_eq!(paid, full.1);
        let mut controls = Vec::new();
        for axis in [1, 2] {
            for cap in initial[axis]..full.1[axis] {
                let mut maximum = full.1;
                maximum[axis] = cap;
                let mut prefix = initial;
                let mut rejected = None;
                for (index, (_, fee)) in fees.0.iter().enumerate() {
                    let proposed = [prefix[0] + fee[0], prefix[1] + fee[1], prefix[2] + fee[2]];
                    if (0..3).any(|i| proposed[i] > maximum[i]) {
                        rejected = Some(index);
                        break;
                    }
                    prefix = proposed;
                }
                let event = rejected.unwrap();
                let failed = run(maximum);
                let failure = failed.0.unwrap_err().kind;
                let expected = if axis == 1 {
                    BookV2ObservedAccountingError::Spool { owner }
                } else {
                    BookV2ObservedAccountingError::Work { owner }
                };
                assert_eq!(
                    failure,
                    BookV2EquationNumberErrorKind::ObservedAccounting(expected)
                );
                assert_eq!(failed.1, prefix, "axis={axis}, cap={cap}, event={event}");
                controls.push((event, axis, maximum, failed.1, format!("{failure:?}")));
            }
        }
        let growth = fees
            .0
            .iter()
            .filter(|(site, _)| {
                matches!(
                    site,
                    Some(shared::Site::Harfrust(
                        harfrust::scratch::Site::IndicSortRecords
                    ))
                )
            })
            .count();
        assert_eq!(growth, 0);
        if let Some(directory) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") {
            use std::fmt::Write as _;
            let output = std::path::PathBuf::from(directory)
                .join("buffer-sort-number-source")
                .join(case);
            std::fs::create_dir_all(&output).unwrap();
            std::fs::write(output.join("derived-font.ttf"), &font).unwrap();
            std::fs::write(
                output.join("glyphs.json"),
                encode_equation_number_glyph_receipt(&legacy.1),
            )
            .unwrap();
            let mut json = String::from("{\"schema\":\"typaxis.number-buffer-sort-source-probe/1\",\"fixture_kind\":\"derived-test-font\",\"owner\":");
            write!(json, "{owner_value},\"source_text\":").unwrap();
            typaxis_core::push_jcs_string(&mut json, text);
            write!(
                json,
                ",\"glyphs\":{},\"initial\":[7,11,13],\"full\":",
                text.chars().count()
            )
            .unwrap();
            write!(json, "{:?},\"events\":[", full.1).unwrap();
            for (index, (site, fee)) in fees.0.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                json.push_str("{\"site\":");
                typaxis_core::push_jcs_string(
                    &mut json,
                    &site.map(|site| format!("{site:?}")).unwrap_or_default(),
                );
                write!(json, ",\"delta\":{fee:?}}}").unwrap();
            }
            json.push_str("],\"controls\":[");
            for (index, (event, axis, maximum, observed, error)) in controls.iter().enumerate() {
                if index > 0 {
                    json.push(',');
                }
                write!(json,"{{\"event\":{event},\"axis\":{axis},\"maximum\":{maximum:?},\"observed\":{observed:?},\"error\":").unwrap();
                typaxis_core::push_jcs_string(&mut json, error);
                json.push('}');
            }
            json.push_str("]}\n");
            std::fs::write(output.join("source-fees.json"), json).unwrap();
        }
        eprintln!(
            "number buffer sort {case}: sort scratch reservations={growth}, typed spool/work cutoffs={}, full={:?}",
            controls.len(),
            full.1
        );
    }

    #[test]
    fn number_storage_buffer_sort_keeps_source_owner_and_every_smaller_spool_work_prefix() {
        exercise_buffer_sort_source(
            shared::tests::normalization_font(),
            "x̣̀̕",
            92,
            "normalization",
        );
        exercise_buffer_sort_source(shared::tests::myanmar_font(), "ကြေကိ", 93, "myanmar");
    }

    #[test]
    fn number_storage_indic_ordinal_limit_keeps_source_owner_and_earlier_budget_error() {
        let font = shared::tests::indic_font();
        let text = format!("{}कि", "क्".repeat(128));
        let owner = NodeId::new(91);
        let initial = [7u64, 11, 13];
        let run = |maximum: [u64; 3]| {
            let limits = ValidatedResourceLimits::new(ResourceLimits {
                max_spool_bytes: maximum[1],
                ..ResourceLimits::default()
            })
            .unwrap();
            let [mut r, mut s, mut w] = initial;
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                maximum[0],
                maximum[2],
                |a| shape_runs_observed(shared::tests::input(&font, &text), owner, a),
            );
            (result, [r, s, w])
        };
        let full = run([100_000, 1_000_000, 1_000_000]);
        let error = full.0.unwrap_err();
        assert_eq!(error.owner, owner);
        assert!(matches!(
            error.kind,
            BookV2EquationNumberErrorKind::Shape(StagingEquationNumberShapeError::Backend(
                LinkedShaperError::OutputBudget(crate::ShapeWorkError::GlyphLimit)
            ))
        ));
        let short = run([100_000, 1_000_000, 13]);
        let error = short.0.unwrap_err();
        assert_eq!(error.owner, owner);
        assert_eq!(
            error.kind,
            BookV2EquationNumberErrorKind::ObservedAccounting(
                BookV2ObservedAccountingError::Work { owner }
            )
        );
        assert_eq!(short.1, initial);
        let short = run([100_000, 11, 1_000_000]);
        let error = short.0.unwrap_err();
        assert_eq!(error.owner, owner);
        assert_eq!(
            error.kind,
            BookV2EquationNumberErrorKind::ObservedAccounting(
                BookV2ObservedAccountingError::Spool { owner }
            )
        );
        assert_eq!(short.1[1], 11);
    }

    fn runs() -> Vec<StagingEquationNumberGlyphRun> {
        [*b"Latn", *b"Arab", *b"\"\\A "]
            .into_iter()
            .enumerate()
            .map(|(index, script)| {
                let span = TextSpan::new(
                    TextBufferId::new(u32::MAX),
                    Utf8ByteOffset::new(123_456),
                    Utf8ByteOffset::new(345_678),
                )
                .unwrap();
                StagingEquationNumberGlyphRun {
                    run_id: GlyphRunId::new(index as u32),
                    bidi_level: BidiLevel::new(index as u8).unwrap(),
                    script: OpenTypeTag::new(script).unwrap(),
                    source_span: span,
                    glyphs: vec![ShapedGlyph {
                        original_gid: OriginalGlyphId::new(u16::MAX),
                        advance_x: Length::from_raw(12_345_678).unwrap(),
                        advance_y: Length::from_raw(-23_456).unwrap(),
                        offset_x: Length::from_raw(-345_678).unwrap(),
                        offset_y: Length::from_raw(456_789).unwrap(),
                    }],
                    clusters: vec![ShapedCluster {
                        source_span: ShapeSourceSpan::Parsed(span),
                        glyph_start: 0,
                        glyph_end: 1,
                    }],
                }
            })
            .collect()
    }

    #[test]
    fn number_storage_canonical_encoder_matches_legacy_and_never_grows_reserved_output() {
        let runs = runs();
        for count in 0..=runs.len() {
            let runs = &runs[..count];
            let legacy = encode_equation_number_glyph_receipt(runs);
            let mut length = Count(0);
            write_receipt(&mut length, runs).unwrap();
            assert_eq!(length.0, legacy.len());
            let mut output = String::with_capacity(length.0);
            let capacity = output.capacity();
            write_receipt(
                &mut Reserved {
                    text: &mut output,
                    length: length.0,
                },
                runs,
            )
            .unwrap();
            assert_eq!(output, legacy);
            assert_eq!(output.capacity(), capacity);
            output.clear();
            assert!(write_receipt(
                &mut Reserved {
                    text: &mut output,
                    length: length.0 - 1
                },
                runs
            )
            .is_err());
            assert!(output.len() < length.0);
            assert_eq!(output.capacity(), capacity);
        }
    }

    #[test]
    fn number_storage_canonical_exact_short_spool_and_work_keep_actual_prior() {
        let runs = runs();
        let canonical = encode_equation_number_glyph_receipt(&runs);
        let bytes = canonical.len() as u64;
        let owner = NodeId::new(91);
        let run = |max_spool, max_work| {
            let raw = ResourceLimits {
                max_spool_bytes: max_spool,
                ..ResourceLimits::default()
            };
            let limits = ValidatedResourceLimits::new(raw).unwrap();
            let (mut r, mut s, mut w) = (7, 11, 13);
            let result = with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                &limits,
                100,
                max_work,
                |a| glyph_digest(owner, &runs, a),
            );
            (result, [r, s, w])
        };
        assert_eq!(
            run(11 + bytes, 13 + 2 * bytes),
            (
                Ok(sha256(canonical.as_bytes())),
                [7, 11 + bytes, 13 + 2 * bytes]
            )
        );
        for (spool, work, error) in [
            (
                10 + bytes,
                13 + 2 * bytes,
                BookV2ObservedAccountingError::Spool { owner },
            ),
            (
                11 + bytes,
                12 + 2 * bytes,
                BookV2ObservedAccountingError::Work { owner },
            ),
        ] {
            let (result, counters) = run(spool, work);
            assert_eq!(
                result.unwrap_err().kind,
                BookV2EquationNumberErrorKind::ObservedAccounting(error)
            );
            assert_eq!(counters, [7, 11, 13]);
        }
    }
}

#[cfg(test)]
#[path = "book_v2_font_selection_tests.rs"]
mod font_selection_tests;
