use crate::types::expr::{
    BinOp, BinaryOp, DiceType, Expr, FunctionCall, FunctionName, ModifierNode, Type1Modifier,
    Type1Op, Type2Modifier, Type2Op, Type3Modifier, Type3Op,
};
use crate::types::hir::{DicePoolType, HIR, ListType, NumberType};
use crate::types::hir_rewriter::HirVisitor;

// ==========================================
// 从 AST 降低到 HIR
// ==========================================

pub fn lower_expr(expr: Expr) -> Result<HIR, String> {
    match expr {
        Expr::Number(n) => Ok(HIR::constant(n)),
        Expr::Neg(expr) => lower_neg(*expr),
        Expr::Dice(dice_item) => lower_dice(dice_item),
        Expr::List(elements) => lower_list(elements),
        Expr::Binary(BinaryOp { lhs, op, rhs }) => lower_binary(*lhs, op, *rhs),
        Expr::Function(FunctionCall { name, args }) => lower_function_call(name, args),
        Expr::Modifier(ModifierNode::Type1(Type1Modifier { lhs, op, param })) => {
            lower_modifier_type1(*lhs, op, *param)
        }
        Expr::Modifier(ModifierNode::Type2(Type2Modifier {
            lhs,
            op,
            param,
            limit,
        })) => lower_modifier_type2(*lhs, op, param, limit),
        Expr::Modifier(ModifierNode::Type3(Type3Modifier { lhs, op, param })) => {
            lower_modifier_type3(*lhs, op, param)
        }
    }
}

// ==========================================
// 复杂逻辑拆出来作为单独的函数
// ==========================================

fn lower_neg(expr: Expr) -> Result<HIR, String> {
    let lowered = lower_expr(expr)?;
    let num = lowered
        .except_number()
        .map_err(|_| "Negation can only be applied to numbers".to_string())?;
    Ok(HIR::negate(num))
}

fn lower_dice(dice_item: DiceType) -> Result<HIR, String> {
    match dice_item {
        DiceType::Standard { count, sides } => {
            let lowered_count = lower_expr(*count)?;
            let lowered_sides = lower_expr(*sides)?;
            let count_num = lowered_count
                .except_number()
                .map_err(|_| "Dice count must be a number".to_string())?;
            let sides_num = lowered_sides
                .except_number()
                .map_err(|_| "Dice sides must be a number".to_string())?;
            Ok(HIR::standard_dice_pool(count_num, sides_num))
        }
        DiceType::Fudge { count } => {
            let lowered_count = lower_expr(*count)?;
            let count_num = lowered_count
                .except_number()
                .map_err(|_| "Fudge dice count must be a number".to_string())?;
            Ok(HIR::fudge_dice_pool(count_num))
        }
        DiceType::Coin { count } => {
            let lowered_count = lower_expr(*count)?;
            let count_num = lowered_count
                .except_number()
                .map_err(|_| "Coin dice count must be a number".to_string())?;
            Ok(HIR::coin_dice_pool(count_num))
        }
    }
}

