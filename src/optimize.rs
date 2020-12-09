/// optimize contains functionality for optimizing the representation of regular expressions for
/// more efficient matching.
use std::iter::{FromIterator, Iterator};
use std::ops::Deref;

use crate::repr::{Pattern, Repetition};

pub fn optimize(mut p: Pattern) -> Pattern {
    p = concat_chars_to_str(p);
    p = flatten_alternate(p);
    p = optimize_recursively(p);
    p
}

/// optimize_recursively applies optimize() to the inner Patterns of a Pattern.
fn optimize_recursively(p: Pattern) -> Pattern {
    match p {
        Pattern::Concat(ps) => Pattern::Concat(ps.into_iter().map(optimize).collect()),
        Pattern::Submatch(bp) => {
            let sub = optimize(bp.deref().clone());
            Pattern::Submatch(Box::new(sub))
        }
        Pattern::Alternate(ps) => Pattern::Alternate(ps.into_iter().map(optimize).collect()),
        Pattern::Repeated(r) => {
            let rep = r.deref().clone();
            Pattern::Repeated(Box::new(match rep {
                Repetition::ZeroOrOnce(rp) => Repetition::ZeroOrOnce(optimize(rp)),
                Repetition::ZeroOrMore(rp) => Repetition::ZeroOrMore(optimize(rp)),
                Repetition::OnceOrMore(rp) => Repetition::OnceOrMore(optimize(rp)),
                Repetition::Specific(rp, min, max) => Repetition::Specific(optimize(rp), min, max),
            }))
        }

        p => p,
    }
}

/// concat_chars_to_str collapses successive single-character patterns into a single string
/// pattern.
fn concat_chars_to_str(p: Pattern) -> Pattern {
    match p {
        Pattern::Concat(mut v) => {
            let mut drain = v.drain(..);
            let mut new_elems = vec![];

            // Find runs of adjacent chars/strings and convert them to single strings.
            // Once a run is broken, append non-char/string patterns and continue afterwards.
            let mut chars = vec![];
            for cp in &mut drain {
                match cp {
                    Pattern::Char(c) => chars.push(c),
                    Pattern::Str(mut s) => {
                        chars.extend(s.drain(..));
                    }
                    e => {
                        // Once a run of chars/strings is broken, merge the run, push the
                        // non-char/string pattern and continue with the next one.
                        if chars.len() == 1 {
                            new_elems.push(Pattern::Char(chars.pop().unwrap()))
                        } else if chars.len() > 1 {
                            let newp = Pattern::Str(String::from_iter(chars.drain(..)));
                            new_elems.push(newp);
                        }
                        new_elems.push(e);
                        assert!(chars.is_empty());
                    }
                }
            }

            if chars.len() == 1 {
                new_elems.push(Pattern::Char(chars[0]));
            } else if chars.len() > 1 {
                let newp = Pattern::Str(String::from_iter(chars.drain(..)));
                new_elems.push(newp);
            }

            if new_elems.len() == 1 {
                new_elems.pop().unwrap()
            } else {
                Pattern::Concat(new_elems)
            }
        }
        _ => p,
    }
}

/// flatten_alternate takes the alternatives in a Pattern::Alternate and reduces the nesting
/// recursively.
fn flatten_alternate(p: Pattern) -> Pattern {
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

    let mut fa = _flatten_alternate(p);
    if fa.len() == 1 {
        fa.pop().unwrap()
    } else {
        Pattern::Alternate(fa)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repr::AnchorLocation;

    #[test]
    fn test_repr_optimize() {
        // case = (want, input)
        let case1 = (
            Pattern::Str("abc".to_string()),
            Pattern::Concat(vec![
                Pattern::Char('a'),
                Pattern::Char('b'),
                Pattern::Char('c'),
            ]),
        );
        let case2 = (
            Pattern::Str("abcd".to_string()),
            Pattern::Concat(vec![
                Pattern::Str("a".to_string()),
                Pattern::Char('b'),
                Pattern::Str("cd".to_string()),
            ]),
        );
        let case3 = (
            Pattern::Concat(vec![
                Pattern::Str("abc".to_string()),
                Pattern::Anchor(AnchorLocation::End),
                Pattern::Char('d'),
            ]),
            Pattern::Concat(vec![
                Pattern::Char('a'),
                Pattern::Char('b'),
                Pattern::Char('c'),
                Pattern::Anchor(AnchorLocation::End),
                Pattern::Char('d'),
            ]),
        );

        for c in vec![case1, case2, case3].into_iter() {
            assert_eq!(c.0, optimize(c.1));
        }
    }
}
