; Go tree-sitter query rules for kept source graph parsing

(function_declaration
  name: (identifier) @definition.function)

(method_declaration
  name: (field_identifier) @definition.function)

(type_spec
  name: (type_identifier) @definition.type
  type: (struct_type) @_struct)

(type_spec
  name: (type_identifier) @definition.interface
  type: (interface_type) @_iface)

; NOTE: Skip the generic type_alias pattern — it duplicates struct/interface captures
; (type_spec
;   name: (type_identifier) @definition.type_alias
;   type: _ @_other)

(import_spec
  path: (interpreted_string_literal) @import.name)
