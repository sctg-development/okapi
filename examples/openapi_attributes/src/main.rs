use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::{get, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub struct User {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use serde_json::Value;

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_json_present_and_paths_match() {
        let rocket = rocket::build().mount("/", openapi_get_routes![list_users]);
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/users")));
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

#[openapi(tag = "Users", ignore = "db")]
#[get("/users")]
fn list_users(db: CustomDB) -> Json<Vec<User>> {
    Json(db.users)
}

// VERY simple fake read only database
#[derive(Clone, Default)]
struct CustomDB {
    users: Vec<User>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CustomDB {
    type Error = String;

    async fn from_request(_request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        Outcome::Success(CustomDB {
            users: vec![User {
                name: "Bob".to_owned(),
            }],
        })
    }
}

#[rocket::main]
async fn main() {
    let launch_result = rocket::build()
        .mount("/", openapi_get_routes![list_users])
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
        .launch()
        .await;
    match launch_result {
        Ok(_) => println!("Rocket shut down gracefully."),
        Err(err) => println!("Rocket had an error: {err}"),
    };
}
