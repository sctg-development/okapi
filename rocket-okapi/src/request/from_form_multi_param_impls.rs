use crate::gen::OpenApiGenerator;
use okapi::openapi3::SchemaObject;
use okapi::openapi3::{Object, Parameter, ParameterValue};
use schemars::JsonSchema;
use schemars::Schema;
use serde_json::Value;

/// Given an object that implements the `JsonSchema` generate all the `Parameter`
/// that are used to create documentation.
/// Use when manually implementing a
/// [Form Guard](https://api.rocket.rs/master/rocket/form/trait.FromForm.html).
///
/// Manual implementation is not needed anymore because of Generic trait implementation of
/// `OpenApiFromForm` and `OpenApiFromFormField`.
/// But still used internally for implementation.
pub fn get_nested_form_parameters<T>(
    gen: &mut OpenApiGenerator,
    name: String,
    required: bool,
) -> Vec<Parameter>
where
    T: JsonSchema,
{
    let schema = gen.json_schema_no_ref::<T>();
    // Get a list of properties from the structure.
    let mut properties: serde_json::Map<String, Value> = serde_json::Map::new();
    // Create all the `Parameter` for every property
    let mut parameter_list: Vec<Parameter> = Vec::new();
    // If schema is an object, extract properties
    if let Some(obj) = schema.as_object() {
        if let Some(props) = obj.get("properties") {
            if let Value::Object(map) = props {
                properties = map.clone();
            }
        }
    }
    if !properties.is_empty() {
        for (key, property) in properties {
            let prop_schema: Schema = match property.try_into() {
                Ok(s) => s,
                Err(_) => Schema::default(),
            };
            parameter_list.push(parameter_from_schema(prop_schema, key, required));
        }
    } else {
        parameter_list.push(parameter_from_schema(schema, name, required));
    }
    // Nothing else to handle here
    parameter_list
}

fn parameter_from_schema(schema: SchemaObject, name: String, mut required: bool) -> Parameter {
    // Check if parameter is optional (only is not already optional)
    if required
        && schema
            .as_object()
            .and_then(|o| o.get("nullable"))
            .and_then(|v| v.as_bool())
            == Some(true)
    {
        required = false;
    }
    let description = schema
        .as_object()
        .and_then(|o| o.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    Parameter {
        name,
        location: "query".to_owned(),
        description,
        required,
        deprecated: false,
        allow_empty_value: false,
        value: ParameterValue::Schema {
            style: None,
            explode: None,
            allow_reserved: false,
            schema,
            example: None,
            examples: None,
        },
        extensions: Object::default(),
    }
}
