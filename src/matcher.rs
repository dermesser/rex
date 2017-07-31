//! The matcher module contains types used for matching individual characters in a string being
//! matched against the conditions in a regular expression.
#![allow(dead_code)]

use std::rc::Rc;
use std::fmt::Debug;

/// Matchee contains a character and position to match.
pub struct Matchee {
    /// Current index within the matched string.
    ix: usize,
    /// Length of the string to match.
    len: usize,
    /// Character at current position.
    c: char,
}

/// Matcher matches characters.
pub trait Matcher: Debug {
    fn matches(&self, m: &Matchee) -> bool;
}

#[derive(Debug)]
pub struct CharMatcher(pub char);
impl Matcher for CharMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        m.c == self.0
    }
}

#[derive(Debug)]
pub struct CharRangeMatcher(pub char, pub char);
impl Matcher for CharRangeMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        m.c >= self.0 && m.c <= self.1
    }
}

#[derive(Debug)]
pub struct CharSetMatcher(pub Vec<char>);
impl Matcher for CharSetMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        self.0.contains(&m.c)
    }
}

#[derive(Debug)]
pub struct AnyMatcher;
impl Matcher for AnyMatcher {
    fn matches(&self, _: &Matchee) -> bool {
        true
    }
}

#[derive(Debug)]
pub enum AnchorMatcher {
    Begin,
    End,
}
impl Matcher for AnchorMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        match self {
            &AnchorMatcher::Begin => m.ix == 0,
            &AnchorMatcher::End => m.ix == m.len,
        }
    }
}

pub fn wrap_matcher(m: Box<Matcher>) -> Option<Rc<Box<Matcher>>> {
    Some(Rc::new(m))
}
