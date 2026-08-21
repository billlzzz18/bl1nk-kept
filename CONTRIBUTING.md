# Contributing to bl1nk-kept

เริ่มจาก `AGENTS.md` เสมอ. ไฟล์นั้นระบุ read order, source-of-truth paths, TDD loop และ non-regression constraints. `SPEC.md` คือ specification เดียว; `TODO.md` คือ backlog เดียว.

## Crate boundaries

| Crate | แก้เมื่อความต้องการเกี่ยวกับ |
|---|---|
| `kept-core` | registry schema/migration/search, scanner, duplicate detection, treemap และ filters |
| `kept-doc` | Universal IR, document conversion, document filters และ sync primitives |
| `kept-cli` | command UX, argument parsing และการ compose crate capabilities |

ห้ามคัดลอก business logic ข้าม crate. Public CLI/schema behavior ต้องเปลี่ยนอย่างชัดเจนและมี test ก่อน production code.

## Required commands

```bash
just check
just schema
just benchmark-chart
just package
```

`just check` ครอบคลุม format, Rust workspace tests, Clippy `-D warnings`, repository contracts, Markdown links และ version contract. หากไม่มี `just` ให้รัน recipe เดียวกันจาก `Justfile`.

## Code and data rules

โค้ดใหม่หรือ bug fix ต้องทำ TDD: เขียนและรัน test ที่ล้มเหลวตาม intent ก่อน แล้วเขียน production code ขั้นต่ำจน test ผ่าน. คอมเมนต์ที่อธิบายเหตุผลหรือ constraint ใช้ภาษาไทยรูปแบบ `NOTE-001: <รายละเอียด>` เท่านั้น.

รักษา compatibility ของ CLI/schema เดิม เว้นแต่ breaking change ถูกระบุใน `SPEC.md`, `CHANGELOG.md` และ migration/compatibility test. คำสั่ง CLI ต้องระบุสถานะจริงว่า read-only, writes output, mutates source หรือ unavailable; ห้ามอ้าง behavior ที่ source ไม่มี.

การเปลี่ยน public registry model ต้อง regenerate `schema/keyword-registry.schema.json`. การเปลี่ยน benchmark ต้องเก็บ raw data ใน `benchmarks/data/` และ regenerate chart จาก raw data; ห้ามแก้ตัวเลขหรือ chart ด้วยมือ. Synthetic benchmark ใช้จับ regression เท่านั้น.

`research/` และ `.learnings/` เป็น asset ของ source handoff. ห้ามลบ `.learnings/` หรือให้ product commands อ่านมัน. Source package รวมสองส่วนนี้ แต่ตัด `target/`, `.git/`, `__pycache__/`, previous package `dist/` และ past presentation artifacts.

## Version and handoff

ใช้ `just version-bump <semver>` เพื่ออัปเดต Cargo workspace version, `SPEC.md` และ changelog heading พร้อมกัน. ก่อนส่งงานให้รัน `just check` และ `just package`; ส่งผล test ที่รันจริง, compatibility impact และ ZIP ที่สร้างจาก `dist/`.
