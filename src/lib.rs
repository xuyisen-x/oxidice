//! A dice utility library for D&D helper.
//!
//! This crate provides functionality for dice rolling and related utilities.

pub(crate) mod compiler;
pub(crate) mod grammar;
pub(crate) mod lower;
pub(crate) mod optimizer;
pub(crate) mod render_result;
pub(crate) mod runtime;
pub(crate) mod runtime_engine;
pub(crate) mod types;

use serde::Serialize;
use tsify::Tsify;
use wasm_bindgen::prelude::*;

use crate::optimizer::constant_fold::constant_fold_hir;

// ==========================================
// main.rs 专供函数
// ==========================================

pub fn parse_dice_and_show(input: &str) -> Result<String, String> {
    let ast = grammar::parse_dice(input)?;
    let hir = lower::lower_expr(ast)?;
    let hir = constant_fold_hir(hir)?;
    Ok(format!("{}", hir))
}

pub use runtime::roll_without_animation;

// ==========================================
// 辅助类型定义
// ==========================================

// 用于检查常量是否是常量整数的结果类型，用于check_constant_integer函数
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
#[serde(tag = "result", content = "value", rename_all = "camelCase")]
pub enum ConstantIntegerCheckResult {
    Constant(f64),
    NotConstant(String),
}

// 用于检查常量是否是常量整数的结果类型，用于check_number函数
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
#[serde(tag = "result", content = "value", rename_all = "camelCase")]
pub enum NumberCheckResult {
    Number,
    NotNumber(String),
}

// 用于表示带有原因的布尔结果，如果为False，则包含原因字符串
#[derive(Tsify, Serialize)]
#[tsify(into_wasm_abi)]
#[serde(tag = "result", content = "value", rename_all = "camelCase")]
pub enum FoldedDiceExpression {
    Valid(String),
    Invalid(String),
}

// ==========================================
// 相关函数定义
// ==========================================

//检查输入的表达式是否为常量整数
#[wasm_bindgen(js_name = checkConstantInteger)]
pub fn check_constant_integer(input: String) -> ConstantIntegerCheckResult {
    use ConstantIntegerCheckResult::*;
    use types::{hir::HIR, hir::NumberType};
    let ast = match grammar::parse_dice(input.as_str()) {
        Ok(a) => a,
        Err(_) => return NotConstant("parse error".to_string()),
    };
    let hir = match lower::lower_expr(ast) {
        Ok(h) => h,
        Err(e) => return NotConstant(e),
    };
    let folded_hir = match constant_fold_hir(hir) {
        Ok(fh) => fh,
        Err(e) => return NotConstant(e),
    };
    match folded_hir {
        HIR::Number(NumberType::Constant(c)) => {
            if c.fract() == 0.0 {
                Constant(c)
            } else {
                NotConstant("The constant is not an integer.".to_string())
            }
        }
        _ => NotConstant("The expression is not a constant.".to_string()),
    }
}

//检查检查输入的表达式是否为数字类型，而不是列表
#[wasm_bindgen(js_name = checkNumber)]
pub fn check_number(input: String) -> NumberCheckResult {
    use NumberCheckResult::*;
    use types::hir::HIR;
    let ast = match grammar::parse_dice(input.as_str()) {
        Ok(a) => a,
        Err(_) => return NotNumber("parse error".to_string()),
    };
    let hir = match lower::lower_expr(ast) {
        Ok(h) => h,
        Err(e) => return NotNumber(e),
    };
    match hir {
        HIR::Number(_) => Number,
        _ => NotNumber("The expression is a list not a number.".to_string()),
    }
}

// 检查输入的表达式是否为合法的骰子表达式
#[wasm_bindgen(js_name = tryFoldDiceExpression)]
pub fn try_fold_dice_expression(input: String) -> FoldedDiceExpression {
    use FoldedDiceExpression::*;
    let ast = match grammar::parse_dice(input.as_str()) {
        Ok(a) => a,
        Err(_) => return Invalid("parse error".to_string()),
    };
    let hir = match lower::lower_expr(ast) {
        Ok(h) => h,
        Err(e) => return Invalid(e),
    };
    let folded_hir = match constant_fold_hir(hir) {
        Ok(fh) => fh,
        Err(e) => return Invalid(e),
    };
    Valid(format!("{}", folded_hir))
}

// 其他wasm_bindgen绑定的函数见runtime.rs
