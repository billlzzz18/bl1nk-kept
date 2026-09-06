# Thai corpus and tokenization sources

เอกสารนี้บันทึกแหล่งข้อมูลที่ใช้เป็น reference สำหรับ Foundation v1.2.0. ไม่ได้อนุญาตให้ copy dataset ลง repository โดยอัตโนมัติ; ก่อนนำข้อมูลแต่ละรายการเข้า seed corpus ต้องตรวจ license และบันทึก source URI, license, retrieval date และ SHA-256 ใน `CorpusManifest`.

| Source | สิ่งที่ยืนยัน | ผลต่อการออกแบบ |
|---|---|---|
| [PyThaiNLP repository][1] | code ใช้ Apache-2.0; data/models มี license แยกตามชุดข้อมูล; มี word/subword/sentence tokenizer และ custom dictionary | runtime `kept` ไม่ต้องพึ่ง Python; ใช้เป็น reference สำหรับ vocabulary/provenance และต้องตรวจ license ราย corpus |
| [PyThaiNLP tokenization API][2] | word tokenization มี engines หลายแบบ; custom dictionary ใช้กับบาง engine; default `newmm` เป็น dictionary based | Foundation เก็บ glossary/custom dictionary tokens และใช้ lexical tokenizer deterministic ของ Rust ใน production |
| [PyThaiNLP corpus API][3] | corpus มี catalog, path/download behavior และมีแหล่งภาษาไทย เช่น words, synonyms, stopwords, Wikipedia titles; license ต้องดูตาม corpus | Corpus manifest ต้องมี license และ checksum ราย entry; importer ไม่ network-fetch ขณะ scan |
| [PyThaiNLP paper][4] | PyThaiNLP เป็น library NLP ไทย open source ที่รวม tools/models/datasets | ใช้เป็น background reference; ไม่ใช่หลักฐานให้เหมารวม license ของทุก dataset |

## License gate

ห้ามเพิ่ม public data เข้า seed corpus หากไม่สามารถระบุ source URI และ license ต่อรายการได้. Corpus ที่ระบุ non-commercial, attribution/share-alike หรือ license อื่นต้องให้ policy allow-list ตัดสินก่อน import. ทุกรอบ import ต้องเก็บ content SHA-256 และ retrieval timestamp.

## References

[1]: https://github.com/pythainlp/pythainlp "PyThaiNLP repository"
[2]: https://pythainlp.org/docs/4.0/api/tokenize.html "PyThaiNLP tokenization API"
[3]: https://pythainlp.org/dev-docs/api/corpus.html "PyThaiNLP corpus API"
[4]: https://aclanthology.org/2023.nlposs-1.4/ "PyThaiNLP: Thai Natural Language Processing in Python"

## คำตัดสินสำหรับ curated seed corpus รุ่นแรก

ตรวจ `corpus_license.md` ของ PyThaiNLP แล้ว: `words_th.txt`, `stopwords_th.txt`, `tnc_freq.txt` และ `ttc_freq.txt` อยู่ในรายการที่ PyThaiNLP ระบุ CC0-1.0. Seed corpus รุ่นแรกจึงใช้ **เฉพาะ excerpt ที่คัดเลือกจาก `words_th.txt` และ `stopwords_th.txt` พร้อม source URI, retrieval date, SHA-256 และ license `CC0-1.0`**. ไม่ใช้ Thai names corpus หรือ Wikipedia titles ในรุ่นแรก เพราะ upstream ระบุ CC-BY-SA-4.0 และต้องมี attribution/share-alike review แยกต่างหาก.[5]

[5]: https://github.com/PyThaiNLP/pythainlp/blob/dev/pythainlp/corpus/corpus_license.md "PyThaiNLP corpus license matrix"
