use super::render_result::render_result;
use super::runtime_engine::ExecutionContext;
use crate::types::output_node::OutputNode;
use crate::types::runtime_value::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

fn generate_response(request: &RuntimeRequest, counter: &mut u32) -> RuntimeResponse {
    use rand::Rng;
    let mut rng = rand::rng();
    let range = match request.face {
        DiceFace::Number(n) => 1..=n, // 这里内部保证n不会小于等于0，至少为1
        DiceFace::Coin => 0..=1,
        DiceFace::Fudge => -1..=1,
    };
    let mut results = Vec::new();
    for _ in 0..request.count {
        let roll_result = rng.random_range(range.clone());
        results.push((roll_result, RollId(*counter)));
        *counter += 1;
    }

    RuntimeResponse { results: results }
}

enum DiceRollerWithoutAnimationState {
    Error(String),                            // 运行时出现错误
    Done(OutputNode),                         // 运行完成
    WaitingForResponses(Vec<RuntimeRequest>), // 等待外部骰子结果
    WaitingForEvaluation,                     // 等待继续评估
}

pub struct DiceRollerWithoutAnimation {
    context: ExecutionContext,
    recursion_limit: u32,
    dice_count_limit: u32,
    state: DiceRollerWithoutAnimationState,
}

impl DiceRollerWithoutAnimation {
    pub fn new(
        dice_expr: String,
        recursion_limit: u32,
        dice_count_limit: u32,
    ) -> Result<Self, String> {
        use super::grammar::parse_dice;
        use crate::compiler::compile_hir_to_eval_graph;
        use crate::lower::lower_expr;
        use crate::optimizer::constant_fold::constant_fold_hir;
        let ast = parse_dice(dice_expr.as_str()).map_err(|_| "parse error".to_string())?;
        let hir = lower_expr(ast)?;
        let hir = constant_fold_hir(hir)?;
        let context = ExecutionContext::new(compile_hir_to_eval_graph(hir));
        Ok(DiceRollerWithoutAnimation {
            context,
            recursion_limit,
            dice_count_limit,
            state: DiceRollerWithoutAnimationState::WaitingForEvaluation,
        })
    }

    pub fn evaluation(&mut self) -> Result<(), String> {
        if !matches!(
            self.state,
            DiceRollerWithoutAnimationState::WaitingForEvaluation
        ) {
            return Err("Cannot evaluate: not in WaitingForEvaluation state".to_string());
        }

        match self.context.eval_node(self.context.get_root_id()) {
            Ok(Some(_)) => {
                let output_node =
                    render_result(self.context.get_graph(), self.context.get_memory());
                self.state = DiceRollerWithoutAnimationState::Done(output_node);
            }
            Ok(None) => {
                // 先检查递归计数是否达到上限
                if self.recursion_limit <= 1 {
                    self.state = DiceRollerWithoutAnimationState::Error(
                        "Recursion limit exceeded".to_string(),
                    );
                    return Ok(());
                }
                self.recursion_limit -= 1;
                // 然后检查骰子计数
                let dice_count = self.context.requests.iter().map(|r| r.count).sum::<u32>();
                if self.dice_count_limit < dice_count {
                    self.state = DiceRollerWithoutAnimationState::Error(
                        "Dice count limit exceeded".to_string(),
                    );
                    return Ok(());
                }
                self.dice_count_limit -= dice_count;
                // 如果都不满足，进入下一个状态
                self.state = DiceRollerWithoutAnimationState::WaitingForResponses(
                    self.context.requests.clone(),
                )
            }
            Err(e) => {
                self.state = DiceRollerWithoutAnimationState::Error(e);
            }
        }
        Ok(())
    }

    pub fn set_responses(&mut self, responses: Vec<RuntimeResponse>) -> Result<(), String> {
        match &self.state {
            DiceRollerWithoutAnimationState::WaitingForResponses(requests) => {
                if responses.len() != requests.len() {
                    return Err("Can not set responses: error responses count".to_string());
                }
                match self.context.process_runtime_responses(responses) {
                    Err(e) => self.state = DiceRollerWithoutAnimationState::Error(e),
                    Ok(()) => self.state = DiceRollerWithoutAnimationState::WaitingForEvaluation,
                }
                Ok(())
            }
            _ => Err("Can not set responses: not in WaitingForResponses state".to_string()),
        }
    }

