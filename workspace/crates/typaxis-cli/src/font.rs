//! Bounded, dependency-free metadata inspection for OpenType fonts.
//!
//! These commands deliberately parse only the table directory and the small
//! pieces of `head`, `maxp`, and `name` needed by the CLI.  They do not hand
//! untrusted bytes to a shaping or rasterization library.

use std::collections::BTreeMap;
use std::fmt;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

const MAX_FONT_BYTES: u64 = 128 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 100_000;
const MAX_FONT_FILES: usize = 16_384;
const MAX_FACES_PER_FILE: usize = 4_096;
const MAX_TOTAL_FACES: usize = 65_536;
const MAX_TABLES_PER_FACE: usize = 4_096;
const MAX_NAME_RECORDS: usize = 4_096;
const MAX_DECODED_NAME_BYTES: usize = 16 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 32 * 1024 * 1024;

/// A small error type that maps directly onto the CLI's documented exit
/// classes: I/O (3), invalid input (1), and resource limit (5).
#[derive(Debug)]
pub(crate) enum FontCommandError {
    Io { path: PathBuf, source: io::Error },
    InvalidInput { path: PathBuf, detail: String },
    ResourceLimit { path: PathBuf, detail: String },
}

impl FontCommandError {
    pub(crate) const fn is_io(&self) -> bool {
        matches!(self, Self::Io { .. })
    }

    pub(crate) const fn is_resource_limit(&self) -> bool {
        matches!(self, Self::ResourceLimit { .. })
    }

    fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.to_path_buf(),
            source,
        }
    }

    fn invalid(path: &Path, detail: impl Into<String>) -> Self {
        Self::InvalidInput {
            path: path.to_path_buf(),
            detail: detail.into(),
        }
    }

    fn limit(path: &Path, detail: impl Into<String>) -> Self {
        Self::ResourceLimit {
            path: path.to_path_buf(),
            detail: detail.into(),
        }
    }
}

impl fmt::Display for FontCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(formatter, "cannot read `{}`: {source}", path.display())
            }
            Self::InvalidInput { path, detail } => {
                write!(
                    formatter,
                    "invalid font input `{}`: {detail}",
                    path.display()
                )
            }
            Self::ResourceLimit { path, detail } => {
                write!(
                    formatter,
                    "font resource limit for `{}`: {detail}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for FontCommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidInput { .. } | Self::ResourceLimit { .. } => None,
        }
    }
}

/// Inspect one standalone SFNT font or TrueType/OpenType collection.
///
/// The returned value is one compact JSON object with one trailing newline.
/// Only the final file name is emitted: a readable display string is paired
/// with a lossless hexadecimal identifier for the platform-native name bytes.
pub(crate) fn inspect_font(path: &Path) -> Result<String, FontCommandError> {
    inspect_font_json(path)
}

pub(crate) fn inspect_font_json(path: &Path) -> Result<String, FontCommandError> {
    let bytes = read_font_file(path)?;
    let metadata = parse_font(&bytes).map_err(|failure| failure.at_path(path))?;
    encode_inspection(path, &metadata)
        .ok_or_else(|| FontCommandError::limit(path, "JSON output exceeds 32 MiB"))
}

/// Inspect the direct children of a directory in platform-native path order.
///
/// Regular files with a recognized SFNT/TTC signature are included regardless
/// of extension.  Ordinary non-font files are ignored.  A file whose extension
/// claims to be a font but whose contents are malformed is reported as invalid
/// input rather than silently disappearing from the listing.
pub(crate) fn list_fonts(directory: &Path) -> Result<String, FontCommandError> {
    list_fonts_json(directory)
}

pub(crate) fn list_fonts_json(directory: &Path) -> Result<String, FontCommandError> {
    let directory_metadata =
        fs::metadata(directory).map_err(|error| FontCommandError::io(directory, error))?;
    if !directory_metadata.is_dir() {
        return Err(FontCommandError::invalid(
            directory,
            "the supplied path is not a directory",
        ));
    }

    let reader = fs::read_dir(directory).map_err(|error| FontCommandError::io(directory, error))?;
    let mut entries = Vec::new();
    for entry in reader {
        if entries.len() == MAX_DIRECTORY_ENTRIES {
            return Err(FontCommandError::limit(
                directory,
                format!("directory contains more than {MAX_DIRECTORY_ENTRIES} entries"),
            ));
        }
        let entry = entry.map_err(|error| FontCommandError::io(directory, error))?;
        entries.push((entry.file_name(), entry.path()));
    }

    // OsString/Path ordering uses the platform-native representation.  It is
    // independent of read_dir enumeration order and does not require UTF-8.
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    let mut fonts = Vec::new();
    let mut budget = FontListingBudget::default();
    let mut total_name_bytes = 0usize;
    for (_, path) in entries {
        let metadata = fs::metadata(&path).map_err(|error| FontCommandError::io(&path, error))?;
        if !metadata.is_file() {
            continue;
        }

        let extension_claims_font = has_font_extension(&path);
        let Some(parsed) =
            inspect_listing_candidate(directory, &path, extension_claims_font, &mut budget)?
        else {
            continue;
        };
        let file_name_bytes = parsed.faces.iter().try_fold(0usize, |total, face| {
            total
                .checked_add(face.family.as_ref().map_or(0, String::len))?
                .checked_add(face.full_name.as_ref().map_or(0, String::len))?
                .checked_add(face.postscript_name.as_ref().map_or(0, String::len))
        });
        total_name_bytes = total_name_bytes
            .checked_add(file_name_bytes.ok_or_else(|| {
                FontCommandError::limit(directory, "decoded font name byte count overflow")
            })?)
            .ok_or_else(|| {
                FontCommandError::limit(directory, "decoded font name byte count overflow")
            })?;
        if total_name_bytes > MAX_JSON_BYTES {
            return Err(FontCommandError::limit(
                directory,
                "decoded font names exceed the JSON output limit",
            ));
        }
        fonts.push((path, parsed));
    }

    encode_listing(&fonts)
        .ok_or_else(|| FontCommandError::limit(directory, "JSON output exceeds 32 MiB"))
}

