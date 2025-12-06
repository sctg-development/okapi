use rocket::config::Config;
use rocket::Request;
use rocket::{catch, catchers, response, response::Responder, Response};
use rocket_okapi::gen::OpenApiGenerator;
use rocket_okapi::okapi;
use rocket_okapi::okapi::openapi3::{MediaType, Responses};
use rocket_okapi::okapi::schemars;
use rocket_okapi::response::OpenApiResponderInner;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi_get_routes, rapidoc::*, swagger_ui::*, OpenApiError};
use tracing_subscriber::EnvFilter;

// --------- All different methods of implementing `OpenApiFromRequest` ------------
// There are a few different ways of doing things.
// And it also depend on the authentication (if any) you want to implement.
// Here are a few different example that cover most of the use cases:
// - No special authentication
// - ApiKey (in http header, query or cookie)
// - HTTP `Authorization` header (inc `basic`, `digest` and `bearer` tokens)
// https://developer.mozilla.org/en-US/docs/Web/HTTP/Authentication#authentication_schemes
// - OAuth 2.0 flows (authorizationCode, implicit, password, clientCredentials)
// - OpenID Connect
// - Just Cookies (for just 1 route/endpoint)
// ---------------------------------------------------------------------------------

mod no_auth;

mod api_key;

mod http_auth;

mod oauth2;

mod open_id;

mod cookies;

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber so RUST_LOG controls logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    let figment = Config::figment()
        // Set a dummy secret
        .merge(("secret_key", vec![1u8; 64]));

    let launch_result = rocket::custom(figment)
        .mount(
            "/",
            openapi_get_routes![
                no_auth::no_special_auth,
                api_key::api_key,
                http_auth::http_auth,
                oauth2::oauth2_auth_code_get_user,
                open_id::open_id,
                cookies::cookie_auth,
            ],
        )
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
                ui: UiConfig {
                    theme: Theme::Dark,
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
        .register("/", catchers![bad_request, unauthorized])
        .launch()
        .await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {err}"),
    };
}

// ----- Catchers -------

/// Error messages returned to user
#[derive(Debug, serde::Serialize, schemars::JsonSchema)]
pub struct MyError {
    /// The title of the error message
    pub err: String,
    /// The description of the error
    pub msg: Option<String>,
    // HTTP Status Code returned
    #[serde(skip)]
    pub http_status_code: u16,
}

#[catch(400)]
fn bad_request() -> MyError {
    MyError {
        err: "Bad Request".to_owned(),
        msg: Some("The request given is wrongly formatted or data was missing.".to_owned()),
        http_status_code: 400,
    }
}

#[catch(401)]
fn unauthorized() -> MyError {
    MyError {
        err: "Unauthorized".to_owned(),
        msg: Some("The authentication given was incorrect or insufficient.".to_owned()),
        http_status_code: 401,
    }
}

/// Create my custom response
///
/// Putting this in a separate function somewhere will resolve issues like
/// <https://github.com/GREsau/okapi/issues/57>
pub fn bad_request_response(gen: &mut OpenApiGenerator) -> okapi::openapi3::Response {
    let schema = gen.json_schema::<MyError>();
    okapi::openapi3::Response {
        description: "\
        # 400 Bad Request\n\
        The request given is wrongly formatted or data was missing. \
        "
        .to_owned(),
        content: okapi::map! {
            "application/json".to_owned() => MediaType {
                schema: Some(schema),
                ..Default::default()
            }
        },
        ..Default::default()
    }
}

pub fn unauthorized_response(gen: &mut OpenApiGenerator) -> okapi::openapi3::Response {
    let schema = gen.json_schema::<MyError>();
    okapi::openapi3::Response {
        description: "\
        # 401 Unauthorized\n\
        The authentication given was incorrect or insufficient. \
        "
        .to_owned(),
        content: okapi::map! {
            "application/json".to_owned() => MediaType {
                schema: Some(schema),
                ..Default::default()
            }
        },
        ..Default::default()
    }
}

impl<'r> Responder<'r, 'static> for MyError {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        // Convert object to json
        let body = serde_json::to_string(&self).unwrap();
        Response::build()
            .sized_body(body.len(), std::io::Cursor::new(body))
            .header(rocket::http::ContentType::JSON)
            .status(rocket::http::Status::new(self.http_status_code))
            .ok()
    }
}

impl OpenApiResponderInner for MyError {
    fn responses(gen: &mut OpenApiGenerator) -> Result<Responses, OpenApiError> {
        use okapi::openapi3::RefOr;
        Ok(Responses {
            responses: okapi::map! {
                "400".to_owned() => RefOr::Object(bad_request_response(gen)),
                "401".to_owned() => RefOr::Object(unauthorized_response(gen)),
            },
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::config::Config;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_okapi::openapi_get_spec;
    use serde_json::Value;

    #[test]
    fn secure_guard_spec_contains_no_auth() {
        let spec = openapi_get_spec![no_auth::no_special_auth];
        assert!(spec.paths.keys().any(|k| k.contains("/no_auth")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_contains_secure_routes_and_matches() {
        let figment = Config::figment().merge(("secret_key", vec![1u8; 64]));
        let rocket = rocket::custom(figment).mount(
            "/",
            openapi_get_routes![
                no_auth::no_special_auth,
                api_key::api_key,
                http_auth::http_auth,
                oauth2::oauth2_auth_code_get_user,
                open_id::open_id,
                cookies::cookie_auth,
            ],
        );
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/no_auth")));
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
