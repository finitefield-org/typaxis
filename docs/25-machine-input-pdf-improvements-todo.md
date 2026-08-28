# Machine input PDF実装タスク

Source: `docs/25-machine-input-pdf-improvements.md`

- Design source commit: `1788c84a508013bc33b8d9c6f9c25f0e3853884f`
- 状態: Pending
- 最初の公開単位: M0 + M1、`typaxis.machine-pdf/paragraph-1`
- 対象範囲: docs/25のM0〜M5。M0/M1は実装可能な作業単位まで確定し、M2以降は必要なADR decision gateと、その決定を入力にするvertical sliceへ分割する。

この文書の`Completed`は各milestoneの受け入れ条件を満たしたことだけを意味する。contract type、Schema、内部receiptの存在をCLI E2E完成と読み替えない。最初のmachine input対応を表明できるのは`MI1-17`完了後である。

## 1. Scope

### 1.1 M0/M1で実装するもの

- macOS/Linuxでbuild可能なbaseline
- `DocumentPackage` 1.0/1.1のstrict bounded decode
- exactly one companion sourceを使うsealed trusted ingestion
- `typaxis.machine-pdf/paragraph-1`のcapability descriptor/preflight
- paragraph、heading、text、anchor、page reference、soft/hard breakのPDF経路
- TrueType sfnt/TTC `glyf` font
- `build-package`、`check-package`、`capabilities`
- package-aware manifest、structured diagnostics、stable host admission、alias保護、terminal publication
- contract 1.1 migration、fixtures、producer guide、E2E/reproducibility/differential gate

### 1.2 M0/M1の明示的な非目標

- multi-source machine package
- list、table、figure、footnote、link annotation
- emphasis/strongのplain-text化
- remote source/resource fetch
- PNG/JPEG/SVG/vector figure、math、OTF/CFF
- outline、tagged PDF、release/book profile
- unsupported inputからreference TSF、別backend、rasterへのfallback

非目標のdomainがwire decode/semantic validationを通る場合でも、`paragraph-1` preflightがresource read/layout前に拒否する。偶然layoutできてもdescriptorへ追加しない。

### 1.3 M2以降の扱い

M2〜M5はM1のtrust boundaryとprofile immutabilityを維持する。設計書が値を固定していないprofile ID、page-break blank-page policy、table split policy、math/vector contract等は、対応するdecision-gate milestoneのADRで確定する。後続implementation milestoneはそのADRが`Completed`になるまで開始しない。

## 2. 全milestone共通の実装規則

### 2.1 Trust boundary

- `WireDocumentPackage`はcaller-constructibleなuntrusted DTOである。
- decoder-issued `DecodedDocumentPackage`、session-bound package/source receipt、`ValidatedMachinePackage`、capability receipt、publication receiptはpublic raw-parts constructor、public field、`Clone`を持たない。
- `typaxis-machine-input`は`typaxis-syntax`へ依存しない。trusted `ValidatedParsedPackage`を発行できるownerは`typaxis-syntax`だけである。
- host path/component walk、same-handle snapshot、stable read、read identityは`typaxis-host-admission`だけに置く。machine/resource crateへ複製しない。
- layout/finalizationはpartial resource progressを受け取らず、complete `AdmittedResourceLedger`だけを受け取る。
- manifest/diagnosticsはsealed progressからfactsをprojectし、caller-supplied record fieldをtrusted factsとして受け取らない。

### 2.2 Determinism and limits

- allocation、work、read、ID発行のmax+1回目より前にchecked budgetをconsumeする。exact maxは許可する。
- HashMap insertion順、filesystem列挙順、thread completion順をdiagnostic、manifest、flow、object順へ使わない。
- JSON object member order/whitespaceはraw hashだけへ影響し、typed JCS hashへ影響しない。
- diagnostic materializationは1 command全体でfixed `MAX_MACHINE_DIAGNOSTICS = 256`を共有する。
- canonical sidecarへabsolute HostPath、raw OS error、source/package本文snippetを入れない。
- fixed host limitsは`MAX_RESOURCE_ROOTS = 64`、`MAX_HOST_READ_CANDIDATES = 131_072`とし、capability artifactと同じ定数を使う。

### 2.3 Contract and profile compatibility

- `typaxis.contract/1.0`のSchema/意味を変更しない。1.1切替は`MI1-14`の単一atomic milestoneで行う。
- `DocumentPackage` shapeは1.0/1.1で同じとし、inputに記載されたcontract IDをcanonical hashへ含める。
- `MI1-14`より前は`typaxis-core`の`MachineInputLimitBounds`を`max_document_package_bytes`/`max_json_nesting_depth`のdefault・hard maximumの唯一の正本とし、decoder/preflightへvalidated scalarを渡す。current config/Schema/capabilityへfieldを公開せず、`MI1-14`で同じboundsから`ResourceLimits` fieldとdescriptorを有効化する。
- M1公開単位のgenerated artifactは`MI1-14`以降1.1だけを出す。M2以降のcurrent contract切替は対応decision gateとintegration milestoneだけが行い、raw旧contractの扱いをmigration tableで固定する。
- profile IDはclosed immutable contractである。feature追加、既定policy変更、以前拒否したdomainの受理は新profile IDを要求する。
- `paragraph-1`の意味はM2以降も拡張しない。
- M2の`AdmittedImageMediaKind::Png`はdecoder-issued internal attestationであり、wire declarationではない。M2 manifestはattested PNG kindだけをprojectする。M4 new contractはuntrusted closed `ImageMediaType`/`FontMediaType`をresource declarationへ追加し、resource decoderだけが`AdmittedImageMediaKind`/`AdmittedFontMediaKind`を発行する。M4 manifestではdeclared/attestedを別field/typeとして照合し、URI suffixからどちらも推測しない。

### 2.4 Publication and side effects

- phase順はconfig/targets、host capability、PACKAGE、JSON、source、syntax、resource candidate alias、capability、resource/style、layout/PDF、terminal publicationで固定する。
- unsupported contentではresource bytesをopenせず、layout/PDF tempを作らない。ただしdeclared resource candidateはfailure sidecarによる上書き防止のためgate前にread ledgerへ登録する。
- file群は個別atomicでありmulti-file transactionとは表現しない。visible orderはfailure時`diagnostics -> failed manifest`、success時`trace -> PDF -> diagnostics -> built manifest`である。
- 上記順序はrequested file targetだけを含み、未指定sidecarを暗黙生成しない。`check-package`はrequested diagnosticsだけをpublishし、manifest/PDFを持たない。
- stdout partial writeはrollback可能と扱わない。file publish後のdirectory sync failureはvisible receiptを保持する。

### 2.5 Milestone completion protocol

各milestoneの実装者は次を行う。

1. `Depends on`の全milestoneが`Completed`であることを確認する。
2. milestoneに列挙したfiles以外へ変更が必要なら、責務境界と依存関係を再確認してtask文書を先に更新する。
3. unit/targeted verificationを実行する。
4. `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`と変更crateのtestを通す。
5. public wire、CLI、profileを変更するmilestoneではSchema fixture、invalid fixture、help、docsも同じchange setへ含める。
6. statusを`Completed`へ変える前に、受け入れ条件をobservable evidenceで確認する。

### 2.6 Fixture and evidence layout

- versioned machine fixtureのrootは`samples/machine-package/`とする。

```text
samples/machine-package/
  capabilities.json
  profiles/
    paragraph-1/
      blank-1.0/
        job/document-package.json
        job/sources/blank.json
        expected.json
      blank-1.1/
        job/document-package.json
        job/sources/blank.json
        expected.json
      combined/
        job/document-package.json
        job/sources/book.json
        job/fonts/body.ttf
        expected.json
  invalid/
    p1101-duplicate-escaped-key/
      job/document-package.json
      expected.json
  matrices/
    m2-basic.json
    m3-table.json
    m3-footnote.json
    m3-all.json
    m4-production.json
```

- 各profile IDの最後のpath segmentを`profiles/`直下のdirectory名にする。新profileの公開milestoneは必ず`combined/`を追加する。
- `job/`がdefault package rootである。resourceを使うfixtureは`job/`を明示resource rootとして渡し、package rootが暗黙resource rootにならないことを保つ。
- `expected.json`はfixture ID、contract/profile、CLI arguments、expected exit/code/location、visible artifacts、page count、normalized extracted text、advertised item coverage、resource hashesをcanonical JCSで持つ。対応SchemaとvalidatorはMI1-16で追加する。
- invalid fixture directoryは小文字primary code、hyphen、stable case nameの順にする。同じbytesを複数caseで共有せず、各caseのread/side-effect expectationを`expected.json`へ明記する。
- `matrices/`はADRで後から確定するprofile IDとprofile fixture pathをstable verification commandへ結ぶcanonical JCSであり、listed fixtureの重複、missing file、profile不一致をvalidatorで拒否する。
- generated PDF/sidecar/evidenceはversioned sample directoryへ書かず、test temporary directoryまたは`target/machine-e2e/`へ書く。external verification toolは`expected.json`からbuildを再実行し、生成pathを引数で受け取る。

### 2.7 Decision-gate operation

- `ADR-0027`以後の番号は各decision-gate開始時の`adr/README.md`で次に空いている値を予約する。同時進行するgateはcatalog更新を直列化し、同じ番号を仮定して並行執筆しない。
- decision-gateの`Status`はADRがAcceptedとなり、profile/contract/Schema ID、closed受理集合、limit、error、fallback/oversize/progress、migration/publication順が全て固定された後だけ`Completed`へ変更する。
- ADRの結論が本書に列挙したprimary files、依存関係、公開単位を変える場合は、implementation開始前に本タスク文書とdependency graphを更新してquality gateを再実行する。
- rejected alternativeは実装時のfallbackに使わない。未採択feature/policyは対応profile preflightのclosed rejectionとしてfixture化する。

## 3. Dependency map

```text
MI0-01 -> MI0-02 -> MI1-01
MI1-01 -> MI1-02 -> MI1-03 -> MI1-04
MI1-01 -> MI1-05 -> MI1-06
MI1-04 + MI1-06 -> MI1-07 -> MI1-08
MI1-04 + MI1-07 -> MI1-09
MI1-06 + MI1-08 + MI1-09 -> MI1-10 -> MI1-11
MI1-10 -> MI1-12
MI1-06 + MI1-09 + MI1-12 -> MI1-13
MI0-02 + MI1-02 + MI1-04 + MI1-09 + MI1-10 + MI1-12 + MI1-13 -> MI1-14
MI1-11 + MI1-13 + MI1-14 -> MI1-15 -> MI1-16 -> MI1-17
MI1-17 -> M2 series -> M3 series -> M4 series -> M5 series
```

`MI1-14`はcontract IDの意味を部分的に変えないため意図的にcross-cuttingである。ほかのmilestoneは原則として1 crate責務または1 vertical sliceへ限定する。

### M1 completion evidence

- Implementation commit: `69e2df43282e6fcb816d4c77fda6fb678020ba2f`。
- Completion review (2026-08-26): MI1-01からMI1-17までをdependency map順に再確認し、各deliverable、non-goal、acceptance criteriaと実装・fixture・public CLI surfaceが一致することを確認した。
- macOS 26.5.2 arm64、rustc/cargo 1.97.1でlocked workspace all-targets check/test、clippy `-D warnings`、fmt、host/resource targeted tests、Schema validator、Python suiteがexit 0だった。obsoleteなMI0 macOS unsupported-resource testは、MI1-06のcontained-open実装と矛盾するため削除し、font付きcombined public E2Eをcurrent regressionとした。
- macOS `aarch64-apple-darwin`とmanaged Linux `aarch64-unknown-linux-gnu`で、同一revision/source snapshot/fixtureからcanonical host evidenceを生成した。各hostの14 checksはclean build、public check/build二回、five-artifact byte identity、Schema、M1-only capability、MuPDF raster、Poppler page/text、異名source snapshot再現性をすべて`passed`とした。
- `python3 tools/verify_machine_profile.py --require-host-evidence target/machine-e2e/host-evidence --required-host macos --required-host linux`は、両hostの同一source/fixture/artifact bindingを確認してexit 0だった。GitHub Actions/GitHub workflowは使用していない。
- macOS linkerがnative archive memberのtarget pathをsymbol tableへ保持する差を検出したため、machine reproducibility buildはsource/target path remapとdebug/local symbol除去を使い、isolated target名長も固定した。異名checkoutのbinary bytes、version、five artifact bytesのexact一致をactual gateと回帰testで確認した。

この共有evidenceにより、各MI1 milestoneの直接verificationとdependency closureが成立した。M2以降は引き続きPendingであり、`paragraph-1`のclosed contractは拡張しない。

## 4. M0: baseline and decisions

### MI0-01 macOS build baselineを復旧する

- Status: Completed
- Implementation commit: `edd8ec9f57a2a58de6f6c23af94b1982fb4da9d1`
- Completion evidence (2026-08-25, macOS 26.5.2 arm64, rustc/cargo 1.97.1):
  - 空の`CARGO_TARGET_DIR`からlocked `typaxis-cli` buildが成功し、同じbinaryは`typaxis 0.1.0`、SHA-256 `6c2364768483afc97ed8fd2502a54ca47ea61d0efb640872ad576f2d2a3a9ade`だった。
  - 同じbinaryによる`samples/minimal/empty.tsf` buildは512-byte、PDF 1.7、1 page、SHA-256 `01bdd2e1b730cab33456b08582ec237ef155ad90f33ca5d1731a9132adb48e8e`を生成した。
  - locked workspace all-targets check/test、targeted macOS resource regression、all-targets clippy `-D warnings`、fmt checkはすべてexit 0だった。
  - macOS resource regressionはstable `UnsupportedContainedOpen`、exit 3、stdoutなしで、requested PDF/manifestの新規作成と`--force`時の既存target置換がないことを確認した。
- Contract/capability/Schema impact: なし。reference TSF CLIと`typaxis.contract/1.0`のwire surfaceは変更していない。
- Scope adjustment: pre-resource failureのside-effect policyをCLI entrypointまで伝播するため`main.rs`を、macOSで作成不能な非UTF-8 host-path testsを実装domainへ合わせるため`font.rs`/`font_commands.rs`をPrimary filesへ実装前に追加した。並列testのtimestamp衝突を避ける一意sequenceも`pipeline.rs` test helperへ追加した。
- Depends on: None
- Design inputs: docs/25 §4.7、TMI-013、§12.3、Slice 0
- Primary files:
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/font.rs`
  - `workspace/crates/typaxis-cli/tests/cli_end_to_end.rs`
  - `workspace/crates/typaxis-cli/tests/font_commands.rs`
  - `workspace/README.md`
- Deliverables:
  - 全targetで存在するfallible `HostResourceFile::exact_length() -> Result<u64, ResourceAdmissionError>`。
  - unsupported platform fallbackが架空のlengthを返さず`UnsupportedContainedOpen`を返す実装。
  - `PathBuf`、`ConfigResourceRoot`、platform test helperの`cfg`/import整合。
  - clean targetからbuildしたCLIのversion/blank PDF smoke evidence。
- Tasks:
  1. `CARGO_TARGET_DIR`を新しい一時directoryへ向け、既存binaryを使わずmacOS compile errorを再現する。
  2. `exact_length` call siteをすべてfallibleにし、length取得失敗をI/O/resource errorへ保ったまま伝播する。
  3. Linux/Android実装とunsupported fallbackが同じtrait/API surfaceを持つよう`cfg`を整理し、Linux/Androidでだけ作成できる非UTF-8 host-path testsをmacOS test targetから除外する。
  4. unsupported fallbackのunit testでopen/read/metadata receiptが発行されないことを確認する。
  5. macOSでresourceを要求するCLI fixtureを実行し、stable `UnsupportedContainedOpen`となってrequested PDF/manifest targetを作成・置換しないintegration testを追加する。pipelineのtyped failure policyを`run_build`が参照し、このpre-resource platform failureではfailed manifest publicationを開始しない。
  6. blank reference buildをcurrent sourceから作ったbinaryで実行する。
- Acceptance criteria:
  - macOSでlocked build/check/test/clippyがcompile gateを通る。
  - platform-independent callerから見たfallible exact-length APIが`cfg`で欠落せず、macOS selected fallbackを含む全call siteがtype-checkする。
  - resourceありのmacOS pathはこのmilestoneではstable unsupported errorでよく、font bytesをadmitしたと表明しない。
  - `workspace/target/debug/typaxis`の既存時刻だけを証拠に使わない。
- Verification:
  - `cargo build --manifest-path workspace/Cargo.toml --package typaxis-cli --locked`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli resource_is_stable_unsupported_on_macos --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `workspace/target/debug/typaxis --version`
  - `workspace/target/debug/typaxis build samples/minimal/empty.tsf -o /tmp/typaxis-mi0-empty.pdf`
- Non-goals:
  - macOS component walker
  - machine package command/decoder
  - Linux/Android runtime再検証。documented Linux gateは`MI1-17`で閉じる。

### MI0-02 Machine input ADRとphase ownershipを採択する

- Status: Completed
- Completion evidence (2026-08-25):
  - `ADR-0027`をAccepted targetとしてcatalogへ登録し、現行1.0実装とM1 targetを分離したowner/dependency/forbidden-edge、single-source/root、receipt、profile、1.1 migration、publication contractを固定した。
  - `typaxis.machine-pdf/paragraph-1`のclosed accepted/rejected domain、host availability、fixed limits、compatible/incompatible change規則を単一contractへ記録した。
  - milestone記載の3本の`rg` verification、`python3 schemas/validate.py`、`cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`はすべてexit 0だった。
  - MI0-01 actual-host evidenceをcompile、atomic reference publication、contained PACKAGE/source、contained resourceの独立statusへ反映した。
- Contract/capability/Schema impact: target契約の採択だけを行い、current `typaxis.contract/1.0` wire、Schema bytes、Rust実装、public CLI/helpは変更していない。current 1.1 switchはMI1-14、public CLI E2E/release claimはMI1-17に留保した。
- Scope adjustment: なし。milestoneのPrimary filesと本completion recordだけを変更した。
- Depends on: MI0-01
- Design inputs: docs/25 §6、§10〜§12、§13.5
- Primary files:
  - `adr/ADR-0027-machine-document-package-ingestion.md`
  - `adr/README.md`
  - `docs/02-workspace-boundaries.md`
  - `README.md`
  - `workspace/README.md`
  - `docs/19-cli.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `schemas/README.md`
  - `contracts/phase-ownership.md`
  - `contracts/machine-pdf-capabilities.md`
- Deliverables:
  - explicit `build-package`/`check-package` command、package-root semantics、single-source M1、receipt ownership、contract 1.1 migration、profile immutability、publication orderを採択するADR。
  - host/document-package/machine-input/syntax/machine-profileのdependency edgeと禁止edge。
  - capability ID、availability、compatible change/incompatible change規則。
  - contract、partial implementation、public CLI E2E、releaseのstatus軸を分けた現行support documentation。
- Tasks:
  1. ADRへcontext、decision、alternatives rejected、security consequences、compatibility、rollout orderを記録する。
  2. `WireDocumentPackage`とdecoder-issued/trusted receiptsを別trust levelとして明記する。
  3. package rootをresource rootへ暗黙追加しないこと、multi-sourceを受理しないことを固定する。
  4. `paragraph-1`がvisual headingを扱うがoutline/tagged heading semanticsを約束しないことを固定する。
  5. `contracts/phase-ownership.md`へhost read ledger、machine parse、capability preflight、diagnostics/manifest projection ownerを追加する。
  6. ADR catalogとworkspace boundaryを更新し、target設計を実装済みと誤記しない。
  7. README/CLIへ現行`build`のINPUTがreference TSFでありDocumentPackage JSONを受理しないこと、`dump-ast`が一方向exportであることを明記する。
  8. roadmap/checklist/contract matrixへcontract-defined、implemented、CLI E2E、release-supportedの列または明示labelを追加し、future targetをcurrent statusから分離する。
  9. workspace/Schema docsへportable validationとtrusted ingestionが別境界であることを記載する。
  10. MI0-01のactual-host evidenceを使い、macOS compile、atomic publish、contained package/resource openのstatusを別項目として記録する。
- Acceptance criteria:
  - docs/25 M0/M1について後続実装者が選択を求められるproduct/trust decisionが残らない。
  - `typaxis-machine-input -> typaxis-syntax`のedgeは禁止される。
  - capability contractが同じprofile IDの意味拡張を許可しない。
  - M2以降のpolicyは対応ADRへ明示的にdeferされる。
  - docsだけを読む利用者が現行CLIへDocumentPackage JSONを渡せる、またはroadmap項目がCLI E2E済みだと誤解しない。
- Verification:
  - `rg -n "machine-input|document-package|machine-profile|host-admission" adr/ADR-0027-machine-document-package-ingestion.md docs/02-workspace-boundaries.md contracts/phase-ownership.md`
  - `rg -n "typaxis.machine-pdf/paragraph-1|compatible|incompatible" contracts/machine-pdf-capabilities.md`
  - `rg -n "reference TSF|DocumentPackage|dump-ast|CLI E2E|release" README.md docs/19-cli.md docs/21-roadmap.md docs/22-contract-matrix.md docs/23-implementation-checklist.md schemas/README.md`
- Non-goals:
  - Rust implementation
  - M2以降のprofile ID/pagination policy決定

## 5. M1: trusted single-source paragraph PDF

### MI1-01 Core identityと4 crateのcompile boundaryを作る

- Status: Completed
- Depends on: MI0-02
- Design inputs: docs/25 §12.1、§12.2、§14 file map
- Primary files:
  - `workspace/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-host-admission/`
  - `workspace/crates/typaxis-document-package/`
  - `workspace/crates/typaxis-machine-input/`
  - `workspace/crates/typaxis-machine-profile/`
  - `workspace/crates/typaxis-testkit/src/lib.rs`
- Deliverables:
  - 4 crate memberと最小`lib.rs`/Cargo manifest。
  - `JsonPointer`、`DocumentPackageContractId`、`MachinePdfProfileId`、algorithm-bearing `MachineInputFingerprint` newtype。
  - M1 stagingで共有する`MachineInputLimitBounds`。
  - forbidden dependency edge test。
  - Rust 1.75互換かつexact-pinnedなJSON dependenciesのlocked resolution。
- Tasks:
  1. 全crateへworkspace package/lintsと`#![forbid(unsafe_code)]`を適用する。
  2. `typaxis-document-package`だけへ`serde`、`serde_json`、`serde_path_to_error`、`serde_stacker`を必要feature込みでexact pinする。
  3. `JsonPointer`はroot empty stringとRFC 6901 segment escapeだけをconstructorから発行できるようにする。
  4. contract/profile IDはarbitrary stringと混同しないclosed known/current typeを用意する。
  5. fingerprint newtypeへalgorithm ID `typaxis.machine-input-sha256/1`を持たせ、raw bytesからのpublic trusted constructorを作らない。
  6. testkitのdependency auditへdocs/25 §12.1のallowed/denied edgeを追加する。
  7. `MachineInputLimitBounds`へpackage bytes default 134,217,728/hard JSON-safe integer maximumとJSON depth default/hard 256を一度だけ定義する。`MI1-14`まではcurrent config/Schema encoderへ出さない。
- Acceptance criteria:
  - workspaceが4 crateを含めてlocked checkできる。
  - `typaxis-machine-input`のCargo manifestに`typaxis-syntax`がない。
  - `typaxis-host-admission`がdocument/style/syntax/manifestへ依存しない。
  - `typaxis-document-package`がhost-admissionへ依存しない。
  - decoderとprofileが同じ`MachineInputLimitBounds`をimportし、数値literalを重複定義しない。
  - dependency auditは意図的に禁止edgeを入れたmutant manifestを検出する。
- Verification:
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
- Non-goals:
  - JSON decode
  - host file open
  - trusted package issuance

### MI1-02 Full wire DTOとshared JCS encoderを実装する

- Status: Completed
- Depends on: MI1-01
- Design inputs: docs/25 §6.3、§6.10、§12.1、§12.4
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
- Deliverables:
  - current DocumentPackage全variantを表すcaller-constructible `WireDocumentPackage` tree。
  - bounded streaming JCS encoderとcount/hash sink。
  - `typaxis-syntax`所有のexhaustive domain-to-wire conversion。
- Tasks:
  1. Schemaのroot/source/text/document/style/page/resource fieldをwire-specific struct/enumへ写し、domain trust typeをDTOとして再利用しない。
  2. integerはwire/domain上限とJCS exact rangeを保持できるexact integer typeにし、float fieldを導入しない。
  3. object memberをRFC 8785のUTF-16 code-unit lexical順、arrayをcontract canonical順でstreaming encodeする。contract-declared ASCII memberは同じwriterで固定順に発行する。
  4. JSON stringはRFC 8785準拠のminimal escaping/Unicode scalar outputにし、UTF-8順とUTF-16順が異なるnon-ASCII key goldenでordering実装を検査する。
  5. encoderへbyte budgetを持つ`Write` sinkを要求し、max+1 byteのwrite前に失敗させる。
  6. hash-only pathはcanonical bytes全体を保持せずSHA-256を計算する。
  7. `ValidatedParsedPackage`の全current enum variantをexhaustive matchでwireへ変換する。unsupported machine profile判定をconverterへ入れない。
  8. CLIの既存partial field mapperをshared conversion/encoderへ置換する準備を行うが、generated contract 1.1切替は`MI1-14`まで行わない。
- Acceptance criteria:
  - style rules、page rules、font、image、footnote、全block/inline variantを変換できる。
  - domain enumへvariantを追加するとconversionがcompile errorまたは明示contract migration errorになる。
  - encoderに別member spelling/JCS order implementationがCLIへ残らない。
  - count pass失敗時はstdout sinkへ1 byteも書かないAPI構造になっている。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax --locked`
  - `rg -n "document_package_json|WireDocumentPackage" workspace/crates/typaxis-cli/src/artifacts.rs workspace/crates/typaxis-syntax/src/lib.rs`
- Non-goals:
  - untrusted JSON parse
  - `paragraph-1` capability filtering

### MI1-03 Iterative strict JSON preflightを実装する

- Status: Completed
- Depends on: MI1-02
- Design inputs: docs/25 §6.3、§12.4、diagnostic `P1100`/`P1101`、limit `I9100`/`I9101`
- Primary files:
  - `workspace/crates/typaxis-document-package/src/preflight.rs`
  - `workspace/crates/typaxis-document-package/src/error.rs`
- Deliverables:
  - recursionを使わないlexical/structural scanner。
  - byte/depth/duplicate-key errorのtyped internal representation。
- Tasks:
  1. `MachineInputLimitBounds`の範囲内でconstructor検証済みのruntime byte/depth scalarをallocation/read ownerから受け取り、decoder entryでも再検査する。current configとの接続は`MI1-14`まで行わない。
  2. UTF-8、BOM、raw NUL、root object、single top-level value、trailing tokenを検査する。
  3. object/array開始をdepth 1とし、max+1 containerへ入る前に停止する。
  4. `\uXXXX`のvalid surrogate pairをUnicode scalarへdecodeし、lone/misordered surrogateを拒否する。decode後のmember nameをobject frame単位で比較して`"a"`と`"\u0061"`をduplicateにし、normalization/case-foldはしない。
  5. JSON string/escape/number/literal grammarを検査し、fraction/exponent integerの最終type errorとlexical grammar errorを区別できるtoken metadataを返す。
  6. full token offset tableを作らず、current container/field/token startだけを保持する。
  7. malformed/truncated inputで最後に確定したcontainer/token offsetを返す。
- Acceptance criteria:
  - exact depth/bytesは成功しmax+1はwork/allocation前に失敗する。
  - duplicate keyを全depth、escaped spellingで検出する。
  - arbitrary bytes/property testでpanic、stack overflow、infinite loopがない。
  - `serde_json::Value`、recursive scanner、whole-input token tableを使わない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package preflight --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package preflight_arbitrary_bytes --locked`
