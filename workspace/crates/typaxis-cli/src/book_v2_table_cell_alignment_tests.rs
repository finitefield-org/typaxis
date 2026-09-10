use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

fn renumber(v: &mut Value, next: &mut u32) {
    if let Some(a) = v.as_array_mut() {
        for c in a {
            renumber(c, next)
        }
    } else if let Some(o) = v.as_object_mut() {
        if let Some(id) = o.get_mut("node_id") {
            *id = (*next).into();
            *next += 1;
        }
        for key in ["blocks", "children", "caption", "head", "body", "cells"] {
            if let Some(c) = o.get_mut(key) {
                renumber(c, next)
            }
        }
    }
}

#[test]
fn book_v2_table_cell_alignment_reaches_measured_columns_and_repeated_headers_pdf() {
    for aligned in [false, true] {
        let root = Root::new();
        let limits = limits();
        let mut data = source_data("Result");
        let para = data["document"]["blocks"][0]["blocks"][0].clone();
        let span = para["span"].clone();
        let cells:Vec<Value>=["start","end","center"].iter().map(|align|json!({
            "node_id":0,"span":span,"colspan":1,"rowspan":1,"classes":if aligned {vec![*align]}else{vec!["start"]},"blocks":[para]
        })).collect();
        let row = json!({"node_id":0,"span":span,"cells":cells});
        data["document"]["blocks"] = json!([{"kind":"table","node_id":0,"span":span,"classes":[],
            "columns":[{"kind":"fixed","width":60*65536},{"kind":"fraction","weight":1},{"kind":"fraction","weight":2}],
            "caption":[para],"head":[row],"body":vec![row.clone();4]}]);
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        for align in ["start", "end", "center"] {
            rules.push(json!({"style_id":format!("cell-{align}"),"selector":format!("table_cell.{align}"),"source_order":rules.len(),"extends":null,
                "declarations":[{"name":"text_align","important":false,"value":{"kind":"keyword","value":align}}]}));
        }
        let master = &mut data["page_masters"]["masters"][0];
        master["width"] = (300 * 65536).into();
        master["height"] = (150 * 65536).into();
        master["trim"] = json!({"x":0,"y":0,"width":300*65536,"height":150*65536});
        master["body"] = json!({"x":10*65536,"y":10*65536,"width":240*65536,"height":64*65536});
        renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"Result", &limits);
        crate::book_v2_resources::with_converged_book_v2_pdf(
            &input,
            &limits,
            JapaneseLineBreakMode::Normal,
            100_000_000,
            |pdf, observation| {
                let registry = pdf.navigation().source().source();
                assert_eq!(registry.source().pages().len(), 2);
                assert_eq!(
                    registry
                        .nodes()
                        .iter()
                        .filter(|n| n.source().pdf_role() == "TH")
                        .count(),
                    3
                );
                assert_eq!(
                    registry
                        .nodes()
                        .iter()
                        .filter(|n| n.source().pdf_role() == "TD")
                        .count(),
                    12
                );
                crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
            },
        )
        .unwrap();
    }
}
