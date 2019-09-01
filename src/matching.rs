//! This module contains the logic matching a compiled regular expression (a State graph) against a
//! string.

#![allow(dead_code)]

use std::ops::Deref;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use matcher::Matchee;
use state::{Submatch, StateGraph, StateRef};

#[derive(Clone, Debug)]
pub struct MatchState {
    node: StateRef,
    matchee: Matchee,
    // The set of submatches encountered, indexed by the start of a submatch. If submatches
    // (with (start,end)) (1,3),(5,10) have been encountered, then submatches[1] = Some(3) and
    // submatches[5] = Some(10). If the contents is None, then the end has not yet been
    // encountered.
    // BUG: This doesn't work for several submatches starting at the same position. For that, we'd
    // need a Rc<RefCell<Vec<Vec<usize>>>> :-)
    submatches: Rc<RefCell<Vec<Option<usize>>>>,
    // We need to clone the submatches queue only rarely (when a submatch starts or ends).
    submatches_todo: Rc<Vec<usize>>,
    debug: bool,
}

impl MatchState {
    fn new(s: &str, ws: StateRef) -> MatchState {
        MatchState {
            node: ws,
            matchee: Matchee::from_string(s),
            submatches: Rc::new(RefCell::new(vec![None; s.len()])),
            submatches_todo: Rc::new(Vec::with_capacity(4)),
            debug: false,
        }
    }
    fn fork(&self, next: StateRef, advance: usize) -> MatchState {
        let mut n = self.clone();
        n.matchee.advance(advance);
        n.node = next;
        n
    }
    fn update(&mut self, next: StateRef, advance: usize) {
        self.matchee.advance(advance);
        self.node = next;
    }
    fn reset(&mut self, new_start: usize) {
        self.submatches = Rc::new(RefCell::new(vec![None; self.matchee.len()]));
        self.submatches_todo = Rc::new(Vec::with_capacity(4));
        self.matchee.reset(new_start);
    }
    fn start_submatch(&mut self) {
        if self.matchee.pos() < self.matchee.len() {
            let mut new_submatches = self.submatches_todo.deref().clone();
            new_submatches.push(self.matchee.pos());
            self.submatches_todo = Rc::new(new_submatches);
        }
    }
    fn stop_submatch(&mut self) {
        if self.submatches_todo.deref().len() > 0 {
            let mut new_submatches = self.submatches_todo.deref().clone();
            let begin = new_submatches.pop().unwrap();
            self.submatches_todo = Rc::new(new_submatches);
            self.submatches.borrow_mut()[begin] = Some(self.matchee.pos());
        }
    }
    fn debug(&self, sg: &StateGraph) -> String {
        let m = self.matchee.string();
        let out = sg[self.node].out.map_or("".to_owned(), |ix| sg[ix].to_string());
        let out1 = sg[self.node].out1.map_or("".to_owned(), |ix| sg[ix].to_string());
        let full = format!("{}   (<{}> next: 1. <{:?}> {} 2. <{:?}> {})\n", m, self.node, sg[self.node].out, out, sg[self.node].out1, out1);
        full
    }
}

/// do_match starts the matching process. It tries to match the supplied compiled regex against the
/// supplied string. If it fails, it skips ahead and tries later in the string (i.e., if the regex
/// isn't anchored, it will do a full-text match).
///
/// The boolean component is true if the match succeeded. The Vec contains tuples of (start,
/// one-past-end) for each submatch, starting with the implicit whole match.
pub fn do_match(sg: &StateGraph, s: &str) -> (bool, Vec<(usize, usize)>) {
    let mut ms = MatchState::new(s, 0);
    let (mut i, len) = (0, s.len());

    // TODO: Find out if a failed match is definitive; an anchored regex can't match anywhere later
    // in the text.
    while i < len || i == 0 {
        ms.reset(i);
        let m = start_match(sg, ms.clone());
        match m {
            // If the match fails, we skip as many characters as were matched at first.
            (false, skip, _) => i = skip + 1,
            (true, _, matchpos) => {
                let mut matches = vec![];
                for i in 0..matchpos.len() {
                    if matchpos[i].is_some() {
                        matches.push((i, matchpos[i].unwrap()));
                    }
                }
                return (true, matches);
            }
        }
    }
    (false, vec![])
}