- Non-goals:
  - typed DTO construction
  - semantic ID/order validation

### MI1-04 Bounded typed decoderとJSON location indexを実装する

- Status: Completed
- Depends on: MI1-03
- Design inputs: docs/25 §12.2、§12.4、§15.1 decoder cases
- Primary files:
  - `workspace/crates/typaxis-document-package/src/decode.rs`
  - `workspace/crates/typaxis-document-package/src/location.rs`
  - `workspace/crates/typaxis-document-package/src/jcs.rs`
- Deliverables:
  - `StrictDocumentPackageDecoder`、private-binding `DecodedDocumentPackage`。
  - `JsonLocationIndex`とtyped canonical JCS hash。
- Tasks:
  1. preflight成功後だけ`serde_json` recursion limitを解除し、`serde_stacker`上でmaximum 256以内をdecodeする。
  2. 全objectをunknown-field rejectにし、`flatten`、generic `Value`、unbounded derive collectionを使わない。
  3. `DeserializeSeed`へsource/text/node/style/master/font/image count、per-text bytes、aggregate bytesのchecked budgetを渡し、reserve/push前にconsumeする。
  4. integer fieldへ直接decodeし、fraction/exponent、negative-to-unsigned、range外を拒否する。
  5. known package contract 1.0/1.1を受理しunknown contract、unknown coordinate unitをtyped errorにする。
  6. line/columnはerror時だけraw bytesを再走査して0-based byte offsetへ変換する。
  7. location indexはarray ordinalを正本にし、bounded `(typed ID, occurrence, ordinal)`列をstable sortする。entry countとsort scratchを対応するAST/style/text/resource limitへreserve前にchargeし、Pointer materialization bytesもpackage budget内で検査する。caller ID値を`Vec`長に使わない。
  8. duplicate IDは二件目、unknown fieldはkey、type/rangeはvalue、missing fieldはcontaining objectをprimaryにする。
  9. canonical encoderをhash sinkへ通し、decoded receiptへraw hash/canonical hash/location indexをprivateにbindする。
- Acceptance criteria:
  - Schema positive DocumentPackage fixtureをdecodeできる。
  - invalid fixtureはSchema rejection/conformance rule/public diagnostic namespaceを混同せずexpected decode phaseへ落ちる。
  - whitespace/member order差でraw hashだけが変わる。
  - semantic field差でcanonical hashが変わる。
  - callerが`DecodedDocumentPackage`をliteral/raw partsから構築できないcompile-fail testがある。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package --locked`
  - `python3 schemas/validate.py`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package decoder_limits_and_locations --locked`
- Non-goals:
  - source file open
  - `ValidatedParsedPackage` issuance

### MI1-05 Generic host admissionをresource crateから抽出する

- Status: Completed
- Depends on: MI1-01
- Design inputs: docs/25 §12.1、§12.3、docs/18 resource trust
- Primary files:
  - `workspace/crates/typaxis-host-admission/src/`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/Cargo.toml`
  - `workspace/crates/typaxis-testkit/src/lib.rs`（dependency/API boundary test）
- Deliverables:
  - `OpenedContainedFile`、bounded read permit、`StableFileBytesReceipt`、root/session/read identity primitives。
  - existing resource admissionが新host ownerを使うadapter。
- Tasks:
  1. directory/root capability、opened handle identity、snapshot、bounded readerをlogical resource IDから独立させる。
  2. openとreadを二段に分け、machine/resource ownerがobserved exact lengthをbudget reserveしてからread permitを渡す。
  3. caller-supplied exact length、arbitrary `Read`、raw root path arrayからtrusted receiptを作るAPIを削除/非公開化する。
  4. same session/root token以外のpermit/receiptを拒否する。
  5. existing font/image admissionのobservable Linux behaviorとerror orderingを維持する。
  6. host crateがcanonical manifest/diagnostic recordを作らないことをdependency/API testで固定する。
- Acceptance criteria:
  - resource admissionだけがlogical FontFaceId/ImageResourceIdへhost receiptをbindする。
  - host crateはdocument/syntax/style/manifestへ依存しない。
  - another-session/root receipt swapが失敗する。
  - existing resource exact/max+1 testsが新owner経由で通る。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-host-admission --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit --locked`
- Non-goals:
  - machine package/source policy
  - manifest progress records

### MI1-06 Cross-platform contained open、stable read、read ledgerを完成する

- Status: Completed
- Depends on: MI1-05
- Design inputs: docs/25 §12.3、§12.6 host availability、`I9102`、`I9110`〜`I9113`
- Primary files:
  - `workspace/crates/typaxis-host-admission/src/lib.rs`
  - `workspace/crates/typaxis-host-admission/src/platform/`
  - `workspace/crates/typaxis-host-admission/src/read_ledger.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
- Deliverables:
  - Linux/Android `openat2` + component fallback、macOS `openat(O_NOFOLLOW)` walker。
  - stable exact read、fixed host budgets、host capability token、generic `HostReadIdentityLedger`。
- Tasks:
  1. root directory handleを一度admitし、中間componentはdirectory、terminalはregular fileとしてsame-handle `fstat`で検査する。
  2. Linux/Androidは`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS`を使い、`NOSYS`だけをcomponent walker fallback条件にする。
  3. macOSは各componentをhandle-relative `openat(O_NOFOLLOW)`し、path内symlinkを拒否する。
  4. shared lock可能targetではnonblocking lock、pre/post snapshot、exact chunk read + SHA-256、short read/growth/mutation検出を実装する。
  5. `MAX_RESOURCE_ROOTS`はroot identity/open前、`MAX_HOST_READ_CANDIDATES`はattempt reserve前に検査する。duplicate targetはwork budgetを別consumeしidentity storageだけdeduplicateする。
  6. candidate parent+leafとopened identityをledgerへ登録し、write-target照合/revalidation用sealed tokenを発行する。
  7. unsupported targetはpackage bytes前にavailability false/`UnsupportedContainedOpen`となるtokenを出す。
- Acceptance criteria:
  - root escape、intermediate/final symlink、non-regular file、multi-root ambiguityをfail closedにする。
  - exact lengthのみ読み、limit検査目的のmax+1 byteを読まない。
  - read中のtruncate/grow/replaceがstable failureになる。
  - macOSでTrueType resource admissionが成功する。
  - fixed root/candidate exact max/max+1とduplicate-target accounting testがある。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-host-admission --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission --all-targets --locked`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
- Non-goals:
  - Windows contained opener。同等機能が無いtargetはavailability falseとする。
  - remote URI fetch

### MI1-07 Machine package/source admission sessionを実装する

- Status: Completed
- Depends on: MI1-04、MI1-06
- Design inputs: docs/25 §6.2、§6.4、§12.2、§12.3
- Primary files:
  - `workspace/crates/typaxis-machine-input/src/`
- Deliverables:
  - `HostMachineInputSession`、`AdmittedPackageBytes`、`SessionBoundDecodedPackage`、`AdmittedMachineSourceSet`、`AdmittedMachinePackage`。
  - monotonic `MachineInputProgress`とportable `MachineInputFingerprint`。
- Tasks:
  1. `--package-root`省略時のlexical parent/current-directory semanticsと明示root containmentを実装する。
  2. PACKAGEをroot-relative `PortablePath`としてopenし、same-handle observed lengthを`max_document_package_bytes`へallocation/read前にreserveしてからexact stable readする。raw bytes/length/SHA-256をreceiptへbindし、max+1 probeをしない。
  3. `decode_and_bind`はsame-session raw receipt所有bytesだけをMI1-04 decoderへ渡す。post-hoc decoded value binding APIを作らない。
  4. decoded source declarationをpreflightし、exactly one、SourceId 0、安全なrelative URI、declared length/hashを要求する。
  5. companion sourceをpackage rootだけからopenし、same-handle observed lengthを既存`max_source_bytes`とchecked aggregate `max_input_bytes`へallocation/read前にreserveしてからexact stable readする。UTF-8/declared length/hashを照合し、失敗時はsource-admitted receiptを発行しない。
  6. `finish`でraw/decoded/source setのsession、package hash、declaration fingerprintをexact照合してconsumeする。
  7. progressをRawPackageAdmitted、PackageDecoded、SourcesAdmittedまで単調に進め、failureにも最後のsealed tokenを返す。
  8. portable fingerprint JCSへpackage raw/canonical/contract/URIとactual source factsだけを入れ、host root/session/profile/configを除外する。
- Acceptance criteria:
  - package/source receiptを別sessionで交換するとbytes/hashが同じでも失敗する。
  - package rootをresource rootへ暗黙追加しない。
  - sourceはread後のowned bytesだけを下流へ渡しpathを再openしない。
  - two sources、nonzero SourceId、unsafe path、hash/length mismatchが専用typed errorになる。
  - crateは`typaxis-syntax`へ依存しない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-input --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
- Non-goals:
  - AST semantic validation
  - include directive scan/source graph synthesis

### MI1-08 Syntax loweringとsealed trusted package issuanceを実装する

- Status: Completed
- Depends on: MI1-07
- Design inputs: docs/25 §4.3、§4.4、§12.5
- Primary files:
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-syntax/Cargo.toml`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-machine-input/src/lib.rs`
  - `workspace/crates/typaxis-machine-input/src/tests.rs`
  - `workspace/crates/typaxis-document-package/src/decode.rs`
- Deliverables:
  - `DocumentPackageParser`、`MachineParseOutcome`、`ValidatedMachinePackage`、`ValidatedMachineProvenance`。
  - entry-only source closureとJSON Pointer付きsemantic error mapping。
- Tasks:
  1. admitted inputをconsumeし、contract/unit/source declarationを再照合する。
  2. actual source bytesから`SourceCatalog`、wire text buffersから`TextStore`をmove主体で構築する。
  3. recursive domain construction前にtyped preorder、count、nesting、dense/canonical ID/orderをiterativeに検査する。
  4. Document、StyleSheet、PageMasterSet、ResourceCatalogを構築する。
  5. syntax owner内部だけでentry-only closureを発行し、arbitrary producer sourceへreference include keyword scanをしない。
  6. private `ValidatedParsedPackage::new_resolved`でSourceSpan、UTF-8 boundary、identity TextMap bytes、anchor/footnote/resource/style/page/limit closureを検査する。
  7. success時だけvalidated packageとprovenanceをwrapし、failureにはlast progress + typed failureを返す。
  8. field/ID errorを`JsonLocationIndex`へmappingし、source locationをprimaryにした場合はpackage locationをnoteへ一つだけ置く。
  9. `src/lib.rs`の`compile_fail` doctestでraw DTO、parsed package、caller-authored include graphからtrusted packageを発行できないことを固定する。
- Acceptance criteria:
  - actual source bytesとidentity map不一致をportable Schema成功後でも拒否する。
  - public `ParsedPackage -> ValidatedParsedPackage`、wire DTO promotion、machine entry-only graph constructorが存在しない。
  - large source/text buffersをparse境界でcloneしない。
  - full wire modelをtrusted domainへ変換できるが、PDF-buildabilityはまだ主張しない。
  - compile-fail testsがcaller-authored promotionを拒否する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax --locked`
  - `cargo test --doc --manifest-path workspace/Cargo.toml --package typaxis-syntax --locked`
- Non-goals:
  - capability subset check
  - resource bytes admission

### MI1-09 Structured diagnostic modelとcommand-wide budgetを実装する

- Status: Completed
- Depends on: MI1-04、MI1-07
- Design inputs: docs/25 §6.9、§12.4、§12.9、§12.10
- Primary files:
  - `workspace/crates/typaxis-diagnostics/src/lib.rs`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/Cargo.toml`
  - `workspace/crates/typaxis-pagination/src/lib.rs`
  - `workspace/Cargo.lock`
- Deliverables:
  - `DiagnosticLocation::{PackageJson, Source}`、nullable global location、validated notes。
  - `MachineDiagnosticBudget`とtyped code/error-subject mapping。
- Tasks:
  1. `PackageJson`へportable URI、JsonPointer、optional raw byte offsetを持たせる。
  2. `Source`はSourceSpan/TextSpan/NodeIdの少なくとも一件をconstructorで要求する。
  3. global config/I/O/publicationだけにnull locationを許可する。
  4. fixed 256 aggregate budgetをcommandが一つだけ所有し、config/host/package/decode/source/syntax/capability/resource/style/layout/PDF/publicationの各phaseへscoped lenderを渡せるAPIを作る。各phaseがbudgetを複製・resetできないようにする。
  5. budget満杯後の最初のerror/fatalで末尾advisoryをevictし、primary failureを保持して省略数を最後のnoteへ集約する。
  6. `P1100`〜`P1112`、`L5100`/`L5101`、`R7100`、`I9100`〜`I9113`、`I9190`をtyped constant/mapperへ置く。
  7. resource/style/layout errorへlogical ID/property subjectを追加し、`Debug`文字列解析を排除する。
  8. canonical message/noteからabsolute path、raw OS detail、input snippetをrejectするbuilder testを追加する。
- Acceptance criteria:
  - command全体のdiagnostic数が256を超えない。
  - fatalはterminalで後続emitを許さない。
  - same logical failureのcanonical diagnosticがcheckout root/platform OS messageに依存しない。
  - public codeとSchema conformance `rule_id`が別namespaceとしてtestされる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-diagnostics --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-diagnostics public_error_mapping --locked`
- Non-goals:
  - diagnostics wire 1.1 switch。encoding/Schema切替は`MI1-14`。
  - CLI sidecar publication

### MI1-10 `paragraph-1` descriptor、preflight、capabilitiesを実装する

- Status: Completed
- Depends on: MI1-06、MI1-08、MI1-09
- Design inputs: docs/25 §6.7、§12.6、§13.5
- Primary files:
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-machine-profile/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-host-admission/src/lib.rs`
  - `workspace/crates/typaxis-machine-input/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-diagnostics/src/lib.rs`
  - `workspace/crates/typaxis-syntax/src/lib.rs`
- Deliverables:
  - single-source `MachineProfileDescriptor::PARAGRAPH_1`。
  - deterministic `MachinePdfPreflight`/receipt。
  - compiled `HostCapabilityDescriptor`とcanonical capabilities encoder。
- Tasks:
  1. accepted/rejected block、inline/reference、footnote、style property/selector、page value/master、font/image、PDF semanticsをone descriptorへ列挙する。
  2. Documentをiterative typed preorder/NodeId順、global itemsをstyle source_order、MasterId、resource ID順に検査する。
  3. all AST workをboundedに完了し、materialized diagnosticだけをshared budget内に制限する。
  4. success receiptへprofile、DocumentFingerprint、StyleFingerprint、MachineInputFingerprint、opaque session bindingを持たせる。
  5. machine-input/resource-admission/atomic-publisher ownerのcompile-time tokenからhost availability/features/limitsを合成する。
  6. unavailable hostでprofile `available=false`、command preflight用`I9110`を同じdescriptorから返す。
  7. capabilities JSONをconfig/filesystem/localeなしでdescriptorからcanonical encodeし、package byte/depthのdefault/hard maximumを`MachineInputLimitBounds`から出す。public command/current Schemaとの接続は`MI1-14`/`MI1-17`まで行わない。
  8. each advertised itemのsingle fixtureと全item combined fixtureをdescriptor mutation testへ結ぶ。
- Acceptance criteria:
  - paragraph、heading、text、anchor、page reference、soft/hard breakだけを受理する。
  - unsupported inline/style/master/image/footnoteをresource open/layout前にstable orderで拒否する。
  - heading levelはfingerprintへ残るがoutline/tagged semanticsをadvertiseしない。
  - TrueType standalone/TTC `glyf`だけをadvertiseする。
  - capabilities encoderとpreflightにduplicate feature listがない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile --locked`
  - `rg -n "paragraph-1|heading-semantics|MAX_RESOURCE_ROOTS|MAX_HOST_READ_CANDIDATES" workspace/crates/typaxis-machine-profile workspace/crates/typaxis-host-admission`
- Non-goals:
  - public `capabilities` command
  - M2 feature/profile

### MI1-11 Machine layout wrapperとcheck-package preflight境界を実装する

- Status: Completed
- Depends on: MI1-10
- Design inputs: docs/25 §12.6、§12.7、existing `layout_reference`
- Primary files:
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/Cargo.toml`
  - `workspace/crates/typaxis-layout/src/lib.rs`
  - `workspace/crates/typaxis-layout-contract/src/lib.rs`
  - `workspace/Cargo.lock`
- Deliverables:
  - `ValidatedMachinePackage + MachinePdfPreflightReceipt`必須のmachine paragraph layout entry。
  - raw packageからstyle/font coverageまでを共有するpreparation boundary。
- Tasks:
  1. wrapper entryでprofile/document/style/package/session bindingを再照合する。
  2. descriptor通過後のparagraph/heading domainを既存paragraph flowへ渡し、unexpected unsupported errorを`I9190` internal mismatchへ変換する。
  3. capability成功後だけfont resource admissionを開始し、layout/finalizerへcomplete ledgerだけを渡す。
  4. 全text-producing/generation siteをtyped preorderで走査し、cascade、family、font instance bindingをpreflightする。
  5. family resolutionとglyph coverageを分離し、missing glyphがbuild-time shaping errorになり得ることをtyped outcomeへ残す。
  6. blank package、paragraph、heading、anchor/page-referenceをinternal pipeline testでPDF graphまで通す。
  7. `check-package`用boundaryはstyle/font coverageで終え、pagination/shaping/PDFを呼ばない。
- Acceptance criteria:
  - layout entryはbare `ValidatedParsedPackage` + arbitrary profile IDを受けない。
  - receipt swap/wrong fingerprintはuser input errorでなくinternal invariant errorになる。
  - check preparation successはtrusted source、semantic package、capability、resource metadata、computed familyまでを保証する。
  - unsupported content testでresource opener/layout temp/PDF temp spyが0回である。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli pipeline --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout --locked`
- Non-goals:
  - public CLI parsing/dispatch
  - final glyph coverage guarantee for `check-package`

### MI1-12 Manifest machine identityとsealed progress ledgerを実装する

- Status: Completed
- Depends on: MI1-10
- Design inputs: docs/25 §6.8、§12.2、§12.8
- Primary files:
  - `workspace/crates/typaxis-manifest/Cargo.toml`
  - `workspace/crates/typaxis-manifest/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/Cargo.lock`
- Deliverables:
  - `BuildInputProfile`、`PackageInputRecord`、machine progress admission APIs。
  - `ResourceAdmissionProgressToken`付きfailure outcome。
- Tasks:
  1. reference/machine input profileをtyped enumにし、output session作成時にresolved profileをbindする。
  2. package recordへportable URI、raw bytes/hash、optional known contract/canonical hashを持たせる。
  3. NoInputからLayoutSelectedまでのsealed monotonic progressをmanifest ledgerへadmitする。
  4. later tokenがledger既存のsession/profile/package/source factsとexact一致するか検査する。
  5. resource resolverは各successful resourceでprogress tokenを更新し、failure outcomeへ最後のtokenを返す。
  6. complete resource ledgerは同じprogressを完成/置換し、manifestへduplicate recordを作らない。
  7. `typaxis-manifest -> typaxis-machine-profile`の採択済みdependency edgeを追加し、built preflightはMI1-10の`MachinePdfPreflightReceipt`、machine provenance、resource、pagination、PDF receiptを同時照合する。profile ID/fingerprintを文字列やcaller-authored recordから再構築しない。
  8. failed preflightは到達済みfactsだけをprojectし、decode前/後のnullabilityを守る。
- Acceptance criteria:
  - callerがrecord field値を引数で渡してtrusted manifestを作れない。
  - reference modeはpackage input null、machine builtはfull non-null、machine failedはprogress相応になる。
  - package JSONをcompanion `inputs`へ重複記録しない。
  - profile/package/resource/session swapのnegative testがある。
  - MI1-10より前の仮profile値やduplicate descriptorをmanifest crateへ持ち込まない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-manifest --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission --locked`
- Non-goals:
  - manifest 1.1 wire switch/fixtures。`MI1-14`で行う。
  - output publication order

### MI1-13 Diagnostics target、read/write alias、terminal publicationを統合する

- Status: Completed
- Depends on: MI1-06、MI1-09、MI1-12
- Design inputs: docs/25 §6.6、§12.7、§12.9
- Primary files:
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-host-admission/src/lib.rs`
  - `workspace/crates/typaxis-host-admission/src/read_ledger.rs`
  - `workspace/crates/typaxis-machine-input/src/lib.rs`
  - `workspace/crates/typaxis-diagnostics/src/lib.rs`
  - `workspace/crates/typaxis-resource-admission/src/lib.rs`
  - `workspace/crates/typaxis-manifest/Cargo.toml`
  - `workspace/crates/typaxis-manifest/src/lib.rs`
  - `workspace/crates/typaxis-cli/Cargo.toml`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/sidecar.rs`
  - `workspace/crates/typaxis-cli/src/main.rs`
- Deliverables:
  - diagnostics targetを含むbuild execution context。
  - PDF outputを持たない`DiagnosticsExecutionContext`。
  - input/write alias revalidationとtyped partial publication outcomes。
- Tasks:
  1. output/trace/manifest/diagnosticsの全pairをconstruction/temp-write/publish直前にcanonical parent+leafとexisting identityで照合する。
  2. PACKAGE/configをopen前、source candidateを各open前、safe font/image全candidateをcapability gate前にread ledgerへ登録する。
  3. missing candidateもlogical targetとして保持し、`--force`がinput pathを作成しないようpublish前に再検査する。
  4. build/check failureから最後のread-ledger tokenをpublication contextへ渡す。
  5. success/failureの全canonical sidecar bytesとmanifest preflightを最初のpublish前に完成させる。
  6. failureはdiagnostics失敗後もfailed manifestを一度試行し、combined errorへ両結果を保持する。
  7. successはtrace、PDF、diagnostics、built manifestの順にし、diagnostics失敗時はbuilt manifestを出さない。
  8. stdout partial、file durability-uncertain、already-visible artifact集合を別typed variantにする。
- Acceptance criteria:
  - write target全pair、各write target対PACKAGE/source/config/resourceのlexical/symlink/hard-link/race testがある。
  - processing failureでPDFはpublishされない。
  - multi-file atomic/rollbackをAPI/docが約束しない。
  - diagnostics sidecar I/O failureはexit 3へmapping可能でprimary/secondary failureを保持する。
  - existing reference buildのmanifestなし/file/stdout publication testsが退行しない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-manifest --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli sidecar --locked`
  - `rg -n "AliasedWriteTarget|partial|durability|diagnostics" workspace/crates/typaxis-manifest/src/lib.rs workspace/crates/typaxis-cli/src`
- Implementation notes (2026-08-25, Linux 7.0.0-28-generic x86_64, rustc/cargo 1.96.1):
  - diagnostics target付きbuild context、PDF-less diagnostics context、command-wide read-ledger seal、全write/input alias再検査、pre-gate resource candidate登録を実装した。
  - terminal planはcanonical trace/PDF/manifest bytesを最初のpublish前にstageし、successをtrace→PDF→diagnostics slot→built manifest、processing failureをdiagnostics→failed manifestへ分離した。stdout partial、file durability uncertain、already-visible集合は別typed outcomeで、各fileだけが個別atomicでありmulti-file transaction/rollbackは約束しない。
  - write target全pairと、output/trace/manifest/diagnostics × PACKAGE/source/config/resourceのlexical/symlink/hard-link/publish-race matrix、missing candidate、read mutation、diagnostics failure、reference file/stdout/no-manifest regressionをtestで固定した。
  - 上記3本のmilestone verification、locked workspace all-targets check/test、all-targets clippy `-D warnings`、fmt check、`python3 schemas/validate.py`はすべてexit 0だった。test evidenceは`typaxis-core`、`typaxis-machine-input`、`typaxis-manifest`、`typaxis-cli::sidecar`および`typaxis-cli/tests/cli_end_to_end.rs`にある。
  - contract/Schema/public command surfaceは変更していない。CLIからgeneric host ownerへの禁止dependencyを追加せず、opaque publication read tokenはmanifest owner経由で渡す。
  - Completion update (2026-08-26): MI1-06/MI1-09/MI1-12を含むdependency closureとshared M1 completion evidenceを確認し、StatusをCompletedとした。
  - Scope adjustment: strict lint closureのためpreceding diagnostics APIのbehavior-preserving lint annotation/test lifetime cleanupをPrimary filesへ追加した。
- Non-goals:
  - machine CLI command exposure
  - multi-file transaction

### MI1-14 Contract 1.1へatomic migrationする

- Status: Completed
- Depends on: MI0-02、MI1-02、MI1-04、MI1-09、MI1-10、MI1-12、MI1-13
- Design inputs: docs/25 §12.4、§12.8、§12.9、Slice 1
- Primary files:
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-cli/src/config.rs`
  - `workspace/crates/typaxis-cli/src/cli.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/tests/cli_end_to_end.rs`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-diagnostics/src/lib.rs`
  - `workspace/crates/typaxis-machine-input/src/`
  - `workspace/crates/typaxis-manifest/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-syntax/src/lib.rs`（current contract epoch golden）
  - `schemas/`
  - `schemas/README.md`
  - `samples/minimal/`
  - `samples/conformance/`
  - `samples/compatibility/`
  - `samples/invalid/`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - frozen 1.0 Schema registry、current 1.1 Schema registry、dual-version validator。
  - 1.1 EffectiveConfig/diagnostics/manifest/capabilities/document-package artifacts。
  - two new configurable limits end-to-end。
- Tasks:
  1. 現行Schema一式を`schemas/1.0/`へfrozen copyし、validatorへ1.0/1.1別registryを作る。
  2. core current output contractを1.1へ切り替え、known input contractを1.0/1.1としてtyped parseする。
  3. `ResourceLimits`へ`max_document_package_bytes: u64`と`max_json_nesting_depth: u16`を追加し、default/hard maximumを`MachineInputLimitBounds`から導出する。MI1-03/MI1-10のstaging値とfield descriptorが一致しない場合はcompile/testを失敗させる。
  4. defaults、partial TOML、environment、CLI `--max-*`、EffectiveConfig JCS、Schema、manifest limit validationを同時更新する。
  5. raw 1.0 configは新二limitのdefaultを補ってenv/CLI override後に1.1 EffectiveConfigへ正規化する。
  6. diagnostics Schema/encoderをtagged `location` unionへ切り替える。
  7. manifest Schema/encoderへ`input_profile`/`package_input`とstatus/mode conditionalを追加する。
  8. `schemas/machine-capabilities.schema.json`とpositive/invalid fixtureを追加する。
  9. `dump-ast`をMI1-02 converter/encoderへ切り替え、count/hash preflight後の二回目だけstdoutへstreamし1.1を出す。
  10. 全minimal/conformance/invalid/cross fixture、JCS golden、expected errors、Schema/contract matrixを新current contractへ更新する。
  11. 1.0 DocumentPackage inputのcanonical hashが1.0 contract fieldを保持するcompatibility fixtureを追加する。
- Acceptance criteria:
  - 1.0 Schema bytes/meaningはfrozenで、1.0 fixtureを1.1 registryへ誤登録しない。
  - generated config/trace/diagnostics/manifest/document-packageは1.1だけを出す。
  - raw 1.0/1.1 configが同じsemantic valuesなら同じ1.1 EffectiveConfig/hashになる。
  - package bytes/depth exact max/max+1が`I9100`/`I9101`へmappingできる。
  - 1.0 consumerが1.1 new shapeを同じIDで受理する状態がない。
  - `python3 schemas/validate.py`が両registry、全fixture、JCS goldenを検証する。
- Verification:
  - `python3 schemas/validate.py`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo run --manifest-path workspace/Cargo.toml --package typaxis-cli -- dump-ast samples/minimal/empty.tsf --format json`
  - `rg -n "typaxis.contract/1.0" workspace schemas docs contracts samples/minimal`
