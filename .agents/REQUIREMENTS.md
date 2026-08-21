# Requirement ledger for agents

เอกสารนี้เป็น **agent-only traceability ledger** สำหรับกัน requirement หล่นข้าม session. ไม่ใช่คู่มือผู้ใช้, product specification, TODO, release note หรือ agent memory. ห้าม product code อ่านไฟล์นี้ และห้ามย้ายเนื้อหานี้ไป `README.md`, `SPEC.md` หรือ `get-start.md`.

> ใช้ `TODO.md` เป็น backlog ของงาน และใช้ `.learnings/` บันทึกความผิดพลาด/บทเรียนของเอเจนต์. Ledger นี้ตอบคนละคำถาม: **คำสั่งของ Director ข้อใดมีหลักฐานแล้ว, ข้อใดยังขาด, และห้ามอ้างว่าปิดแล้วเพราะอะไร**.

## Mandatory work loop

1. ก่อนเลือกงาน ต้องอ่าน ledger นี้, `TODO.md`, `SPEC.md` ส่วนที่เกี่ยวข้อง, `.learnings/ERRORS.md` และ skill ที่เกี่ยวข้อง.
2. เลือก Requirement ID ที่ยัง `MISSING` หรือ `PARTIAL` ก่อนสร้าง subplan. ห้ามสร้าง feature ที่ขัดกับ requirement ที่ยังเปิด.
3. ก่อนแก้ code เพิ่ม acceptance evidence ในแถวที่เกี่ยวข้อง: red test, command simulation หรือ explicit user confirmation ของ public contract.
4. เปลี่ยน status เป็น `COMPLETE` ได้เมื่อมี production path, focused test, command-level proof และไม่ขัดกับ boundary ที่อนุมัติแล้วเท่านั้น.
5. ก่อนส่งมอบ ต้องไล่ทุกแถว `MISSING`/`PARTIAL` ที่แตะในรอบนั้น และระบุว่าไม่ได้ทำเพราะอะไร. ห้ามสรุปว่าทั้ง task เสร็จเพียงเพราะ test ของ feature ล่าสุดผ่าน.
6. หลังการแก้ public CLI ต้องอัปเดต `tests/test_public_cli_smoke.py`, `get-start.md`, `SPEC.md` และ `CHANGELOG.md` ตามหน้าที่ของแต่ละไฟล์; รัน `just check` และตรวจ archive ที่สร้างใหม่.

## Status vocabulary

| Status | ความหมาย |
|---|---|
| `COMPLETE` | มี implementation และ evidence ที่อ้างถึงได้ |
| `PARTIAL` | มีบางส่วน แต่ acceptance ยังไม่ครบหรือ UX/contract ยังขาด |
| `MISSING` | Director สั่งแล้ว แต่ยังไม่มี implementation ที่พิสูจน์ได้ |
| `BLOCKED` | ต้องการ public-contract decision หรือ input ที่ยังไม่มี |
| `NOT APPROVED` | ห้าม implement/mutate จนกว่า Director อนุมัติ boundary แยก |

## Director requirements

| Requirement ID | Directive | Status | Evidence / gap | Next acceptance evidence |
|---|---|---|---|---|
| OPS-001 | ยึดโครงสร้างปัจจุบัน, ลบเฉพาะส่วนล้าสมัย/ซ้ำซ้อน, เอกสารแต่ละไฟล์มีหน้าที่เดียว | `PARTIAL` | Root source-of-truth และ repository contracts มีแล้ว | ตรวจทุกการเปลี่ยนว่าไม่สร้าง report/ticket ใหม่ใน product docs |
| OPS-002 | คอมเมนต์ code ภาษาไทยด้วย `NOTE-001:` เมื่อมีเหตุผลที่ไม่ชัดเจน | `PARTIAL` | ใช้ใน code ที่แก้ล่าสุด | Review code changes ทุกครั้ง; user-facing help ต้องไม่แสดง marker |
| OPS-003 | TDD: red test ก่อน production code ทุก behavior ใหม่ | `PARTIAL` | Cargo/core/CLI contracts และ smoke suite มี | ตรวจ ledger evidence ก่อนปิดแต่ละ requirement |
| OPS-004 | เก็บ `.learnings/` เสมอ; เป็น memory เอเจนต์ ไม่ใช่ product input | `COMPLETE` | `AGENTS.md`, package contract, `.learnings/ERRORS.md` | คงไว้ในทุก source archive |
| OPS-005 | ADR ภาษาไทย; research ที่ตัดสินใจแล้วเป็น ADR, ที่ยังไม่ตัดสินใจอยู่ `plan.md` | `COMPLETE` | `docs/adr/0001_*`, `0002_*`, `plan.md` | ใช้กับ decision ใหม่เท่านั้น |
| OPS-006 | Source ZIP/TAR.GZ ต้องสะอาดแต่เก็บ research/schema/benchmarks/.learnings | `COMPLETE` | `tools/package_source.py`, archive contract tests | Inspect listing ทุกครั้งหลัง package |
| OPS-007 | ก่อน public contract ใหม่ ต้องได้รับการยืนยัน; rename/apply/rollback แยก contract | `COMPLETE` | `SPEC.md` no-mutation boundary | คง `NOT APPROVED` สำหรับ mutation |
| OPS-008 | Final response แสดงเฉพาะการตัดสินใจสำคัญ แต่ต้องไม่ปิดงานก่อน audit requirement ครบ | `PARTIAL` | รอบก่อนปิดเร็วเกินไป; ledger นี้เป็น corrective control | Checklist ledger + `just check` + archive proof ก่อน final |
| OPS-009 | ห้ามทำให้ Director ต้องสั่งเรื่องเดิมซ้ำ | `PARTIAL` | Ledger เริ่มสร้างในรอบนี้ | AGENTS read order และ repository contract ต้องบังคับใช้ |
| CLI-001 | CLI ต้อง task-first, คำสั่งสั้น, option เป็น facts, งานยาวใช้ interactive flow | `PARTIAL` | `scan/find/review/duplicates/search/convert` task-first และ review menus | ตรวจ config/registry management commands ให้สอดคล้อง |
| CLI-002 | `scan` สร้าง persistent index; `find/review/duplicates` ต้องใช้ index เดิม | `COMPLETE` | Scan-index contracts, `tests/test_public_cli_smoke.py` | Regression ผ่าน `just cli-smoke` |
| CLI-003 | Config เป็น user-owned `config.yaml` ที่ OS-appropriate, YAML, สร้างครั้งเดียวไม่ rewrite | `COMPLETE` | `policy.rs`, setup/doctor contracts, smoke test | Preserve across config-management mutations |
| CLI-004 | Scope ต้องเป็น absolute path ที่ผู้ใช้เลือกเอง; ห้ามเดา path จาก scan | `COMPLETE` | naming config contracts and semantic validation | Preserve in every scope add/edit command |
| CLI-005 | Defaults ต้องกว้าง ใช้ได้ทันที และมี starter profiles หลายชุด | `COMPLETE` | `policy.rs` starter config and core contract | Config list/show must expose profiles clearly |
| CLI-006 | Naming settings ต้องรองรับ case, separator, number/word/length/whitespace, alias, shortcut, similarity, replacement, reposition, variables, prefix และ profile | `COMPLETE` | `policy.rs` has typed model, shortcut transform and semantic validation; `config defaults/profile/scope setting set/unset` use the same catalog; focused core and smoke contracts pass | Re-run catalog lifecycle smoke when adding a field |
| CLI-007 | Profile/scope ต้องจัดการได้เป็นงานจริง: list, add, edit, remove และเรียง deterministic | `COMPLETE` | `config profile` lifecycle and `config scope` lifecycle smoke tests; `scope list` prints depth/priority resolve order | Re-run smoke after scope precedence changes |
| CLI-008 | `config edit` เป็น advanced YAML escape hatch ไม่ใช่ UX หลัก | `COMPLETE` | `kept config` landing summary plus defaults/profile/scope command families; `config edit` labelled advanced in smoke-covered docs | Preserve task-level flow when extending config |

