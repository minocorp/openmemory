use schemars::Schema;
use serde_json::Value;

pub(crate) fn omit_null(schema: &mut Schema) {
    let Some(Value::Object(properties)) = schema.get_mut("properties") else {
        return;
    };
    for property in properties.values_mut() {
        let Some(Value::Array(types)) = property.get_mut("type") else {
            continue;
        };
        types.retain(|kind| kind != "null");
        if let [only] = types.as_slice() {
            let only = only.clone();
            property["type"] = only;
        }
    }
}

#[cfg(test)]
mod tests {
    use schemars::{JsonSchema, schema_for};

    #[derive(JsonSchema)]
    #[schemars(transform = super::omit_null)]
    #[expect(dead_code, reason = "only the schema is inspected")]
    struct Sample {
        required: String,
        optional: Option<String>,
        count: Option<i32>,
    }

    #[test]
    fn optional_fields_lose_the_null_type_and_stay_optional() {
        let schema = schema_for!(Sample).to_value();
        assert_eq!(schema["properties"]["optional"]["type"], "string");
        assert_eq!(schema["properties"]["count"]["type"], "integer");
        assert_eq!(schema["required"], serde_json::json!(["required"]));
    }
}
