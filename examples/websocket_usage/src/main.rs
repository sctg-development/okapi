use rocket::futures::{SinkExt, StreamExt};
use rocket::get;
use rocket::response::content::RawHtml;
use rocket_okapi::settings::UrlObject;
use rocket_okapi::{openapi, openapi_get_routes, rapidoc::*, swagger_ui::*};
use tracing_subscriber::EnvFilter;

#[openapi]
#[get("/")]
fn test_websocket() -> RawHtml<&'static str> {
    RawHtml(
        r#"
        <!DOCTYPE html>
        <html>
            <body>
                Echo: <input type="text" id="echo_text" name="echo" size="10" />
                <input type="button" value="Send" onclick="echo_send()" />
                <br/>
                <br/>
                <p id="output"><p>
                <script>
                    // Create WebSocket connection.
                    const hello_socket = new WebSocket("ws://localhost:8000/hello/bob");
                    const echo_socket = new WebSocket("ws://localhost:8000/echo");
                    const output = document.getElementById('output');
                    
                    // Listen for messages
                    hello_socket.addEventListener("message", (event) => {
                        console.log("Hello response: ", event.data);
                        output.innerHTML += "Hello response: " + event.data + "<br/>";
                    });
                    echo_socket.addEventListener("message", (event) => {
                        console.log("Echo response: ", event.data);
                        output.innerHTML += "Echo response: " + event.data + "<br/>";
                    });

                    function echo_send(){
                        echo_socket.send(document.getElementById('echo_text').value);
                    }
                </script>
            </body>
        </html>
        "#,
    )
}

#[openapi]
#[get("/hello/<name>")]
fn hello(ws: rocket_ws::WebSocket, name: &str) -> rocket_ws::Channel<'_> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            let message = format!("Hello, {name}!");
            let _ = stream.send(message.into()).await;
            Ok(())
        })
    })
}

#[openapi]
#[get("/echo")]
fn echo(ws: rocket_ws::WebSocket) -> rocket_ws::Channel<'static> {
    ws.channel(move |mut stream| {
        Box::pin(async move {
            while let Some(message) = stream.next().await {
                let _ = stream.send(message?).await;
            }

            Ok(())
        })
    })
}

#[rocket::main]
async fn main() {
    // Initialize tracing subscriber so RUST_LOG controls logging
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
    let launch_result = rocket::build()
        .mount("/", openapi_get_routes![test_websocket, hello, echo,])
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
                    allow_spec_file_download: true,
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
    fn websocket_spec_contains_echo_and_hello() {
        let spec = openapi_get_spec![test_websocket, hello, echo];
        assert!(spec.paths.keys().any(|k| k.contains("/echo")));
        assert!(spec.paths.keys().any(|k| k.contains("/hello")));
    }

    async fn fetch_openapi_spec(client: &Client, path: &str) -> Value {
        let response = client.get(path).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response.into_string().await.expect("body string");
        serde_json::from_str(&body).expect("valid json")
    }

    #[rocket::async_test]
    async fn server_openapi_contains_websocket_routes() {
        let rocket = rocket::build().mount("/", openapi_get_routes![test_websocket, hello, echo]);
        let client = Client::tracked(rocket).await.expect("client");
        let spec = fetch_openapi_spec(&client, "/openapi.json").await;
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/echo")));
        assert!(spec["paths"]
            .as_object()
            .unwrap()
            .keys()
            .any(|k| k.contains("/hello")));
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
