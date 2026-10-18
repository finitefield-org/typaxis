//! Runtime host for the real, private Book 2 CLI module. Paths are filled by
//! verify_vmb_initial_admission.py; production modules are compiled verbatim.
#![allow(dead_code, unused_imports)]
#![forbid(unsafe_code)]
#[path = "__BOOK_MODULE__"]
pub mod book_v2_resources;
#[path = "__JSTF_CASES__"]
mod jstf_cases;
#[path = "__TABLE_CASES__"]
mod sfnt_cases;
use book_v2_resources::*;
use serde_json::{json, Value};
use typaxis_core::M4EffectiveResourceLimits;
use typaxis_syntax::book_v2::*;

mod fixture {
    use super::*;
    include!("fixture_harness.rs");
    pub(super) fn hosted<R>(
        case: &str,
        inspect: impl FnOnce(
            &PreparedBookV2Resources,
            &M4EffectiveResourceLimits,
            u64,
            typaxis_linebreak::JapaneseLineBreakMode,
        ) -> R,
    ) -> R {
        assert_eq!(case, "no-page");
        hosted_text(TEXT, inspect)
    }
    pub(super) fn hosted_native_cff<R>(font:&[u8], inspect:impl FnOnce(&PreparedBookV2Resources,&M4EffectiveResourceLimits,u64,typaxis_linebreak::JapaneseLineBreakMode)->R)->R {
        // Explicit synthetic native equations; no authored book content changes.
        let prefix="Result ";let inline=r"\sqrt{\frac{x+1}{y}}";let middle=" Result ";let display=r"∑x+∫x+\frac{x+1}{y-2}";
        let source=format!("{prefix}{inline}{middle}{display}");let ie=prefix.len()+inline.len();let ds=ie+middle.len();let end=source.len();
        let span=|a,b|json!({"source_id":0,"start_byte":a,"end_byte":b});let text=|a,b|json!({"text_id":0,"start_byte":a,"end_byte":b});
        let mut value=data(false);
        let mappings=[(0,prefix.len()),(prefix.len(),ie),(ie,ds),(ds,end)].map(|(a,b)|json!({"kind":"identity","source_span":span(a,b),"text_range":{"start_byte":a,"end_byte":b}}));
        value["text_buffers"]=json!([{"text_id":0,"utf8":source,"mappings":mappings}]);
        value["document"]["blocks"]=json!([{"kind":"semantic_container","node_id":1,"classes":[],"semantic_kind":"solution","anchor_id":null,"span":span(0,end),"blocks":[
            {"kind":"paragraph","node_id":2,"classes":[],"span":span(0,ds),"children":[
                {"kind":"text","node_id":3,"span":span(0,prefix.len()),"text_span":text(0,prefix.len())},
                {"kind":"inline_math","node_id":4,"span":span(prefix.len(),ie),"math_source":{"language":"typaxis-math","version":"1","text_span":text(prefix.len(),ie)},"speech":"Synthetic radical and fraction test"},
                {"kind":"text","node_id":5,"span":span(ie,ds),"text_span":text(ie,ds)}]},
            {"kind":"display_math","node_id":6,"classes":[],"span":span(ds,end),"math_source":{"language":"typaxis-math","version":"1","text_span":text(ds,end)},"speech":"Synthetic sum integral and fraction test"}]}]);
        let mut style=value["style_sheet"]["rules"][2].clone();style["selector"]="display_math".into();style["style_id"]="cff-native-display".into();style["source_order"]=4.into();value["style_sheet"]["rules"].as_array_mut().unwrap().push(style);
        value["page_masters"]["masters"][0]["body"]["height"]=(240*65536).into();
        value["sources"][0]["sha256"]=typaxis_core::sha256(source.as_bytes()).iter().map(|b|format!("{b:02x}")).collect::<String>().into();value["sources"][0]["utf8_byte_length"]=end.into();
        value["resources"]["font_faces"][0]["media_type"]="sfnt-cff1".into();value["resources"]["font_faces"][0]["expected_sha256"]=typaxis_core::sha256(font).iter().map(|b|format!("{b:02x}")).collect::<String>().into();
        let caps=caps();let root=Root::new();
        fs::write(root.0.join("document-package.json"),serde_json::to_vec(&value).unwrap()).unwrap();fs::write(root.0.join("input.tsf"),source.as_bytes()).unwrap();fs::write(root.0.join("body.bin"),font).unwrap();fs::write(root.0.join("collection.bin"),COLLECTION).unwrap();fs::write(root.0.join("cover.bin"),PNG).unwrap();
        let (session,raw)=HostBookV2InputSession::open(MachineInputHostOptions::new(HostPath::new(root.0.join("document-package.json")).unwrap(),None),caps.base()).unwrap();
        let decoded=session.decode_and_bind(&raw,&DocumentPackageDecodePolicy::new(caps.base())).unwrap();let sources=session.admit_sources(&decoded,caps.base()).unwrap();
        let body=prepare_admitted_book_v2_body(session.finish(raw,decoded,sources).unwrap(),caps.base()).unwrap();
        let input=crate::book_v2_resources::prepare_book_v2_resources(body,&root.context(),&config(&caps),&caps).unwrap();
        assert_eq!(input.resources().fonts()[0].bytes(),font);inspect(&input,&caps,MAXIMUM,MODE)
    }
    pub(super) fn hosted_text<R>(
        text: &str,
        inspect: impl FnOnce(
            &PreparedBookV2Resources,
            &M4EffectiveResourceLimits,
            u64,
            typaxis_linebreak::JapaneseLineBreakMode,
        ) -> R,
    ) -> R {
        let caps = caps();
        let root = Root::new();
        let mut value = data(false);
        if text != TEXT {
            fn ranges(value: &mut Value, length: usize) {
                match value {
                    Value::Object(object) => {
                        if let Some(end) = object.get_mut("end_byte") {
                            assert_eq!(end.as_u64(), Some(TEXT.len() as u64));
                            *end = length.into();
                        }
                        for value in object.values_mut() {
                            ranges(value, length);
                        }
                    }
                    Value::Array(items) => {
                        for value in items {
                            ranges(value, length);
                        }
                    }
                    _ => {}
                }
            }
            ranges(&mut value, text.len());
            value["text_buffers"][0]["utf8"] = text.into();
        }
        let prepared = input(&root, &value, &caps);
        inspect(&prepared, &caps, MAXIMUM, MODE)
    }
}
use fixture::hosted as with_convergence_probe_fixture;
include!("original_initial_style_tests.rs");

const NAME_KEYED_FIXTURE: &[u8] = include_bytes!("__TINY_CFF__");
fn original_font() -> Vec<u8> {
    let list =
        std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font = std::fs::read(list.lines().next().unwrap()).unwrap();
    assert_eq!(
        hex(&typaxis_core::sha256(&font)),
        "66ef3270e68690612e8bf982acfad0e8b40212ce64661cce2bb6d3a98ac84717"
    );
    font
}
fn hex(value: &[u8]) -> String {
    value.iter().map(|n| format!("{n:02x}")).collect()
}
fn native_math_cff_cuts(input:&PreparedBookV2Resources,caps:&M4EffectiveResourceLimits,maximum:u64)->Vec<Value>{
    use typaxis_layout::book_v2::{preflight_book_v2_native_math_counted,BookV2NativeMathBudgetObservation,BookV2ObservedNativeMathError as N};
    use typaxis_shaping::book_v2::{with_book_v2_observed_source_accounting,BookV2ObservedAccountingError as A};
    let policy=prepare_book_v2_resource_policy(input.body(),caps).unwrap();let bindings=typaxis_layout::book_v2::bind_book_v2_vectors(&policy,input.resources(),caps).unwrap();
    let mut plan=BookV2NativeMathBudgetObservation::default();preflight_book_v2_native_math_counted(&bindings,input.resources(),caps,0,0,&mut plan).unwrap().unwrap();
    let run=|carrier:&mut typaxis_resources::book_v2::BookV2FontCommandSession,cells:&mut[u64;3],max_r:u64,max_w:u64|{
        let [r,s,w]=cells;
        with_book_v2_observed_source_accounting(r,s,w,caps.base(),max_r,max_w,|accounting|carrier.compute_native_math_observed_counted(&bindings,input.resources(),caps,accounting).map(|native|native.unwrap().math().fingerprint()))
    };
    let mut carrier=typaxis_resources::book_v2::BookV2FontCommandSession::new(caps,maximum);let mut full=[0;3];run(&mut carrier,&mut full,caps.base().get().max_fragments,maximum).unwrap();
    let layout=plan.reserved_layout_units();let mut rows=Vec::new();
    for (name,max_r,max_w,expected_owner)in [
        ("preflight-work",caps.base().get().max_fragments,layout-1,0),
        ("cff-header-work",caps.base().get().max_fragments,layout+1051,4),
        ("cff-hash-work",caps.base().get().max_fragments,layout+1052+50815,4),
        ("cff-late-work",caps.base().get().max_fragments,full[2]-1,6),
        ("cff-records",plan.record_charge(),maximum,4),
    ]{
        let mut carrier=typaxis_resources::book_v2::BookV2FontCommandSession::new(caps,max_w);let mut cells=[0;3];
        let error=run(&mut carrier,&mut cells,max_r,max_w).unwrap_err();
        assert!(matches!(&error,N::Accounting(A::Work{owner}|A::Records{owner})if owner.get()==expected_owner),"{name}: {error:?}");
        let before=carrier.cff_program_statistics();let prefix=cells;let retry=run(&mut carrier,&mut cells,max_r,max_w).unwrap_err();let after=carrier.cff_program_statistics();
        assert!(cells.iter().zip(prefix).all(|(a,b)|*a>=b));assert!(after.operations_used>=before.operations_used&&after.cached_glyphs>=before.cached_glyphs);
        rows.push(json!({"name":name,"owner":expected_owner,"paid":prefix,"retry_paid":cells,"error":format!("{error:?}"),"retry":format!("{retry:?}"),"operations_retained":before.operations_used,"cache_retained":before.cached_glyphs}));
    }
    let mut carrier=typaxis_resources::book_v2::BookV2FontCommandSession::new(caps,maximum);let mut cells=[0,caps.base().get().max_spool_bytes-plan.spool_charge(),0];
    let error=run(&mut carrier,&mut cells,caps.base().get().max_fragments,maximum).unwrap_err();assert!(matches!(error,N::Accounting(A::Spool{owner})if owner.get()==4));
    let prefix=cells;let before=carrier.cff_program_statistics();assert!(run(&mut carrier,&mut cells,caps.base().get().max_fragments,maximum).is_err());assert_eq!(before,carrier.cff_program_statistics());
    rows.push(json!({"name":"cff-spool","owner":4,"paid":prefix,"retry_paid":cells,"error":format!("{error:?}"),"operations_retained":before.operations_used,"cache_retained":before.cached_glyphs}));
    let mut wrong=typaxis_resources::book_v2::BookV2FontCommandSession::new(caps,maximum+1);let mut cells=[0;3];let error=run(&mut wrong,&mut cells,caps.base().get().max_fragments,maximum).unwrap_err();assert!(matches!(error,N::Accounting(A::LimitsMismatch)));assert_eq!(cells,[0;3]);assert_eq!(wrong.cff_program_statistics().operations_used,0);
    rows
}
fn native_math_cff_pdf(output:&std::path::Path)->Value {
    let list=std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();let font=std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    assert_eq!(hex(&typaxis_core::sha256(&font)),"6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f");
    fixture::hosted_native_cff(&font,|input,caps,maximum,mode|{
        let cuts=native_math_cff_cuts(input,caps,maximum);
        hosted_with_cff(input,caps,Some(&font),|pending,context,config,_|{
        let mut budget=BookV2ColumnPdfConvergenceBudget::new(caps,maximum);
        let input=prepare_counted_book_v2_column_resources(pending,context,config,caps,&mut budget).unwrap();
        let (first,native)=with_budgeted_converged_book_v2_column_pdf(&input,caps,mode,&mut budget,4,|roots,_|{
            let native=roots.display().source().original_graph().lines().prepared().native_math().unwrap();
            assert_eq!(native.receipts().len(),2);
            let facts=native.receipts().iter().map(|receipt|json!({"node":receipt.node_id().get(),"font":receipt.font_face_id().get(),"computation":hex(&receipt.computation().fingerprint()),"math_table":hex(&receipt.computation().math_table_fingerprint()),"source":receipt.source().parsed().source(),"speech":receipt.source().domain().speech,"glyphs":receipt.computation().paints().iter().filter_map(|p|match p{typaxis_math::MathPaint::Glyph(g)=>Some(json!({"gid":g.original_gid().get(),"unicode":g.unicode().to_string(),"size":g.font_size_raw(),"x":g.x(),"y":g.y(),"advance":g.advance()})),_=>None}).collect::<Vec<_>>(),"paints":receipt.computation().paints().len(),"rules":receipt.computation().paints().iter().filter(|p|matches!(p,typaxis_math::MathPaint::Rule(_))).count()})).collect::<Vec<_>>();
            (roots.pdf().bytes().to_vec(),facts)
        }).unwrap();
        let one=budget.cff_program_statistics();let before_retry=budget.observation();
        let second=with_budgeted_converged_book_v2_column_pdf(&input,caps,mode,&mut budget,4,|roots,_|roots.pdf().bytes().to_vec()).unwrap();let two=budget.cff_program_statistics();
        assert_eq!(first,second);assert_eq!(one,two);assert!(one.cached_glyphs<100);assert!(native.iter().all(|n|n["rules"].as_u64().unwrap()>0));
        std::fs::write(output,&first).unwrap();
        json!({"original_font_sha256":hex(&typaxis_core::sha256(&font)),"native_receipts":native,"typed_quota_cuts":cuts,"first":rsw(before_retry),"second":rsw(budget.observation()),"pdf_bytes":first.len(),"pdf_sha256":hex(&typaxis_core::sha256(&first)),"cff_operations":two.operations_used,"cff_segments":two.outline_segments_used,"cff_cached_glyphs":two.cached_glyphs,"retry_same_pdf":true,"retry_same_evaluation_statistics":true})
        })
    })
}
fn original_cff_pdf(text: &str, output: &std::path::Path) -> Value {
    cff_pdf(original_font(), text, output)
}
fn original_math_cff_pdf(output: &std::path::Path) -> Value {
    let list=std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let font=std::fs::read(list.lines().nth(2).unwrap()).unwrap();
    assert_eq!(hex(&typaxis_core::sha256(&font)),"6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f");
    cff_pdf(font, "A∑∫√𝑥→(", output)
}
fn cff_pdf(font: Vec<u8>, text: &str, output: &std::path::Path) -> Value {
    fixture::hosted_text(text, |original, caps, maximum, mode| {
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            let prepared =
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut budget)
                    .unwrap();
            assert_eq!(prepared.resources().fonts()[0].bytes(), font);
            let prefix = budget.observation();
            let first = with_budgeted_converged_book_v2_column_pdf(
                &prepared,
                caps,
                mode,
                &mut budget,
                4,
                |roots, _| roots.pdf().bytes().to_vec(),
            )
            .unwrap();
            let after_first = budget.observation();
            let cff_first = budget.cff_program_statistics();
            let second = with_budgeted_converged_book_v2_column_pdf(
                &prepared,
                caps,
                mode,
                &mut budget,
                4,
                |roots, _| roots.pdf().bytes().to_vec(),
            )
            .unwrap();
            let cff_second = budget.cff_program_statistics();
            assert_eq!(first, second);
            assert!(after_first.work_steps() > prefix.work_steps());
            assert!(budget.observation().work_steps() > after_first.work_steps());
            assert_eq!(cff_first.operations_used, cff_second.operations_used);
            assert_eq!(cff_first.cached_glyphs, cff_second.cached_glyphs);
            std::fs::write(output, &first).unwrap();
            json!({"text":text,"occurrences":6,"pdf_bytes":first.len(),
                "pdf_sha256":hex(&typaxis_core::sha256(&first)),"initial":rsw(prefix),
                "first":rsw(after_first),"second":rsw(budget.observation()),
                "package_sha256":hex(&prepared.body().styled().body().canonical_jcs_sha256()),
                "font_sha256":hex(&typaxis_core::sha256(&font)),
                "cff_operations":cff_second.operations_used,
                "cff_segments":cff_second.outline_segments_used,
                "cff_cached_glyphs":cff_second.cached_glyphs,"retries_byte_identical":true})
        })
    })
}

