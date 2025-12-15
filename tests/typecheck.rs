use dice_roller::grammar::parse_dice;
use dice_roller::typecheck::{Type, typecheck_expr};

fn typecheck(input: &str) -> Result<Type, String> {
    let parsed_expr = parse_dice(input).map_err(|e| format!("Parse error: {}", e))?;
    let expr_type = typecheck_expr(&parsed_expr);
    expr_type
}

#[test]
fn test_typecheck_constant_fold() {
    // 1. 基础数值
    let result = typecheck("42");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    // 2. 基础算术运算折叠
    let result = typecheck("6 * 7");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    let result = typecheck("10 + 32");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    let result = typecheck("100 / 2");
    assert_eq!(result.unwrap(), Type::constant(50.0));

    let result = typecheck("100 // 2");
    assert_eq!(result.unwrap(), Type::constant(50.0));

    let result = typecheck("101 // 2");
    assert_eq!(result.unwrap(), Type::constant(50.0));

    let result = typecheck("101 // 3");
    assert_eq!(result.unwrap(), Type::constant(33.0));

    let result = typecheck("-1 // 3");
    assert_eq!(result.unwrap(), Type::constant(0.0));

    let result = typecheck("-5 // 3");
    assert_eq!(result.unwrap(), Type::constant(-1.0));

    let result = typecheck("5 // -3");
    assert_eq!(result.unwrap(), Type::constant(-1.0));

    let result = typecheck("100 - 58");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    // 取模运算 (5 % 3 = 2)
    let result = typecheck("5 % 3");
    assert_eq!(result.unwrap(), Type::constant(2.0));

    // 3. 嵌套运算与优先级
    // 1 + 2 * 3 = 7
    let result = typecheck("1 + 2 * 3");
    assert_eq!(result.unwrap(), Type::constant(7.0));

    // (1 + 2) * 3 = 9
    let result = typecheck("(1 + 2) * 3");
    assert_eq!(result.unwrap(), Type::constant(9.0));

    // 4. 数学函数折叠
    // abs(-42) = 42
    let result = typecheck("abs(-42)");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    // round(1.6) = 2
    let result = typecheck("round(1.6)");
    assert_eq!(result.unwrap(), Type::constant(2.0));

    // floor(1.6) = 1
    let result = typecheck("floor(1.6)");
    assert_eq!(result.unwrap(), Type::constant(1.0));

    let result = typecheck("ceil(1.2)");
    assert_eq!(result.unwrap(), Type::constant(2.0));

    // max(1, 10, 5) = 10
    let result = typecheck("max(1, 10, 5)");
    assert_eq!(result.unwrap(), Type::constant(10.0));

    // min([1, 10, 5]) = 1
    let result = typecheck("min([1, 10, 5])");
    assert_eq!(result.unwrap(), Type::constant(1.0));

    // sum(1, 2, 3) = 6
    let result = typecheck("sum(1, 2, 3)");
    assert_eq!(result.unwrap(), Type::constant(6.0));

    // avg(1, 2, 3) = 6
    let result = typecheck("avg(1, 2, 3)");
    assert_eq!(result.unwrap(), Type::constant(2.0));
    let result = typecheck("avg(2)");
    assert_eq!(result.unwrap(), Type::constant(2.0));
    let result = typecheck("avg(1d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("avg(1, 2, 1d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("len([1, 2, 1d6])");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("len([1, 2, 3])");
    assert_eq!(result.unwrap(), Type::constant(3.0));

    // 5. 列表与数值的混合运算折叠
    // [1, 2] * 2 = [1, 2, 1, 2]
    let result = typecheck("[1, 2] * 2");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 2.0, 1.0, 2.0]));

    // [1] + [2] = [1, 2]
    let result = typecheck("[1] + [2]");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 2.0]));

    // 6. rpdice 对常量的处理 (恒等)
    let result = typecheck("rpdice(42)");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    // rpdice 实际上不改变常数列表
    let result = typecheck("rpdice([1, 2])");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 2.0]));

    // 测试其他用法
    let result = typecheck("min([1, 10, 5], 2)");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 5.0]));

    let result = typecheck("max(min([1, 10, 5], 2), 1)");
    assert_eq!(result.unwrap(), Type::const_list(vec![5.0]));

    let result = typecheck("[1, 2, 3] * 2");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0])
    );

    let result = typecheck("2 * [1, 2, max([1, 2, 3])]");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![1.0, 2.0, 3.0, 1.0, 2.0, 3.0])
    );
}

