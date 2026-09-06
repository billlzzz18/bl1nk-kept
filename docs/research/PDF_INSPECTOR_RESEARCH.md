# Research: Firecrawl pdf-inspector สำหรับ kept-doc

**สถานะ:** เอกสารวิจัยและข้อเสนอออกแบบเท่านั้น ไม่ได้เพิ่ม dependency หรือเปลี่ยนพฤติกรรม runtime ของ `kept-doc`  
**ตรวจสอบเมื่อ:** 18 สิงหาคม 2026  
**แหล่งข้อมูล:** Context7 และเอกสาร Rust API/README ของโครงการต้นทาง

## ข้อสรุปสำหรับ kept-doc

`pdf-inspector` เหมาะเป็น **optional inbound adapter** ของ `kept-doc` มากกว่าจะเป็นตัวแทน Universal IR หรือระบบ OCR ทั้งหมดเอง. ตัว library ตรวจประเภท PDF, สกัด Markdown และส่งสัญญาณรายหน้าว่าหน้าใดควรเข้า OCR ได้ ขณะที่ `kept-doc` มี Universal IR และ Markdown converter อยู่แล้ว จึงควรให้แต่ละส่วนรับผิดชอบงานคนละชั้น: `pdf-inspector` อ่านและคัดแยก PDF, `kept-doc` แปลง Markdown/ข้อมูลโครงสร้างเข้าสู่ Universal IR, ส่วน OCR เป็นงานที่เลือกเปิดและควบคุมภายนอกตามนโยบายของผู้ใช้ [1] [2].

> ข้อเสนอคือเริ่มจาก PDF ที่เป็น native text ก่อน และเก็บผลการตรวจว่าเอกสารหรือหน้าใดต้อง OCR ไว้ในผลลัพธ์. ไม่ควรทำให้การเปิดไฟล์ PDF ปกติดาวน์โหลดโมเดล, เรียกบริการภายนอก หรือเปลี่ยน default dependency ของ `kept-doc`.

## ความสามารถที่ยืนยันได้

| ความสามารถของ pdf-inspector | หลักฐาน | ประโยชน์ต่อ kept-doc |
|---|---|---|
| `detect_pdf` แยก `TextBased`, `Scanned`, `ImageBased`, `Mixed` พร้อม confidence และรายการหน้า OCR | Rust API [2] และ Context7 | ทำ routing ก่อนเริ่มแปลงเอกสาร แทนการเดาเพียงจากนามสกุลไฟล์ |
| `process_pdf` ให้ประเภท, confidence, page count และ Markdown เมื่อสกัดได้ | Rust API [2] | เส้นทางง่ายสำหรับ PDF native text ที่ต้องการแปลงเข้าระบบเอกสาร |
| `extract_pages_markdown` คืน Markdown รายหน้า, `needs_ocr` และ `is_complex` | Context7 [3] | ทำ partial conversion และเก็บรายการหน้าที่รอ OCR ได้ชัดเจน |
| `PdfOptions` เลือก `ProcessMode`, sampling strategy และ subset ของหน้าได้ | Rust API [2] | คุมต้นทุนและ latency สำหรับ PDF ขนาดใหญ่หรือ workflow ที่เลือกหน้าเอง |
| default build เป็น pure Rust ไม่มีโมเดลหรือบริการภายนอก | Rust API [2] | เหมาะกับ offline-first ของ CLI ปัจจุบัน |
| OCR เป็น feature opt-in; `vision`, `model-cache`, `model-download`, `render-pdfium` มี dependency/operational boundary ของตนเอง | Rust API [2] | ป้องกันไม่ให้ `kept-doc` ขยาย dependency และ supply-chain surface โดยไม่จำเป็น |
| มี text position/font/layout metadata และสร้าง Markdown ที่ตรวจ heading, list, code และ table | README [1] และ Context7 [4] | เป็นทางเลือกที่ดีกว่าอ่าน PDF เป็น plain text เมื่อโครงสร้างและลำดับการอ่านสำคัญ |

## รูปแบบ integration ที่แนะนำ

