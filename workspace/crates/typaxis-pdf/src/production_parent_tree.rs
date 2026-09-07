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
    fn bytes(&mut self, bytes: &[u8]) -> fmt::Result {
        self.0 = self.0.strip_prefix(bytes).ok_or(fmt::Error)?;
        Ok(())
    }
    fn fixed(&mut self, raw: i64) -> fmt::Result {
        let magnitude = raw.unsigned_abs();
        if raw < 0 {
            self.write_str("-")?;
        }
        write!(self, "{}", magnitude / 65_536)?;
        let mut fraction = (magnitude % 65_536) * 152_587_890_625;
        if fraction != 0 {
            let mut width = 16;
            while fraction % 10 == 0 {
                fraction /= 10;
                width -= 1;
            }
            write!(self, ".{fraction:0width$}")?;
        }
        Ok(())
    }
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

// This writer uses uncompressed, direct-length page streams. Compare their
// framing as well as their bytes; scanning for PDF keywords is not a parser.
fn verify_marked_stream(bytes: &[u8], content: &[u8]) -> Result<(), E> {
    compare_payload(bytes, |sink| {
        write!(sink, "<< /Length {} >>\nstream\n", content.len())?;
        sink.bytes(content)?;
        sink.write_str("\nendstream")
    })
}

// Read the absolute reference to select its source ID, then compare the complete
// entry. Strict ordering plus the exact count proves one-to-one ID coverage.
fn verify_id_tree<'a>(
    bytes: &[u8],
    expected_count: usize,
    source_id: impl Fn(u32) -> Option<&'a str>,
) -> Result<(), E> {
    let mut rest = bytes
        .strip_prefix(b"<< /Names [")
        .ok_or(E::ReceiptMismatch)?;
    let mut previous: Option<&str> = None;
    for _ in 0..expected_count {
        let end = rest
            .iter()
            .position(|&b| b == b'>')
            .ok_or(E::ReceiptMismatch)?;
        let reference = rest
            .get(end + 1..)
            .and_then(|b| b.strip_prefix(b" "))
            .ok_or(E::ReceiptMismatch)?;
        let digits = reference.iter().take_while(|b| b.is_ascii_digit()).count();
        if digits == 0 || digits > 10 || reference[0] == b'0' {
            return Err(E::ReceiptMismatch);
        }
        let mut number = 0u32;
        for &digit in &reference[..digits] {
            number = number
                .checked_mul(10)
                .and_then(|n| n.checked_add(u32::from(digit - b'0')))
                .ok_or(E::ReceiptMismatch)?;
        }
        let id = source_id(number).ok_or(E::ReceiptMismatch)?;
        if previous.is_some_and(|prior| prior.as_bytes() >= id.as_bytes()) {
            return Err(E::ReceiptMismatch);
        }
        let mut sink = ExactBytes(rest);
        sink.text(id).map_err(|_| E::ReceiptMismatch)?;
        write!(&mut sink, " {number} 0 R ").map_err(|_| E::ReceiptMismatch)?;
        rest = sink.0;
        previous = Some(id);
    }
    if rest != b"] >>" {
        return Err(E::ReceiptMismatch);
    }
    Ok(())
}