- Implementation notes (2026-08-25, Linux 7.0.0-28-generic x86_64, rustc/cargo 1.96.1, Python 3.14.4):
  - 旧current七Schemaを`schemas/1.0/`へbyte-for-byteで凍結し、固定SHA-256でdriftを検出する1.0 registryと、1.1 current registryを独立構築した。validatorは1.0 compatibility packageの受理/JCS hash保持、current側での拒否、1.0を名乗る1.1 config/diagnostics/manifest shapeの拒否を検査する。
  - current outputをtyped `typaxis.contract/1.1`へatomic switchし、DocumentPackage/raw config inputは1.0/1.1をtyped parseする。raw 1.0には新limit defaultを補い、同値のraw 1.1と同じ1.1 EffectiveConfig JCS/hashへ正規化する。
  - `MachineInputLimitBounds`を唯一のdefault/hard-maximum ownerとしてpackage bytes/depth limitをTOML、environment、CLI、JCS、Schema、decoder/preflight、manifestへ接続した。exact max/max+1とpublic `I9100`/`I9101` mappingをtestで固定した。
  - diagnostics tagged location union、manifestの`input_profile`/`package_input` conditional、internal capabilities Schema/fixturesを1.1 encoderと同時に切り替えた。`dump-ast`はshared converter/encoderを使い、count/hash preflight後の二回目だけstdoutへstreamする。
  - minimal/conformance/invalid/cross fixtures、JCS golden、expected errors、contract matrixを1.1へ再生成し、1.0 canonical compatibility fixtureを別directoryへ保持した。validatorはfrozen 7/current 8 Schema、全731 `$ref`、全208 invalid fixture、全6 JCS goldenを検証する。
  - milestone verification三本、locked workspace all-targets check/test、all-targets clippy `-D warnings`、fmt check、`python3 schemas/validate.py`はすべてexit 0だった。`dump-ast`のstdoutは1.1 compact JSONで、package byte limit failureはexit 5かつstdout 0 bytesである。
  - Completion update (2026-08-26): MI0-02およびMI1-02/MI1-04/MI1-09/MI1-10/MI1-12/MI1-13のdependency closureとshared M1 completion evidenceを確認し、StatusをCompletedとした。
  - Scope adjustment: public `dump-ast`/limit failure E2Eとcurrent epoch goldenのためCLI main/E2E、machine-input、syntax、compatibility/capabilities documentationをPrimary filesへ追加した。
- Notes:
  - このmilestoneは部分commitへ分割してcurrent 1.1 IDを途中のshapeへ付けない。準備commitを使う場合も最終switch前はpublic current contractを1.0のまま保つ。
- Non-goals:
  - machine command dispatch
  - M2 wire shape

### MI1-15 Machine commandの非公開orchestrationを実装する

- Status: Completed
- Depends on: MI1-11、MI1-13、MI1-14
- Design inputs: docs/25 §6.1、§6.6、§12.7
- Primary files:
  - `workspace/crates/typaxis-cli/src/cli.rs`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-manifest/src/lib.rs`（1.1 machine terminal-plan commit gate）
- Deliverables:
  - `BuildPackageOptions`、`CheckPackageOptions`のdedicated parser helper。
  - shared internal `prepare_machine_package`とbuild/check/capabilities runner。
- Tasks:
  1. source `BuildOptions`へoptional fieldを増やさずmachine command専用optionsを作る。
  2. build-package grammarへ`PACKAGE -o OUTPUT`、`--package-root`、`--profile`、`--config`、`--resource-root`、全`--max-*`、`--strict`、`--no-compress`、`--trace`、`--trace-text`、`--emit-build-manifest`、`--emit-diagnostics`、`--force`を実装する。
  3. check-packageは`PACKAGE`、`--package-root`、`--profile`、`--config`、`--resource-root`、全`--max-*`、`--emit-diagnostics`だけを受理する。`-o`、`--strict`、`--no-compress`、`--trace`、`--trace-text`、`--emit-build-manifest`、`--force`はusage errorにし、受理後に無視しない。
  4. unknown profile/formatはusage exit 2、host unavailableはPACKAGE read前`I9110`/exit 3とする。
  5. CLI grammar成功後に一つの`MachineDiagnosticBudget`とMI1-13 execution/read-ledger contextを作り、config/target、host availability、PACKAGE admission/decode、source admission、MI1-08 parse、safe resource candidate登録、MI1-10 gate、resource/style preflightを§2.4の順で一方向に呼ぶ。
  6. check runnerはstyle/font coverage後に終了し、layout/trace/PDF/manifestを作らない。
  7. build runnerはMI1-11 machine layoutとMI1-13 terminal publicationを呼ぶ。
  8. phase ownerはM1ではsingle-threadedとし、future parallel completion orderをobservable orderingへ使わない。
  9. top-level `COMMANDS`/dispatch/helpへはまだ登録せず、crate内testからrunner/parser helperを検証する。
- Acceptance criteria:
  - source loaderとpackage loaderを共有しない。
  - same invalid inputでprimary code/side effect順がdocs/25 §6.6と一致する。
  - check successはglyph coverage、pagination、PDF successを保証すると誤記しない。
  - processing failureがMI1-12 progressをMI1-13 publisherへ渡す。
  - private runnerのpositive blank/paragraphと主要negative phase testが通る。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine --locked`
  - `rg -n "BuildPackageOptions|CheckPackageOptions|prepare_machine_package" workspace/crates/typaxis-cli/src`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_commands_remain_unregistered --locked`
- Implementation notes (2026-08-25, Linux 7.0.0-28-generic x86_64, rustc/cargo 1.96.1):
  - source commandとは別のtyped `BuildPackageOptions`/`CheckPackageOptions`とprivate parserを追加し、build側の全target/limit flag、check側のclosed受理集合、default/unknown profile、必須JSON formatをcrate内testで固定した。`build-package`/`check-package`/`capabilities`は`COMMANDS`、dispatch、helpへ登録していない。
  - 一つの`MachineDiagnosticBudget`を所有するshared preparationを、host preflight、PACKAGE stable admission、bounded decode、source admission、trusted syntax、unopened resource candidate登録とread/write alias gate、capability、resource/styleの順にsingle-threadedで接続した。resource/style failureはowner-issued partial/complete progressをmanifest admission ledgerへ渡す。
  - check runnerはcomputed family preparationで終了し、layout/trace/PDF/manifestを作らない。build runnerはreceipt-gated machine layout/PDFを呼び、failure時`diagnostics -> failed manifest`、success時`trace -> PDF -> diagnostics -> built manifest`の個別atomic publicationと最終read-ledger再検証を行う。
  - 1.1 machine output bindingをterminal planでも受理するようmanifest commit gateのobsolete reference-only rejectionを除去し、machine built planの実commit testとprivate runnerのblank/paragraph success、unsupported-inline pre-resource failureを追加した。
  - milestone verification三本、locked workspace all-targets check/test、all-targets clippy `-D warnings`、fmt check、`python3 schemas/validate.py`はすべてexit 0だった。
  - Completion update (2026-08-26): MI1-11/MI1-13/MI1-14のdependency closureとshared M1 completion evidenceを確認し、StatusをCompletedとした。Scope adjustmentとして1.1 machine terminal-plan commit gateを実装する`workspace/crates/typaxis-manifest/src/lib.rs`をPrimary filesへimplementation前に追加した。
- Non-goals:
  - public command/help exposure
  - release status update

### MI1-16 Machine package fixtureとinternal E2E matrixを閉じる

- Status: Completed
- Depends on: MI1-15
- Design inputs: docs/25 §15.1、§15.2、§16
- Primary files:
  - `samples/machine-package/`
  - `schemas/machine-fixture-expectation.schema.json`
  - `schemas/machine-fixture-matrix.schema.json`
  - `schemas/validate.py`（fixture/matrix discovery、cross-file整合性、canonical JCS）
  - `workspace/crates/typaxis-cli/src/main.rs`（private test module registration、explicit-root pre-context usage gate）
  - `workspace/crates/typaxis-cli/src/pipeline.rs`（typed capability primary code projection）
  - `workspace/crates/typaxis-cli/src/machine_tests.rs`
  - `workspace/crates/typaxis-document-package/tests/document_package_properties.rs`
  - `workspace/crates/typaxis-manifest/src/lib.rs`
  - `workspace/crates/typaxis-machine-profile/src/preflight.rs`（aggregated violationのtyped primary code）
  - `workspace/crates/typaxis-machine-profile/src/tests.rs`（failure receipt shape）
- Deliverables:
  - runnable package/source/font bundle。
  - positive、negative、tamper、limit、publication、round-trip internal E2E suite。
- Tasks:
  1. §2.6の`blank-1.0`、`blank-1.1`、`combined`へblank compatibilityとfont付きparagraph/heading/page-referenceを作り、combinedのnormalized extracted textを`Typaxis machine input`へ固定する。
  2. BOM/NUL/trailing、malformed/duplicate escaped key、unknown/missing/type/range、unknown contract、depth/bytes exact/max+1を固定する。
  3. multi-source/nonzero entry、unsafe source、hash/length、identity mapを固定し、source receipt未発行とmanifest progressを照合する。
  4. PACKAGE outside explicit root、PACKAGE symlink/unsafe open、source symlink/root escape、package/source stable-read mutation、unavailable compiled hostをそれぞれusage/`I9110`〜`I9113`の期待phase・side effectへ固定する。
  5. unsupported block/inline/style/master/imageをNodeId/global canonical orderで検査し、resource/layout/PDF spyが0回であることを確認する。
  6. raw/decoded/source/profile/resource/output receiptを別sessionで入れ替えるtamper testを追加する。
  7. diagnostics 256/max+1、advisory eviction、fatal retention、省略noteをcommand aggregateで検査する。
  8. all write-target pairsとinput candidatesのlexical/symlink/hard-link/publish-raceを検査する。
  9. diagnostics/PDF/manifest publish failureのvisible artifact集合をtyped outcomeと照合する。
  10. whitespace/member-order差、semantic差、`dump-ast -> build-package`のhash/fingerprint関係を検査する。
  11. `expected.json`とmatrixのSchema/validatorを追加し、capabilities output snapshotとadvertised item coverageをcombined fixtureへ記録する。
- Acceptance criteria:
  - docs/25 §15.1全rowにtest name/fixtureが一対一対応する。
  - combined fixtureがdescriptorの全advertised itemを使いPDF graphまで成功する。
  - failure fixtureはPDFを残さず、diagnostics/failed manifest factsがprogressと一致する。
  - testはtop-level commandをまだadvertiseせずinternal runnerを使う。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package --locked`
  - `python3 schemas/validate.py`
  - `rg -n "P1100|P1101|P1102|P1103|P1110|P1111|P1112|L5100|L5101|R7100|I9100|I9101|I9102|I9110|I9111|I9112|I9113|I9190" workspace/crates/typaxis-cli samples/machine-package`
- Implementation notes (2026-08-26, Linux 7.0.0-28-generic x86_64, rustc/cargo 1.96.1, Python 3.14.4):
  - deterministic generatorからblank 1.0/1.1、all-advertised combined、decoder/admission negative、limit、tamper、publication、round-tripの38 expectationと二つのmatrixを生成した。expectation/matrixはclosed Schemaとcanonical JCSで、validatorがcapability snapshot、resource bytes/hash、全expectationの一回だけのmatrix登録、§15.1のexact 21 row/test/fixture対応、実在するcrate内test名を双方向に検査する。
  - private runner suiteはpositiveをcheck/buildの両方へ通し、failureのexit/primary code/exact typed location/visible artifact、manifest package/source/resource progress、input contract、page countを`expected.json`と照合する。compiled-host unavailableはPACKAGEを削除したfixtureでもtest injectionが先に`I9110`を発行してfailed sidecarをpublishし、PACKAGE未readをobservableに固定した。top-level command登録は行っていない。
  - combinedはparagraph/heading、text/anchor/page reference/soft+hard break、default master/auto page、全style property、別styleで実際に選択するTrueType sfnt/TTCを一つのactual packageへ持ち、descriptorの全advertised itemとのexact集合照合後にresource admission、layout、PDF graph/serializationまで成功する。normalized extracted text expectationは`Typaxis machine input`、page countは1で固定した。
  - BOM/NUL/trailing、JSON/typed/contract、bytes/depth exactとmax+1、source closure/path/identity、contained-open/stable-read、unsupported content/style/image、diagnostic max+1、session receipt swap、全target pair/input alias、partial publication、canonical/semantic/dump-build round tripをclosure testへ接続した。DocumentPackageには0〜1024 arbitrary bytes totality、escaped Unicode duplicate key、hard depth 256/max+1、raw/canonical hash property testを追加した。
  - fixtureで見つかったorchestration差分として、explicit root外PACKAGEをcontext作成前usage gateへ移し、typed decode diagnosticをhost pathを含まないcanonical messageへ射影し、aggregated capability failureへfirst typed `L5100`/`L5101`/`R7100` primary codeを保持した。fixture readerはworkspaceのJSON dependency ownershipを守るtest-local decoderとし、新しいdependency edgeは追加していない。
  - milestone verification四本、locked workspace all-targets check/test、all-targets clippy `-D warnings`、fmt check、machine-profile regression、`git diff --check`はすべてexit 0だった。
  - Completion update (2026-08-26): MI1-15のdependency closureとshared M1 completion evidenceを確認し、StatusをCompletedとした。Scope adjustmentとしてvalidator、CLI module registration/orchestration fixes、machine-profile typed failure filesをPrimary filesへimplementation前に追加した。
- Non-goals:
  - public help/status claim
  - long-running fuzz gate

### MI1-17 Public CLI、producer docs、release gateを閉じる

- Status: Completed
- Depends on: MI1-16
- Design inputs: docs/25 §9、§10、§15.3、§16
- Primary files:
  - `workspace/crates/typaxis-cli/src/cli.rs`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`（generated-reference trace projection）
  - `workspace/crates/typaxis-cli/src/machine_tests.rs`（public capability snapshotとM2-negative assertion）
  - `workspace/crates/typaxis-cli/tests/cli_end_to_end.rs`
  - `workspace/crates/typaxis-display-list/src/lib.rs`（generated reference extraction class）
  - `workspace/crates/typaxis-pdf/src/lib.rs`（artifact extractor fallback suppression）
  - `docs/26-machine-input-cli.md`
  - `samples/machine-package/README.md`
  - `README.md`
  - `docs/19-cli.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `schemas/README.md`
  - `schemas/machine-profile-evidence.schema.json`
  - `workspace/README.md`
  - `tools/verify_machine_profile.py`
  - `tools/test_machine_profile.py`
  - `tools/test_release.py`（macOS filesystem-independent Python gate fixture）
  - `tools/verify_pdf_differential.py`
  - `tools/test_pdf_differential.py`
  - `tools/verify_reproducibility.py`
- Deliverables:
  - public `build-package`、`check-package`、`capabilities --format json`。
  - normative producer guide、support matrix、actual-host evidence。
- Tasks:
  1. top-level command list/parser/dispatch/helpへ3 commandを同時登録する。
  2. public binaryでMI1-16 positive/negative fixtureを再実行するCLI E2Eを追加する。
  3. producer guideへdirectory layout、root resolution、profile、check guarantee、diagnostic codes、manifest facts、exit codes、examplesを記載する。
  4. sample READMEへhash再生成手順とexpected command/artifactを記載する。
  5. `capabilities` outputをSchema検証し、config/filesystem/localeを読まないspy testを追加する。
  6. `verify_machine_profile.py`を追加し、単一`expected.json`または§2.6のmatrixからclean-built CLIを二回実行してPDF/trace/manifest/diagnostics/capabilitiesのbytesを比較する。
  7. 同toolからpositive PDFをMuPDF/Poppler differentialへ渡しpage count/raster/text extractionを検査する。required tool不在をsuccess skipにしない。
  8. `verify_reproducibility.py`へmachine fixture/異名checkout modeを追加し、current source binaryのversion、Git revision、SHA-256をintegration evidenceへ記録する。
  9. `schemas/machine-profile-evidence.schema.json`を追加する。`verify_machine_profile.py`を唯一のper-host evidence writerとし、`verify_reproducibility.py`のtyped resultを取り込んで、OS/arch/target triple、source revision、Cargo.lock/binary/tool hash、fixture/artifact hash、checks/resultを`target/machine-e2e/host-evidence/{target-triple}.json`へcanonical JCSでatomic出力する。
  10. GitHub Actionsを使わず、明示的に管理するclean macOS/Linux hostで同一revisionのgateを実行してhost evidenceを一つのdirectoryへcopyする。operatorは`verify_machine_profile.py --require-host-evidence DIR --required-host macos --required-host linux`を実行し、missing/failed/stale revisionを拒否する。
  11. support matrix、README、roadmap、checklistを集約済みactual E2E statusへ更新する。
  12. capabilityへM2以降のfeatureが無いことをnegative assertionで固定する。
- Acceptance criteria:
  - docs/25 §16の12項目が全てobservable gateで成功する。
  - helpとactual grammarが一致する。
  - documented macOS/Linuxでfont付きparagraph E2Eが成功する。
  - machine input対応済みの表記はmacOS/Linux evidence集約を含むこのmilestoneの全gate成功後だけ追加される。
  - worktreeからbuildしたCLIでPDFと全requested sidecarが生成される。
- Verification:
  - `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 tools/verify_machine_profile.py --repository . --fixture samples/machine-package/profiles/paragraph-1/combined/expected.json --runs 2 --require-external-tools`
  - `python3 tools/verify_reproducibility.py --repository . --revision HEAD --machine-fixture samples/machine-package/profiles/paragraph-1/combined/expected.json`
  - `python3 tools/verify_machine_profile.py --require-host-evidence target/machine-e2e/host-evidence --required-host macos --required-host linux`
- Implementation notes (2026-08-26, Linux 7.0.0-28-generic x86_64, rustc/cargo 1.96.1, Python 3.14.4, MuPDF 1.27.0, Poppler 26.01.0):
  - `build-package`、`check-package`、`capabilities`を一つのchange setでtop-level command list/parser/dispatch/helpへ登録した。large public command payloadは`Invocation` boundaryでbox化し、source commandとmachine commandのtyped grammar/loader分離を維持した。public binary E2Eはfont付きcombinedのcheck/build、PDF/trace/manifest/diagnostics facts、BOM negativeのexit 1/`P1100`/failed progress、hostile config/`TYPAXIS_*`/locale下のexact capability bytesを検査する。
  - current eleven-schema registryへclosed `typaxis.machine-profile-evidence/1`を追加した。唯一のper-host writer `verify_machine_profile.py`はcurrent worktreeからclean binaryをbuildし、positive expectedまたはmatrixをpublic commandで二回実行し、five artifactのSchema/bytes/hash、M1-only capability、manifest/PDF binding、MuPDF raster、Poppler page/text、異名source snapshot再現性を検査して、exact 14 check/six tool/five artifactをcanonical atomic evidenceへ記録する。aggregation modeはmissing/failed/noncanonical/duplicate/incomplete/stale revision、source/fixture/artifact mismatchをfail closedにする。
  - `verify_reproducibility.py --machine-fixture`はtracked+untracked non-ignored current worktreeを異なる二名へmaterializeし、fixed source remapを使う二つのclean buildについてbinary bytes/versionとfive artifact bytesをexact比較する。binary hash、Git revision、source snapshot、Cargo.lock、tool/fixture/resource/artifact hashはhost evidenceへbindする。
  - repository policyはGitHub Actions/GitHub workflowを使用しない。locked fmt/check/test/clippy、Schema/Python suite、required MuPDF/Poppler public gateは明示的に管理する各hostで実行し、canonical evidenceをoperator-controlled directoryへcopyしてaggregation commandへ渡す。completion reviewではmacOS `aarch64-apple-darwin`とmanaged Linux `aarch64-unknown-linux-gnu`のcurrent-source evidenceを生成・集約した。
  - combined fixtureのexternal gateで見つかったintegration差分として、generated-referenceをtraceへcanonical projectionしcomplete traceに`--trace-text`を要求した。generated labelはDisplayでArtifact extractionへ分類し、PDFでempty `ActualText` marked contentを付けてvisual glyphを保持しつつnormalized textから除外した。Popplerが返すunmapped artifact CIDのnonstructural C0 scalarだけをnormalizerで除去し、期待text `Typaxis machine input`を固定した。
  - producer guide、runnable sample README、README/CLI/roadmap/matrix/checklist/Schema/workspace statusをpublic Linux E2Eまで更新し、M2以降をadvertiseしないexact/negative assertionsをRustとPythonの両方へ追加した。
  - locked fmt/check/workspace all-targets test/clippy `-D warnings`、current/frozen Schema validator、21 Python tests、public machine profile gate、machine reproducibility gateはexit 0だった。required-host aggregation commandはactual macOS/Linux evidenceを受理し、missing/failed/stale evidenceを引き続きfail closedにする。
  - Completion update (2026-08-26): MI1-16のdependency closureとactual current-source macOS/Linux evidence aggregateを確認し、StatusをCompletedとした。Scope adjustmentとしてcombined fixtureから発見したtrace/extraction ownerとverifier unit tests、macOSのcase-insensitive/precomposed filesystemでも同じunsafe archive treeを作る`tools/test_release.py`をPrimary filesへ追加した。
- Non-goals:
  - full-book/release profile
  - M2 feature acceptance

## 6. M2: general flowとbasic document semantics

M2は`paragraph-1`を変更せず、採択済みADRで固定した新profileへ機能を一つずつvertical sliceで追加する。各sliceはwire/domain、preflight、layout、Display、PDF、manifest、capability、fixtureを同じchange setで閉じ、部分実装を新profileへadvertiseしない。new contractの実装は非公開stagingとし、current contract/Schema/profileの切替はMI2-08だけが行う。MI2-03〜MI2-07のpositive `typaxis-cli` testは`cli::pipeline`の`pub(crate)` staging runnerをcrate unit testから直接呼び、出力artifactをversioned non-current staging Schemaで検証する。`workspace/crates/typaxis-cli/tests/`はpublic commandがstaging IDを拒否することだけを確認する。public parser/help/current aliasへhidden optionやstaging IDを追加しない。

### MI2-01 Basic document profile ADRとclosed contractを採択する

- Status: Completed
- Depends on: MI1-17
- Design inputs: docs/25 §8 M2、§13.2、§13.5
- Primary files:
  - `adr/`
  - `contracts/phase-ownership.md`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - immutableなbasic document profile IDと、その完全な受理集合・組版policyを固定するADR。
  - style property enum拡張に必要なnew DocumentPackage contract/Schema IDとmigration table。
  - M2各sliceが満たす共通receipt/limit/publication contract。
- Tasks:
  1. repositoryで次に空いているADR番号を採番し、profile ID、style property enumを追加するnew contract/Schema ID、old/new profile compatibility、default profile、`dump-ast` output contractを記載する。M2完了まではdescriptorへ新profileを公開しない。
  2. block/inline/style/resource/page-masterの受理集合をclosed listで固定し、paragraph-1との差分を列挙する。
  3. ordered/unordered marker、連続forced page break、文書末尾forced page break、caption keep、figure oversize、empty painted item、empty link、external URI scheme/normalization、internal anchor、unknown style propertyのpolicyを一意に決める。
  4. spacing、indent、alignment、width、keepのwire tagged value、値域、initial、inherit、cascade、layout consumerを表にする。
  5. flow/list countを`max_ast_nodes`、flow depthを`max_ast_nesting_depth`、marker bytesを`max_text_buffer_bytes`/`max_text_bytes`、figure pixelsを`max_image_pixels`/`max_decoded_image_bytes`、link rectanglesを`max_fragments`/`max_pdf_objects`へmapし、各limitのconsume owner、unit、inclusive境界、error codeを固定する。意味が一致しないlimitの流用や同義field追加はしない。
  6. registry、selected state、trace、manifestがbodyと全subflowをbindする規則を固定する。
  7. 同じprofile IDの受理集合・既定policyを後から広げない互換性規則をcontract matrixへ反映する。
- Acceptance criteria:
  - 実装者が各featureのblank-page、oversize、keep、limit動作を追加判断せず実装できる。
  - profile ID、wire値、error、receipt、manifest factの命名がADR内で一意である。
  - spacing等を1.1 Schemaへ同じIDのまま追加せず、旧Schema/profileの凍結とnew contractのatomic publication順が定義される。
  - `default_profile`はcontract 1.1の`paragraph-1`から変わらない。
- Verification:
  - `rg -n "profile|page break|caption|oversize|max_ast_nodes|max_ast_nesting_depth|max_text_buffer_bytes|max_image_pixels|max_fragments|max_pdf_objects" adr contracts/machine-pdf-capabilities.md`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, macOS):
  - repositoryで次に空いていた`ADR-0028`をAcceptedとし、immutable profile `typaxis.machine-pdf/basic-document-1`、new wire contract `typaxis.contract/1.2`、versioned DocumentPackage Schema `$id` `https://schemas.typaxis.invalid/1.2/document-package.schema.json`を予約した。MI2-08まではpublic decoder/current Schema/capability/helpを1.1/`paragraph-1`のまま維持し、MI2-02〜MI2-07のcrate-private stagingだけがversioned 1.2 artifactを扱う契約とした。
  - paragraph-1との差分をlist/figure/page_break/link、八つのtyped style property、PNG XObject、internal/external annotationへclosed限定した。wire tagged value、initial/inherit/cascade/applicability、list marker/empty item、forced-break blank page、figure caption keep/oversize、URI normalization/internal anchor/empty link policyを一意に固定した。
  - body/list-item/caption registry、selected state、trace/manifest facts、receipt algorithmを全subflowへbindし、既存limitのunit/consume owner/inclusive boundaryとstable error code、no-fallback、個別atomic publication順、1.0/1.1/1.2 migration tableを固定した。contract matrixとphase ownershipも同じdecisionへ更新した。
  - milestone指定の`rg` gate、`python3 schemas/validate.py`、repository共通の`cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`はexit 0だった。Schema validatorはfrozen 1.0/current 1.1 registryと既存fixtureが不変であることを確認した。
- Non-goals:
  - table、footnote、header/footer、column、float
  - math/vector/tagged PDF

### MI2-02 Canonical multi-flow registryを実装する

- Status: Completed
- Depends on: MI2-01
- Design inputs: docs/25 §13.1
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `schemas/`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - `ValidatedFlowContent`、`ValidatedFlowContentRegistry`、`ValidatedFlowRegistry`、`ProductionFlowIrBuilder`。
  - package/epoch-boundなmulti-flow completeness receipt。
- Tasks:
  1. paragraph、list item、figure caption、page breakをexhaustive enumで表し、NodeIdごとのexpected kind、owner-local boundary count、child FlowId、LayoutEpochを登録する。
  2. Document body、list item child blocks、figure captionを別FlowIdとterminalで表し、canonical typed Document preorderでdense allocationする。
  3. builder自身がDocument indexを走査してboundaryを発行し、caller insertion順とworker completion順を入力にしない。
  4. `finish`でmissing、extra、wrong kind、wrong owner、wrong parent、wrong epoch、wrong terminalをtyped errorとして拒否する。
  5. body cursorへsubflow cursorを平坦化せず、明示flow stackとpackage/epoch-bound cursor receiptを導入する。
  6. pagination trace、selected-state fingerprint、manifest factとnew contractのstaging Schemaをregistry全FlowIdのcanonical owner順へ拡張し、current Schema aliasは切り替えない。
  7. registry count/depthをMI2-01が割り当てた既存limitに対してadmission時とfinish時の双方で照合し、上限超過時にlayoutを開始しない。
