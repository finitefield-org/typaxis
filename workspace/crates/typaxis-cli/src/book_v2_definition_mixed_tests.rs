use super::*;
use typaxis_linebreak::JapaneseLineBreakMode;

#[test]
fn book_v2_definition_mixed_candidates_preserve_serial_and_parallel_sources() {
    use typaxis_core::{Length, PositiveLength, Rect};
    use typaxis_pagination::book_v2::{
        prepare_book_v2_body_flow, prepare_book_v2_definition_mixed_search,
        prepare_book_v2_mixed_footnote_demand_search, prepare_book_v2_table_body_search,
        prepare_book_v2_table_measurements, BookV2DefinitionCandidatePart as Request,
    };
    use typaxis_pagination::{
        ProductionBodyPaginationErrorKind as E, ProductionFootnoteDemandStatus as Status,
    };
    for mode in [
        "neighbors",
        "prefix-keep",
        "table-keep",
        "empty-table",
        "empty-table-sequential",
        "terminal-self",
        "overflow",
        "ordinary",
        "forced",
        "nested-forced",
        "cross-cell-keep",
        "keep-conflict",
        "leading-empty",
        "lookback",
        "headers",
        "caption",
        "deep",
        "span",
        "header-span",
        "table-early",
        "table-late",
        "repeat-demand",
    ] {
        let root = Root::new();
        let limits = if mode == "lookback" {
            M4EffectiveResourceLimits::new(
                ValidatedResourceLimits::new(ResourceLimits {
                    max_page_break_lookback: 1,
                    ..ResourceLimits::default()
                })
                .unwrap(),
                M4ResourceLimits::default(),
            )
            .unwrap()
        } else {
            limits()
        };
        let mut data = if mode == "span" || mode == "header-span" {
            super::nested_spans::spanning(
                "LeftRight",
                4,
                if mode == "span" {
                    "natural"
                } else {
                    "header-caption"
                },
            )
        } else {
            super::nested_tables::nested(
                "LeftRight",
                4,
                match mode {
                    "nested-forced" => "forced",
                    "repeat-demand" => "headers",
                    "headers" | "caption" | "deep" => mode,
                    _ => "natural",
                },
            )
        };
        let mut table = data["document"]["blocks"][0].clone();
        let mut paragraph = super::nested_tables::nested("LeftRight", 4, "natural")["document"]
            ["blocks"][0]["body"][0]["cells"][1]["blocks"][0]
            .clone();
        paragraph["classes"] = json!(["neighbor"]);
        let br = super::table_cell_breaks::cells("LeftRight", 4, false, false, false)["document"]
            ["blocks"][0]["body"][0]["cells"][0]["blocks"][1]
            .clone();
        if !matches!(
            mode,
            "nested-forced"
                | "overflow"
                | "headers"
                | "caption"
                | "deep"
                | "span"
                | "header-span"
                | "table-early"
                | "table-late"
                | "repeat-demand"
        ) {
            let child = &mut table["body"][0]["cells"][0]["blocks"][0];
            for cell in child["body"][0]["cells"].as_array_mut().unwrap() {
                cell["blocks"].as_array_mut().unwrap().truncate(1);
                cell["blocks"][0]["classes"] = json!([]);
            }
        }
        if mode == "cross-cell-keep" {
            table["body"][0]["cells"][0]["blocks"][0]["body"][0]["cells"][1]["blocks"][0]
                ["classes"] = json!(["keep"]);
            table["body"][0]["cells"][1]["blocks"]
                .as_array_mut()
                .unwrap()
                .insert(0, br.clone());
        }
        if matches!(mode, "table-early" | "table-late" | "repeat-demand") {
            let child = &mut table["body"][0]["cells"][0]["blocks"][0];
            let target = if mode == "repeat-demand" {
                &mut child["head"][0]["cells"][0]["blocks"][0]
            } else {
                &mut child["body"][0]["cells"][0]["blocks"]
                    [if mode == "table-late" { 3 } else { 0 }]
            };
            target["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"earlier"}));
        }
        let mut prefix = paragraph.clone();
        if mode == "prefix-keep" || mode == "keep-conflict" {
            prefix["classes"] = json!(["keep", "neighbor"]);
        }
        if mode == "table-keep" {
            table["classes"] = json!(["keep"]);
        }
        let mut empty = table.clone();
        empty["classes"] = json!([]);
        for cell in empty["body"][0]["cells"].as_array_mut().unwrap() {
            cell["blocks"] = json!([]);
        }
        // The first definition has a real table too, so selected root indexes
        // are neither definition-local nor equal to the first source table.
        let mut body = paragraph.clone();
        body["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":"target"}));
        let mut suffix = paragraph.clone();
        suffix["children"].as_array_mut().unwrap().push(json!({"kind":"footnote_reference","node_id":0,"span":paragraph["span"],"footnote_id":if mode=="terminal-self"{"target"}else{"earlier"}}));
        let blocks = match mode {
            "ordinary" => json!([prefix, suffix]),
            "leading-empty" => json!([empty.clone(), empty.clone(), prefix, table, suffix]),
            "empty-table" | "empty-table-sequential" => {
                json!([prefix, table, empty.clone(), suffix, empty])
            }
            "forced" => json!([
                br.clone(),
                prefix,
                br.clone(),
                br.clone(),
                table,
                suffix,
                br
            ]),
            "keep-conflict" => json!([prefix, br, table, suffix]),
            _ => json!([prefix, table, suffix]),
        };
        data["document"]["blocks"] = json!([body, paragraph.clone()]);
        data["document"]["footnotes"] = json!([
            {"node_id":0,"span":paragraph["span"],"footnote_id":"earlier","blocks":[empty.clone(),paragraph.clone()]},
            {"node_id":0,"span":paragraph["span"],"footnote_id":"target","blocks":blocks},
            {"node_id":0,"span":paragraph["span"],"footnote_id":"later","blocks":[empty,paragraph.clone()]}
        ]);
        let rules = data["style_sheet"]["rules"].as_array_mut().unwrap();
        for selector in ["paragraph.keep", "table.keep"] {
            rules.push(json!({"style_id":format!("definition-keep-{}",rules.len()),"selector":selector,"source_order":rules.len(),"extends":null,"declarations":[{"name":"keep_with_next","important":false,"value":{"kind":"boolean","value":true}}]}));
        }
        rules.push(json!({"style_id":"definition-spacing","selector":"paragraph.neighbor","source_order":rules.len(),"extends":null,"declarations":[
            {"name":"space_before","important":false,"value":{"kind":"length","value":3*65536}},
            {"name":"space_after","important":false,"value":{"kind":"length","value":5*65536}}]}));
        let width = data["page_masters"]["masters"][0]["body"]["width"]
            .as_i64()
            .unwrap();
        let height = data["page_masters"]["masters"][0]["body"]["height"]
            .as_i64()
            .unwrap();
        data["page_masters"]["masters"][0]["width"] = (width + 20 * 65536).into();
        data["page_masters"]["masters"][0]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["trim"]["height"] = (320 * 65536).into();
        data["page_masters"]["masters"][0]["footnote"] =
            json!({"x":10*65536,"y":150*65536,"width":width,"height":128*65536});
        super::table_caption_breaks::renumber(&mut data["document"], &mut 0);
        let input = prepared(&root, data, b"LeftRight", &limits);
        let nav = prepare_book_v2_navigation(input.body().styled()).unwrap();
        let flow = prepare_book_v2_text_flow(input.body().styled(), &nav).unwrap();
        let policy = prepare_book_v2_resource_policy(input.body(), &limits).unwrap();
        let bindings =
            typaxis_layout::book_v2::bind_book_v2_vectors(&policy, input.resources(), &limits)
                .unwrap();
        let raw = |v| Length::from_raw(v).unwrap();
        let rect = Rect::new(
            raw(10 * 65536),
            raw(10 * 65536),
            PositiveLength::new(raw(width)).unwrap(),
            PositiveLength::new(raw(height)).unwrap(),
        );
        typaxis_layout::book_v2::with_converged_book_v2_body_lines(&policy,&flow,input.resources(),&bindings,&limits,JapaneseLineBreakMode::Normal,rect,1_000_000,|stable| {
            let measured = prepare_book_v2_table_measurements(prepare_book_v2_body_flow(stable.lines(),None,stable.footnotes(),&limits,0).unwrap(),&limits).unwrap();
            if mode == "keep-conflict" {
                assert!(matches!(prepare_book_v2_definition_mixed_search(&measured,1,&limits,1_000_000,0),Err(e) if e.kind==E::KeepAcrossForcedBreak));
                assert!(matches!(prepare_book_v2_mixed_footnote_demand_search(&measured,&limits,1_000_000,0),Err(e) if e.kind==E::KeepAcrossForcedBreak));
                return;
            }
            if mode=="lookback" {
                let mut search=prepare_book_v2_definition_mixed_search(&measured,1,&limits,1_000_000,0).unwrap();
                let state=search.begin(0..measured.flow().body_items().len()).unwrap();
                assert!(search.evaluate(&state,&[Request::Items{end:1}],search.maximum_height()).unwrap().is_some());
                assert!(matches!(search.enumerate(&state,search.maximum_height()),Err(e) if e.kind==E::PageBreakLookbackLimit{limit:1,observed:2}));
                assert_eq!(state.demand().status(0),Some(Status::Unreferenced));
                let mut queue=prepare_book_v2_mixed_footnote_demand_search(&measured,&limits,1_000_000,0).unwrap();
                let initial=queue.begin().unwrap();let pending=queue.require_body(&initial,0..measured.flow().body_items().len()).unwrap();
                let before=queue.work_steps();
                assert!(matches!(queue.enumerate_definition(&pending,1,raw(128*65536)),Err(e) if e.kind==E::PageBreakLookbackLimit{limit:1,observed:2}));
                assert!(queue.work_steps()>before);
                let capacity=measured.flow().definition_items(1).unwrap()[0].consumed_height().unwrap();
                let smaller=queue.select_definition(&pending,1,capacity).unwrap().unwrap();
                assert_eq!(smaller.next_state().next_item(),1);
                assert_eq!(pending.definition_cursor(1).unwrap().next_item(),0);
                return;
            }
            let items = measured.flow().definition_items(1).unwrap();
            let roots:Vec<_> = (0..measured.flow().table_count()).filter(|&index| measured.flow().table_source_definition(index)==Some(Some(1)) && measured.flow().table_parent(index)==Some(None)).collect();
            let replay = |prior,work| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                let mut search=prepare_book_v2_definition_mixed_search(&measured,1,&limits,work,prior)?;
                let mut state=search.begin(0..measured.flow().body_items().len())?;
                let mut seen=Vec::new(); let mut hashes=Vec::new(); let mut fragments=0; let mut forced=0;
                if !matches!(mode,"forced"|"nested-forced"|"cross-cell-keep"|"empty-table-sequential"|"overflow"|"headers"|"caption"|"deep"|"span"|"header-span"|"table-early"|"table-late"|"repeat-demand") {
                    let mut requests=Vec::new(); let mut item=0;
                    for &index in &roots {
                        let range=search.table_range(index).unwrap();
                        if item<range.start { requests.push(Request::Items{end:range.start}); }
                        let cursor=search.begin_table(index)?;
                        requests.push(Request::Table{cursor,capacity:raw(if range.is_empty(){0}else{64*65536})});
                        item=range.end;
                    }
                    if item<items.len(){requests.push(Request::Items{end:items.len()});}
                    let selected=search.evaluate(&state,&requests,search.maximum_height())?.unwrap();
                    selected.verify(&state)?;
                    assert_eq!(state.demand().status(0),Some(Status::Unreferenced));
                    assert!(selected.next_state().is_complete(),"{mode}");
                    let mut end=Length::ZERO; let mut after=Length::ZERO;
                    for part in selected.parts() {
                        let before=if let Some(range)=part.items(){items[range.start].space_before()}else{measured.tables()[part.table().unwrap().before().table_index()].space_before()};
                        let gap=if fragments==0{Length::ZERO}else{after.checked_add(before).unwrap()};
                        assert_eq!(part.top(),end.checked_add(gap).unwrap(),"{mode}");
                        end=part.top().checked_add(part.height()).unwrap();
                        if let Some(range)=part.items(){
                            let mut expected=Length::ZERO;
                            for index in range.clone(){
                                if index>range.start { expected=expected.checked_add(items[index-1].space_after()).unwrap().checked_add(items[index].space_before()).unwrap(); }
                                expected=expected.checked_add(items[index].consumed_height()?).unwrap();
                            }
                            assert_eq!(part.height(),expected,"{mode}");
                            after=items[range.end-1].space_after(); seen.extend(range);
                        }
                        if let Some(table)=part.table(){ assert_eq!(table.used_height(),measured.tables()[table.before().table_index()].height(),"{mode}"); after=measured.tables()[table.before().table_index()].space_after(); seen.extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten()); hashes.push(table.fingerprint()); }
                        fragments+=1;
                    }
                    assert_eq!(selected.used_height(),end);
                    state=selected.into_next_state();
                } else {
                    while !state.is_complete() {
                        assert!(fragments<20,"{mode}");
                        let capacity=if matches!(mode,"overflow"|"table-early"|"table-late"){raw(32*65536)}else if matches!(mode,"headers"|"header-span"|"repeat-demand"){raw(64*65536)}else{search.maximum_height()};
                        let request=if let Some(cursor)=state.table_continuation(){Request::Table{cursor,capacity}}
                        else if search.table_range(state.next_table_index()).is_some_and(|range|range.start==state.next_item()) { Request::Table{cursor:search.begin_table(state.next_table_index())?,capacity} }
                        else if items[state.next_item()].source().is_none(){Request::Forced}
                        else { Request::Items{end:state.next_item()+1} };
                        let selected=search.evaluate(&state,&[request],search.maximum_height())?.unwrap();
                        selected.verify(&state)?;
                        forced+=usize::from(selected.forced_break_owner().is_some());
                        for part in selected.parts(){
                            assert_eq!(part.top(),Length::ZERO);
                            if let Some(range)=part.items(){seen.extend(range);}
                            if let Some(table)=part.table(){seen.extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());hashes.push(table.fingerprint());}
                        }
                        state=selected.into_next_state(); fragments+=1;
                    }
                    if matches!(mode,"forced"|"nested-forced"|"cross-cell-keep"){assert!(forced>0,"{mode}");}
                }
                seen.sort_unstable(); assert_eq!(seen,(0..items.len()).collect::<Vec<_>>(),"{mode}");
                assert_eq!(state.demand().pending_definitions(),if mode=="terminal-self"{&[][..]}else{&[0][..]},"{mode}");
                assert_eq!(state.demand().status(2),Some(Status::Unreferenced));
                Ok((search.record_charge(),search.work_charge(),hashes,fragments,forced))
            };
            let (records,work,hashes,fragments,forced)=replay(0,1_000_000).unwrap();
            let prior=limits.base().get().max_fragments-(records-measured.record_charge());
            assert_eq!(replay(prior,work).unwrap(),(limits.base().get().max_fragments,work,hashes,fragments,forced));
            assert_eq!(replay(prior+1,work).unwrap_err().kind,E::FragmentLimit);
            assert!(matches!(replay(prior,work-1).unwrap_err().kind,E::TableSearchLimit|E::FootnoteSearchLimit));
            let automatic = |prior,work| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                let mut search=prepare_book_v2_definition_mixed_search(&measured,1,&limits,work,prior)?;
                let mut state=search.begin(0..measured.flow().body_items().len())?;
                let mut seen=Vec::new();let mut outcomes=Vec::new();let mut forced=Vec::new();
                while !state.is_complete(){
                    assert!(outcomes.len()<20,"automatic {mode}");
                    let capacity=if matches!(mode,"overflow"|"table-early"|"table-late"){raw(32*65536)}else if matches!(mode,"headers"|"header-span"|"repeat-demand"){raw(64*65536)}else{search.maximum_height()};
                    let before_status=state.demand().status(0);
                    let options=search.enumerate(&state,capacity)?;
                    options.verify(&state)?;
                    assert_eq!(options.available_height(),capacity);
                    assert!(!options.choices().is_empty(),"automatic {mode}");
                    assert!(options.examined_boundaries()>=options.choices().len() as u32);
                    for pair in options.choices().windows(2){assert!(pair[0].cost()<=pair[1].cost());}
                    for choice in options.choices(){
                        let candidate=choice.candidate();
                        candidate.verify(&state)?;assert!(candidate.used_height()<=capacity);
                        let mut leaves=Vec::new();
                        for part in candidate.parts(){
                            if let Some(range)=part.items(){leaves.extend(range);}
                            if let Some(table)=part.table(){leaves.extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());}
                        }
                        let retained=measured.flow().references_in_items(Some(1),0..items.len()).iter().any(|reference|reference.source().definition_index()==0 && (reference.first_item_index()..=reference.last_item_index()).all(|index|leaves.contains(&index)));
                        let expected=if before_status==Some(Status::Pending)||retained{Status::Pending}else{Status::Unreferenced};
                        assert_eq!(candidate.next_state().demand().status(0),Some(expected),"candidate reference {mode}");
                    }
                    assert_eq!(state.demand().status(0),before_status);
                    let selected=options.into_best().unwrap();
                    outcomes.push((selected.used_height(),selected.next_state().next_item(),selected.next_state().next_table_index()));
                    if let Some(owner)=selected.forced_break_owner(){forced.push(owner);}
                    for part in selected.parts(){
                        if let Some(range)=part.items(){seen.extend(range);}
                        if let Some(table)=part.table(){seen.extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());}
                    }
                    state=selected.into_next_state();
                }
                seen.sort_unstable();assert_eq!(seen,(0..items.len()).collect::<Vec<_>>(),"automatic {mode}");
                assert_eq!(state.demand().pending_definitions(),if mode=="terminal-self"{&[][..]}else{&[0][..]});
                if mode=="forced" {assert_eq!(forced.len(),4);}
                assert!(search.select(&state,search.maximum_height())?.is_none());
                Ok((search.record_charge(),search.work_charge(),outcomes,forced))
            };
            let (records,work,outcomes,forced)=automatic(0,1_000_000).unwrap();
            let prior=limits.base().get().max_fragments-(records-measured.record_charge());
            assert_eq!(automatic(prior,work).unwrap(),(limits.base().get().max_fragments,work,outcomes,forced));
            assert_eq!(automatic(prior+1,work).unwrap_err().kind,E::FragmentLimit);
            assert!(matches!(automatic(prior,work-1).unwrap_err().kind,E::TableSearchLimit|E::FootnoteSearchLimit));
            let shared = |prior,work| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                let mut search=prepare_book_v2_mixed_footnote_demand_search(&measured,&limits,work,prior)?;
                let initial=search.begin()?;
                let mut state=search.require_body(&initial,0..measured.flow().body_items().len())?;
                let cursor_key=|cursor:typaxis_pagination::book_v2::BookV2FootnoteCursor<'_, '_, '_, '_, '_>| {
                    (cursor.next_item(),cursor.next_table_index(),cursor.definition_started(),cursor.table_continuation().map(|c|(c.table_index(),c.offset(),c.next_row(),c.next_caption_item(),c.cell_progress_fingerprint())))
                };
                assert!(!state.definition_started(1));
                let mut seen=[Vec::new(),Vec::new()];let mut visits=Vec::new();let mut interleaved=false;
                while !state.pending_definitions().is_empty() {
                    assert!(visits.len()<30,"shared queue {mode}");
                    let definition=if state.status(0)==Some(Status::Pending){0}else{1};
                    let saved=state.definition_cursor(1).map(cursor_key);
                    let capacity=if definition==1&&matches!(mode,"overflow"|"table-early"|"table-late"){raw(32*65536)}else if definition==1&&matches!(mode,"headers"|"header-span"|"repeat-demand"){raw(64*65536)}else{raw(128*65536)};
                    let choices=search.enumerate_definition(&state,definition,capacity)?;
                    choices.verify_demand(&state)?;
                    assert_eq!(choices.definition_index(),definition);
                    for choice in choices.choices(){choice.candidate().verify_demand(&state)?;}
                    let selected=choices.into_best().unwrap();
                    selected.verify_demand(&state)?;
                    for part in selected.parts(){
                        if let Some(range)=part.items(){seen[definition].extend(range);}
                        if let Some(table)=part.table(){assert_eq!(table.definition_index(),Some(definition));seen[definition].extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());}
                    }
                    if definition==1 && selected.next_state().demand().status(0)==Some(Status::Pending) {
                        let other=search.enumerate_definition(selected.next_state().demand(),0,raw(128*65536))?;
                        other.verify_demand(selected.next_state().demand())?;
                        assert!(matches!(other.verify(selected.next_state()),Err(e) if e.kind==E::ReceiptMismatch));
                        for choice in other.choices(){assert!(matches!(choice.candidate().verify(selected.next_state()),Err(e) if e.kind==E::ReceiptMismatch));}
                    }
                    let next=selected.into_next_state().into_demand();
                    if definition==0 {
                        assert_eq!(next.definition_cursor(1).map(cursor_key),saved,"other note changed target cursor {mode}");
                        interleaved|=state.definition_cursor(1).is_some_and(|c|c.table_continuation().is_some());
                    }
                    if next.status(1)==Some(Status::Pending) {
                        let marker=measured.flow().definition_marker(1).unwrap().item_index();
                        assert_eq!(next.definition_started(1),seen[1].contains(&marker),"marker start {mode}");
                    }
                    visits.push((definition,next.definition_cursor(1).map(cursor_key)));
                    state=next;
                }
                if matches!(mode,"table-early"|"repeat-demand") { assert!(interleaved,"missing interleaved continuation {mode}"); }
                for definition in 0..2 {
                    seen[definition].sort_unstable();
                    let expected=if definition==0&&mode=="terminal-self"{Vec::new()}else{(0..measured.flow().definition_items(definition).unwrap().len()).collect::<Vec<_>>()};
                    assert_eq!(seen[definition],expected,"shared source {mode}/{definition}");
                }
                assert_eq!(state.status(1),Some(Status::Complete));
                assert_eq!(state.status(2),Some(Status::Unreferenced));
                Ok((search.record_charge(),search.work_steps(),visits))
            };
            let (records,work,visits)=shared(0,1_000_000).unwrap();
            let prior=limits.base().get().max_fragments-(records-measured.record_charge());
            assert_eq!(shared(prior,work).unwrap(),(limits.base().get().max_fragments,work,visits));
            assert_eq!(shared(prior+1,work).unwrap_err().kind,E::FragmentLimit);
            assert!(matches!(shared(prior,work-1).unwrap_err().kind,E::TableSearchLimit|E::FootnoteSearchLimit));
            let regions = |prior,work,required| -> Result<_,typaxis_pagination::ProductionBodyPaginationError> {
                let mut search=prepare_book_v2_mixed_footnote_demand_search(&measured,&limits,work,prior)?;
                let initial=search.begin()?;
                let mut state=search.require_body(&initial,0..measured.flow().body_items().len())?;
                let mut seen=[Vec::new(),Vec::new()];let mut markers=[0,0];let mut outcomes=Vec::new();
                let capacity=if required {raw(128*65536)}else if matches!(mode,"overflow"|"table-early"|"table-late"){raw(32*65536)}else if matches!(mode,"headers"|"header-span"|"repeat-demand"){raw(64*65536)}else{raw(128*65536)};
                while !state.pending_definitions().is_empty() {
                    assert!(outcomes.len()<30,"regions {mode}");
                    let selected=if required {search.select_required_region(&state,capacity)?}else{search.select_region(&state,capacity)?}.expect(mode);
                    selected.verify(&state)?;
                    assert!(matches!(selected.verify(&initial),Err(e) if e.kind==E::ReceiptMismatch));
                    assert!(selected.used_height()<=capacity);
                    let mut used=Length::ZERO;let mut after=None;
                    for (ordinal, placed) in selected.fragments().iter().enumerate() {
                        let fragment=placed.fragment();let definition=fragment.definition_index();
                        fragment.verify(measured.flow())?;
                        assert!(fragment.items().is_err());assert!(fragment.consumed_range().is_err());
                        let candidate=fragment.mixed().expect("typed mixed region content");
                        let mut current=Vec::new();
                        let first=candidate.parts().first().expect("source progress");
                        let (paint,before)=if let Some(table)=first.table(){
                            let actual=&measured.tables()[table.before().table_index()];
                            (true,if table.before().is_initial(){actual.space_before()}else{Length::ZERO})
                        }else{
                            let item=&measured.flow().definition_items(definition).unwrap()[first.items().unwrap().start];
                            (item.source().is_some(),item.space_before())
                        };
                        let gap=if paint {after.map(|a:Length|a.checked_add(before).unwrap()).unwrap_or(Length::ZERO)}else{Length::ZERO};
                        assert_eq!(placed.offset(),used.checked_add(gap).unwrap(),"region gap {mode}/{ordinal}");
                        used=placed.offset().checked_add(fragment.used_height()).unwrap();after=fragment.space_after();
                        for part in candidate.parts(){
                            if let Some(range)=part.items(){current.extend(range);}
                            if let Some(table)=part.table(){current.extend(table.source_leaf_ranges().collect::<Result<Vec<_>,_>>()?.into_iter().flatten());}
                        }
                        let marker=measured.flow().definition_marker(definition).unwrap().item_index();
                        assert_eq!(fragment.marker().is_some(),current.contains(&marker));
                        markers[definition]+=usize::from(fragment.marker().is_some());
                        let expected:Vec<_>=measured.flow().references_in_items(Some(definition),0..measured.flow().definition_items(definition).unwrap().len()).iter()
                            .filter(|r|(r.first_item_index()..=r.last_item_index()).all(|index|current.contains(&index))).map(|r|r.source().owner()).collect();
                        let actual:Vec<_>=fragment.references().map(|r|r.source().owner()).collect();
                        assert_eq!(actual,expected,"region references {mode}");
                        if required {
                            assert!(state.pending_definitions().contains(&definition),"only entry demands are reserved");
                            if !state.definition_started(definition) && fragment.forced_break_owner().is_none() {
                                assert!(fragment.marker().is_some(),"first reservation includes marker {mode}");
                            }
                        }
                        seen[definition].extend(current);
                        if fragment.forced_break_owner().is_some(){assert_eq!(ordinal+1,selected.fragments().len());}
                    }
                    assert_eq!(used,selected.used_height());
                    outcomes.push((selected.used_height(),selected.fragments().len(),selected.forced_break_owner()));
                    state=selected.into_next_state();
                }
                for definition in 0..2 {
                    seen[definition].sort_unstable();
                    let used=definition==1||mode!="terminal-self";
                    assert_eq!(markers[definition],usize::from(used),"region marker once {mode}");
                    assert_eq!(seen[definition],if used{(0..measured.flow().definition_items(definition).unwrap().len()).collect::<Vec<_>>()}else{Vec::new()},"region source {mode}");
                }
                assert_eq!(state.status(2),Some(Status::Unreferenced));
                Ok((search.record_charge(),search.work_steps(),outcomes))
            };
            for required in [false,true] {
                let (records,work,outcomes)=regions(0,1_000_000,required).unwrap();
                let prior=limits.base().get().max_fragments-(records-measured.record_charge());
                assert_eq!(regions(prior,work,required).unwrap(),(limits.base().get().max_fragments,work,outcomes));
                assert_eq!(regions(prior+1,work,required).unwrap_err().kind,E::FragmentLimit);
                assert!(matches!(regions(prior,work-1,required).unwrap_err().kind,E::TableSearchLimit|E::FootnoteSearchLimit));
            }
            let mut queue=prepare_book_v2_mixed_footnote_demand_search(&measured,&limits,1_000_000,0).unwrap();
            let initial=queue.begin().unwrap();let demanded=queue.require_body(&initial,0..measured.flow().body_items().len()).unwrap();
            assert!(matches!(queue.enumerate_definition(&demanded,1,raw(-1)),Err(e) if e.kind==E::InvalidFootnoteCapacity));
            // Failed enumeration returns the same context to its issuing queue.
            assert!(queue.enumerate_definition(&demanded,1,raw(128*65536)).unwrap().choices().len()>0);
            assert!(matches!(queue.enumerate_definition(&demanded,2,raw(128*65536)),Err(e) if e.kind==E::ReceiptMismatch));
            assert!(queue.evaluate_next(&demanded,raw(128*65536)).unwrap().is_some());
            assert!(queue.select_region(&demanded,raw(128*65536)).unwrap().is_some());
            assert!(queue.select_required_region(&initial,raw(128*65536)).unwrap().is_none());
            assert!(matches!(queue.select_required_region(&demanded,raw(-1)),Err(e) if e.kind==E::InvalidFootnoteCapacity));
            let _required=queue.select_required_region(&demanded,raw(128*65536)).unwrap();
            let first=queue.evaluate_next(&demanded,raw(128*65536)).unwrap().unwrap();
            let advanced=queue.advance(&demanded,&first).unwrap();
            assert!(matches!(queue.advance(&advanced,&first),Err(e) if e.kind==E::ReceiptMismatch));
            if mode=="leading-empty" {
                let empty_region=queue.select_region(&demanded,Length::ZERO).unwrap().unwrap();
                assert_eq!(empty_region.used_height(),Length::ZERO);
                assert_eq!(empty_region.fragments().len(),1);
                let fragment=empty_region.fragments()[0].fragment();
                assert!(fragment.marker().is_none());
                assert_eq!(fragment.mixed().unwrap().parts().len(),2);
                assert_eq!(fragment.continuation().unwrap().next_item(),0);
                assert!(!empty_region.next_state().definition_started(1));
                assert!(!demanded.definition_started(1));
                assert!(queue.select_region(empty_region.next_state(),raw(128*65536)).unwrap().is_some());
                let empty=queue.select_definition(&demanded,1,Length::ZERO).unwrap().unwrap();
                let pending=empty.into_next_state().into_demand();
                assert_eq!(pending.definition_cursor(1).unwrap().next_item(),0);
                assert!(pending.definition_cursor(1).unwrap().next_table_index().is_some());
                assert!(!pending.definition_started(1));
                assert!(queue.select_definition(&pending,1,raw(128*65536)).unwrap().unwrap().next_state().is_complete());
            }
            let mut search=prepare_book_v2_definition_mixed_search(&measured,1,&limits,1_000_000,0).unwrap();
            let state=search.begin(0..measured.flow().body_items().len()).unwrap();
            assert!(matches!(search.evaluate(&state,&[Request::Items{end:items.len()+1}],search.maximum_height()),Err(e) if e.kind==E::ReceiptMismatch));
            if mode=="prefix-keep" {
                assert!(search.evaluate(&state,&[Request::Items{end:1}],search.maximum_height()).unwrap().is_none());
                assert_eq!(state.demand().status(0),Some(Status::Unreferenced));
            }
            if matches!(mode,"neighbors"|"table-keep"|"ordinary") {
                let before_work=search.work_charge();
                assert!(search.evaluate(&state,&[Request::Items{end:1}],Length::ZERO).unwrap().is_none());
                assert!(search.work_charge()>before_work);
                let prefix=search.evaluate(&state,&[Request::Items{end:1}],search.maximum_height()).unwrap().unwrap();
                let other=search.evaluate(&state,&[Request::Items{end:1}],search.maximum_height()).unwrap().unwrap();
                assert!(matches!(prefix.verify(other.next_state()),Err(e) if e.kind==E::ReceiptMismatch));
                let mut other_search=prepare_book_v2_definition_mixed_search(&measured,1,&limits,1_000_000,0).unwrap();
                assert!(matches!(other_search.evaluate(&state,&[Request::Items{end:1}],search.maximum_height()),Err(e) if e.kind==E::ReceiptMismatch));
                assert!(other_search.begin(1..2).is_err());
                if mode=="table-keep" {
                    let cursor=search.begin_table(roots[0]).unwrap();
                    assert!(search.evaluate(prefix.next_state(),&[Request::Table{cursor,capacity:search.maximum_height()}],search.maximum_height()).unwrap().is_none());
                }
            }
            if mode=="leading-empty" {
                let zero=search.enumerate(&state,Length::ZERO).unwrap();
                assert_eq!(zero.choices().len(),2);
                let candidate=zero.into_best().unwrap();
                assert_eq!(candidate.used_height(),Length::ZERO);
                assert_eq!(candidate.parts().len(),2);
                assert_eq!(candidate.next_state().next_item(),0);
                assert_ne!(candidate.next_state().next_table_index(),state.next_table_index());
                assert!(!candidate.next_state().is_complete());
            }
            if mode=="empty-table" {
                let best=search.select(&state,search.maximum_height()).unwrap().unwrap();
                let exact=search.select(&state,best.used_height()).unwrap().unwrap();
                assert!(exact.next_state().is_complete(),"equal-cost trailing empty table must be consumed");
            }
            assert_eq!(prepare_book_v2_table_body_search(&measured,&limits,1_000_000,0).unwrap().body_table_count(), (0..measured.flow().table_count()).filter(|&index|measured.flow().table_source_definition(index)==Some(None)).count());
        }).unwrap();
        if !matches!(mode, "keep-conflict" | "lookback") {
            let result = crate::book_v2_resources::with_converged_book_v2_pdf(
                &input,
                &limits,
                JapaneseLineBreakMode::Normal,
                100_000_000,
                |pdf, observation| {
                    assert!(mode != "forced");
                    crate::book_v2_resources::tests::record_driver_pdf(pdf, observation, &limits);
                },
            );
            if mode == "forced" {
                let error = result.expect_err("forced suffix cannot precede its dependency");
                assert!(matches!(error,
                    crate::book_v2_resources::BookV2ConvergenceError::Stage { stage: "page stability", ref source }
                    if source.downcast_ref::<typaxis_pagination::ProductionBodyPaginationError>()
                        .is_some_and(|e| e.kind == E::JointPageNoFit)
                ), "unexpected forced-suffix refusal: {error:?}");
            } else {
                result.unwrap_or_else(|e| panic!("definition page {mode}: {e:?}"));
            }
        }
    }
}
