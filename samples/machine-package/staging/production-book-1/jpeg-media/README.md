# Private JPEG media fixture

The five `.hex` files are tiny deterministic JFIF baseline JPEG resources used
by the contract-1.4 staging tests. Tests decode the hexadecimal representation
into an isolated temporary resource root; the public/current CLI does not
advertise or dispatch this private profile.

- `color-2x1.jpg.hex`: 2 × 1, three-component YCbCr, 4:4:4 sampling.
- `color-17x9-422.jpg.hex`: 17 × 9, three-component YCbCr, 4:2:2 sampling.
- `color-17x9-440.jpg.hex`: 17 × 9, three-component YCbCr, 4:4:0 sampling.
- `color-17x9-420.jpg.hex`: 17 × 9, three-component YCbCr, 4:2:0 sampling.
- `gray-2x1.jpg.hex`: 2 × 1, one-component grayscale.

All five contain one immediate zero-thumbnail JFIF APP0 segment and no metadata.
The admitted normalized stream is the same JPEG with that APP0 segment
removed. The 4:2:2 resource is also declared but deliberately unused by the
document fixture; its manifest record must retain admission facts while all
PDF plan and object fields remain null.
