use crate::grammar::CompareExpr;

use super::grammar::{BinOp, Expr};

// ==========================================
// 类型定义
// ==========================================

#[derive(Clone, PartialEq, Debug)]
pub enum VariableNumber {
    Unknown,  // 未知的变量数值
    DicePool, // 来自骰池的变量数值
}

#[derive(Clone, PartialEq, Debug)]
pub enum NumberType {
    Constant(f64),            // 常数数值
    Variable(VariableNumber), // 变量数值
}

#[derive(Clone, PartialEq, Debug)]
pub enum ListType {
    ConstantList(Vec<f64>), // 常数列表
    VariableList,           // 变量列表，不记录长度了，支持变长的列表
}

#[derive(Clone, PartialEq, Debug)]
pub enum Type {
    Number(NumberType), // 数值类型
    List(ListType),     // 列表类型
}

// ==========================================
// Type 构造辅助函数
// ==========================================

impl Type {
    pub fn constant(val: f64) -> Self {
        Type::Number(NumberType::Constant(val))
    }

    pub fn unknown_var() -> Self {
        Type::Number(NumberType::Variable(VariableNumber::Unknown))
    }

    pub fn dice_pool() -> Self {
        Type::Number(NumberType::Variable(VariableNumber::DicePool))
    }

    pub fn const_list(list: Vec<f64>) -> Self {
        Type::List(ListType::ConstantList(list))
    }

    pub fn var_list() -> Self {
        Type::List(ListType::VariableList)
    }
}

// ==========================================
// 主类型检查函数
// ==========================================

pub fn typecheck_expr(expr: &Expr) -> Result<Type, String> {
    match expr {
        Expr::Number(x) => Ok(Type::constant(*x)),
        Expr::Dice { count, side } => type_of_dice(count, side),
        Expr::List(args) => type_of_list(args),
        Expr::Binary { lhs, op, rhs } => type_of_binary_op(lhs, op, rhs),
        Expr::SuccessCheck { lhs, compare_expr } => type_of_success_check(lhs, compare_expr),
        Expr::KeepOrDropModifier { lhs, op: _, count } => type_of_keep_drop_modifier(lhs, count),
        Expr::ExplodeModifier {
            lhs,
            op: _,
            compare_expr,
            limit,
        } => type_of_explode_modifier(lhs, compare_expr, limit),
        Expr::RerollModifier {
            lhs,
            op: _,
            compare_expr,
        } => type_of_reroll_modifier(lhs, compare_expr),
        Expr::MinMaxModifier { lhs, op: _, target } => type_of_min_max_modifier(lhs, target),
        Expr::Call { func_name, args } => type_of_call(func_name, args), // 最复杂放到最后处理
    }
}

// ==========================================
// 辅助处理函数
// ==========================================

// 判断一个浮点数是否为整数
pub fn is_integer(num: f64) -> bool {
    (num - num.round()).abs() < 1e-9
}

// 从切片中选出前 n 个最大值或最小值，并按原顺序返回
pub fn top_n_preserve_order<T: Clone + PartialOrd>(
    data: &[T],
    n: usize,
    largest: bool, // true: 取最大值；false: 取最小值
) -> Vec<T> {
    let len = data.len();
    if n == 0 || len == 0 {
        return Vec::new();
    }

    let n = n.min(len);

    // 1) 带上原始下标
    let mut indexed: Vec<(usize, &T)> = data.iter().enumerate().collect();

    // 2) 按值排序（根据 largest 决定升序/降序）
    indexed.sort_by(|a, b| {
        match a.1.partial_cmp(b.1) {
            Some(ord) => {
                if largest {
                    ord.reverse() // 最大值优先
                } else {
                    ord // 最小值优先
                }
            }
            None => std::cmp::Ordering::Equal,
        }
    });

    // 3) 取前 n 个下标
    let mut top_n: Vec<(usize, &T)> = indexed.into_iter().take(n).collect();

    // 4) 再按原始下标排序，保证“保持原来顺序”
    top_n.sort_by_key(|(idx, _)| *idx);

    // 5) 把结果按顺序抽出来
    top_n.into_iter().map(|(_, v)| v.clone()).collect()
}

// ==========================================
// 各类表达式的类型处理函数
// ==========================================

