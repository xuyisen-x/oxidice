use crate::types::eval_graph::*;
use crate::types::expr::CompareOp;
use crate::types::runtime_value::*;

pub struct ExecutionContext {
    graph: EvalGraph,       // 代码 (只读)
    memory: Vec<NodeState>, // 内存 (读写)
    pub requests: Vec<RuntimeRequest>,
}

impl ExecutionContext {
    pub fn new(graph: EvalGraph) -> Self {
        let len = graph.nodes.len();
        Self {
            graph,
            memory: vec![NodeState::Unvisited; len],
            requests: Vec::new(),
        }
    }

    pub fn eval_node(&mut self, id: NodeId) -> Result<Option<RuntimeValue>, String> {
        let idx = id.to_index();
        // 首先先检查缓存
        match &self.memory[idx] {
            NodeState::Computed(v) => return Ok(Some(v.clone())),
            NodeState::Waiting => return Ok(None), // 正在等，无法继续
            NodeState::Computing => return Err("Detected cycle in EvalGraph!".to_string()), // unreachable
            NodeState::Unvisited => {} // 继续向下
        }

        // 标记为正在计算
        self.memory[idx] = NodeState::Computing;
        let node = &self.graph.nodes[idx];

        // 根据指令类型分发进行计算
        let result = match node {
            EvalNode::Constant(v) => Some(RuntimeValue::Number(*v)),
            EvalNode::ListConstruct(elements) => {
                let mut list = Vec::new();
                let mut is_waiting = false;
                let elem_ids: Vec<_> = elements.iter().copied().collect();
                for elem_id in elem_ids {
                    match self.eval_node(elem_id)? {
                        Some(number) => {
                            if !is_waiting {
                                list.push(number.except_number()?);
                            }
                        }
                        None => is_waiting = true,
                    }
                }
                if is_waiting {
                    // 只要有一个元素没有准备好，就返回 None
                    None
                } else {
                    Some(RuntimeValue::List(list))
                }
            }
            EvalNode::NumNegate(node) => match self.eval_node(*node)? {
                Some(v) => Some(RuntimeValue::Number(-v.except_number()?)),
                None => None,
            },
            EvalNode::NumAdd(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    Some(RuntimeValue::Number(n1 + n2))
                } else {
                    None
                }
            }
            EvalNode::NumSubtract(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    Some(RuntimeValue::Number(n1 - n2))
                } else {
                    None
                }
            }
            EvalNode::NumMultiply(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    Some(RuntimeValue::Number(n1 * n2))
                } else {
                    None
                }
            }
            EvalNode::NumDivide(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    if n2 == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    Some(RuntimeValue::Number(n1 / n2))
                } else {
                    None
                }
            }
            EvalNode::NumIntDivide(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    if n2 == 0.0 {
                        return Err("Integer division by zero".to_string());
                    }
                    Some(RuntimeValue::Number((n1 / n2).trunc()))
                } else {
                    None
                }
            }
            EvalNode::NumModulo(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let (v1, v2) = (self.get_number(idx1)?, self.get_number(idx2)?);
                if let (Some(n1), Some(n2)) = (v1, v2) {
                    if n2 == 0.0 {
                        return Err("Modulo by zero".to_string());
                    }
                    Some(RuntimeValue::Number(n1 % n2))
                } else {
                    None
                }
            }
            EvalNode::Concat(idx1, idx2) => {
                let (idx1, idx2) = (idx1.clone(), idx2.clone());
                let ready1 = self.ensure_ready(idx1)?;
                let ready2 = self.ensure_ready(idx2)?;
                if ready1 && ready2 {
                    // 这里unwrap是安全的，因为ensure_ready已经确认了状态
                    let mut list1 = self.get_list(idx1)?.unwrap();
                    let list2 = self.get_list(idx2)?.unwrap();
                    list1.extend(list2);
                    Some(RuntimeValue::List(list1))
                } else {
                    None
                }
            }
            EvalNode::ListAdd(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| Ok(x + n))?
            }
            EvalNode::ListMultiply(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| Ok(x * n))?
            }
            EvalNode::ListSubtract(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| Ok(x - n))?
            }
            EvalNode::ListDivide(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if n == 0.0 {
                        Err("Division by zero in ListDivide".to_string())
                    } else {
                        Ok(x / n)
                    }
                })?
            }
            EvalNode::ListIntDivide(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if n == 0.0 {
                        Err("Integer division by zero in ListIntDivide".to_string())
                    } else {
                        Ok((x / n).floor())
                    }
                })?
            }
            EvalNode::ListModulo(list_idx, number_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if n == 0.0 {
                        Err("Modulo by zero in ListModulo".to_string())
                    } else {
                        Ok(x % n)
                    }
                })?
            }
            EvalNode::ListSubtractReverse(number_idx, list_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| Ok(n - x))?
            }
            EvalNode::ListDivideReverse(number_idx, list_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if x == 0.0 {
                        Err("Division by zero in ListDivideReverse".to_string())
                    } else {
                        Ok(n / x)
                    }
                })?
            }
            EvalNode::ListIntDivideReverse(number_idx, list_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if x == 0.0 {
                        Err("Integer division by zero in ListIntDivideReverse".to_string())
                    } else {
                        Ok((n / x).floor())
                    }
                })?
            }
            EvalNode::ListModuloReverse(number_idx, list_idx) => {
                self.apply_list_scalar_op(*list_idx, *number_idx, |x, n| {
                    if x == 0.0 {
                        Err("Modulo by zero in ListModuloReverse".to_string())
                    } else {
                        Ok(n % x)
                    }
                })?
            }
            EvalNode::NumFloor(node) => match self.eval_node(*node)? {
                Some(v) => Some(RuntimeValue::Number(v.except_number()?.floor())),
                None => None,
            },
            EvalNode::NumCeil(node) => match self.eval_node(*node)? {
                Some(v) => Some(RuntimeValue::Number(v.except_number()?.ceil())),
                None => None,
            },
            EvalNode::NumRound(node) => match self.eval_node(*node)? {
                Some(v) => Some(RuntimeValue::Number(v.except_number()?.round())),
                None => None,
            },
            EvalNode::NumAbs(node) => match self.eval_node(*node)? {
                Some(v) => Some(RuntimeValue::Number(v.except_number()?.abs())),
                None => None,
            },
            EvalNode::NumMax(node) => {
                let list = self.get_list(*node)?;
                if let Some(list) = list {
                    if list.is_empty() {
                        return Err("NumMax called on empty list".to_string());
                    }
                    let max_value = list.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                    Some(RuntimeValue::Number(max_value))
                } else {
                    None
                }
            }
            EvalNode::NumMin(node) => {
                let list = self.get_list(*node)?;
                if let Some(list) = list {
                    if list.is_empty() {
                        return Err("NumMin called on empty list".to_string());
                    }
                    let min_value = list.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                    Some(RuntimeValue::Number(min_value))
                } else {
                    None
                }
            }
            EvalNode::NumSum(node) => match self.get_list(*node)? {
                Some(list) => {
                    let sum_value: f64 = list.iter().sum();
                    Some(RuntimeValue::Number(sum_value))
                }
                None => None,
            },
            EvalNode::NumAvg(node) => match self.get_list(*node)? {
                Some(list) => {
                    let avg_value = if list.is_empty() {
                        0.0
                    } else {
                        list.iter().sum::<f64>() / (list.len() as f64)
                    };
                    Some(RuntimeValue::Number(avg_value))
                }
                None => None,
            },
            EvalNode::NumLen(node) => match self.get_list(*node)? {
                Some(list) => Some(RuntimeValue::Number(list.len() as f64)),
                None => None,
            },
            EvalNode::ListFloor(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let floored: Vec<f64> = list.iter().map(|&x| x.floor()).collect();
                    Some(RuntimeValue::List(floored))
                }
                None => None,
            },
            EvalNode::ListCeil(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let ceiled: Vec<f64> = list.iter().map(|&x| x.ceil()).collect();
                    Some(RuntimeValue::List(ceiled))
                }
                None => None,
            },
            EvalNode::ListRound(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let rounded: Vec<f64> = list.iter().map(|&x| x.round()).collect();
                    Some(RuntimeValue::List(rounded))
                }
                None => None,
            },
            EvalNode::ListAbs(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let absed: Vec<f64> = list.iter().map(|&x| x.abs()).collect();
                    Some(RuntimeValue::List(absed))
                }
                None => None,
            },
            EvalNode::ListMax(list_idx, number_idx) => {
                let (list_idx, number_idx) = (list_idx.clone(), number_idx.clone());
                let list_ready = self.ensure_ready(list_idx)?;
                let number = self.get_number(number_idx)?;
                if list_ready && number.is_some() {
                    let list = self.get_list(list_idx)?.unwrap();
                    let num_val = number.unwrap();
                    let filtered = keep_elements_preserve_order(list, num_val, true);
                    Some(RuntimeValue::List(filtered))
                } else {
                    None
                }
            }
            EvalNode::ListMin(list_idx, number_idx) => {
                let (list_idx, number_idx) = (list_idx.clone(), number_idx.clone());
                let list_ready = self.ensure_ready(list_idx)?;
                let number = self.get_number(number_idx)?;
                if list_ready && number.is_some() {
                    let list = self.get_list(list_idx)?.unwrap();
                    let num_val = number.unwrap();
                    let filtered = keep_elements_preserve_order(list, num_val, false);
                    Some(RuntimeValue::List(filtered))
                } else {
                    None
                }
            }
            EvalNode::ListSort(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let mut sorted = list.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    Some(RuntimeValue::List(sorted))
                }
                None => None,
            },
            EvalNode::ListSortDesc(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let list = v.except_list()?;
                    let mut sorted = list.clone();
                    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                    Some(RuntimeValue::List(sorted))
                }
                None => None,
            },
            EvalNode::ListToListFromDicePool(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let dice_pool = v.except_dice_pool()?;
                    let list: Vec<f64> = dice_pool
                        .details
                        .iter()
                        .filter(|d| d.is_kept == true)
                        .map(|d| d.result as f64)
                        .collect();
                    Some(RuntimeValue::List(list))
                }
                None => None,
            },
            EvalNode::ListToListFromSuccessPool(node) => match self.eval_node(*node)? {
                Some(v) => {
                    let success_pool = v.except_success_pool()?;
                    let list: Vec<f64> = success_pool
                        .details
                        .iter()
                        .filter(|d| d.is_kept == true)
                        .map(|d| match d.outcome {
                            DieOutcome::Success => 1.0,
                            DieOutcome::Failure => -1.0,
                            DieOutcome::None => 0.0,
                        })
                        .collect();
                    Some(RuntimeValue::List(list))
                }
                None => None,
            },
            EvalNode::ListFilter(list_idx, mod_param_node) => {
                let (list_idx, mod_param_node) = (list_idx.clone(), mod_param_node.clone());
                let list_ready = self.ensure_ready(list_idx)?;
                let mod_param_ready = self.ensure_ready(mod_param_node.value)?;
                if list_ready && mod_param_ready {
                    let list = self.get_list(list_idx)?.unwrap();
                    let mod_param_value = self.get_number(mod_param_node.value)?.unwrap();
                    let mod_param_op = mod_param_node.operator;
                    let filter_func = get_compare_function(mod_param_op, mod_param_value);
                    let filtered: Vec<f64> = list.into_iter().filter(|x| filter_func(*x)).collect();
                    Some(RuntimeValue::List(filtered))
                } else {
                    None
                }
            }
            _ => unimplemented!("EvalNode variant not implemented yet"),
        };

        // 存储结果并返回
        match result {
            Some(v) => {
                self.memory[idx] = NodeState::Computed(v);
                Ok(self.get_value(id).map(|v| v.clone()))
            }
            None => {
                self.memory[idx] = NodeState::Waiting;
                Ok(None)
            }
        }
    }

    fn get_value(&self, id: NodeId) -> Option<&RuntimeValue> {
        match &self.memory[id.0 as usize] {
            NodeState::Computed(v) => Some(v),
            _ => None,
        }
    }

    fn get_number(&mut self, id: NodeId) -> Result<Option<f64>, String> {
        match self.eval_node(id)? {
            Some(v) => Ok(Some(v.except_number()?)),
            None => Ok(None),
        }
    }

    fn get_list(&mut self, id: NodeId) -> Result<Option<Vec<f64>>, String> {
        match self.eval_node(id)? {
            Some(v) => {
                let list = v.except_list()?;
                Ok(Some(list.clone()))
            }
            None => Ok(None),
        }
    }

    fn ensure_ready(&mut self, id: NodeId) -> Result<bool, String> {
        let idx = id.to_index();
        if let NodeState::Computed(_) = &self.memory[idx] {
            return Ok(true);
        }
        Ok(self.eval_node(id)?.is_some())
    }

    fn apply_list_scalar_op<F>(
        &mut self,
        list_id: NodeId,
        number_id: NodeId,
        op: F,
    ) -> Result<Option<RuntimeValue>, String>
    where
        F: Fn(f64, f64) -> Result<f64, String>,
    {
        let list_ready = self.ensure_ready(list_id)?;
        let number = self.get_number(number_id)?;

        if list_ready && number.is_some() {
            let list = self.get_list(list_id)?.unwrap();
            let num_val = number.unwrap();
            let result_list: Result<Vec<f64>, String> =
                list.iter().map(|&x| op(x, num_val)).collect();
            Ok(Some(RuntimeValue::List(result_list?)))
        } else {
            Ok(None)
        }
    }
}