- Acceptance criteria:
  - 同じvalidated documentを異なる登録順・worker完了順で与えてもFlowId、trace、fingerprintがbyte一致する。
  - missing/extra/wrong-kind/wrong-epoch fixtureがpagination/PDF開始前に失敗する。
  - nested flowの進捗がbody cursorと独立して単調に進む。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout-contract flow_registry --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout canonical_flow --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination multi_flow --locked`
- Implementation notes (2026-08-27, Linux):
  - `FlowId`、closed owner/content kind、terminal、registry/selected-state fingerprint domainをlayout contractへ追加した。`ProductionFlowIrBuilder`はsealed packageのtyped Document preorderからbody、各list item、figure captionをdense allocationし、paragraph/list item/figure/page break receiptをcaller登録順から独立してcanonicalizeする。package/epoch-boundなnon-cloneable registry receiptを発行し、missing/extra/wrong kind/owner/parent/epoch/terminalをlayout前にtyped errorで拒否する。
  - `max_ast_nodes`と`max_ast_nesting_depth`をmodel admissionと`finish`の双方で再照合し、bodyへsubflowをflattenしないper-flow IRと明示stack cursorを追加した。独立workerの完了順をcanonicalizeしたall-flow selected-state fingerprint、全position trace、manifest projectionをreceiptから導出し、登録順とworker順を同時に変えたtestでFlowId、trace bytes、fingerprintの一致を確認した。
  - non-current `schemas/1.2/`へcommon、multi-flow trace、manifest staging Schemaを追加し、validatorでnested三flowのdense ordinal、parent/child edge、terminal、trace/manifest hash closureとcurrent 1.1 rejectionを検査した。top-level current Schema、public decoder/CLI/profileは1.1/`paragraph-1`のままである。milestone指定test、変更四crateの全test、workspace全target test、strict clippy、Schema validator、repository format checkはすべてexit 0だった。
- Non-goals:
  - table cell、footnote、header/footer、column、floatの具体layout

### MI2-03 Typed block styleをwireからlayout consumerまで閉じる

- Status: Completed
- Depends on: MI2-01
- Design inputs: docs/25 §13.2 style registry
- Primary files:
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-style/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-display-list/Cargo.toml`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/Cargo.toml`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - spacing、indent、alignment、width、keepのtyped property definitionとcomputed value。
  - propertyごとのcapability/fixture/consumer coverage table。
- Tasks:
  1. MI2-01で固定したwire tagged valueをnew contractのWire DTO、非公開staging Schema、domain、syntax loweringへ同時追加し、current Schema aliasは切り替えない。
  2. initial/inherit/cascadeをproperty registryとnon-public staging profile descriptorへ登録し、unknown nameやwrong tagged valueをpreflightで拒否する。旧profile/public capabilitiesの受理集合は変えない。
  3. fixed-point rangeとchecked arithmeticをstyle validationで確定し、layout側にraw number/stringを渡さない。
  4. computed style receiptをNodeId、package fingerprint、style registry versionへbindする。
  5. 各propertyを消費するlayout APIをtyped fieldにし、layout code内のproperty名文字列比較を禁止するtestを追加する。
  6. spacing/indent/alignment/width/keepのselected geometry/fragment/paint observationをDisplay/PDFとmanifest selected-state factへbindし、propertyがlayout後に消失または再解釈されないことを検査する。
  7. exact/min/max/max+1、inherit、override、unknown、unused-advertised-property、page split、PDF observationのfixtureを追加する。
- Acceptance criteria:
  - staging descriptorに列挙したpropertyはwireからlayout、Display、PDF、manifest observationまでpositive testを持つ。
  - unknown/unsupported propertyはlayout開始前にprimary diagnosticを一件発行する。
  - paragraph-1で新propertyを受理しないregression testが通る。
  - MI2-08前のpublic current contract/Schema/capabilities bytesが変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-style machine_properties --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax machine_properties --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile basic_document_styles --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout typed_style_consumers --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf machine_block_styles --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_block_styles --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, Linux):
  - contract 1.2専用のstrict decoder/encoderとversioned DocumentPackage Schemaへ八propertyのexact name、tag、keyword、inclusive fixed-point rangeを追加した。current decoder/encoder、1.1 Schema、`paragraph-1` descriptorは同じ入力を拒否し続ける。staging syntaxはwireから既存domainへlossless loweringし、computed valueをowner/style owner、canonical package、document/style fingerprint、immutable registry versionへbindするsealed receiptだけをlayoutへ渡す。
  - registryはinitial、`text_align`だけのtyped flow-owner inheritance、既存cascade priority、selector applicability、sole consumerをclosed列挙する。non-public `basic-document-1` style preflightはunsupported selector/propertyとfigureの`auto` widthをlayout前の`L5101`にし、layout consumerはchecked fixed-point arithmeticだけでspacing suppression、logical LTR/RTL indent、center odd-unit placement、figure width、keep facts、page splitを選択する。production layout sourceに八property名の文字列比較が無いこともtestで固定した。
  - selected receiptからだけDisplay factを、DisplayからだけPDF geometry/content-stream observationを、PDFからだけcontract 1.2 manifest factを導出し、package/registry hashと全八propertyのconsumer/Display/PDF/manifest coverageをstaging fixtureで照合した。crate-private CLI runnerのpositive chainはdeterministic bytesまで検査し、public CLI E2Eはcontract 1.2と`basic-document-1`をMI2-08まで拒否することを確認する。
  - milestone指定の七command、`cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`、dependency-edge guard、`cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`、format check、`git diff --check`はすべてexit 0だった。Schema validatorは5 non-current 1.2 Schemas、953 refs、exact/max+1/wrong-tag/unknown/current-rejection fixtureを含む全registryを検証した。
- Non-goals:
  - table border collapse、cell vertical alignment

### MI2-04 Listを独立subflowのvertical sliceとして実装する

- Status: Completed
- Depends on: MI2-02, MI2-03
- Design inputs: docs/25 §13.2 list
- Primary files:
  - `workspace/crates/typaxis-diagnostics/src/`
  - `workspace/crates/typaxis-style/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - ordered/unordered/nested listのvalidated layout、fragment、Display、PDF closure。
- Tasks:
  1. non-public staging profile descriptorへ採択list kind/policyをclosed登録し、`ordered/start/item_index`からmarkerをchecked生成してcaller-provided markerを受理しない。旧profile/public capabilitiesはlistを拒否し続ける。
  2. marker bytesを`GeneratedBufferKey`とresource/usage ledgerへ登録し、overflowとlimit max+1をpreflightで拒否する。
  3. 各item child blocksを独立subflowへ登録し、nested listを再帰呼出しではなくbounded flow stackで処理する。
  4. markerとitem最初のpainted lineを同一fragment receiptへbindし、marker orphanをselected-state validatorで拒否する。
  5. empty painted itemとpage末splitにMI2-01のpolicyを適用し、同一candidateを`More`で返す無進捗を検出する。
  6. marker/indent/alignmentをexact placementへ変換し、selected list/item FlowIdとmarker usageをmanifestへbindしてversioned staging Schemaで検証し、Display/PDFのmissing/extra/wrong-itemを閉じる。current manifest Schema aliasは変えない。
  7. single、nested、page split、empty、marker overflow、exact limit、tamper E2Eを追加する。
- Acceptance criteria:
  - list itemとnested subflowがtrace/manifest/PDFで同じFlowId/fragmentへbindされる。
  - markerだけがpage末に残らない。
  - staging descriptorに列挙したlist fixtureがinternal runnerの二重buildで全artifact byte一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout list --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination list --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_list --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, Linux):
  - syntax trust boundaryへpackage-boundな`ValidatedStagingListMarkerUsageReceipt`を追加し、orderedのchecked `start + item_index`と`.`、unorderedのU+2022をcaller markerなしで導出する。marker string allocation前に一bufferとparsed+generated aggregateを`max_text_buffer_bytes`/`max_text_bytes`へ照合し、profile preflightがoverflow=`L5100`、missing text style=`L5101`、limit=`T2100`/`T2101`を発行する。closed staging descriptorだけがordered/unordered/nested policyを列挙し、1.1 decoder、`paragraph-1` descriptor、public capabilities/CLIはlist/1.2/`basic-document-1`を拒否し続ける。
  - MI2-02のcanonical item FlowIdを使い、各listのwidest marker column、font-size gap、logical start/end indent、LTR/RTL end alignmentをchecked fixed-point geometryへ変換した。nested listはbody幅を再利用せずparent itemのchild-flow frameに対して自身のindentを適用する。paginationはbounded `MultiFlowCursorReceipt` stackで全terminalを閉じ、markerとfirst painted lineを同じfirst fragmentへsealしてpage末では一緒に移動する。empty painted item、empty-frameに収まらないkeep、fragment max+1、同一candidate無進捗、marker orphanはterminal errorになる。
  - selected receiptからtrace/Display、DisplayからPDF marker observation、PDFからmanifestだけを導出し、list/item FlowId、fragment、marker key/bytes/usage hash、exact geometryを保持する。Displayのmissing/extra/wrong-item closureを専用testで固定した。versioned `machine-list-manifest.schema.json`とsingle/nested/page-split/empty/overflow/exact/max+1/tamper expectationを追加し、checked-in selected-stateはcrate-private runnerが生成したcanonical JCS goldenで二重buildの全artifact byte一致を検査する。
  - milestone指定の四command、machine-profile/Display/PDF/manifest tests、public-negative CLI E2E、workspace全target test、dependency-edge guard、strict clippy、Schema validator、repository format/diff checkはすべてexit 0だった。layout/paginationからprofile crateへの逆向き依存は作らず、profile-issued receiptのsyntax-owned lower projectionだけをlayoutが消費する。
- Non-goals:
  - arbitrary caller marker text
  - table/footnote内listのrelease claim

### MI2-05 Forced page breakを進捗保証付きvertical sliceとして実装する

- Status: Completed
- Depends on: MI2-02
- Design inputs: docs/25 §13.2 page break
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - zero-size contentと区別されたtyped forced-boundaryとconsume receipt。
- Tasks:
  1. non-public staging profile descriptorへ採択forced-break policyをclosed登録し、PageBreakを独立`ValidatedFlowContent` variantとしてNodeId/FlowId/LayoutEpochへbindする。旧profile/public capabilitiesはPageBreakを拒否し続ける。
  2. empty frame先頭でも一度だけconsumeし、next cursorがstrictly advanceしたことをreceiptで証明する。
  3. 連続breakとdocument末尾breakへMI2-01で採択したblank-page policyを適用する。
  4. traceへbreak source、before/after cursor、produced page ordinalを記録し、selected state、manifest、PDF page countへbindしてversioned staging Schemaで検証する。Displayにはpaint opを発行せず、break由来のextra paintをclosure errorにし、current manifest Schema aliasは変えない。
  5. start、middle、consecutive、trailing、max/max+1、cursor tamper fixtureを追加する。
- Acceptance criteria:
  - 同じcursorで`More`を返す経路がない。
  - blank-page policyがcapability、trace、PDF page count、fixtureで一致する。
  - paragraph-1はPageBreakを引き続きpreflightで拒否する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination forced_page_break --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list forced_page_break --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_page_break --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, Linux):
  - syntaxがcontract 1.2 staging packageのforced-break owner集合とusage hashをcanonical Document順でsealし、private `basic-document-1` descriptorが「open pageから開始、leading/consecutive/trailing blankを保持、1 breakにつきcursorを1回進める、Display paintなし」をclosed policyとしてwrapする。公開`paragraph-1` descriptor/capabilities、1.1 decoder/current Schema alias、CLI routeは変更せずPageBreakを拒否し続ける。
  - layoutはMI2-02の独立`ValidatedFlowContent::PageBreak`をpackage/NodeId/FlowId/LayoutEpoch/flow-local ordinalへ投影するgeometryなしのtyped boundary receiptを発行する。paginationはbounded multi-flow cursorでbreakをちょうど1回consumeし、before/after cursorの同一FlowIdかつ`+1`をreceiptで検証してからpageを開く。leading、consecutive、trailingを含む`N` breakはopen final pageを保持して`N + 1` pageとなり、page max+1とstale cursorはterminal errorになる。
  - selected trace、paint-op空のDisplay、PDF `/Count` observation、versioned forced-page-break trace/manifest Schemaへbreak source/cursor/produced pageとblank factsを同じreceipt chainから伝播した。checked-in fixtureは4 break/5 page、blank index 0/2/4を固定し、二重build byte一致、break由来extra paint、exact/max+1、cursor tamper、current 1.1/public negativeを検査する。
  - milestone指定の四command、workspace全target test、dependency-edge guard、strict clippy、Schema validator、format check、`git diff --check`はすべてexit 0だった。
- Non-goals:
  - named page/master切替
  - recto/verso break

### MI2-06 Non-floating PNG figure/captionをE2Eで実装する

- Status: Completed
- Depends on: MI2-02, MI2-03
- Design inputs: docs/25 §7 figure/assets、§13.2 figure/PNG
- Primary files:
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-resources/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - admitted PNG、caption subflow、alt text、exact placementをbindする`ValidatedFigureLayout`。
  - PDF image XObjectまでのresource closure。
- Tasks:
  1. PNG decoderだけが発行できるclosed internal `AdmittedImageMediaKind::Png`を追加し、pixel width/height、encoded bytes hash、`ImageResourceId`とともにstable-read ledgerへbindする。non-public staging profile descriptorへPNG figure/captionだけをclosed登録する。M2 contractには宣言media fieldを後付けせず、URI suffixやcaller文字列をmedia attestationに使わず、旧profile/public capabilitiesはfigure/imageを拒否し続ける。
  2. computed widthを必須にし、heightをpixel aspect ratioからfixed-point checked roundingで導出する。暗黙DPIを導入しない。
  3. figure owner、ImageResourceId、caption FlowId、alt text、keep/oversize policyを一つのvalidated receiptへbindする。
  4. non-floating block placementだけを実装し、float/unsupported fit policyをpreflightで拒否する。
  5. captionが同一pageに収まらない場合にMI2-01のtyped keep policyを適用し、無進捗oversizeを一度だけterminalへ遷移する。
  6. Displayへexact placementから`DrawImage`を一件発行し、figure/placement/`attested_media_kind = png`/hash factをmanifestへbindしてversioned staging Schemaで検証し、usage collector、admitted ledger、late finalizer、PDF XObjectのmissing/extra/wrong-IDを閉じる。current manifest Schema aliasはMI2-08まで変えず、M4より前のmanifestへcaller-declared media fieldを作らない。
  7. valid、caption split/keep、bad hash、non-PNG bytes、invalid dimensions、pixel limit、wrong `ImageResourceId`、publication failure E2Eを追加する。
- Acceptance criteria:
  - figureのsource resourceからPDF XObjectまでhash/ID/placementがreceipt chainで追跡できる。
  - missing/extra/wrong-IDはfinalized PDF bytesの最初のpublish前に拒否する。
  - same inputでfigure寸法とPDF bytesがplatform非依存で一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission png --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout figure --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf image_xobject --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_figure --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, Linux):
  - stable-read image bytesをfull bounded PNG decodeへ通したownerだけが`AdmittedImageMediaKind::Png`を発行し、`ImageResourceId`、encoded hash/length、nonzero pixel dimensions、canonical decoded-byte budgetをledger fingerprintへbindする。宣言は従来のID/URI/optional hashだけで、opaqueな`figure.data` fixtureによりsuffix/media文字列推論がないことを固定した。private `basic-document-1` descriptorはdocument-bodyのnon-floating Figureとparagraph/heading captionだけを採択し、fit-likeなFigure `keep_with_next`をlayout前にrejectする。公開1.1 decoder、`paragraph-1`、capabilities、manifest aliasは変更していない。
  - `ValidatedFigureLayout`はpackage/usage/admitted-ledger/LayoutEpoch/Flow registryを閉じ、figure owner/ordinal、image ID/hash/media/pixels、alt、caption child FlowId/owners、computed width、logical indent、spacing、typed keep/terminal-oversize policyを一receiptへ保持する。heightはDPIなしで`width * pixel_height / pixel_width`をchecked i128とties-to-evenだけで丸める。paginationはblock placementのみを行い、caption splitまたはimage+caption keepをpage境界で適用し、image/caption/kept groupのoversizeをretryしないterminal errorにする。
  - selected receiptから各Figure exactly one `DrawImage`を作り、missing/extra/wrong draw IDをDisplay前に拒否する。late PNG finalizerはadmitted media/hash/dimensionsとusage setを再照合し、PDF stageはlogical binding/resource name、main image/soft-mask object graph、serializer receiptのXObject emission、page/object/hashをpublish前に閉じる。PDF-derived staging manifestとversioned `machine-figure-manifest.schema.json`だけが`attested_media_kind = png`、source/caption/placement、XObject/PDF factsを公開する。
  - checked-in 2x1 palette+tRNS fixtureは40x20 placement、page-2 caption split、one DrawImage、main+soft-mask XObject、canonical PDF hash/bytesの二重build一致を固定する。focused E2Eはcaption split/keep、terminal oversize、bad hash、non-PNG、invalid dimensions、pixel/decoded limits、missing/extra/wrong image ID、missing/extra/wrong XObject、partial publication failure、current public negativeを覆う。milestone指定の五command、workspace全target test、dependency guard、strict clippy、Schema validator、format/diff checkはすべてlocal exit 0で確認した。
- Non-goals:
  - inline image、float、SVG、JPEG

### MI2-07 Link annotationとnamed destinationをE2Eで実装する

- Status: Completed
- Depends on: MI2-01
- Design inputs: docs/25 §7 links、§13.2 link
- Primary files:
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-linebreak/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - logical cluster rangeからpage-local annotation rectangleまでのlink receipt。
  - internal named destinationとvalidated external `SafeUri`。
- Tasks:
  1. non-public staging profile descriptorへ採択internal/external link policyをclosed登録し、link child rangeをparagraph itemization時にlogical cluster rangeへbindしてempty childrenをpreflightで拒否する。旧profile/public capabilitiesはlink annotationを拒否し続ける。
  2. internal targetをpackage anchor registryのselected named destinationへ解決し、missing/duplicate/wrong-package anchorを拒否する。
  3. external URIをMI2-01のscheme/normalization policyで`SafeUri`へ変換し、raw stringをPDF writerへ渡さない。
  4. selected lineごとにpainted visual cluster rectangleのcanonical unionを作り、page bounds、non-empty、rect count limitを検査する。
  5. link target/cluster/page/rectangleをmanifest selected-state factへbindしてversioned staging Schemaで検証し、Display/PDF closureでlinkごとにannotationを一件以上要求してmissing、extra、wrong page、wrong targetを拒否する。current manifest Schema aliasは変えない。
  6. wrapped link、internal/external、empty/unpainted、bad URI、bad target、rect tamper、exact limit E2Eを追加する。
- Acceptance criteria:
  - annotation rectangleがselected glyph clusterと同じpage/line receiptへbindされる。
  - PDF validatorでinternal destinationとexternal URIが期待値に一致する。
  - unsupported URI schemeはlayout開始前に拒否される。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-linebreak link_clusters --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf annotations --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_link --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-27, Linux):
  - private `basic-document-1` link descriptorとsyntax preflightは、empty/unpainted/nested/unsupported childをlayout前に閉じ、internal targetをcanonical package anchor ownerへ、external targetをMI2-01の`SafeUri`へbindする。receiptはpackage fingerprintと全anchor/link usage setを保持するため、別packageの同名anchorへ差し替えられない。公開1.1 decoder、`paragraph-1` descriptor、capabilities、current Schema/manifest aliasは変更していない。
  - selected paragraph item registryから各link childのlogical shaping cluster rangeを確定し、selected reflowのL1/final reshape/justification/L2 visual orderと同じclusterをpage/line単位のpositive rectangleへcanonical unionする。Display closureは各link一件以上とexact missing/extra/page/target/rectangleを、PDF closureはnamed destination、page `/Annots`、indirect `/Subtype /Link` dictionary、internal `/Dest`またはexternal `/URI`、serializer bytesをpublish前に再照合する。
  - PDF-derived `machine-link-manifest/1`とversioned staging Schemaはpackage/usage/cluster/layout/Display/PDF hash、logical range、target、page/line rectangle、annotation object ID、destination owner/pointを一つのselected-state factへbindする。checked fixtureはuppercase raw schemeを`https`へ正規化し、同一line上の二linkとwrapped external linkから三rectangleを作る。focused testsはpreflight failures、wrong-package receipt、exact rectangle/PDF-object limits、全annotation tamper、deterministic PDF golden、public negativeを覆う。milestone指定の四command、profile receipt test、workspace全target test、workspace全target strict clippy、format/diff checkはすべてlocal exit 0で確認した。
- Non-goals:
  - JavaScript/action annotation
  - arbitrary PDF destination syntax

### MI2-08 Basic document profileを統合・公開する

- Status: Completed
- Depends on: MI2-04, MI2-05, MI2-06, MI2-07
- Design inputs: docs/25 §8 M2、§13.5
- Primary files:
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `docs/26-machine-input-cli.md`
  - `schemas/`
  - `schemas/README.md`
  - `tools/verify_machine_profile.py`
- Deliverables:
  - MI2-01のimmutable profile descriptorとall-advertised combined fixture。
  - MI2-01で採択したnew contract/Schemaのatomic migrationと旧contract/profile golden。
  - M2 support matrix、producer guide、trace/manifest Schema fixture。
- Tasks:
  1. MI2全sliceのpositive、unsupported/tamper、page split、exact/max+1 fixtureをprofile test matrixへ登録する。
  2. list、forced break、PNG figure/caption、internal/external link、全advertised styleを一つのpackageで使用するcombined E2Eを追加する。
  3. descriptorのfeature/style/resource/limit集合をfixture coverageと双方向照合し、advertised-but-untestedとtested-but-unadvertisedを失敗させる。
  4. preflight後かつlayout前のcapability receiptをmanifestへbindし、resolved profile requestとの不一致をtamper testにする。
  5. previous current Schemaをversion directoryへfreezeし、new contract constant/Schema registry/Wire serializer/decoder/`dump-ast`/capability/manifest/fixturesを同じchange setで追加する。同じcommitでcrate-private staging runnerの専用入口を外し、通常pipelineがnew current contract/profileを選択できるようにする。hidden selectorは残さない。
  6. paragraph-1を含む旧profileのcapability bytes、contract受理/拒否集合、default statusをMI2-01のmigration tableどおりgolden testで固定する。
  7. combined packageを二重build・異名checkout・documented hostsで実行し、PDF/trace/manifest/diagnosticsを比較する。
  8. actual profile IDとcombined fixtureを`samples/machine-package/matrices/m2-basic.json`へ登録する。
  9. 全gate成功後にだけnew contract、capabilities、support matrix、producer guideへprofileを公開する。
- Acceptance criteria:
  - capability JSONに列挙した全機能のcombined packageがPDFまで成功する。
  - unknown/unsupported featureがlayout/resource/PDF開始前に拒否される。
  - previous contract/Profile IDの意味は変わらず、new contractのdefault/compatibilityはMI2-01のmigration tableと一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 tools/verify_machine_profile.py --repository . --matrix samples/machine-package/matrices/m2-basic.json --runs 2 --require-external-tools`
- Implementation notes (2026-08-27, Linux):
  - current wire/outputを`typaxis.contract/1.2`へatomic switchし、former current十一Schemaを`schemas/1.1/`へhash固定した。top-level current aliasとcomplete `schemas/1.2/` registryは一致し、DocumentPackage/configは1.0/1.1/1.2のclosed setを受理する。`paragraph-1`は旧semantic bytesとdefault statusを維持し、explicit `basic-document-1`だけがraw 1.2を要求する。
  - `basic-document-1`を通常の`build-package`/`check-package` pipelineへ統合し、専用staging runner入口とhidden selectorを残さなかった。preflight receipt hashをpackage/trace/manifest layoutへ、canonical all-flow registry hashをtrace/manifestへbindし、profile/package/session/flow差し替えを`I9190` closure testで拒否する。
  - `profiles/basic-document-1/combined`はlist、forced break、PNG/caption、internal/external link、全advertised selector/property、sfnt/TTCを一packageで使用し、二page PDFまで到達する。descriptorとfixture coverageは双方向exact照合され、`m2-basic.json`にはcombined positiveとold-contract `/contract` negativeを登録した。Popplerのlayout由来tab/line/page separatorだけを一spaceへ正規化し、page countは独立にexact検査するregression testも追加した。
  - workspace全target test、strict clippy、format check、Schema validator（7 frozen 1.0、11 frozen 1.1、11 current alias、17 versioned 1.2、41 machine expectations）、およびexternal MuPDF/Popplerを含む二重build・異名checkout verifierはすべてlocal exit 0。host evidenceは`target/machine-e2e/host-evidence/x86_64-unknown-linux-gnu.json`へcanonical publishされた。
- Non-goals:
  - M3以降のfeature advertising

## 7. M3: table、footnote、advanced pagination

M3も既存profileを変更せず、table、footnote、advanced paginationをそれぞれADRで閉じたprofileとして扱う。採用しないpolicyはdefault動作へ丸めず、該当profileのpreflightで拒否する。table/footnote profileはMI2 current contractに既にあるwire shapeだけを使い、new wire/style fieldを追加しない。advanced pagination用new contractは非公開stagingとし、current contract/Schemaの切替はMI3-12だけが行う。MI3-09〜MI3-11のpositive CLI testsも同じcrate-private staging runnerを使い、integration testsはpublic parser/help/capabilitiesが新contract/profileを拒否することを確認する。table/footnote ADRがnew wire fieldを必要と判断した場合はMI3-02/MI3-06開始前に本task graphへ別contract migrationを追加し、暗黙にcurrent IDを拡張しない。

### MI3-01 Table profile ADRとgrid policyを採択する

- Status: Completed
- Depends on: MI2-08
- Design inputs: docs/25 §8 M3、§13.1、§13.3 table
- Primary files:
  - `adr/`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - MI2 current contract上のimmutable table profile ID、wire/style受理集合、grid/fragment/paint policy。
- Tasks:
  1. fixed/fraction column wire型、unit、値域、canonical rounding、residual割当を固定する。
  2. cell origin、row/column span、overlap、hole、header row、owner関係のgrid validation規則を固定する。
  3. split可能cell、split禁止cell、row oversize、rowspan continuation、repeated headerの挙動とlimitを固定する。
  4. 初期profileのtable固有visual policyを`border = none`、`background = transparent`、`cell padding = 0`、`vertical alignment = block-start`へ固定する。table固有wire/style propertyは追加せず、可変border/padding/alignment/backgroundをfuture new contract/profileとして明示拒否する。
  5. table row/column/cell countを`max_ast_nodes`、row fragmentsを`max_fragments`、rowspanをdeclared row countへbindし、各existing limitのconsume owner、inclusive境界、diagnostic codeを固定する。意味が一致しないlimitの流用や同義field追加はしない。
  6. grid、cell subflow、row fragment、header repetition、Display/PDFを結ぶreceipt chainを定義する。
- Acceptance criteria:
  - column residual、oversize、split、header repeat、zero-decoration paintに一意な結果がある。
  - unsupported table propertyを黙ってdefaultへ落とす余地がない。
  - MI2 current contract/Schema bytesを変えずにprofileを実装できる。new fieldが必要になった場合のtask graph更新条件がADRにある。
  - profile公開条件がcombined fixtureとclosure検査まで含む。
- Verification:
  - `rg -n "column|fraction|residual|rowspan|header|oversize|border = none|background = transparent|padding = 0|block-start|max_ast_nodes|max_fragments" adr contracts/machine-pdf-capabilities.md`
- Implementation notes (2026-08-27, Linux):
  - repositoryで次に空いていた`ADR-0029`をAcceptedとし、immutable target `typaxis.machine-pdf/table-1`をcurrent `typaxis.contract/1.2`上へ固定した。profileはM2 domainにdirect document-body tableだけを加え、cellをcurrent paragraph wireへ限定する。MI3-04まではpublic ID/help/capabilityへ登録せず、`paragraph-1` defaultと両public profileのtable rejectionを維持する。
  - fixedはpositive `pdf_point_1_65536`、fraction weightは`1..=65535`とし、checked `i128` ties-to-even shareのsigned residualをwire順のlast fraction columnだけへ割り当てる。leftmost-free origin、section内rowspan、overlap/hole禁止、deficit-to-last-row band、common legal cut、one-shot oversize、dense header repetition、zero-decoration (`border = none`、`background = transparent`、`cell padding = 0`、`vertical alignment = block-start`)を一意に固定した。table固有style/split fieldはunknown 1.2 memberとして拒否し、必要ならMI3-02/MI3-03前に別contract migration taskを追加する。
  - columnを既存`max_ast_nodes`へ追加chargeし、row/cellの既存chargeを重複させず、body/header row fragmentを`max_fragments`へ、rowspanをdeclared section row countへbindした。profile/grid/cell Flow/row band/row fragment/rowspan/header/selected layout/Display/PDF/trace/manifestのreceipt chain、`P1120`/`L5110`/`P1102`/`L5100`/`L5101`/`I9190`、combined fixtureとbidirectional closureを含むMI3-04 publication gateをcapability contractとmatrixへ反映した。
  - milestone指定の`rg` gate、`python3 schemas/validate.py`、`cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`、whitespace/diff checkはlocal exit 0だった。current aliasとversioned 1.2 DocumentPackage SchemaはいずれもSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままで、MI3-01はSchema bytesを変更していない。
- Non-goals:
  - footnote、header/footer、column、float

### MI3-02 Table column/grid/cell subflowを実装する

- Status: Completed
- Depends on: MI3-01
- Design inputs: docs/25 §13.1、§13.3 table steps 1-2
- Primary files:
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-style/src/`
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-layout/src/`
- Deliverables:
  - validated column resolution、grid receipt、cell FlowId/frame。
- Tasks:
  1. available inline sizeからfixed columnsをchecked subtractionし、remainingをfraction weightへcanonical roundingする。
  2. rounding residualをMI3-01で指定したcolumnだけへ割り当て、sumがavailable inline sizeと一致するreceiptを発行する。
  3. cell origin/spanからdense gridを検査し、overlap、out-of-range、hole、wrong ownerをlayout開始前に拒否する。
  4. 各cellを独立subflow/frameへ登録し、owner/row/column/span/package/epochをFlowIdへbindする。
  5. MI3-01のfixed zero paddingと実装済みM2 typed styleだけをcontent frameへchecked適用し、table固有raw propertyやnegative sizeを受け付けない。
  6. exact/max/max+1、rounding residual、span、overlap/hole、wrong receipt testを追加する。
- Acceptance criteria:
  - column widthの和がexactにavailable inline sizeへ一致する。
  - cellのcaller登録順がFlowId、layout、traceへ影響しない。
  - malformed gridはcell contentをlayoutしない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout table_grid --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout-contract table_receipts --locked`
