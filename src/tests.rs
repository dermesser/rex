#![cfg(test)]

//! A general test suite aiming for wide coverage of positive and negative matches.

fn match_re(re: &str, s: &str) -> (bool, Vec<(usize, usize)>) {
    crate::match_re_str(re, s).unwrap()
}

#[test]
fn test_notorious_graph() {
    println!("{}", crate::render_graph("(x+x+)+y"));
}

#[test]
fn test_simple_repeat() {
    assert!(match_re("a+", "aaa").0);
    assert!(match_re("aaa+", "aaa").0);
    assert!(match_re("aa(a+)", "aaa").0);
    assert_eq!(vec![(0, 3), (2, 3)], match_re("aa(a+)", "aaa").1);
    assert_eq!(vec![(0, 3)], match_re("aaa+", "aaabcde").1);
    assert!(!match_re("a+", "").0);
    assert!(!match_re("aa+$", "aaabc").0);
}

#[test]
fn test_specific_repeat() {
    assert!(match_re("a{1,3}", "a").0);
    assert!(match_re("a{1,3}", "aa").0);
    assert!(match_re("a{1,3}", "aaa").0);
    assert!(match_re("a{1,3}", "aaaa").0);
    assert!(!match_re("a{1,3}$", "aaaa").0);
    assert_eq!(3, match_re("a{1,3}", "aaaa").1[0].1);

    assert!(match_re("a?", "a").0);
    assert!(match_re("a?", "").0);
    assert!(match_re("xa?", "x").0);

    assert!(!match_re("a{1,3}$", "aaaa").0);
    assert!(match_re("a{1,3}a$", "aaaa").0);
    assert!(match_re("a{1,3}b$", "aaab").0);
    assert!(!match_re("^a{1,3}$", "xaaa").0);
    assert_eq!(vec![(1, 4)], match_re("a{1,3}$", "xaaa").1);

    assert!(match_re("a{3}", "aaa").0);
    assert!(!match_re("a{3}", "aa").0);
    assert!(match_re("xa{,3}", "x").0);
    assert!(match_re("a{0,3}", "a").0);
    assert!(match_re("a{,3}", "").0);
    assert!(match_re("a{,3}", "a").0);
    assert!(match_re("a{,3}", "aa").0);
    assert!(match_re("a{0,3}", "aa").0);
    assert!(match_re("a{,3}", "aaa").0);
    assert!(match_re("a{2,3}", "aaa").0);

    assert!(match_re("a{3,}", "aaa").0);
    assert!(match_re("a{3,}", "aaaa").0);
}

#[test]
fn test_character_classes() {
    assert!(match_re("^[a-z]{1,3}$", "abc").0);
    assert!(!match_re("^[a-z]{1,3}$", "Abc").0);
    assert!(match_re("^[A-z]{1,3}$", "Abc").0);
    assert!(!match_re("^[A-Z]{1,3}$", "Abc").0);
    assert!(match_re("^[A-z]{1,3}$", "Abc").0);
    assert!(!match_re("^[a-Z]{1,3}$", "Abc").0);
    assert!(match_re("^[0-9]{1,3}$", "012").0);
    assert!(match_re("^[0-9]{1,3}$", "02").0);
}

#[test]
fn test_anchoring() {
    assert!(match_re("abc", "012abcdef").0);
    assert!(!match_re("^abc", "012abcdef").0);
    assert!(!match_re("abc$", "012abcdef").0);
    assert!(!match_re("^abc$", "012abcdef").0);
    assert!(match_re("^abc", "abc").0);
    assert!(match_re("abc$", "abc").0);
}

#[test]
fn test_alternate() {
    assert!(match_re("a|bc|d", "a").0);
    assert!(match_re("a|bc|d", "d").0);
    assert!(!match_re("a|bc|d", "b").0);
    assert!(match_re("a|bc|d", "bc").0);
}

#[test]
fn test_submatches() {
    assert_eq!(vec![(0, 3)], match_re("abc", "abcde").1);
    assert_eq!(vec![(1, 4)], match_re("abc", "0abcde").1);
    assert_eq!(vec![(1, 4), (2, 3)], match_re("a(b)c", "0abcde").1);
    assert_eq!(vec![(1, 4), (2, 3)], match_re("a(.)c", "0abcde").1);
    assert_eq!(
        vec![(1, 6), (2, 5), (3, 4)],
        match_re("a(b(.)d)e", "0abcde").1
    );
}
