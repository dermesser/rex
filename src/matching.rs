//! This module contains the logic matching a compiled regular expression (a State graph) against a
//! string.

#![allow(dead_code)]

use std::borrow::Cow;
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;

use state::{WrappedState, Submatch};
use matcher::Matchee;

#[derive(Clone, Debug)]
pub struct MatchState {
    node: WrappedState,
    matchee: Matchee,
    // The set of submatches encountered, indexed by the start of a submatch. If submatches
    // (with (start,end)) (1,3),(5,10) have been encountered, then submatches[1] = Some(3) and
    // submatches[5] = Some(10). If the contents is None, then the end has not yet been
    // encountered.
    // BUG: This doesn't work for several submatches starting at the same position. For that, we'd
    // need a Rc<RefCell<Vec<Vec<usize>>>> :-)
    submatches: Rc<RefCell<Vec<Option<usize>>>>,
    // We need to clone the submatches queue only rarely (when a submatch starts or ends).
    submatches_todo: Cow<'static, Vec<usize>>,
}

impl MatchState {
    fn new(s: &str, ws: WrappedState) -> MatchState {
        MatchState {
            node: ws,
            matchee: Matchee::from_string(s),
            submatches: Rc::new(RefCell::new(vec![None; s.len()])),
            submatches_todo: Cow::Owned(Vec::new()),
        }
    }
    fn fork(&self, next: WrappedState, advance: usize) -> MatchState {
        let mut n = self.clone();
        n.matchee.advance(advance);
        n.node = next;
        n
    }
    fn update(&mut self, next: WrappedState, advance: usize) {
        self.matchee.advance(advance);
        self.node = next;
    }
    fn reset(&mut self, new_start: usize) {
        self.submatches = Rc::new(RefCell::new(vec![None; self.matchee.len()]));
        self.submatches_todo.to_mut().clear();
        self.matchee.reset(new_start);
    }
    fn start_submatch(&mut self) {
        if self.matchee.pos() < self.matchee.len() {
            self.submatches_todo.to_mut().push(self.matchee.pos());
        }
    }
    fn stop_submatch(&mut self) {
        if let Some(begin) = self.submatches_todo.to_mut().pop() {
            self.submatches.borrow_mut()[begin] = Some(self.matchee.pos());
        }
    }
}

pub fn do_match(ws: WrappedState, s: &str) -> (bool, Vec<(usize, usize)>) {
    let mut ms = MatchState::new(s, ws);

    for i in 0..s.len() {
        ms.reset(i);
        match start_match(ms.clone()) {
            (false, _) => continue,
            (true, matchpos) => {
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
/// string matches, otherwise false. It also returns a vector of submatches; if the entry at index
/// I contains Some(J), then that means that there is a submatch starting at I extending to (J-1).
pub fn start_match(m: MatchState) -> (bool, Vec<Option<usize>>) {
    let mut states = Vec::with_capacity(4);
    let mut states_next = Vec::with_capacity(4);
    states.push(m);

    let (mut ismatch, mut matches) = (false, vec![]);
    let mut longestmatch = 0;

    loop {
        if states.is_empty() {
            break;
        }

        // Iterate over all current states, see which match, and add the successors of matching
        // states to the states_next list.
        for mut st in states.drain(..) {
            let (next1, next2) = st.node.borrow().next_states();

            // Check if this node is a submatch start or end. If it is, the list of pending
            // submatches is cloned and the current position pushed (Start) or the most recent
            // submatch start popped and stored in the overall submatch list (End).
            let sub = st.node.borrow().sub.clone();
            match sub {
                Some(Submatch::Start) => st.start_submatch(),
                Some(Submatch::End) => st.stop_submatch(),
                None => {}
            }

            // Found match (intentionally down here, after finalizing submatch processing). Only
            // update match if this match is longer than the previous one.
            if next1.is_none() && next2.is_none() && st.matchee.pos() > longestmatch {
                ismatch = true;
                matches = st.submatches.borrow().clone();
                longestmatch = st.matchee.pos();
                continue;
            }
            let mut advance_by = 0;

            // Check if the current state matches.
            if let Some((matched, howmany)) = st.node.borrow().matches(&st.matchee) {
                // Current state didn't match, throw away.
                if !matched {
                    continue;
                }
                advance_by = howmany;
            }

            // We only clone the current state if there's a fork in the graph. Otherwise we reuse
            // the old state.
            if next1.is_some() && next2.is_some() {
                // If the current state matched, or it didn't have a matcher, push next states into
                // list of next states.
                states_next.push(st.fork(next1.unwrap(), advance_by));
                st.update(next2.unwrap(), advance_by);
                states_next.push(st);
            } else if let Some(n1) = next1 {
                // Reuse current state if only one successor (common case).
                st.update(n1, advance_by);
                states_next.push(st)
            } else if let Some(n2) = next2 {
                st.update(n2, advance_by);
                states_next.push(st);
            }
        }
        // Swap state lists, leaving states_next empty.
        mem::swap(&mut states, &mut states_next);
    }

    return (ismatch, matches);
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::*;
    use state::*;
    use parse;

    fn simple_re0() -> Pattern {
        parse::parse("^a(b+)$c$").unwrap()
    }

    // /a(b|c)(xx)?$/
    fn raw_re() -> Pattern {
        Pattern::Concat(vec![
                       Pattern::CharRange('a', 'a'),
                       Pattern::Submatch(
                           Box::new((Pattern::Alternate(
                                       vec![
                                       ((Pattern::Char('b'))),
                                       ((Pattern::Char('c')))]
                                       )))),
                       Pattern::Submatch(Box::new((
                                   Pattern::Repeated(Box::new(
                                           Repetition::ZeroOrOnce(
                                               Pattern::Str("xx".to_string()))))))),
                       Pattern::Anchor(AnchorLocation::End),
        ])
    }

    #[test]
    fn test_match_simple() {
        let re = simple_re0();
        // println!("{:?}", re);
        println!("{:?}", do_match(start_compile(&re), "abb$c"));
        let dot = dot(start_compile(&re));
        println!("digraph st {{ {} }}", dot);
    }
}
