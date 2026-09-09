# ข้อกำหนดผลิตภัณฑ์ bl1nk-kept

**Workspace package version:** `0.3.0`

**Registry schema version:** `1.2.0`

**CLI หลัก:** `kept`

**สถานะงาน:** `TODO.md` เป็นแหล่งเดียวของลำดับงานและหลักฐานการปิดงาน

## 1. ขอบเขตและเป้าหมายหลักของผลิตภัณฑ์ (Core Purpose & Scope)

`bl1nk-kept` คือ Rust workspace แบบ local-first ที่ถูกสร้างขึ้นมาโดยมี **เป้าหมายหลัก 2 ประการ**:

1. **Cognitive & Semantic Guardrail (ป้องกัน AI Agent เข้าใจผิดและตีความคลาดเคลื่อน):**
   - เป็นตัวกลางแยกแยะเจตนา (Intent Disambiguation) ไม่ให้ Agent ทึกทักคำกำกวมเป็น Action ที่ตนถนัดฝ่ายเดียว (เช่น คำว่า *"ทดสอบ"* แยกแยะระหว่าง Unit Test โค้ด กับ Dogfooding/การใช้งานจริง; คำว่า *"ข้อผิดพลาด"* แยกแยะระหว่าง System Bug กับ Agent Hallucination/ทำงานไม่ตรงบรีฟ)
   - ป้องกันพฤติกรรมสูญเปล่า (Behavioral Accountability): ยับยั้งการสแกน/อ่านข้อมูลเข้ามาสะสมในหัวโดยไม่เคยบันทึกเป็นผลลัพธ์ (Unproductive Acquisition) และยับยั้งการอ่านไฟล์ซ้ำซ้อนทั้งที่ข้อมูลยังอยู่ใน Prompt Context (In-Context Amnesia)
2. **Intelligent Context & Knowledge Vault Management:**
   - ค้นหา จัดทำดัชนี แปลง ตรวจสอบ เปรียบเทียบ และคัดกรอง Context สู่ AI Agent ด้วย FFF Acquisition Core และ SQZ-derived Judge Engine เพื่อให้ได้เนื้อหาที่แม่นยำ ปลอดภัย และลดโทเค็นซ้ำซ้อนสูงสุด

### 1.1 User Stories: Semantic Disambiguation & Cognitive Guardrail

1. **Intent Alignment (ลดช่องว่างความหมายและป้องกันการเดาผิด):**
   - **As a:** ผู้ใช้ที่สื่อสารด้วยภาษาธรรมชาติหรือภาษาไทยและไม่ได้เชี่ยวชาญศัพท์เทคนิคภาษาอังกฤษ
   - **I want:** ระบบที่ทำหน้าที่เสมือน "หูฟังคัดกรองความหมาย" ที่คอยตรวจจับคำกำกวมและไม่ปล่อยให้ Agent ทึกทักคำสั่งเป็น Action ด้านเทคนิคไปเองฝ่ายเดียว
   - **So that:** ผลลัพธ์ที่สะท้อนกลับมาตรงตามเจตนาจริงของฉัน โดยที่ฉันไม่ต้องเสียพลังงานและโทเค็นมาคอยพิมพ์แก้ต่างหรืออธิบายคำเดิมซ้ำ ๆ
2. **Zero-Regression Learning (จดจำเพื่อไม่ให้เข้าใจผิดซ้ำสอง):**
   - **As a:** ผู้ใช้ที่เคยแก้ไขความเข้าใจผิดของ Agent ไปแล้วหนึ่งครั้ง (เช่น ระบุว่า A ไม่ใช่ a)
   - **I want:** ระบบจดจำข้อเท็จจริงและการแก้ไขนั้นไว้เป็นความรู้ถาวรในคลัง (Immutable Correction)
   - **So that:** เมื่อเจอบริบทเดิมในอนาคต Agent ตัวเดิมหรือตัวใหม่จะไม่เข้าใจความหมาย A เป็น a ซ้ำสองอีกเด็ดขาด
