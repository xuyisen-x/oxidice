use crate::types::eval_graph::*;
use crate::types::expr::CompareOp;
use crate::types::runtime_value::*;

pub struct ExecutionContext {
    graph: EvalGraph,                  // 代码 (只读)
    memory: Vec<NodeState>,            // 内存 (读写)
    pub requests: Vec<RuntimeRequest>, // 本轮需要外部骰子结果的请求列表
    pub remove_requests: Vec<RollId>,  // 本轮需要移除的外部骰子请求列表，主要用于动画
}

enum DiceFilterOp {
    KeepHigh,
    KeepLow,
    DropHigh,
    DropLow,
}

impl ExecutionContext {
    pub fn new(graph: EvalGraph) -> Self {
        let len = graph.nodes.len();
        Self {
            graph,
            memory: vec![NodeState::Waiting; len],
            requests: Vec::new(),
            remove_requests: Vec::new(),
        }
    }

    pub fn eval_node(&mut self, id: NodeId) -> Result<Option<RuntimeValue>, String> {
        let idx = id.to_index();
        // 首先先检查缓存
        match &self.memory[idx] {
            NodeState::Computed(v) => return Ok(Some(v.clone())),
            NodeState::Waiting | NodeState::Dynamic(_) => {} // 继续向下
        }
        // 获取节点
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
                    Some(RuntimeValue::Number((n1 / n2).floor()))
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
            EvalNode::DiceStandard(count_id, sides_id) => {
                let (count_id, sides_id) = (count_id.clone(), sides_id.clone());
                let count_val = self.get_number(count_id)?;
                let sides_val = self.get_number(sides_id)?;

                if let (Some(c), Some(s)) = (count_val, sides_val) {
                    let count = c as i32;
                    let sides = s as i32;
                    if sides <= 0 {
                        Some(RuntimeValue::DicePool(Box::new(DicePoolType {
                            total: 0,
                            face: DiceFace::Number(0),
                            details: Vec::new(),
                        })))
                    } else if count <= 0 {
                        Some(RuntimeValue::DicePool(Box::new(DicePoolType {
                            total: 0,
                            face: DiceFace::Number(sides),
                            details: Vec::new(),
                        })))
                    } else {
                        self.requests.push(RuntimeRequest {
                            node_id: id,
                            face: DiceFace::Number(sides),
                            count: count as u32,
                        });
                        None
                    }
                } else {
                    None
                }
            }
            EvalNode::DiceFudge(count_id) => {
                let count_val = self.get_number(*count_id)?;
                if let Some(c) = count_val {
                    let count = c as i32;
                    if count <= 0 {
                        Some(RuntimeValue::DicePool(Box::new(DicePoolType {
                            total: 0,
                            face: DiceFace::Fudge,
                            details: Vec::new(),
                        })))
                    } else {
                        self.requests.push(RuntimeRequest {
                            node_id: id,
                            face: DiceFace::Fudge,
                            count: count as u32,
                        });
                        None
                    }
                } else {
                    None
                }
            }
            EvalNode::DiceCoin(count_id) => {
                let count_val = self.get_number(*count_id)?;
                if let Some(c) = count_val {
                    let count = c as i32;
                    if count <= 0 {
                        Some(RuntimeValue::DicePool(Box::new(DicePoolType {
                            total: 0,
                            face: DiceFace::Coin,
                            details: Vec::new(),
                        })))
                    } else {
                        self.requests.push(RuntimeRequest {
                            node_id: id,
                            face: DiceFace::Coin,
                            count: count as u32,
                        });
                        None
                    }
                } else {
                    None
                }
            }
            EvalNode::DiceKeepHigh(dp_id, count_id) => {
                self.apply_dice_filter(*dp_id, *count_id, DiceFilterOp::KeepHigh)?
            }
            EvalNode::DiceKeepLow(dp_id, count_id) => {
                self.apply_dice_filter(*dp_id, *count_id, DiceFilterOp::KeepLow)?
            }
            EvalNode::DiceDropHigh(dp_id, count_id) => {
                self.apply_dice_filter(*dp_id, *count_id, DiceFilterOp::DropHigh)?
            }
            EvalNode::DiceDropLow(dp_id, count_id) => {
                self.apply_dice_filter(*dp_id, *count_id, DiceFilterOp::DropLow)?
            }
            EvalNode::DiceMin(dp_id, target_id) => {
                self.apply_dice_min_max(*dp_id, *target_id, false)?
            }
            EvalNode::DiceMax(dp_id, target_id) => {
                self.apply_dice_min_max(*dp_id, *target_id, true)?
            }
            EvalNode::DiceSubtractFailures(dp_id, mod_param_node) => {
                let (dp_id, mod_param_node) = (dp_id.clone(), mod_param_node.clone());
                let pool_ready = self.ensure_ready(dp_id)?;
                let mod_param_ready = self.ensure_ready(mod_param_node.value)?;
                if pool_ready && mod_param_ready {
                    let mut dice_pool = self.get_dice_pool(dp_id)?.unwrap();
                    let mod_param_value = self.get_number(mod_param_node.value)?.unwrap();
                    let mod_param_op = mod_param_node.operator;
                    let compare_func = get_compare_function(mod_param_op, mod_param_value);

                    for detail in dice_pool.details.iter_mut() {
                        if detail.is_kept && compare_func(detail.result as f64) {
                            detail.is_kept = false;
                            self.remove_requests.extend(detail.roll_id.iter());
                        }
                    }
                    dice_pool.renew_total();
                    Some(RuntimeValue::DicePool(Box::new(dice_pool)))
                } else {
                    None
                }
            }
            EvalNode::DiceCountSuccessesFromDicePool(dp_id, mod_param_node) => self
                .into_success_pool_from_dice_pool(
                    *dp_id,
                    mod_param_node.clone(),
                    DieOutcome::Success,
                )?,
            EvalNode::DiceDeductFailuresFromDicePool(dp_id, mod_param_node) => self
                .into_success_pool_from_dice_pool(
                    *dp_id,
                    mod_param_node.clone(),
                    DieOutcome::Failure,
                )?,
            EvalNode::DiceCountSuccesses(dp_id, mod_param_node) => {
                self.update_success_pool(*dp_id, mod_param_node.clone(), DieOutcome::Success)?
            }
            EvalNode::DiceDeductFailures(dp_id, mod_param_node) => {
                self.update_success_pool(*dp_id, mod_param_node.clone(), DieOutcome::Failure)?
            }
            EvalNode::DiceExplode(dp_id, mod_param_node, limit_node) => self.process_dynamic_op(
                id,
                *dp_id,
                mod_param_node.clone(),
                limit_node.clone(),
                |state| {
                    let mut new_rolls = Vec::new();
                    for (idx, value, roll_id) in state.pending_dice.iter() {
                        // 原本的骰子标记explode + 1
                        state.pool.details[*idx].exploded_times += 1;
                        // 将新的骰子加入details列表
                        let new_value = value.ok_or("Some value is missing".to_string())?;
                        state.pool.details.push(DieDetail {
                            result: new_value,
                            roll_history: vec![new_value],
                            roll_id: vec![roll_id.ok_or("Some value is missing")?],
                            is_kept: true,
                            outcome: DieOutcome::None,
                            is_rerolled: false,
                            exploded_times: 0,
                        });
                        // 记录新骰子的索引和结果
                        new_rolls.push((state.pool.details.len() - 1, new_value));
                    }
                    Ok((new_rolls, Vec::new()))
                },
            )?,
            EvalNode::DiceCompoundExplode(dp_id, mod_param_node, limit_node) => self
                .process_dynamic_op(
                    id,
                    *dp_id,
                    mod_param_node.clone(),
                    limit_node.clone(),
                    |state| {
                        let mut new_rolls = Vec::new();
                        for (idx, value, roll_id) in state.pending_dice.iter() {
                            // 原本的骰子标记explode + 1
                            state.pool.details[*idx].exploded_times += 1;
                            // 将新的骰子的值加入原本的骰子上，记录新值和新的roll_id
                            let new_value = value.ok_or("Some value is missing".to_string())?;
                            state.pool.details[*idx].result += new_value;
                            state.pool.details[*idx].roll_history.push(new_value);
                            state.pool.details[*idx]
                                .roll_id
                                .push(roll_id.ok_or("Some value is missing")?);
                            // 记录新骰子投掷的索引和结果
                            new_rolls.push((*idx, new_value));
                        }
                        Ok((new_rolls, Vec::new()))
                    },
                )?,
            EvalNode::DiceReroll(dp_id, mod_param_node, limit_node) => self.process_dynamic_op(
                id,
                *dp_id,
                Some(mod_param_node.clone()),
                limit_node.clone(),
                |state| {
                    let mut new_rolls = Vec::new();
                    let mut rolls_to_remove: Vec<RollId> = Vec::new();
                    for (idx, value, roll_id) in state.pending_dice.iter() {
                        // 原本的骰子标记为rerolled，并且不保留
                        state.pool.details[*idx].is_rerolled = true;
                        state.pool.details[*idx].is_kept = false;
                        rolls_to_remove.extend(state.pool.details[*idx].roll_id.iter());
                        // 将新的骰子加入details列表
                        let new_value = value.ok_or("Some value is missing".to_string())?;
                        state.pool.details.push(DieDetail {
                            result: new_value,
                            roll_history: vec![new_value],
                            roll_id: vec![roll_id.ok_or("Some value is missing")?],
                            is_kept: true,
                            outcome: DieOutcome::None,
                            is_rerolled: false,
                            exploded_times: 0,
                        });
                        // 记录新骰子的索引和结果
                        new_rolls.push((state.pool.details.len() - 1, new_value));
                    }
                    Ok((new_rolls, rolls_to_remove))
                },
            )?,
        };

