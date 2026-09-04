use core::num::NonZeroU32;
use std::io::Cursor;
use std::sync::Arc;

use jpeg_decoder::{CodingProcess, ColorTransform, Decoder, PixelFormat};
use typaxis_core::{sha256, ImageResourceId, M4EffectiveResourceLimits};

use super::ResourceAdmissionError;

pub const JPEG_RESOURCE_PROFILE_ID: &str = "typaxis.resource-profile/jpeg-baseline/1";
pub const JPEG_MARKER_PREFLIGHT_ID: &str = "typaxis.jpeg-marker-preflight/1";
pub const JPEG_SANITIZER_ID: &str = "typaxis.jpeg-segment-sanitizer/1";
pub const JPEG_PIXEL_OBSERVATION_ID: &str = "typaxis.jpeg-pixel-observation/1";
pub const JPEG_DECODER_ID: &str = "jpeg-decoder/0.3.2+platform_independent";

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum JpegColorKind {
    Grayscale,
    YCbCr,
}

impl JpegColorKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Grayscale => "grayscale",
            Self::YCbCr => "ycbcr",
        }
    }

    pub const fn channels(self) -> u8 {
        match self {
            Self::Grayscale => 1,
            Self::YCbCr => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum JpegSampling {
    Gray,
    YCbCr444,
    YCbCr422,
    YCbCr440,
    YCbCr420,
}

impl JpegSampling {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Gray => "gray-1x1",
            Self::YCbCr444 => "ycbcr-4:4:4",
            Self::YCbCr422 => "ycbcr-4:2:2",
            Self::YCbCr440 => "ycbcr-4:4:0",
            Self::YCbCr420 => "ycbcr-4:2:0",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum JpegFailureReason {
    Malformed,
    UnsupportedProcess,
    ForbiddenMetadata,
    InvalidTables,
    InvalidEntropy,
    DecodeMismatch,
    SanitizerMismatch,
    PixelLimit,
    DecodeLimit,
    SpoolLimit,
}

impl JpegFailureReason {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Malformed => "malformed",
            Self::UnsupportedProcess => "unsupported_process",
            Self::ForbiddenMetadata => "forbidden_metadata",
            Self::InvalidTables => "invalid_tables",
            Self::InvalidEntropy => "invalid_entropy",
            Self::DecodeMismatch => "decode_mismatch",
            Self::SanitizerMismatch => "sanitizer_mismatch",
            Self::PixelLimit => "pixel_limit",
            Self::DecodeLimit => "decode_limit",
            Self::SpoolLimit => "spool_limit",
        }
    }
}

/// Immutable proof that the exact stable JPEG bytes passed the closed marker,
/// entropy, decoder, and sanitizer policies. Construction is owned by this
/// module; consumers can only inspect the resulting facts and normalized
/// stream.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JpegAdmissionAttestation {
    pub(super) image_id: ImageResourceId,
    pub(super) source_sha256: [u8; 32],
    pub(super) width: NonZeroU32,
    pub(super) height: NonZeroU32,
    pub(super) color_kind: JpegColorKind,
    pub(super) sampling: JpegSampling,
    pub(super) decoded_byte_length: u64,
    pub(super) peak_workspace_bytes: u64,
    pub(super) pixel_sha256: [u8; 32],
    pub(super) normalized_bytes: Arc<[u8]>,
    pub(super) normalized_sha256: [u8; 32],
    pub(super) limits_fingerprint: [u8; 32],
    pub(super) profile_fingerprint: [u8; 32],
}