3. **Behavioral Accountability & Token Hygiene (ความคุ้มค่าและไม่กระทำสูญเปล่า):**
   - **As a:** ผู้ใช้งานระบบ
   - **I want:** ระบบ Judge คอยยับยั้ง Agent ไม่ให้สแกนหรืออ่านข้อมูลสะสมเข้ามาในหัวโดยไม่เคยบันทึกผลลัพธ์ (Unproductive Acquisition) หรือแกล้งลืมแล้วอ่านไฟล์ซ้ำทั้งที่ข้อมูลยังอยู่ใน Context
   - **So that:** ทุกโทเค็นและทุกวินาทีที่สูญเสียไปเกิดผลลัพธ์เป็นชิ้นเป็นอันจริง ไม่ตายไปพร้อมกับเซสชัน

| Crate | ความรับผิดชอบ |
|---|---|
| `kept-core` | โมเดล registry, Foundation/evidence, filesystem domain, scan snapshot, filter, duplicate verification, semantic retrieval และ adapter ของ FFF |
| `kept-vault` | manifest/lockfile, Git package resolver, package store, script runtime และ lifecycle ของ vault package |
| `kept-doc` | Universal IR และ primitive สำหรับแปลง/ตรวจเอกสาร; เป็น library เท่านั้น |
| `kept-mcp` | `bl1nk-kept-mcp` stdio MCP server, FFF manager และ tools สำหรับ filesystem, vault และเอกสาร |
| `kept-cli` | command surface ของ `kept` สำหรับ filesystem, registry, vault, evidence และ document work |

`kept-core` ไม่ผูกกับ transport, `kept-doc` ไม่สร้าง binary, และ CLI/MCP ใช้ domain model เดียวกันแทนการมี logic แยกคนละชุด

## 2. Filesystem intelligence

### 2.1 FFF เป็น filesystem engine

Filesystem engine ของ `kept` ใช้ FFF (`fff-search`) สำหรับ traversal, `.gitignore`/`.ignore`/global Git ignore, Git status, fuzzy path search, content search, live watcher และ index readiness. การเดิน directory แบบ recursive ด้วย `std::fs::read_dir` ไม่ใช่ engine หลักของผลิตภัณฑ์

รากการค้นหาต้องเป็น canonical path ที่ caller ระบุชัดเจน. ระบบไม่เดา scope จาก scan เดิม และใช้ guard ของ FFF สำหรับ filesystem root หรือ home root. ทุกผลลัพธ์ใช้ relative path ที่ deterministic ภายใต้ root นั้น

ข้อมูลไฟล์ขั้นต่ำที่ `kept` เก็บและส่งต่อได้คือ:

```text
relative path, filename, extension, size, modified time,
binary state, Git status, scan issue และ FFF search score เมื่อเป็นผล fuzzy search
```

### 2.2 Persistent scan snapshot

`kept scan <root>` สร้างหรือ refresh persistent `ScanIndex` จาก FFF index. Snapshot ผูกกับ canonical root และ scan options, เก็บ schema version และรองรับ migration ของ metadata ที่เพิ่มในอนาคต

Snapshot เริ่มต้นเป็น user-state data ไม่นำไปวางใน root ที่สแกน; `kept scan --output <path>` สร้าง portable snapshot และ `kept find`, `kept review`, `kept duplicates` รองรับ `--index <path>`.

การ refresh ใช้ watcher events และ refresh plan เพื่อลด traversal/hash ที่ไม่จำเป็น. เมื่อ FFF รายงาน event loss, watcher overflow หรือ ignore-file change ระบบต้องทำ full rescan หรือรายงานสถานะที่ต้อง rescan; ห้ามอ้าง index ที่ไม่ครบว่าเป็นผลสมบูรณ์

`ScanIssue` ต้องเก็บ path, operation, kind, message และ remediation สำหรับ failure ที่เข้าถึงข้อมูลไม่ได้ โดยไม่ทำให้ file อื่นที่อ่านได้หายจาก scan

### 2.3 CLI filesystem contract

| คำสั่ง | สัญญา |
|---|---|
| `kept scan <root>` | สร้าง/refresh FFF-backed snapshot, รายงาน file count, total size, issues และ deterministic delta |
| `kept find <root>` | ใช้ FFF fuzzy path search เพื่อคัด candidate แล้วใช้ FQL/`FilterSet` สำหรับ type, name, path, size, modified time และ duplicate facts |
| `kept review <root>` | ใช้ snapshot เดียวกับ scan เพื่อแสดง space, find, duplicates, integrity, scan issues และ naming review แบบ read-only |
| `kept duplicates <root>` | ใช้ snapshot เดียวกันเพื่อหาและพิสูจน์ duplicate; export explicit plan ได้ |

