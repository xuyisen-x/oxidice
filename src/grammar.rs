use crate::types::expr::*;
use winnow::Parser;
use winnow::Result as WNResultBase;
use winnow::ascii::alpha1;
use winnow::ascii::{Caseless, float, space0};
use winnow::combinator::cut_err;
use winnow::combinator::dispatch;
use winnow::combinator::{alt, delimited, fail, opt, peek, preceded, separated};
use winnow::error::ContextError;
use winnow::error::ErrMode;
use winnow::stream::Stream;
use winnow::token::any;

pub type WNResult<O, E = ContextError> = WNResultBase<O, ErrMode<E>>;

// ==========================================
// 入口函数
// ==========================================

pub fn parse_dice(input: &str) -> Result<Expr, String> {
    match parse_full_expr.parse(input) {
        Ok(expr) => Ok(expr),
        Err(e) => Err(e.to_string()),
    }
}

pub fn parse_full_expr(input: &mut &str) -> WNResult<Expr> {
    let _ = space0.parse_next(input)?; // 吃掉开头的空白
    let x = parse_expr.parse_next(input)?;
    let _ = space0.parse_next(input)?; // 吃掉结尾的空白
    Ok(x)
}

// ==========================================
// 基础工具
// ==========================================

fn ws<'a, F, O>(inner: F) -> impl Parser<&'a str, O, ErrMode<ContextError>>
where
    F: Parser<&'a str, O, ErrMode<ContextError>>,
{
    delimited(space0, inner, space0)
}

fn parse_number(input: &mut &str) -> WNResult<Expr> {
    float.map(Expr::number).parse_next(input)
}

// ==========================================
// 运算符解析
// ==========================================

fn parse_bin_op_add_sub(input: &mut &str) -> WNResult<BinOp> {
    alt(("+".map(|_| BinOp::Add), "-".map(|_| BinOp::Sub))).parse_next(input)
}

fn parse_bin_op_mul_div(input: &mut &str) -> WNResult<BinOp> {
    alt((
        "//".map(|_| BinOp::Idiv),
        "/".map(|_| BinOp::Div),
        "*".map(|_| BinOp::Mul),
        "%".map(|_| BinOp::Mod),
    ))
    .parse_next(input)
}

fn parse_compare_op(input: &mut &str) -> WNResult<CompareOp> {
    alt((
        "<>".map(|_| CompareOp::NotEqual),
        ">=".map(|_| CompareOp::GreaterEqual),
        "<=".map(|_| CompareOp::LessEqual),
        ">".map(|_| CompareOp::Greater),
        "<".map(|_| CompareOp::Less),
        "=".map(|_| CompareOp::Equal),
    ))
    .parse_next(input)
}

// ==========================================
// 递归下降解析主逻辑（递归下降逻辑可以保留，扁平化收益不大）
// ==========================================

// Level 6: Expr (加减法, 优先级最低)
pub fn parse_expr(input: &mut &str) -> WNResult<Expr> {
    let mut left = parse_term(input)?;
    while let Some(op) = opt(ws(parse_bin_op_add_sub)).parse_next(input)? {
        let right = parse_term(input)?;
        left = Expr::binary(left, op, right);
    }
    Ok(left)
}

// Level 5: Term (乘除模)
fn parse_term(input: &mut &str) -> WNResult<Expr> {
    let mut left = parse_unary(input)?;
    while let Some(op) = opt(ws(parse_bin_op_mul_div)).parse_next(input)? {
        let right = parse_unary(input)?;
        left = Expr::binary(left, op, right);
    }
    Ok(left)
}

// Level 4: Unary Prefix (正负号)
fn parse_unary(input: &mut &str) -> WNResult<Expr> {
    alt((
        // 负号: 递归调用 parse_unary (支持 --1) 或进入下一层
        preceded(ws("-"), parse_unary).map(Expr::neg),
        // 正号: 忽略，直接解析下一层
        preceded(ws("+"), parse_unary),
        // 无前缀: 解析 Dice With Modifiers
        parse_dice_with_modifiers,
    ))
    .parse_next(input)
}

// Level 3: Dice Modifiers (后缀修饰符)
fn parse_dice_with_modifiers(input: &mut &str) -> WNResult<Expr> {
    let mut base = parse_dice_expr(input)?;
    while let Some(builder) = opt(parse_modifier_op).parse_next(input)? {
        base = builder(base);
    }
    Ok(base)
}

