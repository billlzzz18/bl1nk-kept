# kept-cli

Command-line Interface (`kept`) สำหรับจัดการระบบไฟล์ เอกสาร Registry และ Context

---

## ขอบเขตคำสั่งหลัก (Command Surface)
- `kept look / view`: ตรวจสอบและดึง context แบบประหยัด token
- `kept search`: ค้นหาข้อมูลแบบ Hybrid (Grep, BM25, Vector)
- `kept scan / dup`: ตรวจจับไฟล์ซ้ำและวิเคราะห์ filesystem
- `kept doc`: แปลงและตรวจสอบเอกสารผ่าน Universal IR
