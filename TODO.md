# TODO.md แผนงาน kept

## P0 — Repository operating baseline

## 1. Handoff, สัญญา และแผนงานที่ใช้งานจริง

- [ ] ปรับ `AGENTS.md` เมื่อเริ่ม implementation ให้มี repository map ของ FFF filesystem engine, `kept-vault`, package manifest, lockfile, package store, script runtime และ MCP tool modules ที่สร้างจริง

- [ ] ระบุ ownership ตามโค้ดจริง: `kept-core` เป็น filesystem/vault domain, `kept-cli` เป็น command surface, `kept-mcp` เป็น long-running MCP service, `kept-doc` เป็น document conversion library

- [ ] เพิ่มงานที่เริ่มจริงลง `TODO.md` เป็น checkbox ระดับ behavior ไม่รวม report หรือ ticket ซ้ำซ้อน

- [ ] อัปเดต `SPEC.md` เฉพาะ public CLI/MCP/package contract ที่ implementation และ smoke proof ผ่านแล้ว

- [ ] อัปเดต `CHANGELOG.md` เฉพาะ behavior สาธารณะที่เปลี่ยนจริงใน session นั้น

- [ ] reconcile `.agents/MEMORY.md` ทุก requirement ที่แตะ พร้อม red test, focused test, command proof และเอกสารที่เปลี่ยน

- [ ] รักษา flow ทุก slice เป็น TDD red → minimal implementation → focused green → command-level proof → `just check`

## 2. Observation Contract, Evidence Model และ FFF Integration ใน kept-core

### 2.1 Foundation: Observation, Identity & Evidence Data Model (ง่ายสุด - Layer 0 Contract)
- [x] ทำ TDD red สำหรับ `Target` URI parser และ format invariants (`file://`, `symbol://`, `search://`, `context://`) ใน `crate/kept-core/tests/observation_contract.rs`
- [x] สร้าง `Target` enum และ URI parser (`crate/kept-core/src/observation/target.rs`): parse canonical URI, resolve schemes, deterministic display
- [x] สร้าง `Revision` และ `ContentIdentity` models (`crate/kept-core/src/observation/identity.rs`): แยก mtime/version จาก content hash (BLAKE3/SHA-256)
- [x] สร้าง `Source`, `Evidence`, `Provenance`, `Observation` struct types (`crate/kept-core/src/observation/types.rs`): รองรับ serialization/deserialization แบบ deterministic
- [x] รัน focused contract tests ยืนยัน Observation model serialization และ identity invariants ผ่าน 100%

### 2.2 FFF Acquisition & Filesystem Engine (ปานกลาง - File Acquisition Primitive)
- [x] ทำ TDD red สำหรับ fixture ที่มี Git repository, `.gitignore`, `.ignore`, hidden files, `node_modules`, `venv`, `.venv`, `__pycache__`, `target`, symlink, binary file, modified file และ untracked file
- [x] เขียน test ยืนยันว่า scan คืน file set ตาม FFF ignore semantics และ path ทุกตัวเป็น relative path แบบ deterministic
- [x] เขียน test ยืนยัน metadata ที่ kept ต้องใช้: path, name, extension, size, modified time, binary state และ Git status
- [x] ขยาย `crate/kept-core/src/scanner/fff.rs` เพิ่ม FFF acquisition layer (`look` สำหรับ outline/metadata vs `view` สำหรับ content read)
- [x] ใช้ FFF mode สำหรับ agent, content indexing, Git status cache และ ignore engine ใน adapter
- [x] สร้าง typed errors สำหรับ FFF initialization failure, invalid root, scan timeout และ index-not-ready
- [x] ขยาย `FileRecord` ให้เก็บ `isBinary` และ `gitStatus` แบบ backward-compatible
- [x] version-bump `PersistentScanSnapshot` และสร้าง migration/read error สำหรับ snapshot รุ่นที่ไม่มี FFF metadata
- [x] แทน `std::fs::read_dir` recursive traversal ใน `scanner/scan.rs` ด้วย FFF adapter
- [x] เก็บ `ScanIssue` สำหรับ metadata/indexing failure โดยไม่ทำให้ผล scan ส่วนที่เข้าถึงได้หายไป
- [x] ลบ traversal implementation เดิมเมื่อ adapter tests ผ่านและไม่มี code path เรียกใช้งานแล้ว

