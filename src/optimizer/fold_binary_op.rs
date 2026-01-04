use crate::types::hir::{DicePoolType, NumberBinaryType, NumberType};
use std::{collections::BTreeMap, mem};

// ==========================================
// 核心入口
// ==========================================

pub fn fold_binary_op(op: &mut NumberBinaryType) -> Result<Option<NumberType>, String> {
    use NumberBinaryType::*;

    match op {
        // 1. 加减法：进入线性构建器 (Linear Builder)
        Add(l, r) => fold_linear_add_sub(l, r, false),
        Subtract(l, r) => fold_linear_add_sub(l, r, true),

        // 2. 乘法：进入乘法构建器 (处理结合律、单位元、零元)
        Multiply(l, r) => fold_linear_multiply(l, r),

        // 3. 除法：原地优化
        Divide(l, r) => fold_divide(l, r),

        // 4. 其他操作：简单的常数折叠
        IntDivide(l, r) => {
            if let (NumberType::Constant(c1), NumberType::Constant(c2)) = (&**l, &**r) {
                if *c2 == 0.0 {
                    return Err("Integer division by zero".to_string());
                }
                Ok(Some(NumberType::Constant((c1 / c2).floor())))
            } else {
                Ok(None)
            }
        }
        Modulo(l, r) => {
            if let (NumberType::Constant(c1), NumberType::Constant(c2)) = (&**l, &**r) {
                if *c2 == 0.0 {
                    return Err("Modulo by zero".to_string());
                }
                Ok(Some(NumberType::Constant(c1 % c2)))
            } else {
                Ok(None)
            }
        }
    }
}

// ==========================================
// 1. 加减法线性优化 (Linear Builder)
// ==========================================
// 目标：将 (A + B) - (C + 1) 展平为 [A, B, -C] 和常数 acc = -1
fn fold_linear_add_sub(
    left: &mut NumberType,
    right: &mut NumberType,
    is_subtract: bool,
) -> Result<Option<NumberType>, String> {
    // 【偷梁换柱】：用 0.0 替换掉原来的节点，从而拿到所有权 (Move)
    // 这一步是 Zero-Copy 的关键，我们拿到了原本的树的所有权
    let l_owned = mem::replace(left, NumberType::Constant(0.0));
    let r_owned = mem::replace(right, NumberType::Constant(0.0));

    let mut terms = Vec::new();
    let mut constant_acc = 0.0;

    // 递归展开
    flatten_add_sub(l_owned, 1.0, &mut terms, &mut constant_acc);
    flatten_add_sub(
        r_owned,
        if is_subtract { -1.0 } else { 1.0 },
        &mut terms,
        &mut constant_acc,
    );

    // 合并terms中可以合并的项，如 1d6 + 2d6 -> 3d6
    let terms = merge_terms(terms);

    // 重组
    if terms.is_empty() {
        return Ok(Some(NumberType::Constant(constant_acc)));
    }

    // 如果常数是 0 且有其他项，常数消失 (x + 0 = x)
    if constant_acc == 0.0 {
        return Ok(Some(rebuild_add_tree(terms)));
    }

    let tree = rebuild_add_tree(terms);

    // 将常数作为最后一项加回去，根据常数正负决定是加还是减
    if constant_acc > 0.0 {
        Ok(Some(NumberType::NumberBinary(NumberBinaryType::Add(
            Box::new(tree),
            Box::new(NumberType::Constant(constant_acc)),
        ))))
    } else {
        Ok(Some(NumberType::NumberBinary(NumberBinaryType::Subtract(
            Box::new(tree),
            Box::new(NumberType::Constant(-constant_acc)),
        ))))
    }
}

