
#[macro_use]
extern crate bencher;

use bencher::Bencher;

fn bench_simple_re(b: &mut Bencher) {
    b.iter(|| {
        assert!(rex::match_re_str("(Hello)? [Ww]orld!?", "Hello world").unwrap().0);
    });
}

benchmark_group!(benchs, bench_simple_re);
benchmark_main!(benchs);
