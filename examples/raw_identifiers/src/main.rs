use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};

#[macro_use]
extern crate rocket;

#[openapi]
#[get("/hello/<type>")]
fn hello(r#type: String) -> String {
    r#type
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", openapi_get_routes![hello])
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .mount(
            "/rapidoc/",
            make_rapidoc(&RapiDocConfig {
                general: GeneralConfig {
                    spec_urls: vec![UrlObject::new("General", "../openapi.json")],
                    ..Default::default()
                },
                hide_show: HideShowConfig {
                    allow_spec_url_load: false,
                    allow_spec_file_load: false,
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket_okapi::openapi_get_spec;

    #[test]
    fn raw_identifier_route_present() {
        let spec = openapi_get_spec![hello];
        assert!(spec.paths.keys().any(|k| k.contains("/hello")));
    }
}