/// start_match takes an initialized MatchState and starts matching. It returns true if the input
/// string matches, otherwise false; the index in the input string to which the match was
/// successful (in case a match fails, but matches some characters at the beginning); and a vector
/// of submatches; if the entry at index I contains Some(J), then that means that there is a
/// submatch starting at I extending to (J-1).
pub fn start_match(sg: &StateGraph, m: MatchState) -> (bool, usize, Vec<Option<usize>>) {
    let mut states = Vec::with_capacity(4);
    let mut states_next = Vec::with_capacity(4);
    states.push(m);

    let (mut ismatch, mut matches) = (false, vec![]);
    let mut longestmatch = 0;
    let mut longest_partial_match = 0;

    loop {
        if states.is_empty() {
            break;
        }

        // Iterate over all current states, see which match, and add the successors of matching
        // states to the states_next list.
        for mut matchst in states.drain(..) {
            let (next1, next2) = sg[matchst.node].next_states();

            // Check if this node is a submatch start or end. If it is, the list of pending
            // submatches is cloned and the current position pushed (Start) or the most recent
            // submatch start popped and stored in the overall submatch list (End).
            let sub = sg[matchst.node].sub.as_ref();
            match sub {
                Some(&Submatch::Start) => matchst.start_submatch(),
                Some(&Submatch::End) => matchst.stop_submatch(),
                None => {}
            }

            // Found match (intentionally down here, after finalizing submatch processing). Only
            // update match if this match is longer than the previous one.
            if next1.is_none() && next2.is_none() && (matchst.matchee.pos() > longestmatch || longestmatch == 0) {
                ismatch = true;
                matches = matchst.submatches.borrow().clone();
                longestmatch = matchst.matchee.pos();
                continue;
            }
            // longest_partial_match contains the furthest any substate has advanced into the
            // string.
            if matchst.matchee.pos() > longest_partial_match {
                longest_partial_match = matchst.matchee.pos();
            }

            let mut advance_by = 0;
            // Check if the current state matches.
            if let Some((matched, howmany)) = sg[matchst.node].matches(&matchst.matchee) {
                // Current state didn't match, throw away.
                if !matched {
                    continue;
                }
                advance_by = howmany;
            }

            // We only clone the current state if there's a fork in the graph. Otherwise we reuse
            // the old state.
            //
            // NOTE: This is what causes exponential cost of parsing "notorious" regular
            // expressions. It is easy to implement; it would be better to at most create one new
            // state, and construct more lazily.
            if next1.is_some() && next2.is_some() {
                // If the current state matched, or it didn't have a matcher, push next states into
                // list of next states.
                states_next.push(matchst.fork(next1.unwrap(), advance_by));
                matchst.update(next2.unwrap(), advance_by);
                states_next.push(matchst);
            } else if let Some(n1) = next1 {
                // Reuse current state if only one successor (common case).
                matchst.update(n1, advance_by);
                states_next.push(matchst)
            } else if let Some(n2) = next2 {
                matchst.update(n2, advance_by);
                states_next.push(matchst);
            }
        }
        // Swap state lists, leaving states_next empty.
        mem::swap(&mut states, &mut states_next);
    }

    return (ismatch, longest_partial_match, matches);
}

#[cfg(test)]
mod tests {
    use super::*;
    use compile::*;
    use parse;
    use repr::*;
    use state::*;

    fn simple_re0() -> Pattern {
        parse::parse("aa+$").unwrap()
    }

    // /a(b|c)(xx)?$/
    fn raw_re() -> Pattern {
        Pattern::Concat(vec![
            Pattern::CharRange('a', 'a'),
            Pattern::Submatch(Box::new(Pattern::Alternate(vec![
                (Pattern::Char('b')),
                (Pattern::Char('c')),
            ]))),
            Pattern::Submatch(Box::new(Pattern::Repeated(Box::new(
                Repetition::ZeroOrOnce(Pattern::Str("xx".to_string())),
            )))),
            Pattern::Anchor(AnchorLocation::End),
        ])
    }

    #[test]
    fn test_match_simple() {
        let re = simple_re0();
        println!("{:?}", re);
        println!("{:?}", do_match(&start_compile(&re), "aaab"));
        let dot = dot(&start_compile(&re));
        println!("digraph st {{ {} }}", dot);
    }
}