fn inspect_listing_candidate(
    directory: &Path,
    path: &Path,
    extension_claims_font: bool,
    budget: &mut FontListingBudget,
) -> Result<Option<FontMetadata>, FontCommandError> {
    let Some(mut candidate) = open_font_candidate(path, extension_claims_font)? else {
        return Ok(None);
    };
    budget.admit_font_file(directory)?;
    let declared_faces = candidate.declared_face_count(path)?;
    budget.admit_faces(directory, declared_faces)?;
    let bytes = candidate.read_all(path)?;
    let parsed = parse_font(&bytes).map_err(|failure| failure.at_path(path))?;
    debug_assert_eq!(parsed.faces.len(), declared_faces);
    Ok(Some(parsed))
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FontMetadata {
    faces: Vec<FaceMetadata>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FaceMetadata {
    face_index: u32,
    family: Option<String>,
    full_name: Option<String>,
    postscript_name: Option<String>,
    units_per_em: u16,
    glyph_count: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParseFailure {
    Invalid(&'static str),
    Limit(&'static str),
}

impl ParseFailure {
    fn at_path(self, path: &Path) -> FontCommandError {
        match self {
            Self::Invalid(detail) => FontCommandError::invalid(path, detail),
            Self::Limit(detail) => FontCommandError::limit(path, detail),
        }
    }
}

fn read_font_file(path: &Path) -> Result<Vec<u8>, FontCommandError> {
    let (file, snapshot) = open_stable_font(path)?;
    read_open_file(path, file, snapshot)
}

#[derive(Default)]
struct FontListingBudget {
    font_files: usize,
    total_faces: usize,
}

impl FontListingBudget {
    fn admit_font_file(&mut self, directory: &Path) -> Result<(), FontCommandError> {
        if self.font_files == MAX_FONT_FILES {
            return Err(FontCommandError::limit(
                directory,
                format!("directory contains more than {MAX_FONT_FILES} font files"),
            ));
        }
        self.font_files += 1;
        Ok(())
    }

    fn admit_faces(&mut self, directory: &Path, face_count: usize) -> Result<(), FontCommandError> {
        let total_faces = self
            .total_faces
            .checked_add(face_count)
            .ok_or_else(|| FontCommandError::limit(directory, "font face count overflow"))?;
        if total_faces > MAX_TOTAL_FACES {
            return Err(FontCommandError::limit(
                directory,
                format!("directory contains more than {MAX_TOTAL_FACES} font faces"),
            ));
        }
        self.total_faces = total_faces;
        Ok(())
    }
}

struct OpenFontCandidate {
    file: File,
    snapshot: FontFileSnapshot,
    signature: [u8; 4],
}

impl OpenFontCandidate {
    fn declared_face_count(&mut self, path: &Path) -> Result<usize, FontCommandError> {
        if self.signature != *b"ttcf" {
            return Ok(1);
        }
        if self.snapshot.length < 12 {
            ensure_font_unchanged(path, &self.file, self.snapshot)?;
            return Err(FontCommandError::invalid(path, "TTC header is truncated"));
        }
        let mut remainder = [0u8; 8];
        self.file
            .read_exact(&mut remainder)
            .map_err(|error| FontCommandError::io(path, error))?;
        let version = u32::from_be_bytes(remainder[0..4].try_into().expect("four-byte slice"));
        if version != 0x0001_0000 && version != 0x0002_0000 {
            return Err(FontCommandError::invalid(
                path,
                "unsupported TTC header version",
            ));
        }
        let face_count = usize::try_from(u32::from_be_bytes(
            remainder[4..8].try_into().expect("four-byte slice"),
        ))
        .map_err(|_| FontCommandError::limit(path, "TTC face count exceeds supported range"))?;
        if face_count == 0 {
            return Err(FontCommandError::invalid(
                path,
                "TTC contains no font faces",
            ));
        }
        if face_count > MAX_FACES_PER_FILE {
            return Err(FontCommandError::limit(
                path,
                "TTC contains too many font faces",
            ));
        }
        Ok(face_count)
    }

    fn read_all(mut self, path: &Path) -> Result<Vec<u8>, FontCommandError> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| FontCommandError::io(path, error))?;
        read_open_file(path, self.file, self.snapshot)
    }
}

fn open_font_candidate(
    path: &Path,
    extension_claims_font: bool,
) -> Result<Option<OpenFontCandidate>, FontCommandError> {
    let (mut file, snapshot) = open_stable_font(path)?;
    if snapshot.length < 4 {
        ensure_font_unchanged(path, &file, snapshot)?;
        if extension_claims_font {
            return Err(FontCommandError::invalid(
                path,
                "file extension denotes a font, but the SFNT/TTC signature is missing",
            ));
        }
        return Ok(None);
    }
    let mut signature = [0u8; 4];
    file.read_exact(&mut signature)
        .map_err(|error| FontCommandError::io(path, error))?;
    if !is_font_signature(signature) {
        ensure_font_unchanged(path, &file, snapshot)?;
        if extension_claims_font {
            return Err(FontCommandError::invalid(
                path,
                "file extension denotes a font, but the SFNT/TTC signature is missing",
            ));
        }
        return Ok(None);
    }
    Ok(Some(OpenFontCandidate {
        file,
        snapshot,
        signature,
    }))
}

fn read_open_file(
    path: &Path,
    mut file: File,
    snapshot: FontFileSnapshot,
) -> Result<Vec<u8>, FontCommandError> {
    let observed_length = snapshot.length;
    if observed_length > MAX_FONT_BYTES {
        return Err(FontCommandError::limit(
            path,
            format!("font is {observed_length} bytes; maximum is {MAX_FONT_BYTES}"),
        ));
    }
    let capacity = usize::try_from(observed_length)
        .map_err(|_| FontCommandError::limit(path, "font length does not fit in memory"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| FontCommandError::limit(path, "cannot reserve memory for font"))?;
    bytes.resize(capacity, 0);
    file.read_exact(&mut bytes)
        .map_err(|error| FontCommandError::io(path, error))?;
    ensure_font_unchanged(path, &file, snapshot)?;
    Ok(bytes)
}

fn ensure_font_unchanged(
    path: &Path,
    file: &File,
    snapshot: FontFileSnapshot,
) -> Result<(), FontCommandError> {
    if FontFileSnapshot::from_file(file, path)? != snapshot {
        Err(FontCommandError::io(
            path,
            io::Error::other("font changed while it was being read"),
        ))
    } else {
        Ok(())
    }
}

fn open_stable_font(path: &Path) -> Result<(File, FontFileSnapshot), FontCommandError> {
    let file = open_font_handle(path)?;
    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockShared)
        .map_err(|error| FontCommandError::io(path, io::Error::from(error)))?;
    let snapshot = FontFileSnapshot::from_file(&file, path)?;
    if !snapshot.regular {
        return Err(FontCommandError::invalid(
            path,
            "the supplied path is not a regular file",
        ));
    }
    if snapshot.length > MAX_FONT_BYTES {
        return Err(FontCommandError::limit(
            path,
            format!(
                "font is {} bytes; maximum is {MAX_FONT_BYTES}",
                snapshot.length
            ),
        ));
    }
    Ok((file, snapshot))
}

fn open_font_handle(path: &Path) -> Result<File, FontCommandError> {
    #[cfg(unix)]
    {
        use rustix::fs::{Mode, OFlags};

        let descriptor = rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|error| FontCommandError::io(path, io::Error::from(error)))?;
        Ok(descriptor.into())
    }
    #[cfg(not(unix))]
    {
        let metadata = fs::metadata(path).map_err(|error| FontCommandError::io(path, error))?;
        if !metadata.is_file() {
            return Err(FontCommandError::invalid(
                path,
                "the supplied path is not a regular file",
            ));
        }
        File::open(path).map_err(|error| FontCommandError::io(path, error))
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FontFileSnapshot {
    device: u128,
    inode: u128,
    length: u64,
    modified_seconds: i128,
    modified_nanoseconds: u128,
    changed_seconds: i128,
    changed_nanoseconds: u128,
    regular: bool,
}

#[cfg(any(target_os = "android", target_os = "linux"))]
impl FontFileSnapshot {
    fn from_file(file: &File, path: &Path) -> Result<Self, FontCommandError> {
        let stat = rustix::fs::fstat(file)
            .map_err(|error| FontCommandError::io(path, io::Error::from(error)))?;
        Ok(Self {
            device: u128::from(stat.st_dev),
            inode: u128::from(stat.st_ino),
            length: u64::try_from(stat.st_size)
                .map_err(|_| FontCommandError::limit(path, "font length is negative"))?,
            modified_seconds: i128::from(stat.st_mtime),
            modified_nanoseconds: u128::from(stat.st_mtime_nsec),
            changed_seconds: i128::from(stat.st_ctime),
            changed_nanoseconds: u128::from(stat.st_ctime_nsec),
            regular: rustix::fs::FileType::from_raw_mode(stat.st_mode)
                == rustix::fs::FileType::RegularFile,
        })
    }
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FontFileSnapshot {
    device: u64,
    inode: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
    regular: bool,
}

#[cfg(all(unix, not(any(target_os = "android", target_os = "linux"))))]
impl FontFileSnapshot {
    fn from_file(file: &File, path: &Path) -> Result<Self, FontCommandError> {
        use std::os::unix::fs::MetadataExt;

        let metadata = file
            .metadata()
            .map_err(|error| FontCommandError::io(path, error))?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
            regular: metadata.is_file(),
        })
    }
}

#[cfg(not(unix))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct FontFileSnapshot {
    length: u64,
    modified: std::time::SystemTime,
    regular: bool,
}

#[cfg(not(unix))]
impl FontFileSnapshot {
    fn from_file(file: &File, path: &Path) -> Result<Self, FontCommandError> {
        let metadata = file
            .metadata()
            .map_err(|error| FontCommandError::io(path, error))?;
        let modified = metadata.modified().map_err(|error| {
            FontCommandError::io(
                path,
                io::Error::other(format!(
                    "cannot read font modification time for a stable read: {error}"
                )),
            )
        })?;
        Ok(Self {
            length: metadata.len(),
            modified,
            regular: metadata.is_file(),
        })
    }
}

fn has_font_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("ttf")
                || extension.eq_ignore_ascii_case("otf")
                || extension.eq_ignore_ascii_case("ttc")
                || extension.eq_ignore_ascii_case("otc")
        })
}

