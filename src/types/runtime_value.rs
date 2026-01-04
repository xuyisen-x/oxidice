use crate::types::eval_graph::*;

// ==========================================
// 运行时值
// ==========================================
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    List(Vec<f64>),
    DicePool(DicePoolType),
    SuccessPool(SuccessPoolType),
}

#[derive(Debug, Clone)]
pub struct DicePoolType {
    pub total: i32,
    pub face: DiceFace,
    pub details: Vec<DieDetail>,
}

#[derive(Debug, Clone)]
pub struct SuccessPoolType {
    pub success_count: i32,
    pub face: DiceFace,
    pub details: Vec<DieDetail>,
}

impl RuntimeValue {
    pub fn except_number(&self) -> Result<f64, String> {
        match self {
            RuntimeValue::Number(v) => Ok(*v),
            RuntimeValue::DicePool(dp) => Ok(dp.total as f64),
            RuntimeValue::SuccessPool(sp) => Ok(sp.success_count as f64),
            RuntimeValue::List(_) => Err("Expected a number but got a list".to_string()),
        }
    }
    pub fn except_list(&self) -> Result<&Vec<f64>, String> {
        match self {
            RuntimeValue::List(v) => Ok(v),
            RuntimeValue::Number(_) => Err("Expected a list but got a number".to_string()),
            RuntimeValue::DicePool(_) => Err("Expected a list but got a dice pool".to_string()),
            RuntimeValue::SuccessPool(_) => {
                Err("Expected a list but got a success pool".to_string())
            }
        }
    }
    pub fn except_dice_pool(&self) -> Result<&DicePoolType, String> {
        match self {
            RuntimeValue::DicePool(v) => Ok(v),
            RuntimeValue::Number(_) => Err("Expected a dice pool but got a number".to_string()),
            RuntimeValue::List(_) => Err("Expected a dice pool but got a list".to_string()),
            RuntimeValue::SuccessPool(_) => {
                Err("Expected a dice pool but got a success pool".to_string())
            }
        }
    }
    pub fn except_success_pool(&self) -> Result<&SuccessPoolType, String> {
        match self {
            RuntimeValue::SuccessPool(v) => Ok(v),
            RuntimeValue::Number(_) => Err("Expected a success pool but got a number".to_string()),
            RuntimeValue::List(_) => Err("Expected a success pool but got a list".to_string()),
            RuntimeValue::DicePool(_) => {
                Err("Expected a success pool but got a dice pool".to_string())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DieOutcome {
    None,    // 不参与成功/失败统计
    Success, // 成功
    Failure, // 失败
}

#[derive(Debug, Clone)]
pub struct DieDetail {
    pub result: i32,
    pub is_kept: bool,
    pub outcome: DieOutcome,
    pub is_rerolled: bool, // 是否导致了重掷
    pub is_exploded: bool, // 是否导致了爆炸
}

#[derive(Debug, Clone)]
pub enum DiceFace {
    Number(i32),
    Fudge,
    Coin,
}

// ==========================================
// 节点状态
// ==========================================
#[derive(Debug, Clone)]
pub enum NodeState {
    Unvisited, // 初始状态
    Computing, // 用于检测循环依赖 (虽然 HIR 树状结构不应该有环)
    Waiting,   // 关键状态：暂停中，等待外部结果
    Computed(RuntimeValue),
}

// ==========================================
// 动画请求 (The Signal)
// ==========================================
pub struct RuntimeRequest {
    pub node_id: NodeId, // 拿着这个 ID，前端跑完动画后还给我
    pub desc: String,    // "1d6", "3d20"
}
