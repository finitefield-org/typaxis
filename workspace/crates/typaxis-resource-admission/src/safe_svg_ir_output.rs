// Shared V2 encoder: an allocation-free measure pass and a reserved write pass.
trait SvgOutput {
    type Error;
    fn text(&mut self, value: &str) -> Result<(), Self::Error>;
    fn character(&mut self, value: char) -> Result<(), Self::Error> {
        let mut buf = [0; 4];
        self.text(value.encode_utf8(&mut buf))
    }
    fn quoted(&mut self, value: &str) -> Result<(), Self::Error> {
        // Only the closed algorithm, paint, and segment vocabularies reach here.
        debug_assert!(value.bytes().all(|b| b >= 32 && b != b'"' && b != b'\\'));
        self.text("\"")?;
        self.text(value)?;
        self.text("\"")
    }
    fn number(&mut self, value: i128) -> Result<(), Self::Error> {
        let mut buf = [0u8; 40];
        let mut at = buf.len();
        let mut n = value.unsigned_abs();
        loop {
            at -= 1;
            buf[at] = b'0' + (n % 10) as u8;
            n /= 10;
            if n == 0 {
                break;
            }
        }
        if value < 0 {
            at -= 1;
            buf[at] = b'-';
        }
        self.text(std::str::from_utf8(&buf[at..]).expect("decimal ASCII"))
    }
}
impl SvgOutput for String {
    type Error = std::convert::Infallible;
    fn text(&mut self, value: &str) -> Result<(), Self::Error> {
        self.push_str(value);
        Ok(())
    }
}
struct Measure<'a, B> {
    budget: &'a mut B,
    image: typaxis_core::ImageResourceId,
    length: usize,
    funded: usize,
}
impl<B: crate::SafeSvgSourceBudget> SvgOutput for Measure<'_, B> {
    type Error = crate::SafeSvgSourceError<B::Error>;
    fn text(&mut self, value: &str) -> Result<(), Self::Error> {
        let end = self
            .length
            .checked_add(value.len())
            .ok_or(ResourceAdmissionError::ResourceLimit)?;
        while end > self.funded {
            crate::safe_svg_source::permit(
                self.budget,
                self.image,
                crate::SafeSvgSourceStage::CanonicalMeasure,
                0,
                0,
                16 * 256 + 64,
            )?;
            self.funded = self
                .funded
                .checked_add(256)
                .ok_or(ResourceAdmissionError::ResourceLimit)?;
        }
        self.length = end;
        Ok(())
    }
}
struct ReservedText {
    text: String,
    bound: usize,
}
impl SvgOutput for ReservedText {
    type Error = ResourceAdmissionError;
    fn text(&mut self, value: &str) -> Result<(), Self::Error> {
        if self
            .text
            .len()
            .checked_add(value.len())
            .map_or(true, |n| n > self.bound)
        {
            return Err(ResourceAdmissionError::ReceiptIdentityMismatch);
        }
        self.text.push_str(value);
        Ok(())
    }
}
fn canonical_ir_observed<B: crate::SafeSvgSourceBudget>(
    built: &ScanResultV2<'_>,
    definitions: &[SafeVectorClipDefinition],
    path_work: u64,
    allocation_charge: u64,
    budget: &mut B,
    image: typaxis_core::ImageResourceId,
) -> Result<(String, [u8; 32]), crate::SafeSvgSourceError<B::Error>> {
    use crate::SafeSvgSourceStage as S;
    let mut measure = Measure {
        budget,
        image,
        length: 0,
        funded: 0,
    };
    write_ir_v2(
        &mut measure,
        built.root,
        definitions,
        &built.draws,
        built.counts,
        path_work,
        allocation_charge,
    )?;
    let length = measure.length;
    crate::safe_svg_source::permit(budget, image, S::CanonicalStorage, 1, length, 64)?;
    let mut text = String::new();
    text.try_reserve_exact(length)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for offset in (0..length).step_by(256) {
        crate::safe_svg_source::permit(
            budget,
            image,
            S::CanonicalWrite,
            0,
            0,
            16 * (length - offset).min(256) as u64 + 64,
        )?;
    }
    let mut output = ReservedText {
        text,
        bound: length,
    };
    write_ir_v2(
        &mut output,
        built.root,
        definitions,
        &built.draws,
        built.counts,
        path_work,
        allocation_charge,
    )?;
    if output.text.len() != length {
        return Err(ResourceAdmissionError::ReceiptIdentityMismatch.into());
    }
    let mut hash = typaxis_core::Sha256::new();
    for bytes in [
        b"{\"algorithm\":\"".as_slice(),
        SAFE_VECTOR_IR_FINGERPRINT_ID_V2.as_bytes(),
        b"\",\"ir\":",
        output.text.as_bytes(),
        b"}",
    ] {
        for chunk in bytes.chunks(256) {
            crate::safe_svg_source::permit(
                budget,
                image,
                S::IrHash,
                0,
                0,
                2 * chunk.len() as u64 + 64,
            )?;
            hash.update(chunk);
        }
    }
    crate::safe_svg_source::permit(budget, image, S::IrFinish, 0, 0, 64)?;
    let fingerprint = hash.finish();
    Ok((output.text, fingerprint))
}

