use std::collections::BTreeSet;
use typaxis_core::{push_jcs_string, sha256, ImageResourceId, M4EffectiveResourceLimits};
use typaxis_display_list::StagingSafeVectorDisplay;
use typaxis_resources::{FrozenSafeVectorFormPlan, StagingSafeVectorFormPlans};
use typaxis_resources::{
    SafeVectorClipUse, SafeVectorFillRule, SafeVectorLineCap, SafeVectorLineJoin, SafeVectorPath,
    SafeVectorPoint, SafeVectorSegment, SafeVectorTransform,
};

pub const STAGING_SAFE_VECTOR_PDF_ALGORITHM: &str = "typaxis.safe-vector-pdf-closure/1";
const FIXED_ONE: i64 = 65_536;
const MAX_COORDINATE: i64 = 1_000_000 * FIXED_ONE;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfFormObject {
    image_id: ImageResourceId,
    object_number: u32,
    resource_name: String,
    admitted_sha256: [u8; 32],
    ir_fingerprint: [u8; 32],
    form_plan_fingerprint: [u8; 32],
    content_stream_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfFormObject {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn object_number(&self) -> u32 {
        self.object_number
    }
    pub fn resource_name(&self) -> &str {
        &self.resource_name
    }
    pub const fn admitted_sha256(&self) -> [u8; 32] {
        self.admitted_sha256
    }
    pub const fn ir_fingerprint(&self) -> [u8; 32] {
        self.ir_fingerprint
    }
    pub const fn form_plan_fingerprint(&self) -> [u8; 32] {
        self.form_plan_fingerprint
    }
    pub const fn content_stream_fingerprint(&self) -> [u8; 32] {
        self.content_stream_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfUsage {
    occurrence: u32,
    page_index: u32,
    page_object_number: u32,
    content_object_number: u32,
    form_object_number: u32,
    display_command_fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfUsage {
    pub const fn occurrence(&self) -> u32 {
        self.occurrence
    }
    pub const fn page_index(&self) -> u32 {
        self.page_index
    }
    pub const fn page_object_number(&self) -> u32 {
        self.page_object_number
    }
    pub const fn content_object_number(&self) -> u32 {
        self.content_object_number
    }
    pub const fn form_object_number(&self) -> u32 {
        self.form_object_number
    }
    pub const fn display_command_fingerprint(&self) -> [u8; 32] {
        self.display_command_fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdfReceipt {
    display_fingerprint: [u8; 32],
    form_plans_fingerprint: [u8; 32],
    limits_fingerprint: [u8; 32],
    pdf_sha256: [u8; 32],
    byte_length: u64,
    object_count: u32,
    canonical_jcs: String,
    fingerprint: [u8; 32],
}

impl StagingSafeVectorPdfReceipt {
    pub const fn display_fingerprint(&self) -> [u8; 32] {
        self.display_fingerprint
    }
    pub const fn form_plans_fingerprint(&self) -> [u8; 32] {
        self.form_plans_fingerprint
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn pdf_sha256(&self) -> [u8; 32] {
        self.pdf_sha256
    }
    pub const fn byte_length(&self) -> u64 {
        self.byte_length
    }
    pub const fn object_count(&self) -> u32 {
        self.object_count
    }
    pub fn canonical_jcs(&self) -> &str {
        &self.canonical_jcs
    }
    pub const fn fingerprint(&self) -> [u8; 32] {
        self.fingerprint
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagingSafeVectorPdf {
    bytes: Vec<u8>,
    forms: Vec<StagingSafeVectorPdfFormObject>,
    usages: Vec<StagingSafeVectorPdfUsage>,
    receipt: StagingSafeVectorPdfReceipt,
}

impl StagingSafeVectorPdf {
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn forms(&self) -> &[StagingSafeVectorPdfFormObject] {
        &self.forms
    }
    pub fn usages(&self) -> &[StagingSafeVectorPdfUsage] {
        &self.usages
    }
    pub const fn receipt(&self) -> &StagingSafeVectorPdfReceipt {
        &self.receipt
    }

    pub fn verify(
        &self,
        display: &StagingSafeVectorDisplay,
        plans: &StagingSafeVectorFormPlans,
        limits: &M4EffectiveResourceLimits,
    ) -> Result<(), StagingSafeVectorPdfError> {
        plans
            .verify_pdf_closure(display, limits)
            .map_err(|_| StagingSafeVectorPdfError::PlanMismatch)?;
        let canonical = encode_receipt(
            display.receipt().fingerprint(),
            plans.fingerprint(),
            limits.fingerprint(),
            &self.forms,
            &self.usages,
            sha256(&self.bytes),
            self.receipt.object_count,
            self.bytes.len() as u64,
        );
        let expected_object_count = preflight_object_count(
            display.pages().len(),
            plans.plans().len(),
            limits.base().get().max_pdf_objects,
        )?;
        if self.receipt.display_fingerprint != display.receipt().fingerprint()
            || self.receipt.form_plans_fingerprint != plans.fingerprint()
            || self.receipt.limits_fingerprint != limits.fingerprint()
            || self.receipt.pdf_sha256 != sha256(&self.bytes)
            || self.receipt.byte_length != self.bytes.len() as u64
            || self.receipt.byte_length > limits.base().get().max_output_bytes
            || self.receipt.object_count != expected_object_count
            || self.receipt.canonical_jcs != canonical
            || self.receipt.fingerprint != sha256(canonical.as_bytes())
            || self.forms.len() != plans.plans().len()
            || self.usages.len() != display.receipt().command_count() as usize
        {
            return Err(StagingSafeVectorPdfError::ReceiptMismatch);
        }
        let form_start = 3usize
            .checked_add(
                display
                    .pages()
                    .len()
                    .checked_mul(2)
                    .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?,
            )
            .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?;
        for (index, (form, plan)) in self.forms.iter().zip(plans.plans()).enumerate() {
            let expected_object_number = u32::try_from(form_start + index)
                .map_err(|_| StagingSafeVectorPdfError::ReceiptMismatch)?;
            if form.image_id != plan.image_id()
                || form.object_number != expected_object_number
                || form.resource_name != format!("V{index}")
                || form.admitted_sha256 != plan.admitted_sha256()
                || form.ir_fingerprint != plan.ir_fingerprint()
                || form.form_plan_fingerprint != plan.fingerprint()
                || form.content_stream_fingerprint != sha256(encode_form_content(plan)?.as_bytes())
            {
                return Err(StagingSafeVectorPdfError::ReceiptMismatch);
            }
        }
        for (index, usage) in self.usages.iter().enumerate() {
            let command = display
                .commands()
                .find(|command| command.occurrence() == usage.occurrence)
                .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?;
            let form = self
                .forms
                .iter()
                .find(|form| form.image_id == command.image_id())
                .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?;
            let expected_page_object = command
                .page_index()
                .checked_mul(2)
                .and_then(|value| value.checked_add(3))
                .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?;
            let expected_content_object = expected_page_object
                .checked_add(1)
                .ok_or(StagingSafeVectorPdfError::ReceiptMismatch)?;
            if usize::try_from(usage.occurrence) != Ok(index)
                || usage.page_index != command.page_index()
                || usage.page_object_number != expected_page_object
                || usage.content_object_number != expected_content_object
                || usage.form_object_number != form.object_number
                || usage.display_command_fingerprint != command.fingerprint()
            {
                return Err(StagingSafeVectorPdfError::ReceiptMismatch);
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StagingSafeVectorPdfError {
    PlanMismatch,
    ObjectLimit,
    OutputLimit,
    InvalidIr,
    ArithmeticOverflow,
    ReceiptMismatch,
    AllocationFailure,
}

impl std::fmt::Display for StagingSafeVectorPdfError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PlanMismatch => formatter.write_str("I9190: SafeVector Form plan mismatch"),
            Self::ObjectLimit => formatter.write_str("G6100: SafeVector PDF object limit exceeded"),
            Self::OutputLimit => formatter.write_str("D8101: SafeVector PDF output limit exceeded"),
            Self::InvalidIr => {
                formatter.write_str("I9190: SafeVector IR is invalid at PDF closure")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("I9190: SafeVector PDF arithmetic overflow")
            }
            Self::ReceiptMismatch => formatter.write_str("I9190: SafeVector PDF receipt mismatch"),
            Self::AllocationFailure => {
                formatter.write_str("D8101: SafeVector PDF allocation failed")
            }
        }
    }
}

impl std::error::Error for StagingSafeVectorPdfError {}

pub fn write_staging_safe_vector_pdf(
    display: &StagingSafeVectorDisplay,
    plans: &StagingSafeVectorFormPlans,
    limits: &M4EffectiveResourceLimits,
) -> Result<StagingSafeVectorPdf, StagingSafeVectorPdfError> {
    plans
        .verify_pdf_closure(display, limits)
        .map_err(|_| StagingSafeVectorPdfError::PlanMismatch)?;
    let page_count = display.pages().len();
    let page_width = display.page_geometry().page_width();
    let page_height = display.page_geometry().page_height();
    let object_count_u32 = preflight_object_count(
        page_count,
        plans.plans().len(),
        limits.base().get().max_pdf_objects,
    )?;
    let object_count = object_count_u32 as usize;
    let form_start = 3usize
        .checked_add(
            page_count
                .checked_mul(2)
                .ok_or(StagingSafeVectorPdfError::ObjectLimit)?,
        )
        .ok_or(StagingSafeVectorPdfError::ObjectLimit)?;
    let mut objects = Vec::new();
    objects
        .try_reserve_exact(object_count)
        .map_err(|_| StagingSafeVectorPdfError::AllocationFailure)?;
    objects.resize_with(object_count, Vec::new);
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    let mut kids = String::from("[");
    for index in 0..page_count {
        if index > 0 {
            kids.push(' ');
        }
        kids.push_str(&(3 + index * 2).to_string());
        kids.push_str(" 0 R");
    }
    kids.push(']');
    objects[1] = format!("<< /Type /Pages /Count {page_count} /Kids {kids} >>").into_bytes();

    let mut forms = Vec::new();
    forms
        .try_reserve_exact(plans.plans().len())
        .map_err(|_| StagingSafeVectorPdfError::AllocationFailure)?;
    for (index, plan) in plans.plans().iter().enumerate() {
        let content = encode_form_content(plan)?;
        let object_number = u32::try_from(form_start + index)
            .map_err(|_| StagingSafeVectorPdfError::ObjectLimit)?;
        let resource_name = format!("V{index}");
        let dictionary = format!(
            "<< /Type /XObject /Subtype /Form /FormType 1 /BBox [0 0 {} {}] /Resources << >> /Length {} >>\nstream\n",
            pdf_fixed(plan.ir().intrinsic_width().get().raw()),
            pdf_fixed(plan.ir().intrinsic_height().get().raw()),
            content.len()
        );
        let mut object = dictionary.into_bytes();
        object.extend_from_slice(content.as_bytes());
        object.extend_from_slice(b"\nendstream");
        objects[form_start + index - 1] = object;
        forms.push(StagingSafeVectorPdfFormObject {
            image_id: plan.image_id(),
            object_number,
            resource_name,
            admitted_sha256: plan.admitted_sha256(),
            ir_fingerprint: plan.ir_fingerprint(),
            form_plan_fingerprint: plan.fingerprint(),
            content_stream_fingerprint: sha256(content.as_bytes()),
        });
    }

    let mut usages = Vec::new();
    usages
        .try_reserve_exact(display.receipt().command_count() as usize)
        .map_err(|_| StagingSafeVectorPdfError::AllocationFailure)?;
    for (page_index, page) in display.pages().iter().enumerate() {
        let page_object_number = u32::try_from(3 + page_index * 2)
            .map_err(|_| StagingSafeVectorPdfError::ObjectLimit)?;
        let content_object_number = page_object_number
            .checked_add(1)
            .ok_or(StagingSafeVectorPdfError::ObjectLimit)?;
        let mut resources = String::from("<< /XObject <<");
        let mut page_images = BTreeSet::new();
        for command in page.commands() {
            if !page_images.insert(command.image_id()) {
                continue;
            }
            let form = forms
                .iter()
                .find(|form| form.image_id == command.image_id())
                .ok_or(StagingSafeVectorPdfError::PlanMismatch)?;
            resources.push_str(&format!(
                " /{} {} 0 R",
                form.resource_name, form.object_number
            ));
        }
        resources.push_str(" >> >>");
        objects[page_object_number as usize - 1] = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources {} /Contents {} 0 R >>",
            pdf_fixed(page_width.get().raw()),
            pdf_fixed(page_height.get().raw()),
            resources,
            content_object_number
        )
        .into_bytes();
        let mut content = format!("q\n1 0 0 -1 0 {} cm\n", pdf_fixed(page_height.get().raw()));
        for command in page.commands() {
            let form = forms
                .iter()
                .find(|form| form.image_id == command.image_id())
                .ok_or(StagingSafeVectorPdfError::PlanMismatch)?;
            let plan = plans
                .plan(command.image_id())
                .ok_or(StagingSafeVectorPdfError::PlanMismatch)?;
            let scale = command.scale_raw();
            if scale <= 0
                || fixed_mul(i64::from(scale), plan.ir().intrinsic_width().get().raw())?
                    != command.bounds().width().get().raw()
                || fixed_mul(i64::from(scale), plan.ir().intrinsic_height().get().raw())?
                    != command.bounds().height().get().raw()
            {
                return Err(StagingSafeVectorPdfError::InvalidIr);
            }
            content.push_str(&format!(
                "q\n{} 0 0 {} {} {} cm\n/{} Do\nQ\n",
                pdf_fixed(i64::from(scale)),
                pdf_fixed(i64::from(scale)),
                pdf_fixed(command.bounds().x().raw()),
                pdf_fixed(command.bounds().y().raw()),
                form.resource_name
            ));
            usages.push(StagingSafeVectorPdfUsage {
                occurrence: command.occurrence(),
                page_index: command.page_index(),
                page_object_number,
                content_object_number,
                form_object_number: form.object_number,
                display_command_fingerprint: command.fingerprint(),
            });
        }
        content.push('Q');
        let stream = format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            content.len(),
            content
        );
        objects[content_object_number as usize - 1] = stream.into_bytes();
    }
    usages.sort_by_key(|usage| usage.occurrence);
    let bytes = serialize_objects(&objects, limits.base().get().max_output_bytes)?;
    let canonical_jcs = encode_receipt(
        display.receipt().fingerprint(),
        plans.fingerprint(),
        limits.fingerprint(),
        &forms,
        &usages,
        sha256(&bytes),
        object_count_u32,
        bytes.len() as u64,
    );
    let pdf = StagingSafeVectorPdf {
        receipt: StagingSafeVectorPdfReceipt {
            display_fingerprint: display.receipt().fingerprint(),
            form_plans_fingerprint: plans.fingerprint(),
            limits_fingerprint: limits.fingerprint(),
            pdf_sha256: sha256(&bytes),
            byte_length: bytes.len() as u64,
            object_count: object_count_u32,
            fingerprint: sha256(canonical_jcs.as_bytes()),
            canonical_jcs,
        },
        bytes,
        forms,
        usages,
    };
    pdf.verify(display, plans, limits)?;
    Ok(pdf)
}

fn encode_form_content(
    plan: &FrozenSafeVectorFormPlan,
) -> Result<String, StagingSafeVectorPdfError> {
    let ir = plan.ir();
    let mut output = format!(
        "q\n0 0 {} {} re W n\n",
        pdf_fixed(ir.intrinsic_width().get().raw()),
        pdf_fixed(ir.intrinsic_height().get().raw())
    );
    let [min_x, min_y, _, _] = ir.view_box();
    let scale = i64::from(ir.root_scale_raw());
    let tx = fixed_mul(scale, min_x)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    let ty = fixed_mul(scale, min_y)?
        .checked_neg()
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    output.push_str(&format!(
        "{} 0 0 {} {} {} cm\n",
        pdf_fixed(scale),
        pdf_fixed(scale),
        pdf_fixed(tx),
        pdf_fixed(ty)
    ));
    let mut active_clips: Vec<SafeVectorClipUse> = Vec::new();
    for draw in ir.draws() {
        let common = active_clips
            .iter()
            .zip(draw.clips())
            .take_while(|(left, right)| left == right)
            .count();
        for _ in common..active_clips.len() {
            output.push_str("Q\n");
        }
        active_clips.truncate(common);
        for clip_use in &draw.clips()[common..] {
            output.push_str("q\n");
            let definition = ir
                .clips()
                .get(clip_use.clip_id() as usize)
                .filter(|definition| definition.clip_id() == clip_use.clip_id())
                .ok_or(StagingSafeVectorPdfError::InvalidIr)?;
            encode_path(
                &mut output,
                definition.path(),
                Some((definition.transform(), clip_use.transform())),
            )?;
            output.push_str(match definition.fill_rule() {
                SafeVectorFillRule::NonZero => "W n\n",
                SafeVectorFillRule::EvenOdd => "W* n\n",
            });
            active_clips.push(*clip_use);
        }
        output.push_str("q\n");
        let transform = draw.transform();
        output.push_str(&format!(
            "{} 0 0 {} {} {} cm\n",
            pdf_fixed(i64::from(transform.a_raw())),
            pdf_fixed(i64::from(transform.d_raw())),
            pdf_fixed(transform.e_raw()),
            pdf_fixed(transform.f_raw())
        ));
        if let Some(fill) = draw.fill() {
            output.push_str(&format!(
                "{} {} {} rg\n",
                pdf_fixed(color_fixed(fill[0])?),
                pdf_fixed(color_fixed(fill[1])?),
                pdf_fixed(color_fixed(fill[2])?)
            ));
        }
        if let Some(stroke) = draw.stroke() {
            let color = stroke.color();
            output.push_str(&format!(
                "{} {} {} RG\n{} w\n{} J\n{} j\n{} M\n",
                pdf_fixed(color_fixed(color[0])?),
                pdf_fixed(color_fixed(color[1])?),
                pdf_fixed(color_fixed(color[2])?),
                pdf_fixed(stroke.width_raw()),
                match stroke.line_cap() {
                    SafeVectorLineCap::Butt => 0,
                    SafeVectorLineCap::Round => 1,
                    SafeVectorLineCap::Square => 2,
                },
                match stroke.line_join() {
                    SafeVectorLineJoin::Miter => 0,
                    SafeVectorLineJoin::Round => 1,
                    SafeVectorLineJoin::Bevel => 2,
                },
                pdf_fixed(stroke.miter_limit_raw())
            ));
        }
        encode_path(&mut output, draw.path(), None)?;
        output.push_str(
            match (
                draw.fill().is_some(),
                draw.stroke().is_some(),
                draw.fill_rule(),
            ) {
                (true, true, SafeVectorFillRule::NonZero) => "B\n",
                (true, true, SafeVectorFillRule::EvenOdd) => "B*\n",
                (true, false, SafeVectorFillRule::NonZero) => "f\n",
                (true, false, SafeVectorFillRule::EvenOdd) => "f*\n",
                (false, true, _) => "S\n",
                (false, false, _) => return Err(StagingSafeVectorPdfError::InvalidIr),
            },
        );
        output.push_str("Q\n");
    }
    for _ in 0..active_clips.len() {
        output.push_str("Q\n");
    }
    output.push('Q');
    Ok(output)
}

fn preflight_object_count(
    page_count: usize,
    form_count: usize,
    maximum: u32,
) -> Result<u32, StagingSafeVectorPdfError> {
    let object_count = 2usize
        .checked_add(
            page_count
                .checked_mul(2)
                .ok_or(StagingSafeVectorPdfError::ObjectLimit)?,
        )
        .and_then(|value| value.checked_add(form_count))
        .ok_or(StagingSafeVectorPdfError::ObjectLimit)?;
    let object_count =
        u32::try_from(object_count).map_err(|_| StagingSafeVectorPdfError::ObjectLimit)?;
    if object_count > maximum {
        return Err(StagingSafeVectorPdfError::ObjectLimit);
    }
    Ok(object_count)
}

fn encode_path(
    output: &mut String,
    path: &SafeVectorPath,
    transform: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<(), StagingSafeVectorPdfError> {
    let mut current = None;
    let mut subpath = None;
    for segment in path.segments() {
        match segment {
            SafeVectorSegment::Move(point) => {
                let point = maybe_transform(*point, transform)?;
                output.push_str(&format!(
                    "{} {} m\n",
                    pdf_fixed(point.x_raw()),
                    pdf_fixed(point.y_raw())
                ));
                current = Some(point);
                subpath = Some(point);
            }
            SafeVectorSegment::Line(point) => {
                let point = maybe_transform(*point, transform)?;
                output.push_str(&format!(
                    "{} {} l\n",
                    pdf_fixed(point.x_raw()),
                    pdf_fixed(point.y_raw())
                ));
                current = Some(point);
            }
            SafeVectorSegment::Quadratic(control, endpoint) => {
                let start = current.ok_or(StagingSafeVectorPdfError::InvalidIr)?;
                let control = maybe_transform(*control, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                let first = SafeVectorPointPdf {
                    x: rational_third(start.x_raw(), control.x_raw())?,
                    y: rational_third(start.y_raw(), control.y_raw())?,
                };
                let second = SafeVectorPointPdf {
                    x: rational_third(endpoint.x_raw(), control.x_raw())?,
                    y: rational_third(endpoint.y_raw(), control.y_raw())?,
                };
                output.push_str(&format!(
                    "{} {} {} {} {} {} c\n",
                    pdf_fixed(first.x),
                    pdf_fixed(first.y),
                    pdf_fixed(second.x),
                    pdf_fixed(second.y),
                    pdf_fixed(endpoint.x_raw()),
                    pdf_fixed(endpoint.y_raw())
                ));
                current = Some(endpoint);
            }
            SafeVectorSegment::Cubic(first, second, endpoint) => {
                let first = maybe_transform(*first, transform)?;
                let second = maybe_transform(*second, transform)?;
                let endpoint = maybe_transform(*endpoint, transform)?;
                output.push_str(&format!(
                    "{} {} {} {} {} {} c\n",
                    pdf_fixed(first.x_raw()),
                    pdf_fixed(first.y_raw()),
                    pdf_fixed(second.x_raw()),
                    pdf_fixed(second.y_raw()),
                    pdf_fixed(endpoint.x_raw()),
                    pdf_fixed(endpoint.y_raw())
                ));
                current = Some(endpoint);
            }
            SafeVectorSegment::Close => {
                output.push_str("h\n");
                current = subpath;
            }
        }
    }
    Ok(())
}

struct SafeVectorPointPdf {
    x: i64,
    y: i64,
}

fn rational_third(endpoint: i64, control: i64) -> Result<i64, StagingSafeVectorPdfError> {
    let numerator = i128::from(endpoint)
        .checked_add(
            i128::from(control)
                .checked_mul(2)
                .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?,
        )
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, 3)?)
        .map_err(|_| StagingSafeVectorPdfError::ArithmeticOverflow)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RawPoint {
    x: i64,
    y: i64,
}

impl RawPoint {
    const fn x_raw(self) -> i64 {
        self.x
    }
    const fn y_raw(self) -> i64 {
        self.y
    }
}

fn maybe_transform(
    point: SafeVectorPoint,
    transforms: Option<(SafeVectorTransform, SafeVectorTransform)>,
) -> Result<RawPoint, StagingSafeVectorPdfError> {
    let point = RawPoint {
        x: point.x_raw(),
        y: point.y_raw(),
    };
    let Some((definition, use_site)) = transforms else {
        return Ok(point);
    };
    // Admission defines clip geometry under
    // `element_ctm * clip_geometry_transform`. Recompose that one fixed CTM
    // here instead of rounding the point once per source transform.
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
) -> Result<[i64; 4], StagingSafeVectorPdfError> {
    let a = fixed_mul(left[0], right[0])?;
    let d = fixed_mul(left[1], right[1])?;
    let e = fixed_mul(left[0], right[2])?
        .checked_add(left[2])
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    let f = fixed_mul(left[1], right[3])?
        .checked_add(left[3])
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    if a == 0
        || d == 0
        || i32::try_from(a).is_err()
        || i32::try_from(d).is_err()
        || e.abs() > MAX_COORDINATE
        || f.abs() > MAX_COORDINATE
    {
        return Err(StagingSafeVectorPdfError::InvalidIr);
    }
    Ok([a, d, e, f])
}

fn apply_fixed_transform(
    point: RawPoint,
    transform: [i64; 4],
) -> Result<RawPoint, StagingSafeVectorPdfError> {
    let x = fixed_mul(transform[0], point.x)?
        .checked_add(transform[2])
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    let y = fixed_mul(transform[1], point.y)?
        .checked_add(transform[3])
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    if x.abs() > MAX_COORDINATE || y.abs() > MAX_COORDINATE {
        return Err(StagingSafeVectorPdfError::InvalidIr);
    }
    Ok(RawPoint { x, y })
}

fn fixed_mul(left: i64, right: i64) -> Result<i64, StagingSafeVectorPdfError> {
    let numerator = i128::from(left)
        .checked_mul(i128::from(right))
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    i64::try_from(round_ties_even(numerator, i128::from(FIXED_ONE))?)
        .map_err(|_| StagingSafeVectorPdfError::ArithmeticOverflow)
}

fn color_fixed(byte: u8) -> Result<i64, StagingSafeVectorPdfError> {
    i64::try_from(round_ties_even(
        i128::from(byte) * i128::from(FIXED_ONE),
        255,
    )?)
    .map_err(|_| StagingSafeVectorPdfError::ArithmeticOverflow)
}

fn round_ties_even(numerator: i128, denominator: i128) -> Result<i128, StagingSafeVectorPdfError> {
    if denominator <= 0 {
        return Err(StagingSafeVectorPdfError::ArithmeticOverflow);
    }
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return Ok(quotient);
    }
    let twice = remainder
        .unsigned_abs()
        .checked_mul(2)
        .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)?;
    let denominator = denominator as u128;
    if twice < denominator || (twice == denominator && quotient % 2 == 0) {
        Ok(quotient)
    } else {
        quotient
            .checked_add(if remainder > 0 { 1 } else { -1 })
            .ok_or(StagingSafeVectorPdfError::ArithmeticOverflow)
    }
}

fn pdf_fixed(raw: i64) -> String {
    const DECIMAL_SCALE: u64 = 10_000_000_000_000_000;
    const BINARY_TO_DECIMAL: u64 = 152_587_890_625;
    let negative = raw < 0;
    let magnitude = raw.unsigned_abs();
    let whole = magnitude / FIXED_ONE as u64;
    let remainder = magnitude % FIXED_ONE as u64;
    if remainder == 0 {
        return if negative {
            format!("-{whole}")
        } else {
            whole.to_string()
        };
    }
    let fraction = remainder * BINARY_TO_DECIMAL;
    debug_assert!(fraction < DECIMAL_SCALE);
    let mut fraction = format!("{fraction:016}");
    while fraction.ends_with('0') {
        fraction.pop();
    }
    if negative {
        format!("-{whole}.{fraction}")
    } else {
        format!("{whole}.{fraction}")
    }
}

fn serialize_objects(
    objects: &[Vec<u8>],
    max_output_bytes: u64,
) -> Result<Vec<u8>, StagingSafeVectorPdfError> {
    fn append(
        output: &mut Vec<u8>,
        bytes: &[u8],
        maximum: u64,
    ) -> Result<(), StagingSafeVectorPdfError> {
        let next = output
            .len()
            .checked_add(bytes.len())
            .ok_or(StagingSafeVectorPdfError::OutputLimit)?;
        if next as u64 > maximum {
            return Err(StagingSafeVectorPdfError::OutputLimit);
        }
        output
            .try_reserve(bytes.len())
            .map_err(|_| StagingSafeVectorPdfError::AllocationFailure)?;
        output.extend_from_slice(bytes);
        Ok(())
    }
    let mut output = Vec::new();
    append(
        &mut output,
        b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n",
        max_output_bytes,
    )?;
    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(objects.len())
        .map_err(|_| StagingSafeVectorPdfError::AllocationFailure)?;
    for (index, object) in objects.iter().enumerate() {
        offsets.push(output.len());
        append(
            &mut output,
            format!("{} 0 obj\n", index + 1).as_bytes(),
            max_output_bytes,
        )?;
        append(&mut output, object, max_output_bytes)?;
        append(&mut output, b"\nendobj\n", max_output_bytes)?;
    }
    let xref = output.len();
    append(
        &mut output,
        format!("xref\n0 {}\n", objects.len() + 1).as_bytes(),
        max_output_bytes,
    )?;
    append(&mut output, b"0000000000 65535 f \n", max_output_bytes)?;
    for offset in offsets {
        append(
            &mut output,
            format!("{offset:010} 00000 n \n").as_bytes(),
            max_output_bytes,
        )?;
    }
    append(
        &mut output,
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
        max_output_bytes,
    )?;
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn encode_receipt(
    display: [u8; 32],
    plans: [u8; 32],
    limits: [u8; 32],
    forms: &[StagingSafeVectorPdfFormObject],
    usages: &[StagingSafeVectorPdfUsage],
    pdf_sha256: [u8; 32],
    object_count: u32,
    byte_length: u64,
) -> String {
    let mut output = String::from("{\"algorithm\":");
    push_jcs_string(&mut output, STAGING_SAFE_VECTOR_PDF_ALGORITHM);
    output.push_str(",\"byte_length\":");
    output.push_str(&byte_length.to_string());
    output.push_str(",\"display_fingerprint\":");
    push_hash(&mut output, display);
    output.push_str(",\"form_plans_fingerprint\":");
    push_hash(&mut output, plans);
    output.push_str(",\"forms\":[");
    for (index, form) in forms.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"admitted_sha256\":");
        push_hash(&mut output, form.admitted_sha256);
        output.push_str(",\"content_stream_fingerprint\":");
        push_hash(&mut output, form.content_stream_fingerprint);
        output.push_str(",\"form_plan_fingerprint\":");
        push_hash(&mut output, form.form_plan_fingerprint);
        output.push_str(",\"image_id\":");
        output.push_str(&form.image_id.get().to_string());
        output.push_str(",\"ir_fingerprint\":");
        push_hash(&mut output, form.ir_fingerprint);
        output.push_str(",\"object_number\":");
        output.push_str(&form.object_number.to_string());
        output.push_str(",\"resource_name\":");
        push_jcs_string(&mut output, &form.resource_name);
        output.push('}');
    }
    output.push_str("],\"limits_fingerprint\":");
    push_hash(&mut output, limits);
    output.push_str(",\"object_count\":");
    output.push_str(&object_count.to_string());
    output.push_str(",\"pdf_sha256\":");
    push_hash(&mut output, pdf_sha256);
    output.push_str(",\"usages\":[");
    for (index, usage) in usages.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"content_object_number\":");
        output.push_str(&usage.content_object_number.to_string());
        output.push_str(",\"display_command_fingerprint\":");
        push_hash(&mut output, usage.display_command_fingerprint);
        output.push_str(",\"form_object_number\":");
        output.push_str(&usage.form_object_number.to_string());
        output.push_str(",\"occurrence\":");
        output.push_str(&usage.occurrence.to_string());
        output.push_str(",\"page_index\":");
        output.push_str(&usage.page_index.to_string());
        output.push_str(",\"page_object_number\":");
        output.push_str(&usage.page_object_number.to_string());
        output.push('}');
    }
    output.push_str("]}");
    output
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

    #[test]
    fn vector_pdf_is_deterministic_and_closes_forms_and_usages() {
        let fixture = typaxis_resources::staging_safe_vector_resource_fixture().unwrap();
        let first = write_staging_safe_vector_pdf(
            &fixture.display.display,
            &fixture.plans,
            &fixture.display.layout.limits,
        )
        .unwrap();
        let second = write_staging_safe_vector_pdf(
            &fixture.display.display,
            &fixture.plans,
            &fixture.display.layout.limits,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.forms().len(), 1);
        assert_eq!(first.usages().len(), 1);
        assert!(first.bytes().starts_with(b"%PDF-1.7"));
        assert!(first
            .bytes()
            .windows(b"/MediaBox [0 0 1000 800]".len())
            .any(|window| window == b"/MediaBox [0 0 1000 800]"));
        assert!(first
            .bytes()
            .windows(b"1 0 0 1 100 100 cm".len())
            .any(|window| window == b"1 0 0 1 100 100 cm"));
        assert!(first
            .bytes()
            .windows(b"/Subtype /Form".len())
            .any(|window| window == b"/Subtype /Form"));
        assert!(!first
            .bytes()
            .windows(b"/Subtype /Image".len())
            .any(|window| window == b"/Subtype /Image"));
        first
            .verify(
                &fixture.display.display,
                &fixture.plans,
                &fixture.display.layout.limits,
            )
            .unwrap();
    }

    #[test]
    fn vector_pdf_limits_are_inclusive() {
        assert_eq!(preflight_object_count(1, 1, 5), Ok(5));
        assert_eq!(
            preflight_object_count(1, 1, 4),
            Err(StagingSafeVectorPdfError::ObjectLimit)
        );
        let objects = vec![b"<<>>".to_vec()];
        let bytes = serialize_objects(&objects, u64::MAX).unwrap();
        assert_eq!(
            serialize_objects(&objects, bytes.len() as u64).unwrap(),
            bytes
        );
        assert_eq!(
            serialize_objects(&objects, bytes.len() as u64 - 1),
            Err(StagingSafeVectorPdfError::OutputLimit)
        );

        let source_transform = [20_000, 20_000, 0, 0];
        let combined = compose_fixed_transform(source_transform, source_transform).unwrap();
        assert_eq!(combined, [6_104, 6_104, 0, 0]);
        let point = RawPoint { x: 5, y: 5 };
        let composed_once = apply_fixed_transform(point, combined).unwrap();
        let rounded_twice = apply_fixed_transform(
            apply_fixed_transform(point, source_transform).unwrap(),
            source_transform,
        )
        .unwrap();
        assert_eq!((composed_once.x, composed_once.y), (0, 0));
        assert_eq!((rounded_twice.x, rounded_twice.y), (1, 1));
    }

    #[test]
    fn vector_pdf_tamper_is_rejected() {
        let fixture = typaxis_resources::staging_safe_vector_resource_fixture().unwrap();
        let mut pdf = write_staging_safe_vector_pdf(
            &fixture.display.display,
            &fixture.plans,
            &fixture.display.layout.limits,
        )
        .unwrap();
        pdf.forms[0].object_number += 1;
        assert_eq!(
            pdf.verify(
                &fixture.display.display,
                &fixture.plans,
                &fixture.display.layout.limits,
            ),
            Err(StagingSafeVectorPdfError::ReceiptMismatch)
        );
    }
}
