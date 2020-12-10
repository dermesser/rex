# rex

![CI](https://github.com/dermesser/rex/workflows/CI/badge.svg)

Rex is a straight-forward regular expression engine based on state machines,
with the *secondary* goal of having similar complexity characteristics as RE2 (of course
without being so fast, as that entails a lot more work). On various pathological
REs this goal has already been achieved.

The primary goal however is to have a navigable documented code base for a
regular expression engine. For this purpose, there is an all-members-documented
documentation available at
[borgac.net/~lbo/doc/rex\_regex/rex\_regex/](https://borgac.net/~lbo/doc/rex_regex/rex_regex/).

Benchmarks can be run with `cargo bench`.

## Bugs

* Submatches can not start at the same position.
