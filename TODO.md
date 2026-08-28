# TODO

## P0 — Repository operating baseline

- [x] Public registry schema generate จาก Rust model ที่ `schema/keyword-registry.schema.json`
- [x] Schema drift check, Draft-07 consumer validation, runtime shared-fixture validation และ policy-boundary regression tests
- [x] Project-local repository operating skill ที่ `.agents/skills/repository-operating-recovery/`
- [x] `Justfile`, Rust format/lint policy, repository contracts, public CLI smoke audit, link/version/package tools และ GitHub workflow template
- [x] Source package รวม `.learnings/`, research, schema, benchmarks และ project-local skills; ตัด build cache/VCS/previous packages/presentation

## P0 — Duplicate scan ที่ยืนยันเนื้อหา

- [x] Content-verified duplicate engine: `size bucket → partial SHA-256 → full SHA-256`
- [x] จำแนก `same_name`, `near_name`, `same_content`, `hard_link` พร้อม confidence/evidence และ deterministic output
- [x] `kept scan <directory>` สร้างหรือ refresh persistent index พร้อม deterministic delta และ scan diagnostics แบบ read-only
- [x] `kept review`, `kept find` และ `kept duplicates` ใช้ index เดียวกัน; มี TTY review menu และ non-interactive `--json`/`--action export-plan --yes`
- [x] **User config lifecycle and read-only naming review**: `kept setup` สร้าง OS-appropriate `config.yaml` เพียงครั้งเดียว; `kept config` สรุป config และ `config defaults/profile/scope` จัดการ YAML ของผู้ใช้ผ่าน task-level commands; `config edit` เป็น advanced path; `doctor`/`doctor --fix` รายงานและกู้ missing/invalid config พร้อม backup; `review` วิเคราะห์ naming จาก scope absolute ที่ผู้ใช้กำหนดโดยไม่ rename/apply
- [x] **Registry group management**: `kept group` แสดง supported field types, list/show/add/set/remove/move groups และ list/add/remove group schema fields โดยตรวจ registry ก่อน save
- [x] **Duplicate mutation policy and residual regressions**
  - เริ่มจาก review/export เท่านั้น; ออกแบบ mutation policy, allow-list, rollback และ simulation ก่อนเปิด delete/rename/hard-link replacement
  - เพิ่ม unreadable-file, partial-hash collision และ action-scope regression cases

## P1 — Evidence system and data foundation

- [x] Foundation schema 1.2.0: deterministic migration, UTF-8/NFC profile, glossary/provenance/regex/classifier/corpus-manifest slots
- [x] Public CC0 Thai seed corpus (`words_th`, `stopwords_th`) พร้อม URI/license/SHA-256 manifest และ integrity test
- [x] JSONL anonymized importer, evidence-first classifier, regex vector validator และ repeated experiment selection
- [ ] **Evidence run and correction loop**
  - immutable run manifest, raw JSONL, offline rescore และ append-only correction history
  - `kept:debt` marker parser/ledger และ gain scoreboard ที่เปรียบเทียบเฉพาะ comparable runs
  - self-test good/bad fixtures และ correctness/safety gates
- [ ] **Gold keyword corpus, review and dictionary**
  - 10,000 provenance-linked reviewed assertions ครอบคลุม canonical/variant/synonym/homograph/homophone/semantic relation/transliteration/command/prohibition/named entity/numeral/ambiguity/negative/boundary
  - เก็บ raw scan/Markdown/HTML/PDF เป็น source evidence และใช้ sidecar annotation เมื่อแก้ source ไม่ได้
  - accepted review materialize เป็น dictionary; hypothesis/fuzzy candidate ห้าม promote อัตโนมัติ
  - [x] เพิ่ม CLI สำหรับ corpus import, validation report, snapshot save/load และ experiment replay โดยไม่เปิด automatic keyword mutation
- [ ] เพิ่ม anonymized real-world corpus เมื่อมี export ที่ตัดข้อมูลอ่อนไหวแล้ว

## P1 — Filesystem query and incremental index

- [ ] **Filesystem query language and saved queries**: compile `ext:pdf size>50MB dup:content` เข้า `FilterSet`, พร้อม `--explain`, saved query และ config presets
- [x] **Persistent scan index baseline**: `kept scan` อ่าน/เขียน snapshot, report refresh delta และให้ `find`/`review`/`duplicates` ใช้ default หรือ portable `--index` snapshot
- [ ] **Persistent incremental index optimization**: ลดงาน traversal/hash ตาม refresh plan โดยไม่ลดความถูกต้องของ full scan contract

## P2 — kept-doc fidelity and intake

- [x] MarkdownAlertFilter: GitHub Alerts → typed Universal IR `Callout` พร้อม fixtures
- [ ] GitHub Flavored Markdown writer: quote/list/table/code/divider/image/link/nesting พร้อม golden round-trip และ explicit unsupported diagnostics
- [ ] Notion converter fidelity: typed mapping/diagnostics, property mapping และ page/database mention resolution แบบไม่บังคับ network
- [ ] Optional PDF inspector adapter: `kept doc inspect-pdf`, native-text Markdown, per-page provenance และ OCR opt-in; ใช้ `research/PDF_INSPECTOR_RESEARCH.md`
  - library adapter (`kept-doc` `PdfAdapter`) เสร็จแล้ว; ยังขาด CLI command `kept doc inspect-pdf`, per-page provenance และ OCR opt-in
- [ ] Review queue: duplicate groups, large files, scanned PDF pages, validation errors และ unsupported conversion

## P3 — Remote integration

- [ ] Notion Safe Sync: stable remote IDs, idempotency, revision preconditions, dry-run diff, conflict policy, audit log และ rollback design
- [ ] Reconciler: แทน `block_{index}` ด้วย stable remote IDs และ idempotent move/insert/delete detection

## P4 — Operator experience and later extensions

- [ ] Interactive TUI สำหรับ scan, review queue และ action plan บน model เดียวกับ CLI
- [ ] MCP transport แยกจาก CLI core พร้อม schema/versioning, dry-run tools และ permission boundary
- [ ] Thai synonym governance และ explain-search
- [ ] Real-world benchmark corpus สำหรับ duplicate/search recall และ latency จาก distribution
