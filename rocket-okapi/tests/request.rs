//! Tests specifically for request module trait implementations

use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::request::*;
use rocket_okapi::settings::OpenApiSettings;
// Note: avoid importing rocket_okapi::Result here to not conflict with std::result::Result
use rocket::form::FromForm;
use rocket::serde::json::Json;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

#[test]
fn test_openapi_from_param_i32() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let p = <i32 as OpenApiFromParam>::path_parameter(&mut gen, "id".to_owned()).unwrap();
    assert_eq!(p.name, "id");
    assert_eq!(p.location, "path");
    assert!(p.required);
}

#[test]
fn test_openapi_from_form_field_i32() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let p =
        <i32 as OpenApiFromFormField>::form_parameter(&mut gen, "page".to_owned(), true).unwrap();
    assert_eq!(p.name, "page");
    assert_eq!(p.location, "query");
    assert!(p.required);
}

#[derive(FromForm, JsonSchema, Serialize, Deserialize)]
struct MyForm {
    id: i32,
    name: Option<String>,
}

#[test]
fn test_get_nested_form_parameters() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let params = get_nested_form_parameters::<MyForm>(&mut gen, "myform".to_owned(), true);
    // There should be two parameters: id and name
    assert!(params.iter().any(|p| p.name == "id"));
    assert!(params.iter().any(|p| p.name == "name"));
    // id should be required
    assert!(params.iter().any(|p| p.name == "id" && p.required));
}

#[derive(JsonSchema, Serialize, Deserialize)]
struct BodyShape {
    field: String,
}

#[test]
fn test_openapi_from_data_json() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let rb = <Json<BodyShape> as OpenApiFromData>::request_body(&mut gen).unwrap();
    assert!(rb.content.contains_key("application/json"));
    assert!(rb.required);
}

#[test]
fn test_openapi_from_data_string_and_vec() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let rb = <String as OpenApiFromData>::request_body(&mut gen).unwrap();
    assert!(rb.content.contains_key("application/octet-stream"));
    let rb2 = <Vec<u8> as OpenApiFromData>::request_body(&mut gen).unwrap();
    assert!(rb2.content.contains_key("application/octet-stream"));
}

#[test]
fn test_openapi_from_data_form_and_option() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Form of MyForm => multipart/form-data
    let rb = <rocket::form::Form<MyForm> as OpenApiFromData>::request_body(&mut gen).unwrap();
    assert!(rb.content.contains_key("multipart/form-data"));

    // Option<Json> should be not required
    let rb2 = <Option<Json<BodyShape>> as OpenApiFromData>::request_body(&mut gen).unwrap();
    assert!(!rb2.required);
}

#[test]
fn test_openapi_from_request_accept_and_option() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let res = <&rocket::http::Accept as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "a".to_owned(),
        true,
    )
    .unwrap();
    match res {
        RequestHeaderInput::Parameter(p) => {
            assert_eq!(p.name, "Accept");
            assert_eq!(p.location, "header");
            assert!(p.required);
        }
        _ => panic!("Expected Parameter"),
    }
    // Option should set required = false
    let res2 = <Option<&rocket::http::Accept> as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "a".to_owned(),
        true,
    )
    .unwrap();
    match res2 {
        RequestHeaderInput::Parameter(p) => assert!(!p.required),
        _ => panic!("Expected Parameter"),
    }
}

#[test]
fn test_openapi_from_request_content_type() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let res = <&rocket::http::ContentType as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "ct".to_owned(),
        false,
    )
    .unwrap();
    match res {
        RequestHeaderInput::Parameter(p) => {
            assert_eq!(p.name, "Content-Type");
            assert_eq!(p.location, "header");
            assert!(!p.required);
        }
        _ => panic!("Expected Parameter"),
    }
}

#[test]
fn test_openapi_from_request_none_types() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // IpAddr -> None
    let res_ip = <std::net::IpAddr as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "ip".to_owned(),
        true,
    )
    .unwrap();
    match res_ip {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // CookieJar -> None
    let res_cookie = <&rocket::http::CookieJar as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "cookie".to_owned(),
        false,
    )
    .unwrap();
    match res_cookie {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // Origin -> None
    let res_origin = <&rocket::http::uri::Origin as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "origin".to_owned(),
        true,
    )
    .unwrap();
    match res_origin {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // Route -> None
    let res_route = <&rocket::route::Route as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "route".to_owned(),
        true,
    )
    .unwrap();
    match res_route {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // Method -> None
    let res_method = <rocket::http::Method as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "method".to_owned(),
        false,
    )
    .unwrap();
    match res_method {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // Shutdown -> None
    let res_shutdown = <rocket::Shutdown as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "shutdown".to_owned(),
        false,
    )
    .unwrap();
    match res_shutdown {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }

    // FlashMessage -> None
    let res_flash = <rocket::request::FlashMessage as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "flash".to_owned(),
        false,
    )
    .unwrap();
    match res_flash {
        RequestHeaderInput::None => {}
        _ => panic!("Expected None"),
    }
}

#[test]
fn test_openapi_from_request_host() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let res = <&rocket::http::uri::Host as OpenApiFromRequest>::from_request_input(
        &mut gen,
        "host".to_owned(),
        true,
    )
    .unwrap();
    match res {
        RequestHeaderInput::Parameter(p) => assert_eq!(p.name, "Host"),
        _ => panic!("Expected Parameter"),
    }
}

#[derive(JsonSchema, Serialize, Deserialize, Debug)]
struct MySegments(String);

impl<'r> rocket::request::FromSegments<'r> for MySegments {
    type Error = Infallible;
    fn from_segments(
        _segments: rocket::http::uri::Segments<'r, rocket::http::uri::fmt::Path>,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(MySegments("".to_owned()))
    }
}

#[test]
fn test_openapi_from_segments_impl() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let p = <MySegments as OpenApiFromSegments>::path_multi_parameter(&mut gen, "seg".to_owned())
        .unwrap();
    assert_eq!(p.name, "seg");
    assert_eq!(p.location, "path");
    assert!(p.required);
    if let okapi::openapi3::ParameterValue::Schema { schema, .. } = p.value {
        // Schema is present
        assert!(schema.as_object().is_some());
    } else {
        panic!("Expected schema parameter");
    }
}

#[test]
fn test_openapi_from_request_result_and_outcome() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // StdResult wrapper should preserve required flag
    let res_r = <std::result::Result<&'static rocket::http::Accept, Infallible> as OpenApiFromRequest>::from_request_input(&mut gen, "a".to_owned(), true).unwrap();
    match res_r {
        RequestHeaderInput::Parameter(p) => assert!(p.required),
        _ => panic!("Expected Parameter"),
    }

    // Outcome wrapper should also preserve required
    let res_o = <rocket::request::Outcome<&'static rocket::http::Accept, Infallible> as OpenApiFromRequest>::from_request_input(&mut gen, "a".to_owned(), false).unwrap();
    match res_o {
        RequestHeaderInput::Parameter(p) => assert!(!p.required),
        _ => panic!("Expected Parameter"),
    }
}
