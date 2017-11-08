
#![allow(dead_code)]

mod compile;
mod matcher;
mod matching;
mod parse;
mod repr;
mod state;

/// Compiles a parsed regular expression into the internal state graph and matches s against it.
/// Returns whether the string matched as well as a list of submatches. The first submatch is the
/// entire matched string. A submatch is a tuple of (start, end), where end is the index of the
/// first character that isn't part of the submatch anymore (i.e. [start, end)).
fn compile_and_match(re: &repr::Pattern, s: &str) -> (bool, Vec<(usize, usize)>) {
    matching::do_match(compile::start_compile(re), s)
}
