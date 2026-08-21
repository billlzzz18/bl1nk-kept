# Ponytail: บทเรียนด้าน testing, benchmark และ YAGNI สำหรับ bl1nk-kept

**ผู้จัดทำ:** Manus AI  
**วันที่วิจัย:** 2026-08-19  
**ขอบเขต:** วิเคราะห์ Ponytail เพื่อปรับปรุงการทดสอบ, benchmark, การนำเสนอผล และการตัดสินใจตาม YAGNI ของ bl1nk-kept โดยไม่เพิ่ม embedding, model judge, service หรือ abstraction ที่ยังไม่มีข้อมูลรองรับ

## ข้อสรุป

Ponytail มีบทเรียนที่นำมาใช้กับ bl1nk-kept ได้จริงสองส่วน. ส่วนแรกคือการทำให้ benchmark ตรวจสอบย้อนกลับได้: ใช้ baseline ที่ยุติธรรม, ทำ self-test ให้เครื่องมือวัดก่อนเก็บผลจริง, เก็บ raw artifacts, แยก correctness/safety ออกจาก metric ประสิทธิภาพ และบอกข้อจำกัดของผล. ส่วนที่สองคือ YAGNI แบบมีเงื่อนไข: ลดสิ่งที่ไม่จำเป็นหลังอ่าน flow จริงแล้ว แต่ไม่ลด validation, data-loss guard, security, accessibility หรือ test ที่ปกป้อง logic สำคัญ.[1] [2] [3]

สิ่งที่ไม่ควรคัดลอกคือ agentic benchmark ที่ต้องเรียก LLM judge, multiple model arms, หรือ harness แยก repo ต่อ cell. Ponytail ต้องใช้สิ่งเหล่านั้นเพื่อตอบคำถามว่า skill เปลี่ยนพฤติกรรม agent ได้หรือไม่; bl1nk-kept กำลังทดสอบ Rust library/CLI ที่ deterministic เป็นหลัก. การเพิ่ม LLM evaluator ตอนนี้จะเพิ่ม cost, variance และ maintenance โดยยังไม่ตอบ customer problem ใหม่.

> ข้อเสนอหลักคือสร้าง **deterministic benchmark contract** สำหรับ bl1nk-kept ก่อน: fixture manifest, self-test, repetitions, baseline, raw JSONL, offline summary และ evidence page. ใช้ข้อมูลจริง/เปิดเผย source เท่าที่มีอยู่แล้ว แล้วขยาย strata เฉพาะเมื่อมี failure หรือ anonymized corpus ใหม่.

## วิธีวิจัยและความน่าเชื่อถือของหลักฐาน

งานนี้ใช้ Context7 กับ library ID `/dietrichgebert/ponytail` เพื่ออ่าน source-level documentation และ command examples ของ repository. Query ครอบคลุม test runner, agentic benchmark, repeat count, self-test, offline rescore, scoreboard และ YAGNI rule. ใช้ Exa เพื่อค้นหาและ fetch บทวิเคราะห์ภายนอกของ Colin Eberhardt ซึ่งเป็นต้นทางของคำวิจารณ์ต่อ benchmark รุ่นแรก. จากนั้นอ่าน source files ของ Ponytail โดยตรงและเทียบกับ benchmark/test artifacts ใน bl1nk-kept.

| แหล่ง | ใช้ตอบคำถาม | น้ำหนักในการสรุป |
|---|---|---:|
| Ponytail `benchmarks/agentic/README.md` | harness, tasks, gates, isolation, reproduce flow | สูง |
| Ponytail agentic result report | การแก้ baseline contamination, metrics, limitations | สูง |
| Ponytail rule file | YAGNI ladder และข้อยกเว้นด้านความปลอดภัย | สูง |
| Scott Logic ผ่าน Exa | ข้อบกพร่องของ prompt benchmark เดิมและเหตุผลที่ต้องแก้ | กลาง, ใช้ cross-check |
| Martin Fowler | ขอบเขต YAGNI และเหตุผลที่ self-testing/refactoring ไม่ใช่ over-engineering | สูงสำหรับหลักการ |
| Source ของ bl1nk-kept | สิ่งที่มีอยู่, ช่องว่าง และข้อเสนอที่ทำได้จริง | สูง |