การทำงานควรเพิ่มเป็น feature ใหม่ในอนาคต เช่น `pdf-inspector-adapter` โดยปิดไว้เป็นค่าเริ่มต้น. Adapter รับ bytes หรือ path ของ PDF แล้วคืนผลลัพธ์ที่แยกชัดระหว่างข้อมูลที่สกัดได้, สถานะของแต่ละหน้า และคำแนะนำเรื่อง OCR. ชั้น CLI เลือกว่าจะส่ง Markdown ที่ได้เข้า Markdown converter ของ `kept-doc` หรือส่งออกเป็น report ก่อนเท่านั้น

| ขั้น | ความรับผิดชอบ | Output ที่ควรมี |
|---:|---|---|
| 1 | เรียก `detect_pdf` หรือ `process_pdf_with_options` | PDF type, confidence, page count, `pages_needing_ocr` |
| 2 | ถ้าเป็น native text ให้เรียก `extract_pages_markdown` หรือ `process_pdf` | Markdown รายหน้า, `needs_ocr`, `is_complex` |
| 3 | ส่งเฉพาะ Markdown ของหน้าที่พร้อมเข้า `MarkdownConverter` | `UniversalDocument` ตาม API ของ kept-doc ปัจจุบัน |
| 4 | เก็บหน้าที่ `needs_ocr` ใน result/report โดยไม่สร้างข้อความแทน | queued pages และ reason ที่ตรวจสอบได้ |
| 5 | OCR เป็น workflow แยกและ opt-in | provenance ของผล OCR, engine/model revision และการยอมรับของผู้ใช้ |

ตัวอย่าง boundary ที่ควรรักษาไว้มีลักษณะดังนี้ โดยเป็น **pseudocode สำหรับออกแบบ** ไม่ใช่โค้ดที่เพิ่มใน Workspace:

```rust
let inspection = pdf_inspector::detect_pdf(path)?;
if inspection.pdf_type == PdfType::TextBased {
    let pages = pdf_inspector::extract_pages_markdown(path, None)?;
    // ส่งเฉพาะ page.markdown ที่ !page.needs_ocr เข้า kept_doc::converter::markdown
} else {
    // ส่ง report ว่าหน้าใดต้อง OCR; ไม่เปิด model หรือ network โดยอัตโนมัติ
}
```

## Data model ที่ควรเพิ่มเมื่อเริ่มทำจริง

Universal IR ปัจจุบันเก็บโครงสร้างเอกสารได้ แต่การรับ PDF ที่มี OCR แบบรายหน้าต้องมี provenance ที่ไม่หลุดระหว่าง conversion. ควรเพิ่ม metadata แบบ backward-compatible แทนการยัดค่าลงใน text block หรือ metadata ทั่วไปที่ตีความไม่ได้

| Field ที่เสนอ | ตัวอย่าง | เหตุผล |
|---|---|---|
| `source.format` | `pdf` | แยกที่มาจาก Markdown/Notion เดิม |
| `source.page_number` | `12` | trace กลับไปยังหน้าต้นฉบับได้ |
| `source.extraction_method` | `native_text`, `ocr`, `mixed` | ผู้ใช้รู้ว่าส่วนใดผ่าน OCR |
| `source.confidence` | `0.93` | ช่วยเลือกจุด review โดยไม่แอบแก้ผลลัพธ์ |
| `source.needs_ocr` | `true` | แสดงงานที่ยังค้างอย่างตรงไปตรงมา |
| `source.engine` | `pdf-inspector@<resolved-version>` | ทำผลลัพธ์ให้ reproduce/debug ได้ |

อย่าเพิ่งบันทึก pixel coordinates ทุกตัวอักษรใน Universal IR หลัก เพราะเพิ่มขนาดข้อมูลและยังไม่มีผู้ใช้ใน CLI ปัจจุบัน. หากต้องใช้ annotation/visual review ภายหลัง ให้เก็บ geometry เป็น sidecar artifact ที่อ้างด้วย document/page ID ก่อน

## OCR และ dependency policy