`kept find` คง FQL และ options เดิมไว้. JSON output เพิ่ม FFF score และ Git status ได้แบบ additive แต่ไม่เปลี่ยนชื่อหรือความหมายของ fields เดิมโดยไม่มี migration contract

### 2.4 Duplicate, integrity และ naming

Duplicate verification ต้องเป็นลำดับนี้เสมอ:

```text
size bucket → partial SHA-256 → full SHA-256 → group evidence
```

รายงานต้องแยก `same_name`, `near_name`, `same_content` และ `hard_link` พร้อม evidence/confidence ที่ตรวจสอบได้. FFF เป็นผู้จัดหา file inventory และ live state ไม่ใช่ตัวแทนของ SHA-256 verification

Naming review อ่าน scope และ config ที่ผู้ใช้เลือกเอง, วิเคราะห์จาก persisted snapshot, แสดง violation/proposed target/collision แบบ deterministic และไม่แก้ชื่อไฟล์เอง

## 3. Unified MCP contract

`bl1nk-kept-mcp` เป็น stdio MCP server ที่รวม document, filesystem และ vault tools. Logging ต้องออก stderr เท่านั้นเพื่อไม่ทำลาย JSON-RPC บน stdout

### 3.1 FFF manager

MCP server เก็บ FFF instance ต่อ canonical root ตลอดอายุ process, reuse index/watcher ระหว่าง requests และ cleanup worker/cache handles เมื่อ server ปิด. Instance ต้องรายงาน scan progress, watcher readiness, warmup state และ error ล่าสุดได้

### 3.2 Filesystem tools

| Tool | สัญญา |
|---|---|
| `filesystem_find` | fuzzy path search; คืน path, metadata, Git status, score, total matches และ cursor |
| `filesystem_grep` | content search; คืน path, line/column, line content, context, match range และ cursor |
| `filesystem_multi_grep` | OR search หลาย patterns พร้อม constraints แยกจาก patterns |
| `filesystem_rescan` | trigger rescan ของ root ที่ระบุและคืนสถานะงาน |
| `filesystem_status` | คืน root, indexed file count, scan/watcher/warmup state และ error |

Filesystem tools เป็น read-only. ทุก tool ต้อง validate canonical root, รองรับ pagination/cursor อย่างมีขอบเขต และไม่ข้าม root isolation

### 3.3 Schema, permission และ dry run

MCP tools ทุกตัวมี input/output schema และ version ที่ตรวจได้. Tool ที่เขียน vault/package state ต้องมี permission boundary ชัดเจน และรองรับ dry-run สำหรับ operation ที่เปลี่ยน persistent state หรือ remote state. Document conversion, filesystem search และ read-only inspection ไม่ต้องใช้ mutation permission

## 4. Vault package manager

### 4.1 Vault structure

`kept vault init <path>` สร้าง vault local-first ที่มีโครงสร้างขั้นต่ำดังนี้:

```text
vault/
├─ raw/                 # source evidence ที่เก็บตามเดิม
├─ wiki/                # entity/concept notes และคำตอบที่มี citation
├─ index.md             # catalog ที่สร้าง/อัปเดต deterministic
├─ log.md               # append-only lifecycle/runtime log
├─ kept-vault.toml      # package declarations และ vault settings
├─ kept-vault.lock      # resolved immutable commits
└─ .kept/               # state ภายใน vault
```

Vault ต้องมี template instructions สำหรับ agent และ schema definition ที่บอก source-of-truth ของ raw, wiki, package และ log

### 4.2 Git-first packages

Package declaration ใน `kept-vault.toml` ระบุ package id, Git URL, requested ref, optional subdirectory, content roots และ scripts. `kept-vault.lock` ต้องเก็บ resolved immutable commit SHA ของทุก package และ serialize แบบ deterministic

| คำสั่ง | สัญญา |
|---|---|
| `kept vault package add <git-url>` | เพิ่ม declaration โดยไม่ clone หรือ run script เอง |
| `kept vault package install` | resolve ref, เขียน lockfile และ materialize checkout ใน package store |
| `kept vault package list` | แสดง declaration และ resolved identity ของทุก package |
| `kept vault package status` | แสดง checkout state, dirty state, lock drift, content roots และ scripts |
| `kept vault package update [package]` | resolve ref ใหม่และ update lockfile โดยไม่รัน scripts เอง |
| `kept vault package remove <package>` | ลบ declaration/lock entry; cache GC เป็น operation แยก |

