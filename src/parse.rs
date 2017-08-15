//! This module contains functionality for parsing a regular expression into the intermediate
//! representation in repr.rs (from which it is compiled into a state graph), and optimizing that
//! intermediate representation.

#![allow(dead_code)]

use std::ops::{Index, Range, RangeFull};

use repr::Pattern;

pub fn parse(s: &str) -> Result<Pattern, String> {
    let src: Vec<char> = s.chars().collect();
    parse_re(ParseState::new(&src)).map(|t| t.0)
}

/// ParseStack contains already parsed elements of a regular expression. It can be converted to an
/// RETree.
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
    fn to_retree(mut self) -> Pattern {
        if self.s.len() > 1 {
            Pattern::Concat(self.s)
        } else if self.s.len() == 1 {
            self.s.pop().unwrap()
        } else {
            panic!("empty stack")
        }
    }
}

/// State of the parser, quite a simple struct. It contains the current substring that a parser
/// function is concerned with as well as the position within the overall parsed string, so that
/// useful positions can be reported to users. In addition, it provides functions to cheaply create
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
    /// new returns a new ParseState operating on the specified input string.
    fn new(s: &'a [char]) -> ParseState<'a> {
        ParseState { src: s, pos: 0 }
    }
    /// from returns a new ParseState operating on the [from..] sub-string of the current
    /// ParseState.
    fn from(&self, from: usize) -> ParseState<'a> {
        self.sub(from, self.len())
    }
    /// sub returns a sub-ParseState containing [from..to] of the current one.
    fn sub(&self, from: usize, to: usize) -> ParseState<'a> {
        ParseState {
            src: &self.src[from..to],
            pos: self.pos + from,
        }
    }
    /// len returns how many characters this ParseState contains.
    fn len(&self) -> usize {
        self.src.len()
    }
    /// err returns a formatted error string containing the specified message and the overall
    /// position within the original input string.
    fn err<T>(&self, s: &str, i: usize) -> Result<T, String> {
        Err(format!("{} at {}", s, self.pos + i))
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
impl<'a> Clone for ParseState<'a> {
    fn clone(&self) -> ParseState<'a> {
        ParseState {
            src: self.src,
            pos: self.pos,
        }
    }
}

// parse_re is the parser entry point; like all parser functions, it returns either a pair of
// (parsed pattern, new ParseState) or an error string.
fn parse_re<'a>(mut s: ParseState<'a>) -> Result<(Pattern, ParseState<'a>), String> {
    // The stack assists us in parsing the linear parts of a regular expression, e.g. non-pattern
    // characters, or character sets.
    let mut stack = ParseStack::new();
    loop {
        if s.len() == 0 {
            break;
        }

        println!("{:?} {}", &s[..], s.pos);
        match s[0] {
            c if c.is_alphanumeric() => {
                stack.push(Pattern::Char(c));
                s = s.from(1)
            }
            // Alternation: Parse the expression on the right of the pipe sign and push an
            // alternation between what we've already seen and the stuff on the right.
            '|' => {
                let (rest, newst) = parse_re(s.from(1))?;
                let left = stack.to_retree();
                stack = ParseStack::new();
                stack.push(Pattern::Alternate(vec![left, rest]));
                s = newst;
            }
            '(' => {
                match split_in_parens(s.clone(), ROUND_PARENS) {
                    Some((parens, newst)) => {
                        // Parse the sub-regex within parentheses.
                        let (pat, rest) = parse_re(parens)?;
                        assert!(rest.len() == 0);

                        stack.push(Pattern::Submatch(Box::new(pat)));
                        // Set the current state to contain the string after the parentheses.
                        s = newst;
                    }
                    None => return s.err("unmatched (", s.len()),
                }
            }
            '[' => {
                match parse_char_set(s) {
                    Ok((pat, newst)) => {
                        stack.push(pat);
                        s = newst;
                    }
                    Err(e) => return Err(e),
                }
            }
            _ => {
                return s.err("unimplemented pattern", 0);
            }
        }
    }
    Ok((stack.to_retree(), s))
}