        // 存储结果并返回
        match result {
            Some(v) => {
                let ret = v.clone();
                self.memory[idx] = NodeState::Computed(v);
                Ok(Some(ret))
            }
            None => Ok(None),
        }
    }

    pub fn process_runtime_responses(
        &mut self,
        responses: Vec<RuntimeResponse>,
    ) -> Result<(), String> {
        // 首先检查request和response长度是否匹配
        if responses.len() != self.requests.len() {
            return Err("Mismatched number of RuntimeResponses".to_string());
        }
        // 将Response写入内存
        for (request_idx, response) in responses.into_iter().enumerate() {
            let idx = self.requests[request_idx].node_id.to_index();
            let dice_result = response.results;
            // 分两种情况处理
            match &mut self.memory[idx] {
                NodeState::Dynamic(state) => {
                    if state.pending_dice.len() != dice_result.len() {
                        return Err("Mismatched dice result length".to_string());
                    }
                    // 更新pending_dice
                    for (i, (_, v, id)) in state.pending_dice.iter_mut().enumerate() {
                        *v = Some(dice_result[i].0);
                        *id = Some(dice_result[i].1);
                    }
                }
                NodeState::Waiting => {
                    // 查看一下对应的节点类型，必须是DiceStandard, DiceFudge, 或 DiceCoin
                    let node = &self.graph.nodes[idx];
                    match node {
                        EvalNode::DiceStandard(_, _)
                        | EvalNode::DiceFudge(_)
                        | EvalNode::DiceCoin(_) => {
                            let mut new_dice_pool = DicePoolType {
                                total: 0,
                                face: self.requests[request_idx].face.clone(),
                                details: dice_result
                                    .iter()
                                    .map(|(r, id)| DieDetail {
                                        result: *r,
                                        roll_history: vec![*r],
                                        roll_id: vec![*id],
                                        is_kept: true,
                                        outcome: DieOutcome::None,
                                        is_rerolled: false,
                                        exploded_times: 0,
                                    })
                                    .collect(),
                            };
                            new_dice_pool.renew_total();
                            self.memory[idx] = NodeState::Computed(RuntimeValue::DicePool(
                                Box::new(new_dice_pool),
                            ));
                        }
                        _ => {
                            return Err("RuntimeResponse received for non-dice node".to_string());
                        }
                    }
                }
                NodeState::Computed(_) => {
                    return Err("RuntimeResponse received for already computed node".to_string());
                }
            }
        }

        // 清空请求列表
        self.requests.clear();
        self.remove_requests.clear();
        Ok(())
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

    fn get_dice_pool(&mut self, id: NodeId) -> Result<Option<DicePoolType>, String> {
        match self.eval_node(id)? {
            Some(v) => {
                let dp = v.except_dice_pool()?;
                Ok(Some(dp.clone()))
            }
            None => Ok(None),
        }
    }

    fn get_success_pool(&mut self, id: NodeId) -> Result<Option<SuccessPoolType>, String> {
        match self.eval_node(id)? {
            Some(v) => {
                let sp = v.except_success_pool()?;
                Ok(Some(sp.clone()))
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

    fn apply_dice_filter(
        &mut self,
        pool_id: NodeId,
        count_id: NodeId,
        op: DiceFilterOp,
    ) -> Result<Option<RuntimeValue>, String> {
        let pool_ready = self.ensure_ready(pool_id)?;
        let count_val = self.get_number(count_id)?;

        if pool_ready && count_val.is_some() {
            let mut dice_pool = self.get_dice_pool(pool_id)?.unwrap();
            let raw_count = count_val.unwrap() as i32;
            let count = if raw_count < 0 { 0 } else { raw_count as usize };

            // 收集当前还是 "is_kept" 的骰子索引
            let mut active_indices: Vec<usize> = dice_pool
                .details
                .iter()
                .enumerate()
                .filter(|(_, d)| d.is_kept)
                .map(|(i, _)| i)
                .collect();

            // 如果没有活着的骰子，直接返回
            if active_indices.is_empty() {
                return Ok(Some(RuntimeValue::DicePool(Box::new(dice_pool))));
            }

            // 排序索引，High操作需要大数在前，Low操作需要小数在前
            active_indices.sort_by(|&a, &b| {
                let val_a = dice_pool.details[a].result;
                let val_b = dice_pool.details[b].result;
                match op {
                    DiceFilterOp::KeepHigh | DiceFilterOp::DropHigh => {
                        // 降序：大 -> 小
                        val_b.cmp(&val_a)
                    }
                    DiceFilterOp::KeepLow | DiceFilterOp::DropLow => {
                        // 升序：小 -> 大
                        val_a.cmp(&val_b)
                    }
                }
            });

            // 执行丢弃逻辑
            match op {
                DiceFilterOp::KeepHigh | DiceFilterOp::KeepLow => {
                    // Keep K: 保留前 K 个，丢弃剩余的
                    // 如果 K >= 数量，则全保留 (不做操作)
                    if count < active_indices.len() {
                        // 即使是 KeepHigh，因为我们已经降序排好了，所以前K个就是最大的
                        // 我们要丢弃的是从 K 开始的所有索引
                        for &idx_to_drop in &active_indices[count..] {
                            dice_pool.details[idx_to_drop].is_kept = false;
                            self.remove_requests
                                .extend(dice_pool.details[idx_to_drop].roll_id.iter());
                        }
                    }
                }
                DiceFilterOp::DropHigh | DiceFilterOp::DropLow => {
                    // Drop K: 丢弃前 K 个
                    if count >= active_indices.len() {
                        // 数量超标，全丢
                        for &idx in &active_indices {
                            dice_pool.details[idx].is_kept = false;
                            self.remove_requests
                                .extend(dice_pool.details[idx].roll_id.iter());
                        }
                    } else {
                        // 丢弃前 K 个
                        for &idx_to_drop in &active_indices[..count] {
                            dice_pool.details[idx_to_drop].is_kept = false;
                            self.remove_requests
                                .extend(dice_pool.details[idx_to_drop].roll_id.iter());
                        }
                    }
                }
            }

            // 重新计算 Total
            dice_pool.renew_total();

            Ok(Some(RuntimeValue::DicePool(Box::new(dice_pool))))
        } else {
            Ok(None)
        }
    }

    fn apply_dice_min_max(
        &mut self,
        pool_id: NodeId,
        target_id: NodeId,
        is_max: bool,
    ) -> Result<Option<RuntimeValue>, String> {
        let pool_ready = self.ensure_ready(pool_id)?;
        let target_val = self.get_number(target_id)?;

        if pool_ready && target_val.is_some() {
            let mut dice_pool = self.get_dice_pool(pool_id)?.unwrap();
            let target_val = target_val.unwrap() as i32;
            let mut changed = false;
            for detail in dice_pool.details.iter_mut() {
                if detail.is_kept {
                    if is_max && detail.result > target_val {
                        detail.result = target_val;
                        changed = true;
                    } else if !is_max && detail.result < target_val {
                        detail.result = target_val;
                        changed = true;
                    }
                }
            }
            if changed {
                dice_pool.renew_total();
            }
            Ok(Some(RuntimeValue::DicePool(Box::new(dice_pool))))
        } else {
            Ok(None)
        }
    }

    fn into_success_pool_from_dice_pool(
        &mut self,
        pool_id: NodeId,
        mod_param_node: ModParamNode,
        outcome: DieOutcome,
    ) -> Result<Option<RuntimeValue>, String> {
        let pool_ready = self.ensure_ready(pool_id)?;
        let mod_param_ready = self.ensure_ready(mod_param_node.value)?;
        if pool_ready && mod_param_ready {
            let dice_pool = self.get_dice_pool(pool_id)?.unwrap();
            let mod_param_value = self.get_number(mod_param_node.value)?.unwrap();
            let mod_param_op = mod_param_node.operator;
            let compare_func = get_compare_function(mod_param_op, mod_param_value);

            let mut success_pool = SuccessPoolType {
                success_count: 0,
                face: dice_pool.face,
                details: dice_pool.details,
            };

            for detail in success_pool.details.iter_mut() {
                if detail.is_kept {
                    if compare_func(detail.result as f64) {
                        detail.outcome = outcome.clone();
                    }
                }
            }
            success_pool.renew_success_count();
            Ok(Some(RuntimeValue::SuccessPool(Box::new(success_pool))))
        } else {
            Ok(None)
        }
    }

    fn update_success_pool(
        &mut self,
        pool_id: NodeId,
        mod_param_node: ModParamNode,
        outcome: DieOutcome,
    ) -> Result<Option<RuntimeValue>, String> {
        let pool_ready = self.ensure_ready(pool_id)?;
        let mod_param_ready = self.ensure_ready(mod_param_node.value)?;
        if pool_ready && mod_param_ready {
            let mut success_pool = self.get_success_pool(pool_id)?.unwrap();
            let mod_param_value = self.get_number(mod_param_node.value)?.unwrap();
            let mod_param_op = mod_param_node.operator;
            let compare_func = get_compare_function(mod_param_op, mod_param_value);

            for detail in success_pool.details.iter_mut() {
                if detail.is_kept {
                    if compare_func(detail.result as f64) {
                        detail.outcome = outcome.clone();
                    }
                }
            }
            success_pool.renew_success_count();
            Ok(Some(RuntimeValue::SuccessPool(Box::new(success_pool))))
        } else {
            Ok(None)
        }
    }

    fn process_dynamic_op<MergeFn>(
        &mut self,
        node_id: NodeId,
        dp_id: NodeId,
        mod_param_node: Option<ModParamNode>,
        limit_node: Option<LimitNode>,
        merge_fn: MergeFn,
    ) -> Result<Option<RuntimeValue>, String>
    where
        MergeFn: Fn(&mut DynamicState) -> Result<(Vec<(usize, i32)>, Vec<RollId>), String>,
    {
        let idx = node_id.to_index();

        // ====================================================
        // 阶段 1: 如果不是dynamic状态则进行初始化，同时拿到最新一次的投掷数据
        // ====================================================
        let is_init = match &self.memory[idx] {
            NodeState::Dynamic(_) => false,
            _ => {
                // 检查依赖项是否就绪
                let dp_ready = self.ensure_ready(dp_id.clone())?;
                let limit_count_ready = match &limit_node {
                    Some(ln) => match ln.limit_counts {
                        Some(id) => self.ensure_ready(id)?,
                        None => true,
                    },
                    None => true,
                };
                let limit_times_ready = match &limit_node {
                    Some(ln) => match ln.limit_times {
                        Some(id) => self.ensure_ready(id)?,
                        None => true,
                    },
                    None => true,
                };
                let mod_ready = match &mod_param_node {
                    Some(node) => self.ensure_ready(node.value.clone())?,
                    None => true,
                };

                if dp_ready && limit_count_ready && limit_times_ready && mod_ready {
                    let initial_pool = self.get_dice_pool(dp_id)?.unwrap();
                    let limit_count = match &limit_node {
                        Some(ln) => match ln.limit_counts {
                            Some(id) => {
                                let val = self.get_number(id)?.unwrap();
                                Some(val as i32)
                            }
                            None => None,
                        },
                        None => None,
                    };
                    let limit_times = match &limit_node {
                        Some(ln) => match ln.limit_times {
                            Some(id) => {
                                let val = self.get_number(id)?.unwrap();
                                Some(val as i32)
                            }
                            None => None,
                        },
                        None => None,
                    };
                    self.memory[idx] = NodeState::Dynamic(Box::new(DynamicState {
                        pool: initial_pool,
                        limit_times: limit_times,
                        limit_count: limit_count,
                        pending_dice: Vec::new(),
                    }));
                    true
                } else {
                    return Ok(None); // 依赖未就绪，继续等待
                }
            }
        };

        // ====================================================
        // 阶段 2: 准备环境 (构建比较函数)
        // ====================================================

        // 构建比较器
        let (operator, target_value) = match mod_param_node {
            Some(node) => {
                let val = self.get_number(node.value)?.unwrap();
                (node.operator, val)
            }
            None => {
                // 先获取当前的最大面值
                let max_face_val = if let NodeState::Dynamic(state) = &self.memory[idx] {
                    match state.pool.face {
                        DiceFace::Number(n) => n as f64,
                        DiceFace::Fudge => 1.0, // Fudge: -1, 0, 1
                        DiceFace::Coin => 1.0,  // Coin: 0, 1
                    }
                } else {
                    unreachable!()
                };
                (CompareOp::Equal, max_face_val)
            }
        };
        let compare_func = get_compare_function(operator, target_value);

        // ====================================================
        // 阶段 3: 状态机循环 (State Machine Loop)
        // ====================================================

        let mut request_to_send: Option<RuntimeRequest> = None;
        let mut final_result: Option<RuntimeValue> = None;

        if let NodeState::Dynamic(state) = &mut self.memory[idx] {
            // --- A: 合并阶段 ---
            // 并收集新的骰子结果
            let new_dice = if is_init {
                state
                    .pool
                    .details
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.is_kept)
                    .map(|(i, d)| (i, d.result))
                    .collect::<Vec<(usize, i32)>>()
            } else {
                let (new_dice, dice_to_remove) = merge_fn(state)?;
                self.remove_requests.extend(dice_to_remove.into_iter());
                new_dice
            };
            state.pending_dice.clear(); // 无论如何，清空旧的待处理骰子

            // --- B: 扫描阶段 ---
            // 是否达到次数限制，没有达到，则可以继续扫描
            if state.try_resume_times() {
                let new_rolls = new_dice
                    .into_iter()
                    .filter_map(|(i, result)| {
                        if compare_func(result as f64) && state.try_resume_count() {
                            // 这个骰子符合条件，并且次数限制允许，加入新请求列表
                            Some(i)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<usize>>();

                // 如果不为空，准备新的接受对象，并准备请求
                if !new_rolls.is_empty() {
                    state.pending_dice = new_rolls.iter().map(|i| (*i, None, None)).collect();

                    // 构造请求
                    let count = new_rolls.len() as i32;
                    request_to_send = Some(RuntimeRequest {
                        node_id,
                        face: state.pool.face.clone(),
                        count: count as u32,
                    });
                }
            }

            // --- C: 结算阶段 ---
            // 没有请求要发，且没有待处理的骰子，说明结束了
            if request_to_send.is_none() && state.pending_dice.is_empty() {
                state.pool.renew_total();
                final_result = Some(RuntimeValue::DicePool(Box::new(state.pool.clone())));
            }
        }

        // ====================================================
        // 阶段 4: 执行副作用
        // ====================================================

        if let Some(req) = request_to_send {
            self.requests.push(req);
            return Ok(None);
        }

        if let Some(res) = final_result {
            self.memory[idx] = NodeState::Computed(res.clone());
            Ok(Some(res))
        } else {
            unreachable!()
        }
    }

    pub fn get_root_id(&self) -> NodeId {
        self.graph.root.clone()
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