fn keep_elements_preserve_order(values: Vec<f64>, raw_count: f64, keep_highest: bool) -> Vec<f64> {
    if raw_count < 0.0 {
        return Vec::new();
    }

    let count = raw_count as usize;

    if count >= values.len() {
        return values;
    }

    let mut indexed_values: Vec<(usize, f64)> = values.into_iter().enumerate().collect();

    indexed_values.sort_by(|a, b| {
        let (val_a, val_b) = (a.1, b.1);
        if keep_highest {
            // true (Max): 降序 (b compare a)
            val_b
                .partial_cmp(&val_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        } else {
            // false (Min): 升序 (a compare b)
            val_a
                .partial_cmp(&val_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut top_k: Vec<(usize, f64)> = indexed_values.into_iter().take(count).collect();

    top_k.sort_by_key(|(index, _)| *index);

    let result = top_k.into_iter().map(|(_, val)| val).collect();

    result
}

fn get_compare_function(op: CompareOp, number: f64) -> impl Fn(f64) -> bool {
    move |x: f64| match op {
        CompareOp::Greater => x > number,
        CompareOp::GreaterEqual => x >= number,
        CompareOp::Less => x < number,
        CompareOp::LessEqual => x <= number,
        CompareOp::Equal => (x - number).abs() < f64::EPSILON,
        CompareOp::NotEqual => (x - number).abs() >= f64::EPSILON,
    }
}
