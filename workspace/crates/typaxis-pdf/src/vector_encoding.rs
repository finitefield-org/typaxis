//! Allocation-free physical safe-vector operators shared by PDF owners.
use super::*;
use crate::font_encoding::Sink;

pub(crate) trait VectorSink: Sink {
    // Frozen callers retain their established operators. Book-2 preserves SVG
    // fill-then-stroke compositing when both paints carry independent alpha.
    fn separate_fill_stroke(&self) -> bool {
        false
    }
    fn state(&mut self, fill: u32, stroke: u32) -> Result<(), Self::Error>;
    fn push_str(&mut self, text: &str) -> Result<(), Self::Error> {
        self.extend(text.as_bytes())
    }
    fn formatted(&mut self, args: std::fmt::Arguments<'_>) -> Result<(), Self::Error>
    where
        Self: Sized,
    {
        struct Adapter<'s, S: Sink> {
            sink: &'s mut S,
            error: Option<S::Error>,
        }
        impl<S: Sink> std::fmt::Write for Adapter<'_, S> {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                self.sink.extend(text.as_bytes()).map_err(|e| {
                    self.error = Some(e);
                    std::fmt::Error
                })
            }
        }
        let mut adapter = Adapter {
            sink: self,
            error: None,
        };
        let result = std::fmt::write(&mut adapter, args);
        match adapter.error {
            Some(e) => Err(e),
            None => {
                debug_assert!(result.is_ok());
                Ok(())
            }
        }
    }
}
// Same canonical decimal as pdf_fixed, using a bounded stack fraction buffer.
struct Fixed(i64);
impl std::fmt::Display for Fixed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let magnitude = self.0.unsigned_abs();
        if self.0 < 0 {
            f.write_str("-")?;
        }
        write!(f, "{}", magnitude / 65536)?;
        let mut fraction = (magnitude % 65536) * 152_587_890_625;
        if fraction != 0 {
            let mut digits = [b'0'; 16];
            for digit in digits.iter_mut().rev() {
                *digit += (fraction % 10) as u8;
                fraction /= 10;
            }
            let mut end = digits.len();
            while digits[end - 1] == b'0' {
                end -= 1;
            }
            f.write_str(".")?;
            f.write_str(std::str::from_utf8(&digits[..end]).expect("decimal digits"))?;
        }
        Ok(())
    }
}
pub(crate) fn encode<S: VectorSink>(ir: &AdmittedSafeVector, output: &mut S) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    output.push_str("q\n0 0 ")?;
    output.formatted(format_args!(
        "{} {} re W n\n",
        Fixed(ir.intrinsic_width().get().raw()),
        Fixed(ir.intrinsic_height().get().raw())
    ))?;
    match ir {
        AdmittedSafeVector::V1(ir) => {
            encode_root_view_box(output, ir.view_box(), ir.root_scale_raw())?;
            for draw in ir.draws() {
                encode_v1_draw(output, ir, draw)?;
            }
        }
        AdmittedSafeVector::V2(ir) => {
            encode_root_view_box(output, ir.view_box(), ir.root_scale_raw())?;
            for draw in ir.draws() {
                encode_v2_draw(output, ir, draw)?;
            }
        }
    }
    output.push_str("Q")
}
fn encode_root_view_box<S: VectorSink>(
    output: &mut S,
    view_box: [i64; 4],
    root_scale: i32,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    let [min_x, min_y, width, height] = view_box;
    if root_scale <= 0 || width <= 0 || height <= 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    let scale = i64::from(root_scale);
    let tx = fixed_mul(scale, min_x)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let ty = fixed_mul(scale, min_y)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    output.formatted(format_args!(
        "{} 0 0 {} {} {} cm\n",
        Fixed(scale),
        Fixed(scale),
        Fixed(tx),
        Fixed(ty)
    ))
}

