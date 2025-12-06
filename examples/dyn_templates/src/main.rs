use rocket::get;
use rocket_dyn_templates::{context, Template};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use tracing_subscriber::EnvFilter;

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
    // Initialize tracing subscriber so RUST_LOG controls logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
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
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_okapi::openapi_get_spec;
    use serde_json::Value;

    #[test]
    fn spec_contains_page_route() {
        let spec = openapi_get_spec![get_page];
        assert!(spec.paths.keys().any(|k| k.contains("/page")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_provides_openapi_json_and_matches_routes() {
        let rocket = rocket::build()
            .mount("/", openapi_get_routes![get_page])
            .attach(Template::fairing());
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/page")));
        for path in spec["paths"].as_object().unwrap().keys() {
            let rocket_style = path.replace('{', "<").replace('}', ">");
            let rocket_style_alt = rocket_style.replace('>', "..>");
            let found = client.rocket().routes().any(|r| {
                r.uri.to_string().contains(&rocket_style)
                    || r.uri.to_string().contains(&rocket_style_alt)
            });
            assert!(
                found,
                "OpenApi path '{}' not found among Rocket routes",
                path
            );
        }
    }
}