### 2.3 Context Registry & Judge Engine Baseline (ท้าทาย - Context Admission Pipeline)
- [x] ทำ TDD red สำหรับ `ContextRegistry` เก็บ snapshot revision ที่ agent เคยอ่านแล้ว เพื่อป้องกัน duplicate context reads
- [x] สร้าง `ContextRegistry` (`crate/kept-core/src/context/registry.rs`) รองรับ `context://<target>@<rev>` tracking
- [x] ทำ TDD red สำหรับ `Judge` admission decisions: `Pass`, `Reference`, `Delta`, `Drop`, `Warn`, `Block`
- [x] สร้าง `Judge` engine v1 (`crate/kept-core/src/context/judge.rs`) สำหรับตัดสิน treat observations ก่อนส่งให้ agent / context stream

## 3. ย้าย kept scan, find, review, duplicates และ incremental index ไปใช้ FFF-backed index

- [ ] ทำ TDD red สำหรับ `kept scan <root>` ให้สร้าง snapshot จาก FFF file inventory และรายงาน file count, total size, issues และ refresh delta แบบ deterministic

- [ ] แก้ `create_or_refresh_scan` ให้รอ FFF initial scan/index readiness ก่อนเขียน snapshot

- [ ] รักษา `kept scan --output`, `--json`, default user-state snapshot และ portable snapshot ให้ทำงานต่อเนื่อง

- [ ] ทำ TDD red ให้ `kept scan` เคารพ `.gitignore`; `--include-hidden` ต้องไม่ bypass ignore rule

- [ ] ทำ TDD red สำหรับ `kept find --query` ด้วย typo path query แล้ว assert ว่าคืน fuzzy match, score และ Git status

- [ ] รักษา `kept find --type`, `--name`, `--path`, `--min-size`, `--max-size`, `--after`, `--before` และ FQL facts เดิม

- [ ] ให้ FFF คัด candidate paths ก่อน แล้วให้ `FilterSet` ตัดสิน extension, size, modified time, duplicate facts และ custom filters ต่อ

- [ ] เพิ่ม FFF score และ Git status ใน JSON output แบบ additive โดยไม่เปลี่ยน field เดิม

- [ ] ทำ TDD red สำหรับ `kept review` ให้ใช้ FFF-backed snapshot เดียวกับ scan และยังแสดง space summary, scan issue, integrity finding และ naming finding ได้

- [ ] คง duplicate verification `size bucket → partial SHA-256 → full SHA-256` ไว้ โดยใช้ candidate records จาก FFF-backed snapshot

- [ ] ทำ TDD red ยืนยัน same-name, near-name, same-content และ hard-link reports ไม่ regress หลังเปลี่ยน engine

- [ ] ทำ persistent incremental index optimization โดยใช้ FFF watcher events และ refresh plan เพื่อลด traversal/hash งานซ้ำ แต่ยังมี full rescan fallback เมื่อ watcher แจ้ง event loss หรือ ignore rules เปลี่ยน

- [ ] อัปเดต CLI smoke ให้ครอบคลุม ignore semantics, fuzzy find, portable index, review และ duplicate report จาก snapshot เดียวกัน

## 4. รวม FFF, filesystem tools และ MCP contract เข้า bl1nk-kept-mcp

- [ ] ทำ TDD red สำหรับ unified MCP tool registration: `filesystem_find`, `filesystem_grep`, `filesystem_multi_grep`, `filesystem_rescan`, `filesystem_status`

- [ ] สร้าง `FffManager` ใน `crate/kept-mcp/src/mcp/` เก็บ FFF instance ต่อ canonical root และ reuse index/watcher ตลอดอายุ MCP server