fn type_of_dice(count: &Expr, side: &Expr) -> Result<Type, String> {
    use NumberType::*;
    use Type::*;

    let count_type = typecheck_expr(count)?;
    let side_type = typecheck_expr(side)?;
    match (count_type, side_type) {
        // 两边必须都是常数
        (Number(Constant(c)), Number(Constant(s))) => {
            if is_integer(c) && is_integer(s) && c > 0.0 && s >= 2.0 {
                Ok(Type::dice_pool())
            } else {
                Err(format!(
                    "Invalid dice parameters: count = {}, side = {}",
                    c, s
                ))
            }
        }
        // 针对变量的特殊警告
        (Number(Variable(_)), _) | (_, Number(Variable(_))) => {
            Err("Dice count and side must be constant numbers.".to_string())
        }
        _ => Err("Dice count and side must be numbers.".to_string()),
    }
}

fn type_of_list(args: &Vec<Expr>) -> Result<Type, String> {
    use NumberType::*;
    use Type::*;
    let mut consts = Vec::new();
    let mut is_var_list = false;
    for arg in args {
        let arg_type = typecheck_expr(arg)?;
        match arg_type {
            List(_) => return Err("Nested lists are not allowed.".to_string()), // 不允许嵌套列表
            Number(Variable(_)) => is_var_list = true,                          // 统计变量数值
            Number(Constant(c)) => {
                if !is_var_list {
                    consts.push(c)
                }
            } // 收集常数数值
        }
    }
    if is_var_list {
        Ok(Type::var_list())
    } else {
        Ok(Type::const_list(consts))
    }
}

fn type_of_binary_op(lhs: &Expr, op: &BinOp, rhs: &Expr) -> Result<Type, String> {
    use ListType::*;
    use NumberType::*;
    use Type::*;

    let lhs_type = typecheck_expr(lhs)?;
    let rhs_type = typecheck_expr(rhs)?;
    match (lhs_type, rhs_type) {
        // 两个标量数值之间的操作
        (Number(lt), Number(rt)) => {
            match (lt, rt) {
                (Constant(lc), Constant(rc)) => {
                    // 常数与常数之间的操作，结果仍为常数
                    match op {
                        BinOp::Add => Ok(Type::constant(lc + rc)),
                        BinOp::Sub => Ok(Type::constant(lc - rc)),
                        BinOp::Mul => Ok(Type::constant(lc * rc)),
                        BinOp::Div => {
                            if rc != 0.0 {
                                Ok(Type::constant(lc / rc))
                            } else {
                                Err("Division by zero.".to_string())
                            }
                        }
                        BinOp::Mod => {
                            if rc == 0.0 {
                                Err("Modulo by zero.".to_string())
                            } else if is_integer(lc) && is_integer(rc) {
                                Ok(Type::constant((lc as i64 % rc as i64) as f64))
                            } else {
                                Err("Modulo operator requires integer operands.".to_string())
                            }
                        }
                        BinOp::Idiv => {
                            if rc == 0.0 {
                                Err("Integer division by zero.".to_string())
                            } else if is_integer(lc) && is_integer(rc) {
                                Ok(Type::constant((lc as i64 / rc as i64) as f64))
                            } else {
                                Err("Integer division operator requires integer operands."
                                    .to_string())
                            }
                        }
                    }
                }
                (_, Constant(rc)) => {
                    // 检查除零和整数要求
                    if (op == &BinOp::Div || op == &BinOp::Mod || op == &BinOp::Idiv) && rc == 0.0 {
                        Err("Division or modulo by zero.".to_string())
                    } else if (op == &BinOp::Mod || op == &BinOp::Idiv) && !is_integer(rc) {
                        Err(
                            "Modulo or integer division operator requires integer operands."
                                .to_string(),
                        )
                    } else {
                        // 变量与常数之间的操作，结果为变量数值
                        Ok(Type::unknown_var())
                    }
                }
                _ => Ok(Type::unknown_var()), // 其他情况，结果为未知变量数值
            }
        }
        // 列表与常数标量之间的操作
        (List(l), Number(Constant(c))) | (Number(Constant(c)), List(l)) => {
            if !is_integer(c) || c < 0.0 {
                Err("List operations require non-negative integer constants.".to_string())
            } else if *op != BinOp::Mul {
                Err("Only multiplication is allowed between list and constant.".to_string())
            } else {
                match l {
                    ConstantList(lst) => {
                        // 列表与常数相乘，结果为常数列表
                        let mut new_list = Vec::new();
                        for _ in 0..(c as i64) {
                            new_list.extend(lst.iter());
                        }
                        Ok(Type::const_list(new_list))
                    }
                    VariableList => Ok(Type::var_list()),
                }
            }
        }
        // 列表与列表之间的操作
        (List(l), List(r)) => {
            // 只允许加法运算
            match op {
                BinOp::Add => {
                    match (l, r) {
                        (ConstantList(lc), ConstantList(rc)) => {
                            //两个常数列表相加，结果为常数列表
                            let mut new_list = lc.clone();
                            new_list.extend(rc.iter());
                            Ok(Type::const_list(new_list))
                        }
                        _ => Ok(Type::var_list()), // 其他情况，结果为变量列表
                    }
                }
                _ => Err("Only addition is allowed between lists.".to_string()),
            }
        }
        // 列表与变量之间执行特殊警告
        (List(_), Number(Variable(_))) | (Number(Variable(_)), List(_)) => {
            Err("Cannot perform operations between list and variable number.".to_string())
        }
    }
}

