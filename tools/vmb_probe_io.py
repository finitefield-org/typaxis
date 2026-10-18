"""Strict input reads shared by the diagnostic VMB scale harness and verifier."""
import json
import math
from pathlib import Path, PurePosixPath


def strict_json(raw):
    def members(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError(f'duplicate JSON member: {key}')
            result[key] = value
        return result

    def number(value):
        result = float(value)
        if not math.isfinite(result):
            raise ValueError('nonfinite JSON number')
        return result

    def constant(value):
        raise ValueError(f'nonfinite JSON constant: {value}')

    result = json.loads(raw.decode('utf-8'), object_pairs_hook=members,
                        parse_float=number, parse_constant=constant)
    pending = [result]
    while pending:
        value = pending.pop()
        if isinstance(value, str):
            if any(0xD800 <= ord(c) <= 0xDFFF for c in value):
                raise ValueError('unpaired JSON surrogate')
        elif isinstance(value, dict):
            pending.extend(value)
            pending.extend(value.values())
        elif isinstance(value, list):
            pending.extend(value)
    return result


def contained_bytes(root, relative):
    if not isinstance(relative, str) or not relative or '\\' in relative or '\0' in relative:
        raise ValueError('invalid relative input path')
    parts = relative.split('/')
    if PurePosixPath(relative).is_absolute() or any(p in ('', '.', '..') for p in parts):
        raise ValueError('input path must be contained and canonical')
    base = Path(root).resolve(strict=True)
    path = base
    for part in parts:
        path = path / part
        if path.is_symlink():
            raise ValueError('symlink input is not permitted')
    resolved = path.resolve(strict=True)
    if not resolved.is_relative_to(base) or not resolved.is_file():
        raise ValueError('input must be a contained regular file')
    return resolved.read_bytes()


def same_json(left, right):
    """Compare source values without treating JSON booleans as integer IDs."""
    if type(left) is not type(right):
        return False
    if isinstance(left, dict):
        return left.keys() == right.keys() and all(same_json(left[k], right[k]) for k in left)
    if isinstance(left, list):
        return len(left) == len(right) and all(same_json(a, b) for a, b in zip(left, right))
    return left == right


def extracted_occurrences(raw, expected, extractor):
    """Preserve source speech; normalize extractor-owned paragraph boundaries.

    This applies to the numeric-template scale corpus, whose semantic strings
    have no line/page control characters. The existing differential verifier
    maps line/page/tab separators to one space. Apply that rule only between
    these separate source paragraphs, without rewriting their semantic text.
    This is not a mixed-body text normalizer.
    """
    text = raw.decode('utf-8').replace('\r\n', '\n').replace('\r', '\n')
    if not expected:
        raise ValueError('missing expected extraction occurrences')
    offset = 0
    separators = '\t\n\f'
    while offset < len(text) and text[offset] in separators:
        offset += 1
    for index, want in enumerate(expected):
        if not isinstance(want, str) or not want or any(c in want for c in '\t\r\n\f'):
            raise ValueError('scale speech must be nonempty without line controls')
        if not text.startswith(want, offset):
            raise ValueError(f'{extractor}: occurrence {index} text or source spaces differ')
        offset += len(want)
        if index + 1 < len(expected):
            if offset < len(text) and text[offset] == ' ':
                offset += 1
            elif offset >= len(text) or text[offset] not in separators:
                raise ValueError(f'{extractor}: missing source paragraph boundary after {index}')
        while offset < len(text) and text[offset] in separators:
            offset += 1
    if offset != len(text):
        raise ValueError(f'{extractor}: extra extracted text or spaces')
    return len(expected)
