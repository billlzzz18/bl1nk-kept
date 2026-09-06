# บันทึกการตัดสินใจ 0001: สัญญาหลักฐานและ benchmark แบบกำหนดผลได้

**สถานะ:** ตัดสินใจแล้ว  
**วันที่:** 2026-08-20

## บริบท

งานวิจัย Ponytail แสดงว่าตัวเลข benchmark จะมีความหมายเมื่อเครื่องมือวัดจับ input ที่ผิดได้ เก็บ raw run ไว้ แยก correctness ออกจาก performance และเปิดเผยการแก้ไขผลภายหลังได้ `bl1nk-kept` เป็น Rust library และ CLI ที่ผลลัพธ์หลักเป็น deterministic จึงไม่จำเป็นต้องใช้ LLM judge หรือการเปรียบเทียบ agent หลายแขนในขอบเขตปัจจุบัน

## การตัดสินใจ

`bl1nk-kept` ใช้สัญญาหลักฐานแบบ deterministic สำหรับงาน benchmark และการเลือก default

| องค์ประกอบ | การตัดสินใจ |
|---|---|
| correctness ก่อน ranking | integrity, no-mutation, negative fixture, migration และ conversion gate ต้องผ่านก่อนเปรียบเทียบ latency หรือ score |
| raw evidence | เก็บ raw JSONL, revision ของ fixture/corpus, options, repetitions และ environment metadata กับทุก run ที่นำมาเทียบกัน |
| instrument validation | เพิ่ม fixture ที่ดีและผิดโดยเจตนาเพื่อ self-test ก่อนเชื่อผล benchmark หรือ selection |
| การแก้ไขผล | เก็บ correction/supersession โดยไม่เขียนทับประวัติผลเก่าเงียบ ๆ |
| การเปรียบเทียบ | baseline และ candidate ต้องใช้ run contract เดียวกัน; หากต่างกันต้องรายงาน `incomparable` |
| สิ่งที่ไม่ทำ | ไม่เพิ่ม LLM judge, agentic arm, benchmark database หรือ benchmark service สำหรับ Rust/CLI scope ปัจจุบัน |

## ผลกระทบ

Foundation repeated experiment, public corpus manifest, raw JSONL, summary, F1, false-positive และ P95 ที่มีอยู่แล้วเป็นฐานเริ่มต้นของ implementation ต่อไป งานถัดไปเพิ่ม run manifest, self-test, offline rescore, correction history และ comparable gain เมื่อเข้าเกณฑ์ยอมรับใน TODO

การตัดสินใจนี้ไม่ได้ใช้ YAGNI เป็นข้อห้ามสร้าง feature. Validation, การป้องกันข้อมูลเสียหาย, corpus provenance, test ที่ทำซ้ำได้ และ requirement ที่ผู้ใช้สั่งยังเป็นงานจำเป็น

## เอกสารอ้างอิง

- [งานวิจัย Ponytail ด้าน testing และ benchmark](../research/PONYTAIL_TESTING_BENCHMARK_YAGNI_APPLICABILITY.md)
- [Ponytail source findings](../research/PONYTAIL_SOURCE_FINDINGS.md)
- [รายการงาน evidence system](../../TODO.md)
