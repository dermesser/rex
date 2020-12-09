//! The repr module is concerned with the representation of parsed regular expressions. A Pattern
//! is compiled by the `compile` module into a state graph defined in `state`.
#![allow(dead_code)]

/// A Pattern is either a repeated pattern, a stored submatch, an alternation between two patterns,
/// two patterns following each other, or a character range or set.
#[derive(Clone, Debug, PartialEq)]
pub enum Pattern {
    Concat(Vec<Pattern>),
    /// A repeated sub-pattern.
    Repeated(Box<Repetition>),
    /// A stored submatch.
    Submatch(Box<Pattern>),
    /// An alternation between patterns (a|bb|ccc)
    Alternate(Vec<Pattern>),
    /// A single character.
    Char(char),
    /// Any character (.).
    Any,
    /// A string.
    Str(String),
    /// A character range.
    CharRange(char, char),
    /// A set of characters.
    CharSet(Vec<char>),
    /// A position anchor.
    Anchor(AnchorLocation),
}

/// `AnchorLocation` encodes `^` and `$` anchors, respectively.
#[derive(Clone, Debug, PartialEq)]
pub enum AnchorLocation {
    Begin,
    End,
}

/// A pattern can be repeated in various manners, which is represented by the pattern being wrapped
/// in a Repetition.
///
/// The inner type is a pattern, because a repetition is either concerned with only one pattern
/// (`/.?/`), or a submatch (`/(abc)?/`).
#[derive(Clone, Debug, PartialEq)]
pub enum Repetition {
    /// /P+/
    ZeroOrOnce(Pattern),
    /// /P*/
    ZeroOrMore(Pattern),
    /// /P+/
    OnceOrMore(Pattern),
    /// /P{min, (max)}/
    Specific(Pattern, u32, Option<u32>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::*;

    // /a(b|c)/
    fn simple_re0() -> Pattern {
        Pattern::Concat(vec![
            Pattern::CharRange('a', 'a'),
            Pattern::Alternate(vec![(Pattern::Char('b')), (Pattern::Char('c'))]),
        ])
    }
    // Returns compiled form of /(a[bc])?(cd)*(e|f)+x{1,3}(g|hh|i)j{2,}klm/
    fn simple_re1() -> Pattern {
        Pattern::Concat(vec![
            Pattern::Repeated(Box::new(Repetition::ZeroOrOnce(Pattern::Submatch(
                Box::new(Pattern::Concat(vec![
                    Pattern::Char('a'),
                    Pattern::CharRange('b', 'c'),
                ])),
            )))),
            Pattern::Repeated(Box::new(Repetition::ZeroOrMore(Pattern::Submatch(
                Box::new(Pattern::Concat(vec![
                    Pattern::Char('c'),
                    Pattern::Char('d'),
                ])),
            )))),
            Pattern::Submatch(Box::new(Pattern::Repeated(Box::new(
                Repetition::OnceOrMore(Pattern::Alternate(vec![
                    (Pattern::Char('e')),
                    (Pattern::Char('f')),
                ])),
            )))),
            Pattern::Repeated(Box::new(Repetition::Specific(
                Pattern::Char('x'),
                1,
                Some(3),
            ))),
            Pattern::Alternate(vec![
                Pattern::Char('g'),
                Pattern::Repeated(Box::new(Repetition::Specific(
                    Pattern::Char('h'),
                    2,
                    Some(2),
                ))),
                (Pattern::Char('i')),
            ]),
            Pattern::Repeated(Box::new(Repetition::Specific(Pattern::Char('j'), 2, None))),
            Pattern::Str("klm".to_string()),
        ])
    }

    use crate::compile::start_compile;

    #[test]
    fn test_re1() {
        // println!("{:?}", start_compile(simple_re0()));
        let dot = dot(&start_compile(&simple_re1()));
        println!("digraph st {{ {} }}", dot);
    }
}