impl ProductionFootnotePdfAssembly<'_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub(super) fn verify_page_graph(&self) -> Result<(), E> {
        let annotations = self.source.structure_objects().annotations();
        let geometry = annotations
            .marked()
            .structure()
            .display()
            .source()
            .block_layout()
            .page_geometry();
        let reference = |role| self.object_number(role).ok_or(fmt::Error);
        compare_payload(self.object_bytes(2).ok_or(E::ReceiptMismatch)?, |sink| {
            write!(sink, "<< /Type /Pages /Count {} /Kids [", self.page_count())?;
            for page in 0..self.page_count() {
                write!(sink, "{} 0 R ", reference(R::Page(page))?)?;
            }
            sink.write_str("] >>")
        })?;
        let mut annotation_cursor = 0;
        for page in 0..self.page_count() {
            let object = self
                .object_number(R::Page(page))
                .ok_or(E::ReceiptMismatch)?;
            let links = annotations
                .page_annotations(page)
                .ok_or(E::ReceiptMismatch)?;
            if links.start != annotation_cursor
                || links.start > links.end
                || links.end > annotations.bindings().len()
            {
                return Err(E::ReceiptMismatch);
            }
            annotation_cursor = links.end;
            compare_payload(
                self.object_bytes(object).ok_or(E::ReceiptMismatch)?,
                |sink| {
                    sink.write_str("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ")?;
                    sink.fixed(geometry.page_width().get().raw())?;
                    sink.write_str(" ")?;
                    sink.fixed(geometry.page_height().get().raw())?;
                    write!(
                        sink,
                        "] /Resources {} 0 R /Contents {} 0 R /StructParents {page} /Tabs /S",
                        reference(R::PageResources(page))?,
                        reference(R::PageContent(page))?
                    )?;
                    if !links.is_empty() {
                        sink.write_str(" /Annots [")?;
                        for index in links {
                            if annotations.bindings()[index].page_index() != page {
                                return Err(fmt::Error);
                            }
                            write!(
                                sink,
                                "{} 0 R ",
                                reference(R::LinkAnnotation(
                                    u32::try_from(index).map_err(|_| fmt::Error)?
                                ))?
                            )?;
                        }
                        sink.write_str("]")?;
                    }
                    sink.write_str(" >>")
                },
            )?;
        }
        if annotation_cursor != annotations.bindings().len() {
            return Err(E::ReceiptMismatch);
        }
        Ok(())
    }
    pub(super) fn verify_marked_streams(&self) -> Result<(), E> {
        let marked = self.source.structure_objects().annotations().marked();
        if marked.pages().len() != self.page_count() as usize {
            return Err(E::ReceiptMismatch);
        }
        for (index, page) in marked.pages().iter().enumerate() {
            if page.page_index() as usize != index {
                return Err(E::ReceiptMismatch);
            }
            let number = self
                .object_number(R::PageContent(page.page_index()))
                .ok_or(E::ReceiptMismatch)?;
            verify_marked_stream(
                self.object_bytes(number).ok_or(E::ReceiptMismatch)?,
                page.content(),
            )?;
        }
        Ok(())
    }
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
        let nodes = structure.registry().nodes();
        let id_count = nodes
            .iter()
            .filter(|node| node.structure_id().is_some())
            .count();
        match (id_count, self.object_number(R::IdTree)) {
            (0, None) => {}
            (0, Some(_)) | (_, None) => return Err(E::ReceiptMismatch),
            (_, Some(object)) => verify_id_tree(
                self.object_bytes(object).ok_or(E::ReceiptMismatch)?,
                id_count,
                |number| {
                    let observation = self.objects().get(number.checked_sub(1)? as usize)?;
                    if observation.number() != number {
                        return None;
                    }
                    let A::Body(R::StructureNode(id)) = observation.role() else {
                        return None;
                    };
                    let node = nodes.get(id.get() as usize)?;
                    if node.structure_node_id() != id {
                        return None;
                    }
                    node.structure_id()
                },
            )?,
        }
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
    fn production_page_dimensions_match_exact_fixed_point_writer() {
        for raw in [
            i64::MIN,
            i64::MAX,
            -65537,
            -65536,
            -32768,
            -1,
            0,
            1,
            2,
            32768,
            65535,
            65536,
            65537,
        ] {
            let expected = crate::tagged_pdf_v2::pdf_number_v2(raw);
            assert_eq!(
                compare_payload(expected.as_bytes(), |sink| sink.fixed(raw)),
                Ok(())
            );
        }
        for (raw, expected) in [
            (1, "0.0000152587890625"),
            (-32768, "-0.5"),
            (65536, "1"),
            (0, "0"),
        ] {
            assert_eq!(
                compare_payload(expected.as_bytes(), |sink| sink.fixed(raw)),
                Ok(())
            );
        }
    }

    #[test]
    fn production_marked_stream_requires_exact_length_content_and_framing() {
        // Payload contains delimiter-like text and non-UTF8 bytes. Verification
        // must use the retained length, not an endstream/EMC keyword search.
        let content = b"BDC\nendstream\nEMC\n\x00\xff";
        let bytes = b"<< /Length 20 >>\nstream\nBDC\nendstream\nEMC\n\x00\xff\nendstream";
        assert_eq!(content.len(), 20);
        assert_eq!(verify_marked_stream(bytes, content), Ok(()));
        for index in 0..bytes.len() {
            let mut changed = bytes.to_vec();
            changed[index] ^= 1;
            assert_eq!(
                verify_marked_stream(&changed, content),
                Err(E::ReceiptMismatch)
            );
            assert_eq!(
                verify_marked_stream(&bytes[..index], content),
                Err(E::ReceiptMismatch)
            );
        }
        assert_eq!(
            verify_marked_stream(bytes, &content[..19]),
            Err(E::ReceiptMismatch)
        );
        assert_eq!(
            verify_marked_stream(b"<< /Length 0 >>\nstream\n\nendstream", b""),
            Ok(())
        );
        let mut extra = bytes.to_vec();
        extra.push(b' ');
        assert_eq!(
            verify_marked_stream(&extra, content),
            Err(E::ReceiptMismatch)
        );
    }

    #[test]
    fn production_id_tree_requires_exact_sorted_source_coverage() {
        let bytes = b"<< /Names [<FEFF0041> 7 0 R <FEFF65E5D83DDE00> 12 0 R ] >>";
        let resolve = |number| match number {
            7 => Some("A"),
            12 => Some("日😀"),
            _ => None,
        };
        assert_eq!(verify_id_tree(bytes, 2, resolve), Ok(()));
        for count in [0, 1, 3] {
            assert_eq!(
                verify_id_tree(bytes, count, resolve),
                Err(E::ReceiptMismatch)
            );
        }
        for index in 0..bytes.len() {
            let mut changed = bytes.to_vec();
            changed[index] ^= 1;
            assert_eq!(
                verify_id_tree(&changed, 2, resolve),
                Err(E::ReceiptMismatch)
            );
            assert_eq!(
                verify_id_tree(&bytes[..index], 2, resolve),
                Err(E::ReceiptMismatch)
            );
        }
        for invalid in [
            b"<< /Names [<FEFF65E5D83DDE00> 12 0 R <FEFF0041> 7 0 R ] >>".as_slice(),
            b"<< /Names [<FEFF0041> 7 0 R <FEFF0041> 7 0 R ] >>",
            b"<< /Names [<FEFF0041> 4294967296 0 R <FEFF65E5D83DDE00> 12 0 R ] >>",
            b"<< /Names [<FEFF0041> 07 0 R <FEFF65E5D83DDE00> 12 0 R ] >>",
            b"<< /Names [<FEFF0041> 7 0 R <FEFF65E5D83DDE00> 12 0 R ] >> ",
        ] {
            assert_eq!(verify_id_tree(invalid, 2, resolve), Err(E::ReceiptMismatch));
        }
        assert_eq!(verify_id_tree(b"<< /Names [] >>", 0, resolve), Ok(()));
    }

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
