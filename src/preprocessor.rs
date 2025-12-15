// use crate::fold_binary::fold_binary;
// use crate::grammar::CompareOp;
// use crate::grammar::Expr;
// use crate::grammar::ModifierOp;
// use crate::grammar::ModifierParam;
// use crate::typecheck::typecheck_expr;

// // ==========================================
// // 类型定义，首先定义IR相关类型
// // ==========================================

// pub type CompareIr = (CompareOp, f64); // 已经通过typecheck，不可能会出现比较非常数的情况
// pub type NewDiceItem = (i64, i64); // (数量，面数)，通过typecheck已经保证是常量整数

// #[derive(Debug, Clone, PartialEq)]
// pub enum Ir {
//     Constant(f64),         // 基础值，如数字字面量，常量折叠的终点
//     DicePool(NewDiceItem), // 骰池节点，包含骰子数量和面数等信息
//     List(Vec<Ir>),         // 列表节点

//     // 二元运算符
//     Add(Box<Ir>, Box<Ir>),
//     Sub(Box<Ir>, Box<Ir>),
//     Mul(Box<Ir>, Box<Ir>),
//     Div(Box<Ir>, Box<Ir>),
//     Idiv(Box<Ir>, Box<Ir>),
//     Mod(Box<Ir>, Box<Ir>),

//     // 具体化后的修饰符
//     KeepHigh(Box<Ir>, i64),
//     KeepLow(Box<Ir>, i64),
//     DropHigh(Box<Ir>, i64),
//     DropLow(Box<Ir>, i64),
//     // Option<i64>表示是否有上限，Limit修饰符会被吸收进ExplodeCompoundLimit
//     ExplodeCompoundLimit(Box<Ir>, CompareIr, Option<i64>),
//     Explode(Box<Ir>, CompareIr), // 只爆炸一次的修饰符
//     Reroll(Box<Ir>, CompareIr),
//     RerollOnce(Box<Ir>, CompareIr),

//     // 具体化的函数，RpDice消失了，应该在预处理后不复存在
//     // 输入必须是求值为 List 的 Ir
//     Sum(Box<Ir>),
//     Max(Box<Ir>),
//     Min(Box<Ir>),
//     // 单参数数学函数
//     Floor(Box<Ir>),
//     Ceil(Box<Ir>),
//     Round(Box<Ir>),
//     Abs(Box<Ir>),

//     // 成功判定
//     SuccessCheck(Box<Ir>, CompareIr),
// }

// // ==========================================
// // 主要函数
// // ==========================================

// // 预处理入口函数
// pub fn pre_process(expr: &Expr) -> Result<Ir, String> {
//     transform(expr)
// }

// // 将AST转换为IR
// fn transform(expr: &Expr) -> Result<Ir, String> {
//     use Expr::*;
//     match expr {
//         Number(c) => Ok(Ir::Constant(*c)),
//         Expr::Dice { count, side } => {
//             let c_ir = transform(count)?;
//             let s_ir = transform(side)?;
//             let c_val = expect_constant_integer(&c_ir)?;
//             let s_val = expect_constant_integer(&s_ir)?;
//             Ok(Ir::DicePool((c_val, s_val)))
//         }
//         Expr::List(items) => {
//             let ir_list = items
//                 .into_iter()
//                 .map(transform)
//                 .collect::<Result<Vec<_>, _>>()?;
//             Ok(Ir::List(ir_list))
//         }
//         Expr::Binary { lhs, op, rhs } => {
//             let l_ir = transform(lhs)?;
//             let r_ir = transform(rhs)?;
//             Ok(fold_binary(l_ir, op.clone(), r_ir)?)
//         }
//         Expr::Call { func_name, args } => {
//             if func_name == "rpdice" {
//                 expand_rpdice(args)
//             } else {
//                 map_function(func_name, args)
//             }
//         }
//         // --- 6. 修饰符 (合并 !! 和 l) ---
//         Expr::Modifier { lhs, op, param } => handle_modifier(lhs, op, param),

//         Expr::SuccessCheck { lhs, compare_expr } => {
//             let lhs_ir = transform(lhs)?;
//             // 提取比较条件。TypeCheck 保证 compare_expr.val 是常量
//             let val_ir = transform(&compare_expr.val)?;
//             let val = expect_constant(&val_ir)?;

//             Ok(Ir::SuccessCheck(
//                 Box::new(lhs_ir),
//                 (compare_expr.op.clone(), val),
//             ))
//         }
//     }
// }

