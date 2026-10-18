use super::*;
use typaxis_core::Rect;
use typaxis_layout::book_v2::BookV2InlineLineLayout;
use typaxis_layout::ProductionPlacedInline;
use typaxis_pagination::ProductionBodyFlowItem;
use typaxis_pagination::{ProductionTableCellContent, ProductionTableContentSource};

fn hex(bytes: [u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn rect(bounds: Rect) -> [i64; 4] {
    [
        bounds.x().raw(),
        bounds.y().raw(),
        bounds.width().get().raw(),
        bounds.height().get().raw(),
    ]
}
fn source(value: Option<Source>, lines: &BookV2InlineLineLayout<'_, '_>) -> Value {
    match value {
        None => Value::Null,
        Some(Source::ParagraphLine {
            paragraph_index,
            line_index,
        }) => {
            let paragraph = &lines.paragraphs()[paragraph_index as usize];
            let selected = paragraph
                .selected()
                .map(|p| &p.lines()[line_index as usize]);
            json!({"kind":"paragraph","paragraph_index":paragraph_index,"line_index":line_index,
                "units":selected.map(|line|[line.line().start_unit(),line.line().end_unit()]),
                "line_height":selected.map(|line|line.line().metrics().line_height().get().raw()),
                "line_baseline":paragraph.lines()[line_index as usize].baseline().raw(),
                "line_width":selected.map(|line|line.inline_size().get().raw())})
        }
        Some(Source::Figure { figure_index }) => json!({"kind":"figure","index":figure_index}),
        Some(Source::VectorBlock { block_index }) => {
            json!({"kind":"vector_block","index":block_index})
        }
        Some(Source::NativeMathBlock { block_index }) => {
            json!({"kind":"native_math_block","index":block_index})
        }
    }
}
fn items(values: &[ProductionBodyFlowItem], lines: &BookV2InlineLineLayout<'_, '_>) -> Vec<Value> {
    values.iter().enumerate().map(|(index,item)| json!({"index":index,"owner":item.owner().get(),
        "height":item.height().raw(),"before":item.space_before().raw(),"after":item.space_after().raw(),
        "leading":item.leading().raw(),"trailing":item.trailing().raw(),
        "consumed":item.consumed_height().unwrap().raw(),"keep":item.keep_with_next(),
        "x":item.x().raw(),"width":item.width().get().raw(),"source":source(item.source(),lines)})).collect()
}

fn table_content(values: &[ProductionTableCellContent]) -> Vec<Value> {
    values
        .iter()
        .map(|item| {
            let source = match item.source() {
                ProductionTableContentSource::FlowItem(index) => {
                    json!({"kind":"item","index":index})
                }
                ProductionTableContentSource::Table(index) => json!({"kind":"table","index":index}),
            };
            json!({"source":source,"top":item.top().raw(),"end":item.end().raw()})
        })
        .collect()
}

pub(super) fn record(
    case: &str,
    closed: &BookV2ColumnSourceClosure<'_, '_, '_, '_, '_, '_, '_>,
    search: &BookV2ColumnPageSearch<'_, '_, '_, '_, '_>,
    authored: &Value,
    caps: &M4EffectiveResourceLimits,
) {
    let Some(root) = std::env::var_os("TYPAXIS_BOOK_NAVIGATION_GEOMETRY_PROBE") else {
        return;
    };
    let original = search.source_lines();
    let source_frames = original.frames().unwrap();
    let plan = source_frames.column_plan().unwrap();
    let mut names = Vec::new();
    while let Some(name) = plan.name(names.len()) {
        names.push(name);
    }
    let mut tables = Vec::new();
    while let Some(table) = search.source_table(tables.len()) {
        tables.push(json!({"index":tables.len(),"owner":table.owner().get(),"source_index":table.source_index(),
            "height":table.height().raw(),"before":table.space_before().raw(),"after":table.space_after().raw(),
            "keep":table.keep_with_next(),"keep_together":table.keep_together(),
            "caption":table.caption().map(|caption|json!({"height":caption.height().raw(),"content":table_content(caption.content())})),
            "cells":table.cells().iter().enumerate().map(|(index,cell)|json!({"index":index,"owner":cell.owner().get(),
                "source_index":cell.source_index(),"height":cell.natural_height().raw(),"content":table_content(cell.content())})).collect::<Vec<_>>(),
            "rows":table.rows().iter().enumerate().map(|(index,row)|json!({"index":index,"owner":row.owner().get(),
                "section":format!("{:?}",row.section()),"top":row.top().raw(),"height":row.height().raw()})).collect::<Vec<_>>()
        }));
    }
    let state = closed
        .geometry()
        .pages()
        .last()
        .unwrap()
        .selection()
        .next_state()
        .source_state();
    let definitions = (0..authored["document"]["footnotes"].as_array().unwrap().len())
        .map(|index| {
            json!({"index":index,"status":format!("{:?}",state.definition_status(index).unwrap()),
            "items":items(search.source_items(Some(index)).unwrap(),original)})
        })
        .collect::<Vec<_>>();
    let paragraphs = original.paragraphs().iter().enumerate().map(|(index, paragraph)| {
        json!({"index":index,"owner":paragraph.owner().get(),"lines":paragraph.selected().map(|p|p.lines().iter()
            .enumerate().map(|(index,line)|json!({"index":index,"units":[line.line().start_unit(),line.line().end_unit()],
                "height":line.line().metrics().line_height().get().raw(),"width":line.inline_size().get().raw(),
                "baseline":paragraph.lines()[index].baseline().raw()})).collect::<Vec<_>>())})
    }).collect::<Vec<_>>();
    let pages = closed.geometry().pages().iter().enumerate().map(|(page_index, page)| {
        let selection = page.selection();
        let frames = selection.candidate().frames();
        let balance = selection.balance().map(|receipt| json!({
            "extent":receipt.initial_extent().raw(),"full_height":receipt.full_height().get().raw(),
            "initial_height":receipt.initial_height().get().raw(),"selected_height":receipt.selected_height().get().raw(),
            "input_canonical_bytes":receipt.input_canonical_bytes(),"input_fingerprint":hex(receipt.input_fingerprint()),
            "canonical_bytes":receipt.canonical_bytes(),"fingerprint":hex(receipt.fingerprint()),
            "candidates":receipt.candidates().iter().enumerate().map(|(index,candidate)| json!({"index":index+1,
                "target_height":candidate.target().get().raw(),"terminal":candidate.terminal(),
                "rejection_fingerprint":hex(candidate.rejection_fingerprint()),
                "capacity_canonical_bytes":candidate.capacity_canonical_bytes(),"capacity_fingerprint":hex(candidate.capacity_fingerprint()),
                "rejections":candidate.rejections().iter().map(|rejection|json!({"column":rejection.column_index(),
                    "owner":rejection.owner().get(),"source_items":[rejection.source_items().start,rejection.source_items().end],
                    "required_height":rejection.required_height().get().raw(),"deficit":rejection.deficit().get().raw(),
                    "table_group_fingerprint":rejection.table_group_fingerprint().map(hex),
                    "table_group_certificates":rejection.table_group_certificates().collect::<Vec<_>>()
                })).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        }));
        let columns = page.columns().iter().enumerate().map(|(index,column)| {
            let selected = &selection.candidate().columns()[index];
            json!({"index":index,"bounds":rect(column.bounds()),"original_bounds":rect(frames.column(index as u16).unwrap()),
                "used_height":selected.used_height().raw(),"fragment_range":[column.fragments().start,column.fragments().end],
                "parts":selected.parts().iter().map(|part|json!({"top":part.top().raw(),"height":part.height().raw(),
                    "items":part.items().map(|range|[range.start,range.end]),
                    "table":part.table().map(|table|json!({"table_index":table.before().table_index(),
                        "fingerprint":hex(table.fingerprint()),"before_row":table.before().next_row(),"after_row":table.after().next_row(),
                        "before_offset":table.before().offset().raw(),"after_offset":table.after().offset().raw(),
                        "before_initial":table.before().is_initial(),"after_initial":table.after().is_initial(),
                        "before_started_rows":table.before().has_started_rows(),"after_started_rows":table.after().has_started_rows(),
                        "before_caption_item":table.before().next_caption_item(),"after_caption_item":table.after().next_caption_item(),
                        "available_height":table.available_height().raw(),"used_height":table.used_height().raw(),"header_height":table.header_height().raw(),
                        "repeats_header":table.repeats_header(),"header_variant_fingerprint":table.header_variant().map(|header|hex(header.fingerprint())),
                        "cells":table.cells().iter().map(|cell|json!({"owner":cell.owner().get(),"cell_index":cell.cell_index(),
                            "top":cell.top().raw(),"height":cell.height().raw(),"offset_before":cell.offset_before().raw(),"offset_after":cell.offset_after().raw(),
                            "content_range":[cell.content_range().start,cell.content_range().end]})).collect::<Vec<_>>(),
                        "before_progress":hex(table.before().cell_progress_fingerprint()),"after_progress":hex(table.after().cell_progress_fingerprint()),
                        "semantic_ranges":table.semantic_leaf_ranges().map(|range|[range.start,range.end]).collect::<Vec<_>>()
                    }))})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>();
        let fragments = page.fragments_with_roles().enumerate().map(|(index,(placed,cell,repeated))| {
            let fragment = placed.fragment();
            let lines = closed.fragment_lines(page_index,index).unwrap();
            let mut math = Vec::new();
            match fragment.source() {
                Source::ParagraphLine {paragraph_index,line_index} => {
                    for inline in lines.paragraphs()[paragraph_index as usize].lines()[line_index as usize].items() {
                        match inline {
                            ProductionPlacedInline::BookV2Math(selected) => math.push(json!({"owner":selected.owner().get(),"kind":"native",
                                "fingerprint":hex(BookV2BodyMathSource::Native(selected.receipt()).fingerprint())})),
                            ProductionPlacedInline::Vector(selected) => {
                                let item = selected.occurrence().item();
                                let binding = lines.prepared().vector_bindings().unwrap().receipt(item.node_id()).unwrap();
                                if binding.kind() == typaxis_syntax::PrecomposedVectorKind::MathVector {
                                    math.push(json!({"owner":item.node_id().get(),"kind":"vector","fingerprint":hex(binding.binding_fingerprint().bytes())}));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Source::NativeMathBlock { .. } => {
                    let receipt = lines.prepared().native_math().unwrap().receipt(fragment.owner()).unwrap();
                    math.push(json!({"owner":fragment.owner().get(),"kind":"native",
                        "fingerprint":hex(BookV2BodyMathSource::Native(receipt).fingerprint())}));
                }
                Source::VectorBlock { .. } => {
                    let binding = lines.prepared().vector_bindings().unwrap().receipt(fragment.owner()).unwrap();
                    if binding.kind() == typaxis_syntax::PrecomposedVectorKind::MathVectorBlock {
                        math.push(json!({"owner":fragment.owner().get(),"kind":"vector","fingerprint":hex(binding.binding_fingerprint().bytes())}));
                    }
                }
                _ => {}
            }
            json!({"index":index,"column":page.fragment_column(index),"definition":placed.definition_index(),
                "item":placed.item_index(),"owner":fragment.owner().get(),"bounds":rect(fragment.bounds()),
                "baseline":fragment.baseline().map(|value|value.raw()),"viewport":fragment.viewport().map(rect),
                "source":source(Some(fragment.source()),lines),"cell_owner":cell.map(|cell|cell.owner().get()),"repeated":repeated,
                "header":page.header_variant(index).map(|header|json!({"table_index":header.table_index(),"global_item_index":header.global_item_index(),
                    "fingerprint":hex(header.fingerprint()),"lines_fingerprint":hex(header.lines().fingerprint())})),"math":math})
        }).collect::<Vec<_>>();
        json!({"index":selection.page_index(),"name":selection.named_page(),"requires_balance":frames.requires_last_page_balance(),
            "forced_break_owner":selection.forced_break().map(|owner|owner.get()),
            "body_bounds":rect(frames.body()),"declared_footnotes":frames.footnote().map(rect),
            "placed_footnotes":selection.candidate().footnote_bounds().map(rect),"columns":columns,"fragments":fragments,"balance":balance,
            "next_item":selection.next_state().source_state().next_item(),"next_table":selection.next_state().source_state().next_table_index(),
            "pending_definitions":selection.next_state().source_state().pending_definitions()})
    }).collect::<Vec<_>>();
    let test_name = std::thread::current()
        .name()
        .unwrap_or("callback")
        .rsplit("::")
        .next()
        .unwrap()
        .to_owned();
    let case = format!("{case}-{test_name}");
    let value = json!({"schema":"typaxis.book-2-column-balance-probe/1","case":case,"authored_document":authored["document"],
        "authored_page_masters":authored["page_masters"],
        "source_package_fingerprint":hex(original.prepared().source_flow().package_sha256()),
        "source_measurements_fingerprint":hex(closed.geometry().sequence().measurements_fingerprint()),
        "source_body_bounds":rect(source_frames.body()),"source_footnote_bounds":source_frames.footnote_region().map(rect),
        "source_page_names":names,
        "source_line_fingerprint":hex(original.fingerprint()),"source_items":{"body":items(search.source_items(None).unwrap(),original),"definitions":definitions},
        "source_paragraphs":paragraphs,"source_tables":tables,"pages":pages,
        "source_coverage":{"semantic_fragments":closed.semantic_fragments(),"repeated_fragments":closed.repeated_fragments(),
            "header_variant_fragments":closed.header_variant_fragments(),"unreferenced_definitions":closed.unreferenced_definitions(),
            "semantic_math":closed.semantic_math(),"repeated_math":closed.repeated_math()},
        "limits_canonical_jcs":caps.canonical_jcs(),"limits_fingerprint":hex(caps.fingerprint()),
        "caps":{"max_column_balance_candidates":caps.base().get().max_column_balance_candidates,
            "max_fragments":caps.base().get().max_fragments,"max_spool_bytes":caps.base().get().max_spool_bytes},
        "observed":{"scope":"callback_search","balance_candidates":search.balance_candidates(),"work_steps":search.work_steps(),
            "record_charge":search.record_charge(),"spool_charge":search.terminal_spool_charge()}});
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    let directory = std::path::PathBuf::from(root).join("column-balance");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join(format!("{case}-{}.json", hex(typaxis_core::sha256(&bytes)))),
        bytes,
    )
    .unwrap();
}
