use crate::types::expr::*;
use lazy_static::lazy_static;
use pest::Parser;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

// ==========================================
// 加载 Pest 语法定义
// ==========================================
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct DiceGrammar;

// ==========================================
// Pratt Parser 配置 (优先级控制)
// ==========================================

lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        PrattParser::new()
            // 优先级 1: 加减
            .op(Op::infix(Rule::add, Assoc::Left) | Op::infix(Rule::sub, Assoc::Left))
            // 优先级 2: 乘除模
            .op(Op::infix(Rule::mul, Assoc::Left) |
                Op::infix(Rule::div, Assoc::Left) |
                Op::infix(Rule::rem, Assoc::Left) |
                Op::infix(Rule::idiv, Assoc::Left))
            // 优先级 3: 前缀 (负号)
            .op(Op::prefix(Rule::neg) | Op::prefix(Rule::pos))
            // 优先级 4: 后缀 (修饰符) - 优先级最高，紧贴左侧
            .op(Op::postfix(Rule::modifier))
    };
}

// ==========================================
// 4. 解析函数声明
// ==========================================

pub fn parse_dice(input: &str) -> Result<Expr, pest::error::Error<Rule>> {
    // 语法解析
    let mut pairs = DiceGrammar::parse(Rule::main, input)?;
    let expr_pair = pairs.next().unwrap(); // expr
    // 转换为 AST
    Ok(parse_expr_pratt(expr_pair))
}

fn parse_expr_pratt(pair: pest::iterators::Pair<Rule>) -> Expr {
    PRATT_PARSER
        .map_primary(process_primary)
        .map_infix(process_infix)
        .map_prefix(process_prefix)
        .map_postfix(process_postfix)
        .parse(pair.into_inner())
}

// ==========================================
// 5. 辅助处理函数
// ==========================================

fn process_primary(pair: pest::iterators::Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::dice_expr => {
            // 进入里面一层
            let pairs = pair
                .into_inner()
                .map(|p| (p.as_rule(), p))
                .collect::<Vec<_>>();
            let (left_expr, (rhs_rule, rhs_pair)) = match pairs.as_slice() {
                // 只有一个atom，直接返回
                [(Rule::atom, atom_pair)] => return parse_atom(atom_pair.clone()),
                // 正常的骰子表达式
                [
                    (Rule::atom, left_pair),
                    (Rule::dice_op, _),
                    (rhs_rule, rhs_pair),
                ] => (parse_atom(left_pair.clone()), (rhs_rule, rhs_pair)),
                // 省略了前面的数量，默认为1
                [(Rule::dice_op, _), (rhs_rule, rhs_pair)] => {
                    (Expr::number(1.0), (rhs_rule, rhs_pair))
                }
                _ => unreachable!("Unknown primary expression: {:?}", pairs),
            };
            match rhs_rule {
                Rule::atom => {
                    let right_expr = parse_atom(rhs_pair.clone());
                    Expr::normal_dice(left_expr, right_expr)
                }
                Rule::fate_dice => Expr::fudge_dice(left_expr),
                Rule::coin_dice => Expr::coin_dice(left_expr),
                _ => unreachable!("Unknown primary expression: {:?}", pairs),
            }
        }
        _ => unreachable!("Unknown primary expression: {:?}", pair.as_rule()),
    }
}

fn process_infix(lhs: Expr, op: pest::iterators::Pair<Rule>, rhs: Expr) -> Expr {
    let bin_op = match op.as_rule() {
        Rule::add => BinOp::Add,
        Rule::sub => BinOp::Sub,
        Rule::mul => BinOp::Mul,
        Rule::div => BinOp::Div,
        Rule::rem => BinOp::Mod,
        Rule::idiv => BinOp::Idiv,
        _ => unreachable!("Unknown infix operator: {:?}", op.as_rule()),
    };
    Expr::binary(lhs, bin_op, rhs)
}

fn process_prefix(op: pest::iterators::Pair<Rule>, rhs: Expr) -> Expr {
    match op.as_rule() {
        Rule::pos => rhs, // 正号不做处理
        Rule::neg => Expr::neg(rhs),
        _ => unreachable!("Unknown prefix operator: {:?}", op.as_rule()),
    }
}

