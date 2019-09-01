//! The matcher module contains types used for matching individual characters in a string being
//! matched against the conditions in a regular expression.
#![allow(dead_code)]

use std::fmt::Debug;
use std::iter::{self, FromIterator};
use std::rc::Rc;

/// Matchee contains a character and position to match. It's used by the matching logic to check
/// whether a certain position within a string is matched by a matcher. The driving logic is
/// external to this, however (except for the advance() method).
#[derive(Clone, Debug)]
pub struct Matchee {
    /// A random-addressable string to be matched. This is the overall string. It's wrapped inside
    /// a shared pointer because during the matching process, there may be different Matchees
    /// at different positions within the source.
    src: Rc<Vec<char>>,
    /// Current index within the matched string.
    ix: usize,
}

impl Matchee {
    pub fn from_string(s: &str) -> Matchee {
        Matchee {
            src: Rc::new(Vec::from_iter(s.chars())),
            ix: 0,
        }
    }
    fn current(&self) -> char {
        self.src[self.ix]
    }
    pub fn pos(&self) -> usize {
        self.ix
    }
    pub fn len(&self) -> usize {
        self.src.len()
    }
    /// advance takes the result of a matcher and advances the cursor in the Matchee if there was a
    /// match.
    pub fn advance(&mut self, n: usize) {
        self.ix += n;
    }
    pub fn reset(&mut self, n: usize) {
        self.ix = n;
    }
    pub fn finished(&self) -> bool {
        self.ix == self.src.len()
    }
    pub fn string(&self) -> String {
        let matchee = String::from_iter(self.src.iter());
        let pointer = String::from_iter(iter::repeat(' ').take(self.ix).chain(iter::once('^')));
        format!("{}\n{}", matchee, pointer)
    }
}

/// A Matcher matches parts of a Matchee (where a Matchee is a string to be matched). While
/// matching, a matcher may consume zero or more characters of the string.
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
        (!m.finished() && m.current() == self.0, 1)
    }
}

#[derive(Debug)]
pub struct StringMatcher(pub Vec<char>);
impl StringMatcher {
    pub fn new(s: &str) -> StringMatcher {
        StringMatcher(Vec::from_iter(s.chars()))
    }
}
impl Matcher for StringMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        if m.pos() + self.0.len() <= m.src.len() {
            (
                m.src[m.pos()..m.pos() + self.0.len()].starts_with(&self.0),
                self.0.len(),
            )
        } else {
            (false, self.0.len())
        }
    }
}

#[derive(Debug)]
pub struct CharRangeMatcher(pub char, pub char);
impl Matcher for CharRangeMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (
            !m.finished() && m.current() >= self.0 && m.current() <= self.1,
            1,
        )
    }
}

#[derive(Debug)]
pub struct CharSetMatcher(pub Vec<char>);
impl Matcher for CharSetMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        (!m.finished() && self.0.contains(&m.current()), 1)
    }
}

/// AnyMatcher matches any character.
#[derive(Debug)]
pub struct AnyMatcher;
impl Matcher for AnyMatcher {
    fn matches(&self, _: &Matchee) -> (bool, usize) {
        (true, 1)
    }
}

/// AnchorMatcher matches the beginning or end of a string. It doesn't consume a character.
#[derive(Debug)]
pub enum AnchorMatcher {
    Begin,
    End,
}
impl Matcher for AnchorMatcher {
    fn matches(&self, m: &Matchee) -> (bool, usize) {
        match self {
            &AnchorMatcher::Begin => (m.pos() == 0, 0),
            &AnchorMatcher::End => (m.finished(), 0),
        }
    }
}

pub fn wrap_matcher(m: Box<dyn Matcher>) -> Option<Rc<Box<dyn Matcher>>> {
    Some(Rc::new(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_char_matcher() {
        let m1 = CharMatcher('a');
        let m2 = CharMatcher('b');
        let mut me = Matchee::from_string("xabc");
        me.ix = 1;
        assert_eq!(m1.matches(&me), (true, 1));
        assert_eq!(m2.matches(&me), (false, 1));
        me.ix += 1;
        assert_eq!(m2.matches(&me), (true, 1));
    }

    #[test]
    fn test_str_matcher() {
        let m1 = StringMatcher::new("abc");
        let m2 = StringMatcher::new("def");
        let mut me = Matchee::from_string("xabcydef");
        assert_eq!(m1.matches(&me), (false, 3));
        me.advance(1);
        assert_eq!(m1.matches(&me), (true, 3));
        assert_eq!(m2.matches(&me), (false, 3));
        me.advance(3);
        assert_eq!(m2.matches(&me), (false, 3));
        me.advance(1);
        assert_eq!(m2.matches(&me), (true, 3));
    }
}