// 消耗性展开
fn flatten_add_sub(
    node: NumberType, // 注意：这里接收的是所有权
    sign: f64,
    terms: &mut Vec<(NumberType, f64)>,
    acc: &mut f64,
) {
    match node {
        NumberType::Constant(c) => *acc += c * sign,

        // 递归展开 Add
        NumberType::NumberBinary(NumberBinaryType::Add(l, r)) => {
            flatten_add_sub(*l, sign, terms, acc);
            flatten_add_sub(*r, sign, terms, acc);
        }

        // 递归展开 Subtract (注意右边变号)
        NumberType::NumberBinary(NumberBinaryType::Subtract(l, r)) => {
            flatten_add_sub(*l, sign, terms, acc);
            flatten_add_sub(*r, -sign, terms, acc);
        }

        NumberType::Neg(inner) => {
            flatten_add_sub(*inner, -sign, terms, acc);
        }

        // 其他节点直接移动进 terms，不 Clone
        other => terms.push((other, sign)),
    }
}

fn rebuild_add_tree(mut terms: Vec<(NumberType, f64)>) -> NumberType {
    if terms.is_empty() {
        return NumberType::Constant(0.0);
    }

    let (first_node, first_sign) = terms.remove(0);

    let mut current = if first_sign < 0.0 {
        NumberType::Neg(Box::new(first_node))
    } else {
        first_node
    };

    for (item, sign) in terms {
        if sign > 0.0 {
            current =
                NumberType::NumberBinary(NumberBinaryType::Add(Box::new(current), Box::new(item)));
        } else {
            current = NumberType::NumberBinary(NumberBinaryType::Subtract(
                Box::new(current),
                Box::new(item),
            ));
        }
    }
    current
}

// 合并 terms 中可以合并的项，如 1d6 + 2d6 -> 3d6
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum DiceType {
    Standard(bool, i32), // (is_add, sides)
    Fudge(bool),
    Coin(bool),
}

fn merge_terms(terms: Vec<(NumberType, f64)>) -> Vec<(NumberType, f64)> {
    let mut dice_map = BTreeMap::new();
    let mut unmergeable_terms = Vec::new();
    for (node, sign) in terms {
        if let Some((dice_type, count)) = mergeable_dice(&node, sign) {
            let entry = dice_map.entry(dice_type).or_insert(0);
            *entry += count;
        } else {
            unmergeable_terms.push((node, sign));
        }
    }
    // 重建合并后的项
    let mut new_terms = Vec::new();
    new_terms.reserve(dice_map.len() + unmergeable_terms.len());
    for (dice_type, count) in dice_map.into_iter().rev() {
        if count == 0 {
            continue;
        }
        match dice_type {
            DiceType::Standard(is_add, sides) => {
                let counts_node = NumberType::Constant(count as f64);
                let sides_node = NumberType::Constant(sides as f64);
                let dice_pool = NumberType::DicePool(DicePoolType::Standard(
                    Box::new(counts_node),
                    Box::new(sides_node),
                ));
                new_terms.push((dice_pool, if is_add { 1.0 } else { -1.0 }));
            }
            DiceType::Fudge(is_add) => {
                let counts_node = NumberType::Constant(count as f64);
                let dice_pool = NumberType::DicePool(DicePoolType::Fudge(Box::new(counts_node)));
                new_terms.push((dice_pool, if is_add { 1.0 } else { -1.0 }));
            }
            DiceType::Coin(is_add) => {
                let counts_node = NumberType::Constant(count as f64);
                let dice_pool = NumberType::DicePool(DicePoolType::Coin(Box::new(counts_node)));
                new_terms.push((dice_pool, if is_add { 1.0 } else { -1.0 }));
            }
        }
    }
    new_terms.extend(unmergeable_terms); // 将未合并的项加入
    new_terms
}

// 识别出counts和sides均为常数的项
fn mergeable_dice(node: &NumberType, sign: f64) -> Option<(DiceType, i32)> {
    use NumberType::*;
    fn get_const_value(n: &NumberType) -> f64 {
        if let Constant(c) = n { *c } else { 0.0 }
    }
    if let DicePool(dice_pool) = node {
        use DicePoolType::*;
        match dice_pool {
            Standard(counts, sides) => {
                if counts.is_constant() && sides.is_constant() {
                    let c = get_const_value(counts);
                    let s = get_const_value(sides) as i32;
                    let c = if c > 0.0 { c as i32 } else { 0 };
                    Some((DiceType::Standard(sign > 0.0, s), c))
                } else {
                    return None;
                }
            }
            Fudge(counts) => {
                if counts.is_constant() {
                    let c = get_const_value(counts);
                    let c = if c > 0.0 { c as i32 } else { 0 };
                    Some((DiceType::Fudge(sign > 0.0), c))
                } else {
                    return None;
                }
            }
            Coin(counts) => {
                if counts.is_constant() {
                    let c = get_const_value(counts);
                    let c = if c > 0.0 { c as i32 } else { 0 };
                    Some((DiceType::Coin(sign > 0.0), c))
                } else {
                    return None;
                }
            }
            _ => None,
        }
    } else {
        None
    }
}

