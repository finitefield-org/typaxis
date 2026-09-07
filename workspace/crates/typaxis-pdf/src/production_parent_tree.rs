//! Compare actual structure payloads without allocating a second tree.
use super::*;
use std::fmt::{self, Write};

struct ExactBytes<'a>(&'a [u8]);
impl Write for ExactBytes<'_> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.strip_prefix(text.as_bytes()).ok_or(fmt::Error)?;
        Ok(())
    }
}
impl ExactBytes<'_> {
    fn text(&mut self, text: &str) -> fmt::Result {
        self.write_str("<FEFF")?;
        for unit in text.encode_utf16() {
            write!(self, "{unit:04X}")?;
        }
        self.write_str(">")
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
    pub(super) fn verify_structure_nodes(&self) -> Result<(), E> {
        let annotations = self.source.structure_objects().annotations();
        let structure = annotations.marked().structure();
        let reference = |role| self.object_number(role).ok_or(fmt::Error);
        let root = self
            .object_number(R::StructureRoot)
            .ok_or(E::ReceiptMismatch)?;
        compare_payload(self.object_bytes(root).ok_or(E::ReceiptMismatch)?, |sink| {
            sink.write_str("<< /Type /StructTreeRoot /RoleMap << /Em /Span /Exercise /Div /Proof /Div /Result /Div /Strong /Span >> /ParentTree ")?;
            let next_key = self
                .page_count()
                .checked_add(u32::try_from(annotations.bindings().len()).map_err(|_| fmt::Error)?)
                .ok_or(fmt::Error)?;
            write!(
                sink,
                "{} 0 R /ParentTreeNextKey {next_key} /K [",
                reference(R::ParentTree)?
            )?;
            for node in structure
                .registry()
                .nodes()
                .iter()
                .filter(|n| n.parent().is_none())
            {
                write!(
                    sink,
                    "{} 0 R ",
                    reference(R::StructureNode(node.structure_node_id()))?
                )?;
            }
            sink.write_str("]")?;
            if structure
                .registry()
                .nodes()
                .iter()
                .any(|n| n.structure_id().is_some())
            {
                write!(sink, " /IDTree {} 0 R", reference(R::IdTree)?)?;
            }
            sink.write_str(" >>")
        })?;
        for node in structure.registry().nodes() {
            let id = node.structure_node_id();
            let object = self
                .object_number(R::StructureNode(id))
                .ok_or(E::ReceiptMismatch)?;
            compare_payload(
                self.object_bytes(object).ok_or(E::ReceiptMismatch)?,
                |sink| {
                    write!(
                        sink,
                        "<< /Type /StructElem /S /{} /P {} 0 R /Lang ",
                        node.role().pdf_name(),
                        reference(node.parent().map_or(R::StructureRoot, R::StructureNode))?
                    )?;
                    sink.text(node.language())?;
                    for (key, value) in [
                        (" /Alt ", node.alternative()),
                        (" /ID ", node.structure_id()),
                    ] {
                        if let Some(text) = value {
                            sink.write_str(key)?;
                            sink.text(text)?;
                        }
                    }
                    if let Some(numbering) = node.list_numbering() {
                        write!(
                            sink,
                            " /A << /O /List /ListNumbering /{} >>",
                            numbering.pdf_name()
                        )?;
                    } else if let Some(table) = node.table_attributes() {
                        sink.write_str(" /A << /O /Table")?;
                        if node.role() == typaxis_display_list::StructureRole::TableHeader {
                            sink.write_str(" /Scope /Column")?;
                        }
                        if table.rowspan() > 1 {
                            write!(sink, " /RowSpan {}", table.rowspan())?;
                        }
                        if table.colspan() > 1 {
                            write!(sink, " /ColSpan {}", table.colspan())?;
                        }
                        if !table.header_ids().is_empty() {
                            sink.write_str(" /Headers [")?;
                            for header in table.header_ids() {
                                sink.text(header)?;
                                sink.write_str(" ")?;
                            }
                            sink.write_str("]")?;
                        }
                        sink.write_str(" >>")?;
                    }
                    sink.write_str(" /K [")?;
                    for &index in structure.node_groups(id).ok_or(fmt::Error)? {
                        let group = structure.groups().get(index).ok_or(fmt::Error)?;
                        if group.node() != id {
                            return Err(fmt::Error);
                        }
                        write!(
                            sink,
                            "<< /Type /MCR /Pg {} 0 R /MCID {} >> ",
                            reference(R::Page(group.page_index()))?,
                            group.mcid()
                        )?;
                    }
                    for &child in node.children() {
                        write!(sink, "{} 0 R ", reference(R::StructureNode(child))?)?;
                    }
                    for &index in annotations.node_annotations(id).ok_or(fmt::Error)? {
                        let annotation = annotations
                            .bindings()
                            .get(index as usize)
                            .ok_or(fmt::Error)?;
                        if annotation.node() != id {
                            return Err(fmt::Error);
                        }
                        write!(
                            sink,
                            "<< /Type /OBJR /Pg {} 0 R /Obj {} 0 R >> ",
                            reference(R::Page(annotation.page_index()))?,
                            reference(R::LinkAnnotation(index))?
                        )?;
                    }
                    sink.write_str("] >>")
                },
            )?;
        }
        Ok(())
    }
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
    fn production_structure_text_checks_utf16_surrogates_and_literal_delimiters() {
        let text = "日😀<>()\n";
        let expected = b"<FEFF65E5D83DDE00003C003E00280029000A>";
        assert_eq!(compare_payload(expected, |sink| sink.text(text)), Ok(()));
        for index in 0..expected.len() {
            let mut changed = expected.to_vec();
            changed[index] ^= 1;
            assert_eq!(
                compare_payload(&changed, |sink| sink.text(text)),
                Err(E::ReceiptMismatch)
            );
        }
        assert_eq!(compare_payload(b"<FEFF>", |sink| sink.text("")), Ok(()));
    }

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