    pub fn try_get_results(&self) -> Result<Option<OutputNode>, String> {
        match &self.state {
            DiceRollerWithoutAnimationState::Error(e) => Err(e.clone()),
            DiceRollerWithoutAnimationState::Done(v) => Ok(Some(v.clone())),
            _ => Ok(None),
        }
    }
}

#[wasm_bindgen(js_name = rollWithoutAnimation)]
pub fn roll_without_animation(
    dice_expr: String,
    recursion_limit: u32,
    dice_count_limit: u32,
) -> Result<OutputNode, String> {
    let mut dice_roller =
        DiceRollerWithoutAnimation::new(dice_expr, recursion_limit, dice_count_limit)?;
    let mut counter: u32 = 0;
    while dice_roller.try_get_results()?.is_none() {
        dice_roller.evaluation()?;
        if let DiceRollerWithoutAnimationState::WaitingForResponses(requests) = &dice_roller.state {
            // 模拟骰子结果，这里简单地将每个请求都返回1
            let responses: Vec<RuntimeResponse> = requests
                .into_iter()
                .map(|req| generate_response(req, &mut counter))
                .collect();
            dice_roller.set_responses(responses)?;
        }
    }
    Ok(dice_roller.try_get_results()?.unwrap())
}

// ==========================================
// 用于配合 @3d-dice/dice-box 使用的类型
// 对应项目地址：https://github.com/3d-dice/dice-box
// ==========================================

enum DiceRollerWithDiceBoxState {
    Error(String),        // 运行时出现错误
    Done(OutputNode),     // 运行完成
    WaitingForResponses,  // 等待外部骰子结果
    WaitingForEvaluation, // 等待继续评估
}

#[wasm_bindgen]
#[derive(Tsify)]
pub struct DiceRollerWithDiceBox {
    context: ExecutionContext,
    recursion_limit: u32,
    dice_count_limit: u32,
    state: DiceRollerWithDiceBoxState,
    id_map: HashMap<RollId, DiceBoxId>,
    _roll_id_counter: u32,
}

#[derive(Clone, Copy, Serialize, Tsify)]
#[tsify(into_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct DiceBoxRequest {
    pub idx: usize,
    pub face: u32,
    pub count: u32,
}

#[derive(Clone, Copy, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct DiceBoxId {
    pub group_id: f64,
    pub roll_id: f64,
}

#[derive(Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
#[serde(rename_all = "camelCase")]
pub struct DiceBoxResponse {
    pub idx: usize,
    pub results: Vec<DiceBoxId>,
    pub values: Vec<f64>,
}

#[wasm_bindgen]
impl DiceRollerWithDiceBox {
    #[wasm_bindgen(constructor)]
    pub fn new(
        dice_expr: String,
        recursion_limit: u32,
        dice_count_limit: u32,
    ) -> Result<Self, String> {
        use super::grammar::parse_dice;
        use crate::compiler::compile_hir_to_eval_graph;
        use crate::lower::lower_expr;
        use crate::optimizer::constant_fold::constant_fold_hir;
        let ast = parse_dice(dice_expr.as_str()).map_err(|_| "parse error".to_string())?;
        let hir = lower_expr(ast)?;
        let hir = constant_fold_hir(hir)?;
        let context = ExecutionContext::new(compile_hir_to_eval_graph(hir));
        Ok(DiceRollerWithDiceBox {
            context,
            recursion_limit,
            dice_count_limit,
            state: DiceRollerWithDiceBoxState::WaitingForEvaluation,
            id_map: HashMap::new(),
            _roll_id_counter: 0,
        })
    }