// ==========================================
// 2. 乘法线性优化
// ==========================================

fn fold_linear_multiply(
    left: &mut NumberType,
    right: &mut NumberType,
) -> Result<Option<NumberType>, String> {
    // 拿到所有权
    let l_owned = mem::replace(left, NumberType::Constant(0.0));
    let r_owned = mem::replace(right, NumberType::Constant(0.0));

    let mut terms = Vec::new();
    let mut constant_acc = 1.0;

    flatten_mul(l_owned, &mut terms, &mut constant_acc);
    flatten_mul(r_owned, &mut terms, &mut constant_acc);

    // 零元优化: x * 0 = 0 (丢弃 terms，这就释放了 terms 的内存)
    if constant_acc == 0.0 {
        return Ok(Some(NumberType::Constant(0.0)));
    }

    if terms.is_empty() {
        return Ok(Some(NumberType::Constant(constant_acc)));
    }

    // 单位元优化: x * 1 = x
    if constant_acc == 1.0 {
        return Ok(Some(rebuild_mul_tree(terms)));
    }

    // 重组: terms * C
    let tree = rebuild_mul_tree(terms);
    Ok(Some(NumberType::NumberBinary(NumberBinaryType::Multiply(
        Box::new(tree),
        Box::new(NumberType::Constant(constant_acc)),
    ))))
}

fn flatten_mul(node: NumberType, terms: &mut Vec<NumberType>, acc: &mut f64) {
    match node {
        NumberType::Constant(c) => *acc *= c,
        NumberType::NumberBinary(NumberBinaryType::Multiply(l, r)) => {
            flatten_mul(*l, terms, acc);
            flatten_mul(*r, terms, acc);
        }
        other => terms.push(other), // Move
    }
}

fn rebuild_mul_tree(mut terms: Vec<NumberType>) -> NumberType {
    if terms.is_empty() {
        return NumberType::Constant(1.0);
    }
    let mut current = terms.remove(0);
    for item in terms {
        current = NumberType::NumberBinary(NumberBinaryType::Multiply(
            Box::new(current),
            Box::new(item),
        ));
    }
    current
}

// ==========================================
// 3. 除法优化
// ==========================================

fn fold_divide(l: &mut NumberType, r: &mut NumberType) -> Result<Option<NumberType>, String> {
    use NumberBinaryType::*;
    use NumberType::*;

    // 1. 检查右边是否为常数 (只读检查)
    let c2 = match r {
        Constant(v) => *v,
        _ => return Ok(None), // 不是常数，放弃
    };

    if c2 == 0.0 {
        return Err("Division by zero".to_string());
    }

    // 2. 检查左边是否为常数
    if let Constant(c1) = l {
        return Ok(Some(Constant(*c1 / c2)));
    }

    // 3. 检查左边是否是 (Inner / C1) 结构
    if let NumberBinary(Divide(inner_box, c1_box)) = l {
        if let Constant(c1) = **c1_box {
            // 命中优化逻辑: (A / C1) / C2 -> A / (C1 * C2)
            let new_divisor = c1 * c2;
            if new_divisor == 0.0 {
                return Err("Division by zero after optimization".to_string());
            }

            let owned_inner = mem::replace(inner_box, Box::new(Constant(0.0)));

            // 构造新节点，此时 owned_inner 已经被 Move 过来，没有 Clone
            return Ok(Some(NumberBinary(Divide(
                owned_inner,
                Box::new(Constant(new_divisor)),
            ))));
        }
    }

    Ok(None)
}