impl JpegAdmissionAttestation {
    pub const fn image_id(&self) -> ImageResourceId {
        self.image_id
    }
    pub const fn source_sha256(&self) -> [u8; 32] {
        self.source_sha256
    }
    pub const fn width(&self) -> NonZeroU32 {
        self.width
    }
    pub const fn height(&self) -> NonZeroU32 {
        self.height
    }
    pub const fn color_kind(&self) -> JpegColorKind {
        self.color_kind
    }
    pub const fn sampling(&self) -> JpegSampling {
        self.sampling
    }
    pub const fn decoded_byte_length(&self) -> u64 {
        self.decoded_byte_length
    }
    pub const fn peak_workspace_bytes(&self) -> u64 {
        self.peak_workspace_bytes
    }
    pub const fn pixel_sha256(&self) -> [u8; 32] {
        self.pixel_sha256
    }
    pub fn normalized_bytes(&self) -> &[u8] {
        &self.normalized_bytes
    }
    pub fn normalized_stream(&self) -> Arc<[u8]> {
        Arc::clone(&self.normalized_bytes)
    }
    pub const fn normalized_sha256(&self) -> [u8; 32] {
        self.normalized_sha256
    }
    pub const fn limits_fingerprint(&self) -> [u8; 32] {
        self.limits_fingerprint
    }
    pub const fn profile_fingerprint(&self) -> [u8; 32] {
        self.profile_fingerprint
    }
    pub const fn marker_preflight_id(&self) -> &'static str {
        JPEG_MARKER_PREFLIGHT_ID
    }
    pub const fn sanitizer_id(&self) -> &'static str {
        JPEG_SANITIZER_ID
    }
    pub const fn pixel_observation_id(&self) -> &'static str {
        JPEG_PIXEL_OBSERVATION_ID
    }
    pub const fn decoder_id(&self) -> &'static str {
        JPEG_DECODER_ID
    }
    pub const fn resource_profile_id(&self) -> &'static str {
        JPEG_RESOURCE_PROFILE_ID
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FrameComponent {
    id: u8,
    horizontal: u8,
    vertical: u8,
    quantization_table: u8,
    dc_table: u8,
    ac_table: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HuffmanTable {
    first_code: [u16; 17],
    max_code: [Option<u16>; 17],
    first_symbol: [u16; 17],
    symbols: Vec<u8>,
}

impl HuffmanTable {
    fn parse(class: u8, counts: &[u8], symbols: &[u8]) -> Result<Self, JpegFailureReason> {
        if counts.len() != 16 || symbols.is_empty() {
            return Err(JpegFailureReason::InvalidTables);
        }
        let symbol_count = counts
            .iter()
            .try_fold(0usize, |total, count| {
                total.checked_add(usize::from(*count))
            })
            .ok_or(JpegFailureReason::InvalidTables)?;
        if symbol_count != symbols.len() || symbol_count > 256 {
            return Err(JpegFailureReason::InvalidTables);
        }
        let mut seen = [false; 256];
        for symbol in symbols {
            if seen[usize::from(*symbol)] {
                return Err(JpegFailureReason::InvalidTables);
            }
            seen[usize::from(*symbol)] = true;
            match class {
                0 if *symbol <= 11 => {}
                1 if *symbol == 0x00 || *symbol == 0xf0 => {}
                1 if (*symbol >> 4) <= 15 && (1..=10).contains(&(*symbol & 0x0f)) => {}
                _ => return Err(JpegFailureReason::InvalidTables),
            }
        }

        let mut first_code = [0u16; 17];
        let mut max_code = [None; 17];
        let mut first_symbol = [0u16; 17];
        let mut code = 0u32;
        let mut symbol_index = 0u32;
        let mut slots = 1i32;
        for length in 1..=16usize {
            slots = slots
                .checked_mul(2)
                .and_then(|value| value.checked_sub(i32::from(counts[length - 1])))
                .ok_or(JpegFailureReason::InvalidTables)?;
            if slots < 0 {
                return Err(JpegFailureReason::InvalidTables);
            }
            first_code[length] =
                u16::try_from(code).map_err(|_| JpegFailureReason::InvalidTables)?;
            first_symbol[length] =
                u16::try_from(symbol_index).map_err(|_| JpegFailureReason::InvalidTables)?;
            let count = u32::from(counts[length - 1]);
            if count != 0 {
                let last = code
                    .checked_add(count - 1)
                    .ok_or(JpegFailureReason::InvalidTables)?;
                let all_ones = (1u32 << length) - 1;
                if last >= all_ones {
                    // Annex C reserves the all-ones code and an oversubscribed
                    // tree was already rejected by `slots` above.
                    return Err(JpegFailureReason::InvalidTables);
                }
                max_code[length] =
                    Some(u16::try_from(last).map_err(|_| JpegFailureReason::InvalidTables)?);
            }
            symbol_index = symbol_index
                .checked_add(count)
                .ok_or(JpegFailureReason::InvalidTables)?;
            code = code
                .checked_add(count)
                .and_then(|value| value.checked_mul(2))
                .ok_or(JpegFailureReason::InvalidTables)?;
        }
        Ok(Self {
            first_code,
            max_code,
            first_symbol,
            symbols: symbols.to_vec(),
        })
    }

    fn decode(&self, reader: &mut EntropyReader<'_>) -> Result<u8, JpegFailureReason> {
        let mut code = 0u16;
        for length in 1..=16usize {
            let bit = reader.read_bit()?;
            code = code
                .checked_mul(2)
                .and_then(|value| value.checked_add(u16::from(bit)))
                .ok_or(JpegFailureReason::InvalidEntropy)?;
            let Some(max_code) = self.max_code[length] else {
                continue;
            };
            if code <= max_code {
                let relative = code
                    .checked_sub(self.first_code[length])
                    .ok_or(JpegFailureReason::InvalidEntropy)?;
                let index = self.first_symbol[length]
                    .checked_add(relative)
                    .ok_or(JpegFailureReason::InvalidEntropy)?;
                return self
                    .symbols
                    .get(usize::from(index))
                    .copied()
                    .ok_or(JpegFailureReason::InvalidEntropy);
            }
        }
        Err(JpegFailureReason::InvalidEntropy)
    }
}

#[derive(Clone, Debug)]
struct JpegPreflight {
    width: NonZeroU32,
    height: NonZeroU32,
    color_kind: JpegColorKind,
    sampling: JpegSampling,
    decoded_byte_length: u64,
    peak_workspace_bytes: u64,
    app0_end: usize,
    entropy_start: usize,
    components: Vec<FrameComponent>,
    dc_tables: [Option<HuffmanTable>; 4],
    ac_tables: [Option<HuffmanTable>; 4],
    restart_interval: Option<u16>,
    total_mcus: u64,
}

struct EntropyReader<'a> {
    bytes: &'a [u8],
    position: usize,
    current: u8,
    bits_remaining: u8,
}

impl<'a> EntropyReader<'a> {
    fn new(bytes: &'a [u8], position: usize) -> Self {
        Self {
            bytes,
            position,
            current: 0,
            bits_remaining: 0,
        }
    }

    fn read_bit(&mut self) -> Result<u8, JpegFailureReason> {
        if self.bits_remaining == 0 {
            self.current = self.read_entropy_byte()?;
            self.bits_remaining = 8;
        }
        self.bits_remaining -= 1;
        Ok((self.current >> self.bits_remaining) & 1)
    }

    fn read_bits(&mut self, count: u8) -> Result<u16, JpegFailureReason> {
        if count > 16 {
            return Err(JpegFailureReason::InvalidEntropy);
        }
        let mut value = 0u16;
        for _ in 0..count {
            let bit = self.read_bit()?;
            value = value
                .checked_mul(2)
                .and_then(|current| current.checked_add(u16::from(bit)))
                .ok_or(JpegFailureReason::InvalidEntropy)?;
        }
        Ok(value)
    }

    fn read_entropy_byte(&mut self) -> Result<u8, JpegFailureReason> {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
        self.position = self
            .position
            .checked_add(1)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
        if byte != 0xff {
            return Ok(byte);
        }
        let stuffed = *self
            .bytes
            .get(self.position)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
        if stuffed != 0x00 {
            return Err(JpegFailureReason::InvalidEntropy);
        }
        self.position += 1;
        Ok(0xff)
    }

    fn align_with_ones(&mut self) -> Result<(), JpegFailureReason> {
        if self.bits_remaining != 0 {
            let mask = (1u16 << self.bits_remaining) - 1;
            if u16::from(self.current) & mask != mask {
                return Err(JpegFailureReason::InvalidEntropy);
            }
            self.bits_remaining = 0;
        }
        Ok(())
    }