use typaxis_font::{CffAdmissionBudgetV2, CffBaseCmapBudgetV2, CffCmapBudgetV2};
use typaxis_shaping::book_v2::BookV2ObservedAccountingError as AccountingError;
struct StopJstf<B> {
    inner: B,
    stop: usize,
    amounts: Vec<u64>,
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffBaseCmapBudgetV2 for StopJstf<B> {
    type Error = AccountingError;
    fn reserve(
        &mut self,
        s: typaxis_font::CffBaseCmapStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work(n)
    }
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffCmapBudgetV2 for StopJstf<B> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_variation(s, r, b)
    }
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffAdmissionBudgetV2 for StopJstf<B> {
    const OBSERVE_METADATA: bool = B::OBSERVE_METADATA;
    const OBSERVE_TABLES: bool = B::OBSERVE_TABLES;
    const OBSERVE_PROGRAM: bool = B::OBSERVE_PROGRAM;
    const OBSERVE_OPTIONAL_LOCAL: bool = B::OBSERVE_OPTIONAL_LOCAL;
    const OBSERVE_MATH: bool = B::OBSERVE_MATH;
    const OBSERVE_JSTF: bool = B::OBSERVE_JSTF;
    const OBSERVE_SOURCE_OWNER: bool = B::OBSERVE_SOURCE_OWNER;
    const OBSERVE_RESOURCE_MEDIA: bool = B::OBSERVE_RESOURCE_MEDIA;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        s: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_program(s, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_program(n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_optional_local(n)
    }
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_math(n)
    }
    fn work_jstf(&mut self, n: u64) -> Result<(), Self::Error> {
        self.amounts.push(n);
        if self.amounts.len() - 1 == self.stop {
            return Err(AccountingError::Work {
                owner: typaxis_core::NodeId::new(0),
            });
        }
        self.inner.work_jstf(n)
    }
}
fn positive_jstf() -> Value {
    let name_keyed = NAME_KEYED_FIXTURE;
    let base = original_font();
    let (_, lookup) = jstf_cases::lookup_seeds()
        .into_iter()
        .find(|(n, _)| n == "context2-true-1-2")
        .unwrap();
    // This is an explicit derived fixture. The original-font PDF controls use
    // the unchanged bytes; no original input file is rewritten here.
    let font = sfnt_cases::replace(&base, *b"JSTF", Some(jstf_cases::jstf(&lookup)));
    with_convergence_probe_fixture("no-page", |original, caps, maximum, _| {
        let named = typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(name_keyed), 0, caps).unwrap();
        assert_eq!(named.program().font_dict_count(), 1);
        assert!(named.program().cid_for_gid(1).is_none());
        assert!(named.program().sid_for_gid(1).is_some());
        let native =
            typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(font.as_slice()), 0, caps)
                .unwrap();
        assert_eq!(native.source_sha256(), typaxis_core::sha256(&font));
        let mut style = [0; 3];
        let mut ledger_reference = None;
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let body = prepare_admitted_book_v2_body_with_counted_style(
                input,
                caps.base(),
                &mut |r, s, w| {
                    for (v, n) in style.iter_mut().zip([r, s, w]) {
                        *v += n;
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
            ledger_reference = Some(
                crate::book_v2_resources::prepare_book_v2_resources(body, context, config, caps)
                    .unwrap(),
            );
        });
        let reference = |stop| {
            let [mut r, mut s, mut w] = style;
            let mut amounts = Vec::new();
            let result = typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                caps.base(),
                caps.base().get().max_fragments,
                maximum,
                |a| {
                    fund_resource_catalog_reference(a, original.resources(), None);
                    fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                    fund_resource_read_reference(
                        a,
                        font.len(),
                        original.resources().fonts()[0].uri().as_str(),
                        original.resources().fonts()[0].family(), 2 + original.resources().fonts().len() + original.resources().images().len(), true);
                    fund_resource_media_reference(a, &font, original.resources());
                    let mut trace = StopJstf {
                        inner: a.resource_admission_budget(typaxis_core::NodeId::new(0)),
                        stop,
                        amounts: Vec::new(),
                    };
                    let result = typaxis_font::admit_sfnt_cff1_v2_owned_with_admission_budget(
                        font.clone(),
                        0,
                        caps,
                        &mut trace,
                    );
                    amounts = trace.amounts;
                    if matches!(result, Ok(Ok(_))) {
                        for font in original.resources().fonts().iter().skip(1) {
                            fund_resource_read_reference(
                                a,
                                font.bytes().len(),
                                font.uri().as_str(),
                                font.family(), 2 + original.resources().fonts().len() + original.resources().images().len(), false);
                        }
                        fund_resource_attempt_storage_reference(
                            a,
                            original.resources().images().len(),
                        );
                        for image in original.resources().images() {
                            fund_resource_read_reference(
                                a,
                                image.bytes().len(),
                                image.uri().as_str(),
                                "", 2 + original.resources().fonts().len() + original.resources().images().len(), false);
                        }
                        fund_resource_ledger_reference(
                            a,
                            ledger_reference.as_ref().unwrap().resources(),
                        );
                        fund_read_ledger_reference(a, ledger_reference.as_ref().unwrap());
                    }
                    result
                },
            );
            (result, [r, s, w], amounts)
        };
        let (ready, full, amounts) = reference(usize::MAX);
        ready.unwrap().unwrap();
        let bitmap = amounts.iter().position(|n| *n == 65536).unwrap();
        let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let prepared =
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            assert_eq!(prepared.resources().fonts()[0].bytes(), font);
        });
        assert_eq!(rsw(warm.observation()), full);
        let mut cuts = Vec::new();
        for stop in [0, bitmap] {
            let (result, prefix, _) = reference(stop);
            let typaxis_font::CffAdmissionBudgetFailureV2::Tables(expected) = result.unwrap_err()
            else {
                panic!()
            };
            assert_eq!(expected.point.table_tag, Some(*b"JSTF"));
            assert_eq!(
                expected.point.stage,
                if stop == 0 {
                    typaxis_font::CffTableAdmissionStageV2::JstfHeader
                } else {
                    typaxis_font::CffTableAdmissionStageV2::JstfSetInit
                }
            );
            let cap = prefix[2] + amounts[stop] - 1;
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, cap);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                let error = prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget,
                )
                .unwrap_err();
                let BookV2ColumnInitialStylePreparationError::Admission { failure, .. } = error
                else {
                    panic!()
                };
                assert_eq!(failure.font_face_id.get(), 0);
                let typaxis_font::CffAdmissionBudgetFailureV2::Tables(actual) = failure.failure
                else {
                    panic!()
                };
                assert_eq!(actual, expected);
            });
            assert_eq!(rsw(budget.observation()), prefix);
            assert_eq!(budget.observation().output_charge(), 0);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                assert!(prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget
                )
                .is_err());
            });
            let retry = rsw(budget.observation());
            assert!(retry[2] >= prefix[2] && retry[2] <= cap);
            cuts.push(json!({"request":stop,"stage":format!("{:?}",expected.point.stage),"paid":prefix,"cap":cap,"retry":retry}));
        }
        json!({"base_fixture_sha256":hex(&typaxis_core::sha256(&base)),"font_sha256":hex(&typaxis_core::sha256(&font)),"derived_fixture":true,"name_keyed_positive":true,"style":style,"full":full,"jstf_requests":amounts.len(),"jstf_work":amounts.iter().sum::<u64>(),"cuts":cuts,"font_subject":0,"quota_owner":0,"positive_resource":true})
    })
}
struct StopMath<B> {
    inner: B,
    stop: usize,
    amounts: Vec<u64>,
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffBaseCmapBudgetV2 for StopMath<B> {
    type Error = AccountingError;
    fn reserve(
        &mut self,
        s: typaxis_font::CffBaseCmapStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve(s, r, b)
    }
    fn work(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work(n)
    }
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffCmapBudgetV2 for StopMath<B> {
    fn reserve_variation(
        &mut self,
        s: typaxis_font::CffVariationStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_variation(s, r, b)
    }
}
impl<B: CffAdmissionBudgetV2<Error = AccountingError>> CffAdmissionBudgetV2 for StopMath<B> {
    const OBSERVE_METADATA: bool = B::OBSERVE_METADATA;
    const OBSERVE_TABLES: bool = B::OBSERVE_TABLES;
    const OBSERVE_PROGRAM: bool = B::OBSERVE_PROGRAM;
    const OBSERVE_OPTIONAL_LOCAL: bool = B::OBSERVE_OPTIONAL_LOCAL;
    const OBSERVE_MATH: bool = B::OBSERVE_MATH;
    const OBSERVE_JSTF: bool = B::OBSERVE_JSTF;
    const OBSERVE_SOURCE_OWNER: bool = B::OBSERVE_SOURCE_OWNER;
    const OBSERVE_RESOURCE_MEDIA: bool = B::OBSERVE_RESOURCE_MEDIA;
    fn reserve_sfnt(
        &mut self,
        s: typaxis_font::CffSfntStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_sfnt(s, r, b)
    }
    fn work_sfnt(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_sfnt(n)
    }
    fn reserve_metadata(
        &mut self,
        s: typaxis_font::CffMetadataStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_metadata(s, r, b)
    }
    fn work_metadata(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_metadata(n)
    }
    fn work_tables(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_tables(n)
    }
    fn reserve_program(
        &mut self,
        s: typaxis_font::CffProgramStorageV2,
        r: usize,
        b: usize,
    ) -> Result<(), Self::Error> {
        self.inner.reserve_program(s, r, b)
    }
    fn work_program(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_program(n)
    }
    fn work_optional_local(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_optional_local(n)
    }
    fn work_jstf(&mut self, n: u64) -> Result<(), Self::Error> {
        self.inner.work_jstf(n)
    }
    fn work_math(&mut self, n: u64) -> Result<(), Self::Error> {
        self.amounts.push(n);
        if self.amounts.len() - 1 == self.stop {
            return Err(AccountingError::Work {
                owner: typaxis_core::NodeId::new(0),
            });
        }
        self.inner.work_math(n)
    }
}
fn positive_math_ties() -> Value {
    let name_keyed = NAME_KEYED_FIXTURE;
    let base = original_font();
    let paths = std::fs::read_to_string(std::env::var("TYPAXIS_HARFRUST_CACHE_FONTS").unwrap()).unwrap();
    let math_font = std::fs::read(paths.lines().nth(2).unwrap()).unwrap();
    assert_eq!(hex(&typaxis_core::sha256(&math_font)), "6075562b771f8b82f0c179e363389684f2dd09de30038269e2628e504bd7be0f");
    let count=u16::from_be_bytes(math_font[4..6].try_into().unwrap()) as usize;
    let record=math_font[12..12+count*16].chunks_exact(16).find(|r|&r[..4]==b"MATH").unwrap();
    let offset=u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
    let length=u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
    let table=&math_font[offset..offset+length];
    // Explicit structural CID fixture: the MATH bytes retain their original
    // order, including equal advances. This is not the original Latin font.
    let font=sfnt_cases::replace(&base,*b"MATH",Some(table.to_vec()));
    with_convergence_probe_fixture("no-page", |original, caps, maximum, _| {
        let named = typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(name_keyed), 0, caps).unwrap();
        assert_eq!(named.program().font_dict_count(), 1);
        assert!(named.program().cid_for_gid(1).is_none());
        assert!(named.program().sid_for_gid(1).is_some());
        let native =
            typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(font.as_slice()), 0, caps)
                .unwrap();
        assert_eq!(native.source_sha256(), typaxis_core::sha256(&font));
        let mut style = [0; 3];
        let mut ledger_reference = None;
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let body = prepare_admitted_book_v2_body_with_counted_style(
                input,
                caps.base(),
                &mut |r, s, w| {
                    for (v, n) in style.iter_mut().zip([r, s, w]) {
                        *v += n;
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
            ledger_reference = Some(
                crate::book_v2_resources::prepare_book_v2_resources(body, context, config, caps)
                    .unwrap(),
            );
        });
        let reference = |stop| {
            let [mut r, mut s, mut w] = style;
            let mut amounts = Vec::new();
            let result = typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
                &mut r,
                &mut s,
                &mut w,
                caps.base(),
                caps.base().get().max_fragments,
                maximum,
                |a| {
                    fund_resource_catalog_reference(a, original.resources(), None);
                    fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                    fund_resource_read_reference(
                        a,
                        font.len(),
                        original.resources().fonts()[0].uri().as_str(),
                        original.resources().fonts()[0].family(), 2 + original.resources().fonts().len() + original.resources().images().len(), true);
                    fund_resource_media_reference(a, &font, original.resources());
                    let mut trace = StopMath {
                        inner: a.resource_admission_budget(typaxis_core::NodeId::new(0)),
                        stop,
                        amounts: Vec::new(),
                    };
                    let result = typaxis_font::admit_sfnt_cff1_v2_owned_with_admission_budget(
                        font.clone(),
                        0,
                        caps,
                        &mut trace,
                    );
                    amounts = trace.amounts;
                    if matches!(result, Ok(Ok(_))) {
                        for font in original.resources().fonts().iter().skip(1) {
                            fund_resource_read_reference(
                                a,
                                font.bytes().len(),
                                font.uri().as_str(),
                                font.family(), 2 + original.resources().fonts().len() + original.resources().images().len(), false);
                        }
                        fund_resource_attempt_storage_reference(
                            a,
                            original.resources().images().len(),
                        );
                        for image in original.resources().images() {
                            fund_resource_read_reference(
                                a,
                                image.bytes().len(),
                                image.uri().as_str(),
                                "", 2 + original.resources().fonts().len() + original.resources().images().len(), false);
                        }
                        fund_resource_ledger_reference(
                            a,
                            ledger_reference.as_ref().unwrap().resources(),
                        );
                        fund_read_ledger_reference(a, ledger_reference.as_ref().unwrap());
                    }
                    result
                },
            );
            (result, [r, s, w], amounts)
        };
        let (ready, full, amounts) = reference(usize::MAX);
        ready.unwrap().unwrap();
        assert_eq!(amounts.len(),9622);
        assert!(amounts.iter().all(|n|*n==64));
        let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let prepared =
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            assert_eq!(prepared.resources().fonts()[0].bytes(), font);
            assert_eq!(prepared.resources().canonical_jcs(),ledger_reference.as_ref().unwrap().resources().canonical_jcs());
            let typaxis_resources::AdmittedProductionFontV3::Cff1V2(face)=&prepared.resources().fonts()[0] else {panic!("missing CID fixture")};
            assert_eq!(face.admission().fingerprint(),native.fingerprint());
        });
        assert_eq!(rsw(warm.observation()), full);
        let mut cuts = Vec::new();
        for stop in [0,1,57,5618,5619,6000,9621] {
            let (result, prefix, _) = reference(stop);
            let typaxis_font::CffAdmissionBudgetFailureV2::Tables(expected) = result.unwrap_err()
            else {
                panic!()
            };
            assert_eq!(expected.point.table_tag, Some(*b"MATH"));
            assert!(matches!(expected.point.stage,typaxis_font::CffTableAdmissionStageV2::MathHeader|typaxis_font::CffTableAdmissionStageV2::MathField|typaxis_font::CffTableAdmissionStageV2::MathOffset));
            let cap = prefix[2] + amounts[stop] - 1;
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, cap);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                let error = prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget,
                )
                .unwrap_err();
                let BookV2ColumnInitialStylePreparationError::Admission { failure, body } = error
                else {
                    panic!()
                };
                assert_eq!(failure.font_face_id.get(), 0);
                assert_eq!(body.styled().body().limits(),caps.base());
                body.provenance().read_ledger_token().unwrap().revalidate().unwrap();
                let typaxis_font::CffAdmissionBudgetFailureV2::Tables(actual) = failure.failure
                else {
                    panic!()
                };
                assert_eq!(actual, expected);
            });
            assert_eq!(rsw(budget.observation()), prefix);
            assert_eq!(budget.observation().output_charge(), 0);
            hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
                assert!(prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget
                )
                .is_err());
            });
            let retry = rsw(budget.observation());
            assert!(retry.iter().zip(prefix).all(|(a,b)|*a>=b) && retry[2] <= cap);
            cuts.push(json!({"request":stop,"stage":format!("{:?}",expected.point.stage),"file_offset":expected.point.file_offset,"table_offset":expected.point.table_offset,"paid":prefix,"cap":cap,"retry":retry}));
        }
        json!({"base_fixture_sha256":hex(&typaxis_core::sha256(&base)),"font_sha256":hex(&typaxis_core::sha256(&font)),"derived_fixture":true,"original_math_font_sha256":hex(&typaxis_core::sha256(&math_font)),"original_math_table_sha256":hex(&typaxis_core::sha256(table)),"equal_advance_variants_preserved":true,"name_keyed_positive":true,"style":style,"full":full,"math_requests":amounts.len(),"math_work":amounts.iter().sum::<u64>(),"cuts":cuts,"font_subject":0,"quota_owner":0,"positive_resource":true})
    })
}
// Independent logical fee for two declaration classes and each authentic read.
fn resource_attempt_reference(
    ledger: &typaxis_resources::AdmittedProductionResourceLedgerV3,
) -> [u64; 3] {
    let mut fee = resource_catalog_reference_fee(ledger, None);
    let resources = (ledger.fonts().len() + ledger.images().len()) as u64;
    let attempts=resources+2;let identity_work=typaxis_resources::host_open_identity_work_bound(attempts as usize).unwrap();
    fee[0]+=attempts;fee[1]+=attempts*typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64;
    fee[2]+=64+attempts*typaxis_resources::HOST_OPEN_IDENTITY_SLOT_BYTES as u64+2*identity_work+64+resources*identity_work;
    for (v, n) in fee
        .iter_mut()
        .zip(resource_resolver_reference_fee(ledger, None))
    {
        *v += n;
    }
    for count in [ledger.fonts().len(), ledger.images().len()] {
        if count > 0 {
            let bytes = count.div_ceil(64) as u64 * 8;
            fee[0] += 1;
            fee[1] += bytes;
            fee[2] += bytes + 64 + 128 * count as u64;
        }
    }
    for font in ledger.fonts() {
        let uri = font.uri().as_str().len() as u64;
        fee[0] += 1;
        fee[1] += uri;
        fee[2] += 128 + 20 * uri + 4 * font.family().len() as u64;
    }
    for image in ledger.images() {
        let uri = image.uri().as_str().len() as u64;
        fee[0] += 1;
        fee[1] += uri;
        fee[2] += 128 + 20 * uri;
    }
    for path in ledger
        .fonts()
        .iter()
        .map(|v| v.uri())
        .chain(ledger.images().iter().map(|v| v.uri()))
    {
        let uri = path.as_str();
        let bytes = uri.len() as u64;
        if bytes >= 256 {
            fee[0] += 1;
            fee[1] += bytes + 1;
        }
        fee[2] += 32 * bytes
            + 128 * uri.split('/').count() as u64
            + 128
            + if bytes < 256 { 256 } else { 0 };
        fee[0] += 1 + u64::from(bytes >= 256);
        fee[1] +=
            2 * std::mem::size_of::<usize>() as u64 + if bytes >= 256 { bytes + 1 } else { 0 };
        fee[2] += 32 * bytes
            + 128 * uri.split('/').count() as u64
            + 384
            + if bytes < 256 { 256 } else { 0 };
    }
    fee
}
fn resource_reads() -> Value {
    use typaxis_resources::{HostReadRequest as Request, HostReadStage as Stage};
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let mut style = [0; 3];
        hosted(original, caps, |input, _, _, _| {
            prepare_admitted_book_v2_body_with_counted_style(input, caps.base(), &mut |r, s, w| {
                for (v, n) in style.iter_mut().zip([r, s, w]) {
                    *v += n;
                }
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        });
        let mut requests = Vec::new();
        let resources = original
            .resources()
            .fonts()
            .iter()
            .map(|f| {
                (
                    false,
                    f.font_face_id().get(),
                    f.bytes().len(),
                    f.uri().as_str(),
                    f.family().len(),
                )
            })
            .chain(original.resources().images().iter().map(|i| {
                (
                    true,
                    i.image_id().get(),
                    i.bytes().len(),
                    i.uri().as_str(),
                    0,
                )
            }));
        for (image, id, length, uri, family_bytes) in resources {
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::OpenPath,
                    file_offset: 0,
                    buffer_bytes: if uri.len() < 256 {
                        0
                    } else {
                        uri.len() as u64 + 1
                    },
                    work: 16 * uri.len() as u64 + 64 + if uri.len() < 256 { 256 } else { 0 },
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::OpenRoot,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 16 * uri.len() as u64 + 128 * uri.split('/').count() as u64 + 64,
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::OpenLock,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 128,
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::OpenLedger,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 64,
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::OpenOwner,
                    file_offset: 0,
                    buffer_bytes: 2 * std::mem::size_of::<usize>() as u64,
                    work: 64,
                },
            ));
            let attempts=2+original.resources().fonts().len()+original.resources().images().len();let identity_work=typaxis_resources::host_open_identity_work_bound(attempts).unwrap();
            if !image&&id==0 {
                requests.push((image,id,Request{stage:Stage::OpenIdentityStorage,file_offset:0,buffer_bytes:attempts as u64*typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64,work:64}));
                requests.push((image,id,Request{stage:Stage::OpenIdentityMigrate,file_offset:0,buffer_bytes:0,work:attempts as u64*typaxis_resources::HOST_OPEN_IDENTITY_SLOT_BYTES as u64+2*identity_work+64}));
            }
            requests.push((image,id,Request{stage:Stage::OpenIdentityLookup,file_offset:0,buffer_bytes:0,work:identity_work}));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::DeclarationCheck,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 64 + 4 * (uri.len() + family_bytes) as u64,
                },
            ));
            if id == 0 {
                let count = if image {
                    original.resources().images().len()
                } else {
                    original.resources().fonts().len()
                };
                let bytes = count.div_ceil(64) as u64 * 8;
                requests.push((
                    image,
                    id,
                    Request {
                        stage: Stage::AttemptStorage,
                        file_offset: 0,
                        buffer_bytes: bytes,
                        work: bytes + 64,
                    },
                ));
            }
            for stage in [Stage::AttemptLookup, Stage::AttemptMark] {
                requests.push((
                    image,
                    id,
                    Request {
                        stage,
                        file_offset: 0,
                        buffer_bytes: 0,
                        work: 64,
                    },
                ));
            }
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::Buffer,
                    file_offset: 0,
                    buffer_bytes: length as u64,
                    work: 8256,
                },
            ));
            for offset in (0..length).step_by(8192) {
                requests.push((
                    image,
                    id,
                    Request {
                        stage: Stage::Chunk,
                        file_offset: offset as u64,
                        buffer_bytes: 0,
                        work: (length - offset).min(8192) as u64 + 64,
                    },
                ));
            }
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::RevalidatePath,
                    file_offset: 0,
                    buffer_bytes: if uri.len() < 256 {
                        0
                    } else {
                        uri.len() as u64 + 1
                    },
                    work: 16 * uri.len() as u64 + 64 + if uri.len() < 256 { 256 } else { 0 },
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::RevalidateRoot,
                    file_offset: 0,
                    buffer_bytes: 0,
                    work: 16 * uri.len() as u64 + 128 * uri.split('/').count() as u64 + 64,
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::DigestFinish,
                    file_offset: length as u64,
                    buffer_bytes: 0,
                    work: 192,
                },
            ));
            requests.push((
                image,
                id,
                Request {
                    stage: Stage::DeclarationUri,
                    file_offset: 0,
                    buffer_bytes: uri.len() as u64,
                    work: 64 + 16 * uri.len() as u64,
                },
            ));
        }
        let mut prefix = style;
        for (v, n) in prefix
            .iter_mut()
            .zip(resource_resolver_reference_fee(original.resources(), None))
        {
            *v += n;
        }
        for (v, n) in prefix
            .iter_mut()
            .zip(resource_catalog_reference_fee(original.resources(), None))
        {
            *v += n;
        }
        let mut cuts = Vec::new();
        for (image, id, request) in &requests {
            for axis in 0..3 {
                let amount = [
                    if request.stage==Stage::OpenIdentityStorage{request.buffer_bytes/typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64}else{u64::from(
                        matches!(
                            request.stage,
                            Stage::Buffer
                                | Stage::AttemptStorage
                                | Stage::DeclarationUri
                                | Stage::OpenOwner
                        ) || (matches!(request.stage, Stage::RevalidatePath | Stage::OpenPath)
                            && request.buffer_bytes > 0),
                    )},
                    request.buffer_bytes,
                    request.work,
                ][axis];
                if amount == 0 {
                    continue;
                }
                let cap = prefix[axis] + amount - 1;
                let mut base = caps.base().get().clone();
                if axis == 0 {
                    base.max_fragments = cap;
                }
                if axis == 1 {
                    base.max_spool_bytes = cap;
                }
                let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                let limited = M4EffectiveResourceLimits::defaults_for(&base);
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                    &limited,
                    if axis == 2 { cap } else { maximum },
                );
                hosted(original, &limited, |input, context, config, _| {
                    let error = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                    .unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Read { failure, body } = error
                    else {
                        panic!("expected read refusal, got {error:?}")
                    };
                    assert_eq!(body.styled().body().limits(), limited.base());
                    assert_eq!(failure.failure.request, *request);
                    assert!(
                        matches!(failure.subject, typaxis_diagnostics::ResourceErrorSubject::Image(found) if *image && found.get()==*id)
                            || matches!(failure.subject, typaxis_diagnostics::ResourceErrorSubject::FontFace(found) if !*image && found.get()==*id)
                    );
                    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                    let owner = match (axis, failure.failure.error) {
                        (0, E::Records { owner })
                        | (1, E::Spool { owner })
                        | (2, E::Work { owner }) => owner,
                        _ => panic!("wrong debit error"),
                    };
                    assert_eq!(owner.get(), 0);
                });
                assert_eq!(rsw(budget.observation()), prefix);
                assert_eq!(budget.observation().output_charge(), 0);
                hosted(original, &limited, |input, context, config, _| {
                    assert!(prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget
                    )
                    .is_err());
                });
                let retry = rsw(budget.observation());
                assert!(retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                assert!(retry[axis] <= cap);
                cuts.push(json!({"axis":axis,"image":image,"id":id,"stage":format!("{:?}",request.stage),"file_offset":request.file_offset,"prefix":prefix,"cap":cap,"retry":retry}));
            }
            for (v, n) in prefix.iter_mut().zip([
                if request.stage==Stage::OpenIdentityStorage{request.buffer_bytes/typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64}else{u64::from(
                    matches!(
                        request.stage,
                        Stage::Buffer
                            | Stage::AttemptStorage
                            | Stage::DeclarationUri
                            | Stage::OpenOwner
                    ) || (matches!(request.stage, Stage::RevalidatePath | Stage::OpenPath)
                        && request.buffer_bytes > 0),
                )},
                request.buffer_bytes,
                request.work,
            ]) {
                *v += n;
            }
        }
        let [mut r, mut s, mut w] = prefix;
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            caps.base().get().max_fragments,
            maximum,
            |a| fund_resource_ledger_reference(a, original.resources()),
        );
        prefix = [r, s, w];
        let before_read_ledger = prefix;
        let ledger_requests = read_ledger_reference_requests(original);
        for request in &ledger_requests {
            let cells = read_ledger_request_cells(*request);
            for axis in 0..3 {
                if cells[axis] == 0 { continue; }
                let cap = prefix[axis] + cells[axis] - 1;
                let mut base = caps.base().get().clone();
                if axis == 0 { base.max_fragments = cap; }
                if axis == 1 { base.max_spool_bytes = cap; }
                let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                let limited = M4EffectiveResourceLimits::defaults_for(&base);
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limited, if axis == 2 { cap } else { maximum });
                hosted(original, &limited, |input, context, config, _| {
                    let error = prepare_counted_book_v2_column_resources(input, context, config, &limited, &mut budget).unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::ReadLedger { failure, body } = error else {
                        panic!("expected host read ledger refusal, got {error:?}");
                    };
                    assert_eq!(failure.request, *request);
                    assert_eq!(body.styled().body().limits(), limited.base());
                    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                    let owner = typaxis_core::NodeId::new(0);
                    let expected = match axis { 0 => E::Records { owner }, 1 => E::Spool { owner }, _ => E::Work { owner } };
                    assert_eq!(failure.error, expected);
                    body.provenance().read_ledger_token().unwrap().revalidate().unwrap();
                });
                assert_eq!(rsw(budget.observation()), prefix);
                assert_eq!(budget.observation().output_charge(), 0);
                hosted(original, &limited, |input, context, config, _| {
                    assert!(prepare_counted_book_v2_column_resources(input, context, config, &limited, &mut budget).is_err());
                });
                let retry = rsw(budget.observation());
                assert!(retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                assert!(retry[axis] <= cap);
                cuts.push(json!({"kind":"read-ledger","axis":axis,"stage":format!("{:?}",request.stage),"prefix":prefix,"cap":cap,"retry":retry}));
            }
            for (value, amount) in prefix.iter_mut().zip(cells) { *value += amount; }
        }
        for axis in 0..3 {
            let mut base = caps.base().get().clone();
            if axis == 0 {
                base.max_fragments = prefix[0];
            }
            if axis == 1 {
                base.max_spool_bytes = prefix[1];
            }
            let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
            let limited = M4EffectiveResourceLimits::defaults_for(&base);
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                &limited,
                if axis == 2 { prefix[2] } else { maximum },
            );
            hosted(original, &limited, |input, context, config, _| {
                prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    &limited,
                    &mut budget,
                )
                .unwrap();
            });
            assert_eq!(rsw(budget.observation()), prefix);
        }
        json!({"style":style,"full":prefix,"read_requests":requests.len(),"ledger_requests":ledger_requests.len(),"before_read_ledger":before_read_ledger,"read_ledger_fee":[prefix[0]-before_read_ledger[0],prefix[1]-before_read_ledger[1],prefix[2]-before_read_ledger[2]],"typed_cuts":cuts,"exact_axes":3,"quota_owner":0,"partial_receipts_published":0,"attempt_permits_connected":true,"declaration_permits_connected":true,"resolution_permits_connected":true,"open_permits_connected":true,"read_ledger_permits_connected":true})
    })
}
fn resource_paths() -> Value {
    use typaxis_core::*;
    use typaxis_document::*;
    use typaxis_resources::*;
    use typaxis_shaping::book_v2::{
        with_book_v2_observed_source_accounting, BookV2ObservedAccountingError as Error,
    };
    #[derive(Default)]
    struct Trace(Vec<HostReadRequest>);
    impl HostReadBudget for Trace {
        type Error = std::convert::Infallible;
        const OBSERVE_ATTEMPTS: bool = true;
        const OBSERVE_DECLARATIONS: bool = true;
        const OBSERVE_RESOLUTION: bool = true;
        const OBSERVE_OPEN: bool = true;
        const OBSERVE_OPEN_IDENTITIES: bool = true;
        fn charge(&mut self, r: HostReadRequest) -> Result<(), Self::Error> {
            self.0.push(r);
            Ok(())
        }
    }
    let charge = |r: HostReadRequest| {
        [
            if r.stage==HostReadStage::OpenIdentityStorage{r.buffer_bytes/typaxis_resources::HOST_OPEN_IDENTITY_NODE_BYTES as u64}else{u64::from(
                matches!(
                    r.stage,
                    HostReadStage::Buffer
                        | HostReadStage::AttemptStorage
                        | HostReadStage::DeclarationUri
                        | HostReadStage::OpenOwner
                ) || (matches!(
                    r.stage,
                    HostReadStage::RevalidatePath | HostReadStage::OpenPath
                ) && r.buffer_bytes > 0),
            )},
            r.buffer_bytes,
            r.work,
        ]
    };
    fixture::hosted("no-page", |_, caps, maximum, _| {
        let root = std::env::temp_dir().join(format!("typaxis-book-paths-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let mut names = vec![String::from("resource.bin"), String::from("日本語/式.bin")];
        for n in [255usize, 256, 257, 1842] {
            let mut uri = String::new();
            let mut left = n;
            while left > 200 {
                uri.push_str(&"d".repeat(180));
                uri.push('/');
                left -= 181;
            }
            uri.push_str(&"f".repeat(left));
            names.push(uri);
        }
        let mut cases = Vec::new();
        let data = b"authentic host path read";
        for name in names {
            let path = root.join(&name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, data).unwrap();
            let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
                font_faces: vec![],
                images: vec![StagingM4ImageDeclaration {
                    image_id: ImageResourceId::new(0),
                    uri: PortablePath::new(name.clone()).unwrap(),
                    expected_sha256: Some(sha256(data)),
                    media: ImageMediaDeclaration::Declared(ImageMediaType::Png),
                    vector_provenance: None,
                }],
            })
            .unwrap();
            let config = EffectiveConfig::new(
                false,
                PdfStreamCompression::Flate,
                vec![ConfigResourceRoot::ProjectRoot],
                DEFAULT_ALLOWED_URI_SCHEMES
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect(),
                EffectiveDataVersions::new(
                    REGISTERED_UNICODE_VERSION,
                    REGISTERED_JAPANESE_LINE_BREAK_VERSION,
                )
                .unwrap(),
                caps.base().get().clone(),
            )
            .unwrap();
            let context = HostAdmissionContext::new(
                HostPath::new(root.join("input.typ")).unwrap(),
                HostPath::new(root.clone()).unwrap(),
                None,
                vec![],
            );
            let host = HostResourceAdmissionSession::new(
                &context,
                &config,
                declarations.resource_catalog(),
            )
            .unwrap();
            let mut native = StagingProductionResourceResolverV3::new(
                &declarations,
                caps,
                [177; 32],
                host.roots(),
            )
            .unwrap();
            let mut trace = Trace::default();
            let source = host
                .open_image_with_budget(ImageResourceId::new(0), &mut trace)
                .unwrap();
            let bytes = native.read_image_with_budget(source, &mut trace).unwrap();
            assert_eq!(bytes.bytes(), data);
            let run = |limited: &M4EffectiveResourceLimits, work: u64, cells: &mut [u64; 3]| {
                let mut resolver = StagingProductionResourceResolverV3::new(
                    &declarations,
                    limited,
                    [177; 32],
                    host.roots(),
                )
                .unwrap();
                let [r, s, w] = cells;
                with_book_v2_observed_source_accounting(
                    r,
                    s,
                    w,
                    limited.base(),
                    limited.base().get().max_fragments,
                    work,
                    |accounting| {
                        let mut budget = accounting.resource_admission_budget(NodeId::new(0));
                        let source =
                            host.open_image_with_budget(ImageResourceId::new(0), &mut budget)?;
                        resolver.read_image_with_budget(source, &mut budget)
                    },
                )
            };
            let prior = [17, 29, 41];
            let mut full = prior;
            assert_eq!(run(caps, maximum, &mut full).unwrap().bytes(), data);
            let mut expected = prior;
            for r in &trace.0 {
                for (v, n) in expected.iter_mut().zip(charge(*r)) {
                    *v += n;
                }
            }
            assert_eq!(full, expected);
            let mut prefix = prior;
            let mut cuts = Vec::new();
            for (at, request) in trace.0.iter().copied().enumerate() {
                for (axis, amount) in charge(request).into_iter().enumerate() {
                    if amount == 0 {
                        continue;
                    }
                    let cap = prefix[axis] + amount - 1;
                    let mut base = caps.base().get().clone();
                    if axis == 0 {
                        base.max_fragments = cap;
                    }
                    if axis == 1 {
                        base.max_spool_bytes = cap;
                    }
                    let base = ValidatedResourceLimits::new(base).unwrap();
                    let limited = M4EffectiveResourceLimits::defaults_for(&base);
                    let work = if axis == 2 { cap } else { maximum };
                    let mut cells = prior;
                    let ResourceReadError::Budget(f) = run(&limited, work, &mut cells).unwrap_err()
                    else {
                        panic!("expected typed read refusal")
                    };
                    assert_eq!(
                        f.subject,
                        typaxis_diagnostics::ResourceErrorSubject::Image(ImageResourceId::new(0))
                    );
                    assert_eq!(f.failure.request, request);
                    assert_eq!(cells, prefix);
                    match (axis, f.failure.error) {
                        (0, Error::Records { owner })
                        | (1, Error::Spool { owner })
                        | (2, Error::Work { owner }) => assert_eq!(owner.get(), 0),
                        _ => panic!("wrong owner/axis"),
                    }
                    assert!(run(&limited, work, &mut cells).is_err());
                    assert!(cells.iter().zip(prefix).all(|(a, b)| *a >= b));
                    assert!(cells[axis] <= cap);
                    cuts.push(json!({"request":at,"axis":axis,"stage":format!("{:?}",request.stage),"prefix":prefix,"cap":cap,"retry":cells}));
                }
                for (v, n) in prefix.iter_mut().zip(charge(request)) {
                    *v += n;
                }
            }
            assert_eq!(prefix, full);
            cases.push(json!({"uri":name,"uri_bytes":name.len(),"components":name.split('/').count(),"body_bytes":data.len(),"prior":prior,"full":full,"requests":trace.0.iter().map(|r|json!({"stage":format!("{:?}",r.stage),"file_offset":r.file_offset,"buffer_bytes":r.buffer_bytes,"work":r.work})).collect::<Vec<_>>(),"typed_cuts":cuts,"quota_owner":0,"partial_receipts_published":0}));
        }
        std::fs::remove_dir_all(root).unwrap();
        json!({"cases":cases,"resolution_permits_connected":true,"open_permits_connected":true,"scope":"authentic resource reads using Book Source cells; parser and ledger publication excluded"})
    })
}

