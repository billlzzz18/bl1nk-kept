# เริ่มใช้ `kept`

หลังติดตั้ง binary `kept` ให้สร้าง user configuration หนึ่งครั้ง, กำหนด scope ที่เป็น absolute path ด้วยคำสั่ง task-level แล้วจึง scan directory นั้น. `setup` สร้าง `config.yaml` ที่ตำแหน่งของระบบปฏิบัติการเพียงครั้งเดียวและไม่เขียนทับไฟล์เดิมของผู้ใช้.

## ตั้งค่า kept

`kept config` เป็นหน้าสรุป config, profiles, scopes และคำสั่งถัดไป. `kept config edit` เปิด `nano` ตามค่าเริ่มต้นและเป็นทางเลือกสำหรับแก้ YAML ขั้นสูงเท่านั้น; กำหนด `KEPT_EDITOR` เป็น path ของ editor executable เพื่อใช้ editor อื่น.

`config fields` แสดง key, type, ค่าที่รองรับ และ default value ที่ใช้อยู่จริง. ห้ามเดาชื่อ field หรือค่าที่รองรับจาก YAML.

### Profiles

Profile รวม naming rules ที่นำกลับมาใช้กับหลายโฟลเดอร์. Profile ใหม่เริ่มจาก defaults; `unset` ทำให้ field นั้นกลับไปใช้ defaults.

ใช้ `kept config fields` เป็นรายการ key ที่ตั้งได้ทั้งหมด รวม boolean, numbers, words, length, whitespace, prefix, aliases, shortcuts, variables, similarity, replacements, reposition และ `stemRegex`. ค่า `replacements` และ `reposition` รับ YAML list เป็น argument เดียว; ใช้ `config edit` เฉพาะเมื่อชุด rule ยาวจนอ่านใน command line ไม่สะดวก.

> [!NOTE]
> `shortcuts.<token>` เป็น keymap สำหรับขยาย token ในชื่อไฟล์ เช่น `rpt -> report` ไม่ใช่ keyboard shortcut.

### Scopes และลำดับ rule

Scope ต้องเป็น **absolute path ที่ผู้ใช้เลือกเอง**. kept ไม่เดา scope จาก directory ที่ scan. การ resolve เลือก path ที่ลึกกว่าเป็นอันดับแรก, แล้วจึงเลือก `priority` ที่สูงกว่า; `scope list` แสดงลำดับนี้ให้ตรวจได้.

```bash
kept config scope add invoices "$HOME/work/invoices" --priority 200
kept config scope list
kept config scope show "$HOME/work/invoices"
kept config scope set "$HOME/work/invoices" --priority 300 --recursive true
kept config scope exception add "$HOME/work/invoices" "$HOME/work/invoices/generated"
kept config scope setting set "$HOME/work/invoices" separator snake
kept config scope setting unset "$HOME/work/invoices" separator
kept config scope exception remove "$HOME/work/invoices" "$HOME/work/invoices/generated"
kept config scope remove "$HOME/work/invoices"
```

Scope setting เป็น override เฉพาะโฟลเดอร์ จึงเหมาะกับความต่างเล็กน้อยโดยไม่ต้องสร้าง profile ใหม่. `review` ใช้ ScanIndex และ config เดิมเพื่อรายงาน naming violation, proposed target และ collision แบบ read-only; รุ่นนี้ไม่มี rename, apply หรือ rollback.

### Doctor

```bash
kept doctor
kept doctor --fix
```

`doctor` รายงาน path, รหัสปัญหา และวิธีแก้เมื่อ config YAML, scope, naming value หรือ editor ใช้ไม่ได้. `doctor --fix` สร้าง config ที่หาย หรือย้าย config ที่เสียไปเป็นไฟล์ `.invalid*.bak` ก่อนสร้าง starter config ใหม่. คำสั่งนี้ไม่เขียนทับ config ที่ parse และ validate ได้.

## Scan และ index

| งาน | คำสั่ง |
|---|---|
| Scan หรือ refresh index ปกติ | `kept scan "$HOME/work"` |
| รวมไฟล์ซ่อน | `kept scan "$HOME/work" --include-hidden` |
| สร้าง snapshot ที่พกพาได้ | `kept scan "$HOME/work" --output ~/work-scan.json` |
| ใช้ใน script | `kept scan "$HOME/work" --json` |

Index ปกติอยู่ใน user state ของ `kept` ไม่ถูกเขียนลงใน directory ที่ scan. เมื่อ scan path เดิมซ้ำ `kept` รายงานไฟล์ที่เพิ่ม เปลี่ยน ลบ และคงเดิมจาก index ก่อนหน้า.

## กลับมา review

```bash
kept review "$HOME/work"
kept review "$HOME/work" --index ~/work-scan.json
```

## หาไฟล์จาก index

`find` อ่าน index ที่มีอยู่แล้ว ไม่ scan directory ใหม่. ใส่เฉพาะ facts ที่ต้องการ; หากรันจาก review menu โปรแกรมจะถามค่าเหล่านี้ทีละข้อและกด Enter เพื่อข้ามได้.

```bash
kept find "$HOME/work" --type pdf --name report
kept find "$HOME/work" --path archive --min-size 50mb --max-size 500mb
kept find "$HOME/work" --after 1704067200 --before 1735689600 --json
kept find "$HOME/work" --type pdf --index ~/work-scan.json
```