- Implementation notes (2026-08-27, Linux):
  - strict decoderの`max_ast_nodes` ownerへNodeIdを持たない各table columnを1 unitとして追加し、location index用semantic node countとは分離した。private table prevalidationでもsemantic node total + column countをgrid vector/Cell `FlowId` allocation前に再検査し、exact maxを受理、max+1を拒否する。
  - table styleはpublic `basic-document-1`を拡張せず、private sealed receiptで既存M2の`page = auto`、spacing、logical indent、`keep_with_next`だけをtyped computed valueへ閉じた。cell paragraph用parent receiptもtable owner/package/styleへbindし、table固有raw property/negative geometryの入力経路を追加していない。
  - 全direct body tableのone-dimensional gridをcell layout前に検査し、head/body別のleftmost-free origin、colspan/rowspan、overlap、out-of-range、hole、row/cell ownerを固定した。canonical flow registryへ`TableRow` contentとdense `TableCell` child flowを追加し、rowごとの全cell FlowId、owner、parent、terminal、package、`LayoutEpoch`をcaller registration順から独立して発行する。
  - fixedをchecked subtractionし、fraction shareをchecked `i128` ties-to-evenで丸め、signed residualをwire順last fractionだけへ加えた。`ValidatedTableGridReceipt`はinput/rounded/final column、exact sum、residual recipient、row/cell span、cell FlowId/terminal、zero-padding/block-start frame、typed table spacing/indentを再導出・fingerprintする。
  - milestone指定の2 command、affected crate full tests (`typaxis-layout`、`typaxis-layout-contract`、`typaxis-style`、`typaxis-syntax`、`typaxis-document-package`)、workspace all-target check、Schema validation、format/diff checkをlocal実行した。rounding positive/negative residual、last-fraction-before-fixed、fixed exact/safe max/over、colspan/rowspan、overlap/hole/out-of-range、wrong style/flow/row receipt、reverse registration、AST exact/max+1を含む。current alias/versioned 1.2 DocumentPackage SchemaはいずれもSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままである。
- Non-goals:
  - row fragmentation、header repetition、paint

### MI3-03 Row fragmentation、rowspan continuation、header repeatを実装する

- Status: Completed
- Depends on: MI3-02
- Design inputs: docs/25 §13.3 table step 3
- Primary files:
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-manifest/src/`
- Deliverables:
  - common break candidate選択、row continuation state、header repetition receipt。
- Tasks:
  1. 全active cellのnext break candidateから共通block sizeをcanonicalに選び、各cell cursorのbefore/afterをrow fragmentへbindする。
  2. rowspan continuationをboundedな一次元stateとして次row/pageへ渡し、recursive grid stateを持ち込まない。
  3. split禁止cellを含むrow、empty pageでも一行進めないrowをMI3-01のoversize terminalへ一度だけ遷移する。
  4. repeated headerをcloneではなくoriginal header subflow、source fragment、repetition index、target pageへbindする。
  5. row/header/cellのselected fragmentを全subflow selected state、trace、manifestへbindする。
  6. zero-progress、same-candidate retry、duplicate/missing continuation、wrong repetition indexをnegative testにする。
  7. multi-page、rowspan、header repeat、split禁止、oversize、fragment exact limit E2Eを追加する。
- Acceptance criteria:
  - 各successful pagination stepでrow cursorまたはterminal stateがstrictly advanceする。
  - repeated headerがoriginal header内容とtamper不能に結ばれる。
  - row fragmentの全active cellが同じselected block extentと整合する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination table_fragmentation --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout table_rowspan --locked`
- Implementation notes (2026-08-27, Linux):
  - canonical cell orderのindivisible fragment sizeを`TableCellLayoutReceipt`の累積break endpointへsealし、row bandをsectionごとにzero初期化した。cellを`(origin_row, origin_column)`順に処理し、rowspan covered bandとの差分をlast covered rowだけへchecked加算する。empty cellはzero extent/terminalのままでminimum heightを発行しない。
  - body rowがremaining frameへ収まらない場合はframe extentと全active cell endpointの有限集合を降順評価し、全cellで合法なgreatest positive cutだけを選択する。全cell receiptへ同じselected block extentとflow cursor/vertical offsetのbefore/afterをbindし、zero-height rowはzero-size structural fragmentでlogical cursorを進める。
  - continuationはcolumn順の一次元`RowspanContinuationReceipt`だけを運び、cell owner、FlowId、cell cursor、vertical offset、remaining logical rowsを各covered columnへbindした。physical splitではlogical rowを保持してfragment ordinalを進め、logical completionではrowspanを一度だけ減らし、final covered rowのcell terminalを明示検査する。missing、duplicate、wrong-owner/resurrected stateは`I9190`で閉じる。
  - complete head groupはbody progressを先に証明してからmaterializeし、first occurrenceのheader row/cell/subflow fragmentをsource receiptとして保持する。各later occurrenceは新しいAST/NodeId/FlowIdを作らず、source fragment、連続repetition index、target page、`typaxis.table-selected-layout/1` fingerprintだけへbindする。
  - `SelectedTableLayoutReceipt`はgrid/row-band/all-flow terminal receipt、全row/cell fragment、continuation、header occurrence、pageを一方向にsealする。同じfactsをtable traceへ埋め込み、manifest projectionはflow/grid/row-band/selected hash一致後にrow/header/cell factsをcopyするため、trace/manifest側のgeometry再解釈経路を持たない。
  - cell内paragraph fragmentの既存消費数とbody/header row recordを同じ`max_fragments` stateへ合算し、各row/header ID issuance前にmax+1を`L5110`で拒否する。multi-page、rowspan、header repeat、common cut、split-prohibited/empty-frame oversize、zero-height progress、exact combined limit、same-candidate retry、missing/duplicate/wrong continuation、header source/repetition tamper、deterministic fingerprint、manifest closureをlocal testへ追加した。
  - milestone指定の2 command、manifest table projection test、locked workspace all-target tests、workspace all-target clippy `-D warnings`、format/diff check、`python3 schemas/validate.py`はすべてlocal exit 0だった。current aliasとversioned 1.2 DocumentPackage SchemaはともにSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままで、MI3-03はSchema bytesやpublic profile surfaceを変更していない。
- Non-goals:
  - footnote inside cellのrelease claim

### MI3-04 Table Display/PDF closureとprofileを公開する

- Status: Completed
- Depends on: MI3-03
- Design inputs: docs/25 §8 M3、§13.3 table
- Primary files:
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `docs/26-machine-input-cli.md`
  - `schemas/`
  - `tools/verify_machine_profile.py`
- Deliverables:
  - table contentとfixed zero-decoration policyのpaint closure。
  - immutable table profile descriptorとcombined table fixture。
- Tasks:
  1. selected grid/row/cell fragmentからcanonical paint orderとexact rectanglesを生成する。
  2. border/background paint opが0件であることをselected table policyへbindし、callerが追加したdecoration opをextraとして拒否する。
  3. Display/PDF closureでmissing/extra/wrong-cell/wrong-page/wrong-repetitionを拒否する。
  4. table-onlyとM2 feature併用fixtureでtrace、manifest、Display、PDF selected stateを照合する。
  5. descriptorとfixture coverageの双方向検査、paragraph-1/basic profileの凍結regressionを追加する。
  6. PDF validator/raster differentialでcell content、unexpected decoration不在、header repetition、page countを検査する。
  7. actual table profile IDとcombined fixtureを`samples/machine-package/matrices/m3-table.json`へ登録する。
  8. 全gate成功後だけtable profileをcapabilities/docsへ公開する。
- Acceptance criteria:
  - multi-page tableがtrace、manifest、PDFで同じrow/cell/header selected stateへbindされる。
  - descriptorの全table policyがpositiveまたはexplicit rejection fixtureを持つ。
  - older profileがtableを受理しない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list table --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf table --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_table --locked`
  - `python3 tools/verify_machine_profile.py --repository . --matrix samples/machine-package/matrices/m3-table.json --runs 2 --require-external-tools`
- Implementation notes (2026-08-27, Linux):
  - immutable public `typaxis.machine-pdf/table-1` descriptorをcurrent `typaxis.contract/1.2`へ追加し、default `paragraph-1`と既存2 profileの受理集合を変更しなかった。table profileはraw 1.0/1.1を`P1103`、older profileのtableをpre-resourceで拒否し、tableなしのcomplete M2 packageもsupersetとして受理する。
  - selected grid/row/cell/header receiptからpage、table、header-before-body、row fragment、cell origin順のexact rectangleとglyph commandを生成した。Display closureとfrozen PDF graph/serialized-byte receiptをactual page command indexまで再照合し、missing/extra/wrong-cell/wrong-page/wrong-repetition、重複・unclaimed intersecting command、path decorationをpublication前の`I9190`へ閉じた。
  - trace/manifestのconditional `table_layouts`は同じcanonical selected factsを共有し、resolved fixed/fraction columns、pre-residual share、signed residual/recipient、cell FlowId/span/cursor/vertical offset、rowspan continuation、row/header block offsetとtarget pageを保持する。table profileのbuilt artifactだけmemberを必須とし、旧profile artifact bytesへ空memberを追加しない。
  - `profiles/table-1/only`とcomplete M2併用`combined`、older-profile/old-contract/decoration/inapplicable-style negative、bidirectional capability coverageを`m3-table.json`へ登録した。combinedはfixed/fraction、colspan、rowspan、multi-page header repeat、PNG/font/link/list/page breakを同時に通し、Poppler text、全page MuPDF raster、page count、zero path decorationを独立検査する。
  - milestone指定3 Cargo test、locked workspace all-target check/test、workspace clippy `-D warnings`、`python3 schemas/validate.py`、PDF differential unit tests、format/diff check、指定の2-run external/reproducibility gateはlocal exit 0だった。current alias/versioned 1.2 DocumentPackage SchemaはともにSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままである。
- Non-goals:
  - footnote、advanced frames

### MI3-05 Footnote profile ADRとbounded reflow policyを採択する

- Status: Completed
- Depends on: MI2-08
- Design inputs: docs/25 §8 M3、§13.1、§13.3 footnote
- Primary files:
  - `adr/`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - immutable footnote profile ID、marker/discovery/reservation/split/carry/convergence policy。
- Tasks:
  1. reference marker生成、definition ownership、duplicate reference、unreferenced definitionの受理規則を固定する。
  2. first-reference順、page-local discovery、reserved height、body reflow、convergence fingerprintを固定する。
  3. footnote split/continuation、separator、empty/oversize definition、bodyとのkeep policyを固定する。
  4. definition/reference countを`max_ast_nodes`、page-local assignment/fragment countを`max_fragments`、reflow evaluationを既存`max_footnote_reflows_per_page`へmapし、consume owner、inclusive境界、error codeを固定する。
  5. body selected state、FootnoteFlowId、continuation、paint、trace/manifestを結ぶreceipt chainを定義する。
- Acceptance criteria:
  - reflow convergenceとmax+1停止の判定が実装者依存でない。
  - repeated reference、definition split、carry、unreferenced definitionの結果が一意である。
  - unsupported policyをpreflightで識別できる。
- Verification:
  - `rg -n "first-reference|reservation|converged|max_ast_nodes|max_fragments|max_footnote_reflows_per_page|continuation|unreferenced" adr contracts/machine-pdf-capabilities.md`
- Implementation notes (2026-08-27, Linux):
  - repositoryで次に空いていた`ADR-0030`をAcceptedとし、immutable target `typaxis.machine-pdf/footnote-1`をunchanged `typaxis.contract/1.2`上へ固定した。complete M2 content/style/resource/PDF domainにbody/list-item/captionのreferenceと、paragraph/headingだけのDocument-owned definitionを加える。tableとのcomposition、nested footnote、authored note policyは拒否し、MI3-07まではpublic descriptor/help/capabilityへ登録しない。
  - markerはcanonical FootnoteId UTF-8 byte順definition catalogの1-based shortest ASCII decimal、page assignment/paintはselected first-reference順として分離した。duplicate referenceはmarkerだけを繰り返し、definitionはexactly once logical content、unreferenced/empty definitionは`L5100`とした。body block-endとinline boundsを共有するbounded master region、1 pt band内のfull-width black 0.5 pt separator、`allow` split、minimum-first capacity allocation、dedicated strictly advancing FootnoteFlowId carry、body/definition keepとterminal oversizeを固定した。
  - evaluation 0をuncharged initial body fragmentation、以後を既存`max_footnote_reflows_per_page`へ1 unitずつmapし、body fingerprint/ordered set/全continuation/exact reservationの連続完全一致だけをconvergedとした。max回目は実行可能、不安定またはoscillationはmax+1開始前の`G6002`とする。definition/referenceは既存`max_ast_nodes` (`P1120`)、marker bytesは既存text limits (`T2100`/`T2101`)、page assignment/carry・separator・definition fragmentは既存`max_fragments` (`L5110`)へmapし、profile/flow/discovery/reservation/evaluation/convergence/carry/selected/Display/PDF/trace/manifest receipt chainとMI3-07 gateを採択した。
  - milestone指定の`rg` gate、Markdown link/table check、`python3 schemas/validate.py`、locked workspace all-target tests、workspace all-target clippy `-D warnings`、cargo format、whitespace/diff checkはlocal exit 0だった。current aliasとversioned 1.2 DocumentPackage SchemaはいずれもSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままで、MI3-05はSchema bytesやpublic profile surfaceを変更していない。
- Non-goals:
  - sidenote/endnote
  - footnote内footnote

### MI3-06 Footnote discovery、reservation、bounded reflowを実装する

- Status: Completed
- Depends on: MI3-05
- Design inputs: docs/25 §13.3 footnote steps 1-5
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
- Deliverables:
  - FootnoteFlowId registry、page pass、reservation/convergence receipt。
- Tasks:
  1. definitionをbodyとは別FootnoteFlowIdへcanonical owner順で登録し、reference/definition closureをpreflightする。
  2. body candidateから初出FootnoteIdをlogical orderで収集し、first-reference順にdefinition subflowをmaterializeする。
  3. ordered footnote setからreserved heightをchecked算出し、body frameを再構築してreflowする。
  4. body fingerprint、ordered footnote set、各continuation、reservationのtupleが一致した場合だけconverged receiptを発行する。
  5. `max_footnote_reflows_per_page`のmax+1 evaluation前にfatal errorで停止する。
  6. oscillation、wrong order、wrong reservation、missing definition、receipt replayをnegative testにする。
  7. zero/one/multiple/reference-repeat、boundary exact/max+1のunit/property testsを追加する。
- Acceptance criteria:
  - worker/registry順に依存せずfirst-reference順が再現される。
  - convergenceしていないpageはselected stateを発行しない。
  - limit超過時にDisplay/PDFを開始しない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination footnote_reflow --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout footnote_registry --locked`
- Implementation notes (2026-08-27, Linux):
  - private `footnote-1` preflightでtyped body reference preorder、reference/definition/unreferenced closure、definitionのclosed paragraph/heading-inline subset、single bounded master geometryを再導出し、package/body registry/LayoutEpochへbindしたprofile receiptを発行する。canonical FootnoteId catalog順の独立dense `FootnoteFlowId`、definition owner/block owners、positive measured fragment extents、terminalを持つregistryをworker登録順と無関係に構築し、domain-separated JCS/SHA-256 receiptへ閉じた。
  - page evaluatorはincoming carry reservationをevaluation 0 seedにし、全reference occurrenceをdocument logical orderで保持しつつnew definitionだけをfirst-reference順へcandidate-local assignmentする。separatorと全active minimumを先取りして残余をordered greatest-prefixで配分し、trailing minimumが共存できない場合はそのreference直前のpersistent body-cut requestで再評価する。legal cutをbody evaluatorが守れないindivisible keepは`L5100`とし、carry cursorはbody continuationと別型でstrictに進める。
  - body candidate/continuation、全discovery、ordered assignment、各before/after cursor・selected fragment・carry、exact reservation、body cutを含むcomplete tupleの連続一致だけがconvergence receiptを発行する。evaluation 0はuncharged、1..=Mだけを実行し、oscillationまたはunstable Mはmax+1 callback前にfatal `G6002`となる。candidate fragment chargeはconvergence後だけstateへatomic commitし、wrong order/reservation/cut、stale/replayed receipt、cross-page occurrence replayはselected stateを変更せず拒否する。
  - focused registry/reflow tests、locked workspace all-target tests、workspace clippy `-D warnings`、`python3 schemas/validate.py`、cargo format、whitespace/diff checkをlocalで成功させた。zero/one/multiple/same-page・later-page repeat、catalog-vs-first-reference order、split/carry、movable/unsplittable boundary、oscillation、exact/max+1 reflow/fragment limits、missing/empty/unreferenced definition、master geometry、receipt tamper/replayを含む。public profile/Display/PDF surfaceとcurrent/versioned 1.2 Schema bytesは変更せず、両SchemaのSHA-256は`de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままである。
- Non-goals:
  - footnote continuation paint

### MI3-07 Footnote continuation、paint closure、profileを公開する

- Status: Completed
- Depends on: MI3-06
- Design inputs: docs/25 §8 M3、§13.3 footnote
- Primary files:
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `docs/26-machine-input-cli.md`
  - `schemas/`
  - `tools/verify_machine_profile.py`
- Deliverables:
  - dedicated footnote carry receipt、marker/definition/continuation paint closure。
  - immutable footnote profile descriptorとcombined fixture。
- Tasks:
  1. continuationをbody cursorへ混ぜず、FootnoteFlowId/source page/next cursorを持つ専用carry receiptで次pageへ渡す。
  2. reference marker、separator、definition fragmentsをcanonical paint orderへ変換する。
  3. duplicate definition paint、unreferenced definition paint、referenced definition missing、wrong page/carryをselected-state closureで拒否する。
  4. definition split、multiple page carry、repeated reference、bodyとM2 featureの併用fixtureを追加する。
  5. descriptor/fixture coverageとolder profile freezeを検査する。
  6. trace/manifest/PDF validatorでbody fingerprint、ordered footnote set、continuation、reservationの一致を検査する。
  7. actual footnote profile IDとcombined fixtureを`samples/machine-package/matrices/m3-footnote.json`へ登録する。
  8. 全gate成功後だけfootnote profileをcapabilities/docsへ公開する。
- Acceptance criteria:
  - referenceを持つ全definitionがexactly-once logical contentとしてpaintされる。
  - continuationがpage間で欠落・重複せず、body cursorと独立する。
  - footnote packageがtrace、manifest、PDFで同じselected stateへbindされる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination footnote_carry --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf footnote --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_footnote --locked`
  - `python3 tools/verify_machine_profile.py --repository . --matrix samples/machine-package/matrices/m3-footnote.json --runs 2 --require-external-tools`
- Implementation notes (2026-08-28, Linux):
  - immutable public `typaxis.machine-pdf/footnote-1` descriptorをcanonical 4-profile registry、CLI help/dispatch、capability Schema/snapshotへ追加した。raw 1.2だけを受理し、complete M2 domainへbody/list-item/caption reference、Document-owned paragraph/heading definition、bounded Footnote frameだけを加える。default `paragraph-1`とolder profileの受理集合は変更していない。
  - selected layout sealは専用`StagingFootnoteCarryReceipt`のFootnoteFlowId/source/target/next cursorをpageごとに再検証し、body terminalを保持したcarry-only pageを許可する一方、missing/duplicate/wrong-page definition paintと未解決carryを拒否する。Displayはbody reference marker、fixed separator、first-reference順definition fragmentをexact command indexへ閉じ、definition内anchor/page-reference/linkをnamed destination/annotationへ継承する。frozen PDF graphとserialized-byte receiptは同じbody/selected/paint hashとmarker/separator/definition command countを保持する。
  - conditional `machine-footnote-manifest.schema.json`をcurrent/versioned 1.2 registryへ追加し、built `footnote-1`だけに同一のtrace/manifest `footnote_layout`を必須化した。zero-reference fixtureとcomplete M2 combined fixtureを`m3-footnote.json`へ登録し、combinedはcatalog順`a,z`とfirst-reference順`z,a`、repeat、heading/paragraph、anchor/page-reference/soft・hard break/internal link、2 carry edge、3 page目のcarry-only paintを同時に検査する。older artifactはmember不在を維持する。
  - 指定3 focused Cargo test、Schema validator、2-run external PDF/raster/text/separator/reproducibility verifier、locked workspace all-target test/check、workspace clippy `-D warnings`、format/diff checkをlocalで成功させた。current alias/versioned 1.2 DocumentPackage SchemaはともにSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままである。
- Non-goals:
  - semantic tagging of notes

### MI3-08 Advanced pagination ADRとprofile分割を採択する

- Status: Completed
- Depends on: MI2-08
- Design inputs: docs/25 §7 page master/writing mode、§8 M3、§13.1 subflows
- Primary files:
  - `adr/`
  - `contracts/phase-ownership.md`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
- Deliverables:
  - header/footer、columns、floatの採用subsetとimmutable profile ID群。
  - advanced pagination wire extension用new contract/Schema IDとold/new profile migration table。
  - frame selection、carry、balance、oversize、progress policy。
- Tasks:
  1. page size/trim/margins/PDF page boxesとheader/footerのpage-master selection、first/left/right page、content ownership、body overlap、repeat policyを固定する。
  2. column count/gap、sequential fill、最終frame balance、span、column breakのうち実装する集合を固定する。
  3. float placement class、anchor、queue ordering、carry、clearance、oversize、max deferralのうち実装する集合を固定する。
  4. header/footer FlowId、column FlowId、float FlowIdとbody/table/footnoteのowner/parent関係を固定する。
  5. frame/queue/balance iterationのlimit名、inclusive境界、terminal progress、diagnostic codeを固定する。
  6. 各featureを一つのprofileにまとめるか独立profileにするかをfixture組合せ数と互換性根拠付きで決定する。
  7. header/footer content ownership、column/float factsのwire shapeを固定し、current contractを同じIDのまま拡張せずnew contract/Schema IDを採番する。
  8. serializer/decoder/`dump-ast`、default、旧profile contract受理集合、capability/manifestのatomic migration順を固定する。
- Acceptance criteria:
  - selected page master、frame、float queue、carryのcanonical orderが一意である。
  - old/new contract、Schema、profile、defaultの対応が一意である。
  - 採用しないwriting mode/column/float policyがclosed rejection listにある。
  - empty/oversize状態からのterminal遷移が定義される。
- Verification:
  - `rg -n "header|footer|column|balance|float|carry|oversize|progress|profile" adr contracts/machine-pdf-capabilities.md`
- Implementation notes (2026-08-28, Linux):
  - 次の空き番号`ADR-0031`をAccepted targetとして登録し、new `typaxis.contract/1.3` / `https://schemas.typaxis.invalid/1.3/document-package.schema.json`とimmutable `header-footer-1`、`columns-1`、`float-1`を固定した。MI3-08はcurrent 1.2 Schema/Rust/public CLI bytesを変更せず、MI3-09〜11をcrate-private staging、MI3-12だけをatomic publication gateとした。
  - unified optional profileのheader/footer × columns × floatで最低8 presence/absence fixtureが必要になる案を退け、3 profile/3 all-advertised combined fixtureへ分割した。float profileだけはcolumn-boundary closureのためunbalanced sequential columnsを含み、header/footer + column/float、balance + float、table/footnote + advancedのcompositionをclosed rejectionにした。
  - contract 1.3 wireへrequired horizontal-tb/LTR、trim、nullable master-owned header/footer content、nullable count/gap/sequential/balance column layout、required Figure block/float placementを採択した。marginはtrim/bodyからchecked導出し、MediaBox/CropBox/TrimBox、singleまたはcanonical first/left/right selection、region repetition、exact last-column residual、bounded final balance、FIFO here/top/bottom/next-page float carryを一意にした。
  - header/footer/column/floatのtyped Flow owner/parentとdense allocation、selected frame/queue/carry order、empty/oversize terminalを固定した。既存`max_style_rules`、`max_ast_nodes`/nesting、`max_pages`、`max_fragments`、`max_column_balance_candidates`、`max_float_queue`、`max_float_carry_pages`へworkをmapし、inclusive max、balance `G6003`、float `G6004`、receipt/progress `I9190`を採択した。
  - MI3-12 migration tableはdefault `paragraph-1`を維持し、paragraphは従来contract + neutral 1.3、basic/table/footnoteはraw 1.2 + frozen semanticsのexact neutral 1.3、新3 profileはraw 1.3-onlyとした。これによりold profileの意味を広げずcurrent `dump-ast -> build-package`を保つ。full registry検査後にcontract/Schema alias、serializer/decoder/`dump-ast`、config、diagnostics、capability、trace/manifest、dispatch/help、fixture、private-runner removalを同一change setで切り替える。
  - milestone指定`rg`、changed Markdownのlocal link/table/invariant-order check、`python3 schemas/validate.py`、locked workspace all-target check/test、clippy `-D warnings`、cargo format、whitespace/diff checkはlocal exit 0だった。MI3-08はSchema file/Rust/public surfaceを変更せず、current aliasとversioned 1.2 DocumentPackage SchemaはともにSHA-256 `de407de17438ca09b1a9d7af24dfc2ed46ef0ec36d4a748a6179fe8b996f288a`のままである。
- Non-goals:
  - vertical/RTL writing modeをADRが明示採択しない限り実装済みにすること

### MI3-09 Header/footer subflowとpage-master selectionを実装する

