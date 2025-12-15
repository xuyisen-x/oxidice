// ==========================================
// AST 数据结构
// ==========================================

// 运算符
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
    GreaterEqual,
    Less,
    LessEqual,
    Equal,
    NotEqual,
}

// 修饰符相关
// Type1: keep_high、 keep_low、drop_high、drop_low、min、max 等接受1个atom
// Type2: compound_explode、explode、reroll 接受1个mod_param，一个limit
// Type3: count_successes等接受一个mod_param，没有limit

#[derive(Debug, Clone, PartialEq)]
pub enum Type1Op {
    KeepHigh,
    KeepLow,
    DropHigh,
    DropLow,
    Min,
    Max,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type2Op {
    CompoundExplode,
    Explode,
    Reroll,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type3Op {
    CountSuccesses,
    DeductFailures,
    SubtractFailures,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModParam {
    pub operator: CompareOp,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub limit_times: Option<Box<Expr>>,
    pub limit_counts: Option<Box<Expr>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type1Modifier {
    pub lhs: Box<Expr>,
    pub op: Type1Op,
    pub param: Box<Expr>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type2Modifier {
    pub lhs: Box<Expr>,
    pub op: Type2Op,
    pub param: Option<ModParam>,
    pub limit: Option<Limit>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type3Modifier {
    pub lhs: Box<Expr>,
    pub op: Type3Op,
    pub param: ModParam,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModifierNode {
    Type1(Type1Modifier),
    Type2(Type2Modifier),
    Type3(Type3Modifier),
}

// 函数相关
#[derive(Debug, Clone, PartialEq)]
pub enum FunctionName {
    Floor,
    Ceil,
    Round,
    Abs,
    Max,
    Min,
    Sum,
    Avg,
    Len,
    Rpdice,
    Sortd,
    Sort,
    ToList,
    Filter(ModParam),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub name: FunctionName,
    pub args: Vec<Expr>,
}

// 骰子相关
#[derive(Debug, Clone, PartialEq)]
pub enum DiceType {
    Standard { count: Box<Expr>, sides: Box<Expr> },
    Fudge { count: Box<Expr> },
    Coin { count: Box<Expr> },
}

// 二元运算
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    pub lhs: Box<Expr>,
    pub op: BinOp,
    pub rhs: Box<Expr>,
}

// 表达式定义
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Neg(Box<Expr>),
    Number(f64),
    Dice(DiceType),
    List(Vec<Expr>),
    Modifier(ModifierNode),
    Function(FunctionCall),
    Binary(BinaryOp),
}

impl Expr {
    pub fn neg(expr: Expr) -> Self {
        Expr::Neg(Box::new(expr))
    }

    pub fn number(value: f64) -> Self {
        Expr::Number(value)
    }

    pub fn normal_dice(count: Expr, sides: Expr) -> Self {
        Expr::Dice(DiceType::Standard {
            count: Box::new(count),
            sides: Box::new(sides),
        })
    }

    pub fn fudge_dice(count: Expr) -> Self {
        Expr::Dice(DiceType::Fudge {
            count: Box::new(count),
        })
    }

    pub fn coin_dice(count: Expr) -> Self {
        Expr::Dice(DiceType::Coin {
            count: Box::new(count),
        })
    }

    pub fn list(elements: Vec<Expr>) -> Self {
        Expr::List(elements)
    }

    pub fn binary(lhs: Expr, op: BinOp, rhs: Expr) -> Self {
        Expr::Binary(BinaryOp {
            lhs: Box::new(lhs),
            op,
            rhs: Box::new(rhs),
        })
    }

    pub fn function(name: FunctionName, args: Vec<Expr>) -> Self {
        Expr::Function(FunctionCall { name, args })
    }

    pub fn modifier_type1(lhs: Expr, op: Type1Op, param: Expr) -> Self {
        Expr::Modifier(ModifierNode::Type1(Type1Modifier {
            lhs: Box::new(lhs),
            op,
            param: Box::new(param),
        }))
    }

    pub fn modifier_type2(
        lhs: Expr,
        op: Type2Op,
        param: Option<ModParam>,
        limit: Option<Limit>,
    ) -> Self {
        Expr::Modifier(ModifierNode::Type2(Type2Modifier {
            lhs: Box::new(lhs),
            op,
            param,
            limit,
        }))
    }

    pub fn modifier_type3(lhs: Expr, op: Type3Op, param: ModParam) -> Self {
        Expr::Modifier(ModifierNode::Type3(Type3Modifier {
            lhs: Box::new(lhs),
            op,
            param,
        }))
    }

    pub fn mod_param(operator: CompareOp, value: Expr) -> ModParam {
        ModParam {
            operator,
            value: Box::new(value),
        }
    }
}
