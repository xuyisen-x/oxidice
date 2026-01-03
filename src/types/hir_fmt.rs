use super::expr::CompareOp;
use super::hir::*;
use std::fmt;

// ==========================================
// 优先级定义
// ==========================================

#[derive(PartialEq, PartialOrd, Copy, Clone)]
enum Precedence {
    Sum = 10,     // 加法、减法
    Product = 20, // 乘法、除法、取模
    Dice = 30,    // 骰子运算 (d, kh, !, etc.)
    Prefix = 40,  // 单目运算符 (Neg)
    Call = 50,    // 函数调用、原子值
}

// ==========================================
// Display 实现入口
// ==========================================

impl fmt::Display for HIR {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HIR::Number(n) => write!(f, "{}", n),
            HIR::List(l) => write!(f, "{}", l),
        }
    }
}

// ==========================================
// NumberType 实现
// ==========================================

impl NumberType {
    fn precedence(&self) -> Precedence {
        match self {
            NumberType::Constant(_) => Precedence::Call,
            NumberType::DicePool(_) => Precedence::Dice,
            NumberType::SuccessPool(_) => Precedence::Dice,
            NumberType::NumberFunction(_) => Precedence::Call,
            NumberType::Neg(_) => Precedence::Prefix,
            NumberType::NumberBinary(op) => match op {
                NumberBinaryType::Add(_, _) | NumberBinaryType::Subtract(_, _) => Precedence::Sum,
                NumberBinaryType::Multiply(_, _)
                | NumberBinaryType::Divide(_, _)
                | NumberBinaryType::IntDivide(_, _)
                | NumberBinaryType::Modulo(_, _) => Precedence::Product,
            },
        }
    }
}

impl fmt::Display for NumberType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberType::Constant(c) => write!(f, "{}", c),
            NumberType::DicePool(d) => write!(f, "{}", d),
            NumberType::SuccessPool(s) => write!(f, "{}", s),
            NumberType::NumberFunction(func) => write!(f, "{}", func),
            NumberType::Neg(inner) => {
                if inner.precedence() < Precedence::Prefix {
                    write!(f, "-({})", inner)
                } else {
                    write!(f, "-{}", inner)
                }
            }
            NumberType::NumberBinary(op) => {
                let prec = self.precedence();
                let (lhs, symbol, rhs) = match op {
                    NumberBinaryType::Add(l, r) => (l, "+", r),
                    NumberBinaryType::Subtract(l, r) => (l, "-", r),
                    NumberBinaryType::Multiply(l, r) => (l, "*", r),
                    NumberBinaryType::Divide(l, r) => (l, "/", r),
                    NumberBinaryType::IntDivide(l, r) => (l, "//", r),
                    NumberBinaryType::Modulo(l, r) => (l, "%", r),
                };

                // 左侧：如果优先级低于当前，加括号
                if lhs.precedence() < prec {
                    write!(f, "({})", lhs)?;
                } else {
                    write!(f, "{}", lhs)?;
                }

                write!(f, "{}", symbol)?;

                // 右侧：处理左结合律 (Left Associative)
                // 如果右侧优先级低于 OR 等于当前(例如 1-2-3 -> (1-2)-3，右侧是3，不需要括号；
                // 但如果是 1-(2-3)，右侧是减法，需要括号)。
                // 注意：这里假设同级运算如果不加括号，解析器默认向左结合。
                // 所以如果右侧是同级运算，为了保持AST意图（它作为右子树），必须加括号。
                if rhs.precedence() < prec || (rhs.precedence() == prec) {
                    write!(f, "({})", rhs)
                } else {
                    write!(f, "{}", rhs)
                }
            }
        }
    }
}

// ==========================================
// DicePoolType 实现
// ==========================================