fn owned_source() -> Value {
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let font = original_font();
        let bytes = font.clone();
        let pointer = bytes.as_ptr();
        let native = typaxis_font::admit_sfnt_cff1_v2_owned(bytes, 0, caps).unwrap();
        assert_eq!(native.source().as_ptr(), pointer);
        let sliced =
            typaxis_font::admit_sfnt_cff1_v2(std::sync::Arc::from(font.as_slice()), 0, caps)
                .unwrap();
        assert_eq!(native.fingerprint(), sliced.fingerprint());
        let mut style = [0; 3];
        let mut ledger_reference = None;
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            let body = prepare_admitted_book_v2_body_with_counted_style(
                input,
                caps.base(),
                &mut |r, s, w| {
                    for (cell, n) in style.iter_mut().zip([r, s, w]) {
                        *cell += n;
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
            ledger_reference = Some(
                crate::book_v2_resources::prepare_book_v2_resources(body, context, config, caps)
                    .unwrap(),
            );
        });
        let [mut r, mut s, mut w] = style;
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            caps.base().get().max_fragments,
            maximum,
            |a| {
                fund_resource_catalog_reference(a, original.resources(), None);
                fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                fund_resource_read_reference(
                    a,
                    font.len(),
                    original.resources().fonts()[0].uri().as_str(),
                    original.resources().fonts()[0].family(), 2 + original.resources().fonts().len() + original.resources().images().len(), true);
                fund_resource_media_reference(a, &font, original.resources());
            },
        );
        let prefix = [r, s, w];
        let owner_bytes =
            (std::mem::size_of::<Vec<u8>>() + 2 * std::mem::size_of::<usize>()) as u64;
        let mut cuts = Vec::new();
        for axis in 0..3 {
            let mut expected = prefix;
            if axis != 2 {
                expected[2] += 64;
            }
            let cap = expected[axis] + [1, owner_bytes, 64][axis] - 1;
            let mut base = caps.base().get().clone();
            if axis == 0 {
                base.max_fragments = cap;
            }
            if axis == 1 {
                base.max_spool_bytes = cap;
            }
            let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
            let limited = M4EffectiveResourceLimits::defaults_for(&base);
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                &limited,
                if axis == 2 { cap } else { maximum },
            );
            hosted_with_cff(
                original,
                &limited,
                Some(&font),
                |input, context, config, _| {
                    let error = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                    .unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Admission { failure, .. } = error
                    else {
                        panic!("wrong refusal: {error:?}")
                    };
                    assert_eq!(failure.font_face_id.get(), 0);
                    let typaxis_font::CffAdmissionBudgetFailureV2::Metadata(f) = failure.failure
                    else {
                        panic!()
                    };
                    assert_eq!(f.point.stage, typaxis_font::CffMetadataStageV2::SourceOwner);
                    assert_eq!(
                        (f.point.table_tag, f.point.file_offset, f.point.table_offset),
                        (None, None, None)
                    );
                    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                    let owner = match (axis, f.error) {
                        (0, E::Records { owner })
                        | (1, E::Spool { owner })
                        | (2, E::Work { owner }) => owner,
                        _ => panic!("wrong axis"),
                    };
                    assert_eq!(owner.get(), 0);
                },
            );
            assert_eq!(rsw(budget.observation()), expected);
            assert_eq!(budget.observation().output_charge(), 0);
            hosted_with_cff(
                original,
                &limited,
                Some(&font),
                |input, context, config, _| {
                    assert!(prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget
                    )
                    .is_err());
                },
            );
            let retry = rsw(budget.observation());
            assert!(retry.iter().zip(expected).all(|(a, b)| *a >= b));
            assert!(retry[axis] <= cap);
            cuts.push(json!({"axis":axis,"prefix":expected,"cap":cap,"retry":retry}));
        }
        let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        hosted_with_cff(original, caps, Some(&font), |input, context, config, _| {
            prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                .unwrap();
        });
        let full = rsw(warm.observation());
        let mut ledger_prefix = [205023, 7939405 + owner_bytes, 17341638];
        for (v, n) in ledger_prefix
            .iter_mut()
            .zip(resource_attempt_reference(original.resources()))
        {
            *v += n;
        }
        let [mut r, mut s, mut w] = ledger_prefix;
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            caps.base().get().max_fragments,
            maximum,
            |a| {
                fund_resource_media_reference(a, &font, original.resources());
                fund_resource_ledger_reference(a, ledger_reference.as_ref().unwrap().resources());
                fund_read_ledger_reference(a, ledger_reference.as_ref().unwrap());
            },
        );
        assert_eq!(full, [r, s, w]);
        for axis in 0..3 {
            let mut base = caps.base().get().clone();
            if axis == 0 {
                base.max_fragments = full[0];
            }
            if axis == 1 {
                base.max_spool_bytes = full[1];
            }
            let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
            let limited = M4EffectiveResourceLimits::defaults_for(&base);
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                &limited,
                if axis == 2 { full[2] } else { maximum },
            );
            hosted_with_cff(
                original,
                &limited,
                Some(&font),
                |input, context, config, _| {
                    prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                    .unwrap();
                },
            );
            assert_eq!(rsw(budget.observation()), full);
        }
        json!({"zero_copy_original_buffer":true,"original_font_sha256":hex(&native.source_sha256()),"same_admission_fingerprint":true,"owner_bytes":owner_bytes,"owner_work":64,"source_read_prefix":prefix,"full":full,"typed_cuts":cuts,"exact_axes":3,"quota_owner":0})
    })
}
fn ledger_limits() -> Value {
    use typaxis_resources::ProductionResourceLedgerStageV3 as Stage;
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let font = original_font();
        let mut cases = Vec::new();
        for cff in [false, true] {
            let mut prepared = None;
            hosted_with_cff(
                original,
                caps,
                if cff { Some(&font) } else { None },
                |input, context, config, _| {
                    let body = prepare_admitted_book_v2_body_with_counted_style(
                        input,
                        caps.base(),
                        &mut |_, _, _| Ok::<_, std::convert::Infallible>(()),
                    )
                    .unwrap();
                    prepared = Some(
                        crate::book_v2_resources::prepare_book_v2_resources(
                            body, context, config, caps,
                        )
                        .unwrap(),
                    );
                },
            );
            let prepared = prepared.unwrap();
            let requests = resource_ledger_reference_requests(prepared.resources());
            let mut prefix = if cff {
                [205023, 7939445, 17341638]
            } else {
                [268, 17116, 81926]
            };
            if cff { prefix[2] += 64 * (u64::from(u16::from_be_bytes(font[4..6].try_into().unwrap())) + 1); }
            for (v, n) in prefix
                .iter_mut()
                .zip(resource_attempt_reference(original.resources()))
            {
                *v += n;
            }
            let mut cuts = Vec::new();
            let mut tested_hash = false;
            for (index, (point, cells)) in requests.iter().enumerate() {
                let selected = !cff
                    || point.stage == Stage::FamilyEntries
                    || (point.font_face_id == Some(typaxis_core::FontFaceId::new(0))
                        && matches!(
                            point.stage,
                            Stage::FamilyValidate
                                | Stage::FamilyLookup
                                | Stage::FamilyEntries
                                | Stage::FamilyNameCopy
                                | Stage::FamilyInsert
                        ))
                    || point.stage == Stage::FontConflicts
                    || (point.stage == Stage::Fonts && cells[0] > 0)
                    || point.stage == Stage::CanonicalFinish
                    || (point.stage == Stage::CanonicalHash && !tested_hash);
                if selected {
                    if point.stage == Stage::CanonicalHash {
                        tested_hash = true;
                    }
                    for axis in 0..3 {
                        if cells[axis] == 0 {
                            continue;
                        }
                        let cap = prefix[axis] + cells[axis] - 1;
                        let mut base = caps.base().get().clone();
                        if axis == 0 {
                            base.max_fragments = cap;
                        }
                        if axis == 1 {
                            base.max_spool_bytes = cap;
                        }
                        let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                        let limited = M4EffectiveResourceLimits::defaults_for(&base);
                        let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                            &limited,
                            if axis == 2 { cap } else { maximum },
                        );
                        hosted_with_cff(
                            original,
                            &limited,
                            if cff { Some(&font) } else { None },
                            |input, context, config, _| {
                                let error = prepare_counted_book_v2_column_resources(
                                    input,
                                    context,
                                    config,
                                    &limited,
                                    &mut budget,
                                )
                                .unwrap_err();
                                let BookV2ColumnInitialStylePreparationError::Ledger {
                                    failure,
                                    body,
                                } = error
                                else {
                                    panic!("expected ledger refusal: {error:?}")
                                };
                                assert_eq!(body.styled().body().limits(), limited.base());
                                assert_eq!(failure.point, *point);
                                use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                                let owner = match (axis, failure.error) {
                                    (0, E::Records { owner })
                                    | (1, E::Spool { owner })
                                    | (2, E::Work { owner }) => owner,
                                    _ => panic!("wrong axis"),
                                };
                                assert_eq!(owner.get(), 0);
                            },
                        );
                        assert_eq!(rsw(budget.observation()), prefix);
                        assert_eq!(budget.observation().output_charge(), 0);
                        hosted_with_cff(
                            original,
                            &limited,
                            if cff { Some(&font) } else { None },
                            |input, context, config, _| {
                                assert!(prepare_counted_book_v2_column_resources(
                                    input,
                                    context,
                                    config,
                                    &limited,
                                    &mut budget
                                )
                                .is_err());
                            },
                        );
                        let retry = rsw(budget.observation());
                        assert!(retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                        assert!(retry[axis] <= cap);
                        cuts.push(json!({"request":index,"stage":format!("{:?}",point.stage),"font":point.font_face_id.map(|i|i.get()),"image":point.image_id.map(|i|i.get()),"axis":axis,"prefix":prefix,"cap":cap,"retry":retry}));
                    }
                }
                for (cell, n) in prefix.iter_mut().zip(cells) {
                    *cell += n;
                }
            }
            for request in read_ledger_reference_requests(&prepared) {
                for (cell, amount) in prefix.iter_mut().zip(read_ledger_request_cells(request)) {
                    *cell += amount;
                }
            }
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
            hosted_with_cff(
                original,
                caps,
                if cff { Some(&font) } else { None },
                |input, context, config, _| {
                    let actual = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        caps,
                        &mut budget,
                    )
                    .unwrap();
                    assert_eq!(
                        actual.resources().canonical_jcs(),
                        prepared.resources().canonical_jcs()
                    );
                    assert_eq!(
                        actual.resources().fingerprint(),
                        prepared.resources().fingerprint()
                    );
                },
            );
            assert_eq!(rsw(budget.observation()), prefix);
            cases.push(json!({"cff":cff,"requests":requests.len(),"full":prefix,"typed_cuts":cuts,"canonical_bytes":prepared.resources().canonical_jcs().len(),"canonical_jcs":prepared.resources().canonical_jcs(),"canonical_sha256":hex(&prepared.resources().fingerprint())}));
        }
        json!({"cases":cases,"quota_owner":0,"partial_ledgers_published":0,"whole_finalization_accounting_complete":false})
    })
}
fn svg_ir_reference_requests(
    svg: &[u8],
    count: u32,
) -> Vec<typaxis_resources::SafeSvgSourceRequest> {
    use typaxis_core::*;
    use typaxis_document::{
        ImageMediaDeclaration, ImageMediaType, StagingM4ImageDeclaration, StagingM4ResourceCatalog,
    };
    use typaxis_resources::*;
    #[derive(Default)]
    struct Trace(Vec<SafeSvgSourceRequest>);
    impl SafeSvgSourceBudget for Trace {
        type Error = std::convert::Infallible;
        const OBSERVE_IR: bool = true;
        const OBSERVE_BINDING: bool = true;
        fn work(&mut self, r: SafeSvgSourceRequest) -> Result<(), Self::Error> {
            self.0.push(r);
            Ok(())
        }
    }
    let token = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "typaxis-svg-ir-reference-{}-{token}",
        std::process::id()
    ));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("formula.svg"), svg).unwrap();
    let config = EffectiveConfig::new(
        false,
        PdfStreamCompression::Flate,
        vec![ConfigResourceRoot::ProjectRoot],
        DEFAULT_ALLOWED_URI_SCHEMES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        EffectiveDataVersions::new(
            REGISTERED_UNICODE_VERSION,
            REGISTERED_JAPANESE_LINE_BREAK_VERSION,
        )
        .unwrap(),
        ResourceLimits::default(),
    )
    .unwrap();
    let caps = M4EffectiveResourceLimits::defaults_for(config.limits());
    let declarations = staging_declared_base_catalog(&StagingM4ResourceCatalog {
        font_faces: vec![],
        images: (0..count)
            .map(|id| StagingM4ImageDeclaration {
                image_id: ImageResourceId::new(id),
                uri: PortablePath::new("formula.svg").unwrap(),
                expected_sha256: Some(sha256(svg)),
                media: ImageMediaDeclaration::Declared(ImageMediaType::SvgSafe2),
                vector_provenance: None,
            })
            .collect(),
    })
    .unwrap();
    let context = HostAdmissionContext::new(
        HostPath::new(root.join("input.typ")).unwrap(),
        HostPath::new(root.clone()).unwrap(),
        None,
        vec![],
    );
    let host =
        HostResourceAdmissionSession::new(&context, &config, declarations.resource_catalog())
            .unwrap();
    let mut resolver =
        StagingProductionResourceResolverV3::new(&declarations, &caps, [111; 32], host.roots())
            .unwrap();
    let mut trace = Trace::default();
    for id in 0..count {
        let pending = resolver
            .read_image(host.open_image(ImageResourceId::new(id)).unwrap())
            .unwrap();
        resolver
            .parse_and_bind_image_with_source_budget(pending, &mut trace)
            .unwrap();
    }
    assert_eq!(resolver.finish().unwrap().images().len(), count as usize);
    std::fs::remove_dir_all(root).unwrap();
    trace.0
}