fn is_font_signature(signature: [u8; 4]) -> bool {
    matches!(
        &signature,
        b"\0\x01\0\0" | b"OTTO" | b"true" | b"typ1" | b"ttcf"
    )
}

fn parse_font(bytes: &[u8]) -> Result<FontMetadata, ParseFailure> {
    let signature = read_array_4(bytes, 0)?;
    let offsets = if signature == *b"ttcf" {
        parse_collection_header(bytes)?
    } else if is_sfnt_signature(signature) {
        vec![0usize]
    } else {
        return Err(ParseFailure::Invalid(
            "unsupported or missing SFNT/TTC signature",
        ));
    };

    let mut decoded_name_bytes = 0usize;
    let mut faces = Vec::new();
    faces
        .try_reserve_exact(offsets.len())
        .map_err(|_| ParseFailure::Limit("cannot reserve memory for font faces"))?;
    for (face_index, offset) in offsets.into_iter().enumerate() {
        let face_index = u32::try_from(face_index)
            .map_err(|_| ParseFailure::Limit("font face index exceeds supported range"))?;
        faces.push(parse_face(
            bytes,
            offset,
            face_index,
            &mut decoded_name_bytes,
        )?);
    }
    Ok(FontMetadata { faces })
}

fn parse_collection_header(bytes: &[u8]) -> Result<Vec<usize>, ParseFailure> {
    let version = read_u32(bytes, 4)?;
    if version != 0x0001_0000 && version != 0x0002_0000 {
        return Err(ParseFailure::Invalid("unsupported TTC header version"));
    }
    let count = usize::try_from(read_u32(bytes, 8)?)
        .map_err(|_| ParseFailure::Limit("TTC face count exceeds supported range"))?;
    if count == 0 {
        return Err(ParseFailure::Invalid("TTC contains no font faces"));
    }
    if count > MAX_FACES_PER_FILE {
        return Err(ParseFailure::Limit("TTC contains too many font faces"));
    }
    let offsets_bytes = count
        .checked_mul(4)
        .and_then(|length| length.checked_add(12))
        .ok_or(ParseFailure::Invalid("TTC offset array overflows"))?;
    checked_slice(bytes, 0, offsets_bytes)?;
    if version == 0x0002_0000 {
        checked_slice(bytes, offsets_bytes, 12)?;
    }

    let mut offsets = Vec::new();
    offsets
        .try_reserve_exact(count)
        .map_err(|_| ParseFailure::Limit("cannot reserve memory for TTC offsets"))?;
    for index in 0..count {
        let position = 12usize
            .checked_add(
                index
                    .checked_mul(4)
                    .ok_or(ParseFailure::Invalid("TTC offset position overflows"))?,
            )
            .ok_or(ParseFailure::Invalid("TTC offset position overflows"))?;
        let offset = usize::try_from(read_u32(bytes, position)?)
            .map_err(|_| ParseFailure::Invalid("TTC face offset is out of range"))?;
        if offset % 4 != 0 {
            return Err(ParseFailure::Invalid(
                "TTC face offset is not 4-byte aligned",
            ));
        }
        if offsets.contains(&offset) {
            return Err(ParseFailure::Invalid(
                "TTC contains a duplicate face offset",
            ));
        }
        checked_slice(bytes, offset, 12)?;
        offsets.push(offset);
    }
    Ok(offsets)
}

