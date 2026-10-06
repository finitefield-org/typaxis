//! Dependency-free SHA-256 with bounded storage for admission and canonical sinks.

/// Incremental SHA-256. Complete blocks are borrowed from each update; only an
/// unfinished block is retained. Finishing consumes the state, and no heap
/// allocation or receipt authority is involved.
pub struct Sha256 {
    hash: [u32; 8],
    buffer: [u8; 64],
    used: usize,
    bit_len: u64,
}
impl Sha256 {
    pub const fn new() -> Self {
        Self {
            hash: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            used: 0,
            bit_len: 0,
        }
    }
    pub fn update(&mut self, mut bytes: &[u8]) {
        self.bit_len = self
            .bit_len
            .wrapping_add((bytes.len() as u64).wrapping_mul(8));
        if self.used != 0 {
            let take = bytes.len().min(64 - self.used);
            self.buffer[self.used..self.used + take].copy_from_slice(&bytes[..take]);
            self.used += take;
            bytes = &bytes[take..];
            if self.used < 64 {
                return;
            }
            compress(&mut self.hash, &self.buffer);
            self.used = 0;
        }
        let mut chunks = bytes.chunks_exact(64);
        for chunk in chunks.by_ref() {
            compress(&mut self.hash, chunk);
        }
        let remainder = chunks.remainder();
        self.buffer[..remainder.len()].copy_from_slice(remainder);
        self.used = remainder.len();
    }
    pub fn finish(mut self) -> [u8; 32] {
        let mut tail = [0; 128];
        tail[..self.used].copy_from_slice(&self.buffer[..self.used]);
        tail[self.used] = 0x80;
        let tail_len = if self.used < 56 { 64 } else { 128 };
        tail[tail_len - 8..tail_len].copy_from_slice(&self.bit_len.to_be_bytes());
        for chunk in tail[..tail_len].chunks_exact(64) {
            compress(&mut self.hash, chunk);
        }
        let mut output = [0; 32];
        for (chunk, word) in output.chunks_exact_mut(4).zip(self.hash) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        output
    }
}
impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}
impl std::fmt::Write for Sha256 {
    fn write_str(&mut self, value: &str) -> std::fmt::Result {
        self.update(value.as_bytes());
        Ok(())
    }
}

/// Dependency-free SHA-256 used at admission boundaries.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut state = Sha256::new();
    state.update(bytes);
    state.finish()
}

fn compress(hash: &mut [u32; 8], chunk: &[u8]) {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut words = [0; 64];
    for (index, word) in words[..16].iter_mut().enumerate() {
        let start = index * 4;
        *word = u32::from_be_bytes(chunk[start..start + 4].try_into().unwrap());
    }
    for index in 16..64 {
        let s0 = words[index - 15].rotate_right(7)
            ^ words[index - 15].rotate_right(18)
            ^ (words[index - 15] >> 3);
        let s1 = words[index - 2].rotate_right(17)
            ^ words[index - 2].rotate_right(19)
            ^ (words[index - 2] >> 10);
        words[index] = words[index - 16]
            .wrapping_add(s0)
            .wrapping_add(words[index - 7])
            .wrapping_add(s1);
    }
    let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = *hash;
    for index in 0..64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let choice = (e & f) ^ ((!e) & g);
        let t1 = h
            .wrapping_add(s1)
            .wrapping_add(choice)
            .wrapping_add(K[index])
            .wrapping_add(words[index]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let majority = (a & b) ^ (a & c) ^ (b & c);
        let t2 = s0.wrapping_add(majority);
        h = g;
        g = f;
        f = e;
        e = d.wrapping_add(t1);
        d = c;
        c = b;
        b = a;
        a = t1.wrapping_add(t2);
    }
    for (slot, value) in hash.iter_mut().zip([a, b, c, d, e, f, g, h]) {
        *slot = slot.wrapping_add(value);
    }
}
