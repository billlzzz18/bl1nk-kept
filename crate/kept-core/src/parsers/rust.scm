; Rust tree-sitter query rules for kept source graph parsing
(function_item
  name: (identifier) @definition.function)

; Method signature inside trait (no body)
(function_signature_item
  name: (identifier) @definition.function)

(struct_item
  name: (type_identifier) @definition.struct)

(enum_item
  name: (type_identifier) @definition.enum)

(trait_item
  name: (type_identifier) @definition.trait)

(type_item
  name: (type_identifier) @definition.type_alias)

; macro_rules! definition
(macro_definition
  name: (identifier) @definition.macro)

; impl block — capture entire node, extract info from text
(impl_item) @implementation.block

; NOTE: Capture entire use_declaration as @import.statement
; The code will extract path, alias, and wildcard from the node text
(use_declaration) @import.statement

(call_expression
  function: [
    (identifier) @reference.call
    (field_expression
      field: (field_identifier) @reference.call)
    (scoped_identifier
      name: (identifier) @reference.call)
  ])

(type_identifier) @reference.type