    fn expect_marker(&mut self, marker: u8) -> Result<(), JpegFailureReason> {
        self.align_with_ones()?;
        if self.bytes.get(self.position) != Some(&0xff)
            || self.bytes.get(self.position + 1) != Some(&marker)
        {
            return Err(JpegFailureReason::InvalidEntropy);
        }
        self.position = self
            .position
            .checked_add(2)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
        Ok(())
    }
}

pub(super) fn admit_jpeg(
    image_id: ImageResourceId,
    source_sha256: [u8; 32],
    bytes: &[u8],
    limits: &M4EffectiveResourceLimits,
    profile_fingerprint: [u8; 32],
) -> Result<JpegAdmissionAttestation, ResourceAdmissionError> {
    let preflight = preflight(bytes, limits).map_err(|reason| match reason {
        JpegFailureReason::PixelLimit | JpegFailureReason::SpoolLimit => {
            ResourceAdmissionError::ResourceLimit
        }
        JpegFailureReason::DecodeLimit => ResourceAdmissionError::DecodedImageLimit,
        other => ResourceAdmissionError::InvalidJpeg(other),
    })?;
    validate_entropy(bytes, &preflight).map_err(ResourceAdmissionError::InvalidJpeg)?;

    let decoder_limit =
        usize::try_from(limits.base().get().max_decoded_image_bytes).unwrap_or(usize::MAX);
    let mut decoder = Decoder::new(Cursor::new(bytes));
    decoder.set_max_decoding_buffer_size(decoder_limit);
    decoder.set_color_transform(match preflight.color_kind {
        JpegColorKind::Grayscale => ColorTransform::Grayscale,
        JpegColorKind::YCbCr => ColorTransform::YCbCr,
    });
    let pixels = decoder
        .decode()
        .map_err(|_| ResourceAdmissionError::InvalidJpeg(JpegFailureReason::DecodeMismatch))?;
    let info = decoder.info().ok_or(ResourceAdmissionError::InvalidJpeg(
        JpegFailureReason::DecodeMismatch,
    ))?;
    let expected_format = match preflight.color_kind {
        JpegColorKind::Grayscale => PixelFormat::L8,
        JpegColorKind::YCbCr => PixelFormat::RGB24,
    };
    if u32::from(info.width) != preflight.width.get()
        || u32::from(info.height) != preflight.height.get()
        || info.pixel_format != expected_format
        || info.coding_process != CodingProcess::DctSequential
        || u64::try_from(pixels.len()) != Ok(preflight.decoded_byte_length)
    {
        return Err(ResourceAdmissionError::InvalidJpeg(
            JpegFailureReason::DecodeMismatch,
        ));
    }
    let pixel_sha256 = sha256(&pixels);
    drop(pixels);

    let normalized_length = bytes
        .len()
        .checked_sub(preflight.app0_end.saturating_sub(2))
        .ok_or(ResourceAdmissionError::InvalidJpeg(
            JpegFailureReason::SanitizerMismatch,
        ))?;
    let mut normalized = Vec::new();
    normalized
        .try_reserve_exact(normalized_length)
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    normalized.extend_from_slice(&bytes[..2]);
    normalized.extend_from_slice(bytes.get(preflight.app0_end..).ok_or(
        ResourceAdmissionError::InvalidJpeg(JpegFailureReason::SanitizerMismatch),
    )?);
    if normalized.len() != normalized_length
        || normalized.get(..2) != Some(&[0xff, 0xd8])
        || normalized.get(normalized.len().saturating_sub(2)..) != Some(&[0xff, 0xd9])
    {
        return Err(ResourceAdmissionError::InvalidJpeg(
            JpegFailureReason::SanitizerMismatch,
        ));
    }
    let normalized_sha256 = sha256(&normalized);

    Ok(JpegAdmissionAttestation {
        image_id,
        source_sha256,
        width: preflight.width,
        height: preflight.height,
        color_kind: preflight.color_kind,
        sampling: preflight.sampling,
        decoded_byte_length: preflight.decoded_byte_length,
        peak_workspace_bytes: preflight.peak_workspace_bytes,
        pixel_sha256,
        normalized_bytes: Arc::from(normalized),
        normalized_sha256,
        limits_fingerprint: limits.fingerprint(),
        profile_fingerprint,
    })
}

