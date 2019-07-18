#![cfg(test)]

//! A general test suite aiming for wide coverage of positive and negative matches.

use crate::{compile, matching, parse};

fn match_re(re: &str, s: &str) -> (bool, Vec<(usize, usize)>) {
    let parsed = parse::parse(re).unwrap();
    let ready = compile::start_compile(&parsed);
    matching::do_match(ready, s)
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
