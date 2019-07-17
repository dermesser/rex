use matcher::{self, wrap_matcher};
use repr::{AnchorLocation, Pattern, Repetition};
use state::{wrap_state, State, Submatch, WrappedState};

/// Types implementing Compile can be compiled into a state graph.
pub trait Compile {
    /// to_state returns the start node of a subgraph, and a list of pointers that need to be
    /// connected to the following subgraph. The list can contain the first tuple element.
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>);
}

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

impl Compile for Repetition {
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
                Pattern::Concat(repetition).to_state()
            }
        }
    }
}