// fn expand_rpdice(args: &Vec<Expr>) -> Result<Ir, String> {
//     fn repeat_dice(expr: Ir, count: i64) -> Result<Ir, String> {
//         match expr {
//             Ir::Constant(x) => Ok(Ir::Constant(x)),
//             Ir::DicePool((n, s)) => Ok(Ir::DicePool((n * count, s))),
//             Ir::List(items) => {
//                 let new_items = items
//                     .into_iter()
//                     .map(|item| repeat_dice(item, count))
//                     .collect::<Result<Vec<_>, _>>()?;
//                 Ok(Ir::List(new_items))
//             }
//             Ir::Abs(x) => Ok(Ir::Abs(Box::new(repeat_dice(*x, count)?))),
//             Ir::Floor(x) => Ok(Ir::Floor(Box::new(repeat_dice(*x, count)?))),
//             Ir::Ceil(x) => Ok(Ir::Ceil(Box::new(repeat_dice(*x, count)?))),
//             Ir::Round(x) => Ok(Ir::Round(Box::new(repeat_dice(*x, count)?))),
//             Ir::Sum(x) => Ok(Ir::Sum(Box::new(repeat_dice(*x, count)?))),
//             Ir::Max(x) => Ok(Ir::Max(Box::new(repeat_dice(*x, count)?))),
//             Ir::Min(x) => Ok(Ir::Min(Box::new(repeat_dice(*x, count)?))),
//             Ir::Add(x, y) => Ok(Ir::Add(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::Sub(x, y) => Ok(Ir::Sub(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::Mul(x, y) => Ok(Ir::Mul(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::Div(x, y) => Ok(Ir::Div(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::Idiv(x, y) => Ok(Ir::Idiv(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::Mod(x, y) => Ok(Ir::Mod(
//                 Box::new(repeat_dice(*x, count)?),
//                 Box::new(repeat_dice(*y, count)?),
//             )),
//             Ir::KeepLow(x, c) => Ok(Ir::KeepLow(Box::new(repeat_dice(*x, count)?), c)),
//             Ir::KeepHigh(x, c) => Ok(Ir::KeepHigh(Box::new(repeat_dice(*x, count)?), c)),
//             Ir::DropLow(x, c) => Ok(Ir::DropLow(Box::new(repeat_dice(*x, count)?), c)),
//             Ir::DropHigh(x, c) => Ok(Ir::DropHigh(Box::new(repeat_dice(*x, count)?), c)),
//             Ir::ExplodeCompoundLimit(x, cmp, lim) => Ok(Ir::ExplodeCompoundLimit(
//                 Box::new(repeat_dice(*x, count)?),
//                 cmp,
//                 lim,
//             )),
//             Ir::Explode(x, cmp) => Ok(Ir::Explode(Box::new(repeat_dice(*x, count)?), cmp)),
//             Ir::Reroll(x, cmp) => Ok(Ir::Reroll(Box::new(repeat_dice(*x, count)?), cmp)),
//             Ir::RerollOnce(x, cmp) => Ok(Ir::RerollOnce(Box::new(repeat_dice(*x, count)?), cmp)),
//             Ir::SuccessCheck(x, cmp) => {
//                 Ok(Ir::SuccessCheck(Box::new(repeat_dice(*x, count)?), cmp))
//             }
//         }
//     }

//     // rpdice(expr, count?)
//     let (expr, count) = match args.len() {
//         1 => (&args[0].clone(), 2 as i64), // 默认是2倍
//         2 => {
//             // TypeCheck 保证第二个参数是常数
//             let count_ir = transform(&args[1])?;
//             let c = expect_constant_integer(&count_ir)?;
//             (&args[0].clone(), c)
//         }
//         _ => return Err("rpdice expects 1 or 2 arguments".to_string()), // unreachable
//     };

//     let expr_ir = transform(expr)?;
//     repeat_dice(expr_ir, count)
// }

// fn map_function(func_name: &String, args: &Vec<Expr>) -> Result<Ir, String> {
//     unimplemented!("Function {} is not implemented, args {:?}", func_name, args)
// }

// fn get_dice_sides(expr: &Expr) -> Result<i64, String> {
//     use crate::typecheck::DicePoolType::*;
//     use crate::typecheck::NumberType::*;
//     use crate::typecheck::Type::*;
//     use crate::typecheck::VariableNumber::*;
//     match typecheck_expr(expr) {
//         Number(Variable(DicePool(RawDicePool(dice_item))))
//         | Number(Variable(DicePool(LimitableDicePool(dice_item)))) => Ok(dice_item.side),
//         _ => Err("Expected a dice pool expression".to_string()),
//     }
// }