impl fmt::Display for DicePoolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DicePoolType::Standard(count, sides) => {
                // 如果 count 本身是 Dice (如 1d20)，必须变成 (1d20)d6
                if count.precedence() <= Precedence::Dice {
                    write!(f, "({})", count)?;
                } else {
                    write!(f, "{}", count)?;
                }
                write!(f, "d")?;
                // 同样处理右侧，虽然少见，但支持 1d(1d6)
                if sides.precedence() <= Precedence::Dice {
                    write!(f, "({})", sides)
                } else {
                    write!(f, "{}", sides)
                }
            }
            DicePoolType::Fudge(count) => {
                if count.precedence() <= Precedence::Dice {
                    write!(f, "({})dF", count)
                } else {
                    write!(f, "{}dF", count)
                }
            }
            DicePoolType::Coin(count) => {
                if count.precedence() <= Precedence::Dice {
                    write!(f, "({})dC", count)
                } else {
                    write!(f, "{}dC", count)
                }
            }
            // 修饰符紧凑连接
            DicePoolType::KeepHigh(inner, n) => write!(f, "{}kh{}", inner, n),
            DicePoolType::KeepLow(inner, n) => write!(f, "{}kl{}", inner, n),
            DicePoolType::DropHigh(inner, n) => write!(f, "{}dh{}", inner, n),
            DicePoolType::DropLow(inner, n) => write!(f, "{}dl{}", inner, n),
            DicePoolType::Min(inner, n) => write!(f, "{}min{}", inner, n),
            DicePoolType::Max(inner, n) => write!(f, "{}max{}", inner, n),
            DicePoolType::Explode(inner, mp, limit) => {
                write!(f, "{}!", inner)?;
                if let Some(mp) = mp {
                    write!(f, "{}", mp)?;
                }
                if let Some(l) = limit {
                    write!(f, "{}", l)?;
                }
                Ok(())
            }
            DicePoolType::CompoundExplode(inner, mp, limit) => {
                write!(f, "{}!!", inner)?;
                if let Some(mp) = mp {
                    write!(f, "{}", mp)?;
                }
                if let Some(l) = limit {
                    write!(f, "{}", l)?;
                }
                Ok(())
            }
            DicePoolType::Reroll(inner, mp, limit) => {
                write!(f, "{}r{}", inner, mp)?;
                if let Some(l) = limit {
                    write!(f, "{}", l)?;
                }
                Ok(())
            }
            DicePoolType::SubtractFailures(inner, mp) => write!(f, "{}sf{}", inner, mp),
        }
    }
}

// ==========================================
// SuccessPoolType 实现
// ==========================================

impl fmt::Display for SuccessPoolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SuccessPoolType::CountSuccessesFromDicePool(dp, mp) => write!(f, "{}cs{}", dp, mp),
            SuccessPoolType::DeductFailuresFromDicePool(dp, mp) => write!(f, "{}df{}", dp, mp),
            SuccessPoolType::CountSuccesses(inner, mp) => write!(f, "{}cs{}", inner, mp),
            SuccessPoolType::DeductFailures(inner, mp) => write!(f, "{}df{}", inner, mp),
        }
    }
}

// ==========================================
// ListType 实现
// ==========================================

impl ListType {
    fn precedence(&self) -> Precedence {
        match self {
            // 显式列表和函数调用被视为最高优先级（原子级），不需要括号
            ListType::Explicit(_) => Precedence::Call,
            ListType::ListFunction(_) => Precedence::Call,

            ListType::ListBinary(op) => match op {
                // 加减法归为 Sum
                ListBinaryType::AddList(_, _)
                | ListBinaryType::Add(_, _)
                | ListBinaryType::Subtract(_, _)
                | ListBinaryType::SubtractReverse(_, _) => Precedence::Sum,

                // 其他归为 Product (乘除、取模、指数)
                _ => Precedence::Product,
            },
        }
    }
}

impl fmt::Display for ListType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListType::Explicit(vec) => {
                write!(f, "[")?;
                for (i, item) in vec.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            ListType::ListFunction(func) => write!(f, "{}", func),
            ListType::ListBinary(op) => {
                let self_prec = self.precedence();
                let fmt_child = |f: &mut fmt::Formatter,
                                 child_prec: Precedence,
                                 child_disp: &dyn fmt::Display|
                 -> fmt::Result {
                    if child_prec <= self_prec {
                        write!(f, "({})", child_disp)
                    } else {
                        write!(f, "{}", child_disp)
                    }
                };

                // 分解操作符，分别处理 List 和 Number 类型的子节点
                match op {
                    // --- List op List ---
                    ListBinaryType::AddList(l, r) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "+")?;
                        fmt_child(f, r.precedence(), r)
                    }