fn type_of_keep_drop_modifier(lhs: &Expr, count: &Expr) -> Result<Type, String> {
    use NumberType::*;
    use Type::*;
    use VariableNumber::*;
    let lhs_type = typecheck_expr(lhs)?;
    let count_type = typecheck_expr(count)?;
    match (lhs_type, count_type) {
        (Number(Variable(DicePool)), Number(Constant(c))) => {
            if !is_integer(c) || c <= 0.0 {
                return Err("Keep/Drop count must be a positive integer.".to_string());
            }
            Ok(Type::dice_pool())
        }
        (Number(Variable(DicePool)), _) => {
            Err("Keep/Drop count must be a constant number.".to_string())
        }
        _ => Err("Keep/Drop modifier can only be applied to dice expressions.".to_string()),
    }
}
fn type_of_reroll_modifier(lhs: &Expr, param: &CompareExpr) -> Result<Type, String> {
    let lhs_type = typecheck_expr(lhs)?;
    if !matches!(
        lhs_type,
        Type::Number(NumberType::Variable(VariableNumber::DicePool))
    ) {
        return Err("Reroll modifier can only be applied to dice expressions.".to_string());
    }
    let param_val_type = typecheck_expr(&param.val)?;
    if !matches!(param_val_type, Type::Number(NumberType::Constant(_))) {
        return Err(
            "Comparison parameter for reroll modifier must be a constant number.".to_string(),
        );
    }

    Ok(lhs_type)
}
fn type_of_explode_modifier(
    lhs: &Expr,
    compare_expr: &Option<CompareExpr>,
    limit: &Option<Box<Expr>>,
) -> Result<Type, String> {
    let lhs_type = typecheck_expr(lhs)?;
    if !matches!(
        lhs_type,
        Type::Number(NumberType::Variable(VariableNumber::DicePool))
    ) {
        return Err("Explode modifier can only be applied to dice expressions.".to_string());
    }
    if let Some(param) = compare_expr {
        let param_val_type = typecheck_expr(&param.val)?;
        if !matches!(param_val_type, Type::Number(NumberType::Constant(_))) {
            return Err(
                "Comparison parameter for explode modifier must be a constant number.".to_string(),
            );
        }
    }
    if let Some(limit_expr) = limit {
        let limit_type = typecheck_expr(limit_expr)?;
        match limit_type {
            Type::Number(NumberType::Constant(c)) => {
                if !is_integer(c) || c <= 0.0 {
                    return Err(
                        "Limit parameter for explode modifier must be a positive integer."
                            .to_string(),
                    );
                }
            }
            _ => {
                return Err(
                    "Limit parameter for explode modifier must be a constant number.".to_string(),
                );
            }
        }
    }
    Ok(lhs_type)
}
fn type_of_min_max_modifier(lhs: &Expr, target: &Expr) -> Result<Type, String> {
    use NumberType::*;
    use Type::*;
    use VariableNumber::*;
    let lhs_type = typecheck_expr(lhs)?;
    let target_type = typecheck_expr(target)?;
    match (lhs_type, target_type) {
        (Number(Variable(DicePool)), Number(Constant(c))) => {
            if c <= 0.0 || !is_integer(c) {
                return Err("Min/Max count must be a positive integer.".to_string());
            }
            Ok(Type::dice_pool())
        }
        (Number(Variable(DicePool)), _) => {
            Err("Min/Max count must be a constant number.".to_string())
        }
        _ => Err("Min/Max modifier can only be applied to dice expressions.".to_string()),
    }
}

