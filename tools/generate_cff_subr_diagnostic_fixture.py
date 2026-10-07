"""Generate a synthetic /1 subroutine fixture from the existing Typaxis font.

Run with Python and FontTools (verified with 4.51.0). Only the synthetic fixture
is rewritten below cff-media/diagnostics; no publication input is modified.
"""
import hashlib
from io import BytesIO
from pathlib import Path

from fontTools.cffLib import SubrsIndex
from fontTools.misc.psCharStrings import T2CharString
from fontTools.ttLib import TTFont


def main():
    root = Path(__file__).resolve().parents[1] / 'samples/machine-package/staging/production-book-1/cff-media'
    source = bytes.fromhex((root / 'typaxis-cff-fixture.otf.hex').read_text())
    assert hashlib.sha256(source).hexdigest() == '16da5a41c10414b308aa62db39066e0540e460f3e2d0f9c2811f0b32907d94f0'
    with TTFont(BytesIO(source), recalcTimestamp=False) as font:
        cff = font['CFF '].cff
        top = cff.topDictIndex[0]
        private = top.Private
        # The legacy profile requires an explicit CharstringType at its default.
        top.CharstringType = 2
        top.defaults = dict(top.defaults)
        top.defaults.pop('CharstringType', None)
        private.Subrs = SubrsIndex()
        private.Subrs.append(T2CharString(program=[5, 0, 'rlineto', 'return'],
                                         private=private, globalSubrs=cff.GlobalSubrs))
        cff.GlobalSubrs.append(T2CharString(program=[-107, 'callsubr', 'return'],
                                          private=private, globalSubrs=cff.GlobalSubrs))
        top.CharStrings['A'] = T2CharString(program=[600, 50, 'hmoveto', -107, 'callgsubr', 'endchar'],
                                          private=private, globalSubrs=cff.GlobalSubrs)
        buffer = BytesIO()
        font.save(buffer)
    data = buffer.getvalue()
    encoded = data.hex()
    diagnostics = root / 'diagnostics'
    diagnostics.mkdir(exist_ok=True)
    (diagnostics / 'typaxis-cff-subr-diagnostic-fixture.otf.hex').write_text(
        '\n'.join(encoded[i:i + 96] for i in range(0, len(encoded), 96)) + '\n')
    print(len(data), hashlib.sha256(data).hexdigest())


if __name__ == '__main__':
    main()