                    // --- List op Number (广播) ---
                    ListBinaryType::MultiplyList(l, n) => {
                        // List ** Number
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "**")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::Add(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "+")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::Multiply(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "*")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::Subtract(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "-")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::Divide(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "/")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::IntDivide(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "//")?;
                        fmt_child(f, n.precedence(), n)
                    }
                    ListBinaryType::Modulo(l, n) => {
                        fmt_child(f, l.precedence(), l)?;
                        write!(f, "%")?;
                        fmt_child(f, n.precedence(), n)
                    }

                    // --- Number op List (反向广播) ---
                    ListBinaryType::SubtractReverse(n, l) => {
                        fmt_child(f, n.precedence(), n)?;
                        write!(f, "-")?;
                        fmt_child(f, l.precedence(), l)
                    }
                    ListBinaryType::DivideReverse(n, l) => {
                        fmt_child(f, n.precedence(), n)?;
                        write!(f, "/")?;
                        fmt_child(f, l.precedence(), l)
                    }
                    ListBinaryType::IntDivideReverse(n, l) => {
                        fmt_child(f, n.precedence(), n)?;
                        write!(f, "//")?;
                        fmt_child(f, l.precedence(), l)
                    }
                    ListBinaryType::ModuloReverse(n, l) => {
                        fmt_child(f, n.precedence(), n)?;
                        write!(f, "%")?;
                        fmt_child(f, l.precedence(), l)
                    }
                }
            }
        }
    }
}

// ==========================================
// Function Types 实现
// ==========================================

impl fmt::Display for NumberFunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 函数参数间移除空格
        match self {
            NumberFunctionType::Floor(n) => write!(f, "floor({})", n),
            NumberFunctionType::Ceil(n) => write!(f, "ceil({})", n),
            NumberFunctionType::Round(n) => write!(f, "round({})", n),
            NumberFunctionType::Abs(n) => write!(f, "abs({})", n),
            NumberFunctionType::Max(l) => write!(f, "max({})", l),
            NumberFunctionType::Min(l) => write!(f, "min({})", l),
            NumberFunctionType::Sum(l) => write!(f, "sum({})", l),
            NumberFunctionType::Avg(l) => write!(f, "avg({})", l),
            NumberFunctionType::Len(l) => write!(f, "len({})", l),
        }
    }
}

impl fmt::Display for ListFunctionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 函数参数间移除空格
        match self {
            ListFunctionType::Floor(l) => write!(f, "floor({})", l),
            ListFunctionType::Ceil(l) => write!(f, "ceil({})", l),
            ListFunctionType::Round(l) => write!(f, "round({})", l),
            ListFunctionType::Abs(l) => write!(f, "abs({})", l),
            ListFunctionType::Max(l, n) => write!(f, "max({},{})", l, n),
            ListFunctionType::Min(l, n) => write!(f, "min({},{})", l, n),
            ListFunctionType::Sort(l) => write!(f, "sort({})", l),
            ListFunctionType::SortDesc(l) => write!(f, "sortd({})", l),
            ListFunctionType::ToListFromDicePool(d) => write!(f, "tolist({})", d),
            ListFunctionType::ToListFromSuccessPool(s) => write!(f, "tolist({})", s),
            ListFunctionType::Filter(l, mp) => {
                let ModParam {
                    operator: op,
                    value: v,
                } = mp;
                let mp_str = if v.is_constant() {
                    format!("{}{}", op, v)
                } else {
                    format!("{}({})", op, v)
                };
                write!(f, "filter{}({})", mp_str, l)
            }
        }
    }
}

// ==========================================
// 辅助类型实现
// ==========================================

impl fmt::Display for CompareOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            CompareOp::Equal => "=",
            CompareOp::Greater => ">",
            CompareOp::GreaterEqual => ">=",
            CompareOp::Less => "<",
            CompareOp::LessEqual => "<=",
            CompareOp::NotEqual => "<>",
        };
        write!(f, "{}", s)
    }
}

impl fmt::Display for ModParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", self.operator, self.value)
    }
}

impl fmt::Display for Limit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(times) = &self.limit_times {
            write!(f, "lt{}", times)?;
        }
        if let Some(counts) = &self.limit_counts {
            write!(f, "lc{}", counts)?;
        }
        Ok(())
    }
}