fn vector_alias_limits() -> Value {
    use typaxis_resources::ProductionResourceLedgerStageV3 as Stage;
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let svg=std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg")).unwrap();
        let names = (0..9)
            .map(|i| format!("formula-{i}.svg"))
            .collect::<Vec<_>>();
        let images = names
            .iter()
            .map(|n| (n.as_str(), svg.as_slice()))
            .collect::<Vec<_>>();
        let prepare = |limited: &M4EffectiveResourceLimits, work: u64| {
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(limited, work);
            let result = hosted_with_resources(
                original,
                limited,
                None,
                Some(&images),
                |input, context, config, _| {
                    prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        limited,
                        &mut budget,
                    )
                },
            );
            (result, budget.observation())
        };
        let (full, observation) = prepare(caps, maximum);
        let full = full.unwrap();
        let native = hosted_with_resources(
            original,
            caps,
            None,
            Some(&images),
            |input, context, config, _| {
                let body = prepare_admitted_book_v2_body_with_counted_style(
                    input,
                    caps.base(),
                    &mut |_, _, _| Ok::<_, std::convert::Infallible>(()),
                )
                .unwrap();
                crate::book_v2_resources::prepare_book_v2_resources(body, context, config, caps)
                    .unwrap()
            },
        );
        assert_eq!(
            full.resources().canonical_jcs(),
            native.resources().canonical_jcs()
        );
        assert_eq!(
            full.resources().fingerprint(),
            native.resources().fingerprint()
        );
        assert_eq!(full.resources().images().len(), 9);
        assert!(full
            .resources()
            .images()
            .iter()
            .all(|i| i.bytes() == svg && i.admitted_safe_vector().is_some()));
        let mut style = [0; 3];
        hosted_with_resources(original, caps, None, Some(&images), |input, _, _, _| {
            prepare_admitted_book_v2_body_with_counted_style(input, caps.base(), &mut |r, s, w| {
                for (v, n) in style.iter_mut().zip([r, s, w]) {
                    *v += n;
                }
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        });
        let [mut r, mut s, mut w] = style;
        let image_requests = svg_ir_reference_requests(&svg, 9);
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            caps.base().get().max_fragments,
            maximum,
            |a| {
                fund_resource_catalog_reference(a, original.resources(), Some(&names));
                fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                for f in original.resources().fonts() {
                    fund_resource_read_reference(a, f.bytes().len(), f.uri().as_str(), f.family(), 2 + original.resources().fonts().len() + names.len(), f.font_face_id().get()==0);
                }
                fund_resource_attempt_storage_reference(a, 9);
                for image in 0..9 {
                    fund_resource_read_reference(a, svg.len(), &names[image as usize], "", 2 + original.resources().fonts().len() + names.len(), false);
                    for request in image_requests
                        .iter()
                        .copied()
                        .filter(|r| r.image_id.get() == image)
                    {
                        a.debit(
                            typaxis_core::NodeId::new(0),
                            [request.records, request.storage_bytes, request.work],
                        )
                        .unwrap();
                    }
                }
                a.debit_work(typaxis_core::NodeId::new(0), 9 * 64).unwrap();
            },
        );
        let before = [r, s, w];
        let mut cuts = Vec::new();
        for axis in [0, 1] {
            let fee = if axis == 0 {
                9
            } else {
                9 * std::mem::size_of::<([u8; 32], &[u8], typaxis_core::ImageResourceId)>() as u64
            };
            let cap = before[axis] + fee - 1;
            let mut base = caps.base().get().clone();
            if axis == 0 {
                base.max_fragments = cap;
            } else {
                base.max_spool_bytes = cap;
            }
            let limited = M4EffectiveResourceLimits::defaults_for(
                &typaxis_core::ValidatedResourceLimits::new(base).unwrap(),
            );
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(&limited, maximum);
            let error = hosted_with_resources(
                original,
                &limited,
                None,
                Some(&images),
                |input, context, config, _| {
                    prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                },
            );
            let o = budget.observation();
            let BookV2ColumnInitialStylePreparationError::Ledger { failure, .. } =
                error.unwrap_err()
            else {
                panic!("wrong alias storage refusal")
            };
            assert_eq!(failure.point.stage, Stage::Aliases);
            assert_eq!(failure.point.image_id, None);
            assert_eq!(rsw(o), before);
            assert_eq!(o.output_charge(), 0);
            use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
            assert!(
                matches!((axis,failure.error),(0,E::Records{owner})|(1,E::Spool{owner}) if owner.get()==0)
            );
            assert!(hosted_with_resources(
                original,
                &limited,
                None,
                Some(&images),
                |input, context, config, _| prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    &limited,
                    &mut budget
                )
            )
            .is_err());
            let retry = rsw(budget.observation());
            assert!(retry[axis] <= cap && retry.iter().zip(before).all(|(a, b)| *a >= b));
            cuts.push(json!({"axis":axis,"stage":"Aliases","image":null,"prefix":rsw(o),"cap":cap,"retry":retry}));
        }
        // Locate the sort/comparison boundary through the real CLI. This does
        // not duplicate the heap algorithm or assume its operation count.
        let mut low = before[2] + 9 * 64;
        let mut high = rsw(observation)[2];
        while low < high {
            let mid = low + (high - low) / 2;
            let (result, _) = prepare(caps, mid);
            let after_sort = match result {
                Ok(_) => true,
                Err(BookV2ColumnInitialStylePreparationError::Ledger { failure, .. }) => !matches!(
                    failure.point.stage,
                    Stage::Aliases | Stage::AliasOrder | Stage::ImageIds
                ),
                Err(error) => panic!("unexpected boundary {error:?}"),
            };
            if after_sort {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let first_order = before[2] + 9 * 64;
        let first_bytes = low + 64;
        assert!(svg.len() > 256);
        for (stage, cap) in [
            (Stage::Aliases, before[2] + 63),
            (Stage::AliasOrder, first_order + 63),
            (Stage::AliasCompare, first_bytes + 575),
        ] {
            let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, cap);
            let error = hosted_with_resources(
                original,
                caps,
                None,
                Some(&images),
                |input, context, config, _| {
                    prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        caps,
                        &mut budget,
                    )
                },
            )
            .unwrap_err();
            let BookV2ColumnInitialStylePreparationError::Ledger { failure, .. } = error else {
                panic!("wrong alias work refusal")
            };
            assert_eq!(failure.point.stage, stage);
            assert!(failure.point.image_id.is_some_and(|i| i.get() < 9));
            use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
            assert!(matches!(failure.error,E::Work{owner} if owner.get()==0));
            let prefix = rsw(budget.observation());
            assert!(prefix[2] <= cap);
            assert_eq!(budget.observation().output_charge(), 0);
            assert!(hosted_with_resources(
                original,
                caps,
                None,
                Some(&images),
                |input, context, config, _| prepare_counted_book_v2_column_resources(
                    input,
                    context,
                    config,
                    caps,
                    &mut budget
                )
            )
            .is_err());
            let retry = rsw(budget.observation());
            assert!(retry[2] <= cap && retry.iter().zip(prefix).all(|(a, b)| *a >= b));
            cuts.push(json!({"axis":2,"stage":format!("{stage:?}"),"image":failure.point.image_id.map(|i|i.get()),"prefix":prefix,"cap":cap,"retry":retry}));
        }
        json!({"images":9,"svg_bytes":svg.len(),"svg_sha256":hex(&typaxis_core::sha256(&svg)),"full":rsw(observation),"before_aliases":before,"typed_cuts":cuts,"quota_owner":0,"partial_ledgers_published":0,"canonical_jcs":full.resources().canonical_jcs(),"canonical_sha256":hex(&full.resources().fingerprint()),"whole_finalization_accounting_complete":false})
    })
}
fn svg_source_limits(with_clip: bool) -> Value {
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let svg = if with_clip {
            br#"<svg xmlns="http://www.w3.org/2000/svg" width="10pt" height="10pt" viewBox="0 0 10 10"><defs><clipPath id="c"><rect width="5" height="5" /></clipPath></defs><g clip-path="url(#c)"><path d="M 1 1 L 4 1 L 4 4 Z" clip-path="url(#c)" /></g></svg>"#.to_vec()
        } else {
            std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"),"/../../../samples/machine-package/staging/production-book-1/precomposed-vector/svg/fraction-equality.svg")).unwrap()
        };
        let names = (0..9)
            .map(|i| format!("source-{i}.svg"))
            .collect::<Vec<_>>();
        let images = names
            .iter()
            .map(|n| (n.as_str(), svg.as_slice()))
            .collect::<Vec<_>>();
        let mut style = [0; 3];
        hosted_with_resources(original, caps, None, Some(&images), |input, _, _, _| {
            prepare_admitted_book_v2_body_with_counted_style(input, caps.base(), &mut |r, s, w| {
                for (v, n) in style.iter_mut().zip([r, s, w]) {
                    *v += n;
                }
                Ok::<_, std::convert::Infallible>(())
            })
            .unwrap();
        });
        let [mut r, mut s, mut w] = style;
        let image_requests = svg_ir_reference_requests(&svg, 9);
        let mut requests = Vec::new();
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(
            &mut r,
            &mut s,
            &mut w,
            caps.base(),
            caps.base().get().max_fragments,
            maximum,
            |a| {
                fund_resource_catalog_reference(a, original.resources(), Some(&names));
                fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                for font in original.resources().fonts() {
                    fund_resource_read_reference(
                        a,
                        font.bytes().len(),
                        font.uri().as_str(),
                        font.family(), 2 + original.resources().fonts().len() + names.len(), false);
                }
                fund_resource_attempt_storage_reference(a, 9);
                for image in 0..9 {
                    fund_resource_read_reference(a, svg.len(), &names[image as usize], "", 2 + original.resources().fonts().len() + names.len(), false);
                    for request in image_requests
                        .iter()
                        .copied()
                        .filter(|r| r.image_id.get() == image)
                    {
                        requests.push((request, a.counters()));
                        a.debit(
                            typaxis_core::NodeId::new(0),
                            [request.records, request.storage_bytes, request.work],
                        )
                        .unwrap();
                    }
                }
            },
        );
        let mut cuts = Vec::new();
        let mut selected = std::collections::BTreeSet::new();
        for (at, (request, prefix)) in requests.iter().enumerate() {
            use typaxis_resources::SafeSvgSourceStage as S;
            let stage = format!("{:?}", request.stage);
            let first = selected.insert((request.image_id.get(), stage));
            let last = requests.get(at + 1).map_or(true, |(next, _)| {
                next.image_id != request.image_id || next.stage != request.stage
            });
            let old = matches!(
                request.stage,
                S::Validation
                    | S::Count
                    | S::CountFinish
                    | S::ClipStorage
                    | S::ClipCollect
                    | S::ClipOrder
                    | S::ClipCompare
                    | S::ClipLookup
                    | S::ClipReplay
                    | S::ClipFinish
            );
            if !(old
                || request.stage == S::IrFinish
                || ([0, 8].contains(&request.image_id.get()) && (first || last)))
            {
                continue;
            }

            let prefix = *prefix;
            let cells = [request.records, request.storage_bytes, request.work];
            for axis in 0..3 {
                if cells[axis] == 0 {
                    continue;
                }
                let cap = prefix[axis] + cells[axis] - 1;
                let mut base = caps.base().get().clone();
                if axis == 0 {
                    base.max_fragments = cap;
                }
                if axis == 1 {
                    base.max_spool_bytes = cap;
                }
                let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                let limited = M4EffectiveResourceLimits::defaults_for(&base);
                let caps = &limited;
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                    caps,
                    if axis == 2 { cap } else { maximum },
                );
                let error = hosted_with_resources(
                    original,
                    caps,
                    None,
                    Some(&images),
                    |input, context, config, _| {
                        prepare_counted_book_v2_column_resources(
                            input,
                            context,
                            config,
                            caps,
                            &mut budget,
                        )
                    },
                )
                .unwrap_err();
                let BookV2ColumnInitialStylePreparationError::SvgSource { failure, .. } = error
                else {
                    panic!("expected SVG source refusal")
                };
                assert_eq!(failure.request, *request);
                use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                assert!(match (axis, failure.error) {
                    (0, E::Records { owner })
                    | (1, E::Spool { owner })
                    | (2, E::Work { owner }) => owner.get() == 0,
                    _ => false,
                });
                assert_eq!(rsw(budget.observation()), prefix);
                assert_eq!(budget.observation().output_charge(), 0);
                assert!(hosted_with_resources(
                    original,
                    caps,
                    None,
                    Some(&images),
                    |input, context, config, _| prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        caps,
                        &mut budget
                    )
                )
                .is_err());
                let retry = rsw(budget.observation());
                assert!(retry[axis] <= cap && retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                cuts.push(json!({"axis":axis,"stage":format!("{:?}",request.stage),"image":request.image_id.get(),"byte_offset":request.byte_offset,"byte_length":request.byte_length,"records":request.records,"storage_bytes":request.storage_bytes,"work":request.work,"prefix":prefix,"cap":cap,"retry":retry}));
            }
        }
        let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        let observed = hosted_with_resources(
            original,
            caps,
            None,
            Some(&images),
            |input, context, config, _| {
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut budget)
            },
        )
        .unwrap();
        let native = hosted_with_resources(
            original,
            caps,
            None,
            Some(&images),
            |input, context, config, _| {
                let body = prepare_admitted_book_v2_body_with_counted_style(
                    input,
                    caps.base(),
                    &mut |_, _, _| Ok::<_, std::convert::Infallible>(()),
                )
                .unwrap();
                prepare_book_v2_resources(body, context, config, caps).unwrap()
            },
        );
        assert_eq!(
            observed.resources().canonical_jcs(),
            native.resources().canonical_jcs()
        );
        assert_eq!(
            observed.resources().fingerprint(),
            native.resources().fingerprint()
        );
        json!({"images":9,"svg_bytes":svg.len(),"svg_sha256":hex(&typaxis_core::sha256(&svg)),"requests":requests.len(),"source_work":requests.iter().map(|(r,_)|r.work).sum::<u64>(),"typed_cuts":cuts,"full":rsw(budget.observation()),"quota_owner":0,"partial_ledgers_published":0,"count_pass_connected":true,"clip_closure_connected":true,"with_clip":with_clip,"decoder_ir_permits_connected":true,"binding_permits_connected":true,"reference_kind":"public resource-set /3 SVG permit trace","reference_requests":requests.iter().map(|(r,p)|json!({"image":r.image_id.get(),"stage":format!("{:?}",r.stage),"byte_offset":r.byte_offset,"byte_length":r.byte_length,"records":r.records,"storage_bytes":r.storage_bytes,"work":r.work,"prefix":p})).collect::<Vec<_>>(),"geometry_and_ir_accounting_complete":false})
    })
}
fn resource_catalog_limits() -> Value {
    use typaxis_resources::{ResourceCatalogBudget, ResourceCatalogRequest};
    #[derive(Default)]
    struct Trace(Vec<ResourceCatalogRequest>);
    impl ResourceCatalogBudget for Trace {
        type Error = std::convert::Infallible;
        fn charge(&mut self, r: ResourceCatalogRequest) -> Result<(), Self::Error> {
            self.0.push(r);
            Ok(())
        }
    }
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let expected = resource_catalog_reference_requests(original.resources(), None);
        let mut style = [0; 3];
        let mut trace = Trace::default();
        let mut prior_attempts = 0;
        hosted(original, caps, |input, _, _, _| {
            prior_attempts = input
                .provenance()
                .read_ledger_token()
                .unwrap()
                .candidate_attempt_count();
            let body = prepare_admitted_book_v2_body_with_counted_style(
                input,
                caps.base(),
                &mut |r, s, w| {
                    for (v, n) in style.iter_mut().zip([r, s, w]) {
                        *v += n;
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
            let native =
                typaxis_resources::staging_declared_base_catalog(body.styled().body().resources())
                    .unwrap();
            let observed = typaxis_resources::staging_declared_base_catalog_with_budget(
                body.styled().body().resources(),
                &mut trace,
            )
            .unwrap();
            assert_eq!(observed, native);
        });
        assert_eq!(trace.0, expected);
        let mut prefix = style;
        let mut cuts = Vec::new();
        for (at, request) in expected.iter().copied().enumerate() {
            let cells = [request.records, request.storage_bytes, request.work];
            for axis in 0..3 {
                if cells[axis] == 0 {
                    continue;
                }
                let cap = prefix[axis] + cells[axis] - 1;
                let mut base = caps.base().get().clone();
                if axis == 0 {
                    base.max_fragments = cap;
                }
                if axis == 1 {
                    base.max_spool_bytes = cap;
                }
                let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                let limited = M4EffectiveResourceLimits::defaults_for(&base);
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                    &limited,
                    if axis == 2 { cap } else { maximum },
                );
                hosted(original, &limited, |input, context, config, _| {
                    let error = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                    .unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Catalog { body, failure } = error
                    else {
                        panic!("expected catalog refusal: {error:?}")
                    };
                    assert_eq!(failure.request, request);
                    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                    let owner = match (axis, failure.error) {
                        (0, E::Records { owner })
                        | (1, E::Spool { owner })
                        | (2, E::Work { owner }) => owner,
                        _ => panic!("wrong axis"),
                    };
                    assert_eq!(owner.get(), 0);
                    assert_eq!(body.styled().body().limits(), limited.base());
                    assert_eq!(
                        body.provenance()
                            .read_ledger_token()
                            .unwrap()
                            .candidate_attempt_count(),
                        prior_attempts
                    );
                    body.provenance()
                        .read_ledger_token()
                        .unwrap()
                        .revalidate()
                        .unwrap();
                });
                assert_eq!(rsw(budget.observation()), prefix);
                assert_eq!(budget.observation().output_charge(), 0);
                hosted(original, &limited, |input, context, config, _| {
                    assert!(prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget
                    )
                    .is_err());
                });
                let retry = rsw(budget.observation());
                assert!(retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                assert!(retry[axis] <= cap);
                cuts.push(
                    json!({"request":at,"axis":axis,"prefix":prefix,"cap":cap,"retry":retry}),
                );
            }
            for (v, n) in prefix.iter_mut().zip(cells) {
                *v += n;
            }
        }
        let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        hosted(original, caps, |input, context, config, _| {
            let prepared =
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            assert_eq!(
                prepared.resources().canonical_jcs(),
                original.resources().canonical_jcs()
            );
        });
        json!({"style":style,"after_catalog":prefix,"catalog_fee":resource_catalog_reference_fee(original.resources(),None),"requests":expected.iter().map(|r|json!({"stage":format!("{:?}",r.stage),"font":r.font_face_id.map(|v|v.get()),"image":r.image_id.map(|v|v.get()),"records":r.records,"storage_bytes":r.storage_bytes,"work":r.work})).collect::<Vec<_>>(),"typed_cuts":cuts,"full":rsw(warm.observation()),"quota_owner":0,"partial_catalogs_published":0,"resource_candidates_registered_before_catalog_failure":0})
    })
}

