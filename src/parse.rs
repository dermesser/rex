//! This module contains functionality for parsing a regular expression into the intermediate
//! representation in repr.rs (from which it is compiled into a state graph).

#![allow(dead_code)]

use std::ops::{Index, Range, RangeFull};

use repr::{self, RETree, Pattern, AnchorLocation, Repetition};

struct ParseStack {
    s: Vec<Pattern>,
}

impl ParseStack {
    fn new() -> ParseStack {
        ParseStack { s: Vec::with_capacity(4) }
    }
    fn push(&mut self, p: Pattern) {
        self.s.push(p)
    }
    fn pop(&mut self) -> Option<Pattern> {
        self.s.pop()
    }
    fn to_retree(mut self) -> RETree {
        if self.s.len() > 1 {
            RETree::Concat(self.s)
        } else if self.s.len() == 1 {
            RETree::One(self.s.pop().unwrap())
        } else {
            panic!("empty retree")
        }
    }
}

/// State of the parser, quite a simple struct. It contains the current substring that a parser
/// function is concerned with as well as the position within the overall parsed string, so that
/// useful positions can be reported to users. In addition, it provides functions to create
/// "sub-ParseStates" containing a substring of its current string.
/// 
/// It also supports indexing by ranges and index.
struct ParseState<'a> {
    /// The string to parse. This may be a substring of the "overall" matched string.
    src: &'a [char],
    /// The position within the overall string (for error reporting).
    pos: usize,
}

impl<'a> ParseState<'a> {
    fn new(s: &'a [char]) -> ParseState<'a> {
        ParseState { src: s, pos: 0 }
    }
    fn from(&self, count: usize) -> ParseState<'a> {
        self.sub(count, self.len())
    }
    fn sub(&self, from: usize, to: usize) -> ParseState<'a> {
        ParseState {
            src: &self.src[from..to],
            pos: self.pos + from,
        }
    }
    fn len(&self) -> usize {
        self.src.len()
    }
    fn err<T>(&self, s: &str, i: usize) -> Result<T, String> {
        Err(format!("{} (at {})", s, self.pos + i))
    }
    fn ok<T>(&self, ok: T) -> Result<T, String> {
        Ok(ok)
    }
}

impl<'a> Index<Range<usize>> for ParseState<'a> {
    type Output = [char];
    fn index(&self, r: Range<usize>) -> &Self::Output {
        &self.src[r]
    }
}
impl<'a> Index<RangeFull> for ParseState<'a> {
    type Output = [char];
    fn index(&self, r: RangeFull) -> &Self::Output {
        &self.src[r]
    }
}
impl<'a> Index<usize> for ParseState<'a> {
    type Output = char;
    fn index(&self, i: usize) -> &Self::Output {
        &self.src[i]
    }
}

fn parse_enter(s: &str) -> Result<RETree, String> {
    let src: Vec<char> = s.chars().collect();
    parse_re(ParseState::new(&src))
}

fn parse_re<'a>(s: ParseState<'a>) -> Result<RETree, String> {
    let mut st = ParseStack::new();
    let mut i = 0;
    loop {
        if i == s.len() {
            break;
        }

        println!("{:?} {}", &s[..], i);
        match s[i] {
            c if c.is_alphanumeric() => {
                st.push(Pattern::Char(c));
                i += 1;
            }
            '|' => {
                let rest = parse_re(s.from(i + 1))?;
                let left = st.to_retree();
                st = ParseStack::new();
                st.push(Pattern::Alternate(vec![Box::new(left), Box::new(rest)]));
                i = s.len();
            }
            '(' => {
                if let Some(end) = find_closing_paren(s.from(i), ROUND_PARENS) {
                    st.push(Pattern::Submatch(Box::new(parse_re(s.sub(i + 1, i + end))?)));
                    i += end;
                } else {
                    return s.err("couldn't find closing parenthesis", i);
                }
            }
            ')' => i += 1,
            '[' => {
                if let Some(end) = find_closing_paren(s.from(i), SQUARE_BRACKETS) {
                    st.push(parse_char_set(s.sub(i + 1, end))?);
                    i = end;
                } else {
                    return s.err("couldn't find closing square bracket", i);
                }
            }
            ']' => i += 1,
            _ => {
                return s.err("unimplemented pattern", i);
            }
        }
    }
    s.ok(st.to_retree())
}

// flatten_alternate tries to flatten a nested Alternate structure: Alternate(A, Alternate(B, C))
// -> Alternate(A, B, C)
fn flatten_alternate(mut re: RETree) -> RETree {
    let mut alts = vec![];
    let mut pats = vec![];

    match re {
        RETree::One(p) => pats.push(p),
        RETree::Concat(ref mut ps) => pats.append(ps),
    }

    loop {
        if let Some(pat) = pats.pop() {
            match pat {
                Pattern::Alternate(v) => {
                    for mut a in v.into_iter() {
                        match *a {
                            RETree::One(p) => alts.push(RETree::One(p)),
                            RETree::Concat(ref mut ps) => pats.append(ps),
                        }
                    }
                }
                _ => {}
            }
        } else {
            break;
        }
    }
    RETree::Concat(pats)
}

// parses the content of character classes like [abc] or [ab-] or [a-z] or [a-zA-E].
fn parse_char_set<'a>(s: ParseState<'a>) -> Result<Pattern, String> {
    if s[0] == '-' || s[s.len() - 1] == '-' || !s[..].contains(&'-') {
        return s.ok(Pattern::CharSet(Vec::from(&s[..])));
    } else if s[..].contains(&'-') {
        // dash(es) somewhere in the middle.
        let mut set = vec![];
        let mut i = 0;
        loop {
            if i >= s.len() {
                break;
            }
            if i < s.len() - 1 && s[i + 1] == '-' && s.len() > i + 2 {
                set.push(Pattern::CharRange(s[i], s[i + 2]));
                i += 3;
            } else {
                set.push(Pattern::Char(s[i]));
                i += 1;
            }
        }
        return s.ok(Pattern::Alternate(set.into_iter().map(|p| Box::new(RETree::One(p))).collect()));
    }
    s.err("unrecognized char set", 0)
}

const ROUND_PARENS: (char, char) = ('(', ')');
const SQUARE_BRACKETS: (char, char) = ('[', ']');

// returns the index of the parenthesis closing the opening parenthesis at s[0].
fn find_closing_paren<'a>(s: ParseState<'a>, parens: (char, char)) -> Option<usize> {
    let mut count = 0;
    for i in 0..s.len() {
        if s[i] == parens.0 {
            count += 1;
        } else if s[i] == parens.1 {
            count -= 1;
        }

        if count == 0 {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::start_compile;
    use state::dot;

    #[test]
    fn test_find_closing_paren() {
        for case in &[("(abc)de", 4), ("()a", 1), ("(abcd)", 5)] {
            let src: Vec<char> = case.0.chars().collect();
            assert_eq!(find_closing_paren(ParseState::new(src.as_ref()), ROUND_PARENS),
                       Some(case.1));
        }
    }

    #[test]
    fn test_parse_manual() {
        let rep = parse_enter("a|b|c").unwrap();
        println!("{:?}", flatten_alternate(rep.clone()));

        let dot = dot(start_compile(rep));
        println!("digraph st {{ {} }}", dot);
    }
}
