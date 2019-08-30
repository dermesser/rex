
#[macro_use]
extern crate bencher;
extern crate regex;

use bencher::Bencher;

fn bench_simple_re(b: &mut Bencher) {
    b.iter(|| {
        assert!(rex::match_re_str("^(Hello)? [Ww]orld!?$", "Hello world").unwrap().0);
    });
}

fn bench_simple_precompile(b: &mut Bencher) {
    let re = rex::compile("^(Hello)? [Ww]orld!?$").unwrap();
    b.iter(|| {
        assert!(rex::match_re(&re, "Hello world").0);
    });
}

fn bench_notorious(b: &mut Bencher) {
    let re = rex::compile("(x+x+)+y").unwrap();
    b.iter(|| {
        assert!(rex::match_re(&re, "xxxxxxxxxxxy").0);
    });
}

fn bench_regex_crate(b: &mut Bencher) {
    let re = regex::Regex::new("^(Hello)? [Ww]orld!?$").unwrap();
    b.iter(|| {
        assert!(re.is_match("Hello World"));
    });
}

benchmark_group!(benchs, bench_simple_re, bench_simple_precompile, bench_notorious, bench_regex_crate);
benchmark_main!(benchs);
