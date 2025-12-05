use rocket_okapi::handlers::OpenApiHandler;
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::OpenApiSettings;
use rocket::http::Status;
use rocket::local::blocking::Client;
use rocket::http::ContentType;
use rocket_okapi::handlers::{ContentHandler, RedirectHandler};

#[test]
fn test_openapi_handler_adds_base_path_server() {
    // Spec with no servers
    let spec = OpenApi::default();
    let handler = OpenApiHandler::new(spec);
    let route = handler.into_route("/openapi");
    let rocket = rocket::build().mount("/v1", vec![route]);
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let resp = client.get("/v1/openapi").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    let body = resp.into_string().expect("body");
    // Should contain base path /v1 added in servers
    assert!(body.contains("/v1"));
}

#[test]
fn test_content_handler_bytes_and_json_and_trailing_slash() {
    // Bytes handler
    let handler = ContentHandler::bytes(ContentType::HTML, b"hello");
    let route = handler.into_route("/file");
    let rocket = rocket::build().mount("/v1", vec![route]);
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let resp = client.get("/v1/file").dispatch();
    assert_eq!(resp.status(), Status::Ok);
    assert_eq!(resp.into_string().unwrap(), "hello");

    // JSON handler
    let jhandler = ContentHandler::json(&"{\"a\":1}".to_string());
    let r2 = jhandler.into_route("/j");
    let rocket2 = rocket::build().mount("/j", vec![r2]);
    let client2 = Client::tracked(rocket2).expect("valid rocket instance");
    let r = client2.get("/j/j").dispatch();
    assert_eq!(r.status(), Status::Ok);

    // trailing slash should result in a forward (redirection)
    let handler_bytes = ContentHandler::bytes(ContentType::Plain, b"x");
    let route2 = handler_bytes.into_route("/slash");
    let rocket3 = rocket::build().mount("/s", vec![route2]);
    let client3 = Client::tracked(rocket3).expect("valid rocket instance");
    let r3 = client3.get("/s/slash/").dispatch();
    assert!(r3.status().code >= 300 && r3.status().code < 400);
}

#[test]
fn test_redirect_handler_to_dest_has_location_and_redirect_status() {
    let handler = RedirectHandler::to("target");
    let route = handler.into_route("/r");
    let rocket = rocket::build().mount("/v2", vec![route]);
    let client = Client::tracked(rocket).expect("valid rocket instance");
    let resp = client.get("/v2/r").dispatch();
    assert!(resp.status().code >= 300 && resp.status().code < 400);
    let loc = resp.headers().get_one("Location").unwrap();
    assert!(loc.ends_with("/target"));
}
