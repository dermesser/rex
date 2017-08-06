//! The state module defines the types used for building the so called states graph; a graph used
//! during interpretation of a regular expression. It is built from a (parsed) representation in
//! the repr module.
#![allow(dead_code)]

use std::cell::RefCell;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::fmt::Write;
use std::iter::FromIterator;
use std::rc::Rc;

use matcher::{Matchee, Matcher};

/// Types implementing Compile can be compiled into a state graph.
pub trait Compile {
    /// to_state returns the start node of a subgraph, and a list of pointers that need to be
    /// connected to the following subgraph. The list can contain the first tuple element.
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>);
}

/// State is a single state that the evaluation can be in. It contains several output states as
/// well as a matcher.
#[derive(Debug, Default, Clone)]
pub struct State {
    // Possible following state(s).
    pub out: Option<WrappedState>,
    pub out1: Option<WrappedState>,
    // If matcher is none, this is an "empty" state.
    pub matcher: Option<Rc<Box<Matcher>>>,
}

/// WrappedState is a shared pointer to a state node.
pub type WrappedState = Rc<RefCell<State>>;

pub fn wrap_state(s: State) -> WrappedState {
    Rc::new(RefCell::new(s))
}

impl State {
    pub fn patch(&mut self, next: WrappedState) {
        if self.out.is_none() {
            self.out = Some(next);
        } else if self.out1.is_none() {
            self.out1 = Some(next);
        } else {
            unimplemented!()
        }
    }

    pub fn is_last(&self) -> bool {
        self.out.is_none() && self.out1.is_none()
    }

    pub fn has_matcher(&self) -> bool {
        self.matcher.is_some()
    }

    /// Checks the matchee against the matcher. Returns None if the node doesn't contain a matcher.
    pub fn matches(&self, me: &Matchee) -> Option<(bool, usize)> {
        self.matcher.as_ref().map(|m| m.matches(me))
    }

    /// Returns the following states, if present. Returns (None, None) if it's the final node.
    pub fn next_states(&self) -> (Option<WrappedState>, Option<WrappedState>) {
        (self.out.clone(), self.out1.clone())
    }
}

/// dot converts a graph starting with s into a Dot graph.
pub fn dot(s: WrappedState) -> String {
    let mut result = String::new();

    let mut visited = HashSet::new();
    let mut todo = LinkedList::from_iter(vec![s]);

    loop {
        if todo.is_empty() {
            break;
        }
        let node = todo.pop_front().unwrap();
        let id = format!("{:?}", node.as_ptr());
        if visited.contains(&id) {
            continue;
        }
        visited.insert(id.clone());

        for next in [node.borrow().out.clone(), node.borrow().out1.clone()].into_iter() {
            if let &Some(ref o) = next {
                let nextid = format!("{:p}", o.as_ptr());
                write!(&mut result,
                       "\"{} {:?}\" -> \"{} {:?}\";\n",
                       id,
                       node.borrow().matcher,
                       nextid,
                       o.borrow().matcher)
                    .unwrap();

                if !visited.contains(&nextid) {
                    todo.push_front(o.clone());
                }
            }
        }
    }
    result
}