#[test]
fn test_typecheck_dice_pool_variable() {
    let result = typecheck("6d6");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("(3+3)d(max(1, 2, 6/2) + 3)");
    assert_eq!(result.unwrap(), Type::dice_pool());

    // 吸收率特殊情况（乘法）
    let result = typecheck("0 * 1d6");
    assert_eq!(result.unwrap(), Type::unknown_var());
    let result = typecheck("1d6 * 0");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("1d6 + 5");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("2d10 * 3");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let reuslt = typecheck("4d8 - 2d4");
    assert_eq!(reuslt.unwrap(), Type::unknown_var());

    let result = typecheck("sum(1d6, 2d4, 3)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("max(1d10, 5, 3d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("[1d6, 2, 3] + [4, 5d4]");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("([1d6, 2, 3] + [4, 5d4]) * 2");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("2 * ([1d6, 2, 3] + [4, 5d4]) * 2");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("[1, 2, 3] + [4, 5d4]");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("[1, 2, 3d6] + [4, 5]");
    assert_eq!(result.unwrap(), Type::var_list());
}

#[test]
fn test_typecheck_function_calls() {
    let result = typecheck("max(5)");
    assert_eq!(result.unwrap(), Type::constant(5.0));

    let result = typecheck("max(1d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("max([1, 2, 3, 4, 5], 5)");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![1.0, 2.0, 3.0, 4.0, 5.0])
    );

    let result = typecheck("max([1, 2, 3, 4, 5], 4)");
    assert_eq!(result.unwrap(), Type::const_list(vec![2.0, 3.0, 4.0, 5.0]));

    let result = typecheck("min([5, 3, 1, 4, 2], 4)");
    assert_eq!(result.unwrap(), Type::const_list(vec![3.0, 1.0, 4.0, 2.0]));

    let result = typecheck("min([5, 3, 1, 4, 1d20], 4)");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("sum([1, 2, 3, 4, 5])");
    assert_eq!(result.unwrap(), Type::constant(15.0));

    let result = typecheck("sum([1d6, 2, 3, 4, 5])");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("sum(1, 2, 3, 4, 5)");
    assert_eq!(result.unwrap(), Type::constant(15.0));

    let result = typecheck("sum(15)");
    assert_eq!(result.unwrap(), Type::constant(15.0));

    let result = typecheck("sum(1d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("ceil(1d6)");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("rpdice([1, 2, 3])");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 2.0, 3.0]));

    let result = typecheck("rpdice([1d6, 2, 3])");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("rpdice(42)");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    let result = typecheck("rpdice(1d10)");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("rpdice([1, 2, 3], 2)");
    assert_eq!(result.unwrap(), Type::const_list(vec![1.0, 2.0, 3.0]));

    let result = typecheck("rpdice([1d6, 2, 3], 2)");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("rpdice(42, 2)");
    assert_eq!(result.unwrap(), Type::constant(42.0));

    let result = typecheck("rpdice(1d10, 2)");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("min([5, 3, 1, 4, 2], 6)");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![5.0, 3.0, 1.0, 4.0, 2.0])
    );
    let result = typecheck("min([5, 3, 1, 4, 1d6], 6)");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("sort([5, 3, 1, 4, 2])");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![1.0, 2.0, 3.0, 4.0, 5.0])
    );
    let result = typecheck("sortd([5, 3, 1, 4, 2])");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![5.0, 4.0, 3.0, 2.0, 1.0])
    );
    let result = typecheck("sort(5, 3, 1, 4, 2)");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![1.0, 2.0, 3.0, 4.0, 5.0])
    );
    let result = typecheck("sortd(5, 3, 1, 4, 2)");
    assert_eq!(
        result.unwrap(),
        Type::const_list(vec![5.0, 4.0, 3.0, 2.0, 1.0])
    );
    let result = typecheck("sortd([5, 3, 1, 4, 2d6])");
    assert_eq!(result.unwrap(), Type::var_list());
    let result = typecheck("sort(5, 3, 1, 4, 3d8)");
    assert_eq!(result.unwrap(), Type::var_list());
    let result = typecheck("sort(5)");
    assert_eq!(result.unwrap(), Type::const_list(vec![5.0]));
    let result = typecheck("sortd(5)");
    assert_eq!(result.unwrap(), Type::const_list(vec![5.0]));
    let result = typecheck("sort(1d10)");
    assert_eq!(result.unwrap(), Type::var_list());
    let result = typecheck("sortd(2d10)");
    assert_eq!(result.unwrap(), Type::var_list());

    let result = typecheck("tolist(1d10)");
    assert_eq!(result.unwrap(), Type::var_list());
}

