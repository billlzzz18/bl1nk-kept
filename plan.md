# แผนการตัดสินใจ bl1nk-kept

เอกสารนี้เก็บเฉพาะ **เรื่องจาก research ที่ยังไม่ตัดสินใจ** และต้องปิดก่อนหรือระหว่าง implementation งานที่ต้องทำและเกณฑ์ยอมรับอยู่ใน `TODO.md`; การตัดสินใจที่ปิดแล้วอยู่ใน `docs/adr/`

## Evidence system

| เรื่องที่ยังไม่ตัดสินใจ                                     | ข้อมูลที่ต้องมีก่อนปิด                                                  | กระทบส่วนใด                        |
|-----------------------------------------------------|-----------------------------------------------------------------|-----------------------------------|
| schema และตำแหน่งเก็บ run manifest                     | repeated run จริงหนึ่งรอบ, การ replay หนึ่งครั้ง และตัวอย่าง correction  | replay, rescore, comparable gain  |
| marker syntax และ source location ที่รองรับ            | ตัวอย่าง Rust/Markdown/HTML/PDF ที่มีจริง พร้อม sidecar locator design | debt ledger และ source annotation |
| รูปแบบคำสั่ง `kept inspect`, `kept debt`, `kept gain`   | TDD command contract และ evidence directory จริง                 | CLI surface                       |
| การจัดสรรแหล่งข้อมูล gold corpus                        | แหล่งข้อมูล public/licensed และ provenance rule ครบทุก stratum      | 10K review corpus                 |
| ขั้นตอน review และ policy เมื่อ reviewer ไม่เห็นตรงกัน     | review batch ที่มี accepted/rejected/uncertain จริง                 | dictionary materialization        |
| schema ของ baseline retention และ correction record | สถานการณ์ผลถูก supersede พร้อม run ที่ comparable/incomparable       | gain scoreboard                   |

## PDF adapter

| เรื่องที่ยังไม่ตัดสินใจ                                              | ข้อมูลที่ต้องมีก่อนปิด                                                          | กระทบส่วนใด                     |
|--------------------------------------------------------------|-------------------------------------------------------------------------|--------------------------------|
| version, license และ Cargo feature graph ของ `pdf-inspector` | dependency audit ของ release ที่เลือก                                      | adapter dependency declaration |
| รูปแบบ source metadata ใน Universal IR                        | backward-compatible fixture และ conversion round-trip                   | document model migration       |
| ชุด public PDF fixture                                        | PDF native-text, scanned, mixed, malformed และ complex-layout ที่เผยแพร่ได้ | acceptance tests               |
| OCR engine และ model policy แบบ local                        | checksum, cache, resource limit และ explicit user consent               | optional OCR workflow          |
| รูปแบบ CLI output                                             | inspection/report review ที่มี page diagnostics                            | `kept doc inspect-pdf`         |

## การขยาย benchmark

| เรื่องที่ยังไม่ตัดสินใจ                             | ข้อมูลที่ต้องมีก่อนปิด                                         | กระทบส่วนใด                               |
|---------------------------------------------|--------------------------------------------------------|------------------------------------------|
| search workload และ relevance fixture       | distribution จริงที่ anonymize แล้วหรือ public corpus ที่อนุมัติ | recall และ latency measurement           |
| repetitions, warm-up และ environment policy | trial run บนเครื่องที่รองรับ                                | release comparison metadata              |
| รูปแบบนำเสนอเมื่อเปลี่ยน default                  | baseline/candidate run ที่ comparable ครบหนึ่งชุด           | release notes และคำอธิบาย selected default |

## กติกาการปิดการตัดสินใจ

ปิดรายการได้เมื่อมีข้อมูลตามตารางและ implementation boundary ชัดเจนเท่านั้น จากนั้นสร้าง ADR ลำดับถัดไปใน `docs/adr/` ก่อนเริ่ม implementation จนกว่าจะปิด ให้เก็บ alternatives ไว้ที่นี่และห้ามนำเสนอว่าเป็น feature ที่รองรับแล้ว


## Context Admission & Judge System (ปิดการตัดสินใจแล้ว)

| เรื่องที่ตัดสินใจ | ข้อสรุป | กระทบส่วนใด |
|---|---|---|
| การดึงโค้ด `sqz` เข้า Workspace | พอร์ต/Vendor โมดูลจำเป็น (`sqz_engine`) เข้า `kept-core` / `kept-judge` | `kept-core`, dependencies |
| Context Registry Storage | SQLite backend + In-memory session cache | `kept-core::observation`, `kept-mcp` |
| Code vs Document Parsing | แยก Tree-sitter (Code AST) กับ `kept-doc` (Document IR) เชื่อมกันด้วย `Observation` / `Target` URI | `kept-core`, `kept-doc` |
| Public Contract | เพิ่มหมวด Context Admission & Judge Engine ใน `SPEC.md` | `SPEC.md`, CLI/MCP surfaces |
