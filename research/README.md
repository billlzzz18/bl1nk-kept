# Research index

Research เป็น source-backed input ของ implementation และต้องคงอยู่ใน repository/source handoff. ไฟล์ที่นี่ไม่ใช่ completed ticket และไม่ควรถูกลบเพียงเพราะอ่านจบแล้ว.

| File | Use when |
|---|---|
| `PONYTAIL_SOURCE_FINDINGS.md` | ตรวจ source findings ของ marker, debt, gain, self-test, raw artifacts, rescore และ correction loop |
| `PONYTAIL_TESTING_BENCHMARK_YAGNI_APPLICABILITY.md` | เปลี่ยน benchmark/evaluation contract หรือเทียบสิ่งที่ kept นำมาจาก Ponytail กับสิ่งที่ไม่ควรคัดลอก |
| `PDF_INSPECTOR_RESEARCH.md` | เริ่ม/ทบทวน optional PDF inspector adapter ใน `kept-doc` |

Research-derived work must preserve the distinction between verified source evidence and a kept-specific adaptation. The current evidence-system direction is summarized in `SPEC.md`; active implementation items remain in `TODO.md`.