#[test]
fn test_modifier() {
    let result = typecheck("2d20kh1");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20kh2");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20dh2");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20kh");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20!");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20!!");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20!!3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20!!<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20kh!!<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20kh2!!<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!<3kh2");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!<3kh");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!ro<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!r<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!ro<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!r<3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!!");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!!!");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20!!=3l3");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20!!<(1+2)");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("2d20kh1 + 5");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("4d6kl3 * 2");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("6d10dh2 - 3");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("8d12dl4 / 2");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("2d20min10");
    assert_eq!(result.unwrap(), Type::dice_pool());

    let result = typecheck("3d20max10");
    assert_eq!(result.unwrap(), Type::dice_pool());
}

#[test]
fn test_success_check() {
    let result = typecheck("2d20 < 3");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("2d20 < max(5, (3 + 3))");
    assert_eq!(result.unwrap(), Type::unknown_var());

    let result = typecheck("1d6 >= 4 + 2");
    assert_eq!(result.unwrap(), Type::unknown_var());
}

#[test]
fn test_typecheck_invalid_expressions() {
    // 无效的算术运算
    let result = typecheck("42 + [1, 2, 3]");
    assert!(result.is_err());

    // 除0
    let result = typecheck("10 / 0");
    assert!(result.is_err());
    let result = typecheck("1d10 / 0");
    assert!(result.is_err());

    // 取模0
    let result = typecheck("10 % (1 - 1)");
    assert!(result.is_err());
    let result = typecheck("1d10 % (1 - 1)");
    assert!(result.is_err());

    // 非整数除法除0
    let result = typecheck("10 // max(-1, 2 - 2)");
    assert!(result.is_err());
    let result = typecheck("1d10 // min(0, 2)");
    assert!(result.is_err());

    // 取模非整数
    let result = typecheck("10 % 2.5");
    assert!(result.is_err());
    let result = typecheck("(1d10 + 2) % 2.5");
    assert!(result.is_err());
    let result = typecheck("10 % [1, 2]");
    assert!(result.is_err());
    let result = typecheck("2.5 % 10");
    assert!(result.is_err());

    // 整数除法非整数
    let result = typecheck("10 // 2.5");
    assert!(result.is_err());
    let result = typecheck("(1d10 + 2) // 2.5");
    assert!(result.is_err());
    let result = typecheck("10 // [1, 2]");
    assert!(result.is_err());
    let result = typecheck("2.5 // 10");
    assert!(result.is_err());

    // 无效的骰子表达式
    let result = typecheck("0d6 + 5");
    assert!(result.is_err());
    let result = typecheck("(-1)d10 * 3");
    assert!(result.is_err());
    let result = typecheck("2d0 + 4");
    assert!(result.is_err());
    let result = typecheck("4 + 2d0");
    assert!(result.is_err());
    let result = typecheck("3d(-6) - 2");
    assert!(result.is_err());
    let result = typecheck("3d(1d6) - 2");
    assert!(result.is_err());
    let result = typecheck("(1d6)d6 - 2");
    assert!(result.is_err());
    let result = typecheck("1d(5/2) - 2");
    assert!(result.is_err());
    let result = typecheck("6d[6] - 2");
    assert!(result.is_err());

    // 无效表达式传播
    let result = typecheck("(10/0)d6");
    assert!(result.is_err());
    let result = typecheck("1d(10/0)");
    assert!(result.is_err());
    let result = typecheck("[1d(10/0), 1]");
    assert!(result.is_err());

    // 嵌套列表
    let result = typecheck("[1, [2, 3]]");
    assert!(result.is_err());

    // 无效的success_check表达式
    let result = typecheck("[1, 2, 3] < 5");
    assert!(result.is_err());
    let result = typecheck("1 = 1");
    assert!(result.is_err());
    let result = typecheck("0d6=5");
    assert!(result.is_err());
    let result = typecheck("3d6<(1d6)");
    assert!(result.is_err());
    let result = typecheck("3d6<[1,2,3]");
    assert!(result.is_err());
    let result = typecheck("3d6<(0/0)");
    assert!(result.is_err());

    // 无效函数调用
    let result = typecheck("max(1d6, 2d6, 3d6, 0d6)");
    assert!(result.is_err());
    let result = typecheck("max([])");
    assert!(result.is_err());
    let result = typecheck("max()");
    assert!(result.is_err());
    let result = typecheck("max(1d6, 2d6, 3d6, [1d6, 2d6, 3d6])");
    assert!(result.is_err());

    // 列表间的非法运算
    let result = typecheck("[1, 2] * [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] / [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] - [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] % [1, 2]");
    assert!(result.is_err());

    // 列表与数值的非法运算
    let result = typecheck("[1, 2] / 2");
    assert!(result.is_err());
    let result = typecheck("2 / [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] - 2");
    assert!(result.is_err());
    let result = typecheck("2 - [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] % 2");
    assert!(result.is_err());
    let result = typecheck("2 % [1, 2]");
    assert!(result.is_err());
    let result = typecheck("2 + [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] + 2");
    assert!(result.is_err());

    // 列表与数值的非法乘法
    let result = typecheck("[1, 2] * 2.5");
    assert!(result.is_err());
    let result = typecheck("2.5 * [1, 2]");
    assert!(result.is_err());
    let result = typecheck("[1, 2] * -1");
    assert!(result.is_err());
    let result = typecheck("[1, 2] * (1d10)");
    assert!(result.is_err());
    let result = typecheck("[1, 2] * max(1d20, 2)");
    assert!(result.is_err());

    // min / max非法参数
    let result = typecheck("min([], 6)");
    assert!(result.is_err());
    let result = typecheck("max([1, 2, 3], 0)");
    assert!(result.is_err());
    let result = typecheck("max([1, 2, 3], 1.5)");
    assert!(result.is_err());
    let result = typecheck("min([5, 3, 1, 4, 1d6], 1d6)");
    assert!(result.is_err());

    // sum非法参数
    let result = typecheck("sum([1, 2, 3], 4)");
    assert!(result.is_err());
    let result = typecheck("len(1d6)");
    assert!(result.is_err());
    let reulst = typecheck("avg([1,2,3],4)");
    assert!(reulst.is_err());

    // ceil / floor / round非法参数
    let result = typecheck("ceil([1, 2, 3])");
    assert!(result.is_err());
    let result = typecheck("floor([1, 2, 3])");
    assert!(result.is_err());
    let result = typecheck("round([1, 2, 3])");
    assert!(result.is_err());
    let result = typecheck("ceil(1, 2, 3)");
    assert!(result.is_err());
    let result = typecheck("floor(1, 2, 3)");
    assert!(result.is_err());
    let result = typecheck("round(1, 2, 3)");
    assert!(result.is_err());

    // rpdice非法参数
    let result = typecheck("rpdice(42, 1.5)");
    assert!(result.is_err());
    let result = typecheck("rpdice(1d10, 1.5)");
    assert!(result.is_err());
    let result = typecheck("rpdice(1d10, -1)");
    assert!(result.is_err());
    let result = typecheck("rpdice(1d10, 1d6)");
    assert!(result.is_err());
    let result = typecheck("rpdice(1d10, 1d6, 1d6)");
    assert!(result.is_err());
    let result = typecheck("rpdice(1d10, [1, 2, 3])");
    assert!(result.is_err());

    // success_check非法参数
    let result = typecheck("1d6<(1d6)");
    assert!(result.is_err());

    // 非法modifier用法
    let result = typecheck("2d20kh0");
    assert!(result.is_err());
    let result = typecheck("2d20kh(1/0)");
    assert!(result.is_err());
    let result = typecheck("2d20kh(1d6)");
    assert!(result.is_err());
    let result = typecheck("2d20kh(max(1,2,1d6))");
    assert!(result.is_err());
    let result = typecheck("0d20dh2");
    assert!(result.is_err());
    let result = typecheck("[2d20]dh2");
    assert!(result.is_err());
    let result = typecheck("[2d20]!<3");
    assert!(result.is_err());
    let result = typecheck("[2d20]!!<3");
    assert!(result.is_err());
    let result = typecheck("[2d20]r<3");
    assert!(result.is_err());
    let result = typecheck("[2d20]ro<3");
    assert!(result.is_err());
    let result = typecheck("2d20r<(1d6)");
    assert!(result.is_err());
    let result = typecheck("2d20ro<(1d6)");
    assert!(result.is_err());
    let result = typecheck("2d20r<(10/0)");
    assert!(result.is_err());
    let result = typecheck("2d20ro<(10/0)");
    assert!(result.is_err());
    let result = typecheck("2d20!!<(10/0)");
    assert!(result.is_err());
    let result = typecheck("2d20!!<(1d10)");
    assert!(result.is_err());
    let result = typecheck("2d20!!<10l1d10");
    assert!(result.is_err());
    let result = typecheck("2d20!!<10l(-1)");
    assert!(result.is_err());
    let result = typecheck("2d20!<(10/0)");
    assert!(result.is_err());
    let result = typecheck("2d20ro<[]");
    assert!(result.is_err());
    let result = typecheck("(1/0)ro<[]");
    assert!(result.is_err());
    let result = typecheck("(1/0)ro!![]");
    assert!(result.is_err());
    let result = typecheck("1d2.1!!<3");
    assert!(result.is_err());
    let result = typecheck("2d20!!<3l(1/0)");
    assert!(result.is_err());
    let result = typecheck("2d20kh(-1)");
    assert!(result.is_err());
    let result = typecheck("3d20l3");
    assert!(result.is_err());
    let result = typecheck("3d20!!=3l(1d6)");
    assert!(result.is_err());
    let result = typecheck("sum([])");
    assert!(result.is_err());
    let result = typecheck("avg([])");
    assert!(result.is_err());
    let result = typecheck("sortd([5, 3, 1, 4, 2d6],1)");
    assert!(result.is_err());
    let result = typecheck("sortd([5, 3, 1, 4, 2d6],[5, 3, 1, 4, 2d6])");
    assert!(result.is_err());
    let result = typecheck("sortd(5, 3, 1, [4, 2d6],1)");
    assert!(result.is_err());

    let result = typecheck("tolist([1, 2, 3])");
    assert!(result.is_err());
    let result = typecheck("tolist(1d6, 1d6)");
    assert!(result.is_err());
    let result = typecheck("tolist(1d6, 1d6, 1d20)");
    assert!(result.is_err());

    let result = typecheck("2d20min0");
    assert!(result.is_err());
    let result = typecheck("3d20max0");
    assert!(result.is_err());

    let result = typecheck("2d20min(1d6)");
    assert!(result.is_err());
    let result = typecheck("3d20max(2d6)");
    assert!(result.is_err());
    let result = typecheck("2d20min(1d0)");
    assert!(result.is_err());
    let result = typecheck("3d20max(2d0)");
    assert!(result.is_err());
    let result = typecheck("2d0min10");
    assert!(result.is_err());
    let result = typecheck("3d0max10");
    assert!(result.is_err());
    let result = typecheck("1min10");
    assert!(result.is_err());
    let result = typecheck("2max10");
    assert!(result.is_err());
}
