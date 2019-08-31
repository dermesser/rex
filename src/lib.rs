#![allow(dead_code)]

mod compile;
mod matcher;
mod matching;
mod parse;
mod repr;
mod state;

mod tests;

pub fn render_graph(re: &str) -> String {
    return format!(
        "digraph st {{ {} }}",
        state::dot(&compile::start_compile(parse::parse(re).as_ref().unwrap()))
    );
}

fn parse(re: &str) -> Result<repr::Pattern, String> {
    return parse::parse(re);
}

/// Compiles a parsed regular expression into the internal state graph and matches s against it.
/// Returns whether the string matched as well as a list of submatches. The first submatch is the
/// entire matched string. A submatch is a tuple of (start, end), where end is the index of the
/// first character that isn't part of the submatch anymore (i.e. [start, end)).
fn compile_and_match(re: &repr::Pattern, s: &str) -> (bool, Vec<(usize, usize)>) {
    let compiled = compile::start_compile(re);
    matching::do_match(&compiled, s)
}

/// Parse, compile, and match a regular expression. Not recommended for repeated use, as the
/// regular expression will be compiled every time. Use `compile()` and `match_re()` to make this
/// more efficient (about 3x faster).
pub fn match_re_str(re: &str, s: &str) -> Result<(bool, Vec<(usize, usize)>), String> {
    return Ok(compile_and_match(
        &repr::optimize::optimize(parse::parse(re)?),
        s,
    ));
}

/// Compile a regular expression into a representation that can be directly used for matching with
/// `match_re()`.
pub fn compile(re: &str) -> Result<state::CompiledRE, String> {
    Ok(compile::start_compile(&repr::optimize::optimize(parse(
        re,
    )?)))
}

/// Match a regular expression compiled with `compile()` against a string. Returns a tuple of a
/// boolean (whether there was a match or partial match) and a vector of `(position, length)`
/// tuples for all submatches, where the first element describes the match by the whole regular
/// expression.
pub fn match_re(re: &state::CompiledRE, s: &str) -> (bool, Vec<(usize, usize)>) {
    matching::do_match(re, s)
}