fn write_ir_v2<O: SvgOutput>(
    output: &mut O,
    root: RootGeometry,
    clips: &[SafeVectorClipDefinition],
    draws: &[SafeVectorDrawV2],
    counts: Counts,
    path_work: u64,
    allocation_charge: u64,
) -> Result<(), O::Error> {
    output.text("{\"algorithm\":")?;
    output.quoted(SAFE_VECTOR_IR_ID_V2)?;
    output.text(",\"allocation\":{\"algorithm\":")?;
    output.quoted(SAFE_VECTOR_ALLOCATION_CHARGE_ID_V2)?;
    output.text(",\"charge\":")?;
    output.number((allocation_charge) as i128)?;
    output.text(",\"nodes\":")?;
    output.number((counts.nodes) as i128)?;
    output.text(",\"paint_or_clip_commands\":")?;
    output.number((counts.commands) as i128)?;
    output.text(",\"source_clip_id_bytes\":")?;
    output.number((counts.source_clip_id_bytes) as i128)?;
    output.text(",\"stored_segments\":")?;
    output.number((counts.stored_segments) as i128)?;
    output.text("},\"clips\":[")?;
    for (index, clip) in clips.iter().enumerate() {
        if index > 0 {
            output.character(',')?;
        }
        output.text("{\"clip_id\":")?;
        output.number((clip.clip_id) as i128)?;
        output.text(",\"fill_rule\":")?;
        output.quoted(clip.fill_rule.as_str())?;
        output.text(",\"path\":")?;
        write_path(output, &clip.path)?;
        output.text(",\"transform\":")?;
        write_transform(output, clip.transform)?;
        output.character('}')?;
    }
    output.text("],\"draws\":[")?;
    for (index, draw) in draws.iter().enumerate() {
        if index > 0 {
            output.character(',')?;
        }
        output.text("{\"clips\":[")?;
        for (clip_index, clip) in draw.clips.iter().enumerate() {
            if clip_index > 0 {
                output.character(',')?;
            }
            output.text("{\"clip_id\":")?;
            output.number((clip.clip_id) as i128)?;
            output.text(",\"transform\":")?;
            write_transform(output, clip.transform)?;
            output.character('}')?;
        }
        output.text("],\"fill\":")?;
        write_paint_layer(output, draw.fill)?;
        output.text(",\"fill_rule\":")?;
        output.quoted(draw.fill_rule.as_str())?;
        output.text(",\"path\":")?;
        write_path(output, &draw.path)?;
        output.text(",\"stroke\":{\"alpha\":")?;
        output.number((draw.stroke.paint.alpha.raw()) as i128)?;
        output.text(",\"line_cap\":")?;
        output.quoted(draw.stroke.line_cap.as_str())?;
        output.text(",\"line_join\":")?;
        output.quoted(draw.stroke.line_join.as_str())?;
        output.text(",\"miter_limit\":")?;
        output.number((draw.stroke.miter_limit) as i128)?;
        output.text(",\"paint\":")?;
        write_paint(output, draw.stroke.paint.paint)?;
        output.text(",\"width\":")?;
        output.number((draw.stroke.width) as i128)?;
        output.text("},\"transform\":")?;
        write_transform(output, draw.transform)?;
        output.character('}')?;
    }
    output.text("],\"intrinsic_height\":")?;
    output.number((root.height.get().raw()) as i128)?;
    output.text(",\"intrinsic_width\":")?;
    output.number((root.width.get().raw()) as i128)?;
    output.text(",\"parser\":")?;
    output.quoted(SAFE_SVG_PARSER_ID_V2)?;
    output.text(",\"path_work\":")?;
    output.number((path_work) as i128)?;
    output.text(",\"root_scale\":")?;
    output.number((root.root_scale) as i128)?;
    output.text(",\"root_viewport_clip\":\"outermost\",\"view_box\":[")?;
    for (index, value) in root.view_box.iter().enumerate() {
        if index > 0 {
            output.character(',')?;
        }
        output.number((*value) as i128)?;
    }
    output.text("]}")?;
    Ok(())
}

