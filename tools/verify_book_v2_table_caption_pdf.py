#!/usr/bin/env python3
"""Extract caption/header continuation fixtures with an independent PDF reader.

Run after verify_book_v2_pdf_assembly.py. These small TrueType fixtures use
distinct authored caption (Result) and cell (Proof) text; they are not evidence
of Japanese typography or VMB producer acceptance.
"""
import argparse
import json
from pathlib import Path

from pypdf import PdfReader


EXPECTED = {
    (False, 1): [(1, 4), (0, 4)],
    (False, 5): [(3, 0), (2, 2), (0, 6)],
    (True, 1): [(1, 4), (0, 6), (0, 4)],
    (True, 5): [(3, 0), (2, 0), (0, 6), (0, 6)],
}


def verify(directory):
    drivers = {}
    for path in directory.glob('*.driver'):
        driver = json.loads(path.read_text())
        drivers.setdefault(tuple(driver['display']), []).append(driver['assembly']['pdf'])
    seen = set()
    pdf_count = 0
    page_count = 0
    for path in directory.glob('*.json'):
        probe = json.loads(path.read_text())
        wire = probe['wire']
        if wire['text_buffers'][0]['utf8'] != 'ResultProof':
            continue
        blocks = wire['document']['blocks']
        if len(blocks) != 1 or blocks[0].get('kind') != 'table' or 'caption' not in blocks[0]:
            continue
        table = blocks[0]
        key = (bool(table['head']), len(table['caption']))
        assert key in EXPECTED and key not in seen
        seen.add(key)
        actual = drivers[tuple(probe['display'])]
        assert len(actual) == 1, 'fixture must include one actual driver callback'
        paths = [probe['relations']['navigation']['assembly']['pdf'], *actual]
        for pdf in paths:
            reader = PdfReader(pdf, strict=True)
            text = [page.extract_text() for page in reader.pages]
            counts = [(page.count('Result'), page.count('Proof')) for page in text]
            assert counts == EXPECTED[key], (pdf, counts, EXPECTED[key])
            assert sum(caption for caption, _ in counts) == key[1]
            pdf_count += 1
            page_count += len(reader.pages)
    assert seen == set(EXPECTED), ('missing caption fixtures', set(EXPECTED) - seen)
    return pdf_count, page_count


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    count, pages = verify(parser.parse_args().directory)
    print(f'PASS: {count} PDFs, {pages} pages; exact per-page caption and header/body extraction')
