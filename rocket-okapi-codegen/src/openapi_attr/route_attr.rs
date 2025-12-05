use darling::ast::NestedMeta as DarlingNestedMeta;
use darling::{Error, FromMeta};
use proc_macro::TokenStream;
use quote::ToTokens;
use quote::{quote, quote_spanned};
use rocket_http::{ext::IntoOwned, uri::Origin, MediaType, Method};
use std::str::FromStr;
use syn::ext::IdentExt;
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{Attribute, Meta, MetaList};

#[derive(Debug)]
pub struct Route {
    pub method: Method,
    pub origin: Origin<'static>,
    #[allow(dead_code)]
    pub media_type: Option<MediaType>,
    pub data_param: Option<String>,
}

impl Route {
    pub fn path_params(&self) -> impl Iterator<Item = &str> {
        self.origin.path().segments().filter_map(|s| {
            if s.starts_with('<') && s.ends_with('>') && !s.ends_with("..>") {
                Some(&s[1..s.len() - 1])
            } else {
                None
            }
        })
    }

    pub fn path_multi_param(&self) -> Option<&str> {
        self.origin.path().segments().find_map(|s| {
            if s.starts_with('<') && s.ends_with("..>") {
                Some(&s[1..s.len() - 3])
            } else {
                None
            }
        })
    }

    pub fn query_params(&self) -> impl Iterator<Item = &str> {
        let mut query_params: Vec<&str> = vec![];
        if let Some(query) = self.origin.query() {
            query_params = query.as_str().split('&').collect();
            query_params = query_params
                .into_iter()
                .filter_map(|s| {
                    if s.starts_with('<') && s.ends_with('>') && !s.ends_with("..>") {
                        Some(&s[1..s.len() - 1])
                    } else {
                        None
                    }
                })
                .collect();
        }
        query_params.into_iter()
    }

    pub fn query_multi_params(&self) -> impl Iterator<Item = &str> {
        let mut query_params: Vec<&str> = vec![];
        if let Some(query) = self.origin.query() {
            query_params = query.as_str().split('&').collect();
            query_params = query_params
                .into_iter()
                .filter_map(|s| {
                    if s.starts_with('<') && s.ends_with("..>") {
                        Some(&s[1..s.len() - 3])
                    } else {
                        None
                    }
                })
                .collect();
        }
        query_params.into_iter()
    }
}

