//! This module contains functionality for parsing a regular expression into the intermediate
//! representation in repr.rs (from which it is compiled into a state graph).

#![allow(dead_code)]

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

fn err<T>(s: &str, pos: usize) -> Result<T, String> {
    Err(format!("{} at {}", s, pos))
}

fn ok<T>(ok: T) -> Result<T, String> {
    Ok(ok)
}

fn parse_enter(s: &str) -> Result<RETree, String> {
    let src: Vec<char> = s.chars().collect();
    parse_re(&src)
}

fn parse_re(src: &[char]) -> Result<RETree, String> {
    let mut s = ParseStack::new();
    let mut i = 0;
    loop {
        if i == src.len() {
            break;
        }

        println!("{:?} {}", src, i);
        match src[i] {
            '(' => {
                if let Some(end) = find_closing_paren(&src[i..], ROUND_PARENS) {
                    s.push(Pattern::Submatch(Box::new(parse_re(&src[i + 1..i + end])?)));
                    i += end;
                } else {
                    return err("couldn't find closing parenthesis", i);
                }
            }
            ')' => i += 1,
            '[' => {
                if let Some(end) = find_closing_paren(&src[i..], SQUARE_BRACKETS) {
                    s.push(parse_char_set(&src[i + 1..end])?);
                    i = end;
                } else {
                    return err("couldn't find closing square bracket", i);
                }
            }
            ']' => i += 1,
            c if c.is_alphanumeric() => {
                s.push(Pattern::Char(c));
                i += 1;
            }
            '|' => {
                let rest = parse_re(&src[i + 1..])?;
                let left = s.to_retree();
                s = ParseStack::new();
                s.push(Pattern::Alternate(vec![Box::new(left), Box::new(rest)]));
                i = src.len();
            }
            _ => {
                return err("unimplemented pattern", i);
            }
        }
    }
    ok(s.to_retree())
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
            break
        }
    }
    RETree::Concat(pats)
}

// parses the content of character classes like [abc] or [ab-] or [a-z] or [a-zA-E].
fn parse_char_set(src: &[char]) -> Result<Pattern, String> {
    if src.starts_with(&['-']) || src.ends_with(&['-']) || !src.contains(&'-') {
        return ok(Pattern::CharSet(Vec::from(src)));
    } else if src.contains(&'-') {
        // dash(es) somewhere in the middle.
        let mut set = vec![];
        let mut i = 0;
        loop {
            if i >= src.len() {
                break;
            }
            if i < src.len() - 1 && src[i + 1] == '-' && src.len() > i + 2 {
                set.push(Pattern::CharRange(src[i], src[i + 2]));
                i += 3;
            } else {
                set.push(Pattern::Char(src[i]));
                i += 1;
            }
        }
        return ok(Pattern::Alternate(set.into_iter().map(|p| Box::new(RETree::One(p))).collect()));
    }
    err("unrecognized char set", 0)
}

const ROUND_PARENS: (char, char) = ('(', ')');
const SQUARE_BRACKETS: (char, char) = ('[', ']');

// returns the index of the parenthesis closing the opening parenthesis at s[0].
fn find_closing_paren(s: &[char], parens: (char, char)) -> Option<usize> {
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
            assert_eq!(find_closing_paren(src.as_ref(), ROUND_PARENS), Some(case.1));
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