fn parse_face(
    bytes: &[u8],
    face_offset: usize,
    face_index: u32,
    decoded_name_bytes: &mut usize,
) -> Result<FaceMetadata, ParseFailure> {
    let signature = read_array_4(bytes, face_offset)?;
    if !is_sfnt_signature(signature) {
        return Err(ParseFailure::Invalid(
            "collection face has an unsupported SFNT signature",
        ));
    }
    let table_count_position = face_offset
        .checked_add(4)
        .ok_or(ParseFailure::Invalid("SFNT directory offset overflows"))?;
    let table_count = usize::from(read_u16(bytes, table_count_position)?);
    if table_count == 0 {
        return Err(ParseFailure::Invalid("SFNT table directory is empty"));
    }
    if table_count > MAX_TABLES_PER_FACE {
        return Err(ParseFailure::Limit("SFNT contains too many tables"));
    }
    let directory_length = table_count
        .checked_mul(16)
        .and_then(|length| length.checked_add(12))
        .ok_or(ParseFailure::Invalid("SFNT table directory overflows"))?;
    checked_slice(bytes, face_offset, directory_length)?;

    let mut tables = BTreeMap::new();
    for index in 0..table_count {
        let record = face_offset
            .checked_add(12)
            .and_then(|position| position.checked_add(index.checked_mul(16)?))
            .ok_or(ParseFailure::Invalid("SFNT table record offset overflows"))?;
        let tag = read_array_4(bytes, record)?;
        let offset = usize::try_from(read_u32(bytes, record + 8)?)
            .map_err(|_| ParseFailure::Invalid("SFNT table offset is out of range"))?;
        let length = usize::try_from(read_u32(bytes, record + 12)?)
            .map_err(|_| ParseFailure::Invalid("SFNT table length is out of range"))?;
        checked_slice(bytes, offset, length)?;
        if tables.insert(tag, (offset, length)).is_some() {
            return Err(ParseFailure::Invalid("SFNT contains a duplicate table tag"));
        }
    }

    let head = required_table(bytes, &tables, *b"head", 54)?;
    if read_u32(head, 12)? != 0x5f0f_3cf5 {
        return Err(ParseFailure::Invalid(
            "`head` table has an invalid magic number",
        ));
    }
    let units_per_em = read_u16(head, 18)?;
    if !(16..=16_384).contains(&units_per_em) {
        return Err(ParseFailure::Invalid(
            "`head.unitsPerEm` is outside the OpenType range 16..=16384",
        ));
    }

    let maxp = required_table(bytes, &tables, *b"maxp", 6)?;
    let maxp_version = read_u32(maxp, 0)?;
    if maxp_version != 0x0000_5000 && maxp_version != 0x0001_0000 {
        return Err(ParseFailure::Invalid("`maxp` table has an invalid version"));
    }
    let glyph_count = read_u16(maxp, 4)?;
    if glyph_count == 0 {
        return Err(ParseFailure::Invalid("`maxp.numGlyphs` must be positive"));
    }

    let names = match tables.get(b"name") {
        Some(&(offset, length)) => parse_name_table(&bytes[offset..offset + length])?,
        None => ParsedNames::default(),
    };
    let added_name_bytes = names
        .family
        .as_ref()
        .map_or(0, String::len)
        .checked_add(names.full_name.as_ref().map_or(0, String::len))
        .and_then(|count| count.checked_add(names.postscript_name.as_ref().map_or(0, String::len)))
        .ok_or(ParseFailure::Limit(
            "decoded font name byte count overflows",
        ))?;
    *decoded_name_bytes =
        decoded_name_bytes
            .checked_add(added_name_bytes)
            .ok_or(ParseFailure::Limit(
                "decoded font name byte count overflows",
            ))?;
    if *decoded_name_bytes > MAX_DECODED_NAME_BYTES {
        return Err(ParseFailure::Limit("decoded font names exceed 16 MiB"));
    }

    Ok(FaceMetadata {
        face_index,
        family: names.family,
        full_name: names.full_name,
        postscript_name: names.postscript_name,
        units_per_em,
        glyph_count,
    })
}

