use super::expr::CompareOp;

#[derive(Copy, Clone, Debug)]
pub struct NodeId(pub u32); // A simple wrapper for node identifiers

impl NodeId {
    pub fn to_index(&self) -> usize {
        self.0 as usize
    }
}

pub struct EvalGraph {
    pub nodes: Vec<EvalNode>,
    pub root: NodeId,
}

pub enum EvalNode {
    // 数值类型与列表类型的基本类型
    Constant(f64),
    ListConstruct(Vec<NodeId>),

    // 单目运算
    NumNegate(NodeId),

    // 二元运算——数字
    NumAdd(NodeId, NodeId),
    NumSubtract(NodeId, NodeId),
    NumMultiply(NodeId, NodeId),
    NumDivide(NodeId, NodeId),
    NumIntDivide(NodeId, NodeId),
    NumModulo(NodeId, NodeId),
    // 二元运算——列表
    Concat(NodeId, NodeId),
    ListAdd(NodeId, NodeId),
    ListMultiply(NodeId, NodeId),
    ListSubtract(NodeId, NodeId),
    ListSubtractReverse(NodeId, NodeId),
    ListDivide(NodeId, NodeId),
    ListDivideReverse(NodeId, NodeId),
    ListIntDivide(NodeId, NodeId),
    ListIntDivideReverse(NodeId, NodeId),
    ListModulo(NodeId, NodeId),
    ListModuloReverse(NodeId, NodeId),

    // 函数调用——返回数值
    NumFloor(NodeId),
    NumCeil(NodeId),
    NumRound(NodeId),
    NumAbs(NodeId),
    NumMax(NodeId),
    NumMin(NodeId),
    NumSum(NodeId),
    NumAvg(NodeId),
    NumLen(NodeId),
    // 函数调用——返回列表
    ListFloor(NodeId),
    ListCeil(NodeId),
    ListRound(NodeId),
    ListAbs(NodeId),
    ListMax(NodeId, NodeId),
    ListMin(NodeId, NodeId),
    ListSort(NodeId),
    ListSortDesc(NodeId),
    ListToListFromDicePool(NodeId),
    ListToListFromSuccessPool(NodeId),
    ListFilter(NodeId, ModParamNode),

    // 骰子池
    DiceStandard(NodeId, NodeId),
    DiceFudge(NodeId),
    DiceCoin(NodeId),
    DiceKeepHigh(NodeId, NodeId),
    DiceKeepLow(NodeId, NodeId),
    DiceDropHigh(NodeId, NodeId),
    DiceDropLow(NodeId, NodeId),
    DiceMin(NodeId, NodeId),
    DiceMax(NodeId, NodeId),
    DiceExplode(NodeId, Option<ModParamNode>, Option<LimitNode>),
    DiceCompoundExplode(NodeId, Option<ModParamNode>, Option<LimitNode>),
    DiceReroll(NodeId, ModParamNode, Option<LimitNode>),
    DiceSubtractFailures(NodeId, ModParamNode),
    DiceCountSuccessesFromDicePool(NodeId, ModParamNode),
    DiceDeductFailuresFromDicePool(NodeId, ModParamNode),
    DiceCountSuccesses(NodeId, ModParamNode),
    DiceDeductFailures(NodeId, ModParamNode),
}

#[derive(Debug, Clone)]
pub struct ModParamNode {
    pub operator: CompareOp,
    pub value: NodeId,
}

pub struct LimitNode {
    pub limit_times: Option<NodeId>,
    pub limit_counts: Option<NodeId>,
}
