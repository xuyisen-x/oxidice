use lazy_static::lazy_static;
use pest::Parser;
use pest::pratt_parser::{Assoc, Op, PrattParser};
use pest_derive::Parser;

// 加载语法文件
#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct DiceGrammar;

// ==========================================
// AST 数据结构
// ==========================================

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Idiv,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompareOp {
    Greater,
    Less,
    Equal,
    GreaterEqual,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq)]
pub enum KeepOrDropModifierOp {
    KeepHigh,
    KeepLow,
    DropHigh,
    DropLow,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RerollModifierOp {
    Reroll,
    RerollOnce,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExplodeModifierOp {
    Explode,
    CompoundExplode,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MinMaxModifierOp {
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompareExpr {
    pub op: CompareOp,
    pub val: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Dice {
        count: Box<Expr>,
        side: Box<Expr>,
    },
    Binary {
        lhs: Box<Expr>,
        op: BinOp,
        rhs: Box<Expr>,
    },
    Call {
        func_name: String,
        args: Vec<Expr>,
    },
    List(Vec<Expr>),
    KeepOrDropModifier {
        lhs: Box<Expr>,
        op: KeepOrDropModifierOp,
        count: Box<Expr>,
    },
    RerollModifier {
        lhs: Box<Expr>,
        op: RerollModifierOp,
        compare_expr: CompareExpr, // compare_expr: 必须显式指定
    },
    ExplodeModifier {
        lhs: Box<Expr>,
        op: ExplodeModifierOp,
        compare_expr: Option<CompareExpr>, // compare_expr: 可选指定，如果缺省，需要在预处理阶段确定
        limit: Option<Box<Expr>>,          // limit: 可选指定爆炸上限（最多爆炸多少次）
    },
    SuccessCheck {
        lhs: Box<Expr>,
        compare_expr: CompareExpr, // compare_expr: 必须显式指定
    },
    MinMaxModifier {
        lhs: Box<Expr>,
        op: MinMaxModifierOp,
        target: Box<Expr>,
    },
}

fn string_to_compare_op(s: &str) -> CompareOp {
    match s {
        ">" => CompareOp::Greater,
        "<" => CompareOp::Less,
        "=" => CompareOp::Equal,
        ">=" => CompareOp::GreaterEqual,
        "<=" => CompareOp::LessEqual,
        _ => unreachable!("Unknown compare operator: {}", s),
    }
}

// ==========================================
// 3. Pratt Parser 配置 (优先级控制)
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
            let mut inner_pairs = pair.into_inner();
            let first = inner_pairs.next().unwrap();
            match first.as_rule() {
                Rule::dice_op => {
                    // 以dice_op开头，省略了数量，则默认为1
                    let atom_pair = inner_pairs.next().unwrap();
                    let sides = parse_atom(atom_pair);
                    Expr::Dice {
                        count: Box::new(Expr::Number(1.0)),
                        side: Box::new(sides),
                    }
                }
                Rule::atom => {
                    // 以atom开头，说明有数量，可能是单纯的数值或者ndn的表达式
                    let count_or_number = parse_atom(first);
                    match inner_pairs.next() {
                        Some(_) => {
                            // 后面跟着dice_op，说明是ndn表达式
                            let sides_pair = inner_pairs.next().unwrap();
                            let sides = parse_atom(sides_pair);
                            Expr::Dice {
                                count: Box::new(count_or_number),
                                side: Box::new(sides),
                            }
                        }
                        None => {
                            // 只有一个atom，直接返回
                            count_or_number
                        }
                    }
                }
                _ => unreachable!("Unknown dice expression: {:?}", first.as_rule()),
            }
        }
        // 下面这句话永远不会被reach到，因为expr已经被Pratt Parser处理过了
        // Rule::expr => parse_expr_pratt(pair),
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
    Expr::Binary {
        lhs: Box::new(lhs),
        op: bin_op,
        rhs: Box::new(rhs),
    }
}

fn process_prefix(op: pest::iterators::Pair<Rule>, rhs: Expr) -> Expr {
    match op.as_rule() {
        Rule::neg => Expr::Binary {
            lhs: Box::new(Expr::Number(0.0)),
            op: BinOp::Sub,
            rhs: Box::new(rhs),
        },
        Rule::pos => rhs, // 正号不做处理
        _ => unreachable!("Unknown prefix operator: {:?}", op.as_rule()),
    }
}

fn process_postfix(lhs: Expr, op: pest::iterators::Pair<Rule>) -> Expr {
    let op = op.into_inner().next().unwrap(); // 取得第一个操作符
    match op.as_rule() {
        Rule::keep_high | Rule::keep_low | Rule::drop_high | Rule::drop_low => {
            let op_enum = match op.as_rule() {
                Rule::keep_high => KeepOrDropModifierOp::KeepHigh,
                Rule::keep_low => KeepOrDropModifierOp::KeepLow,
                Rule::drop_high => KeepOrDropModifierOp::DropHigh,
                Rule::drop_low => KeepOrDropModifierOp::DropLow,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_keep_or_drop_modifier(lhs, op_enum, inner_pairs)
        }
        Rule::reroll | Rule::reroll_once => {
            let op_enum = match op.as_rule() {
                Rule::reroll_once => RerollModifierOp::RerollOnce,
                Rule::reroll => RerollModifierOp::Reroll,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_reroll_modifier(lhs, op_enum, inner_pairs)
        }
        Rule::explode | Rule::compound_explode => {
            let op_enum = match op.as_rule() {
                Rule::explode => ExplodeModifierOp::Explode,
                Rule::compound_explode => ExplodeModifierOp::CompoundExplode,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_explode_modifier(lhs, op_enum, inner_pairs)
        }
        Rule::compare_param => {
            let mut inner_pairs = op.into_inner(); // 进入compare_param内部
            let op_symbol = inner_pairs.next().unwrap(); // >, <, =
            let val_pair = inner_pairs.next().unwrap(); // atom
            Expr::SuccessCheck {
                lhs: Box::new(lhs), // 被判定的对象
                compare_expr: CompareExpr {
                    op: string_to_compare_op(op_symbol.as_str()), // 比较符
                    val: Box::new(parse_atom(val_pair)),          // 目标值
                },
            }
        }
        Rule::min | Rule::max => {
            let op_enum = match op.as_rule() {
                Rule::min => MinMaxModifierOp::Min,
                Rule::max => MinMaxModifierOp::Max,
                _ => unreachable!(), // should not reach here
            };
            let inner_pairs = op.into_inner(); // 进入内部
            process_min_max_modifier(lhs, op_enum, inner_pairs)
        }
        _ => unreachable!("Unknown postfix operator: {:?}", op.as_rule()),
    }
}

fn process_mod_parameter(mut inner: pest::iterators::Pairs<Rule>) -> CompareExpr {
    let first = inner.next().unwrap();
    match first.as_rule() {
        Rule::atom => {
            let value = parse_atom(first);
            CompareExpr {
                op: CompareOp::Equal,
                val: Box::new(value),
            }
        }
        Rule::compare_op => {
            let op_symbol = first; // >, <, =
            let val_pair = inner.next().unwrap(); // atom
            CompareExpr {
                op: string_to_compare_op(op_symbol.as_str()),
                val: Box::new(parse_atom(val_pair)),
            }
        }
        _ => unreachable!("Unknown modifier parameter: {:?}", first.as_rule()),
    }
}

fn process_keep_or_drop_modifier(
    lhs: Expr,
    op: KeepOrDropModifierOp,
    mut inner: pest::iterators::Pairs<Rule>,
) -> Expr {
    let param = if let Some(mod_param) = inner.next() {
        Box::new(parse_atom(mod_param))
    } else {
        Box::new(Expr::Number(1.0))
    };
    Expr::KeepOrDropModifier {
        lhs: Box::new(lhs),
        op: op,
        count: param,
    }
}

fn process_reroll_modifier(
    lhs: Expr,
    op: RerollModifierOp,
    mut inner: pest::iterators::Pairs<Rule>,
) -> Expr {
    let param = if let Some(mod_param) = inner.next() {
        let mod_param_inner = mod_param.into_inner(); // mod_param内部
        process_mod_parameter(mod_param_inner)
    } else {
        unreachable!("Reroll modifier requires a compare expression")
    };
    Expr::RerollModifier {
        lhs: Box::new(lhs),
        op: op,
        compare_expr: param,
    }
}

fn process_explode_modifier(
    lhs: Expr,
    op: ExplodeModifierOp,
    mut inner: pest::iterators::Pairs<Rule>,
) -> Expr {
    let mut compare_expr: Option<CompareExpr> = None;
    let mut limit: Option<Box<Expr>> = None;

    match inner.next() {
        Some(first) => {
            match first.as_rule() {
                Rule::mod_param => {
                    let mod_param_inner = first.into_inner(); // mod_param内部
                    compare_expr = Some(process_mod_parameter(mod_param_inner));
                    // 如果还有下一个，则是limit
                    if let Some(second) = inner.next() {
                        let limit_inner = second.into_inner().next().unwrap(); // limit内部只有一个atom
                        limit = Some(Box::new(parse_atom(limit_inner)));
                    }
                }
                Rule::limit => {
                    let limit_inner = first.into_inner().next().unwrap(); // limit内部只有一个atom
                    limit = Some(Box::new(parse_atom(limit_inner)));
                }
                _ => unreachable!("Unknown explode modifier parameter: {:?}", first.as_rule()),
            }
        }
        None => {} // 没有参数，全部为None
    }

    Expr::ExplodeModifier {
        lhs: Box::new(lhs),
        op: op,
        compare_expr: compare_expr,
        limit: limit,
    }
}

fn process_min_max_modifier(
    lhs: Expr,
    op: MinMaxModifierOp,
    mut inner: pest::iterators::Pairs<Rule>,
) -> Expr {
    let param = if let Some(mod_param) = inner.next() {
        Box::new(parse_atom(mod_param))
    } else {
        unreachable!("Min/Max modifier requires a count parameter")
    };
    Expr::MinMaxModifier {
        lhs: Box::new(lhs),
        op: op,
        target: param,
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
            let name = inner.next().unwrap().as_str().to_string(); // func_name
            let args = match inner.next() {
                Some(args_pair) => args_pair
                    .into_inner()
                    .map(|p| parse_expr_pratt(p))
                    .collect(),
                None => vec![],
            };
            Expr::Call {
                func_name: name,
                args: args,
            }
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
