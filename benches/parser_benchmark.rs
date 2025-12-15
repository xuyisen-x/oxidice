// benches/parser_benchmark.rs
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use oxidice::grammar::parse_dice; // 假设你的 lib 名字叫 oxidice

fn criterion_benchmark(c: &mut Criterion) {
    let complex_expr = "(10d6!kh3 + 5d20r<2) * max(floor(3.5), 2) + filter>3([1,2,3])";

    // ==========================================
    // 关键步骤：预热 (Warm Up)
    // ==========================================
    // 在进入 bench 闭包之前，先随便跑一次。
    // 这会强制触发 lazy_static 的初始化，把它排除在测试范围之外。
    let _ = parse_dice("1");

    c.bench_function("parse complex dice", |b| {
        // Criterion 会自动帮你运行成千上万次，直到统计数据稳定
        b.iter(|| parse_dice(black_box(complex_expr)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