fn is_sfnt_signature(signature: [u8; 4]) -> bool {
    matches!(&signature, b"\0\x01\0\0" | b"OTTO" | b"true" | b"typ1")
}

fn required_table<'a>(
    bytes: &'a [u8],
    tables: &BTreeMap<[u8; 4], (usize, usize)>,
    tag: [u8; 4],
    minimum_length: usize,
) -> Result<&'a [u8], ParseFailure> {
    let &(offset, length) = tables.get(&tag).ok_or(ParseFailure::Invalid(
        "SFNT is missing a required metadata table",
    ))?;
    if length < minimum_length {
        return Err(ParseFailure::Invalid(
            "required SFNT metadata table is truncated",
        ));
    }
    checked_slice(bytes, offset, length)
}

#[derive(Default)]
struct ParsedNames {
    family: Option<String>,
    full_name: Option<String>,
    postscript_name: Option<String>,
}

#[derive(Clone, Copy)]
struct NameRecord<'a> {
    platform: u16,
    encoding: u16,
    language: u16,
    name_id: u16,
    order: usize,
    bytes: &'a [u8],
}

fn parse_name_table(table: &[u8]) -> Result<ParsedNames, ParseFailure> {
    checked_slice(table, 0, 6)?;
    let format = read_u16(table, 0)?;
    if format != 0 && format != 1 {
        return Err(ParseFailure::Invalid(
            "`name` table has an unsupported format",
        ));
    }
    let record_count = usize::from(read_u16(table, 2)?);
    if record_count > MAX_NAME_RECORDS {
        return Err(ParseFailure::Limit(
            "`name` table contains too many records",
        ));
    }
    let storage_offset = usize::from(read_u16(table, 4)?);
    let records_end = record_count
        .checked_mul(12)
        .and_then(|length| length.checked_add(6))
        .ok_or(ParseFailure::Invalid("`name` record array overflows"))?;
    checked_slice(table, 0, records_end)?;

    let metadata_end = if format == 1 {
        let language_count = usize::from(read_u16(table, records_end)?);
        if language_count > MAX_NAME_RECORDS {
            return Err(ParseFailure::Limit(
                "`name` table contains too many language-tag records",
            ));
        }
        let language_records_start = records_end
            .checked_add(2)
            .ok_or(ParseFailure::Invalid("`name` language-tag array overflows"))?;
        let language_records_end = language_count
            .checked_mul(4)
            .and_then(|length| length.checked_add(language_records_start))
            .ok_or(ParseFailure::Invalid("`name` language-tag array overflows"))?;
        checked_slice(table, 0, language_records_end)?;
        for index in 0..language_count {
            let position = language_records_start + index * 4;
            let length = usize::from(read_u16(table, position)?);
            let offset = usize::from(read_u16(table, position + 2)?);
            let start = storage_offset
                .checked_add(offset)
                .ok_or(ParseFailure::Invalid(
                    "`name` language-tag offset overflows",
                ))?;
            checked_slice(table, start, length)?;
        }
        language_records_end
    } else {
        records_end
    };
    if storage_offset < metadata_end || storage_offset > table.len() {
        return Err(ParseFailure::Invalid(
            "`name` string storage overlaps metadata or is out of bounds",
        ));
    }

    let mut records = Vec::new();
    records
        .try_reserve_exact(record_count)
        .map_err(|_| ParseFailure::Limit("cannot reserve memory for `name` records"))?;
    for order in 0..record_count {
        let position = 6 + order * 12;
        let length = usize::from(read_u16(table, position + 8)?);
        let offset = usize::from(read_u16(table, position + 10)?);
        let start = storage_offset
            .checked_add(offset)
            .ok_or(ParseFailure::Invalid("`name` string offset overflows"))?;
        let value = checked_slice(table, start, length)?;
        records.push(NameRecord {
            platform: read_u16(table, position)?,
            encoding: read_u16(table, position + 2)?,
            language: read_u16(table, position + 4)?,
            name_id: read_u16(table, position + 6)?,
            order,
            bytes: value,
        });
    }

    Ok(ParsedNames {
        family: choose_name(&records, &[16, 1])?,
        full_name: choose_name(&records, &[4])?,
        postscript_name: choose_name(&records, &[6])?,
    })
}