fn resource_resolver_limits() -> Value {
    use typaxis_resources::{ResourceCatalogBudget, ResourceCatalogRequest};
    #[derive(Default)]
    struct Trace(Vec<ResourceCatalogRequest>);
    impl ResourceCatalogBudget for Trace {
        type Error = std::convert::Infallible;
        fn charge(&mut self, r: ResourceCatalogRequest) -> Result<(), Self::Error> {
            self.0.push(r);
            Ok(())
        }
    }
    fixture::hosted("no-page", |original, caps, maximum, _| {
        let expected = resource_resolver_reference_requests(original.resources(), None);
        let mut style = [0; 3];
        let mut trace = Trace::default();
        let mut prior_attempts = 0;
        let mut prior_opened = 0;
        hosted(original, caps, |input, context, config, _| {
            prior_attempts = input
                .provenance()
                .read_ledger_token()
                .unwrap()
                .candidate_attempt_count();
            prior_opened = input
                .provenance()
                .read_ledger_token()
                .unwrap()
                .stored_opened_identity_count();
            let body = prepare_admitted_book_v2_body_with_counted_style(
                input,
                caps.base(),
                &mut |r, s, w| {
                    for (v, n) in style.iter_mut().zip([r, s, w]) {
                        *v += n;
                    }
                    Ok::<_, std::convert::Infallible>(())
                },
            )
            .unwrap();
            let declared =
                typaxis_resources::staging_declared_base_catalog(body.styled().body().resources())
                    .unwrap();
            let host=typaxis_resources::HostResourceAdmissionSession::new_with_declared_catalog_and_read_ledger(context,config,&declared,body.provenance().read_ledger()).unwrap();
            typaxis_resources::StagingProductionResourceResolverV3::new_with_catalog_budget(
                &declared,
                caps,
                [99; 32],
                host.roots(),
                &mut trace,
            )
            .unwrap();
        });
        assert_eq!(trace.0, expected);
        let mut prefix = style;
        for (v, n) in prefix
            .iter_mut()
            .zip(resource_catalog_reference_fee(original.resources(), None))
        {
            *v += n;
        }
        let mut cuts = Vec::new();
        for (at, request) in expected.iter().copied().enumerate() {
            let cells = [request.records, request.storage_bytes, request.work];
            for axis in 0..3 {
                if cells[axis] == 0 {
                    continue;
                }
                let cap = prefix[axis] + cells[axis] - 1;
                let mut base = caps.base().get().clone();
                if axis == 0 {
                    base.max_fragments = cap;
                }
                if axis == 1 {
                    base.max_spool_bytes = cap;
                }
                let base = typaxis_core::ValidatedResourceLimits::new(base).unwrap();
                let limited = M4EffectiveResourceLimits::defaults_for(&base);
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(
                    &limited,
                    if axis == 2 { cap } else { maximum },
                );
                hosted(original, &limited, |input, context, config, _| {
                    let error = prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget,
                    )
                    .unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Catalog { body, failure } = error
                    else {
                        panic!("expected catalog refusal: {error:?}")
                    };
                    assert_eq!(failure.request, request);
                    use typaxis_shaping::book_v2::BookV2ObservedAccountingError as E;
                    let owner = match (axis, failure.error) {
                        (0, E::Records { owner })
                        | (1, E::Spool { owner })
                        | (2, E::Work { owner }) => owner,
                        _ => panic!("wrong axis"),
                    };
                    assert_eq!(owner.get(), 0);
                    assert_eq!(body.styled().body().limits(), limited.base());
                    assert_eq!(
                        body.provenance()
                            .read_ledger_token()
                            .unwrap()
                            .candidate_attempt_count(),
                        prior_attempts
                            + original.resources().fonts().len()
                            + original.resources().images().len()
                    );
                    assert_eq!(
                        body.provenance()
                            .read_ledger_token()
                            .unwrap()
                            .stored_opened_identity_count(),
                        prior_opened
                    );
                    body.provenance()
                        .read_ledger_token()
                        .unwrap()
                        .revalidate()
                        .unwrap();
                });
                assert_eq!(rsw(budget.observation()), prefix);
                assert_eq!(budget.observation().output_charge(), 0);
                hosted(original, &limited, |input, context, config, _| {
                    assert!(prepare_counted_book_v2_column_resources(
                        input,
                        context,
                        config,
                        &limited,
                        &mut budget
                    )
                    .is_err());
                });
                let retry = rsw(budget.observation());
                assert!(retry.iter().zip(prefix).all(|(a, b)| *a >= b));
                assert!(retry[axis] <= cap);
                cuts.push(
                    json!({"request":at,"axis":axis,"prefix":prefix,"cap":cap,"retry":retry}),
                );
            }
            for (v, n) in prefix.iter_mut().zip(cells) {
                *v += n;
            }
        }
        let mut warm = BookV2ColumnPdfConvergenceBudget::new(caps, maximum);
        hosted(original, caps, |input, context, config, _| {
            let prepared =
                prepare_counted_book_v2_column_resources(input, context, config, caps, &mut warm)
                    .unwrap();
            assert_eq!(
                prepared.resources().canonical_jcs(),
                original.resources().canonical_jcs()
            );
        });
        json!({"style":style,"after_setup":prefix,"setup_fee":resource_resolver_reference_fee(original.resources(),None),"requests":expected.iter().map(|r|json!({"stage":format!("{:?}",r.stage),"font":r.font_face_id.map(|v|v.get()),"image":r.image_id.map(|v|v.get()),"records":r.records,"storage_bytes":r.storage_bytes,"work":r.work})).collect::<Vec<_>>(),"typed_cuts":cuts,"full":rsw(warm.observation()),"quota_owner":0,"partial_resolvers_published":0,"resource_candidates_registered_before_setup_failure":original.resources().fonts().len()+original.resources().images().len(),"resources_opened_before_setup_failure":0,"layouts":{"validated_limits":std::mem::size_of::<typaxis_core::ValidatedResourceLimits>(),"m4_limits":std::mem::size_of::<M4EffectiveResourceLimits>()}})
    })
}