// fn handle_modifier(
//     lhs: &Expr,
//     op: &ModifierOp,
//     param: &Option<ModifierParam>,
// ) -> Result<Ir, String> {
//     let lhs_ir = transform(lhs)?;

//     // 提取参数 (TypeCheck 保证参数类型匹配)
//     let get_int = || -> Result<i64, String> {
//         if let Some(ModifierParam::Value(expr)) = &param {
//             let ir = transform(expr)?;
//             expect_constant_integer(&ir)
//         } else {
//             Err("Expected integer parameter".to_string())
//         }
//     };

//     let get_compare = || -> Result<CompareIr, String> {
//         if let Some(ModifierParam::Compare(ce)) = &param {
//             let ir = transform(&ce.val)?;
//             let val = expect_constant(&ir)?;
//             Ok((ce.op.clone(), val))
//         } else if let Some(ModifierParam::Value(expr)) = &param {
//             // 兼容 r 1 -> r = 1
//             let ir = transform(expr)?;
//             let val = expect_constant(&ir)?;
//             Ok((CompareOp::Equal, val))
//         } else {
//             Err("Expected compare parameter".to_string())
//         }
//     };

//     match op {
//         ModifierOp::KeepHigh => Ok(Ir::KeepHigh(Box::new(lhs_ir), get_int()?)),
//         ModifierOp::KeepLow => Ok(Ir::KeepLow(Box::new(lhs_ir), get_int()?)),
//         ModifierOp::DropHigh => Ok(Ir::DropHigh(Box::new(lhs_ir), get_int()?)),
//         ModifierOp::DropLow => Ok(Ir::DropLow(Box::new(lhs_ir), get_int()?)),

//         ModifierOp::Reroll => Ok(Ir::Reroll(Box::new(lhs_ir), get_compare()?)),
//         ModifierOp::RerollOnce => Ok(Ir::RerollOnce(Box::new(lhs_ir), get_compare()?)),

//         ModifierOp::Explode => match param {
//             Some(ModifierParam::Compare(_)) => Ok(Ir::Explode(Box::new(lhs_ir), get_compare()?)),
//             None => {
//                 let sides = get_dice_sides(lhs)?;
//                 Ok(Ir::Explode(
//                     Box::new(lhs_ir),
//                     (CompareOp::Equal, sides as f64),
//                 ))
//             }
//             _ => Err("Expected compare parameter or none for Explode modifier".to_string()),
//         },
//         ModifierOp::ExplodeCompound => match param {
//             Some(ModifierParam::Compare(_)) => Ok(Ir::ExplodeCompoundLimit(
//                 Box::new(lhs_ir),
//                 get_compare()?,
//                 None,
//             )),
//             None => {
//                 let sides = get_dice_sides(lhs)?;
//                 Ok(Ir::ExplodeCompoundLimit(
//                     Box::new(lhs_ir),
//                     (CompareOp::Equal, sides as f64),
//                     None,
//                 ))
//             }
//             _ => Err("Expected compare parameter or none for ExplodeCompound modifier".to_string()),
//         },

//         ModifierOp::Limit => {
//             let limit_val = get_int()?;
//             // 关键：合并逻辑
//             // 检查 lhs_ir 是否是 ExplodeCompoundLimit
//             if let Ir::ExplodeCompoundLimit(inner_dice, compare, _) = lhs_ir {
//                 // 成功合并：更新 Limit
//                 Ok(Ir::ExplodeCompoundLimit(
//                     inner_dice,
//                     compare,
//                     Some(limit_val),
//                 ))
//             } else {
//                 Err("Limit modifier can only be applied to an ExplodeCompound modifier".to_string())
//             }
//         }
//     }
// }

// // ==========================================
// // 辅助处理函数
// // ==========================================

// pub fn expect_constant(ir: &Ir) -> Result<f64, String> {
//     match ir {
//         Ir::Constant(c) => Ok(*c),
//         _ => Err(format!("{:?} is not a constant", *ir)), // unreachable
//     }
// }

// pub fn expect_constant_integer(ir: &Ir) -> Result<i64, String> {
//     match ir {
//         Ir::Constant(c) if c.fract() == 0.0 => Ok(*c as i64),
//         Ir::Constant(_) => Err(format!("{:?} is not an integer constant", *ir)), // unreachable
//         _ => Err(format!("{:?} is not a constant", *ir)),                        // unreachable
//     }
// }