fn lower_list(elements: Vec<Expr>) -> Result<HIR, String> {
    let number_elements = elements
        .into_iter()
        .map(|e| {
            let number_type = lower_expr(e)?.except_number().map_err(|_| {
                "List elements must be numbers. Nested list is not allowed.".to_string()
            })?;
            Ok(number_type)
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(HIR::explicit_list(number_elements))
}

fn lower_binary(lhs: Expr, op: BinOp, rhs: Expr) -> Result<HIR, String> {
    let lhs_hir = lower_expr(lhs)?;
    let rhs_hir = lower_expr(rhs)?;
    match (lhs_hir, op, rhs_hir) {
        // 数与数之间的二元操作
        (HIR::Number(lhs_num), BinOp::Add, HIR::Number(rhs_num)) => {
            Ok(HIR::add_number(lhs_num, rhs_num))
        }
        (HIR::Number(lhs_num), BinOp::Sub, HIR::Number(rhs_num)) => {
            Ok(HIR::sub_number(lhs_num, rhs_num))
        }
        (HIR::Number(lhs_num), BinOp::Mul, HIR::Number(rhs_num)) => {
            Ok(HIR::multiply_number(lhs_num, rhs_num))
        }
        (HIR::Number(lhs_num), BinOp::Div, HIR::Number(rhs_num)) => {
            Ok(HIR::divide_number(lhs_num, rhs_num))
        }
        (HIR::Number(lhs_num), BinOp::Mod, HIR::Number(rhs_num)) => {
            Ok(HIR::modulo_number(lhs_num, rhs_num))
        }
        (HIR::Number(lhs_num), BinOp::Idiv, HIR::Number(rhs_num)) => {
            Ok(HIR::int_divide_number(lhs_num, rhs_num))
        }
        // 列表特殊操作，列表相加，列表重复
        (HIR::List(lhs_list), BinOp::Add, HIR::List(rhs_list)) => {
            Ok(HIR::add_list(lhs_list, rhs_list))
        }
        (HIR::List(list), BinOp::ListMul, HIR::Number(times))
        | (HIR::Number(times), BinOp::ListMul, HIR::List(list)) => {
            use crate::optimizer::constant_fold::constant_fold_hir;
            // 特殊处理直接原地展开
            let list = constant_fold_hir(HIR::List(list))?
                .except_list()
                .map_err(|_| "unreachable")?;
            let times = constant_fold_hir(HIR::Number(times))?
                .except_number()
                .map_err(|_| "unreachable")?;
            if !list.is_explicit() || !times.is_constant() {
                return Err(
                    "List multiplication can only be applied to explicit list and constant number"
                        .to_string(),
                );
            }
            let times_val = match times {
                NumberType::Constant(val) if (val as i32) > 0 => val as usize,
                NumberType::Constant(_) => {
                    return Err("List multiplication times must be positive".to_string());
                }
                _ => unreachable!(),
            };
            let list_val = match list {
                ListType::Explicit(vals) => vals,
                _ => unreachable!(),
            };

            let mut combined = Vec::with_capacity(list_val.len() * times_val);
            for _ in 0..times_val {
                combined.extend(list_val.iter().cloned()); // 这里的Clone无法避免
            }

            Ok(HIR::explicit_list(combined))
        }
        // 列表与数之间的二元操作（无顺序要求，广播操作）
        (HIR::List(list), BinOp::Add, HIR::Number(num))
        | (HIR::Number(num), BinOp::Add, HIR::List(list)) => Ok(HIR::add_broadcast_list(list, num)),
        (HIR::List(list), BinOp::Mul, HIR::Number(num))
        | (HIR::Number(num), BinOp::Mul, HIR::List(list)) => {
            Ok(HIR::multiply_broadcast_list(list, num))
        }
        // 列表与数之间的二元操作（有顺序要求）
        // 减法
        (HIR::List(list), BinOp::Sub, HIR::Number(num)) => Ok(HIR::sub_broadcast_list(list, num)),
        (HIR::Number(num), BinOp::Sub, HIR::List(list)) => {
            Ok(HIR::sub_reverse_broadcast_list(num, list))
        }
        // 除法
        (HIR::List(list), BinOp::Div, HIR::Number(num)) => Ok(HIR::div_broadcast_list(list, num)),
        (HIR::Number(num), BinOp::Div, HIR::List(list)) => {
            Ok(HIR::div_reverse_broadcast_list(num, list))
        }
        // 整除
        (HIR::List(list), BinOp::Idiv, HIR::Number(num)) => Ok(HIR::idiv_broadcast_list(list, num)),
        (HIR::Number(num), BinOp::Idiv, HIR::List(list)) => {
            Ok(HIR::idiv_reverse_broadcast_list(num, list))
        }
        // 取模
        (HIR::List(list), BinOp::Mod, HIR::Number(num)) => {
            Ok(HIR::modulo_broadcast_list(list, num))
        }
        (HIR::Number(num), BinOp::Mod, HIR::List(list)) => {
            Ok(HIR::modulo_reverse_broadcast_list(num, list))
        }
        // 不支持的操作
        (_, BinOp::ListMul, _) => {
            Err("List multiplication is only supported between a list and a number".to_string())
        }
        (HIR::List(_), _, HIR::List(_)) => {
            Err("Only addition is supported between two lists".to_string())
        }
    }
}

fn lower_function_call(function_name: FunctionName, args: Vec<Expr>) -> Result<HIR, String> {
    use FunctionName::*;
    // 首先处理参数，如果有错，就不用继续处理了
    let args_hir = args
        .into_iter()
        .map(lower_expr)
        .collect::<Result<Vec<_>, String>>()?;
    match function_name {
        Floor => {
            if is_exactly_one_number(&args_hir) {
                let num = exactly_one_number(args_hir);
                Ok(HIR::floor_number(num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::floor_list(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::floor_list(list))
            }
        }
        Ceil => {
            if is_exactly_one_number(&args_hir) {
                let num = exactly_one_number(args_hir);
                Ok(HIR::ceil_number(num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::ceil_list(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::ceil_list(list))
            }
        }
        Round => {
            if is_exactly_one_number(&args_hir) {
                let num = exactly_one_number(args_hir);
                Ok(HIR::round_number(num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::round_list(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::round_list(list))
            }
        }
        Abs => {
            if is_exactly_one_number(&args_hir) {
                let num = exactly_one_number(args_hir);
                Ok(HIR::abs_number(num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::abs_list(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::abs_list(list))
            }
        }
        Max => {
            if is_exactly_one_list_and_one_number(&args_hir) {
                let (list, num) = exactly_one_list_and_one_number(args_hir);
                Ok(HIR::max_list(list, num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::max_number(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::max_number(list))
            }
        }
        Min => {
            if is_exactly_one_list_and_one_number(&args_hir) {
                let (list, num) = exactly_one_list_and_one_number(args_hir);
                Ok(HIR::min_list(list, num))
            } else if is_exactly_one_list(&args_hir) {
                let list = exactly_one_list(args_hir);
                Ok(HIR::min_number(list))
            } else {
                let list = treat_as_list(args_hir)?;
                Ok(HIR::min_number(list))
            }
        }
        Sum => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            Ok(HIR::sum(list))
        }
        Avg => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            Ok(HIR::avg(list))
        }
        Len => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            Ok(HIR::len(list))
        }
        Sort => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            Ok(HIR::sort_list(list))
        }
        Sortd => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            Ok(HIR::sort_desc_list(list))
        }
        ToList => {
            if args_hir.len() != 1 {
                return Err("tolist function requires exactly one argument".to_string());
            }
            let pool = args_hir.into_iter().next().unwrap();
            match pool {
                HIR::Number(NumberType::DicePool(dice_pool)) => {
                    Ok(HIR::tolist_from_dice_pool(dice_pool))
                }
                HIR::Number(NumberType::SuccessPool(success_pool)) => {
                    Ok(HIR::tolist_from_success_pool(success_pool))
                }
                _ => Err(
                    "tolist function requires a dice pool or success pool as argument".to_string(),
                ),
            }
        }
        Filter(compare_expr) => {
            let list = if is_exactly_one_list(&args_hir) {
                exactly_one_list(args_hir)
            } else {
                treat_as_list(args_hir)?
            };
            let compare_param = expr_mp_to_hir_mp(compare_expr)?;
            Ok(HIR::filter_list(list, compare_param))
        }
        // Rpdice函数需要特殊处理
        Rpdice => {
            if args_hir.len() != 1 {
                return Err("rpdice function requires exactly one argument".to_string());
            }
            let orginal_hir = args_hir.into_iter().next().unwrap();
            rpdice(orginal_hir)
        }
    }
}

fn lower_modifier_type1(lhs: Expr, op: Type1Op, param: Expr) -> Result<HIR, String> {
    let lowered_lhs = lower_expr(lhs)?
        .except_dice_pool()
        .map_err(|_| "Type1 modifier can only be applied to a dice pool".to_string())?;
    let param = lower_expr(param)?
        .except_number()
        .map_err(|_| "Type1 modifier parameter must be a number".to_string())?;
    match op {
        Type1Op::DropHigh => Ok(HIR::drop_high(lowered_lhs, param)),
        Type1Op::DropLow => Ok(HIR::drop_low(lowered_lhs, param)),
        Type1Op::KeepHigh => Ok(HIR::keep_high(lowered_lhs, param)),
        Type1Op::KeepLow => Ok(HIR::keep_low(lowered_lhs, param)),
        Type1Op::Max => Ok(HIR::max_dice_pool(lowered_lhs, param)),
        Type1Op::Min => Ok(HIR::min_dice_pool(lowered_lhs, param)),
    }
}

fn lower_modifier_type2(
    lhs: Expr,
    op: Type2Op,
    param: Option<crate::types::expr::ModParam>,
    limit: Option<crate::types::expr::Limit>,
) -> Result<HIR, String> {
    let lowered_lhs = lower_expr(lhs)?
        .except_dice_pool()
        .map_err(|_| "Type2 modifier can only be applied to a dice pool".to_string())?;
    let compare_param = param.map(|mp| expr_mp_to_hir_mp(mp)).transpose()?;
    let limit = limit.map(|lim| expr_limit_to_hir_limit(lim)).transpose()?;
    match op {
        Type2Op::Reroll => {
            if let Some(cp) = compare_param {
                Ok(HIR::reroll(lowered_lhs, cp, limit))
            } else {
                Err("Reroll modifier requires a compare parameter".to_string()) // unreachable
            }
        }
        Type2Op::Explode => Ok(HIR::explode(lowered_lhs, compare_param, limit)),
        Type2Op::CompoundExplode => Ok(HIR::compound_explode(lowered_lhs, compare_param, limit)),
    }
}

fn lower_modifier_type3(
    lhs: Expr,
    op: Type3Op,
    param: crate::types::expr::ModParam,
) -> Result<HIR, String> {
    let lowered_lhs = lower_expr(lhs)?;
    let compare_param = expr_mp_to_hir_mp(param)?;
    match op {
        Type3Op::SubtractFailures => {
            let lowered_lhs = lowered_lhs.except_dice_pool().map_err(|_| {
                "SubtractFailures modifier can only be applied to a dice pool".to_string()
            })?;
            Ok(HIR::subtract_failures(lowered_lhs, compare_param))
        }
        Type3Op::CountSuccesses => {
            if lowered_lhs.is_dice_pool() {
                let lowered_lhs = lowered_lhs.except_dice_pool().unwrap(); // safe unwrap
                Ok(HIR::count_successes_from_dice_pool(
                    lowered_lhs,
                    compare_param,
                ))
            } else if lowered_lhs.is_success_pool() {
                let lowered_lhs = lowered_lhs.except_success_pool().unwrap(); // safe unwrap
                Ok(HIR::count_successes(lowered_lhs, compare_param))
            } else {
                Err(
                    "CountSuccesses modifier can only be applied to a dice pool or success pool"
                        .to_string(),
                )
            }
        }
        Type3Op::DeductFailures => {
            if lowered_lhs.is_dice_pool() {
                let lowered_lhs = lowered_lhs.except_dice_pool().unwrap(); // safe unwrap
                Ok(HIR::deduct_failures_from_dice_pool(
                    lowered_lhs,
                    compare_param,
                ))
            } else if lowered_lhs.is_success_pool() {
                let lowered_lhs = lowered_lhs.except_success_pool().unwrap(); // safe unwrap
                Ok(HIR::deduct_failures(lowered_lhs, compare_param))
            } else {
                Err(
                    "DeductFailures modifier can only be applied to a dice pool or success pool"
                        .to_string(),
                ) // unreachable
            }
        }
    }
}

// ==========================================
// Vec<HIR> 特殊逻辑
// ==========================================

fn is_exactly_one_number(args: &Vec<HIR>) -> bool {
    args.len() == 1 && args[0].is_number()
}

fn exactly_one_number(args: Vec<HIR>) -> NumberType {
    args.into_iter().next().unwrap().except_number().unwrap()
}

fn is_exactly_one_list(args: &Vec<HIR>) -> bool {
    args.len() == 1 && args[0].is_list()
}

fn exactly_one_list(args: Vec<HIR>) -> ListType {
    args.into_iter().next().unwrap().except_list().unwrap()
}

fn is_exactly_one_list_and_one_number(args: &Vec<HIR>) -> bool {
    args.len() == 2 && (args[0].is_list() && args[1].is_number())
}

fn exactly_one_list_and_one_number(args: Vec<HIR>) -> (ListType, NumberType) {
    let mut iter = args.into_iter();
    let list = iter.next().unwrap().except_list().unwrap();
    let number = iter.next().unwrap().except_number().unwrap();
    (list, number)
}

fn treat_as_list(args: Vec<HIR>) -> Result<ListType, String> {
    // 尝试将所有参数都解释为数字，然后组成一个显式列表
    args.into_iter()
        .map(|hir| {
            hir.except_number()
                .map_err(|_| "All arguments must be numbers to form an explicit list".to_string())
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|numbers| ListType::Explicit(numbers))
}

// ==========================================
// RpDice 专用函数
// ==========================================

fn rpdice(orginal_hir: HIR) -> Result<HIR, String> {
    fn double_count(count: &mut NumberType) {
        let old_count_val = std::mem::replace(&mut *count, NumberType::Constant(0.0));
        let new_count_val =
            HIR::multiply_number(HIR::constant(2.0).except_number().unwrap(), old_count_val)
                .except_number()
                .unwrap();
        *count = new_count_val;
    }

    struct RpDiceRewriter;
    impl HirVisitor for RpDiceRewriter {
        fn visit_dice_pool_self(&mut self, d: &mut DicePoolType) -> Result<(), String> {
            use DicePoolType::*;
            match d {
                Standard(count, _) | Fudge(count) | Coin(count) => {
                    double_count(count);
                }
                _ => {}
            }
            Ok(())
        }
    }

    let mut hir_copy = orginal_hir;
    let mut rewriter = RpDiceRewriter;
    rewriter.visit_hir(&mut hir_copy)?;
    Ok(hir_copy)
}

// ==========================================
// 辅助函数
// ==========================================

fn expr_mp_to_hir_mp(
    mod_param: crate::types::expr::ModParam,
) -> Result<crate::types::hir::ModParam, String> {
    let op = mod_param.operator;
    let expr = lower_expr(*mod_param.value)?
        .except_number()
        .map_err(|_| "Only number can be compared".to_string())?;
    Ok(HIR::compare_param(op, expr))
}

fn expr_limit_to_hir_limit(
    limit: crate::types::expr::Limit,
) -> Result<crate::types::hir::Limit, String> {
    let limit_times = if let Some(limit_times) = limit.limit_times {
        let expr = lower_expr(*limit_times)?
            .except_number()
            .map_err(|_| "Limit times must be a number".to_string())?;
        Some(expr)
    } else {
        None
    };

    let limit_counts = if let Some(limit_counts) = limit.limit_counts {
        let expr = lower_expr(*limit_counts)?
            .except_number()
            .map_err(|_| "Limit counts must be a number".to_string())?;
        Some(expr)
    } else {
        None
    };

    Ok(HIR::limit_param(limit_times, limit_counts))
}