- Status: Completed
- Depends on: MI3-08
- Design inputs: docs/25 §7 page master、§13.1 future subflows
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `schemas/`
  - `README.md`
  - `docs/19-cli.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `contracts/machine-pdf-capabilities.md`
- Deliverables:
  - typed page-master selection、page boxesとheader/footer independent subflows。
- Tasks:
  1. MI3-08のheader/footer content/page-box wireをnew contract DTO、domain、syntax validation、staging Schemaへ追加し、public current Schema aliasを変えない。
  2. page ordinal/document factsからpage masterをcanonical選択し、caller-selected masterを受理しない。
  3. header/footerをowner masterにboundした独立FlowId/frameへ登録する。
  4. selected page size、trim、marginsからMediaBox/TrimBoxとheader/body/footer frameをchecked導出し、non-overlap/page boundsを検査する。
  5. repeated contentをsource subflow、selected page、repetition indexへbindする。
  6. page master receipt、selected header/footer repetition、page boxesをmanifestへbindしてPDF page boxesとclosureし、missing/extra/wrong-master/wrong-page/wrong-repetition/wrong-boxを拒否する。
  7. custom trim、first/left/right、multi-page、empty、oversize、paragraph/list/figure併用fixtureを追加する。
- Acceptance criteria:
  - master selectionとpage boxesがtrace、manifest、Display、PDFで一致する。
  - header/footer進捗がbody cursorへ混入しない。
  - unsupported master propertyはpreflightで拒否される。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination page_master --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_header_footer --locked`
- Implementation notes (2026-08-28, Linux):
  - current/public 1.2を変更せず、private `typaxis.contract/1.3` staging decoder、DTO/domain、global dense NodeIdとregion grammarのsyntax validation、three-file staging Schema registryを追加した。public strict decoderはraw 1.3を引き続き拒否し、current aliasとversioned 1.2 DocumentPackage Schemaは同一byteのままである。
  - private `header-footer-1` preflightをeffective limitsとopaque session identityへbindし、horizontal-tb/LTR、singleまたはcanonical first/left/right master集合、custom trim、checked Media/Crop/Trim boxes、margin/header/body/footer non-overlap、unsupported columns/footnote/name/rule/style domainをresource/layout前に閉じた。
  - body、master-owned header/footer、list/caption descendantをcanonical dense FlowId registryへ分離した。page ordinalからmasterを選択し、regionは各selected pageでsource startからterminalまで独立実行し、per-master/kind repetition index、body cursor、frame/fragment workをchecked limitの範囲でreceiptへbindした。
  - selected layout、page-region block boundsのprivate structural Display paint、classic PDF page dictionary、trace/build共通advanced projectionをprofile/flow/selected/paint hashで相互検証した。PDFはexact MediaBox=CropBox、custom TrimBox、no BleedBox/ArtBox/Rotate/UserUnitを発行し、manifest projectorはpage/master/box/repetition/paintのmissing/extra/mismatchをtyped failureにする。public resource admission、text/image paint、通常artifact publicationとの統合はMI3-12のままである。
  - custom trim + first/left/right + three-page paragraph/list/block-Figure、empty region/body、region oversize、unsupported master、same-position progress、session/limits/raw-package replay、page/object/output exact/max+1をfixture/testへ追加した。combined runner outputはschema-valid canonical three-page goldenとexact byte比較する。
  - milestone指定test、private Schema validator、forbidden-dependency test、locked workspace all-target check/test、clippy `-D warnings`、cargo format、Markdown link/table、whitespace/diff checkをlocalで完了した。public CLI/profile/capability/current Schema aliasesは未変更で、columns/floatとfull 1.3 publication/release evidenceはMI3-10〜12に残る。
- Non-goals:
  - running element expression language

### MI3-10 Multi-column flowとbounded balanceを実装する

- Status: Completed
- Depends on: MI3-08
- Design inputs: docs/25 §7 writing/page requirements、§13.1 future subflows
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `schemas/`
- Deliverables:
  - column frame registry、sequential fill、bounded final balance receipt。
- Tasks:
  1. MI3-08のcolumn wireをnew contract DTO、domain、syntax validation、staging Schemaへ追加し、public current Schema aliasを変えない。
  2. page content frameをcolumn count/gapからfixed-point checked partitionし、residual policyを適用する。
  3. 各columnをparent flowにboundしたFlowId/frameとしてcanonical orderで登録する。
  4. sequential fillでcursorを単調に進め、column transitionをtraceへ記録する。
  5. ADRで採択した最終frame balanceをbounded candidate searchで実装し、selected target heightとinput fingerprintをreceiptへbindする。
  6. balance max+1前停止、oscillation、empty column、oversize、wrong target receiptをnegative testにする。
  7. selected column frame/FlowId/break/balance targetをmanifestへbindし、Display/PDFのpaintを同じpage/frame receiptへclosureしてmissing、extra、wrong-column、wrong-pageを拒否する。
  8. paragraph/list/figure跨ぎのsplit、closure tamper、same-toolchain reproducibility fixtureを追加する。
- Acceptance criteria:
  - column widthの和とgapがcontent frameへexactに一致する。
  - balance iterationは上限内でconvergeまたはtyped terminal errorになる。
  - worker順によりcolumn breakが変わらない。
  - selected column frameとbreakがtrace、Display、manifest、PDFで一致する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination columns --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list columns --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf columns --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_columns --locked`
- Implementation notes (2026-08-28, Linux):
  - public/current 1.2を変更せず、MI3-08で採択しMI3-09で着地したprivate `typaxis.contract/1.3` column DTO/domain/syntax/three-schema staging registryを`columns-1`専用runnerへ接続した。public strict decoderはraw 1.3を引き続き拒否し、public profile dispatch/capability/current Schema aliasは未変更である。
  - `columns-1` preflightをopaque session、raw/canonical package fingerprint、style/document epoch、全effective limitsへbindした。horizontal-tb/LTR、exact one master、full-media trim、null auxiliary regions/content、sequential + last-page column form、checked count/gap geometry、block Figure、closed basic-document structural subsetだけをlayout前に受理する。
  - body/list-item/figure-captionをpreorderで割り当てた後、physical columnをascending indexでparent body/source bodyへbindするcanonical dense FlowId registryを実装した。`(count-1)*gap`、available、floor width、last-physical-column residualをchecked導出し、receipt verificationはpackageから全flow/column/block registryを独立再導出する。
  - nonfinal pageをfull-height ascending columnへsequential fillし、body cursorのstrict monotonic progressと各frame before/after positionをselected receiptへbindした。nonempty terminal pageだけをfull-height evaluationから`ceil(selected_extent/count)`で開始し、typed rejection deficitによるstrictly increasing target、candidate/input/rejection fingerprint、inclusive max、max+1前`G6003`、oscillation、empty trailing frame、indivisible oversizeを閉じた。
  - selected column frame/FlowId/cursor/balance receiptからstructural Display commandsとclassic PDF page dictionaries/contentを構築し、manifest projectorがpage/master/box/column/frame/source/node/bounds/paint hashを相互検証する。paragraph/list/block-Figure combined、empty、neutral null-layout、oversize、wrong target/column/page/object、candidate/page/fragment/output exact/max+1、session/limits/package replay、same-toolchain reproducibilityとcanonical Schema goldenを追加した。
  - milestone指定の4 test、layout/manifest focused test、private Schema validator、forbidden-dependency test、locked workspace all-target check/test、clippy `-D warnings`、cargo format、Markdown link/table、whitespace/diff checkをlocalで完了した。current aliasとversioned 1.2 DocumentPackage Schemaは同一byteのままで、floatとfull 1.3 publication/release evidenceはMI3-11/MI3-12に残る。
- Non-goals:
  - unbounded optimal balancing
  - vertical writing mode

### MI3-11 Float queue、placement、carryを実装する

- Status: Completed
- Depends on: MI3-08, MI3-10
- Design inputs: docs/25 §8 M3、§13.1 future subflows
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `schemas/`
- Deliverables:
  - anchor-bound float queue、placement receipt、dedicated carry state。
- Tasks:
  1. MI3-08のfloat wireをnew contract DTO、domain、syntax validation、staging Schemaへ追加し、public current Schema aliasを変えない。
  2. anchor logical orderとNodeIdからcanonical queue keyを生成し、caller queue順を受理しない。
  3. ADRで採択したplacement class/clearanceだけをtyped enumとして実装する。
  4. available frameに対するplacementをchecked決定し、float rect、anchor、source FlowId、page/frameをreceiptへbindする。
  5. deferred floatをbody cursorと分離したcarryへ移し、max deferral到達時にoversize/terminal policyへ一度だけ遷移する。
  6. text wrapを採択した場合はexclusion geometryをline layout input fingerprintへbindし、採択しない場合はnon-wrapping placementだけを許す。
  7. selected float placement/carryをmanifestとPDF object usageへbindし、duplicate/missing/wrong anchor/wrong page/queue reorder/carry replayをDisplay/PDF closure testにする。
  8. single/multiple/deferred/oversize/column/page-boundary、exact/max+1 fixtureを追加する。
- Acceptance criteria:
  - float queueとcarryが各pagination stepでstrictly advanceまたはterminalになる。
  - selected float placementがtrace、Display、manifest、PDFで一致する。
  - M2 non-floating figureの挙動が変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pagination floats --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_float --locked`
- Implementation notes (2026-08-28, Linux):
  - public/current 1.2を変更せず、MI3-08で予約したprivate `typaxis.contract/1.3` DTO/domain/syntax/three-schema registryを`float-1`専用runnerへ接続した。profile receiptはraw/canonical package、document/style、全effective limits、opaque sessionへbindし、direct-body float、explicit positive width、unsplittable caption、zero spacing/indent/clearance、single full-media master、sequential unbalanced columnsだけをlayout前に受理する。public decoder/profile/help/capability/current Schema aliasは未変更である。
  - document preorderからFloat/FloatCaptionをbody/list/block-caption flowよりtyped ownershipで分離し、non-null column templateを最後にascending allocationするdense FlowId registryを実装した。style declaration既存charge、semantic node、column templateを同じ`max_ast_nodes`へchecked合算し、caption/list/nested float、wrong owner/depth/package/profile/limits receiptをselected state前に拒否する。
  - body anchor到達とFIFO enqueueをatomicにし、headだけを`here`、`top`、`bottom`、`next_page`順で評価するchecked schedulerを実装した。placementはfull-column-width exclusion band、zero clearance、source/body/frame/anchor/page/columnへbindし、column境界ではcarryを増やさず、page境界だけでdedicated carryをincrementする。queue/carry exact maxを受理してmax+1前に`G6004`、page/frame/body/placement/carry record exact maxを受理してallocation前に`L5110`とし、forced page break後のlater column placement、trailing-breakのpost-break blank page、duplicate candidate、same-state progressを閉じた。
  - selected placement/queue/carryからcanonical frame paint ordinalを発行し、Display command、placementごとのdedicated PDF Form XObject usage、actual page box/object observation、advanced trace/manifest projectionまでprofile/flow/selected/paint hashで閉じた。duplicate/missing/wrong page/class/anchor/order/carry/objectとreceipt replayを`I9190` negative testにした。
  - combined/empty/oversize fixtureはsingle/multiple、here/top deferral、column/page carry、non-floating neutral、exact/max+1、same-toolchain reproducibilityを含む。milestone指定test、affected layout/profile/Display/PDF/manifest tests、private Schema validator、locked workspace all-target check/test、strict clippy、format、Markdown、forbidden-dependency、whitespace/diff checkをlocalで完了した。current aliasとversioned 1.2 DocumentPackage Schemaは同一byteのままで、public 1.3 descriptor/dispatch、G6003/G6004 registry、external PDF/raster、release evidenceはMI3-12に残る。
- Non-goals:
  - ADRで採択していないCSS相当float behavior

### MI3-12 M3 selected-state closureと公開profileを統合する

- Status: Completed
- Depends on: MI3-04, MI3-07, MI3-09, MI3-10, MI3-11
- Design inputs: docs/25 §8 M3、§13.1、§13.5
- Primary files:
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-pagination/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `samples/machine-package/`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `docs/26-machine-input-cli.md`
  - `schemas/`
  - `schemas/README.md`
  - `tools/verify_machine_profile.py`
- Deliverables:
  - registry全subflowをbindするselected-state/trace/manifest closure。
  - 採択したM3 profileごとのall-advertised combined fixture。
- Tasks:
  1. body、list item、caption、table cell、footnote、header/footer、column、floatの全FlowId/terminalをselected-state fingerprintへ含める。
  2. Display/PDF usageをselected fragments、repetitions、carry、resource ledgerと双方向照合する。
  3. table/footnote/advanced paginationのexact-limitとprogress matrixをpublic CLI E2Eで実行する。
  4. 各profile descriptorとfixture coverageを双方向照合し、feature組合せが許可されないprofileではpreflight rejectする。
  5. previous current Schemaをversion directoryへfreezeし、MI3-08のnew contract constant/Schema registry/Wire serializer/decoder/`dump-ast`/capability/manifestをatomic change setで追加する。同じcommitでcrate-private staging runnerの専用入口を外し、通常pipelineがnew current contract/profileを選択できるようにする。hidden selectorは残さない。
  6. M1/M2と先行table/footnote profileのdescriptor bytes、default、contract受理/拒否集合をmigration tableどおり凍結fixtureで検査する。
  7. combined fixtureを二重build、異名checkout、documented hosts、PDF differentialへ通す。
  8. M3の全公開profile/combined fixtureを`samples/machine-package/matrices/m3-all.json`へ登録する。
  9. 全gate成功後にnew contract、support matrix、producer guide、capabilitiesへ各profileを公開する。
- Acceptance criteria:
  - table/footnoteを含むpackageがtrace、manifest、Display、PDFで同じselected stateへbindされる。
  - exact-limitとzero-progress規則がpublic CLI E2Eで検証される。
  - 各profileはadvertised featureのcombined fixtureを持ち、旧profileの意味を変えない。
  - new contractのdefault/compatibilityがMI3-08 migration tableと一致する。
- Verification:
  - `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 tools/verify_machine_profile.py --repository . --matrix samples/machine-package/matrices/m3-all.json --runs 2 --require-external-tools`
- Implementation notes (2026-08-28, Linux):
  - previous current 1.2 Schemaをversion directoryへbyte-for-byte freezeし、`typaxis.contract/1.3`、current Schema alias、closed contract/profile registry、strict decoder、canonical Wire encoder、`dump-ast`、config、diagnostics、capability、trace/build manifestを一つのchange setで切り替えた。defaultは`paragraph-1`のまま、1.0〜1.2と既存4 profileの受理/拒否集合をfrozen fixtureで維持し、新しい`header-footer-1`、`columns-1`、`float-1`を通常`build-package`/`check-package` pipelineへ登録した。advanced pagination専用runner入口またはhidden selectorは残していない。
  - body、list item、caption、table cell、footnote、header/footer、column、floatを含むcanonical全Flow registryとterminalをselected-stateへbindし、profile/package/session/flow、selected fragment/repetition/carry、Display paint、trace/manifestの双方向closureを`I9190` tamper testsで閉じた。advanced profileではselected node/text/image/anchor/linkとadmitted resource ledgerを追加bindingとし、PDF bytesへActualText、実PNG image XObject/soft mask、internal/external Link annotation、named destinationを発行してserialized object countと再照合する。
  - 7 public profileのdescriptor coverageとcombined fixture coverageをexact双方向照合し、table/footnote/advanced paginationのexact/max/max+1、zero progress、balance、float queue/carryをpublic CLI E2Eへ追加した。全M3 combined fixtureを`m3-all.json`へ登録し、禁止compositionはresource/layout開始前のprofile preflightで拒否する。release verifierもadvertised PNG XObject、Link annotation、named destinationの実serialized markerを要求する。
  - fixture generatorは二回目にcontent差分なし、Schema validatorは7 frozen 1.0、11 frozen 1.1、19 frozen 1.2、14 current 1.3 alias、20 versioned 1.3 Schema、54 machine expectationsを検証した。workspace全target test、strict clippy、format、whitespace/diff、二重build、異名checkout、Poppler text/page、MuPDF raster/PDF policy gateはすべてlocal exit 0で、host evidenceは`target/machine-e2e/host-evidence/x86_64-unknown-linux-gnu.json`へcanonical publishされた。
- Non-goals:
  - M4 model/publication feature advertising

## 8. M4: math/vector/book publication

M4のsemantic container、math、vector、tagged PDFは既存node、PNG、ActualTextへlossy loweringして追加しない。wire shapeまたは公開diagnostic/locationの意味が変わる場合は新contract IDを発行し、旧contract/profileを凍結したままatomic migrationする。MI4-02〜MI4-12の実装は採択済みnew contractを非公開stagingとして扱い、current contract/Schema/profileの切替はMI4-13だけが行う。各positive `machine_*`/staging exporter testは同じcrate-private runnerを直接使い、integration testsはpublic command grammar、help、current constants、Schema alias、capability bytesがstaging selectorを露出しないことを確認する。

### MI4-01 M4 contract versioningとsemantic container ADRを採択する

- Status: Completed
- Depends on: MI3-12
- Design inputs: docs/25 §7 semantic requirements、§13.4、§13.5
- Primary files:
  - `adr/`
  - `contracts/phase-ownership.md`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `schemas/README.md`
- Deliverables:
  - M4 wire changeに対する新contract ID/versioning判断。
  - generic semantic containerのkind、ownership、source mapping、fallback policy。
  - M4 resource declarationで必須にするclosed image/font media discriminatorのfield ownerとversioning boundary。
- Tasks:
  1. result/proof/exercise等を表すcontainer kindをclosed enum、extension mechanism、または両者の組合せとして固定する。
  2. containerのchild ownership、block/inline nesting、NodeId/source span、style scope、outline/tag mappingを固定する。
  3. container childが独立FlowIdを持つ条件と、typed grouping boundaryだけを持つ条件をpage split/selected-state規則とともに固定する。
  4. unknown kind、empty container、invalid nesting、unsupported renderingをrejectするphase/error codeを固定する。
  5. semantic container、math/vector binding、metadata/language/outline、tagged structureに加え、M4 contractで必須にする`resources.images[*].media_type`と`resources.font_faces[*].media_type`を含むplanned DocumentPackage wire shapeのcompatibilityを判定する。current IDを再利用せず次の未使用contract IDとSchema IDを採番する。untrusted `ImageMediaType`/`FontMediaType`はdocument-package/domain、declared mediaの許可判定とpolicy receiptはmachine-profile、decoder attestationと宣言とのexact照合はresource-admissionのownerとする。MI4-02以降が有効なstaging fixtureを作れるよう、既存PNG/TrueType sfnt/TTCのcanonical enum値とsource-mode/`dump-ast` population ruleはこのADRで固定し、MI4-03へvector値、MI4-10へJPEG/OTF-CFF値の追加だけを割り当てる。
  6. domain compatibilityは`ImageMediaDeclaration::{LegacyUnspecified, Declared(ImageMediaType)}`と`FontMediaDeclaration::{LegacyUnspecified, Declared(FontMediaType)}`のclosed enumで表し、nullable/raw stringにしない。syntax loweringだけがfrozen old contractとprovenanceへboundした`LegacyUnspecified`を発行でき、new M4 contractのmissing fieldはdecode error、M4 profileでのlegacy variantはpre-resource rejectionにする。old profileは旧contractと従来media subsetの組だけを受理し、legacy variantからnew declared mediaを合成しない。
  7. old/new contractごとのprofile、default、serializer/decoder、manifest identity、diagnostic schema対応をmigration tableにする。new M4 manifest Schemaのresource recordだけが`media_declaration`を持ち、`kind = legacy_unspecified`なら`media_type` memberを禁止し、`kind = declared`ならtyped `media_type` memberを必須にするtagged unionとする。decoder-issued値は別の`attested_media_kind` fieldとし、そのnullabilityはresource admission前のfailed progressだけに許可する。旧profileのsuccess/failure manifestは従来Schema/bytesを維持して新fieldを得ない。`legacy_unspecified`をnew manifestへ出せるのは、old contractをM4 profileへ渡してpre-resource拒否したfailed progressのようにnew Schemaで互換性failureを記録する場合だけとする。
  8. old IDへnew node setを追加しないことと、atomic publication順をADR acceptanceへ含める。
- Acceptance criteria:
  - semantic containerを文字列付きparagraphへ平文化する経路がない。
  - contract/profile/Schema IDの対応と旧版互換性が一意である。
  - migrationがdecoderだけ先行公開されない順序を持つ。
  - M4 image/font declarationはmedia discriminatorなしでは表現できず、旧contractのdeclaration shapeは凍結される。
  - old-contract legacy declaration、new-contract declared media、missing/unknown fieldがdomain/preflight上で混同されない。
  - old profileのmanifest bytesを変えず、new M4 manifestのsuccessではdeclared/attested mediaが必ず非nullで一致する。
- Verification:
  - `rg -n "semantic|ownership|source span|contract|schema|profile|migration|media_type|PNG|TrueType|TTC" adr docs/22-contract-matrix.md schemas/README.md`
- Implementation notes (2026-08-28, Linux):
  - 次の空き番号`ADR-0032`をAccepted targetとして登録し、non-current `typaxis.contract/1.4`、`https://schemas.typaxis.invalid/1.4/document-package.schema.json`、`typaxis.machine-pdf/production-book-1`を予約した。MI4-01はSchema/Rust/public CLI bytesを変更せず、MI4-02〜12をcrate-private staging、MI4-13だけをatomic publication gateとした。
  - contract 1.4のgeneric `semantic_container`をrequired `kind`/NodeId/SourceSpan/classes/`semantic_kind`/nonempty blocksを持つblock-only recordとし、kindは`result`、`proof`、`exercise`のclosed enumに固定した。open extension、inline/page-region配置、empty/recursively-empty、paragraph/class/text/raster fallbackを採用しない。
  - containerごとにparent/position/package/style/profile/LayoutEpochへbindした独立FlowIdを発行し、通常child itemは同flow、既存subflow ownerとnested containerだけは独立child flowとする。page splitは一つのtyped boundary/structure ownerをfirst/middle/last fragmentで維持し、outlineはcontainer entryなし、tag roleは`/Result`、`/Proof`、`/Exercise`から`/Div`へのmappingとした。
  - 1.4 image/font declarationへrequired `media_type`を追加する方針を採択し、base値を`png`、`sfnt-truetype-glyf`、`ttc-truetype-glyf`へ固定した。domainはnullable/raw stringでなくprovenance-bound `LegacyUnspecified` / `Declared(typed media)`、policyはmachine-profile、stable bytesからのattestation/exact matchはresource-admission、source-mode `dump-ast` populationは同じattestationのconsumerだけとした。
  - migration tableはold raw-contract/profile artifactをfrozen 1.3 encoder/Schemaでbyte維持し、raw 1.4またはM4 profile requestだけを1.4 artifact registryへrouteする。old profileのaccepted-contract集合は凍結し、raw 1.4は明示的な`production-book-1`指定だけで受理する。new production manifestのresource branchだけがtagged `media_declaration`とM4 font attestationを追加し、既存imageのrequired PNG `attested_media_kind`はそのまま維持する。built successはdeclared/non-null/exact match、pre-resource old-contract M4 failureだけがlegacy/nullを許可し、defaultは`paragraph-1`のままである。
  - required vocabulary `rg`、changed Markdownのlocal link/table/JSON fenceとI-001〜I-078連番検査、`python3 schemas/validate.py`、locked workspace全target test、strict clippy、format、diff/whitespaceをlocal exit 0で確認した。Rust/Schema definition/public CLIは変更せず、currentとversioned 1.3 DocumentPackage Schemaは同じSHA-256 `cd6dc1d69e407317687d1b192e6f2f4da086fa4f920a05cc4b1b0c611e7d3796`を維持した。
- Non-goals:
  - planned field owner/versioning判断を超えるmath node、vector payload、tagged PDFの具体wire

### MI4-02 M4 contract scaffoldとSemantic containerをPDF observationまで実装する

- Status: Completed
- Depends on: MI4-01
- Design inputs: docs/25 §7、§13.4
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - MI4-01で固定したnew contract staging scaffoldとPNG/TrueType base declared-media/attestation mapping。
  - semantic container Wire DTO/domain/validator/flow/layout receipt。
  - kind/source/styleを保持するselected-state/Display/PDF observationとnon-lossy fixture。
- Tasks:
  1. MI4-01のnew contract staging scaffoldを作り、required `ImageMediaType`/`FontMediaType` field、base PNG/TrueType sfnt/TTC enum値、version-bound `ImageMediaDeclaration`/`FontMediaDeclaration` compatibility enum、container typeをWire DTO、versioned Schema、domainへexhaustiveに追加してcanonical JCS serializerを更新する。public current Schema alias/contract constantは変えない。
  2. child ownership、nesting、NodeId/source span、style scopeをsyntax validatorで検査する。
  3. M2の`AdmittedImageMediaKind::Png`と新しい`AdmittedFontMediaKind`のTrueType sfnt/TTC variantをdecoder-issued attestationとしてbase declarationへexact照合し、URI suffix、caller value、wrong container/outlineをresource decode/admissionで拒否する。MI4-13で`dump-ast`へ接続するshared staging exporterはMI4-01のpopulation ruleどおり同じstable resource admission/attestationからwire declarationを作り、resource未admit時にsuffix推測で出力を続けない。MI4-13前はinternal test entryだけから呼び、public current outputへ接続しない。
  4. non-public staging profile descriptor/preflightへ採択container kind/style/nestingとbase media mappingをclosed登録し、旧profile/public capabilitiesはcontainerとM4 declaration shapeを拒否し続ける。
  5. container child blocksをMI4-01が選択したflow表現でcanonical registryへ登録し、実装側で方式を再選択しない。
  6. kind-specific visual styleをgeneric container styleからtyped computed styleへ解決し、raw kind stringをlayoutで比較しない。
  7. selected container fragmentとkind-specific computed styleをDisplay ownerへ渡し、child paint、container fingerprint、manifest selected-state fact、PDF/raster observationを同じreceiptへbindする。PDF writerがraw container kindを再解釈しない形で将来のstructure tree inputも保持する。
  8. result/proof/exercise、nested、page split、empty/unknown/wrong owner、base media round-trip/mismatch、round-trip/tamper fixtureを追加する。
  9. 旧contract/profileがcontainerとM4 resource declarationを拒否する凍結testを追加する。