- [ ] ให้ manager สร้าง instance เมื่อเรียก root ครั้งแรก, รอ readiness ด้วย timeout, reuse instance เดิม และ cleanup worker/watcher/cache เมื่อ server ปิด

- [ ] ทำ `filesystem_find` คืน fuzzy path, size, modified time, binary state, Git status, score, pagination cursor และ total matches

- [ ] ทำ `filesystem_grep` คืน path, line number, column, line content, context, match range และ cursor

- [ ] ทำ `filesystem_multi_grep` ใช้ OR semantics ของ FFF พร้อม separate constraints

- [ ] ทำ `filesystem_rescan` trigger rescan เฉพาะ root ที่ระบุ และคืน scan state ก่อน/หลังทำงาน

- [ ] ทำ `filesystem_status` คืน active root, indexed file count, scanning state, watcher readiness, warmup state และ error ล่าสุด

- [ ] validate canonical root ทุก tool และใช้ root guard ของ FFF สำหรับ filesystem root/home root

- [ ] ทำ TDD red สำหรับ root isolation, binary file, unreadable metadata, query ไม่พบ, invalid constraint, stale cursor, initial scan timeout และ watcher overflow

- [ ] ทำ MCP schema/versioning สำหรับ filesystem และ vault tools; ทุก tool มี input/output schema version ที่ตรวจได้

- [ ] เพิ่ม dry-run และ permission boundary ให้ tools ที่จะเขียน vault/package state; filesystem search tools ยังคง read-only

- [ ] ส่ง FFF/MCP logs ไป stderr เท่านั้นเพื่อไม่ทำลาย JSON-RPC stdio

- [ ] รัน MCP stdio smoke ที่เรียก filesystem tools พร้อม document tools เดิมใน process `bl1nk-kept-mcp` เดียว

## 5. ตรวจ lifecycle จริง, dogfooding และ Thai Bigram correctness

- [ ] ทำ TDD red สำหรับ watcher events `created`, `modified`, `removed` แล้ว assert ว่า filesystem query เห็น index ล่าสุดโดยไม่สร้าง instance ใหม่

- [ ] ทำ TDD red สำหรับ `.gitignore` เปลี่ยนแล้ว FFF status/rescan state ถูกส่งต่อถึง kept

- [ ] ทำ TDD red สำหรับ snapshot migration และ regression ที่ `review`, `find`, `duplicates`, `--index` ใช้ snapshot เดียวกันหลัง migration

- [ ] ทำ dogfooding run บน workspace จริงสำหรับ `scan`, `find`, filesystem MCP query, `convert` และ semantic search เมื่อพร้อม

- [ ] เก็บ run manifest ของ dogfooding: root type, file count, options, machine/environment, duration, errors, result summary และข้อจำกัด

- [ ] ทำ Thai Bigram tokenizer corpus tests กับคำศัพท์ UI สมัยใหม่, คำติดกัน, อังกฤษปนไทย, acronym, numeral, path, URL, emoji และ punctuation

- [ ] เปรียบเทียบผล BM25/Thai Bigram ก่อนและหลัง FFF filesystem migration โดยใช้ fixture และ query ชุดเดียวกัน

- [ ] รัน focused tests, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `just check`, `just cli-smoke` และ source archive smoke หลัง public CLI เปลี่ยน

## 6. สร้าง crate kept-vault สำหรับ Git-first vault package manager

- [ ] เพิ่ม `crate/kept-vault` เป็น workspace member โดยไม่สร้าง binary ใหม่

- [ ] ทำ TDD red สำหรับ `kept-vault.toml` ที่ประกอบด้วย package id, Git URL, requested ref, resolved commit, optional subdirectory, content roots และ script declarations

- [ ] สร้าง manifest model, validation errors, parser และ deterministic serializer

- [ ] ทำ TDD red สำหรับ `kept-vault.lock` ให้ทุก requested ref resolve เป็น immutable commit SHA และ lockfile เรียง deterministic