fn preflight(
    bytes: &[u8],
    limits: &M4EffectiveResourceLimits,
) -> Result<JpegPreflight, JpegFailureReason> {
    if bytes.get(..2) != Some(&[0xff, 0xd8]) {
        return Err(JpegFailureReason::Malformed);
    }
    let (app0_marker, app0_payload, app0_end) = read_segment(bytes, 2)?;
    if app0_marker != 0xe0 || app0_payload.len() != 14 {
        return Err(JpegFailureReason::ForbiddenMetadata);
    }
    if app0_payload.get(..5) != Some(b"JFIF\0")
        || app0_payload.get(5) != Some(&1)
        || !matches!(app0_payload.get(6), Some(0..=2))
        || !matches!(app0_payload.get(7), Some(0..=2))
        || app0_payload.get(12) != Some(&0)
        || app0_payload.get(13) != Some(&0)
    {
        return Err(JpegFailureReason::ForbiddenMetadata);
    }
    let x_density = read_u16(app0_payload, 8)?;
    let y_density = read_u16(app0_payload, 10)?;
    if x_density == 0 || x_density != y_density {
        return Err(JpegFailureReason::ForbiddenMetadata);
    }

    let mut offset = app0_end;
    let mut width = None;
    let mut height = None;
    let mut components: Option<Vec<FrameComponent>> = None;
    let mut quantization_tables = [false; 4];
    let mut dc_tables: [Option<HuffmanTable>; 4] = std::array::from_fn(|_| None);
    let mut ac_tables: [Option<HuffmanTable>; 4] = std::array::from_fn(|_| None);
    let mut restart_interval = None;
    let entropy_start;

    loop {
        let (marker, payload, end) = read_segment(bytes, offset)?;
        match marker {
            0xdb => parse_dqt(payload, &mut quantization_tables)?,
            0xc4 => parse_dht(payload, &mut dc_tables, &mut ac_tables)?,
            0xc0 => {
                if components.is_some() {
                    return Err(JpegFailureReason::UnsupportedProcess);
                }
                let (parsed_width, parsed_height, parsed_components) = parse_sof0(payload)?;
                width = Some(parsed_width);
                height = Some(parsed_height);
                components = Some(parsed_components);
            }
            0xdd => {
                if restart_interval.is_some() || payload.len() != 2 {
                    return Err(JpegFailureReason::InvalidTables);
                }
                let interval = read_u16(payload, 0)?;
                if interval == 0 {
                    return Err(JpegFailureReason::InvalidTables);
                }
                restart_interval = Some(interval);
            }
            0xda => {
                let component_set = components
                    .as_mut()
                    .ok_or(JpegFailureReason::UnsupportedProcess)?;
                parse_sos(payload, component_set)?;
                entropy_start = end;
                break;
            }
            0xe0..=0xef | 0xfe => return Err(JpegFailureReason::ForbiddenMetadata),
            _ => return Err(JpegFailureReason::UnsupportedProcess),
        }
        offset = end;
    }

    let width = width.ok_or(JpegFailureReason::UnsupportedProcess)?;
    let height = height.ok_or(JpegFailureReason::UnsupportedProcess)?;
    let components = components.ok_or(JpegFailureReason::UnsupportedProcess)?;
    let (color_kind, sampling) = classify_components(&components)?;
    validate_table_closure(&components, &quantization_tables, &dc_tables, &ac_tables)?;

    let pixels = u64::from(width.get())
        .checked_mul(u64::from(height.get()))
        .ok_or(JpegFailureReason::Malformed)?;
    if pixels > limits.base().get().max_image_pixels {
        return Err(JpegFailureReason::PixelLimit);
    }
    let decoded_byte_length = pixels
        .checked_mul(u64::from(color_kind.channels()))
        .ok_or(JpegFailureReason::Malformed)?;
    let (total_mcus, peak_workspace_bytes) =
        workspace_facts(width, height, &components, decoded_byte_length)?;
    if decoded_byte_length > limits.base().get().max_decoded_image_bytes
        || peak_workspace_bytes > limits.base().get().max_decoded_image_bytes
    {
        return Err(JpegFailureReason::DecodeLimit);
    }
    let normalized_length = bytes
        .len()
        .checked_sub(app0_end.saturating_sub(2))
        .ok_or(JpegFailureReason::Malformed)?;
    let simultaneous_spool = u64::try_from(bytes.len())
        .ok()
        .and_then(|source| source.checked_add(u64::try_from(normalized_length).ok()?))
        .ok_or(JpegFailureReason::Malformed)?;
    if simultaneous_spool > limits.base().get().max_spool_bytes {
        return Err(JpegFailureReason::SpoolLimit);
    }

    Ok(JpegPreflight {
        width,
        height,
        color_kind,
        sampling,
        decoded_byte_length,
        peak_workspace_bytes,
        app0_end,
        entropy_start,
        components,
        dc_tables,
        ac_tables,
        restart_interval,
        total_mcus,
    })
}

fn read_segment(bytes: &[u8], offset: usize) -> Result<(u8, &[u8], usize), JpegFailureReason> {
    if bytes.get(offset) != Some(&0xff) {
        return Err(JpegFailureReason::Malformed);
    }
    let marker = *bytes
        .get(offset.checked_add(1).ok_or(JpegFailureReason::Malformed)?)
        .ok_or(JpegFailureReason::Malformed)?;
    if marker == 0x00
        || marker == 0xff
        || marker == 0xd8
        || marker == 0xd9
        || (0xd0..=0xd7).contains(&marker)
    {
        return Err(JpegFailureReason::Malformed);
    }
    let length_offset = offset.checked_add(2).ok_or(JpegFailureReason::Malformed)?;
    let length = usize::from(read_u16(bytes, length_offset)?);
    if length < 2 {
        return Err(JpegFailureReason::Malformed);
    }
    let payload_start = length_offset
        .checked_add(2)
        .ok_or(JpegFailureReason::Malformed)?;
    let end = length_offset
        .checked_add(length)
        .ok_or(JpegFailureReason::Malformed)?;
    let payload = bytes
        .get(payload_start..end)
        .ok_or(JpegFailureReason::Malformed)?;
    Ok((marker, payload, end))
}

fn parse_dqt(payload: &[u8], tables: &mut [bool; 4]) -> Result<(), JpegFailureReason> {
    if payload.is_empty() {
        return Err(JpegFailureReason::InvalidTables);
    }
    let mut offset = 0usize;
    while offset < payload.len() {
        let selector = *payload
            .get(offset)
            .ok_or(JpegFailureReason::InvalidTables)?;
        offset += 1;
        if selector >> 4 != 0 || selector & 0x0f > 3 {
            return Err(JpegFailureReason::InvalidTables);
        }
        let table = usize::from(selector & 0x0f);
        if tables[table] {
            return Err(JpegFailureReason::InvalidTables);
        }
        let values = payload
            .get(
                offset
                    ..offset
                        .checked_add(64)
                        .ok_or(JpegFailureReason::InvalidTables)?,
            )
            .ok_or(JpegFailureReason::InvalidTables)?;
        if values.contains(&0) {
            return Err(JpegFailureReason::InvalidTables);
        }
        tables[table] = true;
        offset += 64;
    }
    if offset != payload.len() {
        return Err(JpegFailureReason::InvalidTables);
    }
    Ok(())
}