## Ponytail ทำอะไรจริง

### การทดสอบและ benchmark

Ponytail เปลี่ยนจาก benchmark แบบ one prompt/one completion ไปเป็น headless coding-agent session ที่แก้ seeded workspace ของ repository จริง. แต่ละ cell แยก workspace และ agent context; run ล่าสุดใช้ arms หลายแบบ เช่น baseline, ponytail, caveman และ `yagni-oneliner`, พร้อม `n=4` ต่อ task/arm.[1] [2]

การออกแบบที่น่าสนใจคือแยกงานเป็นสอง tier. LOC tier วัด source `git diff` และ source-file count ใน feature task ที่เปิดช่องให้ over-build. Safety tier รันโค้ดที่ agent สร้างด้วย adversarial input แบบ deterministic เช่น path traversal, SQL injection, malformed CSV และ forged token. Test ที่ agent เขียนไม่นับเป็น bloat แต่เก็บเป็น metric แยก (`wrote_tests_rate`).[1]

ก่อน run ที่มี API, Ponytail รัน `python run.py --selftest` เพื่อยืนยันว่า reference ที่ดีผ่านและ reference ที่ไม่ปลอดภัยถูกจับได้. หลัง run สามารถ `--rescore` จาก workspace ที่เก็บไว้โดยไม่เรียก API ซ้ำ. จุดนี้มีค่ามากกว่าตัวเลข LOC: มันป้องกันเครื่องมือวัดที่เสียจากการสร้างผลลัพธ์ที่ดูดีแต่ผิด.[1]

Ponytail ยังใช้ two audit judges สำหรับ over-engineering และ completeness. แต่ทั้งคู่มี fixed model, temperature 0, published rubric และ self-test ที่ต้องจัดอันดับ reference pair ถูกก่อนใช้กับผลจริง. ผล LOC จึงไม่ถูกตีความเป็นชัยชนะหาก implementation เป็น stub หรือ completeness ลดลง.[1]

### การนำเสนอ benchmark

README ของ Ponytail แสดง summary table ที่เทียบกับ baseline ใน LOC, token, cost, time และ safety. จากนั้นลิงก์ไปยังรายงานเต็มที่มี task-level table, safety table, raw-run method, contamination correction และ limitations. Repository ระบุด้วยว่า single-shot score รุ่นแรกถูก supersede เพราะ baseline นับ prose/options ของ conversational answer และทำให้ baseline ไม่ยุติธรรม.[2]

ข้อดีเชิงวิธีวิทยาคือมันเปิดเผยความผิดพลาดของตัวเอง: baseline เคยถูก SessionStart hook ปนเปื้อนเพราะ hook ทำงานทุก arm และจึงต้องแก้ isolation. รายงานยังเปิดเผย one-model scope, deterministic checks เป็นเพียง safety floor, variance และ Windows timeout. นี่เป็นรูปแบบที่ bl1nk-kept ควรนำมาใช้: เก็บ correction history ไว้กับผล ไม่ลบผลเก่าแล้วเหลือแต่เลขที่สวย.[1] [2]

### YAGNI ที่ Ponytail ใช้

Ponytail เริ่มด้วย ladder: จำเป็นจริงหรือไม่, มีใน codebase แล้วหรือไม่, stdlib ทำได้หรือไม่, platform ทำได้หรือไม่, dependency ที่ติดตั้งแล้วทำได้หรือไม่, ทำเป็นหนึ่งบรรทัดได้หรือไม่ และค่อยเขียน minimum code. Ladder ใช้หลัง trace task/flow แล้ว, ไม่ใช่ทางลัดเพื่อเดา solution.[3]

