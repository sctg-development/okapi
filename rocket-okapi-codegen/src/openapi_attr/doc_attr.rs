use syn::{Attribute, Lit::Str, Meta::NameValue, MetaNameValue, Meta};
use syn::ext::IdentExt;

pub fn get_title_and_desc_from_doc(attrs: &[Attribute]) -> (Option<String>, Option<String>) {
    let doc = match get_doc(attrs) {
        None => return (None, None),
        Some(doc) => doc,
    };

    if doc.starts_with('#') {
        let mut split = doc.splitn(2, '\n');
        let title = split
            .next()
            .unwrap()
            .trim_start_matches('#')
            .trim()
            .to_owned();
        let maybe_desc = split.next().and_then(merge_description_lines);
        (none_if_empty(title), maybe_desc)
    } else {
        (None, merge_description_lines(&doc))
    }
}

fn merge_description_lines(doc: &str) -> Option<String> {
    let desc = doc
        .trim()
        .split("\n\n")
        .filter_map(|line| none_if_empty(line.trim().replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n\n");
    none_if_empty(desc)
}

fn get_doc(attrs: &[Attribute]) -> Option<String> {
    let doc = attrs
        .iter()
        .filter_map(|attr| {
            if !attr.path().is_ident("doc") {
                return None;
            }
            let meta = &attr.meta;
            if let NameValue(MetaNameValue { value: syn::Expr::Lit(expr_lit), .. }) = meta {
                if let syn::Lit::Str(s) = &expr_lit.lit {
                    return Some(s.value());
                }
            }

            None
        })
        .collect::<Vec<_>>()
        .iter()
        .flat_map(|a| a.split('\n'))
        .map(str::trim)
        .skip_while(|s| s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    none_if_empty(doc)
}

fn none_if_empty(s: String) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_str;

    #[test]
    fn test_get_title_and_desc_from_doc_markdown() {
        let item: syn::ItemFn = parse_str("#[doc = \"# Title\\n\\nSome description\"] fn f() {} ").unwrap();
        let (title, desc) = get_title_and_desc_from_doc(&item.attrs);
        assert_eq!(title.as_deref(), Some("Title"));
        assert!(desc.unwrap().contains("Some description"));
    }

    #[test]
    fn test_get_title_and_desc_from_doc_description_only() {
        let item: syn::ItemFn = parse_str("#[doc = \"First line\\n\\nSecond paragraph\"] fn f() {} ").unwrap();
        let (title, desc) = get_title_and_desc_from_doc(&item.attrs);
        assert!(title.is_none());
        assert!(desc.unwrap().contains("First line"));
    }

    #[test]
    fn test_none_if_empty_return_none() {
        assert!(none_if_empty("".to_owned()).is_none());
        // Whitespace only is considered non-empty by `none_if_empty` implementation
        assert!(none_if_empty("   ".to_owned()).is_some());
        assert!(none_if_empty(" content ".to_owned()).is_some());
    }
}