fn font_media_limits(damaged_case: bool) -> Value {
    let original_font = original_font();
    let mut damaged = original_font.clone();
    let row_count = u16::from_be_bytes(damaged[4..6].try_into().unwrap()) as usize;
    let cff_at = (0..row_count).map(|n| 12 + n * 16).find(|at| &damaged[*at..*at+4] == b"CFF ").unwrap();
    damaged[cff_at..cff_at+4].copy_from_slice(b"CFF2");
    with_convergence_probe_fixture("no-page", |original, caps, maximum, _| {
        let mut reports = Vec::new();
        for (name, font) in [if damaged_case { ("unsupported-outline", &damaged) } else { ("original", &original_font) }] {
            let mut style = [0;3];
            let mut native = String::new();
            hosted_with_cff(original, caps, Some(font), |input, context, config, _| {
                let body = prepare_admitted_book_v2_body_with_counted_style(input, caps.base(), &mut |r,s,w| {
                    for (cell,n) in style.iter_mut().zip([r,s,w]) { *cell += n; }
                    Ok::<_,std::convert::Infallible>(())
                }).unwrap();
                native = match prepare_book_v2_resources(body, context, config, caps) {
                    Ok(prepared) => prepared.resources().canonical_jcs().to_owned(),
                    Err(error) => format!("{:?}", error.failure()),
                };
            });
            let [mut r,mut s,mut w] = style;
            typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,caps.base(),caps.base().get().max_fragments,maximum,|a| {
                fund_resource_catalog_reference(a, original.resources(), None);
                fund_resource_attempt_storage_reference(a, original.resources().fonts().len());
                let first = &original.resources().fonts()[0];
                fund_resource_read_reference(a, font.len(), first.uri().as_str(), first.family(), 2 + original.resources().fonts().len() + original.resources().images().len(), true);
                fund_resource_binding_request(a, resource_cff_binding_reference_requests(original.resources())[0]);
            });
            let before_media = [r,s,w];
            use typaxis_font::CffSfntStageV2 as Stage;
            let mut requests = vec![(Stage::DeclaredMediaHeader,0u64,64u64)];
            requests.extend((0..row_count).map(|n|(Stage::DeclaredMediaRecord,(12+n*16) as u64,64)));
            if name != "original" {
                requests.push((Stage::DeclaredMediaDiagnosticHeader,0,64));
                requests.extend((0..=cff_at/16).map(|n|(Stage::DeclaredMediaDiagnosticRecord,(12+n*16) as u64,64)));
            }
            let mut prefix = before_media;
            let mut cuts = Vec::new();
            for (stage,offset,work) in &requests {
                let cap = prefix[2] + work - 1;
                let mut budget = BookV2ColumnPdfConvergenceBudget::new(caps, cap);
                hosted_with_cff(original, caps, Some(font), |input,context,config,_| {
                    let prior_candidates = input.provenance().read_ledger().token().unwrap().candidate_attempt_count();
                    let error = prepare_counted_book_v2_column_resources(input,context,config,caps,&mut budget).unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Admission { body, failure } = error else { panic!("wrong media failure: {error:?}"); };
                    assert_eq!(failure.font_face_id.get(),0);
                    assert_eq!(body.styled().body().limits(), caps.base());
                    let typaxis_font::CffAdmissionBudgetFailureV2::Sfnt(f) = failure.failure else { panic!("untyped media failure"); };
                    assert_eq!(f.point.stage,*stage);
                    assert_eq!(f.point.file_offset,Some(*offset));
                    assert_eq!(f.point.table_tag,None);
                    assert_eq!(f.point.table_offset,None);
                    assert_eq!(f.error,AccountingError::Work {owner:typaxis_core::NodeId::new(0)});
                    assert_eq!(body.provenance().read_ledger_token().unwrap().candidate_attempt_count(), prior_candidates + original.resources().fonts().len()+original.resources().images().len());
                    body.provenance().read_ledger_token().unwrap().revalidate().unwrap();
                });
                assert_eq!(rsw(budget.observation()),prefix);
                assert_eq!(budget.observation().output_charge(),0);
                hosted_with_cff(original,caps,Some(font),|input,context,config,_| {
                    assert!(prepare_counted_book_v2_column_resources(input,context,config,caps,&mut budget).is_err());
                });
                let retry=rsw(budget.observation());
                assert!(retry.iter().zip(prefix).all(|(a,b)|*a>=b));
                assert!(retry[2]<=cap);
                cuts.push(json!({"stage":format!("{stage:?}"),"offset":offset,"work":work,"prefix":prefix,"cap":cap,"retry":retry}));
                prefix[2]+=work;
            }
            let mut budget=BookV2ColumnPdfConvergenceBudget::new(caps,maximum);
            hosted_with_cff(original,caps,Some(font),|input,context,config,_| {
                let observed=match prepare_counted_book_v2_column_resources(input,context,config,caps,&mut budget) {
                    Ok(prepared)=>prepared.resources().canonical_jcs().to_owned(),
                    Err(BookV2ColumnInitialStylePreparationError::Resources(error))=>format!("{:?}",error.failure()),
                    Err(error)=>panic!("unexpected native failure: {error:?}"),
                };
                assert_eq!(observed,native);
            });
            reports.push(json!({"name":name,"font_sha256":hex(&typaxis_core::sha256(font)),"before_media":before_media,"after_media":prefix,"typed_work_cuts":cuts,"full":rsw(budget.observation()),"native_equal":true}));
        }
        json!({"quota_owner":0,"partial_fonts_published":0,"cases":reports})
    })
}

