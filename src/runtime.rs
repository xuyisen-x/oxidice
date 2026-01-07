use super::render_result::render_result;
use super::runtime_engine::ExecutionContext;
use crate::types::output_node::OutputNode;
use crate::types::runtime_value::*;
use wasm_bindgen::prelude::*;

enum DiceRollerState {
    Error(String),                            // 运行时出现错误
    Done(OutputNode),                         // 运行完成
    WaitingForResponses(Vec<RuntimeRequest>), // 等待外部骰子结果
    WaitingForEvaluation,                     // 等待继续评估
}

pub struct DiceRollerWithoutAnimation {
    context: ExecutionContext,
    recursion_limit: u32,
    dice_count_limit: u32,
    state: DiceRollerState,
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
        let ast = parse_dice(dice_expr.as_str())?;
        let hir = lower_expr(ast)?;
        let hir = constant_fold_hir(hir)?;
        let context = ExecutionContext::new(compile_hir_to_eval_graph(hir));
        Ok(DiceRollerWithoutAnimation {
            context,
            recursion_limit,
            dice_count_limit,
            state: DiceRollerState::WaitingForEvaluation,
        })
    }

    pub fn evaluation(&mut self) -> Result<(), String> {
        if !matches!(self.state, DiceRollerState::WaitingForEvaluation) {
            return Err("Cannot evaluate: not in WaitingForEvaluation state".to_string());
        }

        match self.context.eval_node(self.context.get_root_id()) {
            Ok(Some(_)) => {
                let output_node =
                    render_result(self.context.get_graph(), self.context.get_memory());
                self.state = DiceRollerState::Done(output_node);
            }
            Ok(None) => {
                // 先检查递归计数是否达到上限
                if self.recursion_limit <= 1 {
                    self.state = DiceRollerState::Error("Recursion limit exceeded".to_string());
                    return Ok(());
                }
                self.recursion_limit -= 1;
                // 然后检查骰子计数
                let dice_count = self.context.requests.iter().map(|r| r.count).sum::<u32>();
                if self.dice_count_limit < dice_count {
                    self.state = DiceRollerState::Error("Dice count limit exceeded".to_string());
                    return Ok(());
                }
                self.dice_count_limit -= dice_count;
                // 如果都不满足，进入下一个状态
                self.state = DiceRollerState::WaitingForResponses(self.context.requests.clone())
            }
            Err(e) => {
                self.state = DiceRollerState::Error(e);
            }
        }
        Ok(())
    }

    pub fn set_responses(&mut self, responses: Vec<RuntimeResponse>) -> Result<(), String> {
        match &self.state {
            DiceRollerState::WaitingForResponses(requests) => {
                if responses.len() != requests.len() {
                    return Err("Can not set responses: error responses count".to_string());
                }
                match self.context.process_runtime_responses(responses) {
                    Err(e) => self.state = DiceRollerState::Error(e),
                    Ok(()) => self.state = DiceRollerState::WaitingForEvaluation,
                }
                Ok(())
            }
            _ => Err("Can not set responses: not in WaitingForResponses state".to_string()),
        }
    }

    pub fn try_get_results(&self) -> Result<Option<OutputNode>, String> {
        match &self.state {
            DiceRollerState::Error(e) => Err(e.clone()),
            DiceRollerState::Done(v) => Ok(Some(v.clone())),
            _ => Ok(None),
        }
    }
}

#[wasm_bindgen]
pub fn roll_without_animation(
    dice_expr: String,
    recursion_limit: u32,
    dice_count_limit: u32,
) -> Result<OutputNode, String> {
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

    let mut dice_roller =
        DiceRollerWithoutAnimation::new(dice_expr, recursion_limit, dice_count_limit)?;
    let mut counter: u32 = 0;
    while dice_roller.try_get_results()?.is_none() {
        dice_roller.evaluation()?;
        if let DiceRollerState::WaitingForResponses(requests) = &dice_roller.state {
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
