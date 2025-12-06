use rocket::http::Status;
use rocket::{get, post, serde::json::Json};
use rocket_okapi::okapi::schemars;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// # Get data
#[openapi(tag = "Users")]
#[post("/get_date", data = "<req_body>")]
fn get_data(req_body: Json<String>) -> Option<Json<()>> {
    let _ = req_body;
    Some(Json(()))
}

#[openapi]
#[get("/paths/<path..>")]
fn path_info(path: PathBuf) -> (rocket::http::Status, String) {
    (rocket::http::Status::ImATeapot, format!("info {path:?}"))
}

#[openapi(tag = "Users")]
#[post("/user", data = "<req_body>", format = "application/json")]
fn create_user(req_body: Json<String>) -> Result<Json<User>, (Status, Json<ErrorMessage>)> {
    let _ = req_body;
    Ok(Json(User {
        name: "bob".to_owned(),
    }))
}

#[derive(Serialize, Deserialize, Debug, JsonSchema, Clone)]
pub struct User {
    name: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct ErrorMessage {
    pub message: String,
    pub code: u16,
}

#[rocket::main]
async fn main() {
    let launch_result = rocket::build()
        .mount("/", openapi_get_routes![get_data, path_info, create_user])
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

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_okapi::openapi_get_spec;
    use serde_json::Value;

    #[test]
    fn special_types_spec_contains_paths() {
        let spec = openapi_get_spec![get_data, path_info, create_user];
        assert!(spec
            .paths
            .keys()
            .any(|k| k.contains("/get_date") || k.contains("/paths")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_contains_special_types_present_and_match() {
        let rocket =
            rocket::build().mount("/", openapi_get_routes![get_data, path_info, create_user]);
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/get_date")));
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
