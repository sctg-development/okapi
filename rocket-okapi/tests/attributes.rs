//! Test openapi attribute parsing (tag, operation_id, deprecated, skip)

use rocket_okapi::openapi_get_spec;

// These functions are never actually called.
#[allow(unused)]
mod endpoints {
    use rocket::{get, serde::json::Json};
    use rocket_okapi::openapi;

    #[openapi(tag = "Users", operation_id = "explicitOp")]
    #[get("/tag_opid")]
    pub fn tag_opid_controller() -> Json<()> {
        Json(())
    }

    #[openapi(deprecated = false)]
    #[get("/deprecated_false")]
    pub fn deprecated_false_controller() -> Json<()> {
        Json(())
    }

    #[openapi(skip)]
    #[get("/skipped")]
    pub fn skipped_controller() -> Json<()> {
        Json(())
    }
}

#[test]
fn explicit_tag_and_operation_id_are_set() {
    let spec = openapi_get_spec![endpoints::tag_opid_controller];

    let operation = spec.paths["/tag_opid"].get.as_ref().unwrap();
    assert_eq!(operation.tags.len(), 1);
    assert_eq!(operation.tags[0], "Users");
    assert_eq!(operation.operation_id.as_deref(), Some("explicitOp"));
}

#[test]
fn deprecated_explicit_false_is_not_deprecated() {
    let spec = openapi_get_spec![endpoints::deprecated_false_controller];

    let operation = spec.paths["/deprecated_false"].get.as_ref().unwrap();
    assert!(!operation.deprecated);
}

#[test]
fn skip_removes_route_from_spec() {
    // If the route is skipped, the generated spec should not contain it.
    let spec = openapi_get_spec![];
    assert!(!spec.paths.contains_key("/skipped"));
}
