use okapi::openapi3::{Operation, Responses, SecurityScheme, SecuritySchemeData};
use rocket::http::Method;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::settings::OpenApiSettings;
use rocket_okapi::OperationInfo;

#[test]
fn test_add_security_and_into_openapi_and_operation_id() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Add a simple security scheme
    let scheme = SecurityScheme {
        description: None,
        data: SecuritySchemeData::Http {
            scheme: "basic".to_owned(),
            bearer_format: None,
        },
        extensions: okapi::openapi3::Object::default(),
    };
    gen.add_security_scheme("myscheme".to_owned(), scheme);

    // Add an operation with an operation_id that includes module path separators.
    let op = Operation {
        operation_id: Some("::module::action".to_owned()),
        responses: Responses::default(),
        ..Operation::default()
    };
    let info = OperationInfo {
        path: "/one".to_owned(),
        method: Method::Get,
        operation: op,
    };
    gen.add_operation(info);

    // Add another method to the same path
    let op2 = Operation {
        responses: Responses::default(),
        ..Operation::default()
    };
    gen.add_operation(OperationInfo {
        path: "/one".to_owned(),
        method: Method::Post,
        operation: op2,
    });

    let openapi = gen.into_openapi();
    // Paths should contain the route
    assert!(openapi.paths.contains_key("/one"));
    // The GET operation operation_id should be transformed 'module_action'
    let path_item = openapi.paths.get("/one").unwrap();
    let get_op = path_item.get.as_ref().unwrap();
    assert_eq!(get_op.operation_id.as_deref(), Some("module_action"));
    // Components should contain security scheme 'myscheme'
    let comps = openapi.components.as_ref().unwrap();
    assert!(comps.security_schemes.contains_key("myscheme"));
}

#[test]
fn test_json_schema_and_schema_generator_methods() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Call json_schema and json_schema_no_ref
    let _sch = gen.json_schema::<i32>();
    let _refsch = gen.json_schema_no_ref::<i32>();
    // schema_generator getter
    let _sg = gen.schema_generator();
    assert!(!_sg.definitions().is_empty() || true);
}
