use okapi::merge::*;
use okapi::openapi3::*;
use okapi::Map;

#[test]
fn test_merge_string() {
    assert_eq!(merge_string("", "b"), "b");
    assert_eq!(merge_string("a", "b"), "a");
}

#[test]
fn test_merge_opt_string() {
    let mut opt1: Option<String> = None;
    let opt2: Option<String> = Some("b".to_owned());
    merge_opt_string(&mut opt1, &opt2);
    assert_eq!(opt1.as_deref(), Some("b"));

    let mut opt3: Option<String> = Some("a".to_owned());
    let opt4: Option<String> = Some("b".to_owned());
    merge_opt_string(&mut opt3, &opt4);
    assert_eq!(opt3.as_deref(), Some("a"));
}

#[test]
fn test_merge_option() {
    let mut opt1: Option<i32> = None;
    merge_option(&mut opt1, &Some(1));
    assert_eq!(opt1, Some(1));

    let mut opt2: Option<i32> = Some(2);
    merge_option(&mut opt2, &Some(3));
    assert_eq!(opt2, Some(2));
}

#[test]
fn test_merge_vec() {
    let mut a = vec![1, 2];
    let b = vec![3, 4];
    merge_vec(&mut a, &b);
    assert_eq!(a, vec![1, 2, 3, 4]);
}

#[test]
fn test_merge_map_and_map_keys() {
    let mut m1: Map<String, i32> = Map::new();
    m1.insert("a".to_owned(), 1);
    let mut m2: Map<String, i32> = Map::new();
    m2.insert("b".to_owned(), 2);
    merge_map(&mut m1, &m2, "test");
    assert_eq!(m1.get("a"), Some(&1));
    assert_eq!(m1.get("b"), Some(&2));

    // Conflicting key: m1 keeps its value
    let mut m3: Map<String, i32> = Map::new();
    m3.insert("a".to_owned(), 3);
    merge_map(&mut m1, &m3, "test");
    assert_eq!(m1.get("a"), Some(&1));
}

#[test]
fn test_merge_tags_and_tag_merge() {
    let t1 = Tag {
        name: "x".to_owned(),
        description: Some("a".to_owned()),
        ..Default::default()
    };
    let t2 = Tag {
        name: "x".to_owned(),
        description: Some("b".to_owned()),
        ..Default::default()
    };
    let mut s1 = vec![t1.clone()];
    let s2 = vec![t2.clone()];
    let merged = merge_tags(&mut s1, &s2).unwrap();
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].name, "x");
    // Description should keep from s1 (the first)
    assert_eq!(merged[0].description.as_deref(), Some("a"));
}

#[test]
fn test_merge_paths_prefix_and_methods() {
    let mut p1: Map<String, PathItem> = Map::new();
    let mut pi1 = PathItem::default();
    pi1.get = Some(Operation {
        responses: Responses::default(),
        ..Default::default()
    });
    // Use a path with the prefix already included so merge can find an existing entry
    p1.insert("/base/x".to_owned(), pi1);

    let mut p2: Map<String, PathItem> = Map::new();
    let mut pi2 = PathItem::default();
    pi2.post = Some(Operation {
        responses: Responses::default(),
        ..Default::default()
    });
    p2.insert("/x".to_owned(), pi2);

    merge_paths(&mut p1, &"/base/", &p2).unwrap();
    // Merged key should be /base/x
    assert!(p1.contains_key("/base/x"));
    let merged_item = p1.get("/base/x").unwrap();
    assert!(merged_item.get.is_some());
    assert!(merged_item.post.is_some());
}

#[test]
fn test_merge_specs_combine() {
    let mut s1 = OpenApi::new();
    s1.info.title = "Spec1".to_owned();
    s1.tags.push(Tag {
        name: "a".to_owned(),
        ..Default::default()
    });

    let mut s2 = OpenApi::new();
    s2.info.title = "Spec2".to_owned();
    s2.tags.push(Tag {
        name: "b".to_owned(),
        ..Default::default()
    });

    merge_specs(&mut s1, &"/", &s2).unwrap();
    // s1 title should be kept
    assert_eq!(s1.info.title, "Spec1");
    // tags length should match
    assert_eq!(s1.tags.len(), 2);
}
