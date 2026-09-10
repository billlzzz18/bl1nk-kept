; Markdown tree-sitter query rules for kept source graph parsing
; NOTE: tree-sitter-md parses block/inline separately; we target block-level headings

(atx_heading
  (atx_h1_marker)
  heading_content: (_) @definition.heading1)

(atx_heading
  (atx_h2_marker)
  heading_content: (_) @definition.heading2)

(atx_heading
  (atx_h3_marker)
  heading_content: (_) @definition.heading3)

(fenced_code_block
  (info_string) @definition.code_block)