type ModifierBuilder = Box<dyn FnOnce(Expr) -> Expr>;

fn parse_modifier_op(input: &mut &str) -> WNResult<ModifierBuilder> {
    fn parse_d_modifiers(input: &mut &str) -> WNResult<ModifierBuilder> {
        dispatch!(peek(preceded(any, any));
            'h' | 'H' | 'l' | 'L' => parse_type1_modifier, // 匹配 dh, dl
            'f' | 'F' => parse_type3_modifier,             // 匹配 df
            _ => fail
        )
        .parse_next(input)
    }
    dispatch!(peek(any);
        'k' | 'K' => parse_type1_modifier, // kh, kl
        'd' | 'D' => parse_d_modifiers, // dh, dl (Type1) vs df (Type3)
        'm' | 'M' => parse_type1_modifier, // min, max
        'r' | 'R' => parse_type2_modifier, // r (Type2)
        '!'       => parse_type2_modifier, // !, !! (Type2)
        'c' | 'C' => parse_type3_modifier, // cs (Type3)
        's' | 'S' => parse_type3_modifier, // sf (Type3)
        _ => fail
    )
    .parse_next(input)
}

// Level 2: Dice Expression (XdY, dY, XdF)
// 逻辑: (Atom ~ "d" ~ Atom) | ("d" ~ Atom)
fn parse_dice_expr(input: &mut &str) -> WNResult<Expr> {
    // 先尝试解析一个Atom
    let left_opt = opt(parse_atom).parse_next(input)?;

    // 检查下一个是否是骰子符号
    let next_is_dice = peek::<_, _, ContextError, _>(Caseless("d"))
        .parse_next(input)
        .is_ok();

    if !next_is_dice {
        // 如果没有骰子符号
        if let Some(left) = left_opt {
            return Ok(left);
        } else {
            // 既没有左值也没有骰子符号，这不是有效的 DiceExpr
            return fail.parse_next(input);
        }
    }

    // 解析操作符
    let op_str = alt((Caseless("df"), Caseless("dc"), Caseless("d"))).parse_next(input)?;

    // 确定左值，默认为 1
    let count = left_opt.unwrap_or_else(|| Expr::number(1.0));

    match op_str.to_lowercase().as_str() {
        "df" => Ok(Expr::fudge_dice(count)),
        "dc" => Ok(Expr::coin_dice(count)),
        "d" => {
            // 标准骰子，必须跟面数
            let sides = parse_atom(input)?;
            Ok(Expr::normal_dice(count, sides))
        }
        _ => unreachable!(),
    }
}

// Level 1: Atom
// 优先级最高的基础单元
fn parse_atom(input: &mut &str) -> WNResult<Expr> {
    dispatch!(peek(any);
        'a'..='z' | 'A'..='Z' => parse_function_call, // 是字母，直接解析函数
        '[' => parse_list,          // 是[，解析列表
        '0'..='9' | '.' => parse_number,        // 是数字，解析数字
        '(' => delimited("(", parse_expr, ")"), // 括号表达式
        '{' => delimited("{", parse_expr, "}"), // 花括号表达
        _ => fail                              // 其他字符直接报错
    )
    .parse_next(input)
}

// ==========================================
// 具体组件解析 (Lists, Functions)
// ==========================================

fn parse_list(input: &mut &str) -> WNResult<Expr> {
    delimited(
        "[",
        separated(0.., parse_expr, ws(",")).map(Expr::list),
        "]",
    )
    .parse_next(input)
}

fn parse_function_call(input: &mut &str) -> WNResult<Expr> {
    let start = input.checkpoint();
    let name = alpha1.parse_next(input)?; // 吃掉函数名
    let func_type = match name.to_lowercase().as_str() {
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
        "filter" => {
            let param = cut_err(parse_mod_param).parse_next(input)?;
            FunctionName::Filter(param)
        }
        _ => {
            input.reset(&start);
            return fail(input);
        }
    };
    let args = cut_err(delimited("(", parse_args, ")")).parse_next(input)?;
    Ok(Expr::function(func_type, args))
}

fn parse_args(input: &mut &str) -> WNResult<Vec<Expr>> {
    separated(0.., parse_expr, ws(",")).parse_next(input)
}

// ==========================================
// 5. 修饰符细节解析
// ==========================================