fn parse_dht(
    payload: &[u8],
    dc_tables: &mut [Option<HuffmanTable>; 4],
    ac_tables: &mut [Option<HuffmanTable>; 4],
) -> Result<(), JpegFailureReason> {
    if payload.is_empty() {
        return Err(JpegFailureReason::InvalidTables);
    }
    let mut offset = 0usize;
    while offset < payload.len() {
        let selector = *payload
            .get(offset)
            .ok_or(JpegFailureReason::InvalidTables)?;
        offset += 1;
        let class = selector >> 4;
        let id = selector & 0x0f;
        if class > 1 || id > 3 {
            return Err(JpegFailureReason::InvalidTables);
        }
        let count_end = offset
            .checked_add(16)
            .ok_or(JpegFailureReason::InvalidTables)?;
        let counts = payload
            .get(offset..count_end)
            .ok_or(JpegFailureReason::InvalidTables)?;
        offset = count_end;
        let symbol_count = counts
            .iter()
            .try_fold(0usize, |total, count| {
                total.checked_add(usize::from(*count))
            })
            .ok_or(JpegFailureReason::InvalidTables)?;
        let symbol_end = offset
            .checked_add(symbol_count)
            .ok_or(JpegFailureReason::InvalidTables)?;
        let symbols = payload
            .get(offset..symbol_end)
            .ok_or(JpegFailureReason::InvalidTables)?;
        offset = symbol_end;
        let table = HuffmanTable::parse(class, counts, symbols)?;
        let slot = if class == 0 {
            &mut dc_tables[usize::from(id)]
        } else {
            &mut ac_tables[usize::from(id)]
        };
        if slot.replace(table).is_some() {
            return Err(JpegFailureReason::InvalidTables);
        }
    }
    if offset != payload.len() {
        return Err(JpegFailureReason::InvalidTables);
    }
    Ok(())
}

fn parse_sof0(
    payload: &[u8],
) -> Result<(NonZeroU32, NonZeroU32, Vec<FrameComponent>), JpegFailureReason> {
    if payload.len() < 6 || payload[0] != 8 {
        return Err(JpegFailureReason::UnsupportedProcess);
    }
    let height = NonZeroU32::new(u32::from(read_u16(payload, 1)?))
        .ok_or(JpegFailureReason::UnsupportedProcess)?;
    let width = NonZeroU32::new(u32::from(read_u16(payload, 3)?))
        .ok_or(JpegFailureReason::UnsupportedProcess)?;
    let count = usize::from(payload[5]);
    if !matches!(count, 1 | 3)
        || payload.len()
            != 6usize
                .checked_add(count * 3)
                .ok_or(JpegFailureReason::Malformed)?
    {
        return Err(JpegFailureReason::UnsupportedProcess);
    }
    let mut components = Vec::new();
    components
        .try_reserve_exact(count)
        .map_err(|_| JpegFailureReason::Malformed)?;
    for index in 0..count {
        let base = 6 + index * 3;
        let id = payload[base];
        let sampling = payload[base + 1];
        let horizontal = sampling >> 4;
        let vertical = sampling & 0x0f;
        let quantization_table = payload[base + 2];
        if horizontal == 0
            || vertical == 0
            || quantization_table > 3
            || components
                .iter()
                .any(|component: &FrameComponent| component.id == id)
        {
            return Err(JpegFailureReason::UnsupportedProcess);
        }
        components.push(FrameComponent {
            id,
            horizontal,
            vertical,
            quantization_table,
            dc_table: u8::MAX,
            ac_table: u8::MAX,
        });
    }
    Ok((width, height, components))
}

fn parse_sos(payload: &[u8], components: &mut [FrameComponent]) -> Result<(), JpegFailureReason> {
    let count = usize::from(*payload.first().ok_or(JpegFailureReason::Malformed)?);
    let expected_length = 1usize
        .checked_add(count.checked_mul(2).ok_or(JpegFailureReason::Malformed)?)
        .and_then(|value| value.checked_add(3))
        .ok_or(JpegFailureReason::Malformed)?;
    if count != components.len() || payload.len() != expected_length {
        return Err(JpegFailureReason::UnsupportedProcess);
    }
    for (index, component) in components.iter_mut().enumerate() {
        let base = 1 + index * 2;
        if payload[base] != component.id {
            return Err(JpegFailureReason::UnsupportedProcess);
        }
        let selector = payload[base + 1];
        component.dc_table = selector >> 4;
        component.ac_table = selector & 0x0f;
        if component.dc_table > 3 || component.ac_table > 3 {
            return Err(JpegFailureReason::InvalidTables);
        }
    }
    if payload[payload.len() - 3..] != [0, 63, 0] {
        return Err(JpegFailureReason::UnsupportedProcess);
    }
    Ok(())
}

fn classify_components(
    components: &[FrameComponent],
) -> Result<(JpegColorKind, JpegSampling), JpegFailureReason> {
    match components {
        [gray] if gray.id == 1 && gray.horizontal == 1 && gray.vertical == 1 => {
            Ok((JpegColorKind::Grayscale, JpegSampling::Gray))
        }
        [y, cb, cr]
            if [y.id, cb.id, cr.id] == [1, 2, 3]
                && cb.horizontal == 1
                && cb.vertical == 1
                && cr.horizontal == 1
                && cr.vertical == 1 =>
        {
            let sampling = match (y.horizontal, y.vertical) {
                (1, 1) => JpegSampling::YCbCr444,
                (2, 1) => JpegSampling::YCbCr422,
                (1, 2) => JpegSampling::YCbCr440,
                (2, 2) => JpegSampling::YCbCr420,
                _ => return Err(JpegFailureReason::UnsupportedProcess),
            };
            Ok((JpegColorKind::YCbCr, sampling))
        }
        _ => Err(JpegFailureReason::UnsupportedProcess),
    }
}

fn validate_table_closure(
    components: &[FrameComponent],
    quantization_tables: &[bool; 4],
    dc_tables: &[Option<HuffmanTable>; 4],
    ac_tables: &[Option<HuffmanTable>; 4],
) -> Result<(), JpegFailureReason> {
    let mut used_quantization = [false; 4];
    let mut used_dc = [false; 4];
    let mut used_ac = [false; 4];
    for component in components {
        used_quantization[usize::from(component.quantization_table)] = true;
        used_dc[usize::from(component.dc_table)] = true;
        used_ac[usize::from(component.ac_table)] = true;
    }
    if *quantization_tables != used_quantization
        || dc_tables
            .iter()
            .map(Option::is_some)
            .ne(used_dc.iter().copied())
        || ac_tables
            .iter()
            .map(Option::is_some)
            .ne(used_ac.iter().copied())
    {
        return Err(JpegFailureReason::InvalidTables);
    }
    Ok(())
}