| Fact | ความหมาย |
|---|---|
| `--type pdf` | นามสกุลไฟล์ โดยไม่ต้องใส่จุดนำหน้า |
| `--name report` | ชื่อไฟล์มีข้อความนี้ |
| `--path archive` | path มีข้อความนี้ |
| `--min-size 50mb` / `--max-size 500mb` | ช่วงขนาด; ใช้ `b`, `kb`, `mb`, `gb` ได้ |
| `--after` / `--before` | เวลาแก้ไขไฟล์ใน Unix time |
| `--index path.json` | ใช้ snapshot ที่ผู้ใช้ระบุ |
| `--json` | คืนรายการที่กรองแล้วเป็น JSON สำหรับ script |

## ตรวจ duplicate

```bash
kept duplicates "$HOME/work"
kept duplicates "$HOME/work" --index ~/work-scan.json --json
kept duplicates "$HOME/work" --action export-plan --yes
```

คำสั่งนี้ใช้ file list จาก index แล้วคัด candidate ด้วยขนาด ก่อนทำ partial SHA-256 และ full SHA-256 เฉพาะ candidate ที่เหลือ. ผลลัพธ์แยก `same_content` และ `hard_link` พร้อม hash evidence; ไม่ลบหรือแก้ไฟล์เอง. `--action export-plan --yes` สร้าง `kept-duplicate-plan.json` ใน root ที่ตรวจเท่านั้น.

## จัดการ registry groups

`search --group` จำกัดผลค้นหาตาม group ใน keyword registry. ใช้ `group` เพื่อสร้าง ดู แก้ ลบ เรียง และกำหนด field schema ของ group; ทุกคำสั่งเขียนเฉพาะ registry path ที่ระบุ.

```bash
kept group types
kept group list registry.json
kept group add registry.json finance_terms --name "Finance terms" --description "คำศัพท์การเงิน"
kept group set registry.json finance_terms --name "Finance vocabulary"
kept group field add registry.json finance_terms currency enum --values THB,USD --required
kept group field list registry.json finance_terms
kept group field remove registry.json finance_terms currency
kept group move registry.json finance_terms --before product_terms
kept group show registry.json finance_terms
kept group remove registry.json finance_terms
```

`kept group types` แสดง field types ที่ validator รองรับ: `string`, `enum`, `array`, `object`, `number` และ `boolean`. Array ต้องกำหนด `--item-type string|number|boolean`; enum ต้องกำหนด `--values`. kept ป้องกันการลบ base search field `id` และ `aliases`.

## ค้นหา keyword

`search` อ่าน registry JSON หรือ YAML ที่ผ่าน validation แล้ว. ใช้ `--group` เมื่อต้องจำกัดกลุ่ม และ `--json` เมื่อส่งต่อผลให้ script.

```bash
kept search registry.json รายงาน --group product_terms
kept search registry.json รายงาน --json
```

## Evidence และ gold corpus แบบ offline

คำสั่งกลุ่มนี้เก็บหลักฐานและ dictionary แบบตรวจสอบย้อนกลับได้ โดยไม่มีคำสั่ง promote หรือ mutation อัตโนมัติ.

```bash
kept evidence run --manifest run-manifest.json --raw-jsonl raw.jsonl --input source.md
kept evidence rescore --offline --manifest run-manifest.json --raw-jsonl raw.jsonl --output rescore.jsonl
kept evidence correct append --history corrections.jsonl --raw-jsonl raw.jsonl --subject-id a --decision eligible --reason reviewed
kept evidence self-test --good-fixture good.jsonl --bad-fixture bad.jsonl --require-no-mutation
kept corpus validate reviewed.jsonl --report json
kept corpus snapshot save reviewed.jsonl gold.snapshot.json
kept corpus replay gold.snapshot.json --json
```

`corpus replay` ใส่เฉพาะ assertion ที่ review เป็น `accepted` ลงใน `dictionary`; candidate ที่ `rejected` หรือยังไม่ผ่าน review จะไม่ถูก promote.

## แปลงเอกสาร

`convert` แปลง Markdown เป็น Universal IR JSON เมื่อ output ลงท้ายด้วย `.json`. ผลลัพธ์เป็นไฟล์ใหม่ตาม path ที่ระบุ.

```bash
kept convert source.md result.json
```

## รันโปรแกรมใน virtual terminal

`terminal run` รันโปรแกรมแล้วป้อน stdout/stderr เข้า virtual terminal grid ก่อนพิมพ์ผล เพื่อให้ output ที่มีการเขียนทับด้วย `\r`, การเลื่อน cursor หรือ alternate screen ถูก collapse เหลือหน้าจอสุดท้ายที่อ่านได้ทันที. คำสั่งนี้ **ไม่แก้ไขไฟล์** และสะท้อน exit code ของโปรแกรมลูกกลับเป็น exit code ของ `kept` (หมดเวลา = `124`).

```bash
kept terminal run cargo test --workspace
kept terminal run --columns 120 --lines 40 npm test
kept terminal run --timeout-ms 5000 --json ping -n 20 127.0.0.1
kept terminal render capture.bin --json
```

`terminal render` ใช้เมื่อมี byte stream ที่บันทึกไว้แล้ว (เช่นผลจาก `script`): ไม่ spawn process และให้ผลแบบ deterministic เหมาะกับ regression test.

## ดู option ที่ใช้ได้

```bash
kept --help
kept setup --help
kept config --help
kept config defaults --help
kept config profile --help
kept config scope --help
kept config fields --help
kept config edit --help
kept doctor --help
kept scan --help
kept find --help
kept review --help
kept duplicates --help
kept group --help
kept search --help
kept convert --help
kept evidence --help
kept corpus --help
kept terminal --help
kept terminal run --help
kept terminal render --help
```
