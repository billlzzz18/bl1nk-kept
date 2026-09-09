# Changelog

เอกสารนี้ยึดแนวคิดของ [Keep a Changelog](https://keepachangelog.com/) และใช้ semantic versioning. Public API, schema, corpus revision และค่า default ที่เปลี่ยนต้องมี release entry และหลักฐานทดสอบเสมอ.

## [Unreleased]

### Changed
- ลบ `panic!` ออกจาก production path ของ kept-doc: `DatabaseSchema::title()` เปลี่ยน signature จาก `Self` เป็น `Result<Self, SyncError>` (คืน `SyncError::ValidationError("Only one title property allowed per database")` เมื่อ schema มี title property อยู่แล้ว) พร้อม public rustdoc ภาษาอังกฤษ; builder นี้ไม่มี call site ภายนอก จึงไม่กระทบ contract อื่น — evidence: unit tests ใหม่ `sync::schema::tests::title_accepts_first_title_property` และ `sync::schema::tests::title_rejects_duplicate_title_property` (2/2 ผ่าน) พร้อม `cargo fmt --all -- --check` และ `cargo clippy --workspace -- -D warnings` ผ่าน

### Fixed
- แก้ CI ล้มทั้ง job เมื่อบริการ GitHub Actions Cache (`ghac`) ล่มชั่วคราว: step "Set sccache env" ใน `.github/workflows/ci.yml` ตรวจ `sccache --start-server` ก่อนเปิด `RUSTC_WRAPPER`; หาก GHAC backend ไม่พร้อมให้ emit `::warning::` และรันต่อโดยไม่มี compiler cache (fail-open) แทนการพังทุก cargo step ตั้งแต่ daemon startup

## [0.3.1] - 2026-09-10
### Fixed
- แก้ stale release-version tests ใน kept-cli (`main.rs`) และ kept-core (`lib.rs`) ให้ตรง workspace version 0.3.1; ยืนยัน evidence run manifest (TODO 2.1) และ gold corpus workflow (TODO 2.2) ผ่าน contract tests ครบ 9/9 (`tests/test_evidence_run_contract.py`, `tests/test_gold_corpus_contract.py`)

## [0.3.0] - 2026-09-10
### Added

- เพิ่ม `kept-grammar` crate: แยก types, config I/O, validation rules สำหรับ keyword grammar ออกจาก kept-core
- เพิ่ม `token_counter.rs` ใน kept-core: BPE token counting จริงผ่าน tiktoken-rs (cl100k, o200k, fast fallback) — ported จาก SQZ
- เพิ่ม `content_router.rs` ใน kept-core: content-aware admission routing (Safe/Default/Aggressive) ตาม risk patterns + Shannon entropy — ported จาก SQZ
- เพิ่ม `regret_tracker.rs` ใน kept-core: learn from re-reads/verifier fallbacks/information loss, per-content aggressiveness profiles — ported จาก SQZ
- เพิ่ม `sha256_hex()` และ `ref_prefix()` ใน `ContextRegistry`: SHA-256 content-hash dedup พร้อม inline `§ref:HASH§` references — ported จาก SQZ CacheManager
- เพิ่ม LRU eviction ใน `ContextRegistry`: ลบ entries เก่าเมื่อเกิน capacity
- เพิ่ม `lookup_by_hash()` ใน `ContextRegistry`: dedup by content hash ไม่ใช่ URI
- เพิ่ม Windows matrix ใน CI: `ubuntu-latest` + `windows-latest` พร้อม `fail-fast: false`
- เพิ่ม `sccache` ใน CI ผ่าน `mozilla-actions/sccache-action@v0.0.6`
- เพิ่ม `* text=auto eol=lf` ใน `.gitattributes` สำหรับ cross-platform line ending normalization
- เพิ่ม `MAVIS_ACP_COMMAND` ใน `.env` สำหรับ Mavis skill runtime
- เพิ่ม Tree-sitter AST symbol extraction (Phase 1.1): `kept-core/source_graph/syntax.rs` ใช้ `tree-sitter` + `tree-sitter-rust` parse definitions (fn/struct/enum/trait/type/mod/macro), imports, impl relationships และ call/type references แทน regex string matching; regex scanner คงไว้เป็น fallback
- เพิ่ม `syntax::parse_rust_outline()` คืน structural outline (`StructureOutlineItem` พร้อม children และ 1-based range) สำหรับผูกเข้า `StructurePayload` ของ Observation
- เพิ่ม contract tests `tests/source_graph_ast.rs` (7 cases): string literal/comment ไม่กลายเป็น definition, multi-line signature, generics, impl trait for type, macro_rules, call references
- เพิ่ม multi-language symbol extraction (Phase 1.1): `source_graph/multilang.rs` รองรับ Python (`def`/`class`/`import`), JavaScript/TypeScript (`function`/`class`/`method`/`interface`/`type`/`enum`/`import`) และ Go (`func`/`method`/`type_spec` struct/interface/alias/`import_spec`) ผ่าน `IndexBuilder::parse_file()` dispatch ตาม extension; `scan_recursive` และ `apply_events` index ทุกภาษาที่รองรับ
- เพิ่ม dogfooding tests: parse ไฟล์จริงใน workspace (`src/source_graph/syntax.rs`, `tools/extract_obsidian_corpus.py`) ยืนยัน extraction ทำงานกับโค้ดจริง

### Changed

- CI verify job ใช้ `shell: bash` ทุก step เพื่อข้าม platform (Windows/Linux)
- CI auto-detect python command (`python` บน Windows, `python3` บน Linux)
- ลบ dead Cargo.lock drift check (`cmp -s`) ออกจาก CI — lock ไม่ track จึงไม่มีผลจริง
- ปรับ `.gitattributes` เพิ่ม `*.zip binary` สำหรับ codeql-db/src.zip
- SQZ source เก็บเฉพาะ reference ที่ `D:\01work\Active\references\campbellr\sqz\` — ไม่ vendor

### Fixed

- แก้ `Cargo.toml` เสียหายจากการแปะทับ: quote ครบถ้วน (`digest`, `block-buffer`/`crypto-common`, `schemars`/`strsim`, `tree-sitter`), แยกบรรทัดที่ติดกันและเติม `strsim = "0.11"` ที่ kept-core ใช้จริงใน duplicate detection
- ลบ workspace dependencies ที่ไม่มี crate ใด inherit (`digest`, `block-buffer`, `crypto-common`, `cpufeatures`, `index`, `hashbrown`) — เป็น transitive deps ของ sha2/hmac อยู่แล้ว
- unify `sha2` เป็น 0.11.0 ผ่าน workspace inheritance (kept-core เดิม 0.10.9, kept-doc เดิม 0.11.0) และเปลี่ยน hex encoding เป็น `hex::encode` (digest 0.11 ไม่มี `LowerHex` บน Output)
- downgrade `schemars` เป็น 0.8 ตาม API ที่โค้ดใช้จริง (`RootSchema`, `definitions`, Draft-07) และลด `.lock()` ใน `token_counter.rs` ตาม tiktoken-rs 0.12 ที่ singleton sync ภายใน
- คืน `tree-sitter` เป็น 0.25.10 (เวอร์ชันที่ verified กับ grammar crates 0.23/0.24 ตาม TODO Phase 1.1) และลบ semver build metadata ออกจาก `toml` requirement เพื่อจบ cargo warning
- แก้ Mavis CLI Windows compatibility: เพิ่ม Winsock init (WSAStartup) + threading-based readline แทน selectors (selectors ใช้ได้แค่ sockets บน Windows)
- ลบ broken symlink `composio` ที่ทำให้ skill traverse crash
- ติดตั้ง Claude Code v2.1.263 ผ่าน winget (Anthropic.ClaudeCode)

- เพิ่มคำสั่ง read-only evidence/corpus foundation: `kept evidence run|rescore|correct append|self-test` และ `kept corpus validate|snapshot save|replay`; raw evidence และ hypothesis ไม่ถูก promote อัตโนมัติ.
- เพิ่ม optional `searchPolicy` ใน public registry contract ให้เจ้าของ registry กำหนด `fuzzyMinSimilarity` (`0.0..=1.0`), candidate limit, n-gram size และ posting cap ได้ โดย registry ที่ไม่มี policy ใช้ default เดิม.
- เพิ่ม task-first commands `kept scan`, `find`, `review`, `duplicates`, `search` และ `convert`; `scan` สร้างหรือ refresh persistent index, `review` อ่าน index เดียวกัน และ `find`/`duplicates` ไม่ scan ซ้ำ.
- เพิ่ม persisted `ScanIssue` เพื่อให้ index บันทึก read/metadata diagnostics และ scan ข้ามจุดที่อ่านไม่ได้แทนการยกเลิกทั้งรอบ.
- เพิ่ม `--index <path>` สำหรับ `find`, `review` และ `duplicates` เพื่อใช้ portable snapshot ที่สร้างจาก `scan --output`.
- เพิ่ม `kept setup`, `kept config`, `kept config edit`, `kept doctor` และ `kept doctor --fix` สำหรับ lifecycle ของ user-owned `config.yaml`: setup สร้างครั้งเดียว, config แสดงหรือเปิด editor, doctor ตรวจ config/editor และ fix เก็บ backup ก่อน restore เมื่อ config เสีย.
- เพิ่ม naming analysis แบบ read-only ใน `kept review` จาก `config.yaml` และ ScanIndex เดิม โดยรายงาน violations, proposed target และ collision แต่ไม่มี rename/apply/rollback command.
- เพิ่ม task-level configuration management: `kept config fields`, `config defaults`, `config profile`, `config scope`, scope exceptions และ scope overrides; ทุก mutation validate ก่อนเขียน user-owned YAML และ scope รับเฉพาะ absolute path ที่ผู้ใช้ระบุ.
- เพิ่ม filename-token `shortcuts` ใน naming model เป็น keymap แยกจาก keyboard shortcut; merge และ read-only analysis ใช้ shortcut ก่อน aliases ตาม deterministic transform order.
- เพิ่ม public `kept group` สำหรับดู supported field types, สร้าง/แก้/ลบ/เรียง registry groups และจัดการ group field schemas ที่ validator รองรับ.
- เพิ่ม `.agents/REQUIREMENTS.md` และ repository contract สำหรับ agent handoff: ต้อง reconcile requirement, evidence และสถานะก่อนเริ่มหรือปิดงาน โดยคง `.learnings/` เป็น agent memory แยกจาก product documentation.
- เพิ่ม `CorrectionLedger` (`kept-core/context/correction_ledger.rs`): SQLite immutable ledger เก็บ `CorrectionRecord` พร้อม supersede history, `conflict_for` query ตรวจ active corrections ก่อน acquisition และ `active_for`/`history_for` ดึง record ตาม subject URI.
- เพิ่ม `Judge::with_ledger` และ `evaluate_with_ledger`: pre-acquisition guard ที่ตรวจ correction ledger ก่อนส่ง observation เข้า context stream — ถ้ามี active correction ที่ `rejected_assertion` ตรงกับ content จะคืน `AdmissionDecision::Block` ทันทีพร้อมแนบ correction ID และ evidence URI.
- เพิ่ม behavioral guardrails 3 ตัวใน `Judge`: ต่อต้าน (1) unproductive verbosity บนการ dispatch ที่ fail, (2) subagent sprawl สำหรับงานที่ควรตอบตรง, (3) narrative extrapolation โดยไม่มีหลักฐาน URI กำกับ.

### Changed

- Generated Draft-07 schema และ runtime validator ปฏิเสธ `fuzzyMinSimilarity` นอกช่วง; `kept registry search` ตรวจ registry policy ก่อนสร้าง search index.
- ซ่อน `registry`, `fs`, `doc`, `tui` และ `mcp` จาก root help; command compatibility เดิมยังรับได้สำหรับ script เดิม แต่เอกสารผู้ใช้และ help หลักใช้ task-first surface.
- ไม่เพิ่ม `policy` command เพราะ policy เป็นโครงสร้าง configuration ภายใน; user-facing flow ใช้ `setup`, `config`, `doctor` และ `review` ตามงานที่ผู้ใช้ต้องการทำ.
- เพิ่ม `just cli-smoke` เข้า `just check` เพื่อสร้าง binary และตรวจ root help, subcommand help, setup/config/doctor, config defaults/profile/scope, group lifecycle/schema, scan/find/review/duplicates, search และ convert บน temporary fixtures ทุกครั้ง.
- เปลี่ยน `kept config` จาก raw YAML dump เป็น summary ของ profiles, scopes และ task-level next steps; `config edit` เหลือเป็น advanced escape hatch.

### Fixed

- แยก `NOTE-001` ออกจาก text ของ Clap help เพื่อไม่ให้ marker ภายในหลุดสู่ CLI.
- คำสั่ง interactive จะเปิด dialog เฉพาะเมื่อทั้ง stdin และ stdout เป็น terminal; การใช้ pipe หรือ script จบแบบ non-interactive.
- CSV registry import สร้าง entry object ที่มี `id` และ `aliases` array ตาม schema จึงใช้กับ `kept search` ได้หลัง import.

## [0.2.0] — 2026-08-19

### Added

- เพิ่ม Foundation schema `1.2.0` สำหรับ normalization profile, glossary, provenance, regex catalog, classifier policy slot และ corpus manifest slot.
- เพิ่ม deterministic migration `1.1.0 → 1.2.0` ที่ทำงานก่อน validation; loader คืน registry ปัจจุบันและ save boundary ปฏิเสธ registry ที่ยังไม่ migrate.
- เพิ่ม UTF-8 decode gate, NFC normalization, whitespace policy และ Latin-only case policy ที่มี regression tests.
- เพิ่ม typed glossary/provenance validation, regex compile/test-vector validation และ evidence-first classifier ที่คืน dimension scores, reason IDs และไม่ auto-add keyword.
- เพิ่ม JSONL anonymized importer ที่ mask email, IP address, phone, UUID และ `/home/<user>` path พร้อม rejection report.
- เพิ่ม public CC0 Thai corpus snapshot (`words_th`, `stopwords_th`) พร้อม URI/license/SHA-256 manifest และ integrity test.
- เพิ่ม repeated experiment primitives สำหรับ raw runs, distribution summary, false-positive extrema และ deterministic default-selection rule.
- เพิ่ม `foundation_experiment` release harness: 7 repetitions, 10,000 public Thai lexical records, exact-name และ near-name strata; เก็บ raw JSONL และ summary ไว้ใน `benchmarks/data/foundation_th_cc0_r1/`.
- เพิ่ม versioned persistent scan snapshot, root/options fingerprint และ deterministic refresh plan (`added`, `modified`, `removed`, `unchanged`).
- เพิ่ม `kept fs duplicates scan` แบบ content-verified (`size → partial SHA-256 → full SHA-256`) พร้อม `same_content` และ hard-link subgroup reporting โดย scan ไม่ mutate filesystem.

### Changed

- ย้าย workspace และ crates ไปที่ package version `0.2.0`; registry schema ปัจจุบันคือ `1.2.0`.
- Validator ยอมรับเฉพาะ registry schema ปัจจุบันที่มี Foundation profile และตรวจ normalization/regex catalog ก่อน validation rules เดิม.
- ค่า default ของ name duplicate detection เปลี่ยนจาก `nameNgramSize=2`, `maxNgramPostings=1024` เป็น `3`, `256`.

> การเปลี่ยนค่า default อ้างอิง `benchmarks/data/foundation_th_cc0_r1/summary.json`: candidate `t090-n3-p256` รักษา F1 ของ baseline ใน stratum ที่มี ขณะลด p95 latency และ mean candidate count ภายใต้ false-positive envelope ของ baseline. ผลนี้ **ไม่** อ้างว่าแทน real-world file distribution; ต้อง replay เมื่อมี anonymized corpus ที่มี strata เพิ่มขึ้น.

### Fixed

- แก้ mixed same-content group ให้รายงาน hard-link subgroup แยกจาก content copies.
- ลบ legacy dynamic converter registry ที่ไม่มี call site และคง Reader/Writer typed registry ด้วย regression test.
- รวม MCP document response serialization และซ่อม MCP feature/binary build path.
- แก้ benchmark fixture ให้ใช้ schema ปัจจุบัน เพื่อให้ validation benchmark วัด path ที่ถูกต้อง.

### Migration notes

Registry `1.1.0` ถูก migrate ใน memory โดยเติม Foundation profile, normalization policy และ collections ว่าง. Migration ใช้ `metadata.lastUpdated` เดิมเป็น timestamps; หากว่าง ใช้ `1970-01-01T00:00:00Z` เพื่อให้ rerun deterministic. Registry version ใหม่กว่าหรือ incompatible จะถูกปฏิเสธ.

## [0.1.0] — 2026-08-18

### Added

- Cargo Workspace สำหรับ `kept-core`, `kept-doc` และ `kept-cli`.
- `kept fs index`, `kept fs filter`, `kept fs treemap` และ `kept doc convert`.
- Keyword registry validation, BM25 inverted index, Thai bigram retrieval, fuzzy candidate retrieval และ staged name duplicate detection.
- `CHANGELOG.md`, `CONTRIBUTING.md` และ baseline test/clippy gates.

### Fixed

- ทำให้ Workspace build/test ได้ด้วย Rust stable และแก้ schema defaults, dependency declarations, CSV adapter, converter error mapping และ public filter exports.
- แก้ README ที่เคยอ้าง Notion live sync และ CLI contract เกินความสามารถที่มีจริง.