fn process_postfix(lhs: Expr, op: pest::iterators::Pair<Rule>) -> Expr {
    let op = op.into_inner().next().unwrap(); // 取得第一个操作符
    match op.as_rule() {
        Rule::keep_high
        | Rule::keep_low
        | Rule::drop_high
        | Rule::drop_low
        | Rule::min
        | Rule::max => {
            let op_enum = match op.as_rule() {
                Rule::keep_high => Type1Op::KeepHigh,
                Rule::keep_low => Type1Op::KeepLow,
                Rule::drop_high => Type1Op::DropHigh,
                Rule::drop_low => Type1Op::DropLow,
                Rule::min => Type1Op::Min,
                Rule::max => Type1Op::Max,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_type1_modifier(lhs, op_enum, inner_pairs)
        }
        Rule::explode | Rule::compound_explode | Rule::reroll => {
            let op_enum = match op.as_rule() {
                Rule::explode => Type2Op::Explode,
                Rule::compound_explode => Type2Op::CompoundExplode,
                Rule::reroll => Type2Op::Reroll,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_type2_modifier(lhs, op_enum, inner_pairs)
        }
        Rule::count_successes | Rule::deduct_failures | Rule::subtract_failures => {
            let op_enum = match op.as_rule() {
                Rule::count_successes => Type3Op::CountSuccesses,
                Rule::deduct_failures => Type3Op::DeductFailures,
                Rule::subtract_failures => Type3Op::SubtractFailures,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_type3_modifier(lhs, op_enum, inner_pairs)
        }
        _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
    }
}

fn process_type1_modifier(lhs: Expr, op: Type1Op, mut inner: pest::iterators::Pairs<Rule>) -> Expr {
    let param = match inner.next() {
        Some(param_pair) => parse_atom(param_pair),
        None => Expr::number(1.0),
    };
    Expr::modifier_type1(lhs, op, param)
}

fn process_mod_parameter(mut inner: pest::iterators::Pairs<Rule>) -> ModParam {
    fn string_to_compare_op(s: &str) -> CompareOp {
        match s {
            ">" => CompareOp::Greater,
            "<" => CompareOp::Less,
            "=" => CompareOp::Equal,
            ">=" => CompareOp::GreaterEqual,
            "<=" => CompareOp::LessEqual,
            "<>" => CompareOp::NotEqual,
            _ => unreachable!("Unknown compare operator: {}", s),
        }
    }
    let first = inner.next().unwrap();
    match first.as_rule() {
        Rule::atom => {
            let value = parse_atom(first);
            Expr::mod_param(CompareOp::Equal, value)
        }
        Rule::compare_op => {
            let op_symbol = string_to_compare_op(first.as_str());
            let val_pair = inner.next().unwrap();
            Expr::mod_param(op_symbol, parse_atom(val_pair))
        }
        _ => unreachable!("Unknown modifier parameter: {:?}", first.as_rule()),
    }
}

fn process_limit(mut inner: pest::iterators::Pairs<Rule>) -> Limit {
    let mut limit_times = None;
    let mut limit_counts = None;
    for pair in inner.by_ref() {
        match pair.as_rule() {
            Rule::limit_times => {
                let expr_pair = pair.into_inner().next().unwrap();
                limit_times = Some(Box::new(parse_atom(expr_pair)));
            }
            Rule::limit_count => {
                let expr_pair = pair.into_inner().next().unwrap();
                limit_counts = Some(Box::new(parse_atom(expr_pair)));
            }
            _ => unreachable!("Unknown limit component: {:?}", pair.as_rule()),
        }
    }
    Limit {
        limit_times,
        limit_counts,
    }
}

fn process_type2_modifier(lhs: Expr, op: Type2Op, mut inner: pest::iterators::Pairs<Rule>) -> Expr {
    let mut mod_param = None;
    let mut limit = None;
    for pair in inner.by_ref() {
        match pair.as_rule() {
            Rule::mod_param => {
                let mod_param_inner = pair.into_inner(); // mod_param内部
                mod_param = Some(process_mod_parameter(mod_param_inner));
            }
            Rule::limit => {
                let limit_inner = pair.into_inner(); // limit内部
                limit = Some(process_limit(limit_inner));
            }
            _ => unreachable!("Unknown Type2 modifier component: {:?}", pair.as_rule()),
        }
    }
    Expr::modifier_type2(lhs, op, mod_param, limit)
}

fn process_type3_modifier(lhs: Expr, op: Type3Op, mut inner: pest::iterators::Pairs<Rule>) -> Expr {
    match inner.next() {
        Some(mod_param) => {
            let mod_param_inner = mod_param.into_inner(); // mod_param内部
            let compare_expr = process_mod_parameter(mod_param_inner);
            Expr::modifier_type3(lhs, op, compare_expr)
        }
        None => {
            unreachable!("Type3 modifier requires a parameter");
        }
    }
}

fn parse_atom(pair: pest::iterators::Pair<Rule>) -> Expr {
    let inner_pairs = pair.into_inner().next().unwrap();
    match inner_pairs.as_rule() {
        Rule::number => {
            let s = inner_pairs.as_str();
            Expr::Number(s.parse::<f64>().unwrap_or(0.0))
        }
        Rule::function => {
            let mut inner = inner_pairs.into_inner();
            let name = inner.next().unwrap();
            let function_name = match name.clone().into_inner().next() {
                // 常规函数，如 floor, ceil 等
                None => match name.as_str().to_lowercase().as_str() {
                    "floor" => FunctionName::Floor,
                    "ceil" => FunctionName::Ceil,
                    "round" => FunctionName::Round,
                    "abs" => FunctionName::Abs,
                    "max" => FunctionName::Max,
                    "min" => FunctionName::Min,
                    "sum" => FunctionName::Sum,
                    "avg" => FunctionName::Avg,
                    "len" => FunctionName::Len,
                    "rpdice" => FunctionName::Rpdice,
                    "sortd" => FunctionName::Sortd,
                    "sort" => FunctionName::Sort,
                    "tolist" => FunctionName::ToList,
                    _ => unreachable!("Unknown function name: {}", name.as_str()),
                },
                // 过滤函数 filter(param)
                Some(tmp) => {
                    let mod_param =
                        process_mod_parameter(tmp.into_inner().next().unwrap().into_inner());
                    FunctionName::Filter(mod_param)
                }
            };
            let args = match inner.next() {
                Some(args_pair) => args_pair
                    .into_inner()
                    .map(|p| parse_expr_pratt(p))
                    .collect(),
                None => vec![],
            };
            Expr::function(function_name, args)
        }
        Rule::list => {
            let mut inner = inner_pairs.into_inner();
            let items = match inner.next() {
                Some(args_pair) => args_pair
                    .into_inner()
                    .map(|p| parse_expr_pratt(p))
                    .collect(),
                None => vec![],
            };
            Expr::List(items)
        }
        // 处理括号 (expr)
        Rule::expr => parse_expr_pratt(inner_pairs),

        // 容错处理
        _ => unreachable!(
            "Unknown atom: {:?} - {}",
            inner_pairs.as_rule(),
            inner_pairs.as_str()
        ),
    }
}
