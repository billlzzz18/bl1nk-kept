# บันทึกการตัดสินใจ 0002: ตัวแปลงรับเข้า PDF แบบเลือกเปิด

**สถานะ:** ตัดสินใจแล้ว  
**วันที่:** 2026-08-20

## บริบท

`kept-doc` มี Universal IR และ Markdown converter อยู่แล้ว งานรับ PDF ต้องเพิ่มการรับรู้แหล่งที่มาโดยไม่ทำให้การแปลงเอกสารปกติกลายเป็นระบบ OCR, model downloader หรือ workflow ที่พึ่งพา network

## การตัดสินใจ

เมื่อเริ่มรองรับ PDF จะใช้ `pdf-inspector` เป็นตัวแปลงรับเข้าแบบ optional ไม่ใช่ตัวแทนของ Universal IR หรือ Markdown converter

| ขอบเขต | การตัดสินใจ |
|---|---|
| build ปกติ | ปิด PDF adapter, vision support, renderer และ model download เป็นค่าเริ่มต้น |
| เส้นทางแรก | ตรวจ PDF และสกัด Markdown จาก native text พร้อมสถานะรายหน้า |
| Universal IR | ส่งเฉพาะหน้าที่ได้ Markdown ใช้งานได้เข้า Markdown converter ที่มีอยู่ |
| หน้าที่ต้อง OCR | คืน diagnostic `needs_ocr` รายหน้า; ห้ามสร้างข้อความเองหรืออ้างว่าแปลงสำเร็จ |
| OCR | local/offline และ explicit เท่านั้น; model path, checksum หรือ download policy ต้องเป็น workflow ที่ผู้ใช้เลือก |
| provenance | เก็บ source format, page number, extraction method, confidence, OCR requirement และ engine revision เมื่อเพิ่ม adapter |
| geometry | เก็บ geometry เป็น sidecar เมื่อ visual review ต้องใช้; ไม่เพิ่มขนาด Universal IR หลักล่วงหน้า |

## ผลกระทบ

งานชิ้นแรกของ adapter คือ inspect, report และ native-text export โดยไม่ผูกกับ Notion sync เกณฑ์ยอมรับต้องมี PDF fixture ที่เผยแพร่ได้, diagnostic รายหน้า, default build ที่ไม่มี model/network dependency, ความปลอดภัยกับ malformed PDF และห้ามรายงานหน้าที่ต้อง OCR ว่าสกัดสำเร็จ

## เอกสารอ้างอิง

- [งานวิจัย PDF inspector](../../research/PDF_INSPECTOR_RESEARCH.md)
- [รายการงาน kept-doc](../../TODO.md)
