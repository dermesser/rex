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

/// optimize contains functionality for optimizing the representation of regular expressions for
/// more efficient matching.
pub mod optimize {
    use super::*;
    use std::iter::{FromIterator, Iterator};
    use std::ops::Deref;

    pub fn optimize(mut p: Pattern) -> Pattern {
        p = concat_chars_to_str(p);
        p = flatten_alternate(p);
        p = optimize_recursively(p);
        p
    }

    /// optimize_recursively applies optimize() to the inner Patterns of a Pattern.
    fn optimize_recursively(p: Pattern) -> Pattern {
        match p {
            Pattern::Concat(ps) => Pattern::Concat(ps.into_iter().map(optimize).collect()),
            Pattern::Submatch(bp) => {
                let sub = optimize(bp.deref().clone());
                Pattern::Submatch(Box::new(sub))
            }
            Pattern::Alternate(ps) => Pattern::Alternate(ps.into_iter().map(optimize).collect()),
            Pattern::Repeated(r) => {
                let rep = r.deref().clone();
                Pattern::Repeated(Box::new(match rep {
                    Repetition::ZeroOrOnce(rp) => Repetition::ZeroOrOnce(optimize(rp)),
                    Repetition::ZeroOrMore(rp) => Repetition::ZeroOrMore(optimize(rp)),
                    Repetition::OnceOrMore(rp) => Repetition::OnceOrMore(optimize(rp)),
                    Repetition::Specific(rp, min, max) => {
                        Repetition::Specific(optimize(rp), min, max)
                    }
                }))
            }

            p => p,
        }
    }

    /// concat_chars_to_str collapses successive single-character patterns into a single string
    /// pattern.
    fn concat_chars_to_str(p: Pattern) -> Pattern {
        match p {
            Pattern::Concat(mut v) => {
                let mut drain = v.drain(..);
                let mut new_elems = vec![];

                // Find runs of adjacent chars/strings and convert them to single strings.
                // Once a run is broken, append non-char/string patterns and continue afterwards.
                let mut chars = vec![];
                for cp in &mut drain {
                    match cp {
                        Pattern::Char(c) => chars.push(c),
                        Pattern::Str(mut s) => {
                            chars.extend(s.drain(..));
                        }
                        e => {
                            // Once a run of chars/strings is broken, merge the run, push the
                            // non-char/string pattern and continue with the next one.
                            if chars.len() == 1 {
                                new_elems.push(Pattern::Char(chars.pop().unwrap()))
                            } else if chars.len() > 1 {
                                let newp = Pattern::Str(String::from_iter(chars.drain(..)));
                                new_elems.push(newp);
                            }
                            new_elems.push(e);
                            assert!(chars.is_empty());
                        }
                    }
                }

                if chars.len() == 1 {
                    new_elems.push(Pattern::Char(chars[0]));
                } else if chars.len() > 1 {
                    let newp = Pattern::Str(String::from_iter(chars.drain(..)));
                    new_elems.push(newp);
                }

                if new_elems.len() == 1 {
                    new_elems.pop().unwrap()
                } else {
                    Pattern::Concat(new_elems)
                }
            }
            _ => p,
        }
    }

    /// flatten_alternate takes the alternatives in a Pattern::Alternate and reduces the nesting
    /// recursively.
    fn flatten_alternate(p: Pattern) -> Pattern {
        fn _flatten_alternate(p: Pattern) -> Vec<Pattern> {
            match p {
                Pattern::Alternate(a) => {
                    let mut alternatives = vec![];
                    for alt in a.into_iter() {
                        alternatives.append(&mut _flatten_alternate(alt));
                    }
                    alternatives
                }
                p_ => vec![p_],
            }
        }

        let mut fa = _flatten_alternate(p);
        if fa.len() == 1 {
            fa.pop().unwrap()
        } else {
            Pattern::Alternate(fa)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::*;

    #[test]
    fn test_repr_optimize() {
        // case = (want, input)
        let case1 = (
            Pattern::Str("abc".to_string()),
            Pattern::Concat(vec![
                Pattern::Char('a'),
                Pattern::Char('b'),
                Pattern::Char('c'),
            ]),
        );
        let case2 = (
            Pattern::Str("abcd".to_string()),
            Pattern::Concat(vec![
                Pattern::Str("a".to_string()),
                Pattern::Char('b'),
                Pattern::Str("cd".to_string()),
            ]),
        );
        let case3 = (
            Pattern::Concat(vec![
                Pattern::Str("abc".to_string()),
                Pattern::Anchor(AnchorLocation::End),
                Pattern::Char('d'),
            ]),
            Pattern::Concat(vec![
                Pattern::Char('a'),
                Pattern::Char('b'),
                Pattern::Char('c'),
                Pattern::Anchor(AnchorLocation::End),
                Pattern::Char('d'),
            ]),
        );

        for c in vec![case1, case2, case3].into_iter() {
            assert_eq!(c.0, optimize::optimize(c.1));
        }
    }

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

    use compile::start_compile;

    #[test]
    fn test_re1() {
        // println!("{:?}", start_compile(simple_re0()));
        let dot = dot(&start_compile(&simple_re1()));
        println!("digraph st {{ {} }}", dot);
    }
}