กฎที่ควรนำมาใช้ตรงตัวคือ root-cause fix แทน symptom patch; no abstraction/dependency/boilerplate ที่ยังไม่มี requirement; deletion ก่อน addition; และ comment ให้ simplification ที่มีเพดานรู้ชัดพร้อม upgrade path. แต่ Ponytail ไม่ถือว่า validation ที่ trust boundary, error handling ป้องกัน data loss, security, accessibility หรือ runnable check สำหรับ logic ที่ไม่ trivial เป็นส่วนที่ตัดได้.[3]

Fowler เพิ่มขอบเขตสำคัญ: YAGNI ห้ามสร้าง capability หรือ abstraction เพื่อ future feature ที่ยังไม่จำเป็น แต่ไม่ห้าม refactoring, self-testing code หรือ continuous delivery เพราะสิ่งเหล่านี้ทำให้ codebase เปลี่ยนได้เมื่อ requirement เกิดจริง. Abstraction ที่ทำให้ requirement ปัจจุบันอ่านยากควรถูกถือว่าไม่จำเป็นก่อนจนกว่าจะมี use case จริง.[5]

## เทียบกับ bl1nk-kept ปัจจุบัน

| ประเด็น | Ponytail | bl1nk-kept ปัจจุบัน | การตีความ |
|---|---|---|---|
| Unit/regression tests | `npm test`; rule-copy consistency check | `cargo test --workspace`, 82 tests; TDD regressions ครอบคลุม migration, corpus integrity, duplicate, converter และ CLI flags | แข็งแรงใน deterministic correctness แล้ว |
| Safety/self-test | good/bad reference ก่อน API | unit tests มี negative cases เช่น invalid UTF-8, invalid regex, PII masking, save boundary | ควรรวมให้เป็น command เดียวและนิยามว่าเป็น instrument self-test |
| Performance benchmark | real agent workspace, repetitions, raw workspaces | `examples/benchmark.rs` เป็น synthetic deterministic, default 10K/100K, query average เดียวต่อ scale | เหมาะกับ throughput smoke แต่ยังไม่ใช่ release evidence ที่มี distribution |
| Parameter experiment | arms, controls, `n=4`, rescore | `foundation_experiment.rs` มี 6 candidates, 7 repetitions, public CC0 Thai corpus, 2 strata, raw JSONL และ summary | เป็นฐานที่ดี; ต้องเพิ่ม replay/check และ manifest ของ command/config |
| Correctness vs size | safety/completeness gates แยกจาก LOC | duplicate experiment วัด F1, false positives, P95 latency, candidate count | ดีสำหรับ duplicate defaults; ส่วน search/index/migration ยังไม่มี release matrix เดียวกัน |
| Presentation | compact scoreboard + report + task tables + limitations | snapshot HTML/PNG, JSONL, release report และ benchmark chart | มีหลักฐาน แต่ index ยังไม่ได้บอก baseline/config/command ในรูปแบบมาตรฐานเดียว |
| CI | repository มี development checks | ไม่มี `.github` directory ใน source tree ที่ตรวจ | ไม่ควรแปลว่าต้องตั้ง CI server ทันที; เริ่มจาก reproducible local command contract ก่อน |

## ช่องว่างที่สำคัญที่สุด

### 1. Benchmark หลักยังไม่รายงาน distribution

`crate/kept-core/examples/benchmark.rs` ใช้ synthetic deterministic input 10K และ 100K โดย default, query count 100 และรายงาน average latency เดียว. มันมีประโยชน์ในการจับ gross regression แต่ไม่มี warm-up policy, repetitions, median/P95, variance, baseline ID, fixture revision หรือ raw per-run record. จึงไม่ควรใช้ประกาศว่า configuration ใด “ดีที่สุด”.

Foundation duplicate experiment แก้ปัญหานี้ได้บางส่วนแล้ว: มี 7 repetitions, exact/near-name strata, raw JSONL, summary, F1, false-positive maximum, P95 และ selection constraints. อย่างไรก็ดี, harness ยัง derive filenames จาก word list และวัด duplicate engine เดียว. ต้องระบุขอบเขตนี้ตลอดเวลา ไม่ขยายผลไปยัง content scan, Thai semantics, filesystem distribution หรือ document conversion.

