//! The repr module is concerned with the representation of parsed regular expressions and their
//! compilation into a state graph. The state graph itself is defined in the state module.
#![allow(dead_code)]

use matcher::{self, wrap_matcher};
use state::{wrap_state, Compile, State, Submatch, WrappedState};

/// start_compile takes a parsed regex as RETree and returns the first node of a directed graph
/// representing the regex.
pub fn start_compile(re: &Pattern) -> WrappedState {
    let (s, sp) = re.to_state();

    let mut before = State::default();
    before.sub = Some(Submatch::Start);
    before.out = Some(s);
    let mut end = State::default();
    end.sub = Some(Submatch::End);

    let endw = wrap_state(end);
    // Connect all loose ends with the final node.
    for p in sp {
        p.borrow_mut().patch(endw.clone());
    }
    wrap_state(before)
}

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

#[derive(Clone, Debug, PartialEq)]
pub enum AnchorLocation {
    Begin,
    End,
}

impl Compile for Pattern {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Pattern::Concat(ref ps) => {
                if ps.is_empty() {
                    panic!("invalid Concat len: 0")
                } else if ps.len() == 1 {
                    return ps[0].to_state();
                }

                let (init, mut lastp) = ps[0].to_state();
                for i in 1..ps.len() {
                    let (next, nextp) = ps[i].to_state();
                    // Connect all loose ends with the new node.
                    for p in lastp {
                        p.borrow_mut().patch(next.clone());
                    }
                    // Remember the loose ends of this one.
                    lastp = nextp;
                }
                (init, lastp)
            }
            Pattern::Any => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::AnyMatcher)),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
            Pattern::Char(c) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharMatcher(c))),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
            Pattern::Str(ref s) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::StringMatcher::new(s))),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
            Pattern::CharRange(from, to) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharRangeMatcher(from, to))),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
            Pattern::CharSet(ref set) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharSetMatcher(set.clone()))),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
            Pattern::Alternate(ref r) => alternate(&r, &vec![]),
            Pattern::Submatch(ref p) => {
                let (s, sp) = p.to_state();
                let before = wrap_state(State {
                    out: Some(s),
                    out1: None,
                    matcher: None,
                    sub: Some(Submatch::Start),
                });
                let after = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: None,
                    sub: Some(Submatch::End),
                });
                for p in sp {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Pattern::Repeated(ref p) => p.to_state(),
            Pattern::Anchor(ref loc) => {
                let mut m = matcher::AnchorMatcher::Begin;
                match loc {
                    &AnchorLocation::End => m = matcher::AnchorMatcher::End,
                    _ => (),
                };
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(m)),
                    sub: None,
                });
                (s.clone(), vec![s])
            }
        }
    }
}

// alternate compiles a list of patterns into a graph that accepts any one of the patterns.
fn alternate(ps: &[Pattern], to_patch: &[WrappedState]) -> (WrappedState, Vec<WrappedState>) {
    if ps.len() == 1 {
        let (s, sp) = ps[0].to_state();
        for e in to_patch {
            e.borrow_mut().patch(s.clone());
        }
        (s, sp)
    } else {
        let mut init = State {
            out: None,
            out1: None,
            matcher: None,
            sub: None,
        };
        let mid = ps.len() / 2;
        let (left, mut leftpatch) = alternate(&ps[..mid], &vec![]);
        let (right, mut rightpatch) = alternate(&ps[mid..], &vec![]);
        init.patch(left);
        init.patch(right);
        leftpatch.append(&mut rightpatch);
        (wrap_state(init), leftpatch)
    }
}

/// A pattern can be repeated in various manners, which is represented by the pattern being wrapped
/// in a Repetition.
/// The inner type is a pattern, because a repetition is either concerned with only one pattern
/// (/.?/), or a submatch (/(abc)?/).
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

impl Repetition {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Repetition::ZeroOrOnce(ref p) => {
                let (s, to_patch) = p.to_state();
                let after = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: None,
                    sub: None,
                });
                let before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: Some(after.clone()),
                    matcher: None,
                    sub: None,
                });
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::ZeroOrMore(ref p) => {
                let (s, to_patch) = p.to_state();
                let before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                });
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                });
                before.borrow_mut().patch(after.clone());
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::OnceOrMore(ref p) => {
                let (s, to_patch) = p.to_state();
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                });
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (s, vec![after])
            }
            // Specific is 'min' concatenations of a simple state and 'max - min' concatenations of
            // a ZeroOrOnce state.
            Repetition::Specific(ref p, min, max_) => {
                let cap = max_.unwrap_or(min) as usize;
                assert!(cap >= min as usize);
                let mut repetition = Vec::with_capacity(cap);

                // Append the minimum required number of occurrences.
                for _ in 0..min {
                    repetition.push(p.clone());
                }

                // If an upper limit is set, append max-min repetitions of ZeroOrOnce states for
                // the repeated pattern.
                if let Some(max) = max_ {
                    for _ in 0..(max - min) {
                        repetition.push(
                            Pattern::Repeated(
                                Box::new(Repetition::ZeroOrOnce(p.clone()))));
                    }
                } else {
                    // If no upper limit is set, append a ZeroOrMore state for the repeated
                    // pattern.
                    repetition.push(Pattern::Repeated(Box::new(Repetition::ZeroOrMore(p.clone()))));
                }
                Pattern::Concat(repetition).to_state()
            }
        }
    }
}

