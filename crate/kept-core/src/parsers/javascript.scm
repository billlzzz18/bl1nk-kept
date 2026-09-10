; JavaScript tree-sitter query rules for kept source graph parsing

(function_declaration
  name: (identifier) @definition.function)

(generator_function_declaration
  name: (identifier) @definition.function)

(class_declaration
  name: (identifier) @definition.class)

(method_definition
  name: (property_identifier) @definition.function)

(import_statement
  source: (string (string_fragment) @import.name))