### 2. ไม่มี instrument self-test entry point รวม

ชุด test ปัจจุบันมี good/negative cases อยู่แล้ว แต่ผู้รันไม่มีคำสั่งเดียวที่แยกว่า “เครื่องมือวัดและ fixture พร้อมก่อน benchmark” จาก “benchmark ผ่าน”. Ponytail ใช้ self-test เพื่อป้องกัน bad probe, invalid judge หรือ corrupted instrument. สำหรับ bl1nk-kept ควรเป็น deterministic test ที่ไม่ต้อง network/API และใช้ fixture ใน repository.

### 3. Evidence มีหลายที่แต่ metadata ยังไม่ uniform

ปัจจุบันมี `benchmarks/data/search_duplicate_release.jsonl`, Foundation experiment raw/summary, generated chart, corpus manifest และ `SPEC.md`. สิ่งที่ยังขาดคือ run manifest กลางที่บอก crate version, git revision ถ้ามี, OS/CPU, Rust version, command, fixture checksum, options, warm-up/repetitions และ status ของ each gate. หากไม่มี metadata นี้ การเทียบผลระหว่าง release จะเสี่ยงตีความ environment change เป็น product change.

## ข้อเสนอแบบ YAGNI-first

### Phase A: ทำให้สิ่งที่มีอยู่ตรวจซ้ำได้ก่อน

เพิ่ม `cargo run -p kept-core --example benchmark_selftest --release` หรือรวมเป็น option `--selftest` ใน benchmark examples. คำสั่งต้องตรวจเฉพาะสามเรื่อง: corpus manifest checksum ตรง, fixture expected exact/near/hard-link/negative behavior ตรง, และ statistics summary/selection ไม่รับ bad reference. ไม่มี dependency ใหม่, ไม่มี LLM, ไม่มี database และไม่ต้องสร้าง benchmark framework ใหม่.

พร้อมกันนั้นเพิ่ม `--runs N`, `--warmup N`, `--output-dir DIR` ให้ `benchmark.rs`. Output เป็น raw JSONL หนึ่ง record ต่อ run และ `summary.json` ที่มี median, P95, min/max และ variance. ให้ benchmark default ยังเป็น 10K/100K เพื่อคุมเวลารัน; 1M เป็น explicit scale ที่ผู้ปฏิบัติเลือก. Baseline ต้องเป็น config ID ที่ commit ใน command/manifest ไม่ใช่ result file จากรอบก่อน.

### Phase B: สร้าง release evaluation matrix ที่ตอบ customer risk

สร้าง `benchmarks/data/<run-id>/manifest.json` ต่อ run โดยอ้าง corpus revision, command, build profile, `rustc --version`, options และ run timestamp. Matrix ไม่ต้องเป็น dashboard service; เป็น Markdown/JSON generator ที่อ่าน raw artifact ที่มีอยู่แล้ว.

| Customer risk | Deterministic gate | Performance/evidence metric |
|---|---|---|
| Registry import ทำข้อมูลเก่าเสีย | load/migrate/validate/save round-trip และ invalid legacy rejection | import time, allocation/size เฉพาะเมื่อพบ regression จริง |
| Duplicate scan ลบ/รายงานผิด | same-content, hard-link, same-size-different-content, unreadable input, no-mutation smoke | F1 per stratum, false-positive maximum, P95, candidate count |
| Search index ช้า/หาไม่เจอ | exact, Thai bigram, synonym, fuzzy negative fixtures | build/query median and P95 by 10K/100K/optional 1M |
| Document conversion ทำ format หาย | round-trip and integration tests | throughput only if users report latency issue |

ใช้ pass/fail gate ก่อน metric ranking: correctness, integrity และ no-mutation ต้องผ่านก่อน. Candidate ที่เร็วกว่าแต่ F1 ต่ำกว่า, false positive เกิน envelope หรือ migration ไม่ผ่าน คือ rejected, ไม่ใช่ trade-off ที่ซ่อนในค่าเฉลี่ย.

