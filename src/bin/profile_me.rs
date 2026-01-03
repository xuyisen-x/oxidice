// src/bin/profile_me.rs
use oxidice::parse_dice_and_show;
use std::hint::black_box;

fn main() {
    // 1. 准备一个稍微复杂点的表达式
    let complex_expr = "(10d6!kh3dh3 + 5d20r<2r>3) * max(floor(3.5), 2 + 2 + 4, min([1,2,3] + [1,2,3])) + filter>3([1,2,3]) + max(tolist(1d10))";

    // 2. 预热 (Warm up) - 触发 lazy_static 初始化
    let _ = parse_dice_and_show("1");

    println!("Starting profile loop...");

    // 3. 这里的循环次数要足够多，让程序至少跑 3-5 秒
    // 如果你的解析器很快，可能需要 1,000,000 次甚至更多
    for _ in 0..500_000 {
        // 使用 black_box 防止编译器把这行代码优化掉
        let _ = black_box(parse_dice_and_show(complex_expr));
    }

    println!("Done.");
}
