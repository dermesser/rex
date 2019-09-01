//! The state module defines the types used for building the so called states graph; a graph used
//! during interpretation of a regular expression. It is built from a (parsed) representation in
//! the repr module.
#![allow(dead_code)]

use std::collections::HashSet;
use std::collections::LinkedList;
use std::fmt::Write;
use std::iter::FromIterator;
use std::rc::Rc;
use std::vec::Vec;

use matcher::{Matchee, Matcher};

/// StateGraph is the graph of states that the interpreter traverses while matching a regular
/// expression. It is represented as flat vector. The first element is the State node to start the
/// evaluation with.
pub type StateGraph = Vec<State>;

/// StateRef is a reference to a state in a StateGraph.
pub type StateRef = usize;

/// CompiledRE is a compiled regular expression that can be used for matching.
pub type CompiledRE = StateGraph;

/// State is a single state that the evaluation can be in. It contains several output states as
/// well as a matcher.
#[derive(Debug, Default, Clone)]
pub struct State {
    // Possible following state(s).
    pub out: Option<StateRef>,
    pub out1: Option<StateRef>,
    // If matcher is none, this is an "empty" state.
    pub matcher: Option<Rc<Box<dyn Matcher>>>,
    // Tells the matching logic to record the start or end of a submatch.
    pub sub: Option<Submatch>,
}

/// A `State` can be marked to start or end a submatch (usually denoted by parentheses in a regular
/// expression).
#[derive(Clone, Debug)]
pub enum Submatch {
    Start,
    End,
}

impl State {
    pub fn patch(&mut self, next: StateRef) {
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
    pub fn next_states(&self) -> (Option<StateRef>, Option<StateRef>) {
        (self.out.clone(), self.out1.clone())
    }

    pub fn to_string(&self) -> String {
        format!(
            "m:{} sub:{}",
            if let Some(ref m) = self.matcher {
                format!("{:?}", m)
            } else {
                "_".to_string()
            },
            if let Some(ref s) = self.sub {
                format!("{:?}", s)
            } else {
                "".to_string()
            }
        )
    }
}

/// dot converts a graph into a graphviz dot representation.
pub fn dot(stateg: &StateGraph) -> String {
    let mut result = String::new();

    let mut visited = HashSet::new();
    let mut todo = LinkedList::from_iter(vec![0 as StateRef]);

    loop {
        if todo.is_empty() {
            break;
        }
        let current = todo.pop_front().unwrap();
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current);

        for next in [stateg[current].out.clone(), stateg[current].out1.clone()].into_iter() {
            if let &Some(nextid) = next {
                let o = &stateg[nextid];
                write!(
                    &mut result,
                    "\"{} {}\" -> \"{} {}\";\n",
                    current,
                    stateg[current].to_string(),
                    nextid,
                    o.to_string(),
                )
                .unwrap();

                if !visited.contains(&nextid) {
                    todo.push_front(nextid);
                }
            }
        }
    }
    result
}
