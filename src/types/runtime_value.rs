use crate::types::eval_graph::*;

// ==========================================
// 运行时值
// ==========================================
#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Number(f64),
    List(Vec<f64>),
    DicePool(Box<DicePoolType>),
    SuccessPool(Box<SuccessPoolType>),
}

#[derive(Debug, Clone)]
pub struct DicePoolType {
    pub total: i32,
    pub face: DiceFace,
    pub details: Vec<DieDetail>,
}

impl DicePoolType {
    pub fn renew_total(&mut self) {
        self.total = self
            .details
            .iter()
            .filter(|d| d.is_kept)
            .map(|d| d.result)
            .sum();
    }
}

#[derive(Debug, Clone)]
pub struct SuccessPoolType {
    pub success_count: i32,
    pub face: DiceFace,
    pub details: Vec<DieDetail>,
}

impl SuccessPoolType {
    pub fn renew_success_count(&mut self) {
        self.success_count = self
            .details
            .iter()
            .filter(|d| d.is_kept)
            .map(|d| match d.outcome {
                DieOutcome::Success => 1,
                DieOutcome::Failure => -1,
                DieOutcome::None => 0,
            })
            .sum();
    }
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RollId(pub u32); // A simple wrapper for dice identifiers

// impl RollId {
//     pub fn get_id(&self) -> u32 {
//         self.0
//     }
// }

#[derive(Debug, Clone)]
pub struct DieDetail {
    pub result: i32,
    pub roll_id: Vec<RollId>, // 该结果对应的所有投掷 ID（用于追踪聚合爆炸等情况）
    pub roll_history: Vec<i32>, // 对于聚合爆炸，会记录所有的投掷结果
    pub is_kept: bool,
    pub outcome: DieOutcome,
    pub is_rerolled: bool,   // 是否导致了重掷
    pub exploded_times: i32, // 该骰子爆炸了多少次，用于compound骰子显示
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
    Waiting,                    // 由于某种原因，当前节点无法计算
    Computed(RuntimeValue),     // 已经计算完成，存储结果
    Dynamic(Box<DynamicState>), // 需要动态计算的节点，存储动态状态
}

#[derive(Debug, Clone)]
pub struct DynamicState {
    pub pool: DicePoolType,
    pub limit_times: Option<i32>,
    pub limit_count: Option<i32>,
    // 记录哪些骰子索引触发了这次操作 (用于Compound/Reroll定位)，并存储对应的掷骰结果
    pub pending_dice: Vec<(usize, Option<i32>, Option<RollId>)>,
}

impl DynamicState {
    pub fn try_resume_times(&mut self) -> bool {
        match self.limit_times {
            Some(times) if times > 0 => {
                self.limit_times = Some(times - 1);
                true
            }
            Some(_) => false,
            None => true,
        }
    }
    pub fn try_resume_count(&mut self) -> bool {
        match self.limit_count {
            Some(count) if count > 0 => {
                self.limit_count = Some(count - 1);
                true
            }
            Some(_) => false,
            None => true,
        }
    }
}

// ==========================================
// 投掷请求
// ==========================================
#[derive(Debug, Clone)]
pub struct RuntimeRequest {
    pub node_id: NodeId,
    pub face: DiceFace,
    pub count: u32,
}

pub struct RuntimeResponse {
    pub results: Vec<(i32, RollId)>, // 每个骰子的结果和对应的投掷 ID
}