fn encode_v1_draw<S: VectorSink>(
    output: &mut S,
    ir: &SafeVectorIr,
    draw: &SafeVectorDraw,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    output.push_str("q\n")?;
    encode_draw_clips(output, ir.clips(), draw.clips())?;
    encode_transform(output, draw.transform())?;
    output.state(FIXED_ONE as u32, FIXED_ONE as u32)?;
    if let Some(fill) = draw.fill() {
        encode_rgb_operator(output, fill, "rg")?;
    }
    if let Some(stroke) = draw.stroke() {
        encode_rgb_operator(output, stroke.color(), "RG")?;
        encode_stroke_style(
            output,
            stroke.width_raw(),
            stroke.line_cap(),
            stroke.line_join(),
            stroke.miter_limit_raw(),
        )?;
    }
    encode_draw_path(
        output,
        draw.path(),
        draw.fill().is_some(),
        draw.stroke().is_some(),
        draw.fill_rule(),
    )?;
    output.push_str("Q\n")
}

fn encode_v2_draw<S: VectorSink>(
    output: &mut S,
    ir: &SafeVectorIrV2,
    draw: &SafeVectorDrawV2,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    output.push_str("q\n")?;
    encode_draw_clips(output, ir.clips(), draw.clips())?;
    encode_transform(output, draw.transform())?;
    let fill = draw.fill();
    let stroke = draw.stroke();
    output.state(fill.alpha().raw(), stroke.paint().alpha().raw())?;
    encode_optional_paint(output, fill.paint(), "rg")?;
    encode_optional_paint(output, stroke.paint().paint(), "RG")?;
    if stroke.paint().paint().enabled() {
        encode_stroke_style(
            output,
            stroke.width_raw(),
            stroke.line_cap(),
            stroke.line_join(),
            stroke.miter_limit_raw(),
        )?;
    }
    encode_draw_path(
        output,
        draw.path(),
        fill.paint().enabled(),
        stroke.paint().paint().enabled(),
        draw.fill_rule(),
    )?;
    output.push_str("Q\n")
}

fn encode_draw_clips<S: VectorSink>(
    output: &mut S,
    definitions: &[SafeVectorClipDefinition],
    uses: &[SafeVectorClipUse],
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    for clip_use in uses {
        let definition = definitions
            .get(clip_use.clip_id() as usize)
            .filter(|definition| definition.clip_id() == clip_use.clip_id())
            .ok_or(StagingSafeVectorPdfV2Error::InvalidIr)?;
        encode_path(
            output,
            definition.path(),
            Some((definition.transform(), clip_use.transform())),
        )?;
        output.push_str(match definition.fill_rule() {
            SafeVectorFillRule::NonZero => "W n\n",
            SafeVectorFillRule::EvenOdd => "W* n\n",
        })?;
    }
    Ok(())
}

fn encode_transform<S: VectorSink>(
    output: &mut S,
    transform: SafeVectorTransform,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    if transform.a_raw() == 0 || transform.d_raw() == 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    output.formatted(format_args!(
        "{} 0 0 {} {} {} cm\n",
        Fixed(i64::from(transform.a_raw())),
        Fixed(i64::from(transform.d_raw())),
        Fixed(transform.e_raw()),
        Fixed(transform.f_raw())
    ))
}

fn encode_optional_paint<S: VectorSink>(
    output: &mut S,
    paint: SafeVectorPaint,
    operator: &str,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    match paint {
        SafeVectorPaint::None | SafeVectorPaint::CurrentColor => Ok(()),
        SafeVectorPaint::FixedRgb8(color) => encode_rgb_operator(output, color, operator),
    }
}

pub(super) fn encode_rgb_operator<S: VectorSink>(
    output: &mut S,
    color: [u8; 3],
    operator: &str,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    output.formatted(format_args!(
        "{} {} {} {}\n",
        Fixed(color_fixed(color[0])?),
        Fixed(color_fixed(color[1])?),
        Fixed(color_fixed(color[2])?),
        operator
    ))
}

fn encode_stroke_style<S: VectorSink>(
    output: &mut S,
    width: i64,
    line_cap: SafeVectorLineCap,
    line_join: SafeVectorLineJoin,
    miter_limit: i64,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    if width <= 0 || miter_limit <= 0 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    output.formatted(format_args!(
        "{} w\n{} J\n{} j\n{} M\n",
        Fixed(width),
        match line_cap {
            SafeVectorLineCap::Butt => 0,
            SafeVectorLineCap::Round => 1,
            SafeVectorLineCap::Square => 2,
        },
        match line_join {
            SafeVectorLineJoin::Miter => 0,
            SafeVectorLineJoin::Round => 1,
            SafeVectorLineJoin::Bevel => 2,
        },
        Fixed(miter_limit)
    ))
}

