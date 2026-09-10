; Python tree-sitter query rules for kept source graph parsing

(function_definition
  name: (identifier) @definition.function)

(class_definition
  name: (identifier) @definition.class)

(import_statement
  name: [(dotted_name) @import.name
         (aliased_import
           name: (dotted_name) @import.name)])

(import_from_statement
  module_name: (dotted_name) @import.name)