fn write_paint_layer<O: SvgOutput>(
    output: &mut O,
    layer: SafeVectorPaintLayer,
) -> Result<(), O::Error> {
    output.text("{\"alpha\":")?;
    output.number((layer.alpha.raw()) as i128)?;
    output.text(",\"paint\":")?;
    write_paint(output, layer.paint)?;
    output.character('}')?;
    Ok(())
}

fn write_paint<O: SvgOutput>(output: &mut O, paint: SafeVectorPaint) -> Result<(), O::Error> {
    output.text("{\"kind\":")?;
    output.quoted(paint.as_str())?;
    if let SafeVectorPaint::FixedRgb8(rgb) = paint {
        output.text(",\"rgb\":[")?;
        output.number((rgb[0]) as i128)?;
        output.character(',')?;
        output.number((rgb[1]) as i128)?;
        output.character(',')?;
        output.number((rgb[2]) as i128)?;
        output.character(']')?;
    }
    output.character('}')?;
    Ok(())
}
fn write_transform<O: SvgOutput>(
    output: &mut O,
    value: SafeVectorTransform,
) -> Result<(), O::Error> {
    output.text("{\"a\":")?;
    output.number((value.a) as i128)?;
    output.text(",\"d\":")?;
    output.number((value.d) as i128)?;
    output.text(",\"e\":")?;
    output.number((value.e) as i128)?;
    output.text(",\"f\":")?;
    output.number((value.f) as i128)?;
    output.character('}')?;
    Ok(())
}

fn write_path<O: SvgOutput>(output: &mut O, path: &SafeVectorPath) -> Result<(), O::Error> {
    output.character('[')?;
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            output.character(',')?;
        }
        output.text("{\"kind\":")?;
        output.quoted(segment.kind_str())?;
        match segment {
            SafeVectorSegment::Move(point) | SafeVectorSegment::Line(point) => {
                output.text(",\"points\":[")?;
                write_point(output, *point)?;
                output.character(']')?;
            }
            SafeVectorSegment::Quadratic(first, second) => {
                output.text(",\"points\":[")?;
                write_point(output, *first)?;
                output.character(',')?;
                write_point(output, *second)?;
                output.character(']')?;
            }
            SafeVectorSegment::Cubic(first, second, third) => {
                output.text(",\"points\":[")?;
                write_point(output, *first)?;
                output.character(',')?;
                write_point(output, *second)?;
                output.character(',')?;
                write_point(output, *third)?;
                output.character(']')?;
            }
            SafeVectorSegment::Close => output.text(",\"points\":[]")?,
        }
        output.character('}')?;
    }
    output.character(']')?;
    Ok(())
}

fn write_point<O: SvgOutput>(output: &mut O, point: SafeVectorPoint) -> Result<(), O::Error> {
    output.character('[')?;
    output.number((point.x) as i128)?;
    output.character(',')?;
    output.number((point.y) as i128)?;
    output.character(']')?;
    Ok(())
}