fn font_binding_limits() -> Value {
    use typaxis_resources::ResourceCatalogStage as Stage;
    let font=original_font();
    with_convergence_probe_fixture("no-page",|original,caps,maximum,_| {
        let mut style=[0;3];
        hosted_with_cff(original,caps,Some(&font),|input,_,_,_| {
            prepare_admitted_book_v2_body_with_counted_style(input,caps.base(),&mut|r,s,w|{for(v,n)in style.iter_mut().zip([r,s,w]){*v+=n;}Ok::<_,std::convert::Infallible>(())}).unwrap();
        });
        let [mut r,mut s,mut w]=style;
        typaxis_shaping::book_v2::with_book_v2_observed_source_accounting(&mut r,&mut s,&mut w,caps.base(),caps.base().get().max_fragments,maximum,|a| {
            fund_resource_catalog_reference(a,original.resources(),None);
            fund_resource_attempt_storage_reference(a,original.resources().fonts().len());
            let first=&original.resources().fonts()[0];fund_resource_read_reference(a,font.len(),first.uri().as_str(),first.family(), 2 + original.resources().fonts().len() + original.resources().images().len(), true);
        });
        let before=[r,s,w];let mut prefix=before;let mut cuts=Vec::new();let requests=resource_cff_binding_reference_requests(original.resources());
        for(request_index,request)in requests.iter().enumerate() {
            if request_index==1 {prefix[2]+=64*(1+u64::from(u16::from_be_bytes(font[4..6].try_into().unwrap())));}
            let cells=[request.records,request.storage_bytes,request.work];
            for axis in 0..3 {
                if cells[axis]==0{continue;}
                let cap=prefix[axis]+cells[axis]-1;let mut base=caps.base().get().clone();
                if axis==0{base.max_fragments=cap;}if axis==1{base.max_spool_bytes=cap;}
                let base=typaxis_core::ValidatedResourceLimits::new(base).unwrap();let limited=M4EffectiveResourceLimits::defaults_for(&base);
                let mut budget=BookV2ColumnPdfConvergenceBudget::new(&limited,if axis==2{cap}else{maximum});
                hosted_with_cff(original,&limited,Some(&font),|input,context,config,_| {
                    let error=prepare_counted_book_v2_column_resources(input,context,config,&limited,&mut budget).unwrap_err();
                    let BookV2ColumnInitialStylePreparationError::Catalog{body,failure}=error else {panic!("wrong binding failure: {error:?}")};
                    assert_eq!(failure.request,*request);assert_eq!(body.styled().body().limits(),limited.base());
                    let owner=typaxis_core::NodeId::new(0);let expected=match axis {0=>AccountingError::Records{owner},1=>AccountingError::Spool{owner},_=>AccountingError::Work{owner}};assert_eq!(failure.error,expected);
                    body.provenance().read_ledger_token().unwrap().revalidate().unwrap();
                });
                assert_eq!(rsw(budget.observation()),prefix);assert_eq!(budget.observation().output_charge(),0);
                hosted_with_cff(original,&limited,Some(&font),|input,context,config,_|{assert!(prepare_counted_book_v2_column_resources(input,context,config,&limited,&mut budget).is_err());});
                let retry=rsw(budget.observation());assert!(retry.iter().zip(prefix).all(|(a,b)|*a>=b));assert!(retry[axis]<=cap);
                cuts.push(json!({"stage":format!("{:?}",request.stage),"axis":axis,"prefix":prefix,"cap":cap,"retry":retry}));
            }
            for(v,n)in prefix.iter_mut().zip(cells){*v+=n;}
        }
        let mut warm=BookV2ColumnPdfConvergenceBudget::new(caps,maximum);let mut native=String::new();
        hosted_with_cff(original,caps,Some(&font),|input,context,config,_| {
            let body=prepare_admitted_book_v2_body(input,caps.base()).unwrap();native=prepare_book_v2_resources(body,context,config,caps).unwrap().resources().canonical_jcs().to_owned();
        });
        hosted_with_cff(original,caps,Some(&font),|input,context,config,_| {let prepared=prepare_counted_book_v2_column_resources(input,context,config,caps,&mut warm).unwrap();assert_eq!(prepared.resources().canonical_jcs(),native);});
        assert_eq!(requests[0].stage,Stage::CffBindingChecks);
        json!({"style":style,"before_binding":before,"after_binding":prefix,"requests":requests.iter().map(|r|json!({"stage":format!("{:?}",r.stage),"font":r.font_face_id.map(|v|v.get()),"records":r.records,"storage_bytes":r.storage_bytes,"work":r.work})).collect::<Vec<_>>(),"typed_cuts":cuts,"full":rsw(warm.observation()),"native_equal":true,"quota_owner":0,"partial_fonts_published":0})
    })
}