fn type_of_success_check(lhs: &Expr, param: &CompareExpr) -> Result<Type, String> {
    use NumberType::*;
    use Type::*;
    use VariableNumber::*;
    let lhs_type = typecheck_expr(lhs)?;
    match lhs_type {
        Number(Variable(DicePool)) => {
            match typecheck_expr(&param.val)? {
                Number(Constant(_)) => Ok(Type::unknown_var()), // 成功检定的结果为未知值
                Number(Variable(_)) => Err(
                    "Comparison parameter for success check cannot be a variable number."
                        .to_string(),
                ),
                _ => Err(
                    "Comparison parameter for success check must be a numeric expression."
                        .to_string(),
                ),
            }
        }
        _ => Err("Success check can only be applied to dice expressions.".to_string()),
    }
}

#[derive(Clone)]
enum ArgsType {
    OneNumber(NumberType),
    OneList(ListType),
    OneListAndOneNumber(ListType, NumberType),
}
fn preprocess_call_args(args: &Vec<Type>) -> Result<ArgsType, String> {
    match args.as_slice() {
        [] => Err("Function requires at least one argument.".to_string()), // 空向量错误
        [Type::Number(nt)] => Ok(ArgsType::OneNumber(nt.clone())),         // 单数值参数
        [Type::List(lt)] => Ok(ArgsType::OneList(lt.clone())),             // 单列表参数
        [Type::List(lt), Type::Number(nt)] => {
            Ok(ArgsType::OneListAndOneNumber(lt.clone(), nt.clone()))
        } // 列表与数值参数
        // 其他情况，尝试将所有参数解释一起解释为列表，如果失败则视为错误
        _ => {
            let mut consts = Vec::new();
            let mut is_var_list = false;
            for arg_type in args {
                use NumberType::*;
                use Type::*;
                match arg_type {
                    List(_) => return Err("Nested lists are not allowed.".to_string()), // 不允许嵌套列表
                    Number(Variable(_)) => is_var_list = true, // 统计变量数值
                    Number(Constant(c)) => {
                        if !is_var_list {
                            consts.push(*c); // 收集常数数值
                        }
                    }
                }
            }
            if is_var_list {
                Ok(ArgsType::OneList(ListType::VariableList))
            } else {
                Ok(ArgsType::OneList(ListType::ConstantList(consts)))
            }
        }
    }
}
fn type_of_call(func_name: &str, args: &Vec<Expr>) -> Result<Type, String> {
    use ArgsType::*;
    use ListType::*;
    use NumberType::*;
    use VariableNumber::*;
    let raw_args_type: Vec<Type> = args
        .iter()
        .map(|arg| typecheck_expr(arg))
        .collect::<Result<_, _>>()?;
    let args_type = preprocess_call_args(&raw_args_type)?;
    match func_name {
        "max" | "min" => {
            match args_type {
                OneNumber(Constant(c)) => Ok(Type::constant(c)), // 单常数参数，结果为该常数
                OneNumber(Variable(_)) => Ok(Type::unknown_var()), // 单变量参数，结果为未知变量数值
                OneList(ConstantList(lst)) => {
                    if lst.is_empty() {
                        Err("max/min function requires at least one element.".to_string())
                    } else {
                        let extreme = if func_name == "max" {
                            lst.iter().cloned().fold(f64::MIN, f64::max)
                        } else {
                            lst.iter().cloned().fold(f64::MAX, f64::min)
                        };
                        Ok(Type::constant(extreme))
                    }
                }
                OneList(VariableList) => Ok(Type::unknown_var()), // 列表参数结果为未知变量数值
                // 从列表中取最大/小的 n 个元素
                OneListAndOneNumber(lst, Constant(c)) => {
                    if !is_integer(c) || c <= 0.0 {
                        return Err("The count parameter must be a positive integer.".to_string());
                    }
                    let n = c as usize;
                    match lst {
                        ConstantList(elements) => {
                            if elements.is_empty() {
                                return Err(
                                    "max/min function requires at least one element.".to_string()
                                );
                            }
                            let selected = top_n_preserve_order(&elements, n, func_name == "max");
                            Ok(Type::const_list(selected))
                        }
                        VariableList => Ok(Type::var_list()), // 列表参数结果为未知变量数值
                    }
                }
                OneListAndOneNumber(_, _) => {
                    Err("The count parameter must be a constant number.".to_string())
                }
            }
        }
        "sum" => {
            match args_type {
                OneNumber(Constant(c)) => Ok(Type::constant(c)), // 单常数参数，结果为该常数
                OneNumber(Variable(_)) => Ok(Type::unknown_var()), // 单变量参数，结果为未知变量数值
                OneList(ConstantList(lst)) => {
                    if lst.is_empty() {
                        return Err("sum function requires at least one element.".to_string());
                    }
                    let total: f64 = lst.iter().sum();
                    Ok(Type::constant(total))
                }
                OneList(VariableList) => Ok(Type::unknown_var()), // 列表参数结果为未知变量数值
                _ => {
                    Err("sum function requires numeric arguments or one list argument.".to_string())
                }
            }
        }
        "avg" => {
            match args_type {
                OneNumber(Constant(c)) => Ok(Type::constant(c)), // 单常数参数，结果为该常数
                OneNumber(Variable(_)) => Ok(Type::unknown_var()), // 单变量参数，结果为未知变量数值
                OneList(ConstantList(lst)) => {
                    if lst.is_empty() {
                        return Err("sum function requires at least one element.".to_string());
                    }
                    let avg = lst.iter().sum::<f64>() / (lst.len() as f64);
                    Ok(Type::constant(avg))
                }
                OneList(VariableList) => Ok(Type::unknown_var()), // 列表参数结果为未知变量数值
                _ => {
                    Err("avg function requires numeric arguments or one list argument.".to_string())
                }
            }
        }
        "len" => match raw_args_type.as_slice() {
            [Type::List(ConstantList(lst))] => {
                Ok(Type::constant(lst.len() as f64)) // 常数列表，结果为常数数值
            }
            [Type::List(VariableList)] => {
                Ok(Type::unknown_var()) // 变量列表，结果为未知变量数值
            }
            _ => Err("len function requires a single list argument.".to_string()),
        },
        "floor" | "ceil" | "round" | "abs" => {
            match args_type {
                OneNumber(Constant(c)) => {
                    let result = match func_name {
                        "floor" => c.floor(),
                        "ceil" => c.ceil(),
                        "round" => c.round(),
                        "abs" => c.abs(),
                        _ => unreachable!(),
                    };
                    Ok(Type::constant(result))
                }
                OneNumber(Variable(_)) => Ok(Type::unknown_var()), // 变量参数，结果为未知变量数值
                _ => Err(format!(
                    "{} function requires a single numeric argument.",
                    func_name
                )),
            }
        }
        "rpdice" => {
            match raw_args_type.as_slice() {
                [t] => Ok(t.clone()), // 其他情况的单参数调用，直接返回该参数的类型
                // 也可以接受第二个参数为常数数值，表示重复次数
                [t, Type::Number(Constant(c))] => {
                    if !is_integer(*c) || *c <= 0.0 {
                        Err(
                            "In rpdice, the repeat count parameter must be a postive integer."
                                .to_string(),
                        )
                    } else {
                        Ok(t.clone())
                    }
                }
                [_, Type::Number(Variable(_))] => Err(
                    "In rpdice, the repeat count parameter must be a constant integer.".to_string(),
                ),
                _ => Err(
                    "rpdice function requires one argument, or one argument and a repeat count."
                        .to_string(),
                ),
            }
        }
        "sort" | "sortd" => {
            match args_type {
                OneNumber(Constant(c)) => Ok(Type::const_list(vec![c])), // 单常数参数，结果为该常数的单元素列表
                OneNumber(Variable(_)) => Ok(Type::var_list()), // 单变量参数，结果为变量列表
                OneList(ConstantList(mut lst)) => {
                    if func_name == "sort" {
                        lst.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    } else {
                        lst.sort_by(|a, b| b.partial_cmp(a).unwrap());
                    }
                    Ok(Type::const_list(lst))
                }
                OneList(VariableList) => Ok(Type::var_list()), // 变量列表，结果为变量列表
                _ => Err(format!(
                    "{} function requires a single list argument.",
                    func_name
                )),
            }
        }
        "tolist" => {
            // 将骰池转换为列表
            match raw_args_type.as_slice() {
                [Type::Number(Variable(DicePool))] => Ok(Type::var_list()), // 单骰池参数，结果为变量列表
                _ => Err("tolist function requires a single dice expression argument.".to_string()),
            }
        }
        _ => Err(format!("Unknown function: {}", func_name)), // 未知函数，should be unreachable
    }
}
