
use std::cell::RefCell;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::iter::FromIterator;
use std::fmt::{Debug, Write};
use std::rc::Rc;
use std::string;

/// Matchee contains a character and position to match.
struct Matchee {
    /// Current index within the matched string.
    ix: usize,
    /// Character at current position.
    c: char,
}

/// Matcher matches characters.
trait Matcher: Debug {
    fn matches(&self, m: &Matchee) -> bool;
}

#[derive(Debug)]
struct CharMatcher(char);
impl Matcher for CharMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        m.c == self.0
    }
}

#[derive(Debug)]
struct CharRangeMatcher(char, char);
impl Matcher for CharRangeMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        m.c >= self.0 && m.c <= self.1
    }
}

#[derive(Debug)]
struct CharSetMatcher(Vec<char>);
impl Matcher for CharSetMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        self.0.contains(&m.c)
    }
}

#[derive(Debug)]
struct AnyMatcher;
impl Matcher for AnyMatcher {
    fn matches(&self, m: &Matchee) -> bool {
        true
    }
}

fn wrap_matcher(m: Box<Matcher>) -> Option<Rc<Box<Matcher>>> {
    Some(Rc::new(m))
}

/// State is a single state that the evaluation can be in. It contains several output states as
/// well as a matcher.
#[derive(Debug, Clone)]
struct State {
    // Possible following state(s).
    out: Option<WrappedState>,
    out1: Option<WrappedState>,
    // If matcher is none, this is an "empty" state.
    matcher: Option<Rc<Box<Matcher>>>,
    // True if this is the last state in the chain, i.e. an re matches.
    last: bool,
}

/// WrappedState is a shared pointer to a state node.
type WrappedState = Rc<RefCell<State>>;

fn wrap_state(s: State) -> WrappedState {
    Rc::new(RefCell::new(s))
}

impl State {
    fn patch(&mut self, next: WrappedState) {
        if self.out.is_none() {
            self.out = Some(next);
        } else if self.out1.is_none() {
            self.out1 = Some(next);
        } else {
            unimplemented!()
        }
    }
}

/// dot converts a graph starting with s into a Dot graph.
fn dot(s: WrappedState) -> String {
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
                       o.borrow().matcher);

                if !visited.contains(&nextid) {
                    todo.push_front(o.clone());
                }
            }
        }
    }
    result
}

/// Types implementing Compile can be compiled into a state graph.
trait Compile {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>);
}

/// start_compile takes a parsed regex as RETree and returns the first node of a directed graph
/// representing the regex.
fn start_compile(re: RETree) -> WrappedState {
    let (s, mut sp) = re.to_state();
    let end = wrap_state(State {
        out: None,
        out1: None,
        last: true,
        matcher: None,
    });
    // Connect all loose ends with the final node.
    for mut p in sp {
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
                let (mut init, mut lastp) = ps[0].to_state();
                for i in 1..ps.len() {
                    let (next, mut nextp) = ps[i].to_state();
                    // Connect all loose ends with the new node.
                    for mut p in lastp {
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
    Chars(char, char),
    CharSet(Vec<char>),
}

impl Compile for Pattern {
    fn to_state(&self) -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Pattern::Chars(from, to) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(CharRangeMatcher(from, to))),
                    last: false,
                });
                (s.clone(), vec![s])
            }
            Pattern::CharSet(ref chars) => {
                let s = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: wrap_matcher(Box::new(CharSetMatcher(chars.clone()))),
                    last: false,
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
                    last: false,
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
            _ => unimplemented!(),
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
              mut s: WrappedState,
              mut to_patch: Vec<WrappedState>)
              -> (WrappedState, Vec<WrappedState>) {
        match *self {
            Repetition::Once => (s.clone(), vec![s]),
            Repetition::ZeroOrOnce => {
                let after = wrap_state(State {
                    out: None,
                    out1: None,
                    matcher: None,
                    last: false,
                });
                let before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: Some(after.clone()),
                    matcher: None,
                    last: false,
                });
                for mut p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::ZeroOrMore => {
                let mut before = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    last: false,
                });
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    last: false,
                });
                before.borrow_mut().out1 = Some(after.clone());
                for mut p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (before, vec![after])
            }
            Repetition::OnceOrMore => {
                let after = wrap_state(State {
                    out: Some(s.clone()),
                    out1: None,
                    matcher: None,
                    last: false,
                });
                for mut p in to_patch {
                    p.borrow_mut().patch(after.clone());
                }
                (s, vec![after])
            }
            // Specific is 'min' concatenations of a simple state and 'max - min' concatenations of
            // a ZeroOrOnce state.
            Repetition::Specific(min, max) => unimplemented!(),
            _ => unimplemented!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // /a(b|c)/
    fn simple_re0() -> RETree {
        RETree::Concat(vec![Pattern::Chars('a', 'a'),
                            Pattern::Alternate(Box::new(RETree::One(Pattern::Chars('b', 'b'))),
                                               Box::new(RETree::One(Pattern::Chars('c', 'c'))))])
    }
    // Returns compiled form of /(ab)?(cd)*(e|f)+x(g|h)i/
    fn simple_re1() -> RETree {
        RETree::Concat(vec!(
                Pattern::Repeated(
                    Box::new(
                        RETree::Concat(vec!(
                            Pattern::Chars('a', 'a'), Pattern::Chars('b', 'b')))),
                        Repetition::ZeroOrOnce),

                Pattern::Repeated(
                    Box::new(
                        RETree::Concat(vec!(
                            Pattern::Chars('c', 'c'), Pattern::Chars('d', 'd')))),
                        Repetition::ZeroOrMore),

                Pattern::Repeated(
                    Box::new(
                        RETree::One(
                            Pattern::Alternate(
                                Box::new(RETree::One(Pattern::Chars('e', 'e'))),
                                Box::new(RETree::One(Pattern::Chars('f', 'f')))))),
                        Repetition::OnceOrMore),


                Pattern::Chars('x', 'x'),
                Pattern::Alternate(Box::new(RETree::One(Pattern::Chars('g', 'g'))), Box::new(RETree::One(Pattern::Chars('h', 'h')))),
                Pattern::Chars('i', 'i'),
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