- Acceptance criteria:
  - kind、children、source span、style、selected fragmentsがreceipt chainで結ばれる。
  - machine Wire decode -> trusted domain -> Wire re-encodeのtyped canonical round-tripでcontainer factsが失われない。containerはM4ではmachine-onlyとし、reference TSFへ新syntaxを暗黙追加しない。
  - unknown kind/nestingをlayout開始前に拒否する。
  - container fixtureのselected fragments/manifest fingerprintとPDF raster observationが一致する。
  - base PNG/TrueType declaration、decoder attestation、profile policyが一致する。missing/unknown/disallowed declarationはresource open前、declared/actual mismatchはstable read後かつdecoded allocation・font outline evaluation・PDF開始前に失敗する。
  - MI4-13前のpublic `dump-ast`/contract/Schema/capabilities bytesは変わらない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission declared_media_base --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_semantic_container --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli dump_ast_m4_base_media --locked`
  - `python3 schemas/validate.py`
- Implementation notes (2026-08-28, Linux):
  - non-current `typaxis.contract/1.4`をprivate staging registryとして追加し、required media declaration、result/proof/exercise semantic container、canonical JCS decode/re-encodeをWire DTOからtrusted domainまでlosslessに接続した。decoderでbyte/depth/node/style/resource limitを固定し、encoderも同じreceipted limitを再適用する。public current contract、1.3 Schema alias、capability bytes、CLI help/command grammarは変更せず、旧contract/profileによる1.4 input拒否をintegration testで凍結した。
  - syntax/profileはNodeId、source span、class/style scope、child ownership、recursive non-empty、closed kind/nesting、package/limit/session authorizationをlayout前に検証する。container、nested container、list/table/footnote subflowをcanonical Flow registryへ登録し、parent/position/style/profile/LayoutEpochをbindしたselected fragmentとfirst/middle/last splitを作る。typed computed styleからDisplay child paint、structure role input、deterministic PDF/raster observationへ渡し、raw kindの再解釈を後段に残していない。
  - PNG、TrueType glyf sfnt、TrueType glyf TTCのrequired declarationをprofile policyへ封印し、opaque suffixのfixtureをstable readしてdecoder-issued attestationとexact照合する。CFF/CFF2、missing/unknown/disallowed declaration、cross-catalog rebinding、generic parse bypass、declared/actual mismatchをresource allocation・outline evaluation・PDF開始前に拒否し、同じattestationだけを使うshared exporterはtest-only staging入口に限定した。
  - selected layout、Display、PDF、declared media、manifestを相互receipt/fingerprintで閉じ、kind/source/style/nested/page-split/mediaを保持するnon-lossy fixtureとtamper/alternate-layout/exact-limit testsを追加した。指定9 gate、workspace全target/all-feature check/test、strict clippy、format、forbidden dependency audit、Schema validator、diff/whitespaceと最終source reviewをlocal exit 0で完了し、21 private 1.4 Schemaとpublic 1.3 byte isolationを確認した。
- Non-goals:
  - tagged PDF role assignmentの公開
  - semantic container用reference TSF grammar

### MI4-03 Math/vector/accessibility binding ADRを採択する

- Status: Completed
- Depends on: MI4-01
- Design inputs: docs/25 §7 math/vector/accessibility、§13.4
- Primary files:
  - `adr/`
  - `contracts/contract-version.md`
  - `contracts/invariants.txt`
  - `contracts/phase-ownership.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `schemas/README.md`
- Deliverables:
  - inline/display math source、speech/ActualText、vector paint、source spanのbinding contract。
  - safe vector IRまたはsafe SVG subsetの選択と禁止機能一覧。
- Tasks:
  1. math source language/version、inline/display distinction、normalization、parser/formatter identityを固定する。
  2. visual layout input、vector output、speech/text alternative、source spanを一つのvalidated math receiptへbindする。
  3. speech/ActualText生成をproducer supplied、engine generated、両者照合のどれにするかとfailure policyを固定する。
  4. safe vector IRまたはsafe SVG subsetを選び、MI4-01で予約した`resources.images[*].media_type`のcanonical `ImageMediaType` enum値と`AdmittedImageMediaKind::SafeVector` attestationを固定する。external reference、script、animation、foreign object、network fetch、unbounded recursion/filter等の禁止規則を列挙する。
  5. coordinate/unit/fixed-point rounding、view box、clip、stroke/fill、font/text primitiveの採用subsetを固定する。
  6. vector complexity/bytes/nodes/path segments/nesting/math layoutのlimitとerror codeを固定する。
  7. math/vector receiptからDisplay/PDF object、ActualText、tagged structureへ至るclosure条件を固定する。
  8. math parser/formatterとvector parserのcrate owner、dependency edge、exact-pinned dependencyまたはin-tree implementation、supply-chain audit、tool identityを固定する。
- Acceptance criteria:
  - mathをPNGへrasterizeすることを正規経路にしない。
  - vector decoderがfilesystem/network/font lookupを暗黙実行しない。
  - source、visual、alternative、spanの取り違えを検出できるreceipt keyが定義される。
- Verification:
  - `rg -n "inline|display|speech|ActualText|source span|vector|external|script|network|limit|receipt" adr contracts/machine-pdf-capabilities.md`
- Implementation notes (2026-08-28, Linux):
  - `ADR-0033`をAccepted targetとして追加し、contract 1.4のprivate targetへclosed `inline_math` / `display_math`、required `typaxis-math` version `1` source/TextSpan/SourceSpan、producer-authored `speech`を固定した。sourceはdelimiter inference、macro/environment/package/file/network/recoveryを持たない小さいTeX-shaped grammarとし、exact bytesを保持したままin-tree `typaxis.math-parser/1`と`typaxis.math-formatter/1`のtyped round tripへbindする。
  - speechはengine生成/照合を採用せずrequired producer alternativeだけとし、source kind/bytes/span、AST、admitted MATH-table font/hash、fixed-point dimensions/baseline/vector paint、LayoutEpoch/workをlayout owner発行の`MathReceiptKey`へ結合した。inlineはatomic item、displayは独立`MathFlowId`/terminalとし、selected page/frame/origin、Display paint、exact `/ActualText`、manifest、将来のsingle `/Formula` + same `/Alt`まで同じreceiptをextendする。plain text/PNG fallbackとsourceからのalternative生成はない。
  - image media valueを`svg-safe-1`、decoder attestationを`AdmittedImageMediaKind::SafeVector`に固定した。stable bytesだけをin-tree iterative parserで、closed SVG namespace/element/attribute/path/clip/solid-RGB subset、no entity/external reference/script/animation/foreign object/text/font/CSS/filter/filesystem/networkとして検査し、checked fixed-point/viewBox/transform、exact curve lowering、canonical `typaxis.safe-vector-ir/1` fingerprintから既存ImageResourceId、DrawVector、frozen Form plan、PDF Form XObject、manifestへ双方向closureする。
  - inclusive `max_vector_nodes` / `max_vector_path_segments` / `max_vector_nesting_depth` / `max_math_layout_units`とprivate codes `R7120` / `R7121` / `R7122` / `L5111`、既存byte/text/AST/fragment/PDF limitへのone-time chargeを採択した。math crateは`core + font`だけ、Safe-SVG parserはresource-admission owner、third-party math/XML/SVG/CSS/browser/speech/network dependencyなしとしてtestkit audit対象を固定した。指定vocabulary gate、local Markdown/link/table/JSON/invariant checks、Schema validator、workspace全target/all-feature check/test、strict clippy、format、diff/whitespaceをlocal exit 0で確認し、public current 1.3 Schema bytesと七profile/defaultは変更していない。
- Non-goals:
  - arbitrary browser SVG/CSS compatibility

### MI4-04 Safe vector/SVG resource admissionとPDF paintを実装する

- Status: Pending
- Depends on: MI4-02, MI4-03
- Design inputs: docs/25 §7 SVG assets、§13.4 safe vector
- Primary files:
  - `workspace/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-core/src/`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-resources/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-testkit/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - closed declared vector media type、bounded safe vector decoder/validator、canonical vector IR、Display/PDF paint closure。
- Tasks:
  1. MI4-03が固定したvector media enum値をMI4 new-contract stagingのimage declaration Wire DTO/versioned Schema/domain/syntax loweringへ追加する。public current Schema alias/contract constantは変えない。
  2. stable-read済みresource bytesをdeclared `ImageMediaType`、content hash、profileへbindしてbounded decoderへ渡し、decoderだけが`AdmittedImageMediaKind::SafeVector`を発行する。宣言値とbytesの不一致をIR allocation前に拒否し、MI4-02のshared staging exporterは同じattestationからだけvector declarationを生成する。
  3. recursive general-purpose DOMを避け、MI4-03のsubsetだけをiterative parser/typed IRへ変換する。採択dependencyをexact pinし、testkitのdependency/supply-chain auditへ登録する。
  4. namespace/name/attribute duplicate/unknown、external URI、script、unsupported featureをresource admissionで拒否する。
  5. node/path/segment/nesting/coordinate/decoded bytesのlimitをallocation前・evaluation前に適用する。
  6. staging profile preflightで採択media typeだけを許可し、vector intrinsic size/aspect ratioをMI2 figureのtyped placementへ渡してcanonical order/fixed-point exact placementのDisplay opsへ変換する。
  7. existing image catalogの`ImageResourceId`、declared/attested media type、admitted hash、IR fingerprint、usage ledger、manifest resource fact、PDF objectを双方向closureする。別のcaller-assigned `VectorResourceId`は追加しない。
  8. allowed primitives、forbidden feature、media mismatch、entity/reference bomb、deep nesting、huge coordinate、tamper、exact/max+1、old-profile rejection fixtureを追加する。
- Acceptance criteria:
  - vector resource admission中にpackage root外readまたはnetwork accessが発生しない。
  - forbidden/unknown SVG featureを無視せずtyped diagnosticで拒否する。
  - same bytes/profileから同じIR fingerprintとPDF bytesを得る。
  - missing/unknown/disallowed declared mediaとnon-advertised profileはresource open前、declared/actual mismatchはstable read後かつvector IR allocation前に拒否される。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-core m4_limits --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission vector --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package vector_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax vector_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile vector_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list vector --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf vector --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_vector --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
  - `python3 schemas/validate.py`
- Non-goals:
  - external resource参照
  - unrestricted SVG filter/text/CSS

### MI4-05 Inline/display mathをsource・vector・alternativeへbindする

- Status: Pending
- Depends on: MI4-02, MI4-04
- Design inputs: docs/25 §7 math/accessibility、§13.4
- Primary files:
  - `workspace/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-math/`
  - `workspace/crates/typaxis-core/src/`
  - `workspace/crates/typaxis-font/src/`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-style/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-linebreak/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-testkit/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - inline/display math domain、validated math receipt、line/block layout、vector paint、alternative mapping。
- Tasks:
  1. `typaxis-math` crateを追加し、MI4-03で採択したdependencyだけをexact pinしてtestkitのallowed/denied edge auditへ登録する。
  2. math Wire DTO/versioned Schema/domain/syntax loweringをMI4-01のnew contract stagingへ追加し、source language/version、source text/span、alternativeをcanonical JCSへ含める。public current Schema alias/contract constantは変えない。
  3. staging profile descriptor/preflightへinline/display、source version、alternative、required vector mediaのclosed受理集合を追加し、旧profileとpublic current capabilitiesは拒否状態を維持する。
  4. parser/formatter identityとlimits下でmath sourceをbounded validated layout inputへ変換し、`typaxis-font`でadmitted faceのMATH table/required glyph/metricを検証し、failureをstable code/locationへmapする。
  5. inline mathをcluster/itemizationへ、display mathを独立block/subflowへtyped登録し、採択済み`display_math` selector/property applicabilityとinline inherited text styleを`typaxis-style`のclosed registryへ追加する。
  6. computed dimensions/baselineとvector IR fingerprintをvalidated math receiptへbindする。
  7. source、speech/ActualText、source span、vector paint、selected page/fragment、manifest factをsame receipt keyでclosureする。
  8. missing/extra/wrong-source/wrong-alternative/wrong-vector/wrong-page tamperを拒否する。
  9. inline/display、wrap、page split/keep、limit、unsupported source version、round-trip、PDF extraction、old-profile rejection fixtureを追加する。
- Acceptance criteria:
  - visualだけ、alternativeだけ、source spanだけが入れ替わったtamperを個別に検出する。
  - math sourceをplain textやPNGへsilent loweringしない。
  - renderer/extractorでvisual contentとActualTextの双方が観測できる。
  - MI4-13より前のpublic contract/Schema/profile bytesは変わらず、旧profileはmathをpreflightで拒否する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-core m4_limits --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-font math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package math_wire --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-style math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-layout math --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf math_actual_text --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_math --locked`
  - `python3 schemas/validate.py`
- Non-goals:
  - MI4-03で採択していないmath dialect

### MI4-06 Document metadata、language、outline ADRを採択する

- Status: Pending
- Depends on: MI4-01
- Design inputs: docs/25 §7 book navigation、§13.4
- Primary files:
  - `adr/`
  - `contracts/machine-pdf-capabilities.md`
  - `schemas/`
- Deliverables:
  - metadata fields、BCP 47 language inheritance、outline hierarchy/destination policy。
- Tasks:
  1. title、author、subject、keywords、identifier、creation/modification factsの受理集合、normalization、determinism policyを固定する。
  2. document/node languageをwell-formed BCP 47 tagとしてvalidationし、inherit/overrideとPDF language mappingを固定する。
  3. outline label、level/parent hierarchy、source heading/container、named destination、duplicate/missing target policyを固定する。
  4. clock/host dependent metadataを禁止し、producer suppliedまたはsource-derived factだけをmanifest/PDFへ許可する。
  5. metadata/language/outline count/depth/string bytes limitsとdiagnostic locationを固定する。
  6. wire、preflight、selected state、PDF catalog/outline、validator observationのreceipt chainを定義する。
- Acceptance criteria:
  - PDF metadataへbuild時刻/host pathが暗黙挿入されない。
  - languageとoutline targetのinvalid stateに明確なpreflight errorがある。
  - source headingとoutline destinationをtamper不能に照合できる。
- Verification:
  - `rg -n "BCP 47|language|outline|destination|metadata|clock|host|limit" adr contracts/machine-pdf-capabilities.md schemas`
- Non-goals:
  - arbitrary XMP extension vocabulary

### MI4-07 Metadata、language、outlineをPDF validatorまで実装する

- Status: Pending
- Depends on: MI4-02, MI4-06
- Design inputs: docs/25 §7 book navigation、§13.4
- Primary files:
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `tools/verify_pdf_structure.py`
  - `tools/test_pdf_structure.py`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - validated metadata/language/outline registryとPDF catalog/outline emission。
- Tasks:
  1. MI4-01のnew contract staging Wire DTO/versioned Schema/domain/syntax loweringへ採択fieldsを追加し、canonical serialization/round-tripを更新する。public current Schema alias/contract constantは変えない。
  2. non-public staging profile descriptor/preflightへ採択metadata/language/outline fieldとpolicyをclosed登録し、旧profile/public capabilitiesはこれらを拒否し続ける。
  3. BCP 47 validation/inheritanceをtyped computed language receiptへ変換する。
  4. outline hierarchyをsource owner preorderでcanonicalizeし、selected named destinationへ解決する。
  5. metadata/language/outline fingerprintsをmanifestとselected-state closureへ含める。
  6. PDF catalog、Info/XMPの採択先、document/marked-content language、outline treeをdeterministic object orderで発行する。
  7. duplicate/missing/wrong target、bad language、host/clock leakage、limit/tamper、old-profile rejection fixtureを追加する。
  8. `verify_pdf_structure.py`とunit testを追加し、独立PDF validatorでmetadata、document language、outline hierarchy/target、link destinationを検査する。
- Acceptance criteria:
  - source facts、manifest、PDF validator observationが一致する。
  - 二重build/異名checkoutでmetadataを含むPDF bytesが一致する。
  - invalid hierarchy/languageはPDF開始前に拒否される。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf metadata_outline --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package book_navigation_wire --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax book_navigation --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile book_navigation --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_book_navigation --locked`
  - `python3 -m unittest tools/test_pdf_structure.py -v`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_book_navigation_external --locked -- --ignored`
  - `python3 schemas/validate.py`
- Non-goals:
  - tagged structure tree

### MI4-08 Tagged PDF/structure tree ADRとvalidation policyを採択する

- Status: Pending
- Depends on: MI4-02, MI4-03, MI4-06
- Design inputs: docs/25 §7 accessibility、§13.4
- Primary files:
  - `adr/`
  - `contracts/phase-ownership.md`
  - `contracts/machine-pdf-capabilities.md`
- Deliverables:
  - semantic nodeからPDF structure element/marked contentへのreceipt contract。
  - accessibility profile、artifact policy、independent validator gate。
- Tasks:
  1. supported semantic node/container/math/list/table/figure/link/footnoteからstructure roleへのexhaustive mappingを固定する。
  2. parent/child ownership、reading order、page跨ぎmarked-content sequence、MCID allocationをcanonicalizeする。
  3. alt/ActualText/language、decorative artifact、figure caption、table header association、footnote reference relationのpolicyを固定する。
  4. selected layout fragmentとstructure node/marked contentを結ぶpackage/epoch-bound receiptを定義する。
  5. missing/extra/wrong-parent/wrong-reading-order/wrong-MCID/wrong-alternativeをclosure errorとして割り当てる。
  6. target PDF conformance/accessibility validators、required versions、warning/failure policyをrelease contractへ固定する。
  7. structure nodes/depth/marked-content count/string bytesのlimitを固定する。
- Acceptance criteria:
  - tag treeをDisplay/PDF writerの推測で再構築しない。
  - visual selected stateとsemantic reading orderの対応がreceiptで検査可能である。
  - validator warningを無条件success扱いしない。
- Verification:
  - `rg -n "structure|marked content|MCID|reading order|ActualText|artifact|validator|limit" adr contracts/machine-pdf-capabilities.md`
- Non-goals:
  - 採択validatorで検証不能なconformance claim

### MI4-09 Tagged structure、marked content、accessibility closureを実装する

- Status: Pending
- Depends on: MI4-05, MI4-07, MI4-08
- Design inputs: docs/25 §7 accessibility、§13.4
- Primary files:
  - `workspace/crates/typaxis-layout-contract/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `tools/verify_pdf_structure.py`
  - `tools/test_pdf_structure.py`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - validated structure registry、marked-content receipts、tagged PDF emission/closure。
- Tasks:
  1. validated semantic registryをsource owner preorderで構築し、parent/child/role/language/alternativeをbindする。
  2. selected visual fragmentsからpage-local marked-content sequenceとdeterministic MCIDを割り当てる。
  3. split/repeated visual fragmentsを同一logical structure nodeへbindし、header cloneやdecorative paintをADR policyでartifact化する。
  4. structure tree、parent tree、marked content、annotation/link、outline/math alternativeをmanifestのversioned staging SchemaとPDF object closureへ含め、current Schema aliasは変えない。
  5. missing/extra/wrong owner/order/page/MCID/alternative/language tamperを個別に拒否する。
  6. staging profile descriptor/preflightへtagged structure、role、alternative、validator requirementのclosed受理集合を追加し、旧profile/public current capabilitiesはtagged PDFを拒否し続ける。
  7. paragraph/list/table/figure/math/link/footnote/containerを併用するaccessibility fixtureとold-profile rejection fixtureを追加する。
  8. independent validatorとtext/accessibility extractorでrole、reading order、language、alt/ActualTextを検査する。
- Acceptance criteria:
  - document language、outline、link、tagged structureが独立validatorで成功する。
  - visual contentとstructure nodeのmissing/extraが双方0件である。
  - same inputでMCID/object order/PDF bytesが決定的である。
  - MI4-13より前に旧profileまたはpublic capabilitiesがtagged PDFを受理・advertiseしない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-display-list tagged_structure --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf tagged_pdf --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile tagged_pdf --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_accessibility --locked`
  - `python3 -m unittest tools/test_pdf_structure.py -v`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_accessibility_external --locked -- --ignored`
  - `python3 schemas/validate.py`
- Non-goals:
  - validator未対応roleのproduction claim

### MI4-10 JPEG、OTF/CFF media/font profile ADRを採択する

- Status: Pending
- Depends on: MI4-01, MI4-03
- Design inputs: docs/25 §7 asset/font、§13.4
- Primary files:
  - `adr/`
  - `contracts/phase-ownership.md`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/22-contract-matrix.md`
  - `schemas/README.md`
- Deliverables:
  - MI4-01のPNG/TrueType base media mappingを維持し、JPEG/OTF-CFFを独立に追加・advertiseするclosed declared-media contractとresource profile/embedding plan。
- Tasks:
  1. MI4-01が固定したPNG/TrueType sfnt/TTCの`ImageMediaType`/`FontMediaType` mappingを変更せず、JPEG、OpenType/CFF、採択するbare CFF/containerのcanonical wire enum値と対応する`AdmittedImageMediaKind`/`AdmittedFontMediaKind`、magic/container照合、URI suffix非依存、mismatch diagnosticを追加する。reference sourceからnew contractをexportする`dump-ast`が新形式の宣言値を得るownerと、attestation不能時のfailureも固定する。
  2. JPEGの許可color space、bit depth、orientation/metadata、progressive、ICC、decode limit、PDF embedding/transcode policyを固定する。
  3. OTF/CFFのcontainer/table subset、glyph closure、variation/color font、hinting、subsetting、embedding permission policyを固定する。
  4. font parser/decoder identity、declared/attested media type、resource hash、face index、selected glyph set、subset fingerprint、PDF objectのbindingを定義する。
  5. malformed/unsupported/media-mismatch/licensing restricted resourceのdiagnostic、location、publication policyを固定する。
  6. MI4-03で固定済みのSafeVector media contractを入力に、PNG/SafeVector/既存font profileと分離したimmutable resource profile IDsと組合せprofileを決める。
  7. bytes/pixels/tables/glyphs/outlines/subset size limitsのinclusive境界を固定する。
  8. decoder/parserをin-tree実装にするかexternal crateにするかを固定し、externalの場合はexact version/features、Rust 1.75 compatibility、dependency edge、supply-chain auditを定義する。
- Acceptance criteria:
  - JPEG、OTF/CFFを一括の曖昧な「image/font対応」としてadvertiseしない。
  - transcode、metadata stripping、font subsettingのdeterminismが定義される。
  - embedding permission違反をPDF publication前に拒否する。
  - declared media type、decoder-attested format、PDF embedding planの取り違えを各resource ID単位で検出できる。
- Verification:
  - `rg -n "media_type|JPEG|color space|OTF|CFF|glyph|subset|embedding|license|profile|limit" adr contracts/phase-ownership.md contracts/machine-pdf-capabilities.md docs/22-contract-matrix.md schemas/README.md`
- Non-goals:
  - variation/color fontをADRが採択しない場合のsupport

### MI4-11 JPEG admission、figure layout、PDF embeddingを実装する

- Status: Pending
- Depends on: MI4-02, MI4-10
- Design inputs: docs/25 §7 JPEG、§13.4 media plan
- Primary files:
  - `workspace/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-resources/src/`
  - `workspace/crates/typaxis-layout/src/`
  - `workspace/crates/typaxis-display-list/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `workspace/crates/typaxis-testkit/src/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - bounded JPEG admission receiptとfigure/PDF vertical slice。
- Tasks:
  1. MI4-10のJPEG enum値をM4 new-contract stagingのimage declaration Wire DTO/versioned Schema/domain/syntax loweringへ追加し、public current Schema alias/contract constantは変えない。
  2. non-public staging profile descriptor/preflightへJPEG media/profile policyをclosed登録し、旧profile/public capabilitiesはJPEGをresource open前に拒否し続ける。
  3. MI4-10で採択したparser dependencyをexact pin/auditするかin-tree marker/segment parserをboundedに実装し、declared media typeをbytesからattestしたJPEG format、dimensions/color/decode facts、resource hash/profileへbindする。mismatchはdecode allocation前に拒否し、MI4-02のshared staging exporterは同じattestationからだけJPEG declarationを生成する。
  4. unsupported color/metadata/progressive/ICC状態をMI4-10 policyどおりrejectまたはcanonical transformする。
  5. PNGと共通のtyped image dimensions/placementと`DrawImage` usageへ変換し、media-specific factを消失させない。
  6. usage ledger、late finalizer、PDF image objectを`ImageResourceId`、declared/attested media type、resource/decoded fingerprintへclosureする。
  7. admitted/decoded media、dimensions、hash、transform/embedding planをmanifest resource factへbindし、PNG/JPEGを同じformatとして記録しない。
  8. valid variants、truncated/bomb/huge dimensions/bad metadata/media mismatch/hash/tamper、old-profile rejection、exact/max+1 fixtureを追加する。
  9. renderer differentialでplacement/color/page countを検査し、reproducibility testを追加する。
- Acceptance criteria:
  - decoder allocation前にdimension/bytes limitsを適用する。
  - admitted JPEGとPDF image objectのmissing/extra/wrong-IDが0件である。
  - non-advertised JPEG profileではpreflight rejectする。
  - declared JPEG、decoder attestation、manifest、PDF embedding planが同じ`ImageResourceId`へbindされる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission jpeg --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package jpeg_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax jpeg_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile jpeg_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf jpeg --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_jpeg --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
  - `python3 schemas/validate.py`
- Non-goals:
  - MI4-10で採択していないJPEG variant

### MI4-12 OTF/CFF admission、glyph closure、PDF subsetを実装する

- Status: Pending
- Depends on: MI4-02, MI4-10
- Design inputs: docs/25 §7 font、§13.4 font embedding plan
- Primary files:
  - `workspace/Cargo.toml`
  - `workspace/Cargo.lock`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-document/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-font/src/`
  - `workspace/crates/typaxis-shaping/src/`
  - `workspace/crates/typaxis-pdf/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `workspace/crates/typaxis-testkit/src/`
  - `schemas/`
  - `samples/machine-package/`
- Deliverables:
  - bounded OTF/CFF admission、selected glyph ledger、deterministic PDF subset/embed closure。
- Tasks:
  1. MI4-10のfont media enum値をM4 new-contract stagingのfont-face declaration Wire DTO/versioned Schema/domain/syntax loweringへ追加し、public current Schema alias/contract constantは変えない。
  2. non-public staging profile descriptor/preflightへ採択OTF/CFF media/font policyをclosed登録し、旧profile/public capabilitiesはOTF/CFFをresource open前に拒否し続ける。
  3. MI4-10で採択したparser dependencyをexact pin/auditするかin-tree font container/table directoryをchecked parseし、declared media typeをactual container/outline kindへattestしてoffset/length overlap、checksum、face index、required tablesを検査する。mismatchをoutline evaluation前に拒否し、MI4-02のshared staging exporterは同じattestationからだけOTF/CFF declarationを生成する。
  4. CFF charstrings/subroutines等をbounded iterative evaluationし、recursion/operation/outline limitsをmax+1前に適用する。
  5. embedding permissionとlicense factをresource receipt/manifestへbindし、restricted fontをpublication前に拒否する。
  6. shapingのselected glyph IDsをcanonical glyph closureへ集約し、resource/face/features/input text fingerprintへbindする。
  7. deterministic subset tag/object/table orderを生成し、selected glyph ledgerとPDF embedded fontを双方向closureする。
  8. declared/attested media、face、glyph closure、license、subset fingerprint、PDF embedding planをmanifestへ同じ`FontFaceId`で記録する。
  9. malformed/truncated/recursive/restricted/media mismatch/wrong face/glyph tamper/old-profile rejection/exact limitsとrepresentative positive font fixtureを追加する。
  10. raster/text extraction differential、two-build、異名checkoutでsubset/PDF bytesを検査する。
- Acceptance criteria:
  - unselected glyph、missing selected glyph、wrong font/faceのPDF embeddingを検出する。
  - parser/evaluatorがpanic、stack overflow、unbounded allocationを起こさない。
  - resource license/embedding factsがrelease evidenceに残る。
  - declared font media、decoder attestation、manifest、PDF FontFile subtypeが同じ`FontFaceId`へbindされる。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission otf_cff --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-document-package font_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-syntax font_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-machine-profile font_media --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-font cff --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-pdf font_subset --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_otf_cff --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit forbidden_dependency_edges --locked`
  - `python3 schemas/validate.py`
- Non-goals:
  - MI4-10で採択していないfont technology

### MI4-13 M4 contract migrationとproduction-book profileを原子的に公開する