fn parse_mod_param(input: &mut &str) -> WNResult<ModParam> {
    let (op, val) = (opt(parse_compare_op), parse_atom).parse_next(input)?;
    // 如果没有提供比较符号，默认为 Equal
    Ok(Expr::mod_param(op.unwrap_or(CompareOp::Equal), val))
}

// Type 1: kh, kl, dh, dl, min, max (Optional Atom)
// 返回一个构建器闭包
fn parse_type1_modifier(input: &mut &str) -> WNResult<ModifierBuilder> {
    let start = input.checkpoint();
    let tag_str = alpha1.parse_next(input)?;

    let op = match tag_str.to_lowercase().as_str() {
        "kh" => Type1Op::KeepHigh,
        "kl" => Type1Op::KeepLow,
        "dh" => Type1Op::DropHigh,
        "dl" => Type1Op::DropLow,
        "min" => Type1Op::Min,
        "max" => Type1Op::Max,
        _ => {
            input.reset(&start);
            return fail(input);
        }
    };

    let val_opt = if op == Type1Op::Min || op == Type1Op::Max {
        // min/max 必须有参数
        Some(cut_err(parse_atom).parse_next(input)?)
    } else {
        // kh, kl, dh, dl 参数可选
        opt(parse_atom).parse_next(input)?
    };

    Ok(Box::new(move |lhs| {
        let param = val_opt.unwrap_or(Expr::number(1.0));
        Expr::modifier_type1(lhs, op, param)
    }))
}

// Type 2: r, !!, ! (ModParam? + Limit?)
fn parse_type2_modifier(input: &mut &str) -> WNResult<ModifierBuilder> {
    let tag_str = alt((
        "!!", // compound explode 必须在 explode 前面匹配
        "!",
        Caseless("r"),
    ))
    .parse_next(input)?;

    let op = match tag_str.to_lowercase().as_str() {
        "!!" => Type2Op::CompoundExplode,
        "!" => Type2Op::Explode,
        "r" => Type2Op::Reroll,
        _ => unreachable!(),
    };

    let param = opt(parse_mod_param).parse_next(input)?;
    let limit = opt(parse_limit).parse_next(input)?;

    if op == Type2Op::Reroll && param.is_none() {
        // r 修饰符必须有参数
        return fail(input);
    }

    Ok(Box::new(move |lhs| {
        Expr::modifier_type2(lhs, op, param.clone(), limit.clone())
    }))
}

// Type 3: cs, df, sf (Required ModParam)
fn parse_type3_modifier(input: &mut &str) -> WNResult<ModifierBuilder> {
    let start = input.checkpoint();
    let tag_str = alpha1.parse_next(input)?;

    let op = match tag_str.to_lowercase().as_str() {
        "cs" => Type3Op::CountSuccesses,
        "df" => Type3Op::DeductFailures,
        "sf" => Type3Op::SubtractFailures,
        _ => {
            input.reset(&start);
            return fail(input);
        }
    };

    let param = cut_err(parse_mod_param).parse_next(input)?;

    Ok(Box::new(move |lhs| Expr::modifier_type3(lhs, op, param)))
}

// 解析 limit: lt3, lc2, 或组合
fn parse_limit(input: &mut &str) -> WNResult<Limit> {
    let mut times = None;
    let mut counts = None;
    let mut parsed_times = false;
    let mut parsed_counts = false;

    // 尝试解析最多两个组件
    for _ in 0..2 {
        if peek::<_, _, ContextError, _>(Caseless("lt"))
            .parse_next(input)
            .is_ok()
        {
            if parsed_times {
                return cut_err(fail).parse_next(input);
            }
            parsed_times = true;
            let val = preceded(Caseless("lt"), parse_atom).parse_next(input)?;
            times = Some(Box::new(val));
        } else if peek::<_, _, ContextError, _>(Caseless("lc"))
            .parse_next(input)
            .is_ok()
        {
            if parsed_counts {
                return cut_err(fail).parse_next(input);
            }
            parsed_counts = true;
            let val = preceded(Caseless("lc"), parse_atom).parse_next(input)?;
            counts = Some(Box::new(val));
        } else {
            break;
        }
    }

    if times.is_none() && counts.is_none() {
        fail(input) // 不是 limit
    } else {
        Ok(Limit {
            limit_times: times,
            limit_counts: counts,
        })
    }
}