#[derive(Debug)]
struct OriginMeta(Origin<'static>);
#[derive(Debug)]
struct MediaTypeMeta(MediaType);
#[derive(Debug)]
struct MethodMeta(Method);

impl FromMeta for OriginMeta {
    fn from_string(value: &str) -> Result<Self, Error> {
        match Origin::parse_route(value) {
            Ok(o) => Ok(OriginMeta(o.into_owned())),
            Err(e) => Err(Error::unsupported_format(&e.to_string())),
        }
    }
}

impl FromMeta for MediaTypeMeta {
    fn from_string(value: &str) -> Result<Self, Error> {
        match MediaType::parse_flexible(value) {
            Some(m) => Ok(MediaTypeMeta(m)),
            None => Err(Error::unsupported_format(&format!(
                "Unknown media type: '{}'",
                value
            ))),
        }
    }
}

impl FromMeta for MethodMeta {
    fn from_string(value: &str) -> Result<Self, Error> {
        match Method::from_str(value) {
            Ok(m) => Ok(MethodMeta(m)),
            Err(()) => {
                // Special handling for paths that might be incorrectly parsed as methods
                // This helps with rust-analyzer false positives for protect_* macros
                if value.starts_with('/') {
                    Err(Error::unsupported_format(
                        "Expected HTTP method but found a path. This might be a rust-analyzer parsing issue with protect_* macros."
                    ))
                } else {
                    Err(Error::unsupported_format(&format!(
                        "Unknown HTTP method: '{}'",
                        value
                    )))
                }
            }
        }
    }
}

#[derive(Debug, FromMeta)]
#[darling(allow_unknown_fields)]
struct RouteAttributeNamedMeta {
    path: OriginMeta,
    #[darling(default)]
    format: Option<MediaTypeMeta>,
    #[darling(default)]
    data: Option<String>,
}

#[derive(Debug, FromMeta)]
#[darling(allow_unknown_fields)]
struct MethodRouteAttributeNamedMeta {
    #[darling(default)]
    format: Option<MediaTypeMeta>,
    #[darling(default)]
    data: Option<String>,
}

fn parse_route_attr(args: &[DarlingNestedMeta]) -> Result<Route, Error> {
    if args.is_empty() {
        return Err(Error::too_few_items(1));
    }

    // Defensive check: if the first argument looks like a path, this might be a protect_* macro
    // being incorrectly parsed. This helps with rust-analyzer false positives.
    if let Some(DarlingNestedMeta::Lit(syn::Lit::Str(lit_str))) = args.first() {
        let value = lit_str.value();
        if value.starts_with('/') {
            return Err(Error::unsupported_format(
                "This appears to be a protect_* macro being parsed incorrectly. This is likely a rust-analyzer issue and should not affect compilation."
            ));
        }
    }

    let method = MethodMeta::from_nested_meta(&args[0])?;
    let named = RouteAttributeNamedMeta::from_list(&args[1..])?;
    Ok(Route {
        method: method.0,
        origin: named.path.0,
        media_type: named.format.map(|x| x.0),
        data_param: named.data.map(trim_angle_brackers),
    })
}

fn parse_method_route_attr(method: Method, args: &[DarlingNestedMeta]) -> Result<Route, Error> {
    if args.is_empty() {
        return Err(Error::too_few_items(1));
    }
    let origin = OriginMeta::from_nested_meta(&args[0])?;
    let named = MethodRouteAttributeNamedMeta::from_list(&args[1..])?;
    Ok(Route {
        method,
        origin: origin.0,
        media_type: named.format.map(|x| x.0),
        data_param: named.data.map(trim_angle_brackers),
    })
}

fn trim_angle_brackers(mut s: String) -> String {
    if s.starts_with('<') && s.ends_with('>') {
        s.pop();
        s.remove(0);
    }
    s
}

fn parse_attr(name: &str, args: &[DarlingNestedMeta]) -> Result<Route, Error> {
    // Handle protect_* methods by extracting the underlying HTTP method
    if let Some(method_str) = name.strip_prefix("protect_") {
        match Method::from_str(method_str) {
            Ok(method) => parse_method_route_attr(method, args),
            Err(()) => {
                return Err(Error::unsupported_format(&format!(
                    "Unknown HTTP method in protect macro: '{}'",
                    method_str
                )))
            }
        }
    } else {
        match Method::from_str(name) {
            Ok(method) => parse_method_route_attr(method, args),
            Err(()) => parse_route_attr(args),
        }
    }
}

fn is_route_attribute(a: &Attribute) -> bool {
    a.path().is_ident("get")
        || a.path().is_ident("put")
        || a.path().is_ident("post")
        || a.path().is_ident("delete")
        || a.path().is_ident("options")
        || a.path().is_ident("head")
        || a.path().is_ident("trace")
        || a.path().is_ident("connect")
        || a.path().is_ident("patch")
        || a.path().is_ident("route")
        || a.path().is_ident("protect_get")
        || a.path().is_ident("protect_put")
        || a.path().is_ident("protect_post")
        || a.path().is_ident("protect_delete")
        || a.path().is_ident("protect_patch")
        || a.path().is_ident("protect_options")
}

fn extract_inner_args_string(attr: &Attribute) -> Option<String> {
    // Convert attribute meta to a token string and extract content inside parentheses
    let s = attr.meta.to_token_stream().to_string();
    if let Some(start) = s.find('(') {
        if let Some(end) = s.rfind(')') {
            return Some(s[start + 1..end].to_string());
        }
    }
    None
}

fn parse_args_string_to_parts(s: &str) -> Vec<String> {
    // Split on commas at top-level, respecting strings inside quotes
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut escape = false;
    for c in s.chars() {
        if escape {
            current.push(c);
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            current.push(c);
            continue;
        }
        if c == '"' {
            in_quotes = !in_quotes;
            current.push(c);
            continue;
        }
        if c == ',' && !in_quotes {
            parts.push(current.trim().to_string());
            current.clear();
        } else {
            current.push(c);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

fn parse_attr_from_attr(attr: &Attribute) -> Result<Route, Error> {
    let name = attr
        .path()
        .get_ident()
        .map(|id| id.to_string())
        .unwrap_or_default();
    let args_str = extract_inner_args_string(attr).unwrap_or_default();
    let parts = parse_args_string_to_parts(&args_str);
    // Simple parsing rules: first positional argument that's a string is the path
    let mut path: Option<String> = None;
    let mut media_type: Option<MediaType> = None;
    let mut data_param: Option<String> = None;
    for part in parts.iter() {
        if part.starts_with('"') && part.ends_with('"') {
            if path.is_none() {
                path = Some(part.trim_matches('"').to_string());
                continue;
            }
        }
        if let Some(rest) = part.strip_prefix("format =") {
            let val = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            match MediaType::parse_flexible(val) {
                Some(m) => media_type = Some(m),
                None => {
                    return Err(Error::unsupported_format(&format!(
                        "Unknown media type: '{}'",
                        val
                    )))
                }
            }
            continue;
        }
        if let Some(rest) = part.strip_prefix("data =") {
            let val = rest.trim().trim_matches(|c| c == '"' || c == '\'');
            data_param = Some(val.to_string());
            continue;
        }
    }
    // Method
    if let Some(method) = name.strip_prefix("protect_") {
        // protect_* macro
        match Method::from_str(method) {
            Ok(m) => {
                let origin = match path {
                    Some(p) => Origin::parse_route(&p)
                        .map(|o| o.into_owned())
                        .map_err(|e| Error::unsupported_format(&e.to_string()))?,
                    None => return Err(Error::too_few_items(1)),
                };
                return Ok(Route {
                    method: m,
                    origin,
                    media_type,
                    data_param: data_param.map(trim_angle_brackers),
                });
            }
            Err(()) => {
                return Err(Error::unsupported_format(&format!(
                    "Unknown HTTP method in protect macro: '{}'",
                    method
                )))
            }
        }
    } else if name == "route" {
        // route macro: first arg could be method string? Not handling for now.
        return Err(Error::unsupported_format(
            "'route' attribute parsing not implemented",
        ));
    } else {
        match Method::from_str(&name) {
            Ok(m) => {
                let origin = match path {
                    Some(p) => Origin::parse_route(&p)
                        .map(|o| o.into_owned())
                        .map_err(|e| Error::unsupported_format(&e.to_string()))?,
                    None => return Err(Error::too_few_items(1)),
                };
                return Ok(Route {
                    method: m,
                    origin,
                    media_type,
                    data_param: data_param.map(trim_angle_brackers),
                });
            }
            Err(()) => {
                return Err(Error::unsupported_format(&format!(
                    "Unknown HTTP method: '{}'",
                    name
                )))
            }
        }
    }
}

pub(crate) fn parse_attrs<'a>(
    attrs: impl IntoIterator<Item = &'a Attribute>,
) -> Result<Route, TokenStream> {
    match attrs.into_iter().find(|a| is_route_attribute(a)) {
        Some(attr) => {
            let span = attr.span();
            parse_attr_from_attr(attr)
                .map_err(|e| e.with_span(&attr).write_errors().into())
        }
        None => Err(quote! {
                compile_error!("Could not find Rocket route attribute. Ensure the #[openapi] attribute is placed *before* the Rocket route attribute.");
            }.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darling::Error as DarlingError;
    use syn::parse_str;

    #[test]
    fn test_parse_args_string_to_parts_basic() {
        let s = "\"/user/<id>?<q>\", format = \"application/json\", data = \"<a>\"";
        let parts = parse_args_string_to_parts(s);
        assert_eq!(parts.len(), 3);
        assert!(parts.iter().any(|p| p.contains("/user/<id>")));
        assert!(parts.iter().any(|p| p.contains("format")));
        assert!(parts.iter().any(|p| p.contains("data")));
    }

    #[test]
    fn test_extract_inner_args_string() {
        let item: syn::ItemFn = parse_str("#[get(\"/a\")] fn f() {} ").unwrap();
        let attr = item.attrs.first().unwrap();
        let out = extract_inner_args_string(&attr).unwrap();
        assert_eq!(out, "\"/a\"");
    }

    #[test]
    fn test_is_route_attribute_get_and_protect() {
        let a: syn::ItemFn = parse_str("#[get(\"/a\")] fn f() {} ").unwrap();
        let a_attr = a.attrs.first().unwrap();
        assert!(is_route_attribute(&a_attr));
        let b: syn::ItemFn = parse_str("#[protect_get(\"/a\")] fn f() {} ").unwrap();
        let b_attr = b.attrs.first().unwrap();
        assert!(is_route_attribute(&b_attr));
    }

    #[test]
    fn test_parse_attr_from_attr_get_success() {
        let a: syn::ItemFn = parse_str("#[get(\"/user/<id>?<q>\")] fn f() {} ").unwrap();
        let a_attr = a.attrs.first().unwrap();
        let r = parse_attr_from_attr(&a_attr).unwrap();
        assert_eq!(r.method, Method::Get);
        assert!(r.origin.path().as_str().contains("/user/<id>"));
        assert!(r.path_params().any(|p| p == "id"));
        assert!(r.query_params().any(|p| p == "q") || r.query_params().any(|p| p == "<q>"));
    }

    #[test]
    fn test_parse_attr_from_attr_protect_get_data_trim() {
        let a: syn::ItemFn =
            parse_str("#[protect_get(\"/api/<a>\", data = \"<param>\")] fn f() {} ").unwrap();
        let a_attr = a.attrs.first().unwrap();
        let r = parse_attr_from_attr(&a_attr).unwrap();
        assert_eq!(r.method, Method::Get);
        assert_eq!(r.data_param.as_deref(), Some("param"));
    }

    #[test]
    fn test_parse_attr_from_attr_invalid_method() {
        let a: syn::ItemFn = parse_str("#[unknown(\"/a\")] fn f() {} ").unwrap();
        let a_attr = a.attrs.first().unwrap();
        let err = parse_attr_from_attr(&a_attr).unwrap_err();
        // Should map to DarlingError
        assert!(err
            .to_string()
            .to_lowercase()
            .contains("unknown http method"));
    }
}
