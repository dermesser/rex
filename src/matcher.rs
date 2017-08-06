//! The matcher module contains types used for matching individual characters in a string being
//! matched against the conditions in a regular expression.
#![allow(dead_code)]

use std::iter::FromIterator;
use std::rc::Rc;
use std::fmt::Debug;

/// Matchee contains a character and position to match.
pub struct Matchee {
    /// A random-addressable string to be matched. This is the overall string.
    src: Vec<char>,
    /// Current index within the matched string.
    ix: usize,
}

impl Matchee {
    fn from_string(s: &str) -> Matchee {
        Matchee {
            src: Vec::from_iter(s.chars()),
            ix: 0,
        }
    }
    fn current(&self) -> char {
        self.src[self.ix]
    }
}

/// Matcher matches characters.
pub trait Matcher: Debug {
    /// Returns whether the Matchee matches, and how many characters were matched (if a match
    /// occurred). For example, a character matcher consumes one character, whereas an anchor
    /// doesn't consume any.
    fn matches(&self, m: &Matchee) -> (bool, usize);
}

#[derive(Debug)]
pub struct CharMatcher(pub char);
impl Matcher for CharMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (m.current() == self.0, 1)
    }
}

#[derive(Debug)]
pub struct StringMatcher(pub Vec<char>);
impl StringMatcher {
    fn new(s: &str) -> StringMatcher {
        StringMatcher(Vec::from_iter(s.chars()))
    }
}
impl Matcher for StringMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (m.src[m.ix..m.ix + self.0.len()].starts_with(&self.0), self.0.len())
    }
}

#[derive(Debug)]
pub struct CharRangeMatcher(pub char, pub char);
impl Matcher for CharRangeMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (m.current() >= self.0 && m.current() <= self.1, 1)
    }
}

#[derive(Debug)]
pub struct CharSetMatcher(pub Vec<char>);
impl Matcher for CharSetMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (self.0.contains(&m.current()), 1)
    }
}

#[derive(Debug)]
pub struct AnyMatcher;
impl Matcher for AnyMatcher {
    fn matches(&self, _: &Matchee) -> (bool, usize) {
        (true, 1)
    }
}

#[derive(Debug)]
pub enum AnchorMatcher {
    Begin,
    End,
}
impl Matcher for AnchorMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        match self {
            &AnchorMatcher::Begin => (m.ix == 0, 0),
            &AnchorMatcher::End => (m.ix == m.src.len(), 0),
        }
    }
}

pub fn wrap_matcher(m: Box<Matcher>) -> Option<Rc<Box<Matcher>>> {
    Some(Rc::new(m))
}
