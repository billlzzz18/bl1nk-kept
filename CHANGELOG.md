# Changelog

เอกสารนี้ยึดแนวคิดของ [Keep a Changelog](https://keepachangelog.com/) และใช้ semantic versioning. Public API, schema, corpus revision และค่า default ที่เปลี่ยนต้องมี release entry และหลักฐานทดสอบเสมอ.

## [Unreleased]

## [0.3.0] - 2026-09-07

### Added

- เพิ่ม Layer 0 Observation Contract (`Target` URI: `file://`, `symbol://`, `search://`, `context://`, `Revision`, `ContentIdentity`, `Source`, `Observation`) ใน `kept-core`.
- เพิ่ม FFF Acquisition Adapter (`look` outline vs `view` content) ใน `kept-core::scanner::fff` แทนที่ recursive `read_dir` traversal เดิม.
- เพิ่ม Context Admission & Judge Engine (`ContextRegistry`, `Judge`, `AdmissionDecision`: `Pass`, `Reference`, `Delta`, `Compress`, `Warn`, `Block`) ป้องกัน duplicate context dumps และตรวจจับ tool-call loops.
- เพิ่ม unified MCP tools ใน `bl1nk-kept-mcp`: `filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`, `filesystem_acquire` ที่ผูกกับ `Judge` engine.
- เพิ่มคำสั่ง `just quick` (fmt + test) และ `just docs` (eol + links + version-check) เพื่อลดรอบเวลาการพัฒนาและรักษา `just check` เป็น pre-release gate.

### Changed

- อัปเกรด Workspace version เป็น `0.3.0`.
- ปรับโครงสร้าง `kept-cli/src/main.rs` แยกคำสั่งย่อยเข้า `src/commands/` เพื่อความกระชับและ maintainability.

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
