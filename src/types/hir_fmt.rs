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
    Dice = 30,    // 骰子运算 (d, kh, !, etc.) 通常结合得很紧密
    Prefix = 40,  // 单目运算符 (Neg)
    Call = 50,    // 函数调用、原子值 (数字、列表字面量)
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
    // 获取当前节点的优先级
    fn precedence(&self) -> Precedence {
        match self {
            NumberType::Constant(_) => Precedence::Call,
            NumberType::DicePool(_) => Precedence::Dice,
            NumberType::SuccessPool(_) => Precedence::Dice, // 成功池类似骰子池
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
                // 如果内部优先级低于前缀运算，加括号: -(1+2)
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

                // 左侧：如果左子节点优先级 < 当前优先级，加括号
                if lhs.precedence() < prec {
                    write!(f, "({})", lhs)?;
                } else {
                    write!(f, "{}", lhs)?;
                }

                write!(f, " {} ", symbol)?;

                // 右侧：处理结合律
                // 对于 -, /, //, % 这种非交换律运算，或者左结合运算，
                // 如果右侧优先级 == 当前优先级，必须加括号。例如: 1 - (2 - 3)
                // 加法乘法虽然满足交换律，但为了保持 AST 结构一致，通常也遵循左结合
                if rhs.precedence() < prec || (rhs.precedence() == prec) {
                    // 注意：这里假设所有二元运算都是左结合的
                    // 如果 rhs 优先级相同，意味着它可能是同级运算，放在右边需要括号
                    // e.g. self is Div, rhs is Div -> a / (b / c)
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

// 骰子池通常是从左向右构建的链式修饰符，例如 3d6kh2!
// 基础部分是 Standard/Fudge/Coin，其他都是修饰符
impl fmt::Display for DicePoolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DicePoolType::Standard(count, sides) => {
                // 检查 count 是否需要括号 (比如 (1+2)d6)
                if count.precedence() < Precedence::Dice {
                    write!(f, "({})", count)?;
                } else {
                    write!(f, "{}", count)?;
                }
                write!(f, "d")?;
                // 检查 sides 是否需要括号
                if sides.precedence() < Precedence::Dice {
                    write!(f, "({})", sides)
                } else {
                    write!(f, "{}", sides)
                }
            }
            DicePoolType::Fudge(count) => {
                if count.precedence() < Precedence::Dice {
                    write!(f, "({})dF", count)
                } else {
                    write!(f, "{}dF", count)
                }
            }
            DicePoolType::Coin(count) => {
                if count.precedence() < Precedence::Dice {
                    write!(f, "({})dC", count)
                } else {
                    write!(f, "{}dC", count)
                }
            }
            // 递归处理修饰符，不需要加括号，因为骰子修饰符紧跟在后面
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
            // SuccessPool 也可以链式调用
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
            ListType::Explicit(_) => Precedence::Call,
            ListType::ListFunction(_) => Precedence::Call,
            ListType::ListBinary(op) => match op {
                ListBinaryType::AddList(_, _)
                | ListBinaryType::Add(_, _)
                | ListBinaryType::Subtract(_, _)
                | ListBinaryType::SubtractReverse(_, _) => Precedence::Sum,
                _ => Precedence::Product, // 其他大部分是乘除类或广播
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
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            ListType::ListFunction(func) => write!(f, "{}", func),
            ListType::ListBinary(op) => {
                let prec = self.precedence();
                // 辅助闭包：处理 NumberType 和 ListType 混合时的括号逻辑
                // is_right: 是否是右操作数 (处理结合律)
                let fmt_num = |f: &mut fmt::Formatter, n: &NumberType, is_right: bool| {
                    if n.precedence() < prec || (is_right && n.precedence() == prec) {
                        write!(f, "({})", n)
                    } else {
                        write!(f, "{}", n)
                    }
                };
                let fmt_list = |f: &mut fmt::Formatter, l: &ListType, is_right: bool| {
                    if l.precedence() < prec || (is_right && l.precedence() == prec) {
                        write!(f, "({})", l)
                    } else {
                        write!(f, "{}", l)
                    }
                };

                match op {
                    ListBinaryType::AddList(l, r) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " + ")?;
                        fmt_list(f, r, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::MultiplyList(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " ** ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    // 广播操作
                    ListBinaryType::Add(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " + ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::Multiply(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " * ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::Subtract(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " - ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::Divide(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " / ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::IntDivide(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " // ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::Modulo(l, n) => {
                        write!(f, "(")?;
                        fmt_list(f, l, false)?;
                        write!(f, " % ")?;
                        fmt_num(f, n, true)?;
                        write!(f, ")")
                    }

                    // 反向操作 (Num op List)
                    ListBinaryType::SubtractReverse(n, l) => {
                        write!(f, "(")?;
                        fmt_num(f, n, false)?;
                        write!(f, " - ")?;
                        fmt_list(f, l, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::DivideReverse(n, l) => {
                        write!(f, "(")?;
                        fmt_num(f, n, false)?;
                        write!(f, " / ")?;
                        fmt_list(f, l, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::IntDivideReverse(n, l) => {
                        write!(f, "(")?;
                        fmt_num(f, n, false)?;
                        write!(f, " // ")?;
                        fmt_list(f, l, true)?;
                        write!(f, ")")
                    }
                    ListBinaryType::ModuloReverse(n, l) => {
                        write!(f, "(")?;
                        fmt_num(f, n, false)?;
                        write!(f, " % ")?;
                        fmt_list(f, l, true)?;
                        write!(f, ")")
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
        match self {
            ListFunctionType::Floor(l) => write!(f, "floor({})", l),
            ListFunctionType::Ceil(l) => write!(f, "ceil({})", l),
            ListFunctionType::Round(l) => write!(f, "round({})", l),
            ListFunctionType::Abs(l) => write!(f, "abs({})", l),
            ListFunctionType::Max(l, n) => write!(f, "max({}, {})", l, n),
            ListFunctionType::Min(l, n) => write!(f, "min({}, {})", l, n),
            ListFunctionType::Sort(l) => write!(f, "sort({})", l),
            ListFunctionType::SortDesc(l) => write!(f, "sortd({})", l),
            ListFunctionType::ToListFromDicePool(d) => write!(f, "tolist({})", d),
            ListFunctionType::ToListFromSuccessPool(s) => write!(f, "tolist({})", s),
            ListFunctionType::Filter(l, mp) => write!(f, "filter({}, {})", l, mp),
        }
    }
}

// ==========================================
// 辅助类型实现
// ==========================================

// 需要手动实现 CompareOp 的 Display，或者从 Expr 引入
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
        // 假设 Limit 格式是 [times] 或者 [times, counts] ?
        // 根据你的HIR定义，Limit 好像是用在 explode 里的，比如 !>5[3]
        // 这里只是示意，格式取决于你的语法设计
        if let Some(times) = &self.limit_times {
            write!(f, "lt{}", times)?;
        }
        if let Some(counts) = &self.limit_counts {
            // 如果两个都有，怎么分？假设不支持同时显示或者用逗号分隔
            write!(f, "lc{}", counts)?;
        }
        Ok(())
    }
}