- [ ] สร้าง Git resolver สำหรับ clone, fetch, resolve branch/tag/ref เป็น commit SHA และ checkout package ตาม SHA ใน lockfile

- [ ] สร้าง package store ใน OS user-data directory แยกจาก vault root, source checkout และ temporary build output

- [ ] ทำ `kept vault init <path>` ให้สร้าง `raw/`, `wiki/`, `index.md`, `log.md`, `.kept/`, `kept-vault.toml` และ schema/template instruction files

- [ ] ทำ `kept vault package add <git-url> [--ref <ref>] [--subdir <path>]` ให้เพิ่ม package declaration

- [ ] ทำ `kept vault package install` ให้ resolve source refs, เขียน lockfile และ materialize checkout ใน package store

- [ ] ทำ `kept vault package list` แสดง package id, source URL, requested ref, resolved commit, subdirectory และ content roots

- [ ] ทำ `kept vault package status` แสดง checkout state, Git dirty state, lock drift, available scripts และ content root state

- [ ] ทำ `kept vault package update [package]` ให้ resolve ref ใหม่และ update lockfile โดยไม่รัน scripts เอง

- [ ] ทำ `kept vault package remove <package>` ให้ลบ declaration/lock entry; แยก package cache garbage collection เป็น command เฉพาะ

## 7. เพิ่ม package script runtime และ Vault lifecycle commands

- [ ] ทำ TDD red สำหรับ script declaration ที่ใช้ structured command/args array และ reject shell string ที่ต้องพึ่ง implicit shell parsing

- [ ] รองรับ scripts เช่น `["cargo", "test"]`, `["just", "check"]`, `["make", "build"]`, `["bun", "run", "build"]` และ runtime commands ที่ package ประกาศ

- [ ] ทำ `kept vault run <package>:<script> [-- <args...>]` ให้รันจาก checkout ที่ตรงกับ locked commit SHA

- [ ] แสดง package id, locked commit, working directory และ command ก่อนเริ่ม child process

- [ ] stream stdout/stderr แบบ live และคืน exit code เดิมให้ caller

- [ ] ทำ TDD red สำหรับ package ไม่มีใน manifest, package ไม่มีใน lockfile, checkout หาย, commit ไม่ตรง, script ไม่มี, executable ไม่พบ, process failed และ argument forwarding

- [ ] ทำ `kept vault run --list <package>` เพื่อแสดง scripts พร้อม command และ working directory

- [ ] บันทึก runtime execution แบบ append-only ใน `log.md`: package id, commit, script, timestamp, exit code และ artifact path ที่ runtime รายงาน

- [ ] ทำ `kept vault ingest <source-or-package>` อ่าน source/content roots, เก็บ raw source, สกัดสาระ, สร้างหรืออัปเดต entity/concept pages, อัปเดต `index.md` และเพิ่ม log entry

- [ ] ทำ `kept vault lint` ตรวจ orphan notes, broken wikilinks, dead links, contradiction markers, manifest/lock drift, missing checkout และ missing content roots

- [ ] ทำ CLI smoke fixture ที่ add Git package → install → run cargo/just/make script → ingest → lint

## 8. ทำ vault knowledge package, query และ MCP agent tools

- [ ] ทำ TDD red สำหรับ `contentRoots` ให้รับเฉพาะ paths ภายใต้ package checkout/subdirectory ที่ package declaration ระบุ

- [ ] ใช้ FFF index กับ package checkout, raw source และ wiki pages เพื่อค้นหา source ของ vault แบบ live

- [ ] ทำ `kept vault query <query>` ให้ใช้ FFF fuzzy path/content retrieval ก่อน lexical search, semantic retrieval และ reranker

- [ ] คืน citations ที่มี package id, locked commit, relative path, line range, excerpt และ retrieval score

- [ ] ทำ `kept vault query` สังเคราะห์คำตอบจาก wiki/source pages และบันทึกผลเป็น wiki note พร้อม citation/backlink/log entry

