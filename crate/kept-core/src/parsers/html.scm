; HTML tree-sitter query rules for kept source graph parsing

(element
  (start_tag
    (tag_name) @_tag_name
    (attribute
      (attribute_name) @_attr
      (quoted_attribute_value
        (attribute_value) @definition.id)))
  (#eq? @_attr "id"))

(element
  (start_tag
    (tag_name) @definition.element))

(script_element
  (raw_text) @definition.script)
