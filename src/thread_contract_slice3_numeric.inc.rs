{
    macro_rules! row {
        ($label:literal, $instance:expr, $schema:expr, OK) => {
            assert_eq!(validate_schema(&$instance, &$schema), Ok(()), $label)
        };
        ($label:literal, $instance:expr, $schema:expr, ITEM) => {
            assert_eq!(validate_schema(&$instance, &$schema), Err(ThreadContractError::InvalidItem), $label)
        };
        ($label:literal, $instance:expr, $schema:expr, SCHEMA) => {
            assert_eq!(validate_schema(&$instance, &$schema), Err(ThreadContractError::InvalidSchema), $label)
        };
    }

    row!("minimum.signed_equal", json!(-2), json!({"minimum":-2}), OK);
    row!("minimum.signed_below", json!(-3), json!({"minimum":-2}), ITEM);
    row!("minimum.u64", json!(u64::MAX), json!({"minimum":0}), OK);
    row!("minimum.f64_accept", json!(1.5), json!({"minimum":1.0}), OK);
    row!("minimum.f64_reject", json!(0.5), json!({"minimum":1.0}), ITEM);
    row!("minimum.malformed", json!(0), json!({"minimum":"0"}), SCHEMA);
    row!("minimum.non_number_ignored", json!("x"), json!({"minimum":10}), OK);
    row!("min_length.unicode_equal", json!("界a"), json!({"minLength":2}), OK);
    row!("min_length.unicode_below", json!("界"), json!({"minLength":2}), ITEM);
    row!("min_length.zero", json!(""), json!({"minLength":0}), OK);
    row!("min_length.string_malformed", json!("x"), json!({"minLength":"2"}), SCHEMA);
    row!("min_length.negative_malformed", json!("x"), json!({"minLength":-1}), SCHEMA);
    row!("min_length.fraction_malformed", json!("x"), json!({"minLength":1.5}), SCHEMA);
    row!("min_length.non_string_ignored", json!(1), json!({"minLength":2}), OK);

    row!("format.int32_min", json!(-2147483648), json!({"type":"integer","format":"int32"}), OK);
    row!("format.int32_max", json!(2147483647), json!({"type":"integer","format":"int32"}), OK);
    row!("format.int32_below", json!(-2147483649_i64), json!({"type":"integer","format":"int32"}), ITEM);
    row!("format.int32_above", json!(2147483648_i64), json!({"type":"integer","format":"int32"}), ITEM);
    row!("format.int64_min", json!(i64::MIN), json!({"type":"integer","format":"int64"}), OK);
    row!("format.int64_max", json!(i64::MAX), json!({"type":"integer","format":"int64"}), OK);
    row!("format.int64_outside", json!(u64::MAX), json!({"type":"integer","format":"int64"}), ITEM);
    row!("format.uint_min", json!(0), json!({"type":"integer","format":"uint"}), OK);
    row!("format.uint_max", json!(u64::MAX), json!({"type":"integer","format":"uint"}), OK);
    row!("format.uint_outside", json!(-1), json!({"type":"integer","format":"uint"}), ITEM);
    row!("format.uint16_min", json!(0), json!({"type":"integer","format":"uint16"}), OK);
    row!("format.uint16_max", json!(65535), json!({"type":"integer","format":"uint16"}), OK);
    row!("format.uint16_below", json!(-1), json!({"type":"integer","format":"uint16"}), ITEM);
    row!("format.uint16_above", json!(65536), json!({"type":"integer","format":"uint16"}), ITEM);
    row!("format.uint32_min", json!(0), json!({"type":"integer","format":"uint32"}), OK);
    row!("format.uint32_max", json!(u32::MAX), json!({"type":"integer","format":"uint32"}), OK);
    row!("format.uint32_below", json!(-1), json!({"type":"integer","format":"uint32"}), ITEM);
    row!("format.uint32_above", json!(u32::MAX as u64 + 1), json!({"type":"integer","format":"uint32"}), ITEM);
    row!("format.uint64_min", json!(0), json!({"type":"integer","format":"uint64"}), OK);
    row!("format.uint64_max", json!(u64::MAX), json!({"type":"integer","format":"uint64"}), OK);
    row!("format.uint64_outside", json!(-1), json!({"type":"integer","format":"uint64"}), ITEM);
    row!("format.non_string_malformed", json!(1), json!({"format":1}), SCHEMA);
    row!("format.unknown_ignored", json!(1), json!({"type":"integer","format":"future"}), OK);
    row!("format.non_integer_ignored", json!("x"), json!({"format":"int32"}), OK);

    assert_eq!(
        validate_schema(&json!(null), &json!({"$ref":"#"})),
        Err(ThreadContractError::ValidationBudgetExceeded),
        "budget.root_ref"
    );
    assert_eq!(
        validate_schema(
            &json!(null),
            &json!({"$ref":"#/definitions/self","definitions":{"self":{"$ref":"#/definitions/self"}}}),
        ),
        Err(ThreadContractError::ValidationBudgetExceeded),
        "budget.local_ref"
    );

    fn nested(levels: usize) -> (Value, Value) {
        let mut instance = json!(null);
        let mut schema = json!(true);
        for _ in 0..levels {
            instance = json!([instance]);
            schema = json!({"type":"array","items":schema});
        }
        (instance, schema)
    }
    let (instance_64, schema_64) = nested(64);
    assert_eq!(
        validate_schema(&instance_64, &schema_64),
        Ok(()),
        "budget.depth_64"
    );
    let (instance_65, schema_65) = nested(65);
    assert_eq!(
        validate_schema(&instance_65, &schema_65),
        Err(ThreadContractError::ValidationBudgetExceeded),
        "budget.depth_65"
    );

    let item_schema = json!({"type":"array","items":true});
    let work_99_999 = Value::Array(vec![Value::Null; 99_999]);
    assert_eq!(
        validate_schema(&work_99_999, &item_schema),
        Ok(()),
        "budget.work_99_999"
    );
    let work_100_000 = Value::Array(vec![Value::Null; 100_000]);
    assert_eq!(
        validate_schema(&work_100_000, &item_schema),
        Err(ThreadContractError::ValidationBudgetExceeded),
        "budget.work_100_000"
    );

    let recursive_one = json!({"oneOf":[false,{"$ref":"#"}]});
    let recursive_any = json!({"anyOf":[true,{"$ref":"#"}]});
    let recursive_all = json!({"allOf":[true,{"$ref":"#"}]});
    for (label, schema) in [
        ("budget.one_of_propagates", recursive_one),
        ("budget.any_of_propagates", recursive_any),
        ("budget.all_of_propagates", recursive_all),
    ] {
        assert_eq!(
            validate_schema(&json!(null), &schema),
            Err(ThreadContractError::ValidationBudgetExceeded),
            "{label}"
        );
    }

    row!("schema.one_of_later_invalid", json!(null), json!({"oneOf":[false,{"type":"invalid"}]}), SCHEMA);
    row!("schema.any_of_later_invalid", json!(null), json!({"anyOf":[true,{"type":"invalid"}]}), SCHEMA);
    row!("schema.all_of_later_invalid", json!(null), json!({"allOf":[true,{"type":"invalid"}]}), SCHEMA);
}