Package checkout อยู่ใน OS user-data package store แยกจาก vault root และ source checkout ของผู้ใช้. ทุก install/run/query ต้องอ้าง identity จาก lockfile ไม่ใช่ branch name ที่เปลี่ยนได้

### 4.3 Script runtime

Package scripts เป็น structured command และ argument array ไม่ใช้ shell string implicit. Runtime รองรับ command เช่น `cargo`, `just`, `make`, `bun` และ executable อื่นที่ manifest ประกาศ

`kept vault run <package>:<script> [-- <args...>]` ต้อง:

1. resolve package และ script จาก manifest/lockfile;
2. ยืนยัน checkout ว่าตรง resolved commit;
3. รันจาก working directory ของ package/subdirectory;
4. stream stdout/stderr แบบ live;
5. คืน exit code เดิม; และ
6. เขียน append-only execution record ลง `log.md`.

Execution record เก็บ package id, locked commit, script, command, working directory, timestamp, exit code และ artifact path ที่ runtime รายงาน

## 5. Vault knowledge lifecycle

### 5.1 Content roots และ FFF index

`contentRoots` ระบุ paths ภายใต้ package checkout/subdirectory ที่อนุญาตให้ใช้เป็น raw content, Markdown, wiki หรือ source code. Path นอก roots ที่ประกาศต้องถูกปฏิเสธ

FFF ทำหน้าที่ index package checkout, raw source และ wiki pages แบบ live เพื่อให้ package source, scripts และ knowledge content ใช้ search engine เดียวกัน

### 5.2 Ingest, query และ lint

| คำสั่ง/Tool | สัญญา |
|---|---|
| `kept vault ingest <source-or-package>` / `vault_ingest` | อ่านเฉพาะ allowed content roots, เก็บ raw evidence, สร้าง/อัปเดต entity-concept pages, อัปเดต catalog และ append log |
| `kept vault query <query>` / `vault_query` | ใช้ FFF retrieval ก่อน lexical/semantic/rerank, สังเคราะห์คำตอบพร้อม citations แล้วบันทึกเป็น wiki note/backlink/log |
| `kept vault lint` / `vault_lint` | ตรวจ orphan notes, broken wikilinks, dead links, contradiction markers, manifest/lock drift, missing checkout และ missing roots |

Citation ของ vault ต้องมี package id เมื่อมาจาก package, locked commit, relative path, line range, excerpt และ retrieval score. Citation ห้ามชี้ branch/ref ที่ไม่ได้ resolve เป็น commit

## 6. Registry และ hybrid semantic search

Registry รองรับ JSON/YAML, CSV import, validation, analysis และ search. Public contract คือ Draft-07 JSON Schema ที่ `schema/keyword-registry.schema.json`; loader migrate schema แบบ deterministic ก่อน validate/save

Search lexical ใช้ BM25, Thai Bigram, synonym compatibility และ n-gram fuzzy candidate retrieval. `searchPolicy` ควบคุม fuzzy similarity, candidate limit, n-gram size และ posting cap โดย registry ที่ไม่มี policy ใช้ compatibility defaults

Hybrid search รวมสามชั้น:

```text
BM25 + Thai Bigram + dense embedding retrieval
                    ↓
             deduplicated candidates
                    ↓
          cross-encoder reranking
```

Dense retrieval รองรับ Ollama `bge-m3` และ Jina. Reranker รองรับ `bge-reranker-v2-m3` และ Jina Rerank. Disk cache ต้องผูกกับ content hash, provider, model id และ schema version และ invalidate เมื่อ identity เหล่านี้เปลี่ยน

`kept search --semantic`, `kept search --hybrid` และ MCP semantic tools ต้องคืน source, lexical score, vector score, rerank score และ explanation แยกกัน รวมทั้ง citations เมื่อค้นจาก vault

Thai tokenizer/search tests ต้องครอบคลุมคำไทยสมัยใหม่, ไทยปนอังกฤษ, acronym, numeral, path, URL, emoji และ punctuation

## 7. Evidence, corpus และ benchmark

