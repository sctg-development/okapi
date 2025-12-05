use crate::get_add_operation_fn_name;
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::Parser, punctuated::Punctuated, token::Comma, Path, Result};

/// Parses routes and returns a function that takes `OpenApiSettings` and returns `OpenApi` spec.
pub fn create_openapi_spec(routes: TokenStream) -> Result<TokenStream2> {
    let paths = <Punctuated<Path, Comma>>::parse_terminated.parse(routes)?;
    let add_operations = create_add_operations(paths);
    Ok(quote! {
        |settings: &::rocket_okapi::settings::OpenApiSettings| -> ::rocket_okapi::okapi::openapi3::OpenApi {
            let mut gen = ::rocket_okapi::gen::OpenApiGenerator::new(settings);
            #add_operations
            let mut spec = gen.into_openapi();
            let mut info = ::rocket_okapi::okapi::openapi3::Info {
                title: env!("CARGO_PKG_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                ..Default::default()
            };
            if !env!("CARGO_PKG_DESCRIPTION").is_empty() {
                info.description = Some(env!("CARGO_PKG_DESCRIPTION").to_owned());
            }
            if !env!("CARGO_PKG_REPOSITORY").is_empty() {
                info.contact = Some(::rocket_okapi::okapi::openapi3::Contact{
                    name: Some("Repository".to_owned()),
                    url: Some(env!("CARGO_PKG_REPOSITORY").to_owned()),
                    ..Default::default()
                });
            }
            if !env!("CARGO_PKG_HOMEPAGE").is_empty() {
                info.contact = Some(::rocket_okapi::okapi::openapi3::Contact{
                    name: Some("Homepage".to_owned()),
                    url: Some(env!("CARGO_PKG_HOMEPAGE").to_owned()),
                    ..Default::default()
                });
            }
            spec.info = info;

            spec
        }
    })
}

pub(crate) fn create_openapi_spec_ts(routes: TokenStream2) -> Result<TokenStream2> {
    let paths = <Punctuated<Path, Comma>>::parse_terminated.parse2(routes)?;
    let add_operations = create_add_operations(paths);
    Ok(quote! {
        |settings: &::rocket_okapi::settings::OpenApiSettings| -> ::rocket_okapi::okapi::openapi3::OpenApi {
            let mut gen = ::rocket_okapi::gen::OpenApiGenerator::new(settings);
            #add_operations
            let mut spec = gen.into_openapi();
            let mut info = ::rocket_okapi::okapi::openapi3::Info {
                title: env!("CARGO_PKG_NAME").to_owned(),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                ..Default::default()
            };
            if !env!("CARGO_PKG_DESCRIPTION").is_empty() {
                info.description = Some(env!("CARGO_PKG_DESCRIPTION").to_owned());
            }
            if !env!("CARGO_PKG_REPOSITORY").is_empty() {
                info.contact = Some(::rocket_okapi::okapi::openapi3::Contact{
                    name: Some("Repository".to_owned()),
                    url: Some(env!("CARGO_PKG_REPOSITORY").to_owned()),
                    ..Default::default()
                });
            }
            if !env!("CARGO_PKG_HOMEPAGE").is_empty() {
                info.contact = Some(::rocket_okapi::okapi::openapi3::Contact{
                    name: Some("Homepage".to_owned()),
                    url: Some(env!("CARGO_PKG_HOMEPAGE").to_owned()),
                    ..Default::default()
                });
            }
            spec.info = info;

            spec
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::TokenStream as TS2;
    use quote::quote;
    use syn::parse_str;

    #[test]
    fn test_operation_id_and_fn_name_for_add_operation() {
        let path: syn::Path = parse_str("crate::module::do_stuff").unwrap();
        let fn_name = fn_name_for_add_operation(path.clone());
        assert!(fn_name
            .segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .starts_with("okapi_add_operation_for_do_stuff"));
        let id = operation_id(&path);
        assert_eq!(id, "crate_module_do_stuff");
    }

    #[test]
    fn test_create_add_operations_tokens() {
        let mut paths: syn::punctuated::Punctuated<syn::Path, syn::token::Comma> =
            syn::punctuated::Punctuated::new();
        paths.push(parse_str::<syn::Path>("crate::a").unwrap());
        paths.push(parse_str::<syn::Path>("crate::b").unwrap());
        let tokens = create_add_operations(paths);
        let out = tokens.to_string();
        assert!(out.contains("okapi_add_operation_for_a"));
        assert!(out.contains("okapi_add_operation_for_b"));
    }

    #[test]
    fn test_create_openapi_spec_ts_generates_closure() {
        let ts: TS2 = quote!(crate::a);
        let tokens = create_openapi_spec_ts(ts).expect("should generate spec closure");
        let out = tokens.to_string();
        assert!(out.len() > 0);
        assert!(out.contains("spec"));
    }
}

fn create_add_operations(paths: Punctuated<Path, Comma>) -> TokenStream2 {
    let function_calls = paths.into_iter().map(|path| {
        let fn_name = fn_name_for_add_operation(path.clone());

        let operation_id = operation_id(&path);
        quote! {
            #fn_name(&mut gen, #operation_id.to_owned())
                .expect(&format!("Could not generate OpenAPI operation for `{}`.", stringify!(#path)));
        }
    });
    quote! {
        #(#function_calls)*
    }
}

fn fn_name_for_add_operation(mut fn_path: Path) -> Path {
    let last_seg = fn_path.segments.last_mut().expect("syn::Path has segments");
    last_seg.ident = get_add_operation_fn_name(&last_seg.ident);
    fn_path
}

fn operation_id(fn_path: &Path) -> String {
    let idents: Vec<String> = fn_path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    idents.join("_")
}
