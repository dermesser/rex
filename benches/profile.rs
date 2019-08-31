
fn bench_complicated() {
    let re_s = "^[hH][eE]l+o +[Ww]orld!?$";
    let re = rex::compile(re_s).unwrap();
    let inputs = vec!["Hello World", "hEllo world!", "HEllllllo   world", "Helllllllllo      world!"];
    let size = inputs.len();
    println!("{}", rex::render_graph(re_s));
    for i in 0..100_000 {
        assert!(rex::match_re(&re, inputs[i%size]).0);
    }
}

fn main() {
    bench_complicated();
}