### Phase C: การนำเสนอผลที่คงความหมาย

เก็บ summary หน้าสั้น 1 หน้าแบบ Ponytail แต่ไม่ใช้ claim กว้าง. แสดง **baseline**, **candidate**, **corpus/fixture revision**, **run count**, **correctness gates**, และ **limitations** บนหน้าเดียว. ลิงก์ไป raw JSONL, manifest และ snapshot. แยก “result ที่ selected default” จาก “performance smoke” เพื่อไม่ทำให้ synthetic benchmark กลายเป็นการรับรอง production.

ผลที่ต้องรายงานเมื่อมีการเปลี่ยน default คือ median/P95, false-positive max per run, F1/recall ตาม stratum, candidate count และ gate status. Report ควรมี correction log: หาก ground-truth counter, fixture, harness หรือ isolation ผิด ให้เก็บ result revision พร้อมเหตุผลเหมือน Ponytail แทนการแก้ไฟล์เงียบ ๆ.[1] [2]

## สิ่งที่ไม่ควรทำตอนนี้

| สิ่งที่อาจดูดี | เหตุผลที่ยังไม่ทำ |
|---|---|
| LLM over-engineering/completeness judge | bl1nk-kept มี deterministic behavior contracts และไม่มีการเปรียบเทียบ agent arms; จะเพิ่ม cost/variance โดยไม่ตอบ acceptance criterion ใหม่ |
| Benchmark service/dashboard database | raw JSONL + manifest + static Markdown/HTML เพียงพอต่อผู้ตรวจปัจจุบัน; server เพิ่ม attack surface และงานดูแล |
| General benchmark framework | มี benchmark และ experiment สองตัวที่ทำงานจริงแล้ว; refactor เฉพาะเมื่อ duplication ที่พิสูจน์ได้เกินสอง use cases |
| Filesystem watcher/USN backend เพื่อ benchmark | persistent snapshot/delta planner มีอยู่ แต่ไม่มี real dataset/requirement ที่ยืนยันว่าการ refresh แบบ batch ไม่พอ |
| Embedding/reranker เพื่อเพิ่ม score | current public corpus วัด lexical filename behavior เท่านั้น; ต้องมี anonymized production corpus, labelled relevance และ repeatable offline evaluation ก่อน |

## Definition of done สำหรับรอบปรับปรุงการทดสอบถัดไป

งานนี้จบเมื่อ benchmark main และ duplicate experiment มี self-test ที่ fail ได้กับ intentionally bad fixture; ทุก run มี manifest; raw results กับ summary สร้างจาก command เดียวและตรวจ consistency ได้ offline; release note แสดง baseline/config/limitations; และ CI หรือ local release gate เรียก commands เหล่านี้โดยไม่ใช้ network หรือ secret.

เกณฑ์นี้ใช้ effort น้อยกว่าการสร้าง platform ใหม่ แต่ทำให้ทุกตัวเลขที่เลือก default หรือใช้ตัดสิน performance มี provenance, distribution และ failure condition ที่ตรวจได้. นี่คือการใช้ YAGNI กับ testing: เพิ่มเฉพาะเครื่องมือที่ทำให้ผลปัจจุบันเชื่อถือได้และแก้ไขได้ภายหลัง.

## References

[1]: https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/agentic/README.md "Ponytail agentic benchmark README"

[2]: https://github.com/DietrichGebert/ponytail/blob/main/benchmarks/results/2026-06-18-agentic.md "Ponytail agentic benchmark results and limitations"

[3]: https://github.com/DietrichGebert/ponytail/blob/main/.cursor/rules/ponytail.mdc "Ponytail operational YAGNI rule"

[4]: https://blog.scottlogic.com/2026/06/16/ponytail-yagni-and-the-problem-with-prompt-benchmarks.html "Scott Logic: Ponytail? YAGNI!"

[5]: https://martinfowler.com/bliki/Yagni.html "Martin Fowler: Yagni"