ทุก evidence run ต้องมี immutable manifest, raw JSONL, input/corpus revision, model/config, command/options, environment, timestamp, result hashes และข้อจำกัด. Offline rescore สร้างผลใหม่โดยไม่แก้ raw history

Correction history เป็น append-only: record ใหม่ supersede record เก่าได้ แต่ต้อง trace กลับไปยังต้นฉบับได้. `kept:debt` marker ledger บอก ceiling, trigger และ upgrade path โดยไม่ block งานที่ไม่เกี่ยวข้อง

Gain scoreboard เปรียบเทียบได้เฉพาะ runs ที่มี corpus/model/config/run contract เดียวกัน; ต่างกันต้องรายงาน `incomparable`. Good/bad fixtures และ correctness/safety gates ต้องผ่านก่อนยอมรับ performance gain

Gold corpus เป้าหมายคือ 10,000 reviewed assertions ที่มี provenance, stable locator, fragment/context hash, normalized candidate, assertion kind, reviewer decision/reason และ split `build/validation/holdout = 7,000 / 1,500 / 1,500`

Corpus ต้องครอบคลุม canonical, variant, synonym, homograph, homophone, semantic relation, transliteration, command/prohibition, named entity, numeral, ambiguity, negative และ boundary cases. เก็บ raw scan/Markdown/HTML/PDF หรือ sidecar annotation เมื่อแก้ source ไม่ได้. Materialize เป็น dictionary ได้เฉพาะ accepted reviewed assertions

Benchmark ต้องเก็บ raw data, corpus distribution, options, environment, repetition/warm-up, gate status และ limitations. Synthetic benchmark ใช้จับ regression; real-world anonymized corpus ใช้วัด duplicate/search recall และ latency จาก distribution จริง

Thai synonym governance และ `explain-search` ต้องแสดง source relation, rule, score contribution, confidence และเหตุผลที่ candidate ไม่ถูก promote

## 8. Document fidelity และ review queue

`kept-doc` แปลงระหว่าง Universal IR กับ Markdown, GitHub, Notion, Lark และ adapters อื่น โดยให้ typed mapping และ diagnostics มากกว่าการทิ้งข้อมูลเงียบ ๆ

GitHub Flavored Markdown writer ต้องรองรับ quote, list, table, code, divider, image, link และ nesting พร้อม golden round-trip tests และ explicit unsupported diagnostics

Notion converter ต้องรองรับ typed mapping, property mapping และ page/database mention resolution โดยไม่บังคับ network. หาก resolve ไม่ได้ ต้องคืน diagnostic ที่อธิบายข้อมูลขาดหาย

`kept doc inspect-pdf <input>` ใช้ `PdfAdapter` เพื่อคืน native-text Markdown, per-page provenance และ diagnostics. OCR เป็น opt-in เท่านั้น

Review queue ใช้ model เดียวกับ CLI/MCP เพื่อรวม duplicate groups, large files, scanned PDF pages, validation errors และ unsupported conversion ให้ list/show/filter ได้แบบ deterministic

## 9. Notion Safe Sync และ reconciler

Notion sync ใช้ stable remote IDs, idempotency key, revision preconditions, dry-run diff, conflict policy, audit log และ rollback design ก่อนทำ remote mutation

Reconciler ห้ามใช้ `block_{index}` เป็น identity. ต้องตรวจ move, insert, delete และ content update แบบ idempotent จาก stable remote IDs

ทุก remote mutation ต้องมี dry-run action plan ที่อธิบาย action, precondition, local/remote revision และผลกระทบ. เมื่อ revision ไม่ตรง ต้องคืน conflict ที่แยก local state กับ remote state ชัดเจน และไม่เขียนทับเงียบ ๆ

## 10. Operator experience และการตรวจรับ

Interactive TUI ใช้ domain model เดียวกับ CLI/MCP สำหรับ scan, review queue และ action plan. ต้องมี keyboard flow, state restoration และ non-interactive fallback

