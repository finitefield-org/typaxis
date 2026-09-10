#!/usr/bin/env python3
"""Check extraction of saved VMB Book-2 description and semantic fixtures.

Reads the original exporter package. This intentionally rejects unsupported
fixture kinds; it is not a general full-book text normalizer or PDF/UA gate.
"""
import argparse
import json
import subprocess
from pathlib import Path


def expected_text(package, descriptions=1, math_count=3):
    assert package['contract'] == 'typaxis.contract/1.5'
    buffers = {v['text_id']: v['utf8'].encode() for v in package['text_buffers']}
    terms = []
    math = []

    def inlines(values):
        out = []
        for node in values:
            kind = node['kind']
            if kind == 'text':
                span = node['text_span']
                out.append(buffers[span['text_id']][span['start_byte']:span['end_byte']].decode())
            elif kind in ('strong', 'emphasis', 'link'):
                out.append(inlines(node['children']))
            elif kind == 'math_vector':
                out.append(formula(node))
            elif kind in ('soft_break', 'hard_break'):
                out.append('\n')
            elif kind != 'anchor':
                raise AssertionError(f'unsupported fixture inline: {kind}')
        return ''.join(out)

    def formula(node):
        value = node.get('actual_text') or node['alt']
        assert value and value != '数式'
        math.append(value)
        return value

    def blocks(values):
        out = []
        for node in values:
            kind = node['kind']
            if kind in ('paragraph', 'heading'):
                out.append(inlines(node['children']))
            elif kind == 'description_list':
                assert 'ordered' not in node and 'start' not in node
                for item in node['items']:
                    term = inlines(item['term']['children'])
                    assert term
                    terms.append(term)
                    out.extend((term, blocks(item['blocks'])))
            elif kind == 'semantic_container':
                out.append(blocks(node['blocks']))
            elif kind == 'math_vector_block':
                assert node.get('equation_number') is None
                out.append(formula(node))
            else:
                raise AssertionError(f'unsupported fixture block: {kind}')
        return '\n'.join(out)

    assert not package['document']['footnotes']
    expected = blocks(package['document']['blocks'])
    assert len(terms) == descriptions and len(math) == math_count, 'wrong VMB text fixture'
    return expected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('package', type=Path)
    parser.add_argument('pdf', type=Path)
    parser.add_argument('--pdftotext', required=True)
    parser.add_argument('--descriptions', type=int, default=1)
    parser.add_argument('--math-occurrences', type=int, default=3)
    args = parser.parse_args()
    expected = expected_text(json.loads(args.package.read_text()), args.descriptions, args.math_occurrences)
    run = subprocess.run([args.pdftotext, '-raw', '-enc', 'UTF-8', str(args.pdf), '-'],
                         capture_output=True, text=True, check=True)
    assert not run.stderr, run.stderr
    # Only physical line/page boundaries vary. Preserve every other code point,
    # authored space and semantic replacement, including Japanese punctuation.
    normalize = lambda text: text.replace('\r', '').replace('\n', '').replace('\f', '')
    assert normalize(run.stdout) == normalize(expected), (run.stdout, expected)
    assert '\ufffc' not in run.stdout
    print(f'PASS: exact authored text, {args.descriptions} description terms and {args.math_occurrences} semantic math occurrences in source order')


if __name__ == '__main__':
    main()