OCR ไม่ควรเป็น default behavior. เอกสารต้นทางระบุว่า OCR mode มี `Off`, `Auto`, `Force`; `model-cache` จัดการ manifest และ checksum, ส่วน `model-download` เป็น feature แยกที่ใช้ดาวน์โหลด artifact เมื่อ routing เลือกงาน OCR แล้ว [2]. หาก kept-doc เริ่มรองรับ OCR ควรบังคับให้ผู้ใช้เลือกอย่างใดอย่างหนึ่ง: path ของโมเดล offline ที่ตรวจ checksum แล้ว, หรือ policy ที่อนุญาต download อย่างชัดเจน. การใช้ renderer จาก `render-pdfium` ต้องจัดการ runtime PDFium เพิ่มเติมและควรแยกออกจาก converter ปกติ [2].

| Policy | ค่าเริ่มต้นที่เสนอ | เหตุผล |
|---|---|---|
| Cargo feature | ปิด `pdf-inspector-adapter`, `vision`, OCR renderer และ model download | build ปกติยังเบาและ offline ได้ |
| PDF ที่สแกน | คืน `needs_ocr` report | ไม่อ้างว่าแปลงเป็นข้อความได้แล้ว |
| Model source | local directory/checksum ก่อน | ลด dependency ต่อ network และทำงานซ้ำได้ |
| Network download | ต้องเลือก explicit flag | ผู้ใช้รู้ว่ามี artifact ใหม่เข้ามา |
| Error handling | คืน diagnostics รายหน้า | หลีกเลี่ยงการทิ้งทั้งเอกสารเมื่อเสียเพียงบางหน้า |

## ความเสี่ยงและเกณฑ์ยอมรับก่อน merge

ผล benchmark ใน repository ต้นทางเป็นข้อมูลของ corpus และเครื่องของผู้พัฒนา ไม่ใช่ SLA ของ `kept-doc`. เอกสารต้นทางระบุ benchmark 200 PDFs บน Apple M4 Pro และ OCR ปิดอยู่; จึงใช้เปรียบเทียบแนวทางได้ แต่ไม่ควรคัดลอกตัวเลขมาเป็นคำรับประกันของ CLI นี้ [1] [2].

| ความเสี่ยง | วิธีลดความเสี่ยง | เกณฑ์ยอมรับ |
|---|---|---|
| Markdown ที่ได้ไม่ map เป็น Universal IR ครบ | ทำ golden fixtures สำหรับ heading, list, table, code และ page boundary | round-trip test ของทุก fixture ผ่าน |
| ลำดับอ่านของ multi-column หรือ table ผิด | เก็บ PDF fixture ที่อนุญาตเผยแพร่และเทียบผล page-by-page | ไม่ regression จาก baseline ที่ทีมกำหนด |
| OCR เพิ่ม runtime/dependency โดยไม่ตั้งใจ | feature gates และ test matrix default vs OCR | default build ไม่มี model/network dependency |
| ผลลัพธ์ mixed PDF ทำให้ผู้ใช้เข้าใจผิด | แสดง page status และ provenance ใน output | ไม่มีหน้าที่ OCR-required ถูกระบุว่า extracted สำเร็จ |
| PDF อันตรายหรือผิดรูป | size/page limits, error boundary และ fuzz/regression fixtures | malformed input ไม่ทำให้ CLI panic |

## ลำดับงานที่ควรทำภายหลัง

เริ่มจาก `pdf-inspector-adapter` ที่แค่ inspect และ export report/Markdown ของ native text. ขั้นที่สองจึง map Markdown ไป Universal IR พร้อม source metadata. ขั้นที่สามค่อยพิจารณา OCR routing แบบ local-only. Integration กับ Notion live sync ไม่ควรถูกผูกเข้ากับงาน PDF เพราะ live sync ยังไม่พร้อมสำหรับ production ตามสถานะ Workspace ปัจจุบัน.

## References

[1]: https://github.com/firecrawl/pdf-inspector "Firecrawl pdf-inspector README"
[2]: https://github.com/firecrawl/pdf-inspector/blob/main/docs/rust-api.md "Firecrawl pdf-inspector Rust API"
[3]: https://context7.com/firecrawl/pdf-inspector "Context7: pdf-inspector query on process, pages, and OCR routing"
[4]: https://context7.com/firecrawl/pdf-inspector "Context7: pdf-inspector query on text positions, tables, and Markdown options"