fn encode_draw_path<S: VectorSink>(
    output: &mut S,
    path: &SafeVectorPath,
    fill: bool,
    stroke: bool,
    rule: SafeVectorFillRule,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    encode_path(output, path, None)?;
    if fill && stroke && output.separate_fill_stroke() {
        encode_paint_operator(output, true, false, rule)?;
        encode_path(output, path, None)?;
        encode_paint_operator(output, false, true, rule)
    } else {
        encode_paint_operator(output, fill, stroke, rule)
    }
}
fn encode_paint_operator<S: VectorSink>(
    output: &mut S,
    fill: bool,
    stroke: bool,
    fill_rule: SafeVectorFillRule,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    output.push_str(match (fill, stroke, fill_rule) {
        (true, true, SafeVectorFillRule::NonZero) => "B\n",
        (true, true, SafeVectorFillRule::EvenOdd) => "B*\n",
        (true, false, SafeVectorFillRule::NonZero) => "f\n",
        (true, false, SafeVectorFillRule::EvenOdd) => "f*\n",
        (false, true, _) => "S\n",
        (false, false, _) => return Err(StagingSafeVectorPdfV2Error::InvalidIr.into()),
    })
}

fn encode_path<S: VectorSink>(
    output: &mut S,
    path: &SafeVectorPath,
    transform: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    let mut current = None;
    let mut subpath = None;
    for segment in path.segments() {
        match segment {
            SafeVectorSegment::Move(point) => {
                let point = maybe_transform(*point, transform)?;
                output.formatted(format_args!("{} {} m\n", Fixed(point.x), Fixed(point.y)))?;
                current = Some(point);
                subpath = Some(point);
            }
            SafeVectorSegment::Line(point) => {
                let point = maybe_transform(*point, transform)?;
                output.formatted(format_args!("{} {} l\n", Fixed(point.x), Fixed(point.y)))?;
                current = Some(point);
            }
            SafeVectorSegment::Quadratic(control, endpoint) => {
                let start = current.ok_or(StagingSafeVectorPdfV2Error::InvalidIr)?;
                let control = maybe_transform(*control, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                let first = RawPoint {
                    x: rational_third(start.x, control.x)?,
                    y: rational_third(start.y, control.y)?,
                };
                let second = RawPoint {
                    x: rational_third(endpoint.x, control.x)?,
                    y: rational_third(endpoint.y, control.y)?,
                };
                output.formatted(format_args!(
                    "{} {} {} {} {} {} c\n",
                    Fixed(first.x),
                    Fixed(first.y),
                    Fixed(second.x),
                    Fixed(second.y),
                    Fixed(endpoint.x),
                    Fixed(endpoint.y)
                ))?;
                current = Some(endpoint);
            }
            SafeVectorSegment::Cubic(first, second, endpoint) => {
                let first = maybe_transform(*first, transform)?;
                let second = maybe_transform(*second, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                output.formatted(format_args!(
                    "{} {} {} {} {} {} c\n",
                    Fixed(first.x),
                    Fixed(first.y),
                    Fixed(second.x),
                    Fixed(second.y),
                    Fixed(endpoint.x),
                    Fixed(endpoint.y)
                ))?;
                current = Some(endpoint);
            }
            SafeVectorSegment::Close => {
                output.push_str("h\n")?;
                current = subpath;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct RawPoint {
    x: i64,
    y: i64,
}

fn maybe_transform(
    point: SafeVectorPoint,
    transforms: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<RawPoint, StagingSafeVectorPdfV2Error> {
    let point = RawPoint {
        x: point.x_raw(),
        y: point.y_raw(),
    };
    let Some((definition, use_site)) = transforms else {
        return Ok(point);
    };
    let transform = compose_fixed_transform(raw_transform(use_site), raw_transform(definition))?;
    apply_fixed_transform(point, transform)
}

const fn raw_transform(transform: SafeVectorTransform) -> [i64; 4] {
    [
        transform.a_raw() as i64,
        transform.d_raw() as i64,
        transform.e_raw(),
        transform.f_raw(),
    ]
}

fn compose_fixed_transform(
    left: [i64; 4],
    right: [i64; 4],
) -> Result<[i64; 4], StagingSafeVectorPdfV2Error> {
    let a = fixed_mul(left[0], right[0])?;
    let d = fixed_mul(left[1], right[1])?;
    let e = fixed_mul(left[0], right[2])?
        .checked_add(left[2])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let f = fixed_mul(left[1], right[3])?
        .checked_add(left[3])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    if a == 0
        || d == 0
        || i32::try_from(a).is_err()
        || i32::try_from(d).is_err()
        || e.abs() > MAX_COORDINATE
        || f.abs() > MAX_COORDINATE
    {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    Ok([a, d, e, f])
}

fn apply_fixed_transform(
    point: RawPoint,
    transform: [i64; 4],
) -> Result<RawPoint, StagingSafeVectorPdfV2Error> {
    let x = fixed_mul(transform[0], point.x)?
        .checked_add(transform[2])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let y = fixed_mul(transform[1], point.y)?
        .checked_add(transform[3])
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    if x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    Ok(RawPoint { x, y })
}

fn rational_third(endpoint: i64, control: i64) -> Result<i64, StagingSafeVectorPdfV2Error> {
    let numerator = i128::from(endpoint)
        .checked_add(
            i128::from(control)
                .checked_mul(2)
                .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, 3)?)
        .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

pub(super) fn fixed_mul(left: i64, right: i64) -> Result<i64, StagingSafeVectorPdfV2Error> {
    let numerator = i128::from(left)
        .checked_mul(i128::from(right))
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, i128::from(FIXED_ONE))?)
        .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

fn color_fixed(byte: u8) -> Result<i64, StagingSafeVectorPdfV2Error> {
    i64::try_from(round_ties_even(
        i128::from(byte) * i128::from(FIXED_ONE),
        255,
    )?)
    .map_err(|_| StagingSafeVectorPdfV2Error::ArithmeticOverflow)
}

fn round_ties_even(
    numerator: i128,
    denominator: i128,
) -> Result<i128, StagingSafeVectorPdfV2Error> {
    if denominator <= 0 {
        return Err(StagingSafeVectorPdfV2Error::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)?;
    let denominator = denominator as u128;
    if twice < denominator || (twice == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(if remainder > 0 { 1 } else { -1 })
            .ok_or(StagingSafeVectorPdfV2Error::ArithmeticOverflow)
    }
}

pub(crate) fn state_dictionary<S: Sink>(out: &mut S, fill: u32, stroke: u32) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    if fill > 65536 || stroke > 65536 {
        return Err(StagingSafeVectorPdfV2Error::InvalidIr.into());
    }
    out.extend(b"<< /Type /ExtGState /ca ")?;
    crate::text_encoding::number(out, i64::from(fill))?;
    out.extend(b" /CA ")?;
    crate::text_encoding::number(out, i64::from(stroke))?;
    out.extend(b" >>")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_vector_fixed_formatter_preserves_canonical_signed_coordinates() {
        for raw in [
            i64::MIN,
            i64::MAX,
            -65537,
            -65536,
            -1,
            0,
            1,
            32768,
            65535,
            65536,
            65537,
        ] {
            assert_eq!(Fixed(raw).to_string(), super::super::pdf_fixed(raw));
        }
    }
}

pub(crate) fn placement<S: VectorSink>(
    out: &mut S,
    matrix: AffineTransform,
    color: [u8; 3],
    name: impl FnOnce(&mut S) -> Result<(), S::Error>,
) -> Result<(), S::Error>
where
    S::Error: From<StagingSafeVectorPdfV2Error>,
{
    out.push_str("q\n")?;
    encode_rgb_operator(out, color, "rg")?;
    encode_rgb_operator(out, color, "RG")?;
    for (i, raw) in [
        i64::from(matrix.a.raw()),
        i64::from(matrix.b.raw()),
        i64::from(matrix.c.raw()),
        i64::from(matrix.d.raw()),
        matrix.e.raw(),
        matrix.f.raw(),
    ]
    .into_iter()
    .enumerate()
    {
        if i > 0 {
            out.push_str(" ")?;
        }
        out.formatted(format_args!("{}", Fixed(raw)))?;
    }
    out.push_str(" cm\n/")?;
    name(out)?;
    out.push_str(" Do\nQ")
}