- [ ] ทำ MCP tools `vault_ingest`, `vault_query`, `vault_lint` บน `bl1nk-kept-mcp`

- [ ] ทำ TDD red สำหรับ source นอก content roots, missing package checkout, citation ชี้ commit ผิด, broken link หลัง package update และ query ที่ไม่มีผลลัพธ์

- [ ] ทำ end-to-end fixture: add package → install → FFF index → query with citations → ingest → lint → update package → lint drift

## 9. ทำ Hybrid Semantic Search และ Reranking Engine

- [ ] ทำ TDD red สำหรับ dense embedding retrieval ด้วย Ollama `bge-m3` และ Jina provider โดยคง provider/model/endpoint/env override contract ที่มีอยู่

- [ ] เชื่อม dense vector retrieval กับ BM25 lexical search และ Thai Bigram retrieval เป็น hybrid candidate set เดียว

- [ ] ทำ disk-backed vector similarity cache พร้อม key ที่ผูกกับ document/content hash, embedding model id, provider และ schema version

- [ ] ทำ cache invalidation เมื่อ source hash, package commit, model id หรือ embedding settings เปลี่ยน

- [ ] ทำ cross-encoder reranker pipeline รองรับ `bge-reranker-v2-m3` และ Jina Rerank

- [ ] ทำ TDD red สำหรับ reranker unavailable, provider timeout, invalid API key, empty candidate set, duplicate candidate และ fallback ที่ยังคืน lexical result ได้

- [ ] เพิ่ม `kept search --semantic` และ `kept search --hybrid` โดยคืน source, lexical score, vector score, rerank score และ explanation ที่แยกกัน

- [ ] เพิ่ม MCP semantic search tools สำหรับ registry และ vault query โดยคืน citations ไม่ใช่เพียง answer text

- [ ] ทำ regression corpus สำหรับภาษาไทย, ไทยปนอังกฤษ, synonym, typo, named entity, numeral, path และ URL

## 10. ทำ Evidence run, correction loop, gold corpus และ benchmark จริง

- [ ] ทำ immutable evidence run manifest ที่เก็บ input snapshot, corpus revision, model/config, command/options, environment, timestamp และ result hashes

- [ ] เก็บ raw JSONL ของ every evidence run และทำ offline rescore โดยไม่แก้ raw history

- [ ] ทำ append-only correction history ที่ supersede record เดิมได้โดยยัง trace กลับได้

- [ ] ทำ `kept:debt` marker parser และ debt ledger สำหรับ source marker ที่ต้องวัด/ยกระดับในอนาคต

- [ ] ทำ gain scoreboard ที่เปรียบเทียบเฉพาะ runs ที่มี comparable contract; ต่าง corpus/model/config ต้องคืน `incomparable`

- [ ] ทำ good/bad fixtures และ self-test correctness/safety gates ก่อนยอมรับ benchmark gain

- [ ] ทำ workflow เก็บ raw scan, Markdown, HTML, PDF และ sidecar annotation เมื่อแก้ source ไม่ได้

- [ ] ทำ review workflow สำหรับ gold corpus 10,000 assertions พร้อม provenance, stable locator, fragment/context hash, normalized candidate, assertion kind, reviewer decision/reason และ split

- [ ] ครอบคลุม canonical, variant, synonym, homograph, homophone, semantic relation, transliteration, command/prohibition, named entity, numeral, ambiguity, negative และ boundary cases

- [ ] materialize เฉพาะ accepted reviewed assertions เป็น dictionary; hypothesis/fuzzy candidates ห้าม promote เอง

- [ ] เพิ่ม anonymized real-world corpus เมื่อมี export ที่ตัดข้อมูลอ่อนไหวแล้ว

- [ ] สร้าง real-world benchmark corpus สำหรับ duplicate/search recall/latency จาก distribution จริง

- [ ] ทำ Thai synonym governance และ `explain-search` ที่แสดง source relation, rule, score contribution, confidence และเหตุผลที่ไม่ promote candidate

