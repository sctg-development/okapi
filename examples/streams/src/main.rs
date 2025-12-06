//! Note streams are treated in a special way at the moment.
//! So this might mean that if the return type is different it might result in errors.
//! This might be incontinent but there is also a workaround listed below.
//!
//! You can not use the Rocker provided macro's inside the return types because these
//! macro's might generated a lot of code and not just a type.

use rocket::futures::stream::Stream;
use rocket::get;
use rocket::response::stream::{ByteStream, Event, EventStream, ReaderStream, TextStream};
use rocket::tokio::fs::File;
use rocket::tokio::time::{self, Duration};
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use tracing_subscriber::EnvFilter;

#[openapi]
#[get("/event_stream")]
// Same return type as: `EventStream![]`
fn event_stream() -> EventStream<impl Stream<Item = Event>> {
    EventStream! {
        let mut interval = time::interval(Duration::from_secs(1));
        loop {
            yield Event::data("ping");
            interval.tick().await;
        }
    }
}

#[openapi]
#[get("/byte_stream")]
// Same return type as: `ByteStream![&'static [u8]]`
fn byte_stream() -> ByteStream<impl Stream<Item = &'static [u8]>> {
    ByteStream(rocket::futures::stream::repeat(&[1, 2, 3][..]))
}

#[openapi]
#[get("/reader_stream")]
// Same return type as: `ReaderStream![File]`
fn reader_stream() -> ReaderStream<impl Stream<Item = File>> {
    ReaderStream! {
        let paths = &["README.md"];
        for path in paths {
            if let Ok(file) = File::open(path).await {
                yield file;
            }
        }
    }
}

#[openapi]
#[get("/text_stream")]
// Same return type as: `TextStream![&'static str]`
fn text_stream() -> TextStream<impl Stream<Item = &'static str>> {
    TextStream(rocket::futures::stream::repeat("hi"))
}

/// This function skips the Okapi spec entirely.
/// So this will always allow you to use all Rocket functionally.
/// Even when OpenAPI gives you compile errors. (you can still report errors so we can fix them)
///
/// Consider this a general workaround.
#[get("/undocumented_stream")]
async fn stream_one() -> std::io::Result<ReaderStream![File]> {
    let file = File::open("README.md").await?;
    Ok(ReaderStream::one(file))
}

#[rocket::main]
async fn main() {
    // Initialize tracing subscriber so RUST_LOG controls logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    let launch_result = rocket::build()
        .mount(
            "/",
            openapi_get_routes![event_stream, byte_stream, reader_stream, text_stream],
        )
        // Skip Okapi parser to prevent compile errors.
        .mount("/", rocket::routes![stream_one])
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
    fn streams_spec_contains_routes() {
        let spec = openapi_get_spec![event_stream, byte_stream, reader_stream, text_stream];
        assert!(spec
            .paths
            .keys()
            .any(|k| k.contains("/event_stream") || k.contains("/byte_stream")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_matches_stream_routes() {
        let rocket = rocket::build()
            .mount(
                "/",
                openapi_get_routes![event_stream, byte_stream, reader_stream, text_stream],
            )
            .mount("/", rocket::routes![stream_one]);
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/event_stream")));
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
