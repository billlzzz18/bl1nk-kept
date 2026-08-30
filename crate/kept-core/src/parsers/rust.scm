// Rust tree-sitter query rules for kept source graph parsing
(function_item
  name: (identifier) @definition.function) @scope

(struct_item
  name: (type_identifier) @definition.struct) @scope

(enum_item
  name: (type_identifier) @definition.enum) @scope

(trait_item
  name: (type_identifier) @definition.trait) @scope

(type_item
  name: (type_identifier) @definition.type_alias) @scope

(impl_item
  trait: (type_identifier)? @implementation.trait
  type: (type_identifier) @implementation.target) @scope

(use_declaration
  argument: [
    (identifier) @import.name
    (scoped_identifier) @import.scoped
    (use_as_clause
      path: [(identifier) (scoped_identifier)] @import.source
      alias: (identifier) @import.alias)
    (use_wildcard) @import.wildcard
  ])

(call_expression
  function: [
    (identifier) @reference.call
    (field_expression
      field: (field_identifier) @reference.call)
    (scoped_identifier
      name: (identifier) @reference.call)
  ])

(type_identifier) @reference.type
