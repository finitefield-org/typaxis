//! The shared authored-order merge has six stack cursors and one result array.
use super::display_projection::Projector;
use super::*;

pub(super) fn merge<I>(
    projector: &mut Projector<'_, '_, '_, '_, '_, '_, '_, '_, '_>,
    algorithm: &str,
    hashes: [[u8; 32]; 8],
    counts: [usize; 6],
    mut next_paint: impl FnMut(usize, usize) -> (I, (usize, u8, u32)),
) -> Result<(Vec<I>, [u8; 32]), BookV2MathDisplayError> {
    let root = NodeId::new(0);
    let mut count = 0usize;
    for n in counts {
        projector.step(root)?;
        count = count
            .checked_add(n)
            .ok_or_else(|| error(root, E::RecordLimit))?;
    }
    take(
        projector.remaining,
        count
            .checked_add(1)
            .ok_or_else(|| error(root, E::RecordLimit))?,
        root,
    )?;
    let mut paints = Vec::new();
    paints
        .try_reserve_exact(count)
        .map_err(|_| error(root, E::AllocationFailure))?;
    projector.step(root)?;
    let mut fingerprint = sha256(algorithm.as_bytes());
    for hash in hashes {
        fingerprint = projector.fold(fingerprint, &hash, root)?;
    }
    fingerprint = projector.fold(fingerprint, &(count as u64).to_be_bytes(), root)?;
    let mut cursors = [0; 6];
    let mut previous = None;
    for _ in 0..count {
        let mut next = None;
        for stream in 0..6 {
            projector.step(root)?;
            if cursors[stream] < counts[stream] {
                let (paint, key) = next_paint(stream, cursors[stream]);
                if next.as_ref().is_none_or(|(_, _, first)| key < *first) {
                    next = Some((stream, paint, key));
                }
            }
        }
        let (stream, paint, key) = next.ok_or_else(|| error(root, E::ReceiptMismatch))?;
        if previous.is_some_and(|p| p > key || (p == key && key.1 == 2)) {
            return Err(error(root, E::ReceiptMismatch).into());
        }
        let mut bytes = [0; 22];
        bytes[0] = stream as u8;
        bytes[1..9].copy_from_slice(&(cursors[stream] as u64).to_be_bytes());
        bytes[9..17].copy_from_slice(&(key.0 as u64).to_be_bytes());
        bytes[17] = key.1;
        bytes[18..22].copy_from_slice(&key.2.to_be_bytes());
        fingerprint = projector.fold(fingerprint, &bytes, root)?;
        paints.push(paint);
        cursors[stream] += 1;
        previous = Some(key);
    }
    if cursors != counts {
        return Err(error(root, E::ReceiptMismatch).into());
    }
    Ok((paints, fingerprint))
}
