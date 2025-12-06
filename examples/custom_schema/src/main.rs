use rocket::{Build, Rocket};
use rocket_okapi::okapi::openapi3::OpenApi;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{mount_endpoints_and_merged_docs, rapidoc::*, swagger_ui::*};
use tracing_subscriber::EnvFilter;

mod error;
mod message;
mod post;

pub type Result<T> = std::result::Result<rocket::serde::json::Json<T>, error::Error>;
pub type DataResult<'a, T> =
    std::result::Result<rocket::serde::json::Json<T>, rocket::serde::json::Error<'a>>;

#[rocket::main]
async fn main() {
    // Initialize tracing subscriber so `RUST_LOG` controls the logging output.
    // Default to `info` if `RUST_LOG` is not set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    let launch_result = create_server().launch().await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {err}"),
    };
}

pub fn create_server() -> Rocket<Build> {
    let mut building_rocket = rocket::build()
        .mount(
            "/swagger-ui/",
            make_swagger_ui(&SwaggerUIConfig {
                url: "../v1/openapi.json".to_owned(),
                ..Default::default()
            }),
        )
        .mount(
            "/rapidoc/",
            make_rapidoc(&RapiDocConfig {
                title: Some("My special documentation | RapiDoc".to_owned()),
                general: GeneralConfig {
                    spec_urls: vec![UrlObject::new("General", "../v1/openapi.json")],
                    ..Default::default()
                },
                hide_show: HideShowConfig {
                    allow_spec_url_load: false,
                    allow_spec_file_load: false,
                    ..Default::default()
                },
                ..Default::default()
            }),
        );

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    let custom_route_spec = (vec![], custom_openapi_spec());
    mount_endpoints_and_merged_docs! {
        building_rocket, "/v1".to_owned(), openapi_settings,
        "/external" => custom_route_spec,
        "/post" => post::get_routes_and_docs(&openapi_settings),
        "/message" => message::get_routes_and_docs(&openapi_settings),
    };

    building_rocket
}

fn custom_openapi_spec() -> OpenApi {
    use rocket_okapi::okapi::map;
    use rocket_okapi::okapi::openapi3::*;
    use serde_json::json;
    OpenApi {
        openapi: OpenApi::default_version(),
        info: Info {
            title: "The best API ever".to_owned(),
            description: Some("This is the best API ever, please use me!".to_owned()),
            terms_of_service: Some(
                "https://github.com/GREsau/okapi/blob/master/LICENSE".to_owned(),
            ),
            contact: Some(Contact {
                name: Some("okapi example".to_owned()),
                url: Some("https://github.com/GREsau/okapi".to_owned()),
                email: None,
                ..Default::default()
            }),
            license: Some(License {
                name: "MIT".to_owned(),
                url: Some("https://github.com/GREsau/okapi/blob/master/LICENSE".to_owned()),
                ..Default::default()
            }),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            ..Default::default()
        },
        servers: vec![
            Server {
                url: "http://127.0.0.1:8000/".to_owned(),
                description: Some("Localhost".to_owned()),
                ..Default::default()
            },
            Server {
                url: "https://example.com/".to_owned(),
                description: Some("Possible Remote".to_owned()),
                ..Default::default()
            },
        ],
        // Add paths that do not exist in Rocket (or add extra info to existing paths)
        paths: {
            map! {
                "/home".to_owned() => PathItem{
                get: Some(
                    Operation {
                    tags: vec!["HomePage".to_owned()],
                    summary: Some("This is my homepage".to_owned()),
                    responses: Responses{
                        responses: map!{
                        "200".to_owned() => RefOr::Object(
                            Response{
                            description: "Return the page, no error.".to_owned(),
                            content: map!{
                                "text/html".to_owned() => MediaType{
                                schema: Some(json!({ "type": "string" }).try_into().expect("invalid schema")),
                                ..Default::default()
                                }
                            },
                            ..Default::default()
                            }
                        )
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                    }
                ),
                ..Default::default()
                }
            }
        },
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_okapi::openapi_get_spec;
    use serde_json::Value;

    #[test]
    fn custom_spec_contains_home_path() {
        let spec = custom_openapi_spec();
        assert!(spec.info.title.contains("The best API ever"));
        assert!(spec.paths.keys().any(|k| k.contains("/home")));
    }

    #[test]
    fn post_routes_spec_contains_post_path() {
        let settings = rocket_okapi::settings::OpenApiSettings::default();
        let spec = rocket_okapi::openapi_get_spec![settings: post::create_post, post::get_post];
        assert!(spec.paths.keys().any(|k| k.contains("/")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_provides_openapi_json_v1_and_routes_match() {
        // Start the server using the factory from this example
        let rocket = create_server();
        let client = Client::tracked(rocket).await.expect("valid client");

        // Request the generated OpenAPI JSON that the example mounts to /v1/openapi.json
        let spec = fetch_openapi_spec(&client, "/v1/openapi.json").await;

        // Quick sanity checks
        assert!(spec["openapi"].is_string());
        assert!(spec["paths"].is_object());

        // Helper: checks that every documented path is present in Rocket's mounted routes
        let paths = spec["paths"].as_object().unwrap();
        for path in paths.keys() {
            // If the path is from an external custom spec, it's valid in OpenAPI but not a Rocket route
            if path.starts_with("/external") {
                continue;
            }
            // Convert OpenAPI style path (e.g. /user/{id}) to Rocket style (/user/<id>)
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
