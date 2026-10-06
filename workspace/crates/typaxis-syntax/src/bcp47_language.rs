//! Registry-independent canonicalization with storage bounded by the fixed
//! 255-byte language-tag limit. Successful validation needs no heap allocation.
use super::{is_alnum, is_alpha, is_digit, is_singleton, is_variant, GRANDFATHERED};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Error {
    Invalid,
    TextLimit,
}

pub(super) struct CanonicalLanguage {
    bytes: [u8; 255],
    length: usize,
}
impl CanonicalLanguage {
    fn new() -> Self {
        Self {
            bytes: [0; 255],
            length: 0,
        }
    }
    pub(super) fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.length]).expect("canonical language is ASCII")
    }
    fn push(&mut self, part: &str, case: Case) -> Result<(), Error> {
        let separator = usize::from(self.length != 0);
        let end = self
            .length
            .checked_add(separator)
            .and_then(|n| n.checked_add(part.len()))
            .filter(|n| *n <= self.bytes.len())
            .ok_or(Error::Invalid)?;
        if separator != 0 {
            self.bytes[self.length] = b'-';
            self.length += 1;
        }
        for (index, byte) in part.bytes().enumerate() {
            self.bytes[self.length + index] = match case {
                Case::Lower => byte.to_ascii_lowercase(),
                Case::Upper => byte.to_ascii_uppercase(),
                Case::Title if index == 0 => byte.to_ascii_uppercase(),
                Case::Title => byte.to_ascii_lowercase(),
                Case::Preserve => byte,
            };
        }
        self.length = end;
        Ok(())
    }
}
#[derive(Clone, Copy)]
enum Case {
    Lower,
    Upper,
    Title,
    Preserve,
}

pub(super) fn canonicalize(value: &str, maximum: u64) -> Result<CanonicalLanguage, Error> {
    if value.is_empty()
        || value.len() > 255
        || !value.is_ascii()
        || value
            .bytes()
            .any(|b| !(b.is_ascii_alphanumeric() || b == b'-'))
        || value.starts_with('-')
        || value.ends_with('-')
        || value.contains("--")
    {
        return Err(Error::Invalid);
    }
    // Preserve lexical-error precedence over the configurable buffer limit.
    if value.len() as u64 > maximum {
        return Err(Error::TextLimit);
    }
    let mut output = CanonicalLanguage::new();
    if let Some(canonical) = GRANDFATHERED
        .iter()
        .find(|tag| tag.eq_ignore_ascii_case(value))
    {
        output.push(canonical, Case::Preserve)?;
        return Ok(output);
    }
    // A 255-byte tag has at most 128 nonempty hyphen-separated subtags.
    let mut storage = [""; 128];
    let mut length = 0;
    for part in value.split('-') {
        *storage.get_mut(length).ok_or(Error::Invalid)? = part;
        length += 1;
    }
    let parts = &storage[..length];
    if parts[0].eq_ignore_ascii_case("x") {
        if parts.len() < 2 || parts[1..].iter().any(|p| p.len() > 8 || !is_alnum(p)) {
            return Err(Error::Invalid);
        }
        for part in parts {
            output.push(part, Case::Lower)?;
        }
        return Ok(output);
    }
    let primary = parts[0];
    if !is_alpha(primary) || !(2..=8).contains(&primary.len()) {
        return Err(Error::Invalid);
    }
    output.push(primary, Case::Lower)?;
    let mut index = 1;
    if primary.len() <= 3 {
        let mut extlangs = 0;
        while index < parts.len()
            && extlangs < 3
            && parts[index].len() == 3
            && is_alpha(parts[index])
        {
            output.push(parts[index], Case::Lower)?;
            index += 1;
            extlangs += 1;
        }
    }
    if index < parts.len() && parts[index].len() == 4 && is_alpha(parts[index]) {
        output.push(parts[index], Case::Title)?;
        index += 1;
    }
    if index < parts.len()
        && ((parts[index].len() == 2 && is_alpha(parts[index]))
            || (parts[index].len() == 3 && is_digit(parts[index])))
    {
        output.push(parts[index], Case::Upper)?;
        index += 1;
    }
    let variants_start = index;
    while index < parts.len() && is_variant(parts[index]) {
        if parts[variants_start..index]
            .iter()
            .any(|prior| prior.eq_ignore_ascii_case(parts[index]))
        {
            return Err(Error::Invalid);
        }
        output.push(parts[index], Case::Lower)?;
        index += 1;
    }
    // Slots are in ASCII singleton order, matching the previous stable sort.
    // Ranges borrow the input; neither subtags nor duplicate sets are cloned.
    let mut extensions = [None; 36];
    while index < parts.len()
        && is_singleton(parts[index])
        && !parts[index].eq_ignore_ascii_case("x")
    {
        let byte = parts[index].as_bytes()[0].to_ascii_lowercase();
        let slot = if byte.is_ascii_digit() {
            (byte - b'0') as usize
        } else {
            10 + (byte - b'a') as usize
        };
        if extensions[slot].is_some() {
            return Err(Error::Invalid);
        }
        let start = index;
        index += 1;
        while index < parts.len() && (2..=8).contains(&parts[index].len()) && is_alnum(parts[index])
        {
            index += 1;
        }
        if index == start + 1 {
            return Err(Error::Invalid);
        }
        extensions[slot] = Some((start, index));
    }
    let private_start = if index < parts.len() && parts[index].eq_ignore_ascii_case("x") {
        if index + 1 == parts.len()
            || parts[index + 1..]
                .iter()
                .any(|p| p.len() > 8 || !is_alnum(p))
        {
            return Err(Error::Invalid);
        }
        Some(index)
    } else {
        if index != parts.len() {
            return Err(Error::Invalid);
        }
        None
    };
    for (start, end) in extensions.into_iter().flatten() {
        for part in &parts[start..end] {
            output.push(part, Case::Lower)?;
        }
    }
    if let Some(start) = private_start {
        for part in &parts[start..] {
            output.push(part, Case::Lower)?;
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "bcp47_language_tests.rs"]
mod tests;
