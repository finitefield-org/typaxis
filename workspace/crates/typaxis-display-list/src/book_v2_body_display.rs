//! One source-owned display in physical fragment and authored inline order.
use super::*;

pub const BOOK_V2_BODY_DISPLAY_ALGORITHM: &str = "typaxis.book-2-body-display/1";

/// Indices address this display's owned components. Anchors are nonpainting;
/// numbers remain separate from formula replacement text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BookV2BodyPaintIndex {
    FootnoteSeparator(usize),
    Marker(usize),
    Text(usize),
    Math(usize),
    Image(usize),
    EquationNumber(usize),
}
pub struct BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    math: BookV2MathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    text: BookV2TextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    numbers: BookV2EquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    markers: BookV2MarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    images: BookV2ImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    anchors: BookV2AnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>,
    paints: Vec<BookV2BodyPaintIndex>,
    fingerprint: [u8; 32],
    records: u64,
    work: u64,
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    pub fn source(&self) -> &'d BookV2BodyMathTerminals<'g, 'q, 'b, 'f, 's, 'p, 'a> {
        self.math.source()
    }
    pub fn admitted(&self) -> &'d AdmittedProductionResourceLedgerV3 {
        self.math.admitted()
    }
    pub fn math(&self) -> &BookV2MathDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.math
    }
    pub fn text(&self) -> &BookV2TextDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.text
    }
    pub fn numbers(&self) -> &BookV2EquationNumberDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.numbers
    }
    pub fn markers(&self) -> &BookV2MarkerDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.markers
    }
    pub fn images(&self) -> &BookV2ImageDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.images
    }
    pub fn anchors(&self) -> &BookV2AnchorDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
        &self.anchors
    }
    pub fn paints(&self) -> &[BookV2BodyPaintIndex] {
        &self.paints
    }
    pub fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
    pub fn record_charge(&self) -> u64 {
        self.records
    }
    pub fn work_steps(&self) -> u64 {
        self.work
    }

    fn counts(&self) -> [usize; 6] {
        [
            self.markers.separators().len(),
            self.markers.draws().len(),
            self.text.draws().len(),
            self.math.draws().len(),
            self.images.draws().len(),
            self.numbers.draws().len(),
        ]
    }
    fn next(&self, stream: usize, index: usize) -> (BookV2BodyPaintIndex, (usize, u8, u32)) {
        use BookV2BodyPaintIndex as P;
        match stream {
            0 => (
                P::FootnoteSeparator(index),
                (
                    self.markers.separators()[index].before_fragment_index(),
                    0,
                    0,
                ),
            ),
            1 => (
                P::Marker(index),
                (self.markers.draws()[index].fragment_index(), 1, 0),
            ),
            2 => {
                let draw = &self.text.draws()[index];
                (
                    P::Text(index),
                    (draw.fragment_index(), 2, draw.inline_index()),
                )
            }
            3 => {
                let terminal = self.math.draws()[index].terminal();
                (
                    P::Math(index),
                    (
                        terminal.fragment_index(),
                        2,
                        terminal.inline_index().unwrap_or(0),
                    ),
                )
            }
            4 => {
                let draw = &self.images.draws()[index];
                (
                    P::Image(index),
                    (draw.fragment_index(), 2, draw.inline_index().unwrap_or(0)),
                )
            }
            5 => (
                P::EquationNumber(index),
                (
                    self.numbers.draws()[index]
                        .placement()
                        .geometry()
                        .fragment_index() as usize,
                    3,
                    0,
                ),
            ),
            _ => unreachable!("fixed six-stream display merge"),
        }
    }
}
impl<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> BookV2MathDisplayBuilder<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a> {
    /// Owns all components from this exact source/admission pair. The merge
    /// uses six stack cursors, not a second document-sized sorting buffer.
    pub fn build_body(
        &mut self,
    ) -> Result<BookV2BodyDisplay<'d, 'g, 'q, 'b, 'f, 's, 'p, 'a>, BookV2MathDisplayError> {
        let root = NodeId::new(0);
        let mut display = BookV2BodyDisplay {
            math: self.build()?,
            text: self.build_text()?,
            numbers: self.build_equation_numbers()?,
            markers: self.build_markers()?,
            images: self.build_images()?,
            anchors: self.build_anchors()?,
            paints: Vec::new(),
            fingerprint: [0; 32],
            records: 0,
            work: 0,
        };
        let counts = display.counts();
        let mut count = 0usize;
        for n in counts {
            self.step(root)?;
            count = count
                .checked_add(n)
                .ok_or_else(|| error(root, E::RecordLimit))?;
        }
        take(
            &mut self.remaining,
            count
                .checked_add(1)
                .ok_or_else(|| error(root, E::RecordLimit))?,
            root,
        )?;
        display
            .paints
            .try_reserve_exact(count)
            .map_err(|_| error(root, E::AllocationFailure))?;
        self.step(root)?;
        let mut fp = sha256(BOOK_V2_BODY_DISPLAY_ALGORITHM.as_bytes());
        for hash in [
            self.source.fingerprint(),
            self.admitted.fingerprint(),
            display.math.fingerprint(),
            display.text.fingerprint(),
            display.numbers.fingerprint(),
            display.markers.fingerprint(),
            display.images.fingerprint(),
            display.anchors.fingerprint(),
        ] {
            fp = self.fold(fp, &hash, root)?;
        }
        fp = self.fold(fp, &(count as u64).to_be_bytes(), root)?;
        let mut cursors = [0; 6];
        let mut previous = None;
        for _ in 0..count {
            let mut next = None;
            for stream in 0..6 {
                self.step(root)?;
                if cursors[stream] < counts[stream] {
                    let (paint, key) = display.next(stream, cursors[stream]);
                    if next.is_none_or(|(_, _, first)| key < first) {
                        next = Some((stream, paint, key));
                    }
                }
            }
            let (stream, paint, key) = next.ok_or_else(|| error(root, E::ReceiptMismatch))?;
            if previous.is_some_and(|p| p > key || (p == key && key.1 == 2)) {
                return Err(error(root, E::ReceiptMismatch).into());
            }
            // Component fingerprints bind geometry/roles. This record also
            // binds exact interleaving, including separate number placement.
            let mut bytes = [0; 22];
            bytes[0] = stream as u8;
            bytes[1..9].copy_from_slice(&(cursors[stream] as u64).to_be_bytes());
            bytes[9..17].copy_from_slice(&(key.0 as u64).to_be_bytes());
            bytes[17] = key.1;
            bytes[18..22].copy_from_slice(&key.2.to_be_bytes());
            fp = self.fold(fp, &bytes, root)?;
            display.paints.push(paint);
            cursors[stream] += 1;
            previous = Some(key);
        }
        if cursors != counts {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        display.fingerprint = fp;
        display.records = self.record_charge();
        display.work = self.work_steps();
        Ok(display)
    }
}
