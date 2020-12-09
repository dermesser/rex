//! The compile module is responsible for taking a `Pattern` and compiling it into a `StateGraph`
//! (as defined in `state`). It is recommended to optimize Patterns before compiling them.
//!
//! The output of `to_state()`, which is implemented for Pattern and some subtypes, is a
//! `StateGraph`, which itself is a vector of `State` objects, each of which in turn is a state of
//! the generated state machine. The states reference each other by their indices in the
//! `StateGraph`.
//!
//! `start_compile()` is the entry point and public API of this module.

use crate::matcher::{self, wrap_matcher};
use crate::repr::{AnchorLocation, Pattern, Repetition};
use crate::state::{State, StateGraph, StateRef, Submatch};

/// Types implementing Compile can be compiled into a state graph.
pub trait Compile {
    /// to_state returns the start node of a subgraph, and a list of pointers that need to be
    /// connected to the following subgraph. The list can contain the first tuple element.
    fn to_state(&self, sg: &mut StateGraph) -> (StateRef, Vec<StateRef>);
}

/// start_compile takes a parsed regex as RETree and returns the first node of a directed graph
/// representing the regex.
pub fn start_compile(re: &Pattern) -> StateGraph {
    let mut state_graph = Vec::with_capacity(64);

    let mut before = State::default();
    before.sub = Some(Submatch::Start);
    // First element in graph vector.
    let beforeref = 0;
    state_graph.push(before);

    let (s, sp) = re.to_state(&mut state_graph);
    state_graph[beforeref].out = Some(s);

    let mut end = State::default();
    end.sub = Some(Submatch::End);
    let endref = state_graph.len();
    state_graph.push(end);

    // Connect all loose ends with the final node.
    for p in sp {
        state_graph[p].patch(endref);
    }
    state_graph
}

impl Compile for Pattern {
    fn to_state(&self, sg: &mut StateGraph) -> (StateRef, Vec<StateRef>) {
        match *self {
            Pattern::Concat(ref ps) => {
                if ps.is_empty() {
                    panic!("invalid Concat len: 0")
                } else if ps.len() == 1 {
                    return ps[0].to_state(sg);
                }

                let (init, mut lastp) = ps[0].to_state(sg);
                for i in 1..ps.len() {
                    let (next, nextp) = ps[i].to_state(sg);
                    // Connect all loose ends with the new node.
                    for p in lastp {
                        sg[p].patch(next);
                    }
                    // Remember the loose ends of this one.
                    lastp = nextp;
                }
                (init, lastp)
            }
            Pattern::Any => {
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::AnyMatcher)),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
            Pattern::Char(c) => {
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharMatcher(c))),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
            Pattern::Str(ref s) => {
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::StringMatcher::new(s))),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
            Pattern::CharRange(from, to) => {
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharRangeMatcher(from, to))),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
            Pattern::CharSet(ref set) => {
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(matcher::CharSetMatcher(set.clone()))),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
            Pattern::Alternate(ref r) => alternate(sg, &r, &vec![]),
            Pattern::Submatch(ref p) => {
                let (s, sp) = p.to_state(sg);
                let before = State {
                    out: Some(s),
                    out1: None,
                    matcher: None,
                    sub: Some(Submatch::Start),
                };
                let after = State {
                    out: None,
                    out1: None,
                    matcher: None,
                    sub: Some(Submatch::End),
                };
                let beforeref = sg.len();
                sg.push(before);
                let afterref = sg.len();
                sg.push(after);
                for p in sp {
                    sg[p].patch(afterref);
                }
                (beforeref, vec![afterref])
            }
            Pattern::Repeated(ref p) => p.to_state(sg),
            Pattern::Anchor(ref loc) => {
                let mut m = matcher::AnchorMatcher::Begin;
                match loc {
                    &AnchorLocation::End => m = matcher::AnchorMatcher::End,
                    _ => (),
                };
                let s = State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(m)),
                    sub: None,
                };
                let sref = sg.len();
                sg.push(s);
                (sref, vec![sref])
            }
        }
    }
}

/// alternate compiles a list of patterns into a graph that accepts any one of the patterns.
fn alternate(
    sg: &mut StateGraph,
    ps: &[Pattern],
    to_patch: &[StateRef],
) -> (StateRef, Vec<StateRef>) {
    if ps.len() == 1 {
        let (s, sp) = ps[0].to_state(sg);
        for e in to_patch {
            sg[*e].patch(s);
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
        let (left, mut leftpatch) = alternate(sg, &ps[..mid], &vec![]);
        let (right, mut rightpatch) = alternate(sg, &ps[mid..], &vec![]);
        init.patch(left);
        init.patch(right);
        leftpatch.append(&mut rightpatch);
        let initref = sg.len();
        sg.push(init);
        (initref, leftpatch)
    }
}

impl Compile for Repetition {
    fn to_state(&self, sg: &mut StateGraph) -> (StateRef, Vec<StateRef>) {
        match *self {
            Repetition::ZeroOrOnce(ref p) => {
                let (s, to_patch) = p.to_state(sg);
                let after = State {
                    out: None,
                    out1: None,
                    matcher: None,
                    sub: None,
                };
                let afterref = sg.len();
                sg.push(after);
                let before = State {
                    out: Some(s),
                    out1: Some(afterref),
                    matcher: None,
                    sub: None,
                };
                let beforeref = sg.len();
                sg.push(before);
                for p in to_patch {
                    sg[p].patch(afterref);
                }
                (beforeref, vec![afterref])
            }
            Repetition::ZeroOrMore(ref p) => {
                let (s, to_patch) = p.to_state(sg);
                let before = State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                };
                let beforeref = sg.len();
                sg.push(before);
                let after = State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                };
                let afterref = sg.len();
                sg.push(after);
                sg[beforeref].patch(afterref);
                for p in to_patch {
                    sg[p].patch(afterref);
                }
                (beforeref, vec![afterref])
            }
            Repetition::OnceOrMore(ref p) => {
                let (s, to_patch) = p.to_state(sg);
                let after = State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    sub: None,
                };
                let afterref = sg.len();
                sg.push(after);
                for p in to_patch {
                    sg[p].patch(afterref);
                }
                (s, vec![afterref])
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
                        repetition.push(Pattern::Repeated(Box::new(Repetition::ZeroOrOnce(
                            p.clone(),
                        ))));
                    }
                } else {
                    // If no upper limit is set, append a ZeroOrMore state for the repeated
                    // pattern.
                    repetition.push(Pattern::Repeated(Box::new(Repetition::ZeroOrMore(
                        p.clone(),
                    ))));
                }
                Pattern::Concat(repetition).to_state(sg)
            }
        }
    }
}
