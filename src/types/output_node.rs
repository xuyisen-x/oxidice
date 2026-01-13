use serde::Serialize;
use tsify::Tsify;

// 优先级枚举，完全参考你的 HIR 定义
#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub enum Precedence {
    Sum = 10,     // +, -
    Product = 20, // *, /, %
    Dice = 30,    // d, kh, !, etc.
    Prefix = 40,  // - (负号)
    Call = 50,    // 函数调用, 原子值, 列表构造
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum DiceFaceType {
    Standard(i32), // 标准骰子，面数
    Fudge,         // Fudge骰子
    Coin,          // Coin骰子
}

// 简化的值的摘要，方便前端直接显示，不需要处理复杂的 Enum
#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ValueSummary {
    Number(f64),
    List(Vec<f64>),
    DicePool {
        total: i32,
        face: DiceFaceType,
        details: Vec<DieDetailSummary>,
    },
    SuccessPool {
        count: i32,
        face: DiceFaceType,
        details: Vec<DieDetailSummary>,
    },
    NotComputed, // 对应 Waiting 或 Error，应该不会出现在最终结果中，但是保留，以保证健壮性
}

// 定义节点的渲染布局模式
#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(tag = "type", content = "children", rename_all = "camelCase")]
pub enum NodeLayout {
    Atom,                                         // 原子值，直接显示 label (例如常量)
    List(Vec<OutputNode>),                        // 列表构造: "[" + Children.join(", ") + "]"
    Prefix(Box<OutputNode>),                      // 前缀操作符: Label + Child (例如 -5)
    Infix(Box<OutputNode>, Box<OutputNode>),      // 中缀操作符: Left + Label + Right (例如 1 + 2)
    TightInfix(Box<OutputNode>, Box<OutputNode>), // 紧凑中缀操作符: Left+Label+Right (例如 2d6, kh3)
    TightPostfix(Box<OutputNode>),                // 后缀操作符: Child+Label (例如 3 dF, 4 dC)
    Function(Vec<OutputNode>), // 函数调用: Label + "(" + Children.join(",") + ")"
    // 特殊函数调用: Label + Children[0] + Children[2] + "(" + Children[1] + ")"
    Filter(Box<String>, Box<OutputNode>, Box<OutputNode>),
    // 特殊修饰符，如爆炸、重投等，mod_param, lt, lc
    SpecialModifier(
        Box<OutputNode>,
        Option<Box<(String, OutputNode)>>,
        Option<Box<OutputNode>>,
        Option<Box<OutputNode>>,
    ),
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub enum OutcomeType {
    Success,
    Failure,
    None,
}

#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct DieDetailSummary {
    pub result: i32,
    pub is_kept: bool,
    pub roll_history: Vec<i32>, // 包含聚合爆炸的所有结果
    pub is_rerolled: bool,      // 是否导致了重掷
    pub exploded_times: i32,    // 该骰子爆炸了多少次，用于compound骰子显示
    pub outcome: OutcomeType,   // "Success", "Failure", "None"
}

// 核心输出节点
#[derive(Debug, Clone, Serialize, Tsify)]
#[serde(rename_all = "camelCase")]
#[tsify(into_wasm_abi)]
pub struct OutputNode {
    pub id: u32,

    // 显示文本，例如 "+", "1d6", "floor", "keep high"
    // 对于二元运算，这是操作符；对于函数，这是函数名；对于常量，这是值。
    pub label: String,

    // 计算出的实际值
    pub value: ValueSummary,

    // 节点的布局方式，决定如何渲染
    pub layout: NodeLayout,

    // 是否要在节点周围加括号，以保持正确的运算顺序
    // 在rust中渲染完成，避免由js来处理优先级关系
    pub wrap_in_parentheses: bool,
}
