; TypeScript tree-sitter query rules for kept source graph parsing

(function_declaration
  name: (identifier) @definition.function)

(class_declaration
  name: (type_identifier) @definition.class)

(method_definition
  name: (property_identifier) @definition.function)

(interface_declaration
  name: (type_identifier) @definition.interface)

(type_alias_declaration
  name: (type_identifier) @definition.type_alias)

(enum_declaration
  name: (identifier) @definition.enum)

(import_statement
  source: (string (string_fragment) @import.name))