## 11. เพิ่ม kept-doc fidelity, PDF inspection และ review queue

- [ ] ทำ GitHub Flavored Markdown writer ครอบคลุม quote, list, table, code, divider, image, link และ nesting

- [ ] ทำ golden round-trip tests สำหรับ Markdown → Universal IR → Markdown และ explicit unsupported diagnostics

- [ ] ทำ Notion converter fidelity สำหรับ typed mapping, diagnostics, property mapping และ page/database mention resolution โดยไม่บังคับ network

- [ ] ทำ TDD red สำหรับ property type ที่ไม่รองรับ, mention หาไม่พบ, nested blocks และ conversion ที่ต้องคืน diagnostic แทนข้อมูลเงียบ ๆ

- [ ] ทำ `kept doc inspect-pdf <input>` บน `kept-doc::PdfAdapter` ที่มีอยู่แล้ว

- [ ] คืน native-text Markdown, per-page provenance, page diagnostics และ OCR opt-in ที่ไม่ทำงานเองโดยอัตโนมัติ

- [ ] ทำ PDF fixture สำหรับ text PDF, scanned PDF, malformed PDF, password/permission error และ per-page extraction failure

- [ ] สร้าง review queue model สำหรับ duplicate groups, large files, scanned PDF pages, validation errors และ unsupported conversion

- [ ] เพิ่ม CLI/MCP surfaces สำหรับ list/show/filter review queue โดยใช้ model เดียวกับ scan/vault/document diagnostics

## 12. ทำ Notion Safe Sync, reconciler และ operator experience

- [ ] ทำ Notion Safe Sync model สำหรับ stable remote IDs, idempotency key, revision precondition, dry-run diff, conflict policy, audit log และ rollback design

- [ ] ทำ TDD red สำหรับ retry request เดิมแล้วห้ามสร้าง duplicate remote block/page

- [ ] ทำ TDD red สำหรับ remote revision เปลี่ยนระหว่าง sync แล้วต้องคืน conflict ที่ระบุ local/remote state

- [ ] เปลี่ยน reconciler จาก `block_{index}` ไปใช้ stable remote IDs

- [ ] ทำ idempotent detection ของ move, insert, delete และ content update

- [ ] ทำ dry-run output ที่แสดง action plan ก่อนทุก remote mutation

- [ ] สร้าง Interactive TUI สำหรับ scan, review queue และ action plan โดยใช้ domain model เดียวกับ CLI/MCP

- [ ] ทำ TUI keyboard flow, non-interactive fallback และ state restoration tests

- [ ] รัน focused tests ต่อ slice, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `just check`, `just cli-smoke`, MCP stdio smoke และ source archive smoke ก่อนปิด public behavior

## Installer scripts

- [x] ปรับ `install-mcp.sh` ให้ติดตั้ง release asset `bl1nk-kept-mcp` จาก GitHub บน Linux/macOS/Windows shell
- [x] เพิ่ม `install-mcp.ps1` ให้ติดตั้ง release asset `bl1nk-kept-mcp.exe` บน Windows
- [x] เพิ่ม release workflow ให้ build และแนบ MCP binary asset พร้อม SHA-256 checksum
- [x] เพิ่ม release-gate hook จับ `git tag` และ tag push พร้อมหน่วงเวลา 40 วินาที
- [x] เพิ่ม `release-verifier` agent ตรวจ checklist, แก้ TODO และวนตรวจซ้ำก่อนปล่อย release
- [x] ให้ release gate ส่ง deterministic preflight และ detailed findings กลับ main agent ก่อน verifier ตัดสิน PASS/FAIL
- [x] แยก pre-commit ให้รัน `just check` ก่อน commit; release gate ไม่แทนที่ pre-commit
- [x] เพิ่มคำสั่ง `/commit-push-tag` ให้ main agent เก็บงาน ตรวจ แอด คอมมิต ติดแท็ก และส่งต่อ release verifier ตามลำดับ