- Status: Pending
- Depends on: MI4-02, MI4-05, MI4-09, MI4-11, MI4-12
- Design inputs: docs/25 §7、§8 M4、§13.4、§13.5
- Primary files:
  - `workspace/crates/typaxis-core/src/lib.rs`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-diagnostics/src/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-cli/src/main.rs`
  - `workspace/crates/typaxis-cli/src/pipeline.rs`
  - `workspace/crates/typaxis-cli/src/artifacts.rs`
  - `workspace/crates/typaxis-cli/tests/`
  - `schemas/`
  - `schemas/README.md`
  - `samples/machine-package/`
  - `contracts/contract-version.md`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `docs/26-machine-input-cli.md`
  - `tools/verify_machine_profile.py`
  - `tools/verify_pdf_differential.py`
  - `tools/verify_pdf_structure.py`
- Deliverables:
  - MI4 ADR群が採択した新contract/Schema/profileのatomic migration。
  - VMB相当production-book combined fixtureとindependent validation evidence。
- Tasks:
  1. previous current Schema一式をversion directoryへfreezeし、MI4 stagingのnew Schema registryが旧registryと混ざらないこと、およびnew resource declarationだけがrequired closed `resources.images[*].media_type`/`resources.font_faces[*].media_type`を持つことをvalidatorで確認する。
  2. new contract constant/current Schema alias、Wire serializer/decoder/`dump-ast`、diagnostics、capability、manifest、fixturesを一つのchange setでstagingから有効化し、partial shapeへnew IDを付けない。
  3. 旧contract/profileのWire・capability・manifest golden bytes、default profile、`LegacyUnspecified` lowering、受理/拒否集合を維持し、旧ID/旧manifest Schemaへdeclared media fieldやM4 featureを追加しない。new M4 profileがlegacy declarationをpre-resourceで拒否し、new failed-manifest Schemaだけがその`legacy_unspecified` stateを記録するfixtureも固定する。
  4. chapter/section heading、semantic containers、inline/display math、list、table、footnote、figure/caption、link、metadata/language、outline、tagged structureと、PNG/SafeVector/JPEG/TrueType/OTF-CFFのうちproduction profileが採択した全media/fontを一つのproduction fixtureで使う。
  5. source factsからwire、trusted package、declared/decoder-attested media、all-flow selected state、Display、PDF、manifestまで全receipt closureを検査する。
  6. lossy producer preprocessingを検出するため、node kind/count、math source/span、resource IDs、outline、reading orderのexpected ledgerをfixtureへ同梱する。
  7. independent renderer、text extractor、structure/accessibility validatorでpage/raster/text/language/outline/link/tags/alternativesを検査する。
  8. two-build、異名checkout、documented hostsで全artifactを比較し、tool identitiesをevidenceへ記録する。
  9. actual profile ID/fixtureを参照する`samples/machine-package/matrices/m4-production.json`を作る。
  10. descriptor/fixture coverage、unsupported feature preflight、M4 feature-local tamper matrixを通した後だけcapabilities/docsへproduction-book profileを公開する。
- Acceptance criteria:
  - production fixture全体をsilent deletion、flattening、rasterizationなしで生成できる。
  - math source、vector paint、alternative、source spanがtamper不能なreceiptで結ばれる。
  - language、outline、link、tagged structureが独立validatorで成功する。
  - 旧contract/profileのobservable behaviorが凍結fixtureと一致する。
  - 各resourceのdeclared media type、decoder attestation、manifest fact、PDF embedding planがlogical ID単位で一致する。
- Verification:
  - `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 tools/verify_machine_profile.py --repository . --matrix samples/machine-package/matrices/m4-production.json --runs 2 --require-external-tools`
- Non-goals:
  - M5 release/hardening gateの代替

## 9. M5: hardeningとrelease

M5は新しい表現機能を追加するphaseではない。M1〜M4でadvertiseした契約に対し、長時間fuzz、tamper、differential、platform/resource governanceをrelease blockerとして自動化し、release profileのclosed contractを証明する。

### MI5-01 Machine JSON/source/resourceの継続fuzz gateを構築する

- Status: Pending
- Depends on: MI4-13
- Design inputs: docs/25 §8 M5、§15.2
- Primary files:
  - `workspace/fuzz/`
  - `workspace/crates/typaxis-document-package/src/`
  - `workspace/crates/typaxis-host-admission/src/`
  - `workspace/crates/typaxis-machine-input/src/`
  - `workspace/crates/typaxis-syntax/src/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `workspace/crates/typaxis-testkit/src/`
  - `tools/verify_fuzz_targets.py`
  - `tools/run_fuzz_matrix.py`
  - `release/fuzz-evidence/`
- Deliverables:
  - deterministic seed corpus、bounded fuzz targets、crash/minimization/replay workflow。
- Tasks:
  1. strict JSON scanner、typed decoder、JCS encoder、source identity map、contained open metadata logic、PNG/JPEG/vector/font decoderを独立targetにする。
  2. arbitrary bytesだけでなくvalid typed seedからduplicate escaped key、depth、length/hash、URI、container/resource structureをmutateするtargetを追加する。
  3. panic、abort、stack overflow、OOM相当、hang、limit bypass、non-deterministic resultをfailure oracleにする。
  4. targetごとにbytes/depth/operation/time/memoryのharness上限を固定し、engine limit failureとharness timeoutを区別する。
  5. M1〜M4のregression fixtureと過去crashをversioned seed corpusへ登録する。
  6. local smoke budgetと明示的に管理するlong-run host budgetを分け、toolchain/engine/seed corpus identityをevidenceへ記録する。GitHub Actionsは使用しない。
  7. minimized reproducerを通常unit testへ昇格してからcorpusへ追加する運用を文書化する。
  8. `verify_fuzz_targets.py`でcapability/decoder inventory coverageを検査し、`run_fuzz_matrix.py`で同じtarget matrixをlocal smoke/managed-host long-runへ実行する。
- Acceptance criteria:
  - 全advertised decoder/admission boundaryに少なくとも一つのfuzz targetがある。
  - same seed/toolchainでclassificationとdiagnostic codeが再現される。
  - timeout/OOMをsuccessやunsupportedとして扱わない。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit fuzz_corpus --locked`
  - `python3 tools/verify_fuzz_targets.py --workspace workspace --capabilities samples/machine-package/capabilities.json`
  - `python3 tools/run_fuzz_matrix.py --matrix workspace/fuzz/targets.json --mode smoke`
- Non-goals:
  - fuzzでcontract correctnessの全証明を代替すること

### MI5-02 Capability/manifest/trace/diagnosticの全receipt tamper matrixを閉じる

- Status: Pending
- Depends on: MI4-13
- Design inputs: docs/25 §8 M5、§15.2
- Primary files:
  - `workspace/crates/typaxis-testkit/src/`
  - `workspace/crates/typaxis-cli/tests/`
  - `workspace/crates/typaxis-manifest/src/`
  - `workspace/crates/typaxis-diagnostics/src/`
  - `contracts/receipt-edges.json`
  - `schemas/receipt-edges.schema.json`
  - `tools/verify_receipt_coverage.py`
- Deliverables:
  - receipt edge inventoryとgenerated tamper matrix。
- Tasks:
  1. raw/decoded/source/profile/style/resource/layout/selected state/Display/PDF/output/diagnostic/manifestのreceipt nodeとedgeを`contracts/receipt-edges.json`へmachine-readable inventoryとして記録し、Schema validatorを追加する。
  2. 各edgeへwrong session、package、epoch、profile、NodeId/FlowId/FontFaceId/ImageResourceId、hash、ordinal、count、terminal、targetを一要素ずつ変えるmutantを生成する。
  3. missing、extra、duplicate、reorder、cross-run replay、same-bytes-different-bindingを各集合receiptへ適用する。
  4. mutantごとにexpected rejecting phase/code、side effects、visible artifactsを宣言し、最初のunexpected acceptを失敗させる。
  5. descriptor、manifest、trace、diagnostics Schema fixtureのfield coverageとtamper coverageを双方向照合する。
  6. diagnostic budget/eviction、省略note、primary order自体のtamperを含める。
  7. harnessからcanonical coverage reportを`target/receipt-coverage.json`へ出力し、release evidenceへ取り込み、未検査edgeをrelease blockerにする。
  8. `verify_receipt_coverage.py`でinventory、test report、Schema fieldsの双方向coverageを検査する。
- Acceptance criteria:
  - inventoryの全receipt edgeがpositive binding testと一件以上のnegative tamper testを持つ。
  - tamper failure後のartifact集合がtyped publication outcomeと一致する。
  - cross-session/cross-package replayがbytes同一でも失敗する。
- Verification:
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-testkit tamper_matrix --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-cli machine_tamper_matrix --locked`
  - `python3 tools/verify_receipt_coverage.py --inventory contracts/receipt-edges.json --test-report target/receipt-coverage.json`
- Non-goals:
  - cryptographic signature/remote attestation

### MI5-03 Renderer/extractor/accessibility differential gateをrelease blockerにする

- Status: Pending
- Depends on: MI4-13
- Design inputs: docs/25 §8 M5、§15.3
- Primary files:
  - `tools/verify_pdf_differential.py`
  - `tools/verify_pdf_structure.py`
  - `tools/pdf-differential-matrix.json`
  - `tools/pdf-structure-matrix.json`
  - `samples/machine-package/`
  - `release/pdf-differential-evidence/`
- Deliverables:
  - pinned independent renderer/extractor/structure validator matrixとexpected ledger。
- Tasks:
  1. 各toolの名称、version、配布hash、supported host、invocation、timeout、warning policyを二つのmatrixへ固定し、両verification toolへ`--matrix` modeを追加する。
  2. paragraph/basic/table/footnote/advanced/production fixtureごとにpage count、raster regions、text order、links、metadata、language、outline、tags、alternativesのexpected ledgerを作る。
  3. 少なくとも二系統のrendererまたはparserでPDF open/renderを検査し、single-tool blind spotを減らす。
  4. raster comparisonのsize/color normalizationとtoleranceを固定し、blank/欠落pageが誤って合格しないsentinelを入れる。
  5. extractor observationをlogical expected text/reading orderへ比較し、font subset差を無視してcontent欠落を検出する。
  6. validator warning、crash、timeout、missing binary、version mismatchを明示failure classにし、success skipを禁止する。
  7. failing output、tool stdout/stderr、identity、input PDF hashをmanaged-host evidence directoryへ保存する。
- Acceptance criteria:
  - 全公開profileに少なくとも一つのpositive differential fixtureがある。
  - production profileはvisual、text、navigation、accessibilityの全gateを通る。
  - required tool不在・version不一致がrelease jobを失敗させる。
- Verification:
  - `python3 tools/verify_pdf_differential.py --matrix tools/pdf-differential-matrix.json`
  - `python3 tools/verify_pdf_structure.py --matrix tools/pdf-structure-matrix.json`
- Non-goals:
  - pixel差だけでsemantic correctnessを判定すること

### MI5-04 Supported platform reproducibilityとtool identityを固定する

- Status: Pending
- Depends on: MI4-13
- Design inputs: docs/25 §8 M5、§15.2、§16
- Primary files:
  - `tools/verify_reproducibility.py`
  - `tools/reproducibility-matrix.json`
  - `docs/20-testing.md`
  - `rust-toolchain.toml`
  - `schemas/host-evidence-index.schema.json`
  - `release/host-evidence/`
- Deliverables:
  - supported host/toolchain matrix、clean checkout build evidence、artifact comparison report。
- Tasks:
  1. supported macOS/Linux arch、Rust toolchain、linker/system dependencies、external PDF toolsを`reproducibility-matrix.json`と`rust-toolchain.toml`へexact versionで列挙する。
  2. current sourceからCLIをclean locked buildし、binary version、Git revision、SHA-256、target tripleを記録する。
  3. two-build、independent checkout、異名absolute path、locale/timezone、parallelism variationで全profile fixtureを実行する。
  4. PDF、trace、manifest、diagnostics、capabilitiesのhashをexact比較し、不一致時は最初のsemantic/byte diffを報告する。
  5. OS固有filesystem identity/stable-read testとpackage/output alias matrixを各supported hostで実行する。
  6. toolchain drift、unlocked dependency、missing host evidence、existing target binary reuseをrelease blockerにする。
  7. generated evidenceへsource revision、Cargo.lock hash、binary/tool hashes、fixture hashes、resultをcanonical JCSで記録する。
  8. M1の`machine-profile-evidence` Schemaで各host job outputを検証し、`schemas/host-evidence-index.schema.json`でrequired host、target triple、evidence path/hash、source revisionの完全性を定義する。`verify_reproducibility.py`へmatrix executionと`--require-all-host-evidence`検査を実装し、`release/host-evidence/index.json`をcanonical JCSでatomic出力する。
- Acceptance criteria:
  - documented全hostでcurrent sourceのbuild/check/test/clippyと全release fixtureが成功する。
  - same-toolchain reproducibility failureが具体artifact/offsetまで追跡できる。
  - evidenceだけで実行binary/source/dependency/toolを再特定できる。
- Verification:
  - `python3 tools/verify_reproducibility.py --repository . --revision HEAD --matrix tools/reproducibility-matrix.json`
  - `python3 tools/verify_reproducibility.py --repository . --revision HEAD --matrix tools/reproducibility-matrix.json --require-all-host-evidence release/host-evidence`
- Non-goals:
  - 未列挙toolchain間のbyte一致保証

### MI5-05 Resource governance、font license、release policyを実装する

- Status: Pending
- Depends on: MI4-13
- Design inputs: docs/25 §8 M5、§13.4 media/font plan
- Primary files:
  - `docs/20-testing.md`
  - `docs/26-machine-input-cli.md`
  - `contracts/resource-governance.json`
  - `schemas/resource-governance.schema.json`
  - `samples/machine-package/`
  - `workspace/crates/typaxis-resource-admission/src/`
  - `tools/verify_resource_governance.py`
  - `release/resource-governance-evidence/`
- Deliverables:
  - fixture/resource provenance ledger、license allow/deny policy、release audit。
- Tasks:
  1. repository同梱font/image/vector/math fixtureごとにsource URLまたは生成手順、license/SPDX、redistribution/embedding条件、content hashを`contracts/resource-governance.json`へ記録し、Schemaを追加する。
  2. release profileで許容するmedia/font embedding permissionとrestricted resource failureをpolicy化する。
  3. fixture bytesとledger hash、declared media/profile、expected admission factsをlocalまたは明示的に管理するrelease hostで照合する。
  4. MI4-12がmanifestへbindしたlicense/embedding factsとMI4-13 production evidenceをgovernance ledgerに照合し、M5で新しいmanifest fieldや旧contractの意味を追加せずrelease evidenceへ参照する。
  5. unknown license、hash drift、missing provenance、prohibited embedding、undeclared binary fixtureをrelease blockerにする。
  6. resource parser/decoder/toolのversionとsecurity update手順、profile停止時のfail-closed手順を文書化する。
  7. capabilityからsecurity停止featureを削除しても旧profile IDを別意味へ再利用しないtestを追加する。
  8. `verify_resource_governance.py`でledger、versioned bytes、manifest fixture、capability resource profileを双方向照合する。
- Acceptance criteria:
  - 全versioned binary fixtureにhash、provenance、license/permissionがある。
  - restricted/unknown resourceはPDF開始前に拒否される。
  - release evidenceからembedded resourceとpermission判断を追跡できる。
- Verification:
  - `python3 tools/verify_resource_governance.py --ledger contracts/resource-governance.json --root .`
  - `cargo test --manifest-path workspace/Cargo.toml --package typaxis-resource-admission embedding_policy --locked`
- Non-goals:
  - 法的助言の自動生成

### MI5-06 Release profile gateと公開statusを閉じる

- Status: Pending
- Depends on: MI5-01, MI5-02, MI5-03, MI5-04, MI5-05
- Design inputs: docs/25 §7 production rule、§8 M5、§10
- Primary files:
  - `workspace/crates/typaxis-machine-profile/src/`
  - `workspace/crates/typaxis-cli/src/`
  - `workspace/crates/typaxis-cli/tests/`
  - `contracts/machine-pdf-capabilities.md`
  - `docs/21-roadmap.md`
  - `docs/22-contract-matrix.md`
  - `docs/23-implementation-checklist.md`
  - `docs/26-machine-input-cli.md`
  - `schemas/release-evidence.schema.json`
  - `release/`
  - `tools/verify_release_profile.py`
- Deliverables:
  - immutable release profile descriptor、pre-output readiness gate、reviewed release evidence。
- Tasks:
  1. release profileのcontract、feature/style/resource/limit/accessibility/publication/tool requirementsをclosed descriptorとして生成する。
  2. producer requestとpackage factsをdescriptorへ照合し、missing/unsupported capabilityをresource/layout/output開始前に全件canonical orderで拒否する。
  3. descriptorの各itemをpositive fixture、negative preflight、fuzz target、tamper edge、differential observation、host evidenceへ双方向mapする。
  4. M1〜M4の旧profile/default/contract/Schema goldenを全て再実行する。
  5. `schemas/release-evidence.schema.json`を追加し、release候補binaryでproduction fixtureをclean check/buildして、全artifactとMI5 evidenceを`release/evidence.json`へbindする。
  6. failed gateが一件でもあればcapabilities/release notes/support matrixへrelease statusを公開しない。
  7. 全gate成功後だけroadmap、contract matrix、checklist、producer docs、release notesをactual statusへ更新する。
  8. security停止時のprofile removal/fail-closed behaviorと旧ID非再利用をrelease regressionへ含める。
  9. `verify_release_profile.py`でdescriptor itemとMI5 evidenceを双方向照合し、source revision/tool identityを持つcanonical release manifestをSchema/JCS検証する。
- Acceptance criteria:
  - same-toolchain reproducibility、limits、fuzz、differential、tamper、resource governance gateが全て成功する。
  - release profileの未対応capabilityはprocess/output開始前に拒否される。
  - public status、capabilities、actual parser/profile behavior、release evidenceが一致する。
- Verification:
  - `cargo fmt --manifest-path workspace/Cargo.toml --all -- --check`
  - `cargo check --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo test --manifest-path workspace/Cargo.toml --workspace --all-targets --locked`
  - `cargo clippy --manifest-path workspace/Cargo.toml --workspace --all-targets --locked -- -D warnings`
  - `python3 schemas/validate.py`
  - `python3 tools/verify_release_profile.py --profile release --evidence release/evidence.json`
- Non-goals:
  - gate未通過featureのexperimental成功をrelease supportとみなすこと

## 10. Requirements traceability

各findingは、下表のmilestoneが完了して初めてResolvedへ更新する。途中milestoneの成功だけでfinding全体を閉じない。

| Finding | Primary closure milestones | Observable closure |
|---|---|---|
| TMI-001 machine input command/decoder | MI1-02, MI1-03, MI1-04, MI1-15, MI1-17 | public commandsとstrict bounded decoderのCLI E2E |
| TMI-002 trusted machine ingestion | MI1-05, MI1-06, MI1-07, MI1-08 | sealed receipt以外のpromotionがcompile-fail |
| TMI-003 source proof/receipt | MI1-07, MI1-08 | single-source actual bytes/hash/identity map closure。multi-sourceはM1で明示拒否 |
| TMI-004 JSON security/limits | MI1-03, MI1-04, MI1-09, MI5-01 | exact/max+1、duplicate escaped key、deep/arbitrary bytes、long fuzz |
| TMI-005 manifest machine identity | MI1-12, MI1-14, MI2-02, MI2-08, MI3-12, MI4-13 | raw/canonical/source/profile/all-flow/resource-media/output factsのSchema-backed manifest |
| TMI-006 capability gate | MI1-10, MI1-15, MI1-17, MI2-08, MI3-12, MI4-13, MI5-06 | descriptor/fixture双方向coverageとpre-layout rejection |
| TMI-007 general flow/fragmentation | MI2-02, MI2-04, MI2-05, MI2-06, MI3-02, MI3-03 | list/page break/figure/tableのmulti-flow progress E2E |
| TMI-008 link/figure/image paint | MI2-06, MI2-07, MI4-04, MI4-11 | PDF XObject/annotation/destination closure |
| TMI-009 production book model/style/resource | MI2-03, MI3-01, MI3-05, MI3-08, MI4-01, MI4-03, MI4-06, MI4-08, MI4-13 | production fixtureのlossless end-to-end生成 |
| TMI-010 resource profile | MI2-06, MI4-01, MI4-02, MI4-03, MI4-04, MI4-10, MI4-11, MI4-12, MI4-13, MI5-05 | closed declared mediaとdecoder attestationをbindし、PNG/SafeVector（SVG subsetまたはvector IR）/JPEG/OTF-CFFをprofile別にadmit/embed/audit |
| TMI-011 structured diagnostics | MI1-09, MI1-13, MI1-14, MI1-17, MI5-02 | canonical success/failure sidecar、stable code/location/order |
| TMI-012 publication semantics | MI1-13, MI1-17, MI2-07, MI4-07, MI4-09, MI5-03 | 個別atomic publication、typed partial outcome、navigation/tagged PDF validator |
| TMI-013 macOS clean build | MI0-01, MI1-17, MI5-04 | current-source locked macOS build/check/test/clippy/E2E evidence |
| DOC-001 status axis混同 | MI0-02, MI1-17, MI2-08, MI3-12, MI4-13, MI5-06 | contract/implemented/E2E/release statusが各公開時点で一致 |
| DOC-002 CLI INPUT/round-trip不明瞭 | MI0-02, MI1-02, MI1-17 | producer guide、canonical serializer、round-trip fixture |
| DOC-003 roadmap/checklistの誤読 | MI0-02, MI1-17, MI5-06 | actual statusとfuture milestoneを分離して公開 |
| DOC-004 producer guide/fixture不足 | MI1-16, MI1-17, MI2-08, MI3-12, MI4-13 | runnable packageとnormative guide |
| DOC-005 binary build/run/version | MI0-01, MI1-17, MI5-04 | source revision/binary hash付きactual-host evidence |

## 11. M0/M1 definition-of-done mapping

docs/25 §16の各条件を、最初の公開単位で閉じるmilestoneへ対応付ける。

| DoD | Closure milestones |
|---:|---|
| 1. documented hostsのlocked build/check/test/clippy/font E2E | MI0-01, MI1-16, MI1-17 |
| 2. helpとactual parser一致 | MI1-15, MI1-17 |
| 3. receiptなしpromotionがcompile-fail | MI1-05, MI1-07, MI1-08 |
| 4. actual source bytesでspan/TextMap再検証 | MI1-06, MI1-07, MI1-08 |
| 5. profile descriptorからmanifestまで同一identity | MI1-10, MI1-11, MI1-12, MI1-14 |
| 6. unsupported featureのpre-resource/pre-layout拒否 | MI1-10, MI1-15, MI1-16 |
| 7. manifestがinput/layout/outputをbind | MI1-12, MI1-13, MI1-14, MI1-16 |
| 8. canonical structured diagnostics sidecar | MI1-09, MI1-13, MI1-14, MI1-16 |
| 9. alias/stable-read/limit/tamper/publication fail-closed | MI1-03, MI1-04, MI1-06, MI1-13, MI1-16 |
| 10. reproducibility/round-trip/differential | MI1-02, MI1-16, MI1-17 |
| 11. Schema/ADR/docs/samples/support matrix一致 | MI0-02, MI1-14, MI1-16, MI1-17 |
| 12. M2以降をadvertiseしない | MI1-10, MI1-16, MI1-17 |

## 12. Execution and verification protocol

### 12.1 Milestone開始条件

1. `Depends on`の全milestoneがCompletedで、そのverification evidenceへのlinkが記録されていること。
2. 冒頭の`Design source commit`以降にdocs/25または参照contractが変わった場合、差分を本タスク文書へ反映してreviewを再実行すること。
3. 対象profile/contract/ADRがdecision gateの場合、ADR Accepted前にimplementation milestoneへ着手しないこと。
4. milestone branch開始時にlocked workspace baselineを実行し、既存failureは新規failureと分けて記録すること。

### 12.2 Milestone完了記録

各milestoneの`Status`をCompletedへ変更するとき、直下へ次を追記する。

- implementation commit
- verification commandとexit status
- test/fixture/evidence path
- capability/contract/Schemaへの変更有無
- documented host/tool identity
- scope deviationと採択ADR

verification未実行、required tool missing、success skip、known failure残存のmilestoneはCompletedにしない。

### 12.3 Change-set invariants

- public commandはMI1-17、各追加profileは対応integration milestone、release profileはMI5-06より前にadvertiseしない。
- profile IDはimmutable closed contractとし、feature追加・拒否から受理への変更・既定policy変更にはnew profile IDを発行する。
- wire shape変更はADRでcontract compatibilityを判定し、必要な新contract、Schema、serializer、decoder、capability、manifest、fixtureをatomic migrationする。
- new domain variantはWire serializerとprofile preflight双方でexhaustiveに扱う。serializer対応とprofile supportを同義にしない。
- raw filesystem handle/pathをtrusted syntax/layoutへ渡さず、receipt chainとstable ledgerを越えるbypass APIを追加しない。
- limitはinclusiveなexact/max/max+1を検査し、max+1 allocation/evaluation前に停止する。
- failure時のvisible artifact集合はtyped publication outcomeと一致させ、PDFだけのpartial successを残さない。

### 12.4 Validation tiers

各milestoneの個別verificationに加え、integration/publication milestoneでは次のtierを順に実行する。

1. Format/static: `cargo fmt`、`cargo check --workspace --all-targets --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings`
2. Unit/property: 変更crateのtargeted tests、exact/max+1、tamper、no-panic/progress tests
3. Workspace: `cargo test --workspace --all-targets --locked`、`python3 schemas/validate.py`
4. Public E2E: clean-built CLI、positive/negative/combined fixtures、help/parser/capability coverage
5. Artifact closure: PDF/trace/manifest/diagnostics/capabilitiesのSchema、hash、selected-state、publication assertions
6. External: reproducibility、renderer/extractor、structure/accessibility validator。required tool不在はfailure
7. Release only: scheduled fuzz、full tamper coverage、supported host matrix、resource governance

## 13. Task document quality gates

本タスク文書自体を更新した場合は、実装着手前に次を通す。

1. 全milestone IDが一意で、`Depends on`参照が存在し、dependency graphがacyclicである。
2. 各milestoneにStatus、dependencies、design inputs、primary files、deliverables、tasks、acceptance criteria、verification、non-goalsがある。
3. docs/25のTMI-001〜TMI-013、DOC-001〜DOC-005、M0/M1 DoD 1〜12にclosure milestoneがある。
4. M2〜M5の未確定policyはimplementation中の暗黙判断にせず、明示ADR milestoneのacceptanceで閉じる。
5. command/path/contract/profile/Schema/error/limit名は導入milestoneで定義され、後続milestoneから追跡できる。
6. public status変更が最後のintegration/release gateより先行しない。
7. placeholder、実行不能なverification、循環依存、旧contract/profileの意味変更が残っていない。

## 14. Review record

- Pass 1: capability contract path、M0/M1 dependency、unproduced PDF verification path、negative assertion、M2/M3 dependency leakを検出し、canonical path、ordered dependency、fixture-driven verifier、named testsへ修正した。
- Pass 2: RFC 8785 member orderingをUTF-8順としていた誤り、M2/M3のstaging contractとcurrent alias切替の混在、既存`ResourceLimits`と重複するlimit名を検出し、UTF-16 code-unit順、integration milestoneでのatomic migration、existing limit mappingへ修正した。
- Pass 3: M1 contract切替前のlimit正本、command-wide diagnostic budget、host matrix evidence、M3 table styleとcontractの衝突、M4 staging activation、M5 evidence Schema ownershipの不足を検出し、owner・publication gate・fixed table subset・Schema/index pathを明記した。
- Initial decomposition final pass: milestone field/dependency/task-number/traceability/table/heading検査、placeholder/stale-name検索、Schema validation、Git whitespace検査を実行し、その時点のfindingを解消した。
- Document review pass 4: `MachinePdfPreflightReceipt`より早いmanifest依存、曖昧なresource ID型、M2/M3 vertical sliceのconsumer/manifest owner不足、M4 resource declarationのmedia discriminator/version owner不足を検出し、dependency、typed ID、Display/PDF/manifest closure、declared/decoder-attested media contractへ修正した。
- Document review pass 5: crate-private staging runnerとversioned staging Schemaのowner不足、multi-column artifact closure不足、old/new manifest media representationの混在、JPEG/OTF ADRから未確定SafeVector contractへの依存を検出し、integration switch、slice-local Schema validation、旧manifest凍結、ADR dependencyへ修正した。
- Document review final pass: design source commit、関連contract/Schema/crate boundary、58 milestoneの必須field/task/dependency DAG、staging/public isolation、requirements traceability、stale term、Schema validator、Git whitespaceを再検査し、未解決findingなし。
