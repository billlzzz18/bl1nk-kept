use kept_doc::converter::{FilterPipeline, ThaiSanitizationFilter};
use kept_doc::ir::{DocumentMetadata, StyleSheet, UniversalBlock, UniversalDocument};
use kept_doc::sync::{Op, Reconciler};

#[test]
fn test_filter_and_sync_workflow() {
    // 1. เตรียมเอกสาร Local (ที่มีช่องว่างซ้ำและเนื้อหาใหม่)
    let mut local_doc = UniversalDocument {
        metadata: DocumentMetadata::default(),
        blocks: vec![
            UniversalBlock::Paragraph {
                content: vec![kept_doc::ir::inline::text("สวัสดี  ชาวโลก")], // มีช่องว่าง 2 จุด
                style: None,
            },
            UniversalBlock::Heading {
                level: 1,
                content: vec![kept_doc::ir::inline::text("บทนำ")],
                style: None,
            },
        ],
        styles: StyleSheet::default(),
    };

    // 2. เตรียมเอกสาร Remote (สถานะปัจจุบันบน Cloud)
    let remote_doc = UniversalDocument {
        metadata: DocumentMetadata::default(),
        blocks: vec![UniversalBlock::Paragraph {
            content: vec![kept_doc::ir::inline::text("สวัสดี ชาวโลก")],
            style: None,
        }],
        styles: StyleSheet::default(),
    };

    // 3. รัน Filter Pipeline (M3)
    let mut pipeline = FilterPipeline::new();
    pipeline.add(Box::new(ThaiSanitizationFilter));
    pipeline.run(&mut local_doc).unwrap();

    // ตรวจสอบว่า Filter ทำงาน (ช่องว่างถูกลดเหลือ 1)
    if let UniversalBlock::Paragraph { content, .. } = &local_doc.blocks[0] {
        if let kept_doc::ir::inline::InlineElement::TextRun { content, .. } = &content[0] {
            assert_eq!(content, "สวัสดี ชาวโลก");
        }
    }

    // 4. คำนวณความแตกต่าง (M5)
    let changeset = Reconciler::diff(&local_doc, &remote_doc).unwrap();

    // 5. ตรวจสอบ Sync Plan
    println!("--- Sync Plan Analysis ---");
    println!("Total Operations: {}", changeset.ops.len());
    for (i, op) in changeset.ops.iter().enumerate() {
        println!("Op {}: {:?}", i + 1, op);
    }

    assert_eq!(changeset.ops.len(), 1);
    match &changeset.ops[0] {
        Op::Insert { index, .. } => assert_eq!(*index, Some(1)),
        _ => panic!("Expected Insert operation for the new heading"),
    }

    println!("--- Summary ---");
    println!("{}", changeset.summary());
}
