# Changelog

เอกสารนี้ยึดแนวคิดของ [Keep a Changelog](https://keepachangelog.com/) และใช้ semantic versioning. Public API, schema, corpus revision และค่า default ที่เปลี่ยนต้องมี release entry และหลักฐานทดสอบเสมอ.

## Unreleased

### Added

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
