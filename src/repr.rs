//! The repr module is concerned with the representation of parsed regular expressions and their
//! compilation into a state graph. The state graph itself is defined in the state module.
#![allow(dead_code)]

use matcher::{self, wrap_matcher};
use state::{wrap_state, Compile, State, WrappedState};

/// start_compile takes a parsed regex as RETree and returns the first node of a directed graph
/// representing the regex.
fn start_compile(re: RETree) -> WrappedState {
    let (s, sp) = re.to_state();
    let end = wrap_state(State::default());
    // Connect all loose ends with the final node.
    for p in sp {
        p.borrow_mut().patch(end.clone());
    }
    s
}

/// The root of a parsed regex. A regex consists of zero or more Patterns.
#[derive(Debug)]
enum RETree {
    Concat(Vec<Pattern>),
    One(Pattern),
}

impl Compile for RETree {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>) {
        match *self {
            RETree::One(ref p) => p.to_state(),
            RETree::Concat(ref ps) => {
                if ps.len() < 1 {
                    panic!("invalid Concat len")
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
        }
    }
}

/// A Pattern is either a repeated pattern, a stored submatch, an alternation between two patterns,
/// two patterns following each other, or a character range or set.
#[derive(Debug)]
enum Pattern {
    Repeated(Box<RETree>, Repetition),
    Submatch(Box<RETree>),
    Alternate(Box<RETree>, Box<RETree>),
    Char(char),
    Chars(char, char),
    CharSet(Vec<char>),
}

impl Compile for Pattern {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Pattern::Char(c) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharMatcher(c))),
                });
                (s.clone(), vec![s])
            }
            Pattern::Chars(from, to) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharRangeMatcher(from, to))),
                });
                (s.clone(), vec![s])
            }
            Pattern::CharSet(ref chars) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharSetMatcher(chars.clone()))),
                });
                (s.clone(), vec![s])
            }
            Pattern::Alternate(ref a1, ref a2) => {
                let (sa1, mut sa1p) = a1.to_state();
                let (sa2, mut sa2p) = a2.to_state();
                let st = wrap_state(State {
                    out: Some(sa1),
                    out1: Some(sa2),
                    matcher: None,
                });
                sa1p.append(&mut sa2p);
                (st, sa1p)
            }
            Pattern::Submatch(ref p) => {
                // TODO: Implement submatch tracking
                p.to_state()
            }
            Pattern::Repeated(ref p, ref rep) => {
                let (s, sp) = p.to_state();
                let (r, rp) = rep.repeat(s, sp);
                (r, rp)
            }
        }
    }
}

/// A pattern can be specified to be repeated once (default), zero or once (?), zero or more times
/// (*), one or more times (+) or a specific range of times ({min,max}).
#[derive(Debug)]
enum Repetition {
    Once, // Remove?
    ZeroOrOnce,
    ZeroOrMore,
    OnceOrMore,
    Specific(u32, u32),
}

impl Repetition {
    fn repeat(&self,
              s: WrappedState,
              to_patch: Vec<WrappedState>)
              -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Repetition::Once => (s.clone(), vec![s]),
            Repetition::ZeroOrOnce => {
                let after = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: None,
                });
                let before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: Some(after.clone()),
                    matcher: None,
                });
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::ZeroOrMore => {
                let before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                });
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                });
                before.borrow_mut().patch(after.clone());
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::OnceOrMore => {
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                });
                for p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (s, vec![after])
            }
            // Specific is 'min' concatenations of a simple state and 'max - min' concatenations of
            // a ZeroOrOnce state.
            Repetition::Specific(_min, _max) => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use state::*;

    // /a(b|c)/
    fn simple_re0() -> RETree {
        RETree::Concat(vec![Pattern::Chars('a', 'a'),
                            Pattern::Alternate(Box::new(RETree::One(Pattern::Char('b'))),
                                               Box::new(RETree::One(Pattern::Char('c'))))])
    }
    // Returns compiled form of /(ab)?(cd)*(e|f)+x(g|h)i/
    fn simple_re1() -> RETree {
        RETree::Concat(vec!(
                Pattern::Repeated(
                    Box::new(
                        RETree::Concat(vec!(
                            Pattern::Char('a'), Pattern::Chars('b', 'c')))),
                        Repetition::ZeroOrOnce),

                Pattern::Repeated(
                    Box::new(
                        RETree::Concat(vec!(
                            Pattern::Char('c'), Pattern::Char('d')))),
                        Repetition::ZeroOrMore),

                Pattern::Repeated(
                    Box::new(
                        RETree::One(
                            Pattern::Alternate(
                                Box::new(RETree::One(Pattern::Char('e'))),
                                Box::new(RETree::One(Pattern::Char('f')))))),
                        Repetition::OnceOrMore),


                Pattern::Char('x'),
                Pattern::Alternate(
                    Box::new(RETree::One(Pattern::Char('g'))),
                    Box::new(RETree::One(Pattern::Char('h')))),
                Pattern::Char('i'),
        ))
    }

    #[test]
    fn test_re1() {
        println!("{:?}", simple_re1());
        // println!("{:?}", start_compile(simple_re1()));
        // println!("{:?}", start_compile(simple_re0()));
        let dot = dot(start_compile(simple_re1()));
        println!("digraph st {{ {} }}", dot);
    }
}