fn workspace_facts(
    width: NonZeroU32,
    height: NonZeroU32,
    components: &[FrameComponent],
    decoded_byte_length: u64,
) -> Result<(u64, u64), JpegFailureReason> {
    let max_horizontal = u64::from(
        components
            .iter()
            .map(|component| component.horizontal)
            .max()
            .ok_or(JpegFailureReason::Malformed)?,
    );
    let max_vertical = u64::from(
        components
            .iter()
            .map(|component| component.vertical)
            .max()
            .ok_or(JpegFailureReason::Malformed)?,
    );
    let mcu_width = max_horizontal
        .checked_mul(8)
        .ok_or(JpegFailureReason::Malformed)?;
    let mcu_height = max_vertical
        .checked_mul(8)
        .ok_or(JpegFailureReason::Malformed)?;
    let mcu_columns = ceil_div(u64::from(width.get()), mcu_width)?;
    let mcu_rows = ceil_div(u64::from(height.get()), mcu_height)?;
    let total_mcus = mcu_columns
        .checked_mul(mcu_rows)
        .ok_or(JpegFailureReason::Malformed)?;

    let mut component_planes = 0u64;
    let mut coefficient_rows = 0u64;
    for component in components {
        let horizontal = u64::from(component.horizontal);
        let vertical = u64::from(component.vertical);
        let padded_width = mcu_columns
            .checked_mul(horizontal)
            .and_then(|value| value.checked_mul(8))
            .ok_or(JpegFailureReason::Malformed)?;
        let padded_height = mcu_rows
            .checked_mul(vertical)
            .and_then(|value| value.checked_mul(8))
            .ok_or(JpegFailureReason::Malformed)?;
        component_planes = component_planes
            .checked_add(
                padded_width
                    .checked_mul(padded_height)
                    .ok_or(JpegFailureReason::Malformed)?,
            )
            .ok_or(JpegFailureReason::Malformed)?;
        let coefficient_row_bytes = mcu_columns
            .checked_mul(horizontal)
            .and_then(|value| value.checked_mul(vertical))
            .and_then(|value| value.checked_mul(64))
            .and_then(|value| value.checked_mul(2))
            .ok_or(JpegFailureReason::Malformed)?;
        // jpeg-decoder's non-rayon worker uses an unbounded per-component
        // channel. If the worker threads are descheduled, every completed MCU
        // row plus the replacement row can be live at once. Charge that
        // deterministic upper bound before constructing the decoder.
        let live_coefficients = coefficient_row_bytes
            .checked_mul(
                mcu_rows
                    .checked_add(1)
                    .ok_or(JpegFailureReason::Malformed)?,
            )
            .ok_or(JpegFailureReason::Malformed)?;
        coefficient_rows = coefficient_rows
            .checked_add(live_coefficients)
            .ok_or(JpegFailureReason::Malformed)?;
    }
    let upsample_rows = u64::from(width.get())
        .checked_mul(u64::try_from(components.len()).map_err(|_| JpegFailureReason::Malformed)?)
        .and_then(|value| value.checked_mul(2))
        .ok_or(JpegFailureReason::Malformed)?;
    let coefficient_peak = component_planes
        .checked_add(coefficient_rows)
        .ok_or(JpegFailureReason::Malformed)?;
    let output_peak = component_planes
        .checked_add(decoded_byte_length)
        .and_then(|value| value.checked_add(upsample_rows))
        .ok_or(JpegFailureReason::Malformed)?;
    Ok((total_mcus, coefficient_peak.max(output_peak)))
}

fn ceil_div(value: u64, divisor: u64) -> Result<u64, JpegFailureReason> {
    value
        .checked_add(divisor.checked_sub(1).ok_or(JpegFailureReason::Malformed)?)
        .map(|adjusted| adjusted / divisor)
        .ok_or(JpegFailureReason::Malformed)
}

fn validate_entropy(bytes: &[u8], preflight: &JpegPreflight) -> Result<(), JpegFailureReason> {
    let mut reader = EntropyReader::new(bytes, preflight.entropy_start);
    let mut predictors = [0i16; 3];
    let interval = preflight.restart_interval.map(u64::from);
    let mut next_restart = 0u8;
    for mcu in 0..preflight.total_mcus {
        for (component_index, component) in preflight.components.iter().enumerate() {
            let blocks = u16::from(component.horizontal)
                .checked_mul(u16::from(component.vertical))
                .ok_or(JpegFailureReason::InvalidEntropy)?;
            for _ in 0..blocks {
                decode_block(
                    &mut reader,
                    &preflight.dc_tables[usize::from(component.dc_table)],
                    &preflight.ac_tables[usize::from(component.ac_table)],
                    &mut predictors[component_index],
                )?;
            }
        }
        let completed = mcu + 1;
        if interval.is_some_and(|value| completed % value == 0) && completed < preflight.total_mcus
        {
            reader.expect_marker(0xd0 + next_restart)?;
            next_restart = (next_restart + 1) & 7;
            predictors.fill(0);
        }
    }
    reader.expect_marker(0xd9)?;
    if reader.position != bytes.len() {
        return Err(JpegFailureReason::InvalidEntropy);
    }
    Ok(())
}

fn decode_block(
    reader: &mut EntropyReader<'_>,
    dc_table: &Option<HuffmanTable>,
    ac_table: &Option<HuffmanTable>,
    predictor: &mut i16,
) -> Result<(), JpegFailureReason> {
    let dc_table = dc_table.as_ref().ok_or(JpegFailureReason::InvalidTables)?;
    let ac_table = ac_table.as_ref().ok_or(JpegFailureReason::InvalidTables)?;
    let category = dc_table.decode(reader)?;
    let difference = receive_extend(reader, category)?;
    let next = i32::from(*predictor)
        .checked_add(i32::from(difference))
        .and_then(|value| i16::try_from(value).ok())
        .ok_or(JpegFailureReason::InvalidEntropy)?;
    *predictor = next;

    let mut coefficient = 1u8;
    while coefficient < 64 {
        let symbol = ac_table.decode(reader)?;
        if symbol == 0x00 {
            break;
        }
        if symbol == 0xf0 {
            coefficient = coefficient
                .checked_add(16)
                .filter(|value| *value <= 64)
                .ok_or(JpegFailureReason::InvalidEntropy)?;
            continue;
        }
        let run = symbol >> 4;
        let size = symbol & 0x0f;
        coefficient = coefficient
            .checked_add(run)
            .filter(|value| *value < 64)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
        let _coefficient_value = receive_extend(reader, size)?;
        coefficient = coefficient
            .checked_add(1)
            .ok_or(JpegFailureReason::InvalidEntropy)?;
    }
    Ok(())
}

