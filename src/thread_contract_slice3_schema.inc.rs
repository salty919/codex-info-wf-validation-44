{
    macro_rules! row {
        ($label:literal, $instance:expr, $schema:expr, OK) => {
            assert_eq!(validate_schema(&$instance, &$schema), Ok(()), $label)
        };
        ($label:literal, $instance:expr, $schema:expr, ITEM) => {
            assert_eq!(
                validate_schema(&$instance, &$schema),
                Err(ThreadContractError::InvalidItem),
                $label
            )
        };
        ($label:literal, $instance:expr, $schema:expr, SCHEMA) => {
            assert_eq!(
                validate_schema(&$instance, &$schema),
                Err(ThreadContractError::InvalidSchema),
                $label
            )
        };
    }

    row!("schema.true", json!(null), json!(true), OK);
    row!("schema.false", json!("DO_NOT_LEAK_THREAD_VALUE"), json!(false), ITEM);
    row!("schema.null", json!(null), json!(null), SCHEMA);
    row!("schema.string", json!(null), json!("schema"), SCHEMA);

    row!("ref.escaped_slash", json!(1), json!({"$ref":"#/definitions/a~1b","definitions":{"a/b":{"type":"integer"}}}), OK);
    row!("ref.escaped_tilde", json!("x"), json!({"$ref":"#/definitions/til~0de","definitions":{"til~de":{"type":"string"}}}), OK);
    row!("ref.false_target", json!(null), json!({"$ref":"#/definitions/no","definitions":{"no":false}}), ITEM);
    row!("ref.non_string", json!(null), json!({"$ref":1}), SCHEMA);
    row!("ref.remote", json!(null), json!({"$ref":"https://invalid.example/schema"}), SCHEMA);
    row!("ref.missing", json!(null), json!({"$ref":"#/definitions/missing","definitions":{}}), SCHEMA);
    row!("ref.bad_escape", json!(null), json!({"$ref":"#/definitions/a~2b","definitions":{"a/b":true}}), SCHEMA);
    row!("ref.sibling_ignored", json!(1), json!({"$ref":"#/definitions/int","type":"not-a-type","definitions":{"int":{"type":"integer"}}}), OK);

    row!("type.null.ok", json!(null), json!({"type":"null"}), OK);
    row!("type.null.reject", json!(false), json!({"type":"null"}), ITEM);
    row!("type.boolean.ok", json!(true), json!({"type":"boolean"}), OK);
    row!("type.boolean.reject", json!(null), json!({"type":"boolean"}), ITEM);
    row!("type.object.ok", json!({}), json!({"type":"object"}), OK);
    row!("type.object.reject", json!([]), json!({"type":"object"}), ITEM);
    row!("type.array.ok", json!([]), json!({"type":"array"}), OK);
    row!("type.array.reject", json!({}), json!({"type":"array"}), ITEM);
    row!("type.string.ok", json!("s"), json!({"type":"string"}), OK);
    row!("type.string.reject", json!(1), json!({"type":"string"}), ITEM);
    row!("type.number.ok", json!(1.5), json!({"type":"number"}), OK);
    row!("type.number.reject", json!("1"), json!({"type":"number"}), ITEM);
    row!("type.integer.ok", json!(1), json!({"type":"integer"}), OK);
    row!("type.integer.reject_fraction", json!(1.5), json!({"type":"integer"}), ITEM);
    row!("type.number_accepts_integer", json!(1), json!({"type":"number"}), OK);
    row!("type.union_string", json!("s"), json!({"type":["string","null"]}), OK);
    row!("type.union_null", json!(null), json!({"type":["string","null"]}), OK);
    row!("type.union_reject", json!(true), json!({"type":["string","null"]}), ITEM);
    row!("type.union_integer_number", json!(1), json!({"type":["integer","number"]}), OK);
    row!("type.unknown", json!(null), json!({"type":"unknown"}), SCHEMA);
    row!("type.empty_union", json!(null), json!({"type":[]}), SCHEMA);
    row!("type.union_unknown", json!(null), json!({"type":["null","unknown"]}), SCHEMA);
    row!("type.union_non_string", json!(null), json!({"type":["null",1]}), SCHEMA);
    row!("type.non_string", json!(null), json!({"type":1}), SCHEMA);

    row!("enum.accept", json!("x"), json!({"enum":[null,"x",2]}), OK);
    row!("enum.reject", json!(true), json!({"enum":[null,"x",2]}), ITEM);
    row!("enum.empty", json!(null), json!({"enum":[]}), ITEM);
    row!("enum.malformed", json!("x"), json!({"enum":"x"}), SCHEMA);

    row!("required.present", json!({"x":1}), json!({"required":["x"]}), OK);
    row!("required.missing", json!({}), json!({"required":["x"]}), ITEM);
    row!("required.non_array", json!({}), json!({"required":"x"}), SCHEMA);
    row!("required.non_string_member", json!({}), json!({"required":[1]}), SCHEMA);
    row!("properties.accept", json!({"x":1}), json!({"properties":{"x":{"type":"integer"}}}), OK);
    row!("properties.reject", json!({"x":"1"}), json!({"properties":{"x":{"type":"integer"}}}), ITEM);
    row!("properties.optional_absent", json!({}), json!({"properties":{"x":{"type":"integer"}}}), OK);
    row!("properties.non_object", json!({}), json!({"properties":[]}), SCHEMA);
    row!("properties.scalar_child", json!({}), json!({"properties":{"x":1}}), SCHEMA);
    row!("properties.false_present", json!({"x":null}), json!({"properties":{"x":false}}), ITEM);

    row!("additional.with_properties_true", json!({"x":1,"y":"v"}), json!({"properties":{"x":{"type":"integer"}},"additionalProperties":true}), OK);
    row!("additional.with_properties_false_known", json!({"x":1}), json!({"properties":{"x":{"type":"integer"}},"additionalProperties":false}), OK);
    row!("additional.with_properties_false_unknown", json!({"x":1,"y":2}), json!({"properties":{"x":{"type":"integer"}},"additionalProperties":false}), ITEM);
    row!("additional.with_properties_schema_accept", json!({"x":1,"y":2}), json!({"properties":{"x":{"type":"integer"}},"additionalProperties":{"type":"integer"}}), OK);
    row!("additional.with_properties_schema_reject", json!({"x":1,"y":"2"}), json!({"properties":{"x":{"type":"integer"}},"additionalProperties":{"type":"integer"}}), ITEM);
    row!("additional.no_properties_true", json!({"a":1}), json!({"additionalProperties":true}), OK);
    row!("additional.no_properties_false_empty", json!({}), json!({"additionalProperties":false}), OK);
    row!("additional.no_properties_false_reject", json!({"a":1}), json!({"additionalProperties":false}), ITEM);
    row!("additional.no_properties_schema_accept", json!({"a":1,"b":2}), json!({"additionalProperties":{"type":"integer"}}), OK);
    row!("additional.no_properties_schema_reject", json!({"a":"x"}), json!({"additionalProperties":{"type":"integer"}}), ITEM);
    row!("additional.malformed", json!({}), json!({"additionalProperties":1}), SCHEMA);

    row!("items.object_accept", json!([1,2]), json!({"items":{"type":"integer"}}), OK);
    row!("items.object_reject", json!([1,"x"]), json!({"items":{"type":"integer"}}), ITEM);
    row!("items.true_nonempty", json!(["x"]), json!({"items":true}), OK);
    row!("items.false_nonempty", json!([null]), json!({"items":false}), ITEM);
    row!("items.false_empty", json!([]), json!({"items":false}), OK);
    row!("items.tuple_accept", json!([1,"x"]), json!({"items":[{"type":"integer"},{"type":"string"}]}), OK);
    row!("items.tuple_reject", json!(["x",1]), json!({"items":[{"type":"integer"},{"type":"string"}]}), ITEM);
    row!("items.tuple_extra_ignored", json!([1,"x",false]), json!({"items":[{"type":"integer"},{"type":"string"}]}), OK);
    row!("items.scalar_malformed", json!([]), json!({"items":1}), SCHEMA);
    row!("items.tuple_scalar_malformed", json!([]), json!({"items":[{"type":"integer"},1]}), SCHEMA);
    row!("items.non_array_ignored", json!({"x":"v"}), json!({"items":false}), OK);

    row!("one_of.one", json!("x"), json!({"oneOf":[{"type":"string"},{"type":"integer"}]}), OK);
    row!("one_of.zero", json!(false), json!({"oneOf":[{"type":"string"},{"type":"integer"}]}), ITEM);
    row!("one_of.multiple", json!(1), json!({"oneOf":[{"type":"integer"},{"type":"number"}]}), ITEM);
    row!("one_of.empty", json!(null), json!({"oneOf":[]}), ITEM);
    row!("one_of.non_array", json!(null), json!({"oneOf":true}), SCHEMA);
    row!("one_of.scalar_branch", json!(null), json!({"oneOf":[1]}), SCHEMA);
    row!("one_of.later_invalid_schema", json!(null), json!({"oneOf":[false,{"type":"invalid"}]}), SCHEMA);
    row!("any_of.one", json!("x"), json!({"anyOf":[{"type":"string"},{"type":"integer"}]}), OK);
    row!("any_of.multiple", json!(1), json!({"anyOf":[{"type":"integer"},{"type":"number"}]}), OK);
    row!("any_of.zero", json!(false), json!({"anyOf":[{"type":"string"},{"type":"integer"}]}), ITEM);
    row!("any_of.empty", json!(null), json!({"anyOf":[]}), ITEM);
    row!("any_of.non_array", json!(null), json!({"anyOf":true}), SCHEMA);
    row!("any_of.scalar_branch", json!(null), json!({"anyOf":[1]}), SCHEMA);
    row!("any_of.later_invalid_schema", json!(null), json!({"anyOf":[true,{"type":"invalid"}]}), SCHEMA);
    row!("all_of.pass", json!(2), json!({"allOf":[{"type":"integer"},{"minimum":1}]}), OK);
    row!("all_of.fail", json!(0), json!({"allOf":[{"type":"integer"},{"minimum":1}]}), ITEM);
    row!("all_of.empty", json!(null), json!({"allOf":[]}), OK);
    row!("all_of.non_array", json!(null), json!({"allOf":true}), SCHEMA);
    row!("all_of.scalar_branch", json!(null), json!({"allOf":[1]}), SCHEMA);
    row!("all_of.later_invalid_schema", json!(null), json!({"allOf":[true,{"type":"invalid"}]}), SCHEMA);

    row!("ignored.required_on_array", json!([]), json!({"required":["x"]}), OK);
    row!("ignored.properties_on_array", json!([]), json!({"properties":{"x":false}}), OK);
    row!("ignored.additional_on_array", json!([]), json!({"additionalProperties":false}), OK);
    row!("ignored.items_on_object", json!({}), json!({"items":false}), OK);
    row!("ignored.vendor_keyword", json!("DO_NOT_LEAK_THREAD_VALUE"), json!({"x-vendor":{"arbitrary":1}}), OK);
}