// parses the content of character classes like [abc] or [ab-] or [a-z] or [a-zA-E].
fn parse_char_set<'a>(s: ParseState<'a>) -> Result<(Pattern, ParseState<'a>), String> {
    if let Some((cs, rest)) = split_in_parens(s.clone(), SQUARE_BRACKETS) {
        // TODO: Parse patterns like [-abc-g].
        if cs[0] == '-' || cs[cs.len() - 1] == '-' || !cs[..].contains(&'-') {
            Ok((Pattern::CharSet(Vec::from(&cs[..])), rest))
        } else if cs[..].contains(&'-') {
            // dash(es) somewhere in the middle.
            let mut set = vec![];
            let mut i = 0;
            loop {
                if i >= cs.len() {
                    break;
                }
                if i < cs.len() - 1 && cs[i + 1] == '-' && cs.len() > i + 2 {
                    set.push(Pattern::CharRange(cs[i], cs[i + 2]));
                    i += 3;
                } else {
                    set.push(Pattern::Char(cs[i]));
                    i += 1;
                }
            }
            Ok((Pattern::Alternate(set), rest))
        } else {
            s.err("unrecognized charset", 0)
        }
    } else {
        s.err("unmatched [", s.len())
    }
}

const ROUND_PARENS: (char, char) = ('(', ')');
const SQUARE_BRACKETS: (char, char) = ('[', ']');
const CURLY_BRACKETS: (char, char) = ('{', '}');

// split_in_parens returns two new ParseStates; the first one containing the contents of the
// parenthesized clause starting at s[0], the second one containing the rest.
fn split_in_parens<'a>(s: ParseState<'a>,
                       parens: (char, char))
                       -> Option<(ParseState<'a>, ParseState<'a>)> {
    if let Some(end) = find_closing_paren(s.clone(), parens) {
        Some((s.sub(1, end), s.from(end + 1)))
    } else {
        None
    }
}

// find_closing_paren returns the index of the parenthesis closing the opening parenthesis at the
// beginning of the state's string.
fn find_closing_paren<'a>(s: ParseState<'a>, parens: (char, char)) -> Option<usize> {
    if s[0] != parens.0 {
        return None;
    }
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

/// The optimize module contains functionality for optimizing Patterns before compiling it to a
/// State graph.
mod optimize {
    use repr::Pattern;

    pub fn optimize(mut p: Pattern) -> Pattern {
        p = flatten_alternate(p);
        // TODO: Define more optimizations.
        p
    }

    // flatten_alternate takes the alternatives in a Pattern::Alternate and reduces the nesting
    // recursively.
    pub fn flatten_alternate(p: Pattern) -> Pattern {
        fn _flatten_alternate(p: Pattern) -> Vec<Pattern> {
            match p {
                Pattern::Alternate(a) => {
                    let mut alternatives = vec![];
                    for alt in a.into_iter() {
                        alternatives.append(&mut _flatten_alternate(alt));
                    }
                    alternatives
                }
                p_ => vec![p_],
            }
        }
        Pattern::Alternate(_flatten_alternate(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repr::start_compile;
    use state::dot;

    #[test]
    fn test_find_closing_paren() {
        for case in &[("(abc)de", Some(4)), ("()a", Some(1)), ("(abcd)", Some(5)), ("(abc", None)] {
            let src: Vec<char> = case.0.chars().collect();
            assert_eq!(find_closing_paren(ParseState::new(src.as_ref()), ROUND_PARENS),
                       case.1);
        }

    }

    #[test]
    fn test_parse_manual() {
        let rep = parse("a|[bed]|(c|d|e)|f").unwrap();
        println!("{:?}\n{:?}",
                 rep.clone(),
                 optimize::flatten_alternate(rep.clone()));

        let dot = dot(start_compile(&rep));
        println!("digraph st {{ {} }}", dot);
    }

    #[test]
    fn test_parse_manual2() {
        println!("{:?}", parse("a([bc)def"));
    }
}
