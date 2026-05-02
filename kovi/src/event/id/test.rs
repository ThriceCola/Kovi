use crate::event::id::ref_id::{RefID, RefIDInner};

use super::*;

#[test]
fn test_user_id() {
    let id = ID::new(123);
    assert_eq!(id.inner, IDInner::Int(123));

    let str = "test".to_string();
    let id = ID::new(str);
    assert_eq!(id.inner, IDInner::String("test".to_string()));
}

#[test]
fn test_id_eq_ref_id() {
    let id = ID::new(123);
    let ref_id = ref_id::RefID::new(&123);
    assert_eq!(id, ref_id);

    let str = "test".to_string();
    let id = ID::new(str.clone());
    let ref_id = ref_id::RefID::new(&str);
    assert_eq!(id, ref_id);

    let ref_str_id = ref_id::RefID {
        inner: ref_id::RefIDInner::String("test"),
    };
    assert_eq!(ID::new("test".to_string()), ref_str_id);
}

#[test]
fn test_id_ord_and_hash() {
    use std::collections::hash_map::DefaultHasher;

    assert!(ID::new(1) < ID::new(2));
    assert!(ID::new(1) < ID::new("1".to_string()));
    assert!(ID::new("a".to_string()) < ID::new("b".to_string()));

    let mut h1 = DefaultHasher::new();
    ID::new("test".to_string()).hash(&mut h1);

    let mut h2 = DefaultHasher::new();
    ID::new("test".to_string()).hash(&mut h2);

    assert_eq!(h1.finish(), h2.finish());
}

#[test]
fn test_comprehensive_id_refid() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Integer comparisons
    let id1 = ID::new(1);
    let id2 = ID::new(2);
    assert!(id1 < id2);
    assert_ne!(id1, id2);

    let rid1 = ref_id::RefID::new(&1);
    let rid2 = ref_id::RefID::new(&2);
    assert!(rid1 < rid2);
    assert_ne!(rid1, rid2);

    // Integer vs string are not equal
    let id_num = ID::new(1);
    let id_str = ID::new("1".to_string());
    assert_ne!(id_num, id_str);

    // String equality across owned and borrowed forms
    let s = "hello".to_string();
    let id_owned = ID::new(s.clone());
    let r_from_string = ref_id::RefID::new(&s);
    let r_from_str = ref_id::RefID::new("hello");

    assert_eq!(id_owned, r_from_string);
    assert_eq!(r_from_string, id_owned);
    assert_eq!(r_from_string, r_from_str);

    // Hash equality for RefId between &String and &str
    let mut h_a = DefaultHasher::new();
    r_from_string.hash(&mut h_a);
    let mut h_b = DefaultHasher::new();
    r_from_str.hash(&mut h_b);
    assert_eq!(h_a.finish(), h_b.finish());

    // Hash equality for Id is consistent
    let mut h1 = DefaultHasher::new();
    id_owned.hash(&mut h1);
    let mut h2 = DefaultHasher::new();
    ID::new("hello".to_string()).hash(&mut h2);
    assert_eq!(h1.finish(), h2.finish());

    // Ordering of RefId (string)
    let a = "a".to_string();
    let b = "b".to_string();
    let ra = ref_id::RefID::new(&a);
    let rb = ref_id::RefID::new(&b);
    assert!(ra < rb);

    // Ensure cross-type partial equality works both ways
    let id_x = ID::new("x".to_string());
    let ref_x = ref_id::RefID::new("x");
    assert_eq!(id_x, ref_x);
    assert_eq!(ref_x, id_x);

    // Ensure Hash/Ord/Eq consistency: equal items have equal hash and compare equal
    let mut h_id = DefaultHasher::new();
    id_x.hash(&mut h_id);
    let mut h_ref = DefaultHasher::new();
    ref_x.hash(&mut h_ref);
    // Hashes between Id and RefId may differ (different type/tag), but within-type are consistent.
    // We assert within-type consistency here.
    let mut h_ref_clone = DefaultHasher::new();
    ref_id::RefID::new("x").hash(&mut h_ref_clone);
    assert_eq!(h_ref.finish(), h_ref_clone.finish());

    // Check that different strings are ordered correctly
    assert!(ID::new("aa".to_string()) < ID::new("b".to_string()));
    assert!(ref_id::RefID::new("aa") < ref_id::RefID::new("b"));
}

#[test]
fn test_ref_user_id() {
    let id = RefID::new(&123);
    assert_eq!(id.inner, RefIDInner::Int(&123));

    let str = "test".to_string();
    let id = RefID::new(&str);
    assert_eq!(id.inner, RefIDInner::String(&str));

    let str_ref = RefID::new("test");
    assert_eq!(str_ref.inner, RefIDInner::String("test"));
    assert_eq!(id.inner, str_ref.inner);
}

#[test]
fn test_ref_id_eq_id() {
    let ref_id = RefID::new(&123);
    let id = super::ID::new(123);
    assert_eq!(ref_id, id);

    let str = "test".to_string();
    let ref_id = RefID::new(&str);
    let id = super::ID::new(str.clone());
    assert_eq!(ref_id, id);

    let str_ref_id = RefID::new("test");
    assert_eq!(str_ref_id, id);
}

#[test]
fn test_ref_id_ord_and_hash() {
    use std::collections::hash_map::DefaultHasher;

    assert!(RefID::new(&1) < RefID::new(&2));

    let a = "a".to_string();
    let b = "b".to_string();
    assert!(RefID::new(&a) < RefID::new(&b));
    assert_eq!(RefID::new(&a).cmp(&RefID::new("a")), Ordering::Equal);

    let mut h1 = DefaultHasher::new();
    RefID::new(&a).hash(&mut h1);

    let mut h2 = DefaultHasher::new();
    RefID::new("a").hash(&mut h2);

    assert_eq!(h1.finish(), h2.finish());
}