ทุก public behavior ใหม่ต้องมี focused test, CLI หรือ MCP command-level proof, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --all -- --check`, `just check`, `just cli-smoke` และ MCP stdio smoke ตามส่วนที่เปลี่ยน

หลัง public CLI เปลี่ยน ต้องทดสอบ source archive ที่สร้างใหม่และรัน `just cli-smoke` จาก archive ที่แตกออกมา

สถานะ implementation, ลำดับงาน และ acceptance checklist ของทุกข้ออยู่ใน `TODO.md` เท่านั้น

## 11. Context Admission, Observation & Judge Engine

ระบบ Context Admission มีหน้าที่ควบคุมการส่งมอบ Context ให้กับ AI Agent/Client เพื่อลดความซ้ำซ้อนของ Token และรักษาความถูกต้องของข้อมูล

### 11.1 Observation Model & Target Locator

ทุกการเข้าถึงข้อมูลต้องถูกแปลงให้อยู่ในรูป `Observation`:
- **Source:** แหล่งที่มา เช่น `File`, `Grep`, `TreeSitter`, `Parser`, `Fts`, `Bm25`, `Vector`
- **Target URI:** ระบุทรัพยากรแบบไม่คลุมเครือ (`file://`, `symbol://`, `search://`, `document://`, `context://`)
- **Revision & ContentIdentity:** ติดตาม timestamp/token และ Hash Digest ของเนื้อหา

### 11.2 Acquisition Layer (FFF & Structure)

- `look`: ดึง identity, size, revision, outline แบบประหยัด token โดยไม่อ่านเนื้อหาทั้งหมด
- `view`: materialize เนื้อหาตาม range หรือ symbol เมื่อจำเป็น
- โครงสร้างโค้ดใช้ Tree-sitter AST, โครงสร้างเอกสารใช้ `kept-doc` Universal IR

### 11.3 Context Registry & State Tracking

- บันทึกประวัติและ state ของ resource ที่ Agent เคยเห็นลงใน Session Store
- ติดตาม `ResourceState` และ revision tracking ป้องกัน duplicate reads

### 11.4 Judge Engine & Treatments

ประเมิน `Observation` เทียบกับ Context History และ Policy เพื่อเลือก Decision:
- `PASS`: ข้อมูลใหม่ ส่งมอบเนื้อหาเต็ม
- `REFERENCE`: ข้อมูลเดิมที่เคยเห็นและยังไม่เปลี่ยนแปลง (ส่งคืนเฉพาะ pointer 13 tokens)
- `DELTA`: ข้อมูลเดิมที่มีการเปลี่ยนแปลง (ส่งคืนเฉพาะ diff)
- `COMPRESS`: บีบอัดเนื้อหาตามโครงสร้างเมื่อคุ้มค่า
- `WARN` / `BLOCK`: แจ้งเตือนหรือระงับเมื่อ Agent ร้องขอข้อมูลซ้ำซ้อนเกินกำหนด (threshold ≥3 ครั้ง)

## 12. Content Acquisition, Retrieval & Benchmark Discipline (FFF vs Ripgrep)

ระบบของ `kept` ไม่ใช่แค่ File Metadata Filter แต่เป็น **Cognitive & Context-Aware Content Engine** ที่จัดการเนื้อหาโค้ดและเอกสารแบบครบวงจร

### 12.1 สถาปัตยกรรม 8 ส่วน (FFF + Tree-sitter + Search + SQZ Judge)

```text
                     KEPT
            user/client semantics
                      │
                      ▼
             ┌─────────────────┐
             │ Request / Intent│
             └────────┬────────┘
                      ▼
            ┌───────────────────┐
            │   FFF-based Core  │
            │ acquire + observe │
            └─────────┬─────────┘
                      │
      ┌───────────────┼────────────────┐
      ▼               ▼                ▼
File Operations    Structure        Retrieval
FFF primitives    Tree-sitter      grep/fuzzy
look/view          Parser           BM25/FTS
watch/git          symbols          vector
diff/apply         outline          rerank
      └───────────────┼────────────────┘
                      ▼
                 Observation
                      │
                      ▼
              ┌───────────────┐
              │     JUDGE     │
              │  SQZ-derived  │
              └───────┬───────┘
                      │
      ┌───────────────┼────────────────┐
      ▼               ▼                ▼
    reuse           delta           select
    pass            compress        warn/error
      └───────────────┼────────────────┘
                      ▼
               Context Output
                      │
                      ▼
                MCP / Client
```