| CLI-009 | `review` วิเคราะห์ naming แบบ read-only และต้องไม่ rename/apply/rollback | `COMPLETE` | naming review contracts, CLI smoke | Keep mutation boundary intact |
| CLI-010 | Registry `search --group` ต้องมี task-level way to inspect, add, edit, remove และ order groups; ต้องอธิบายชนิด group ที่รองรับ | `COMPLETE` | Public `group types/list/show/add/set/remove/move` and `group field list/add/remove`; lifecycle and schema smoke tests use a validated temporary registry | Keep registry group separate from naming profile/scope |
| CLI-011 | Interactive UX ใช้เลข/ลูกศร/j-k; non-interactive commands ต้องไม่พยายามเปิด dialog | `PARTIAL` | review/setup menus; terminal handling regression fixed | Cover all future interactive command paths in smoke suite |
| DATA-001 | Gold corpus 10,000 reviewed assertions ครอบคลุม relation/ambiguity/named entity/numeral/file/path/URL/duplicate พร้อม provenance | `MISSING` | `TODO.md` P1 target only | Data collection/review contract and real artifacts required |
| DATA-002 | Raw scan/Markdown/HTML/PDF + sidecar annotation ต้อง materialize เป็น dictionary หลัง review; fuzzy/hypothesis ห้าม promote เอง | `MISSING` | `TODO.md` P1 target only | Corpus and review workflow required |
| DUP-001 | Duplicate mutation policy, allow-list, rollback, simulation แยกจาก scan/review/export | `NOT APPROVED` | `TODO.md` P0 and `SPEC.md` no-mutation boundary | Director approval before any mutation command |
| DOC-001 | `get-start.md` ต้อง cover public commands และตัวอย่างต้องมี command-level proof | `COMPLETE` | get-start sections cover config, defaults, profile, scope, group, scan, review, find, duplicates, search and convert; `tests/test_public_cli_smoke.py` exercises their public paths | Expand immediately when a new public command is approved |

## Distinctions that must not be collapsed

| Concept | Owner / purpose | Current state |
|---|---|---|
| `.learnings/` | Agent memory about failures and corrections | Retained; never product input |
| `.agents/REQUIREMENTS.md` | Agent requirement coverage, status and evidence | This ledger; never product documentation |
| `TODO.md` | Product backlog and active implementation checklist | Public project work source of truth |
| Naming profile / scope | User config behavior for file naming analysis | Task-level defaults/profile/scope/override management complete; read-only review consumer |
| Registry group | Grouping of keyword registry entries and `search --group` filter | Public group and group-field management complete; separate from naming config |
| Duplicate action plan | Explicit exported artifact | Read-only export exists; filesystem mutation not approved |

## Current stop conditions

- Re-run config profile/scope/defaults smoke contracts before claiming a later config change is complete.
- Re-run group lifecycle/schema smoke contracts before claiming a later registry-group change is complete.
- Do not create rename, apply, delete, copy, hard-link replacement, or rollback commands while `DUP-001` is `NOT APPROVED`.
- Do not claim the 10K corpus, dictionary materialization or real-world benchmark work exists while `DATA-001` or `DATA-002` is `MISSING`.
- Do not package or send a final source archive after a public CLI change until `just check` passes and the archive itself executes `just cli-smoke` after extraction.