fn choose_name(
    records: &[NameRecord<'_>],
    preferred_ids: &[u16],
) -> Result<Option<String>, ParseFailure> {
    let mut best = None;
    for record in records {
        let Some(id_rank) = preferred_ids.iter().position(|id| *id == record.name_id) else {
            continue;
        };
        let Some(value) = decode_name(*record)? else {
            continue;
        };
        if value.is_empty() {
            continue;
        }
        let rank = (
            id_rank,
            language_rank(record.platform, record.language),
            encoding_rank(record.platform, record.encoding),
            record.order,
        );
        if best
            .as_ref()
            .map_or(true, |(best_rank, _): &(_, String)| rank < *best_rank)
        {
            best = Some((rank, value));
        }
    }
    Ok(best.map(|(_, value)| value))
}

fn language_rank(platform: u16, language: u16) -> u8 {
    match (platform, language) {
        (3, 0x0409) => 0,
        (3, language) if language & 0x03ff == 0x0009 => 1,
        (0, _) => 2,
        (3, _) => 3,
        (1, 0) => 4,
        (1, _) => 5,
        _ => 6,
    }
}

fn encoding_rank(platform: u16, encoding: u16) -> u8 {
    match (platform, encoding) {
        (3, 10) => 0,
        (3, 1) => 1,
        (3, 0) => 2,
        (0, _) => 0,
        (1, 0) => 0,
        _ => 3,
    }
}

fn decode_name(record: NameRecord<'_>) -> Result<Option<String>, ParseFailure> {
    match record.platform {
        0 | 3 => {
            if record.bytes.len() % 2 != 0 {
                return Err(ParseFailure::Invalid(
                    "Unicode `name` record has an odd byte length",
                ));
            }
            let units = record
                .bytes
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]));
            let mut value = String::new();
            for scalar in char::decode_utf16(units) {
                value.push(scalar.map_err(|_| {
                    ParseFailure::Invalid("Unicode `name` record contains an invalid surrogate")
                })?);
            }
            Ok(Some(value))
        }
        // Macintosh and ISO-platform names are a compatibility fallback.  The
        // ASCII range (where legacy family/PostScript names normally live) is
        // exact; upper bytes are represented deterministically as Latin-1.
        1 | 2 => Ok(Some(
            record.bytes.iter().map(|byte| char::from(*byte)).collect(),
        )),
        _ => Ok(None),
    }
}

fn encode_inspection(path: &Path, metadata: &FontMetadata) -> Option<String> {
    let mut json = String::new();
    json.push_str("{\"face_count\":");
    json.push_str(&metadata.faces.len().to_string());
    json.push_str(",\"faces\":");
    push_faces(&mut json, &metadata.faces);
    json.push_str(",\"file_name\":");
    push_file_name(&mut json, path);
    json.push('}');
    json.push('\n');
    (json.len() <= MAX_JSON_BYTES).then_some(json)
}

fn encode_listing(fonts: &[(PathBuf, FontMetadata)]) -> Option<String> {
    let mut json = String::from("{\"fonts\":[");
    for (index, (path, metadata)) in fonts.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"face_count\":");
        json.push_str(&metadata.faces.len().to_string());
        json.push_str(",\"faces\":");
        push_faces(&mut json, &metadata.faces);
        json.push_str(",\"file_name\":");
        push_file_name(&mut json, path);
        json.push('}');
        if json.len() > MAX_JSON_BYTES {
            return None;
        }
    }
    json.push_str("]}\n");
    (json.len() <= MAX_JSON_BYTES).then_some(json)
}

fn push_faces(json: &mut String, faces: &[FaceMetadata]) {
    json.push('[');
    for (index, face) in faces.iter().enumerate() {
        if index != 0 {
            json.push(',');
        }
        json.push_str("{\"face_index\":");
        json.push_str(&face.face_index.to_string());
        json.push_str(",\"family\":");
        push_optional_json_string(json, face.family.as_deref());
        json.push_str(",\"full_name\":");
        push_optional_json_string(json, face.full_name.as_deref());
        json.push_str(",\"glyph_count\":");
        json.push_str(&face.glyph_count.to_string());
        json.push_str(",\"postscript_name\":");
        push_optional_json_string(json, face.postscript_name.as_deref());
        json.push_str(",\"units_per_em\":");
        json.push_str(&face.units_per_em.to_string());
        json.push('}');
    }
    json.push(']');
}

fn push_file_name(json: &mut String, path: &Path) {
    let file_name = path.file_name().unwrap_or(path.as_os_str());
    json.push_str("{\"display\":");
    push_json_string(json, &file_name.to_string_lossy());
    json.push_str(",\"native_hex\":\"");
    push_lower_hex(json, file_name.as_encoded_bytes());
    json.push_str("\"}");
}

fn push_lower_hex(output: &mut String, bytes: &[u8]) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in bytes {
        output.push(char::from(HEX[usize::from(*byte >> 4)]));
        output.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
}

fn push_optional_json_string(json: &mut String, value: Option<&str>) {
    match value {
        Some(value) => push_json_string(json, value),
        None => json.push_str("null"),
    }
}

