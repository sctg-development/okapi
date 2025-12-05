//! Tests for OpenApiResponderInner implementations and util functions

use okapi::openapi3::RefOr;
use okapi::openapi3::Responses;
use rocket::fs::NamedFile;
use rocket::response::content::RawXml;
use rocket::response::stream::ByteStream;
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::settings::OpenApiSettings;
use std::sync::Arc;

#[test]
fn test_unit_responses_200() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let r = <() as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r.responses.contains_key("200"));
}

#[test]
fn test_error_io_responses_500() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let r = <std::io::Error as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r.responses.contains_key("500"));
}

#[test]
fn test_string_and_vec_responses() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let r = <String as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => !o.content.is_empty(),
        RefOr::Ref(_) => false,
    }));
    let r2 = <Vec<u8> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r2.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => !o.content.is_empty(),
        RefOr::Ref(_) => false,
    }));
}

#[test]
fn test_no_content_and_redirect() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let r = <rocket::response::status::NoContent as OpenApiResponderInner>::responses(&mut gen)
        .unwrap();
    assert!(r.responses.contains_key("204"));
    let rr = <rocket::response::Redirect as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(
        rr.responses.contains_key("301")
            || rr.responses.contains_key("302")
            || rr.responses.contains_key("500")
    );
}

#[test]
fn test_result_combination_and_option() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let r =
        <std::result::Result<String, std::io::Error> as OpenApiResponderInner>::responses(&mut gen)
            .unwrap();
    assert!(r.responses.contains_key("200"));
    assert!(r.responses.contains_key("500"));
    let r2 = <Option<String> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r2.responses.contains_key("404"));
}

#[test]
fn test_box_and_vec_and_slice_wrapper() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Box<[u8]> -> Vec<u8>
    let b = <Box<[u8]> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(b.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));

    // Arc<[u8]> -> Vec<u8>
    let a = <Arc<[u8]> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(a.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));

    // &str -> String -> text/plain
    let s = <&str as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(s.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.keys().any(|k| k.contains("text/plain")),
        RefOr::Ref(_) => false,
    }));

    // &[u8]
    let slice = <&[u8] as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(slice.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));
}

#[test]
fn test_either_and_content_type_wrappers() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    let re =
        <rocket::Either<String, Vec<u8>> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(re.responses.contains_key("200"));
    // Test content type wrapper: (ContentType, R)
    let rct = <(rocket::http::ContentType, String) as OpenApiResponderInner>::responses(&mut gen)
        .unwrap();
    // Content type set to Any -> media type key `*/*` is not used here but ensure responses exist
    assert!(!rct.responses.is_empty());
}

#[test]
fn test_status_responders_and_rawjson() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Created: 201
    let r =
        <rocket::response::status::Created<String> as OpenApiResponderInner>::responses(&mut gen)
            .unwrap();
    assert!(r.responses.contains_key("201"));
    // RawJson wrapper should set content type to application/json
    let rj =
        <rocket::response::content::RawJson<String> as OpenApiResponderInner>::responses(&mut gen)
            .unwrap();
    assert!(rj.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/json"),
        RefOr::Ref(_) => false,
    }));
}

#[test]
fn test_text_and_event_streams() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // TextStream generic: we can instantiate with unit type
    type Ts = rocket::response::stream::TextStream<()>;
    let t = <Ts as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(t.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.keys().any(|k| k.contains("text/plain")),
        RefOr::Ref(_) => false,
    }));

    // EventStream
    type Es = rocket::response::stream::EventStream<()>;
    let e = <Es as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(e.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.keys().any(|k| k.contains("event-stream")),
        RefOr::Ref(_) => false,
    }));
}

#[test]
fn test_box_and_arc_and_cow_and_debug() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // Box and Arc -> delegate
    let b = <Box<str> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(b.responses.contains_key("200"));
    let a = <Arc<str> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(a.responses.contains_key("200"));
    // Debug<E> -> 500
    let d = <rocket::response::Debug<std::io::Error> as OpenApiResponderInner>::responses(&mut gen)
        .unwrap();
    assert!(d.responses.contains_key("500"));
}

#[test]
fn test_namedfile_and_capped_and_streams() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // NamedFile -> Vec<u8>
    let nf = <NamedFile as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(nf.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));

    // Capped -> delegate
    type CappedVec = rocket::data::Capped<Vec<u8>>;
    let c = <CappedVec as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(c.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));

    // ByteStream -> Vec<u8>
    type Bs = ByteStream<()>;
    let b = <Bs as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(b.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.contains_key("application/octet-stream"),
        RefOr::Ref(_) => false,
    }));

    // ReaderStream -> ensure 200 exists (requires a Stream item type; skip direct instantiation here)
}

#[test]
fn test_status_responders_others_and_flash_and_box_and_capped() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    // BadRequest: 400
    let br = <rocket::response::status::BadRequest<String> as OpenApiResponderInner>::responses(
        &mut gen,
    )
    .unwrap();
    assert!(br.responses.contains_key("400"));

    // Forbidden: 403
    let f =
        <rocket::response::status::Forbidden<String> as OpenApiResponderInner>::responses(&mut gen)
            .unwrap();
    assert!(f.responses.contains_key("403"));

    // Flash forwards to inner
    let fl =
        <rocket::response::Flash<String> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(fl.responses.contains_key("200"));

    // Box<T>
    let boxed = <Box<String> as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(boxed.responses.contains_key("200"));
}

#[test]
fn test_raw_xml_content_wrapper() {
    let mut gen = OpenApiGenerator::new(&OpenApiSettings::new());
    type RawXmlType = RawXml<String>;
    let r = <RawXmlType as OpenApiResponderInner>::responses(&mut gen).unwrap();
    assert!(r.responses.iter().any(|(_, resp)| match resp {
        RefOr::Object(o) => o.content.keys().any(|k| k.contains("text/xml")),
        RefOr::Ref(_) => false,
    }));
}

// UTIL function tests
#[test]
fn test_set_content_type_and_default() {
    use okapi::openapi3::Response;
    use rocket_okapi::util::*;
    let mut r = Responses::default();
    // add 200 response with application/json
    let mut resp = Response::default();
    resp.content.insert(
        "application/json".to_owned(),
        okapi::openapi3::MediaType::default(),
    );
    r.responses.insert("200".to_owned(), resp.into());
    // change to text/plain
    set_content_type(&mut r, "text/plain").unwrap();
    assert!(r.responses.values().any(|rr| ensure_not_ref_for_tests(rr)
        .content
        .contains_key("text/plain")));
    // change all responses to default
    change_all_responses_to_default(&mut r);
    assert!(r.responses.contains_key("default"));
}

fn ensure_not_ref_for_tests(
    response: &RefOr<okapi::openapi3::Response>,
) -> okapi::openapi3::Response {
    match response {
        RefOr::Ref(..) => okapi::openapi3::Response::default(),
        RefOr::Object(o) => o.clone(),
    }
}
