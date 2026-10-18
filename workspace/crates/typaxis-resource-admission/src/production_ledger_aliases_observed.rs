//! A fallible, in-place alias index; no SVG payload is cloned.
use super::observed_ledger::{point, reserve, work};
use super::*;
use ProductionResourceLedgerStageV3 as Stage;

pub(super) type Alias<'a> = ([u8; 32], &'a [u8], ImageResourceId);

pub(super) fn validate_images<B: ProductionResourceLedgerBudgetV3>(
    images: &crate::image_bindings::Images,
    budget: &mut B,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    if !B::OBSERVED {
        let mut aliases = Vec::new();
        aliases
            .try_reserve_exact(images.len())
            .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
        for image in images.values() {
            if image.admitted_safe_vector().is_some() {
                aliases.push((image.content_hash(), image.bytes()));
            }
        }
        return validate_safe_vector_digest_aliases(&aliases).map_err(Into::into);
    }
    let bytes = images
        .len()
        .checked_mul(std::mem::size_of::<Alias<'_>>())
        .ok_or(ResourceAdmissionError::ResourceLimit)?;
    reserve(
        budget,
        ProductionResourceLedgerStorageV3::Aliases,
        images.len(),
        bytes,
        point(Stage::Aliases, None, None),
    )?;
    let mut aliases = Vec::new();
    aliases
        .try_reserve_exact(images.len())
        .map_err(|_| ResourceAdmissionError::ResourceLimit)?;
    for image in images.values() {
        work(
            budget,
            point(Stage::Aliases, None, Some(image.image_id())),
            64,
        )?;
        if image.admitted_safe_vector().is_some() {
            aliases.push((image.content_hash(), image.bytes(), image.image_id()));
        }
    }
    validate(&mut aliases, budget)
}

fn order<B: ProductionResourceLedgerBudgetV3>(
    a: &Alias<'_>,
    z: &Alias<'_>,
    budget: &mut B,
) -> Result<std::cmp::Ordering, ProductionResourceLedgerErrorV3<B::Error>> {
    work(budget, point(Stage::AliasOrder, None, Some(a.2)), 64)?;
    Ok(a.0.cmp(&z.0).then_with(|| a.2.cmp(&z.2)))
}
fn swap<B: ProductionResourceLedgerBudgetV3>(
    aliases: &mut [Alias<'_>],
    a: usize,
    z: usize,
    budget: &mut B,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    work(
        budget,
        point(Stage::AliasOrder, None, Some(aliases[a].2)),
        128,
    )?;
    aliases.swap(a, z);
    Ok(())
}
fn sift<B: ProductionResourceLedgerBudgetV3>(
    aliases: &mut [Alias<'_>],
    mut root: usize,
    end: usize,
    budget: &mut B,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    loop {
        work(
            budget,
            point(Stage::AliasOrder, None, Some(aliases[root].2)),
            64,
        )?;
        let child = root.checked_mul(2).and_then(|v| v.checked_add(1));
        let Some(mut child) = child.filter(|v| *v < end) else {
            return Ok(());
        };
        if child + 1 < end && order(&aliases[child], &aliases[child + 1], budget)?.is_lt() {
            child += 1;
        }
        if !order(&aliases[root], &aliases[child], budget)?.is_lt() {
            return Ok(());
        }
        swap(aliases, root, child, budget)?;
        root = child;
    }
}
pub(super) fn validate<B: ProductionResourceLedgerBudgetV3>(
    aliases: &mut [Alias<'_>],
    budget: &mut B,
) -> Result<(), ProductionResourceLedgerErrorV3<B::Error>> {
    // Explicit heap operations permit refusal before every comparison and swap.
    // A native sort callback cannot propagate this error without unwinding.
    for root in (0..aliases.len() / 2).rev() {
        sift(aliases, root, aliases.len(), budget)?;
    }
    for end in (1..aliases.len()).rev() {
        swap(aliases, 0, end, budget)?;
        sift(aliases, 0, end, budget)?;
    }
    let mut first = 0;
    for index in 1..aliases.len() {
        let current = &aliases[index];
        work(
            budget,
            point(Stage::AliasCompare, None, Some(current.2)),
            64,
        )?;
        if aliases[first].0 != current.0 {
            first = index;
            continue;
        }
        if aliases[first].1.len() != current.1.len() {
            return Err(conflict());
        }
        for (a, z) in aliases[first].1.chunks(256).zip(current.1.chunks(256)) {
            work(
                budget,
                point(Stage::AliasCompare, None, Some(current.2)),
                (a.len() as u64) * 2 + 64,
            )?;
            if a != z {
                return Err(conflict());
            }
        }
    }
    Ok(())
}
fn conflict<E>() -> ProductionResourceLedgerErrorV3<E> {
    ResourceAdmissionError::InvalidSafeVectorV2(SafeVectorFailureReason::ResourceConflict).into()
}

#[cfg(test)]
#[path = "production_ledger_aliases_tests.rs"]
mod tests;