fn push_json_string(json: &mut String, value: &str) {
    json.push('"');
    for character in value.chars() {
        match character {
            '"' => json.push_str("\\\""),
            '\\' => json.push_str("\\\\"),
            '\u{08}' => json.push_str("\\b"),
            '\u{0c}' => json.push_str("\\f"),
            '\n' => json.push_str("\\n"),
            '\r' => json.push_str("\\r"),
            '\t' => json.push_str("\\t"),
            character if character <= '\u{1f}' => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let value = character as usize;
                json.push_str("\\u00");
                json.push(char::from(HEX[(value >> 4) & 0x0f]));
                json.push(char::from(HEX[value & 0x0f]));
            }
            character => json.push(character),
        }
    }
    json.push('"');
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ParseFailure> {
    let value = checked_slice(bytes, offset, 2)?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ParseFailure> {
    let value = checked_slice(bytes, offset, 4)?;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_array_4(bytes: &[u8], offset: usize) -> Result<[u8; 4], ParseFailure> {
    let value = checked_slice(bytes, offset, 4)?;
    Ok([value[0], value[1], value[2], value[3]])
}

fn checked_slice(bytes: &[u8], offset: usize, length: usize) -> Result<&[u8], ParseFailure> {
    let end = offset
        .checked_add(length)
        .ok_or(ParseFailure::Invalid("font byte range overflows"))?;
    bytes
        .get(offset..end)
        .ok_or(ParseFailure::Invalid("font byte range is out of bounds"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(target_os = "android", target_os = "linux"))]
    use std::ffi::OsString;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "typaxis-font-test-{}-{nonce}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create unique test directory");
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn utf16_be(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_be_bytes).collect()
    }

    fn make_name_table(family: &str, full_name: &str, postscript_name: &str) -> Vec<u8> {
        let values = [
            (1u16, utf16_be(family)),
            (4u16, utf16_be(full_name)),
            (6u16, utf16_be(postscript_name)),
        ];
        let storage_offset = 6 + values.len() * 12;
        let mut table = Vec::new();
        push_u16(&mut table, 0);
        push_u16(&mut table, values.len() as u16);
        push_u16(&mut table, storage_offset as u16);
        let mut string_offset = 0usize;
        for (name_id, value) in &values {
            push_u16(&mut table, 3);
            push_u16(&mut table, 1);
            push_u16(&mut table, 0x0409);
            push_u16(&mut table, *name_id);
            push_u16(&mut table, value.len() as u16);
            push_u16(&mut table, string_offset as u16);
            string_offset += value.len();
        }
        for (_, value) in values {
            table.extend_from_slice(&value);
        }
        table
    }

    fn make_sfnt(
        absolute_base: usize,
        family: &str,
        full_name: &str,
        postscript_name: &str,
        units_per_em: u16,
        glyph_count: u16,
    ) -> Vec<u8> {
        let name = make_name_table(family, full_name, postscript_name);
        let directory_length = 12 + 3 * 16;
        let head_offset = absolute_base + directory_length;
        let maxp_offset = head_offset + 54;
        let name_offset = maxp_offset + 6;

        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\0\x01\0\0");
        push_u16(&mut bytes, 3);
        bytes.extend_from_slice(&[0; 6]);
        for (tag, offset, length) in [
            (*b"head", head_offset, 54usize),
            (*b"maxp", maxp_offset, 6usize),
            (*b"name", name_offset, name.len()),
        ] {
            bytes.extend_from_slice(&tag);
            push_u32(&mut bytes, 0);
            push_u32(&mut bytes, offset as u32);
            push_u32(&mut bytes, length as u32);
        }
        let mut head = [0u8; 54];
        head[12..16].copy_from_slice(&0x5f0f_3cf5u32.to_be_bytes());
        head[18..20].copy_from_slice(&units_per_em.to_be_bytes());
        bytes.extend_from_slice(&head);
        push_u32(&mut bytes, 0x0001_0000);
        push_u16(&mut bytes, glyph_count);
        bytes.extend_from_slice(&name);
        bytes
    }

    fn make_collection() -> Vec<u8> {
        let first_offset = 20usize;
        let first = make_sfnt(
            first_offset,
            "Alpha",
            "Alpha Regular",
            "Alpha-Regular",
            1000,
            7,
        );
        let second_offset = (first_offset + first.len() + 3) & !3;
        let second = make_sfnt(
            second_offset,
            "Beta",
            "Beta Regular",
            "Beta-Regular",
            2048,
            9,
        );
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"ttcf");
        push_u32(&mut bytes, 0x0001_0000);
        push_u32(&mut bytes, 2);
        push_u32(&mut bytes, first_offset as u32);
        push_u32(&mut bytes, second_offset as u32);
        bytes.extend_from_slice(&first);
        bytes.resize(second_offset, 0);
        bytes.extend_from_slice(&second);
        bytes
    }

    #[test]
    fn standalone_metadata_and_json_are_deterministic() {
        let bytes = make_sfnt(0, "Example", "Example Regular", "Example-Regular", 1000, 7);
        let metadata = parse_font(&bytes).unwrap();
        assert_eq!(metadata.faces.len(), 1);
        assert_eq!(metadata.faces[0].family.as_deref(), Some("Example"));
        assert_eq!(
            metadata.faces[0].full_name.as_deref(),
            Some("Example Regular")
        );
        assert_eq!(
            metadata.faces[0].postscript_name.as_deref(),
            Some("Example-Regular")
        );
        assert_eq!(metadata.faces[0].units_per_em, 1000);
        assert_eq!(metadata.faces[0].glyph_count, 7);
        assert_eq!(
            encode_inspection(Path::new("/private/host/path/font.ttf"), &metadata).unwrap(),
            "{\"face_count\":1,\"faces\":[{\"face_index\":0,\"family\":\"Example\",\"full_name\":\"Example Regular\",\"glyph_count\":7,\"postscript_name\":\"Example-Regular\",\"units_per_em\":1000}],\"file_name\":{\"display\":\"font.ttf\",\"native_hex\":\"666f6e742e747466\"}}\n"
        );
    }

    #[test]
    fn collections_report_count_and_stable_face_indexes() {
        let metadata = parse_font(&make_collection()).unwrap();
        assert_eq!(metadata.faces.len(), 2);
        assert_eq!(metadata.faces[0].face_index, 0);
        assert_eq!(metadata.faces[0].family.as_deref(), Some("Alpha"));
        assert_eq!(metadata.faces[1].face_index, 1);
        assert_eq!(metadata.faces[1].family.as_deref(), Some("Beta"));
    }

    #[test]
    fn truncated_inputs_fail_without_panicking() {
        let font = make_sfnt(0, "Example", "Example Regular", "Example-Regular", 1000, 7);
        for length in 0..font.len() {
            assert!(parse_font(&font[..length]).is_err(), "prefix {length}");
        }
        assert!(parse_font(&font).is_ok());
    }

    #[test]
    fn conformance_placeholder_is_clearly_invalid() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../samples/conformance/test-font.bin"
        ));
        assert_eq!(
            parse_font(bytes),
            Err(ParseFailure::Invalid(
                "unsupported or missing SFNT/TTC signature"
            ))
        );
    }

    #[test]
    fn directory_listing_is_sorted_and_ignores_non_fonts() {
        let directory = TempDirectory::new();
        fs::write(
            directory.0.join("b.ttf"),
            make_sfnt(0, "Beta", "Beta Regular", "Beta-Regular", 1000, 2),
        )
        .unwrap();
        fs::write(
            directory.0.join("a.otf"),
            make_sfnt(0, "Alpha", "Alpha Regular", "Alpha-Regular", 1000, 1),
        )
        .unwrap();
        fs::write(directory.0.join("notes.txt"), b"not a font").unwrap();

        let json = list_fonts(&directory.0).unwrap();
        let alpha = json.find("a.otf").unwrap();
        let beta = json.find("b.ttf").unwrap();
        assert!(alpha < beta);
        assert!(!json.contains("notes.txt"));
    }

    #[test]
    fn corrupt_font_extension_is_an_input_error() {
        let directory = TempDirectory::new();
        fs::write(directory.0.join("broken.ttf"), b"not a font").unwrap();
        let error = list_fonts(&directory.0).unwrap_err();
        assert!(matches!(error, FontCommandError::InvalidInput { .. }));
    }

    #[test]
    fn oversized_font_is_rejected_from_metadata_before_signature_read() {
        let directory = TempDirectory::new();
        let path = directory.0.join("oversized.ttf");
        fs::File::create(&path)
            .unwrap()
            .set_len(MAX_FONT_BYTES + 1)
            .unwrap();
        let error = inspect_font(&path).unwrap_err();
        assert!(matches!(error, FontCommandError::ResourceLimit { .. }));
    }

    #[test]
    fn font_inspection_rejects_a_directory() {
        let directory = TempDirectory::new();
        let error = inspect_font(&directory.0).unwrap_err();
        assert!(matches!(error, FontCommandError::InvalidInput { .. }));
        assert!(error.to_string().contains("not a regular file"));
    }

    #[test]
    fn font_snapshot_detects_same_length_timestamp_change() {
        let directory = TempDirectory::new();
        let path = directory.0.join("timestamp.ttf");
        fs::write(&path, b"same length").unwrap();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_000)),
        )
        .unwrap();
        let first = FontFileSnapshot::from_file(&file, &path).unwrap();

        file.set_times(
            fs::FileTimes::new()
                .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(1_000_000_001)),
        )
        .unwrap();
        let second = FontFileSnapshot::from_file(&file, &path).unwrap();

        assert_eq!(first.length, second.length);
        assert_ne!(first, second);
    }

    #[test]
    fn listing_limits_are_consumed_before_full_candidate_reads_and_parses() {
        let directory = TempDirectory::new();
        let standalone = directory.0.join("extra.ttf");
        // A recognized signature followed by a deliberately truncated body.
        // At max_font_files + 1, the file-count error must win before a full
        // read or SFNT parse can observe that truncation.
        fs::write(&standalone, b"\0\x01\0\0").unwrap();
        let mut file_budget = FontListingBudget {
            font_files: MAX_FONT_FILES,
            total_faces: 0,
        };
        let error = inspect_listing_candidate(&directory.0, &standalone, true, &mut file_budget)
            .unwrap_err();
        assert!(matches!(error, FontCommandError::ResourceLimit { .. }));
        assert!(error.to_string().contains("font files"));

        let collection = directory.0.join("faces.ttc");
        let mut header = Vec::from(&b"ttcf"[..]);
        push_u32(&mut header, 0x0001_0000);
        push_u32(&mut header, 2);
        // The offset array is absent. The aggregate-face check must reject the
        // declared two faces before allocating/reading/parsing the full TTC.
        fs::write(&collection, header).unwrap();
        let mut face_budget = FontListingBudget {
            font_files: 0,
            total_faces: MAX_TOTAL_FACES - 1,
        };
        let error = inspect_listing_candidate(&directory.0, &collection, true, &mut face_budget)
            .unwrap_err();
        assert!(matches!(error, FontCommandError::ResourceLimit { .. }));
        assert!(error.to_string().contains("font faces"));
    }

    #[cfg(all(
        unix,
        not(any(
            target_os = "espidf",
            target_os = "horizon",
            target_os = "solaris",
            target_os = "vita",
            target_os = "wasi"
        ))
    ))]
    #[test]
    fn font_inspection_rejects_a_concurrent_writer() {
        use std::fs::OpenOptions;

        let directory = TempDirectory::new();
        let path = directory.0.join("locked.ttf");
        fs::write(
            &path,
            make_sfnt(0, "Example", "Example Regular", "Example-Regular", 1000, 1),
        )
        .unwrap();
        let writer = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        rustix::fs::flock(
            &writer,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .unwrap();

        let error = inspect_font(&path).unwrap_err();
        assert!(matches!(error, FontCommandError::Io { .. }));
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[test]
    fn font_inspection_rejects_a_fifo_without_blocking() {
        use rustix::fs::{Mode, CWD};

        let directory = TempDirectory::new();
        let path = directory.0.join("font.fifo");
        rustix::fs::mkfifoat(CWD, &path, Mode::RUSR | Mode::WUSR).unwrap();

        let error = inspect_font(&path).unwrap_err();
        assert!(matches!(error, FontCommandError::InvalidInput { .. }));
    }

    #[cfg(any(target_os = "android", target_os = "linux"))]
    #[test]
    fn non_utf8_host_paths_do_not_panic() {
        use std::os::unix::ffi::OsStringExt;

        let directory = TempDirectory::new();
        let first_name = OsString::from_vec(b"font-\xff.ttf".to_vec());
        let second_name = OsString::from_vec(b"font-\xfe.ttf".to_vec());
        fs::write(
            directory.0.join(first_name),
            make_sfnt(0, "Example", "Example Regular", "Example-Regular", 1000, 1),
        )
        .unwrap();
        fs::write(
            directory.0.join(second_name),
            make_sfnt(0, "Example", "Example Regular", "Example-Regular", 1000, 1),
        )
        .unwrap();
        let json = list_fonts(&directory.0).unwrap();
        assert!(json.contains("font-"));
        assert!(json.contains(".ttf"));
        assert!(json.contains("666f6e742dff2e747466"));
        assert!(json.contains("666f6e742dfe2e747466"));
        assert!(!json.contains(&*directory.0.to_string_lossy()));
    }
}