1. **Object กลาง (`Observation`):** ทุกกลไก acquisition (FFF, Tree-sitter, Grep, BM25, Parser) ต้องคืน `Observation` ที่มี schema เดียวกัน ประกอบด้วย:
   - `Source`: `File`, `Grep`, `Fuzzy`, `Glob`, `Git`, `Diff`, `TreeSitter`, `Parser`, `Fts`, `Bm25`, `Vector`, `Hybrid`
   - `Target`: Identity ที่ชัดเจน (`file://...`, `symbol://...#...`, `search://...`)
   - `Revision` & `ContentIdentity`: แยก revision ออกจาก content digest เพื่อตรวจจับ event การแก้ไข
   - `ContentPayload`, `Evidence`, `Provenance`
2. **FFF Acquisition & Observation Core:**
   - ใช้ FFF เป็น library wrap capabilities: `find`, `grep`, `glob`, `look`, `view`, `watch`, `git`, `diff`, `apply`
   - `look`: ต้นทุนต่ำ คืน identity, size, revision, outline โดยไม่อ่านไฟล์เต็ม
   - `view`: materialize เนื้อหาตาม range หรือ symbol
3. **State / Event Semantic:**
   - ติดตาม `ResourceState` (id, revision, content_identity, last_event) ร่วมกับ watcher
   - เมื่อไฟล์เปลี่ยน `revision++` และ invalidate observation เดิมทันที ทำให้ระบบรู้ว่า context เก่าหรือใหม่โดยไม่ต้องอ่านไฟล์ซ้ำ
4. **Structural Acquisition (Tree-sitter & Document IR):**
   - Tree-sitter รับหน้าที่แยก structure/symbols/outline ของ source code
   - `kept-doc` รับหน้าที่แยก structure ของ Markdown, NFM, DOCX, PDF
   - สนับสนุนการ view เจาะจงเฉพาะ `symbol://<path>#<symbol_name>`
5. **Unified Retrieval Engine:**
   - รวมกลไก `grep` (exact), `fuzzy`, `BM25/FTS`, `vector`, `rerank` ภายใต้ Retrieval Planner เดียว
   - Public intent เป็น `search(query)` หรือ `find(query)` โดยระบบเลือกกลไกภายในที่เหมาะสมที่สุด
6. **Judge Pipeline (SQZ Decision):**
   - ประเมินตามขั้นตอน: `VALIDATE` → `CHANGE DETECTION` → `DUPLICATE DETECTION` → `RELEVANCE/SCOPE` → `POLICY` → `TREATMENT SELECTION` → `BUDGET` → `Context Admission`
   - คืน `Decision`: `Pass`, `Reference`, `Compress`, `Delta`, `Select`, `Summarize`, `Defer`, `Drop`, `Warn`, `Block`
7. **Context Registry:**
   - จดจำประวัติ `ContextEntry` ใน session ทำให้ request ใหม่สามารถเลือกระหว่าง `REFERENCE` หรือ `DELTA` ได้
8. **Measurable Compression:**
   - Compression ต้องเป็น treatment ที่คำนวณ token reduction ratio และ latency เทียบกับ budget จริง

### 12.2 Benchmark Discipline (FFF vs Ripgrep)

การพัฒนา Acquisition และ Search Engine ต้องมี Benchmark กำกับเสมอเพื่อวัดประสิทธิผลเทียบกับมาตรฐานอุตสาหกรรม (โดยเฉพาะ Ripgrep / `rg`):

1. **Component Benchmark (FFF vs rg):**
   - วัด Throughput, Latency, Memory Usage และ Streaming behavior ระหว่าง FFF grep กับ `rg` บน cold cache และ warm cache
   - วัด Tree-sitter symbol extraction latency, BM25 indexing speed และ Compression latency
2. **Treatment Benchmark:**
   - วัดการประหยัดโทเค็นของแต่ละ Treatment: `PASS` vs `COMPRESS` vs `SELECT` vs `DELTA` vs `REFERENCE`
3. **Workflow Benchmark:**
   - วัด End-to-end task (Understand repository, Fix bug, Refactor, Repeated inquiry, Large-file navigation)
4. **Client Impact Benchmark (เปรียบเทียบระหว่าง Client ธรรมดา กับ Client + kept):**
   - Context emitted reduction (%)
   - Repeated reads reduction (%)
   - Tool calls reduction (%)
   - Time-to-completion (%)
   - Judge overhead latency (ms/call)
   - Task success rate (%)