fn main() {
    let case = std::env::args().nth(1).expect("case");
    let facts = match case.as_str() {
        "cff" => {
            observed_base_cmap_initial_admission_shares_real_command_prefix_and_typed_refusal();
            json!({})
        }
        "pdf" => {
            book_v2_column_counted_initial_style_genuine_admission_same_budget_full_pdf_and_retry();
            json!({})
        }
        "limits" => {
            book_v2_column_counted_initial_style_work_exact_short_and_later_resource_error_keep_prefix();
            json!({})
        }
        "original-cff-pdf" | "japanese-cff-pdf" => {
            let output = std::env::args_os().nth(2).expect("PDF output");
            let text = if case == "original-cff-pdf" {
                "Result"
            } else {
                "日本語 日\u{e0100}"
            };
            original_cff_pdf(text, std::path::Path::new(&output))
        }
        "native-math-cff-pdf" => native_math_cff_pdf(std::path::Path::new(&std::env::args_os().nth(2).expect("PDF output"))),
        "math-cff-pdf" => original_math_cff_pdf(std::path::Path::new(&std::env::args_os().nth(2).expect("PDF output"))),
        "jstf" => positive_jstf(),
        "math-ties" => positive_math_ties(),
        "reads" => resource_reads(),
        "catalog" => resource_catalog_limits(),
        "resolver" => resource_resolver_limits(),
        "font-binding" => font_binding_limits(),
        "font-media" => font_media_limits(false),
        "font-media-errors" => font_media_limits(true),
        "paths" => resource_paths(),
        "owned" => owned_source(),
        "ledger" => ledger_limits(),
        "aliases" => vector_alias_limits(),
        "svg-source" => svg_source_limits(false),
        "svg-clips" => svg_source_limits(true),
        _ => panic!("unknown runtime case"),
    };
    println!("{}", json!({"case":case,"passed":true,"facts":facts}));
}
