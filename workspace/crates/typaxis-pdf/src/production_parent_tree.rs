//! Compare the actual ParentTree payload without allocating a second tree.
use super::*;
use std::fmt::{self, Write};

struct ExactBytes<'a>(&'a [u8]);
impl Write for ExactBytes<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.strip_prefix(text.as_bytes()).ok_or(fmt::Error)?;
        Ok(())
    }
}
fn compare_payload(
    bytes: &[u8],
    emit: impl FnOnce(&mut ExactBytes<'_>) -> fmt::Result,
) -> Result<(), E> {
    let mut sink = ExactBytes(bytes);
    emit(&mut sink).map_err(|_| E::ReceiptMismatch)?;
    if sink.0.is_empty() {
        Ok(())
    } else {
        Err(E::ReceiptMismatch)
    }
}

impl ProductionFootnotePdfAssembly<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(super) fn verify_parent_tree(&self) -> Result<(), E> {
        let annotations = self.source.structure_objects().annotations();
        let structure = annotations.marked().structure();
        let object = self
            .object_number(R::ParentTree)
            .ok_or(E::ReceiptMismatch)?;
        let bytes = self.object_bytes(object).ok_or(E::ReceiptMismatch)?;
        compare_payload(bytes, |sink| {
            sink.write_str("<< /Nums [")?;
            for page in 0..self.page_count() {
                write!(sink, "{page} [")?;
                for (mcid, group) in structure
                    .page_groups(page)
                    .ok_or(fmt::Error)?
                    .iter()
                    .enumerate()
                {
                    if group.page_index() != page
                        || usize::try_from(group.mcid()).ok() != Some(mcid)
                    {
                        return Err(fmt::Error);
                    }
                    let node = self
                        .object_number(R::StructureNode(group.node()))
                        .ok_or(fmt::Error)?;
                    write!(sink, "{node} 0 R ")?;
                }
                sink.write_str("] ")?;
            }
            for (index, annotation) in annotations.bindings().iter().enumerate() {
                let key = self
                    .page_count()
                    .checked_add(u32::try_from(index).map_err(|_| fmt::Error)?)
                    .ok_or(fmt::Error)?;
                if annotation.parent_key() != key || annotation.page_index() >= self.page_count() {
                    return Err(fmt::Error);
                }
                let node = self
                    .object_number(R::StructureNode(annotation.node()))
                    .ok_or(fmt::Error)?;
                write!(sink, "{key} {node} 0 R ")?;
            }
            sink.write_str("] >>")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_parent_tree_comparison_rejects_every_changed_byte_and_truncation() {
        // Empty page, two MCIDs sharing one structure owner, then an annotation.
        let expected = b"<< /Nums [0 [] 1 [12 0 R 12 0 R ] 2 14 0 R ] >>";
        let check = |bytes: &[u8]| {
            compare_payload(bytes, |sink| {
                sink.write_str("<< /Nums [")?;
                write!(sink, "{} [] {} [", 0, 1)?;
                for node in [12, 12] {
                    write!(sink, "{node} 0 R ")?;
                }
                write!(sink, "] {} {} 0 R ] >>", 2, 14)
            })
        };
        assert_eq!(check(expected), Ok(()));
        for index in 0..expected.len() {
            let mut changed = expected.to_vec();
            changed[index] ^= 1;
            assert_eq!(check(&changed), Err(E::ReceiptMismatch));
            assert_eq!(check(&expected[..index]), Err(E::ReceiptMismatch));
        }
        let mut trailing = expected.to_vec();
        trailing.push(b' ');
        assert_eq!(check(&trailing), Err(E::ReceiptMismatch));
    }
}