fn receive_extend(reader: &mut EntropyReader<'_>, size: u8) -> Result<i16, JpegFailureReason> {
    if size == 0 {
        return Ok(0);
    }
    if size > 11 {
        return Err(JpegFailureReason::InvalidEntropy);
    }
    let encoded = i32::from(reader.read_bits(size)?);
    let threshold = 1i32 << (size - 1);
    let value = if encoded < threshold {
        encoded - (1i32 << size) + 1
    } else {
        encoded
    };
    i16::try_from(value).map_err(|_| JpegFailureReason::InvalidEntropy)
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, JpegFailureReason> {
    let end = offset.checked_add(2).ok_or(JpegFailureReason::Malformed)?;
    let value: [u8; 2] = bytes
        .get(offset..end)
        .ok_or(JpegFailureReason::Malformed)?
        .try_into()
        .map_err(|_| JpegFailureReason::Malformed)?;
    Ok(u16::from_be_bytes(value))
}

#[cfg(test)]
mod tests {
    use typaxis_core::{ResourceLimits, ValidatedResourceLimits};

    use super::*;

    const COLOR_HEX: &str = include_str!(
        "../../../../samples/machine-package/staging/production-book-1/jpeg-media/color-2x1.jpg.hex"
    );
    const GRAY_HEX: &str = include_str!(
        "../../../../samples/machine-package/staging/production-book-1/jpeg-media/gray-2x1.jpg.hex"
    );
    const COLOR_422_HEX: &str = include_str!(
        "../../../../samples/machine-package/staging/production-book-1/jpeg-media/color-17x9-422.jpg.hex"
    );
    const COLOR_440_HEX: &str = include_str!(
        "../../../../samples/machine-package/staging/production-book-1/jpeg-media/color-17x9-440.jpg.hex"
    );
    const COLOR_420_HEX: &str = include_str!(
        "../../../../samples/machine-package/staging/production-book-1/jpeg-media/color-17x9-420.jpg.hex"
    );

    fn decode_hex(source: &str) -> Vec<u8> {
        let digits: Vec<_> = source.bytes().filter(u8::is_ascii_hexdigit).collect();
        assert_eq!(digits.len() % 2, 0);
        digits
            .chunks_exact(2)
            .map(|pair| {
                let high = char::from(pair[0]).to_digit(16).unwrap();
                let low = char::from(pair[1]).to_digit(16).unwrap();
                u8::try_from((high << 4) | low).unwrap()
            })
            .collect()
    }

    fn effective(mut base: ResourceLimits) -> M4EffectiveResourceLimits {
        base.max_image_bytes = base.max_image_bytes.max(1_024);
        base.max_resource_bytes = base.max_resource_bytes.max(base.max_image_bytes);
        M4EffectiveResourceLimits::defaults_for(&ValidatedResourceLimits::new(base).unwrap())
    }

    fn admit(
        bytes: &[u8],
        limits: &M4EffectiveResourceLimits,
    ) -> Result<JpegAdmissionAttestation, ResourceAdmissionError> {
        admit_jpeg(
            ImageResourceId::new(7),
            sha256(bytes),
            bytes,
            limits,
            [0x55; 32],
        )
    }

    #[test]
    fn jpeg_color_and_gray_are_fully_decoded_and_sanitized() {
        let limits = effective(ResourceLimits::default());
        let color_bytes = decode_hex(COLOR_HEX);
        let gray_bytes = decode_hex(GRAY_HEX);
        let color = admit(&color_bytes, &limits).unwrap();
        let gray = admit(&gray_bytes, &limits).unwrap();

        assert_eq!((color.width().get(), color.height().get()), (2, 1));
        assert_eq!(color.color_kind(), JpegColorKind::YCbCr);
        assert_eq!(color.sampling(), JpegSampling::YCbCr444);
        assert_eq!(color.decoded_byte_length(), 6);
        assert_eq!((gray.width().get(), gray.height().get()), (2, 1));
        assert_eq!(gray.color_kind(), JpegColorKind::Grayscale);
        assert_eq!(gray.sampling(), JpegSampling::Gray);
        assert_eq!(gray.decoded_byte_length(), 2);
        for (source, receipt) in [(&color_bytes, &color), (&gray_bytes, &gray)] {
            assert_eq!(receipt.source_sha256(), sha256(source));
            assert_eq!(
                receipt.normalized_sha256(),
                sha256(receipt.normalized_bytes())
            );
            assert_eq!(&receipt.normalized_bytes()[..2], &[0xff, 0xd8]);
            assert_ne!(
                receipt.normalized_bytes().get(2..4),
                Some(&[0xff, 0xe0][..])
            );
            assert_eq!(receipt.decoder_id(), JPEG_DECODER_ID);
            assert_eq!(receipt.marker_preflight_id(), JPEG_MARKER_PREFLIGHT_ID);
            assert_eq!(receipt.sanitizer_id(), JPEG_SANITIZER_ID);
            assert_eq!(receipt.pixel_observation_id(), JPEG_PIXEL_OBSERVATION_ID);
        }
        assert_eq!(admit(&color_bytes, &limits).unwrap(), color);
    }

    #[test]
    fn jpeg_preflight_rejects_every_truncated_prefix_without_panicking() {
        let limits = effective(ResourceLimits::default());
        let bytes = decode_hex(COLOR_HEX);
        for end in 0..bytes.len() {
            assert!(admit(&bytes[..end], &limits).is_err(), "prefix {end}");
        }
    }

    #[test]
    fn jpeg_admits_every_closed_sampling_variant() {
        let limits = effective(ResourceLimits::default());
        for (source, sampling) in [
            (COLOR_HEX, JpegSampling::YCbCr444),
            (COLOR_422_HEX, JpegSampling::YCbCr422),
            (COLOR_440_HEX, JpegSampling::YCbCr440),
            (COLOR_420_HEX, JpegSampling::YCbCr420),
        ] {
            let bytes = decode_hex(source);
            let admitted = admit(&bytes, &limits)
                .unwrap_or_else(|error| panic!("{sampling:?} must be admitted: {error:?}"));
            assert_eq!(admitted.color_kind(), JpegColorKind::YCbCr);
            assert_eq!(admitted.sampling(), sampling);
            assert_eq!(
                admitted.decoded_byte_length(),
                u64::from(admitted.width().get()) * u64::from(admitted.height().get()) * 3
            );
        }
    }

    #[test]
    fn jpeg_rejects_unsupported_process_color_metadata_and_entropy() {
        let limits = effective(ResourceLimits::default());
        let bytes = decode_hex(COLOR_HEX);

        let mut progressive = bytes.clone();
        let sof = progressive
            .windows(2)
            .position(|window| window == [0xff, 0xc0])
            .unwrap();
        progressive[sof + 1] = 0xc2;
        assert_eq!(
            admit(&progressive, &limits),
            Err(ResourceAdmissionError::InvalidJpeg(
                JpegFailureReason::UnsupportedProcess
            ))
        );

        let mut unsupported_color = bytes.clone();
        let sof = unsupported_color
            .windows(2)
            .position(|window| window == [0xff, 0xc0])
            .unwrap();
        // The second component must remain Cb with 1x1 sampling. A 2x1 Cb
        // component is outside every closed YCbCr sampling profile.
        unsupported_color[sof + 14] = 0x21;
        assert_eq!(
            admit(&unsupported_color, &limits),
            Err(ResourceAdmissionError::InvalidJpeg(
                JpegFailureReason::UnsupportedProcess
            ))
        );

        let mut invalid_entropy = bytes.clone();
        let sos = invalid_entropy
            .windows(2)
            .position(|window| window == [0xff, 0xda])
            .unwrap();
        let sos_length = usize::from(u16::from_be_bytes([
            invalid_entropy[sos + 2],
            invalid_entropy[sos + 3],
        ]));
        let entropy_start = sos + 2 + sos_length;
        invalid_entropy[entropy_start..entropy_start + 2].copy_from_slice(&[0xff, 0xd0]);
        assert_eq!(
            admit(&invalid_entropy, &limits),
            Err(ResourceAdmissionError::InvalidJpeg(
                JpegFailureReason::InvalidEntropy
            ))
        );

        let mut metadata = Vec::new();
        metadata.extend_from_slice(&bytes[..20]);
        metadata.extend_from_slice(&[0xff, 0xe1, 0x00, 0x04, b'E', b'X']);
        metadata.extend_from_slice(&bytes[20..]);
        assert_eq!(
            admit(&metadata, &limits),
            Err(ResourceAdmissionError::InvalidJpeg(
                JpegFailureReason::ForbiddenMetadata
            ))
        );

        let mut thumbnail = bytes;
        thumbnail[19] = 1;
        assert_eq!(
            admit(&thumbnail, &limits),
            Err(ResourceAdmissionError::InvalidJpeg(
                JpegFailureReason::ForbiddenMetadata
            ))
        );

        for marker in [0xdb, 0xc4] {
            let mut empty_table = Vec::new();
            empty_table.extend_from_slice(&decode_hex(COLOR_HEX)[..20]);
            empty_table.extend_from_slice(&[0xff, marker, 0x00, 0x02]);
            empty_table.extend_from_slice(&decode_hex(COLOR_HEX)[20..]);
            assert_eq!(
                admit(&empty_table, &limits),
                Err(ResourceAdmissionError::InvalidJpeg(
                    JpegFailureReason::InvalidTables
                ))
            );
        }
    }

    #[test]
    fn jpeg_limits_accept_exact_and_reject_max_plus_one_before_decode() {
        let bytes = decode_hex(COLOR_HEX);
        let baseline = admit(&bytes, &effective(ResourceLimits::default())).unwrap();

        let exact = ResourceLimits {
            max_image_pixels: 2,
            max_decoded_image_bytes: baseline.peak_workspace_bytes(),
            max_spool_bytes: u64::try_from(bytes.len() + baseline.normalized_bytes().len())
                .unwrap(),
            ..ResourceLimits::default()
        };
        assert!(admit(&bytes, &effective(exact.clone())).is_ok());

        let mut pixels = exact.clone();
        pixels.max_image_pixels = 1;
        assert_eq!(
            admit(&bytes, &effective(pixels)),
            Err(ResourceAdmissionError::ResourceLimit)
        );

        let mut decode = exact.clone();
        decode.max_decoded_image_bytes -= 1;
        assert_eq!(
            admit(&bytes, &effective(decode)),
            Err(ResourceAdmissionError::DecodedImageLimit)
        );

        let mut spool = exact;
        spool.max_spool_bytes -= 1;
        assert_eq!(
            admit(&bytes, &effective(spool)),
            Err(ResourceAdmissionError::ResourceLimit)
        );

        let mut huge_dimensions = bytes;
        let sof = huge_dimensions
            .windows(2)
            .position(|window| window == [0xff, 0xc0])
            .unwrap();
        huge_dimensions[sof + 5..sof + 9].fill(0xff);
        assert_eq!(
            admit(&huge_dimensions, &effective(ResourceLimits::default())),
            Err(ResourceAdmissionError::ResourceLimit)
        );

        let tall = decode_hex(COLOR_422_HEX);
        let tall_baseline = admit(&tall, &effective(ResourceLimits::default())).unwrap();
        assert_eq!(tall_baseline.peak_workspace_bytes(), 4_096);
        let tall_exact = ResourceLimits {
            max_decoded_image_bytes: tall_baseline.peak_workspace_bytes(),
            ..ResourceLimits::default()
        };
        assert!(admit(&tall, &effective(tall_exact.clone())).is_ok());
        let mut tall_plus_one = tall_exact;
        tall_plus_one.max_decoded_image_bytes -= 1;
        assert_eq!(
            admit(&tall, &effective(tall_plus_one)),
            Err(ResourceAdmissionError::DecodedImageLimit)
        );
    }
}
