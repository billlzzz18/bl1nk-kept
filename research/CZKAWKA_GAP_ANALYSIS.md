# Czkawka gap analysis สำหรับ filesystem workflow

## ขอบเขตการเปรียบเทียบ

เอกสารนี้บันทึกข้อเท็จจริงจาก source ของ Czkawka/Krokiet เพื่อใช้จัดลำดับงานของ `kept` ไม่ใช่ product specification และไม่ใช่แผนงานแยก. งานที่ยอมรับให้ทำจริงอยู่ใน `TODO.md`; public contract อยู่ใน `SPEC.md`.

## สิ่งที่ Czkawka ทำจริง

README ของ repository ระบุเครื่องมือสำหรับ duplicate, big files, empty files/folders, temporary files, invalid symbolic links, broken files, bad extensions และ bad names. Duplicate รองรับสัญญาณชื่อ ขนาด หรือ hash; repository ระบุ cache support เพื่อให้การ scan ครั้งต่อไปรวดเร็วขึ้น. CLI มี subcommands แยกตามเครื่องมือและ options สำหรับ include/exclude path, extension, size, method และผลลัพธ์ไฟล์. [1] [2]

Czkawka ยังมี feature ต่อไปนี้ซึ่ง `kept` ไม่มีในปัจจุบัน: ตรวจ symlink เสีย, ตรวจไฟล์เสียหรือ extension ไม่ตรง content, ตรวจชื่อที่ไม่ต้องการ, empty/temporary files/folders และ media similarity. สิ่งเหล่านี้เป็น scanner เฉพาะชนิด ไม่ใช่ความสามารถของ exact content duplicate โดยอัตโนมัติ. [1]

## Rename เป็น capability แยกจาก duplicate scan

Issue rename ของ Czkawka แสดงว่าการ rename เคยเป็น feature request และ maintainerแยกมันจาก duplicate finding เพราะ rename ไม่เปลี่ยนสถานะ duplicate โดยตัวมันเอง; source อ้างว่า feature มีใน master ในเดือนเมษายน 2026. Discussion เก่ายืนยัน use case ที่ต้องย้ายหรือแทนไฟล์คุณภาพดีกว่า แต่ติด naming collision. ข้อเท็จจริงนี้สนับสนุนให้ `kept` แยก naming policy/rename plan ออกจาก duplicate detector และบังคับ precondition/collision policy ที่อธิบายได้. [3] [4]

## Gap ของ kept ณ 2026-08-20

| เรื่อง | kept ปัจจุบัน | สิ่งที่ยังขาด |
|---|---|---|
| Exact content duplicate | size → partial SHA-256 → full SHA-256, same_content/hard_link evidence | intentional duplicate allow-list, keep/rename decision policy, apply/rollback |
| Near-name / keyword-like duplicate | normalized similarity และ registry policy สำหรับ keyword search | unified review policy ข้าม filename/keyword/content และ user-level thresholds สำหรับ filesystem CLI |
| Persistent scan | snapshot, refresh delta, `ScanIssue`, task-first review | true incremental traversal/hash optimization, stale-index policy |
| Unreadable path | scan เก็บ issue และเดินต่อ | error-kind taxonomy, permission remediation advice, retry/skip policy ที่ผู้ใช้กำหนดได้ |
| Naming policy | ไม่มี | scope ต่อ folder/extension, name grammar, case/separator/Unicode rules, collision detection, exceptions |
| Rename | ไม่มี | deterministic plan, owner confirmation, precondition fingerprint, allow-list, apply log และ rollback |
| File-type integrity | ไม่มี | bad extension/content signature adapter และ broken-file inspectors |
| Other Czkawka tools | ไม่มี | symlink, empty, temp, media similarity; ไม่อยู่ใน P0 ของ rename/duplicate workflow |

## ข้อสรุปเชิงออกแบบ

การทำให้ "ดีกว่า Czkawka" สำหรับ kept ไม่ควรหมายถึงคัดลอกจำนวน scanner ให้มากกว่า. จุดต่างที่เหมาะกับ kept คือใช้ scan index เดียว, registry policy ที่ผู้ใช้กำหนดเอง, evidence ที่ย้อนกลับตรวจได้, naming policy ที่ scoped และ rename workflow ที่แยก plan/review/apply/rollback. ก่อน mutation ต้องมี read-only analysis และ plan ที่ deterministic; ไม่มี auto-rename จาก fuzzy candidate.

## References

[1] [Czkawka repository README](https://github.com/qarmin/czkawka)

[2] [Czkawka CLI README](https://raw.githubusercontent.com/qarmin/czkawka/master/czkawka_cli/README.md)

[3] [Czkawka issue #1594: Add renaming feature](https://github.com/qarmin/czkawka/issues/1594)

[4] [Czkawka discussion #1129: Replace files with duplicate found + rename files](https://github.com/qarmin/czkawka/discussions/1129)

## Policy store ตาม OS

Policy ไม่ควรอยู่ใน directory ที่ scan และไม่ควรบังคับส่ง path config ทุกครั้ง. ผู้ใช้ต้องระบุ config file แบบ absolute ผ่าน `--policy <absolute-path>` ได้เสมอ; หากไม่ระบุ `kept` ต้องค้นหา store ของ OS ตามลำดับที่กำหนดและรายงาน path ที่อ่านจริงใน output/snapshot.

| OS | Configuration ที่ค้นหา/เขียน | Persistent state ที่ใช้กับ snapshot, plan, log |
|---|---|---|
| Linux และ Unix ที่รองรับ XDG | `$XDG_CONFIG_HOME/kept/policies.json`, fallback `~/.config/kept/policies.json` | `$XDG_STATE_HOME/kept/`, fallback `~/.local/state/kept/` |
| macOS | `~/Library/Application Support/kept/policies.json` | `~/Library/Application Support/kept/state/` |
| Windows | `%APPDATA%\\kept\\policies.json` | `%LOCALAPPDATA%\\kept\\state\\` |

XDG ระบุชัดว่า configuration และ persistent state เป็น base directory แยกกัน, environment paths ต้องเป็น absolute และ fallback ของ config/state คือ `~/.config`/`~/.local/state`. Apple ระบุให้เก็บ app-managed support/config resources ใน `Library/Application Support` และไม่เขียนสิ่งที่แอปจัดการเองลง Documents/Desktop/สื่อของผู้ใช้. Windows Known Folders ระบุ `%APPDATA%` และ `%LOCALAPPDATA%` เป็น per-user standard locations; implementation ต้องอ่าน environment/known folder แทนการเขียน hard-code ไปยัง Documents. [5] [6] [7]

Policy file จะมี scope `folder` แบบ absolute เท่านั้น. Rule จับ file เมื่อ absolute path อยู่ใต้ folder scope (เลือก `recursive` ได้) และ extension ตรง; ไม่มี relative glob ที่เปลี่ยนความหมายตาม root ของคำสั่ง. Snapshot บันทึก canonical policy path, SHA-256 ของ policy content และ rule IDs ที่ใช้งาน เพื่อให้ review/rename plan ตรวจย้อน context เดิมได้.

[5] [XDG Base Directory Specification 0.8](https://specifications.freedesktop.org/basedir/)

[6] [Apple File System Programming Guide: Application Support](https://developer.apple.com/library/archive/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html)

[7] [Microsoft KNOWNFOLDERID: LocalAppData and user folders](https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid)
