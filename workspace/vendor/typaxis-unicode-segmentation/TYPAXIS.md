Private copy of unicode-segmentation 1.12.0 (Unicode 16.0.0).
Original src files and licenses are preserved. The registry package remains a separate dependency and differential reference.
The fallible forward methods are implemented in src/grapheme_work.rs, a child of grapheme; ordinary methods retain upstream behavior.
Work refusal restores the forward cursor to its entry state while accepted work remains consumed. No heap or retained-output records are introduced.
Only full-string forward Graphemes/GraphemeIndices calls gain work hooks; other segmentation APIs are unchanged and are not claimed to be metered.