/// optimize contains functionality for optimizing the representation of regular expressions for
/// more efficient matching.
mod optimize {
    use super::*;
    use std::iter::{FromIterator, Iterator};
    use std::ops::Deref;

    pub fn optimize(mut p: Pattern) -> Pattern {
        p = concat_chars_to_str(p);
        p = optimize_recursively(p);
        p
    }

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
                            let newp = Pattern::Str(String::from_iter(chars.drain(..)));
                            new_elems.push(newp);
                            new_elems.push(e);
                            assert!(chars.is_empty());
                        }
                    }
                }
                if !chars.is_empty() {
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
                    Repetition::ZeroOrOnce(p) => Repetition::ZeroOrOnce(optimize(p)),
                    Repetition::ZeroOrMore(p) => Repetition::ZeroOrMore(optimize(p)),
                    Repetition::OnceOrMore(p) => Repetition::OnceOrMore(optimize(p)),
                    Repetition::Specific(p, min, max) => {
                        Repetition::Specific(optimize(p), min, max)
                    }
                }))
            }

            p => p,
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
        let case1 =
            (Pattern::Str("abc".to_string()),
             Pattern::Concat(vec![Pattern::Char('a'), Pattern::Char('b'), Pattern::Char('c')]));
        let case2 = (Pattern::Str("abcd".to_string()),
                     Pattern::Concat(vec![Pattern::Str("a".to_string()),
                                          Pattern::Char('b'),
                                          Pattern::Str("cd".to_string())]));
        let case3 = (Pattern::Concat(vec![Pattern::Str("abc".to_string()),
                                          Pattern::Anchor(AnchorLocation::End),
                                          Pattern::Str("d".to_string())]),
                     Pattern::Concat(vec![Pattern::Char('a'),
                                          Pattern::Char('b'),
                                          Pattern::Char('c'),
                                          Pattern::Anchor(AnchorLocation::End),
                                          Pattern::Char('d')]));

        for c in vec![case1, case2, case3].into_iter() {
            assert_eq!(c.0, optimize::optimize(c.1));
        }
    }

    // /a(b|c)/
    fn simple_re0() -> Pattern {
        Pattern::Concat(vec![Pattern::CharRange('a', 'a'),
                             Pattern::Alternate(vec![((Pattern::Char('b'))),
                                                     ((Pattern::Char('c')))])])
    }
    // Returns compiled form of /(a[bc])?(cd)*(e|f)+x{1,3}(g|hh|i)j{2,}klm/
    fn simple_re1() -> Pattern {
        Pattern::Concat(vec!(
                Pattern::Repeated(
                        Box::new(
                    Repetition::ZeroOrOnce(
                            Pattern::Submatch(Box::new(Pattern::Concat(vec!(
                                    Pattern::Char('a'), Pattern::CharRange('b', 'c')))))))),

                Pattern::Repeated(
                    Box::new(Repetition::ZeroOrMore(
                            Pattern::Submatch(Box::new(Pattern::Concat(vec!(
                                    Pattern::Char('c'), Pattern::Char('d')))))))),

                Pattern::Submatch(
                    Box::new((
                            Pattern::Repeated(
                                Box::new(Repetition::OnceOrMore(
                                        Pattern::Alternate(vec!(
                                                ((Pattern::Char('e'))),
                                                ((Pattern::Char('f'))))))))))),


                Pattern::Repeated(
                    Box::new(Repetition::Specific(Pattern::Char('x'), 1, Some(3)))),

                Pattern::Alternate(vec!(
                    ((Pattern::Char('g'))),
                    ((Pattern::Repeated(
                                Box::new(Repetition::Specific(Pattern::Char('h'), 2, Some(2)))))),
                    ((Pattern::Char('i'))))),

                Pattern::Repeated(
                    Box::new(Repetition::Specific(Pattern::Char('j'), 2, None))),

                Pattern::Str("klm".to_string()),
        ))
    }

    #[test]
    fn test_re1() {
        // println!("{:?}", start_compile(simple_re0()));
        let dot = dot(start_compile(&simple_re1()));
        println!("digraph st {{ {} }}", dot);
    }
}
