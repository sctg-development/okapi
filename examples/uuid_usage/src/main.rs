use rocket::{get, post, serde::json::Json, serde::uuid::Uuid};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct User {
    #[schemars(example = &"fdb12d51-0e3f-4ff8-821e-fbc255d8e413")]
    user_id: uuid::Uuid,
    username: String,
    #[schemars(example = &"test@example.com")]
    email: Option<String>,
}

#[allow(dead_code)]
fn example_email() -> &'static str {
    "test@example.com"
}
#[allow(dead_code)]
fn example_uuid() -> &'static str {
    "fdb12d51-0e3f-4ff8-821e-fbc255d8e413"
}

/// # Get all users
///
/// Returns all users in the system.
#[openapi(tag = "Users")]
#[get("/user")]
fn get_all_users() -> Json<Vec<User>> {
    Json(vec![User {
        user_id: uuid::Uuid::new_v4(),
        username: "bob".to_owned(),
        email: None,
    }])
}

/// # Get user
///
/// Returns a single user by ID.
#[openapi(tag = "Users")]
#[get("/user/<id>")]
fn get_user(id: Uuid) -> Option<Json<User>> {
    Some(Json(User {
        user_id: id,
        username: "bob".to_owned(),
        email: None,
    }))
}

/// # Get user by name
///
/// Returns a single user by username.
#[openapi(tag = "Users")]
#[get("/user_example?<user_id>&<name>&<email>")]
fn get_user_by_name(user_id: Uuid, name: String, email: Option<String>) -> Option<Json<User>> {
    Some(Json(User {
        user_id,
        username: name,
        email,
    }))
}

/// # Create user
#[openapi(tag = "Users")]
#[post("/user", data = "<user>")]
fn create_user(user: Json<User>) -> Json<User> {
    user
}

#[rocket::main]
async fn main() {
    let launch_result = rocket::build()
        .mount(
            "/",
            openapi_get_routes![get_all_users, get_user, get_user_by_name, create_user,],
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
    use rocket_okapi::settings::OpenApiSettings;
    use serde_json::Value;

    #[test]
    fn uuid_spec_contains_user_paths() {
        let settings = OpenApiSettings::default();
        let spec =
            openapi_get_spec![settings: get_all_users, get_user, get_user_by_name, create_user];
        assert!(spec.paths.keys().any(|k| k.contains("/user")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_contains_uuid_routes() {
        let rocket = rocket::build().mount(
            "/",
            openapi_get_routes![get_all_users, get_user, get_user_by_name, create_user],
        );
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/user")));
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
