use rocket::get;
use rocket_dyn_templates::{context, Template};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};

/// # Get Page
///
/// Returns a page by ID.
#[openapi(tag = "Page")]
#[get("/page?<name>")]
fn get_page(name: String) -> Template {
    Template::render("template-1", context! { name: name })
}

#[rocket::main]
async fn main() {
    let launch_result = rocket::build()
        .mount("/", openapi_get_routes![get_page])
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
        .attach(Template::fairing())
        .launch()
        .await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {err}"),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket_okapi::openapi_get_spec;

    #[test]
    fn spec_contains_page_route() {
        let spec = openapi_get_spec![get_page];
        assert!(spec.paths.keys().any(|k| k.contains("/page")));
    }
}