    #[wasm_bindgen(js_name = evaluation)]
    pub fn evaluation(&mut self) -> Result<(), String> {
        if !matches!(self.state, DiceRollerWithDiceBoxState::WaitingForEvaluation) {
            return Err("Cannot evaluate: not in WaitingForEvaluation state".to_string());
        }

        match self.context.eval_node(self.context.get_root_id()) {
            Ok(Some(_)) => {
                let output_node =
                    render_result(self.context.get_graph(), self.context.get_memory());
                self.state = DiceRollerWithDiceBoxState::Done(output_node);
            }
            Ok(None) => {
                // 先检查递归计数是否达到上限
                if self.recursion_limit <= 1 {
                    self.state =
                        DiceRollerWithDiceBoxState::Error("Recursion limit exceeded".to_string());
                    return Ok(());
                }
                self.recursion_limit -= 1;
                // 然后检查骰子计数
                let dice_count = self.context.requests.iter().map(|r| r.count).sum::<u32>();
                if self.dice_count_limit < dice_count {
                    self.state =
                        DiceRollerWithDiceBoxState::Error("Dice count limit exceeded".to_string());
                    return Ok(());
                }
                self.dice_count_limit -= dice_count;
                // 如果都不满足，进入下一个状态
                self.state = DiceRollerWithDiceBoxState::WaitingForResponses;
            }
            Err(e) => {
                self.state = DiceRollerWithDiceBoxState::Error(e);
            }
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = getRequests)]
    pub fn get_requests(&self) -> Result<Vec<DiceBoxRequest>, String> {
        match &self.state {
            DiceRollerWithDiceBoxState::WaitingForResponses => {
                let requests = self.context.requests.clone();
                let requests = requests
                    .into_iter()
                    .enumerate()
                    .filter_map(|(i, r)| match r.face {
                        DiceFace::Number(n)
                            if n == 4
                                || n == 6
                                || n == 8
                                || n == 10
                                || n == 12
                                || n == 20
                                || n == 100 =>
                        {
                            Some(DiceBoxRequest {
                                idx: i,
                                face: n as u32,
                                count: r.count,
                            })
                        }
                        _ => None,
                    })
                    .collect();
                Ok(requests)
            }
            _ => Err("Can not get requests: not in WaitingForResponses state".to_string()),
        }
    }

    #[wasm_bindgen(js_name = setResponses)]
    pub fn set_responses(&mut self, responses: Vec<DiceBoxResponse>) -> Result<(), String> {
        match &self.state {
            DiceRollerWithDiceBoxState::WaitingForResponses => {
                let runtime_request = &self.context.requests;
                let mut runtime_responses: Vec<Option<RuntimeResponse>> =
                    Vec::with_capacity(runtime_request.len());
                // 初始化为空
                for _ in 0..runtime_request.len() {
                    runtime_responses.push(None);
                }
                // 首先，将responses中的所有结果填入对应的位置
                for resp in responses.into_iter() {
                    if resp.idx >= runtime_request.len() {
                        return Err("Can not set responses: invalid response idx".to_string());
                    }
                    if resp.values.len() != resp.results.len() {
                        return Err(
                            "Can not set responses: values and results length mismatch".to_string()
                        );
                    }
                    let mut results: Vec<(i32, RollId)> = Vec::with_capacity(resp.values.len());
                    for (i, db_id) in resp.results.into_iter().enumerate() {
                        let roll_id = RollId(self._roll_id_counter);
                        self._roll_id_counter += 1;
                        results.push((resp.values[i] as i32, roll_id));
                        // 记录id映射
                        self.id_map.insert(
                            roll_id,
                            DiceBoxId {
                                group_id: db_id.group_id,
                                roll_id: db_id.roll_id,
                            },
                        );
                    }
                    runtime_responses[resp.idx] = Some(RuntimeResponse { results });
                }
                // 然后，检查所有没有被填入的请求，由内置的随机数生成器生成结果
                for (i, req) in runtime_request.iter().enumerate() {
                    if runtime_responses[i].is_none() {
                        runtime_responses[i] =
                            Some(generate_response(req, &mut self._roll_id_counter));
                    }
                }
                // 最后，收集所有响应，传递给引擎
                let final_responses: Vec<RuntimeResponse> =
                    runtime_responses.into_iter().map(|r| r.unwrap()).collect();
                match self.context.process_runtime_responses(final_responses) {
                    Err(e) => self.state = DiceRollerWithDiceBoxState::Error(e),
                    Ok(()) => self.state = DiceRollerWithDiceBoxState::WaitingForEvaluation,
                }
                Ok(())
            }
            _ => Err("Can not set responses: not in WaitingForResponses state".to_string()),
        }
    }

    #[wasm_bindgen(js_name = removeRequests)]
    pub fn remove_requests(&mut self) -> Vec<DiceBoxId> {
        self.context
            .remove_requests
            .iter()
            .filter_map(|id| match self.id_map.get(id) {
                Some(db_id) => Some(*db_id),
                None => None,
            })
            .collect::<Vec<DiceBoxId>>()
    }

    #[wasm_bindgen(js_name = tryGetResults)]
    pub fn try_get_results(&self) -> Result<Option<OutputNode>, String> {
        match &self.state {
            DiceRollerWithDiceBoxState::Error(e) => Err(e.clone()),
            DiceRollerWithDiceBoxState::Done(node) => Ok(Some(node.clone())),
            _ => Ok(None),
        }
    }
}
