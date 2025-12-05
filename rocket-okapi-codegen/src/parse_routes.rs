use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::Parser, punctuated::Punctuated, token::Comma, Path, Result};

/// Parses routes and returns a function that takes `OpenApi` and `OpenApiSettings` and
/// returns `Vec<rocket::Route>`.
/// It optionally adds the `openapi.json` route to the list of routes.
pub fn parse_routes(routes: TokenStream) -> Result<TokenStream2> {
    // Convert to proc_macro2 TokenStream and forward to the helper so unit tests can use proc_macro2
    let ts2: TokenStream2 = routes.into();
    parse_routes_ts(ts2)
}

pub(crate) fn parse_routes_ts(routes: TokenStream2) -> Result<TokenStream2> {
    let paths = <Punctuated<Path, Comma>>::parse_terminated.parse2(routes)?;
    // This returns a function so the spec does not have to be generated multiple times.
    Ok(quote! {
        |spec_opt: Option<::rocket_okapi::okapi::openapi3::OpenApi>, settings: &::rocket_okapi::settings::OpenApiSettings|
            -> Vec<::rocket::Route> {
                let mut routes = ::rocket::routes![#paths];
                if let Some(spec) = spec_opt {
                    routes.push(
                        ::rocket_okapi::handlers::OpenApiHandler::new(spec)
                            .into_route(&settings.json_path)
                    );
                }
                routes
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream as PMTokenStream;
    use quote::quote;

    #[test]
    fn test_parse_routes_basic() {
        let ts: PMTokenStream = quote!(crate::a, crate::b);
        let tokens = parse_routes_ts(ts).expect("parse routes ok");
        let out = tokens.to_string();
        assert!(out.contains("into_route") || out.contains("OpenApiHandler"));
    }
}
