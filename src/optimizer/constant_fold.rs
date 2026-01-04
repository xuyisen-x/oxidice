use super::fold_binary_op::fold_binary_op;
use crate::types::hir::{
    DicePoolType, HIR, ListBinaryType, ListFunctionType, ListType, NumberBinaryType,
    NumberFunctionType, NumberType,
};
use crate::types::hir_rewriter::HirVisitor;

pub struct ConstantFolder;

impl HirVisitor for ConstantFolder {
    fn visit_number_self(&mut self, n: &mut NumberType) -> Result<(), String> {
        use NumberType::*;
        let new_val = match n {
            Neg(inner) => {
                if let NumberType::Constant(val) = **inner {
                    Some(NumberType::Constant(-val))
                } else {
                    None
                }
            }
            NumberBinary(bin_op) => fold_binary_op(bin_op)?,
            NumberFunction(func) => fold_number_function(func),
            DicePool(dice_pool) => fold_dice_pool(dice_pool),
            Constant(_) | SuccessPool(_) => None, // 无法折叠，也不应折叠
        };
        // 如果计算出了新值，替换当前节点
        if let Some(val) = new_val {
            *n = val;
        };
        Ok(())
    }
    fn visit_list_self(&mut self, l: &mut ListType) -> Result<(), String> {
        use ListType::*;
        let new_val = match l {
            ListFunction(list_func) => fold_list_function(list_func),
            ListBinary(list_bin_op) => fold_list_binary_op(list_bin_op)?,
            Explicit(_) => None, // 无法折叠，也不应折叠
        };
        if let Some(val) = new_val {
            *l = val
        };
        Ok(())
    }
}

// ==========================================
// 入口函数
// ==========================================

pub fn constant_fold_hir(hir: HIR) -> Result<HIR, String> {
    let mut hir = hir;
    let mut folder = ConstantFolder;
    folder.visit_hir(&mut hir)?;
    Ok(hir)
}

// ==========================================
// 细分函数定义
// ==========================================

fn fold_number_function(func: &mut NumberFunctionType) -> Option<NumberType> {
    use NumberFunctionType::*;

    match func {
        // --- 单值函数 ---
        Floor(inner) => try_map_const(inner, |v| v.floor()),
        Ceil(inner) => try_map_const(inner, |v| v.ceil()),
        Round(inner) => try_map_const(inner, |v| v.round()),
        Abs(inner) => try_map_const(inner, |v| v.abs()),

        // --- 列表聚合函数 (Sum, Avg, Min, Max, Len) ---
        Sum(list_box) => fold_list_aggregate(list_box, |nums| {
            nums.iter().fold(0.0_f64, |acc, x| acc + *x)
        })
        .or_else(|| {
            if !matches!(**list_box, ListType::Explicit(_)) {
                return None;
            }
            let old_list = std::mem::replace(&mut **list_box, ListType::Explicit(Vec::new()));
            if let ListType::Explicit(vec) = old_list {
                let mut iter = vec.into_iter();
                let first = iter.next()?; // 不可能是空
                let sum_tree = iter.fold(first, |acc, item| {
                    NumberType::NumberBinary(NumberBinaryType::Add(Box::new(acc), Box::new(item)))
                });
                // 因为是新创建的，需要优化
                let hir = HIR::Number(sum_tree);
                let optimized_hir = constant_fold_hir(hir).ok()?;
                let sum_tree = optimized_hir.except_number().ok()?;
                Some(sum_tree)
            } else {
                unreachable!("Already checked matches Explicit")
            }
        }),
        Avg(list_box) => fold_list_aggregate(list_box, |nums| {
            if nums.is_empty() {
                0.0
            } else {
                nums.iter().sum::<f64>() / nums.len() as f64
            }
        }),
        Max(list_box) => fold_list_aggregate(list_box, |nums| {
            nums.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        }),
        Min(list_box) => fold_list_aggregate(list_box, |nums| {
            nums.iter().fold(f64::INFINITY, |a, &b| a.min(b))
        }),
        Len(list_box) => {
            // Len 比较特殊，只要是 Explicit 列表，不管里面是不是常数，长度都是固定的
            if let ListType::Explicit(vec) = &**list_box {
                Some(NumberType::Constant(vec.len() as f64))
            } else {
                None
            }
        }
    }
}

fn fold_list_function(func: &mut ListFunctionType) -> Option<ListType> {
    use ListFunctionType::*;

    match func {
        Floor(list_box) => try_map_constant_list(list_box, |v| v.floor()),
        Ceil(list_box) => try_map_constant_list(list_box, |v| v.ceil()),
        Round(list_box) => try_map_constant_list(list_box, |v| v.round()),
        Abs(list_box) => try_map_constant_list(list_box, |v| v.abs()),
        Max(list_box, num_box) if list_box.is_constant_list() && num_box.is_constant() => {
            let values = try_get_constant_values(list_box)?;
            let counts = try_get_constant_value(num_box)?;
            Some(keep_elements_preserve_order(values, counts, true))
        }
        Min(list_box, num_box) if list_box.is_constant_list() && num_box.is_constant() => {
            let values = try_get_constant_values(list_box)?;
            let counts = try_get_constant_value(num_box)?;
            Some(keep_elements_preserve_order(values, counts, false))
        }
        Sort(list_box) if list_box.is_constant_list() => {
            let mut values = try_get_constant_values(list_box)?;
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            Some(ListType::Explicit(
                values.into_iter().map(NumberType::Constant).collect(),
            ))
        }
        SortDesc(list_box) if list_box.is_constant_list() => {
            let mut values = try_get_constant_values(list_box)?;
            values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
            Some(ListType::Explicit(
                values.into_iter().map(NumberType::Constant).collect(),
            ))
        }
        Filter(list_box, param) if list_box.is_constant_list() && param.is_constant() => {
            let cmp_func = param.get_compare_function()?;
            let values = try_get_constant_values(list_box)?;
            let filtered: Vec<NumberType> = values
                .into_iter()
                .filter(|v| cmp_func(*v))
                .map(NumberType::Constant)
                .collect();
            Some(ListType::Explicit(filtered))
        }
        _ => None,
    }
}

fn fold_list_binary_op(list_bin_op: &mut ListBinaryType) -> Result<Option<ListType>, String> {
    use ListBinaryType::*;
    match list_bin_op {
        AddList(left, right) => {
            if !left.is_explicit() || !right.is_explicit() {
                return Ok(None);
            }
            let left_node = std::mem::replace(&mut **left, ListType::Explicit(Vec::new()));
            let right_node = std::mem::replace(&mut **right, ListType::Explicit(Vec::new()));
            if let (ListType::Explicit(left_vec), ListType::Explicit(right_vec)) =
                (left_node, right_node)
            {
                let mut combined = left_vec;
                combined.extend(right_vec);
                Ok(Some(ListType::Explicit(combined)))
            } else {
                unreachable!("Already checked matches Explicit")
            }
        }
        // 接下来是广播操作的折叠，只处理常数列表广播常数的情况
        Add(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            Ok(try_map_constant_list(list, |v| v + num))
        }
        Multiply(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            Ok(try_map_constant_list(list, |v| v * num))
        }
        // 有顺序要求的操作
        Subtract(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            Ok(try_map_constant_list(list, |v| v - num))
        }
        SubtractReverse(num, list) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            Ok(try_map_constant_list(list, |v| num - v))
        }
        Divide(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if num == 0.0 {
                return Err("Division by zero in list division".to_string());
            }
            Ok(try_map_constant_list(list, |v| v / num))
        }
        DivideReverse(num, list) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if let ListType::Explicit(vec) = &**list {
                if let Some(idx) = vec
                    .iter()
                    .position(|n| matches!(n, NumberType::Constant(v) if *v == 0.0))
                {
                    return Err(format!(
                        "Division by zero in reverse list division at index {}",
                        idx
                    ));
                }
            }
            Ok(try_map_constant_list(list, |v| num / v))
        }
        Modulo(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if num == 0.0 {
                return Err("Modulo by zero in list modulo".to_string());
            }
            Ok(try_map_constant_list(list, |v| v % num))
        }
        ModuloReverse(num, list) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if let ListType::Explicit(vec) = &**list {
                if let Some(idx) = vec
                    .iter()
                    .position(|n| matches!(n, NumberType::Constant(v) if *v == 0.0))
                {
                    return Err(format!(
                        "Modulo by zero in reverse list modulo at index {}",
                        idx
                    ));
                }
            }
            Ok(try_map_constant_list(list, |v| num % v))
        }
        IntDivide(list, num) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if num == 0.0 {
                return Err("Integer division by zero in list integer division".to_string());
            }
            Ok(try_map_constant_list(list, |v| (v / num).floor()))
        }
        IntDivideReverse(num, list) => {
            let Some(num) = try_get_constant_value(num) else {
                return Ok(None);
            };
            if let ListType::Explicit(vec) = &**list {
                if let Some(idx) = vec
                    .iter()
                    .position(|n| matches!(n, NumberType::Constant(v) if *v == 0.0))
                {
                    return Err(format!(
                        "Integer division by zero in reverse list integer division at index {}",
                        idx
                    ));
                }
            }
            Ok(try_map_constant_list(list, |v| (num / v).floor()))
        }
    }
}

fn fold_dice_pool(dice_pool: &mut DicePoolType) -> Option<NumberType> {
    // 对常数counts和sides进行预处理
    use DicePoolType::*;
    match dice_pool {
        Standard(count_box, side_box) => {
            if !count_box.is_constant() && !side_box.is_constant() {
                return None;
            }
            let new_count = if count_box.is_constant() {
                let count = try_get_constant_value(&count_box)?; // 一定成功
                let new_count = (count as i64) as f64; // 模拟转化为整数的截断
                if new_count <= 0.0 {
                    return Some(NumberType::Constant(0.0));
                } else if count != new_count {
                    Some(new_count)
                } else {
                    None
                }
            } else {
                None
            };
            let new_side = if side_box.is_constant() {
                let side = try_get_constant_value(&side_box)?; // 一定成功
                let new_side = (side as i64) as f64; // 模拟转化为整数的截断
                if new_side <= 0.0 {
                    return Some(NumberType::Constant(0.0));
                } else if side != new_side {
                    Some(new_side)
                } else {
                    None
                }
            } else {
                None
            };
            if new_count.is_none() && new_side.is_none() {
                return None; // 没有变化，不需要处理
            }
            let new_count_box = if let Some(nc) = new_count {
                Box::new(NumberType::Constant(nc))
            } else {
                std::mem::replace(count_box, Box::new(NumberType::Constant(0.0)))
            };
            let new_side_box = if let Some(ns) = new_side {
                Box::new(NumberType::Constant(ns))
            } else {
                std::mem::replace(side_box, Box::new(NumberType::Constant(0.0)))
            };
            Some(NumberType::DicePool(Standard(new_count_box, new_side_box)))
        }
        Fudge(count_box) if count_box.is_constant() => {
            let count = try_get_constant_value(&count_box)?; // 一定成功
            let new_count = (count as i64) as f64; // 模拟转化为整数的截断
            if new_count > 0.0 {
                if new_count == count {
                    None // 没有变化，保持不变
                } else {
                    Some(NumberType::DicePool(Fudge(Box::new(NumberType::Constant(
                        new_count,
                    )))))
                }
            } else {
                Some(NumberType::Constant(0.0))
            }
        }
        Coin(count_box) if count_box.is_constant() => {
            let count = try_get_constant_value(&count_box)?; // 一定成功
            let new_count = (count as i64) as f64; // 模拟转化为整数的截断
            if new_count > 0.0 {
                if count == new_count {
                    None // 没有变化，保持不变
                } else {
                    Some(NumberType::DicePool(Coin(Box::new(NumberType::Constant(
                        new_count,
                    )))))
                }
            } else {
                Some(NumberType::Constant(0.0))
            }
        }
        _ => None,
    }
}

// ==========================================
// 辅助函数定义
// ==========================================

fn try_map_const<F>(n: &NumberType, f: F) -> Option<NumberType>
where
    F: Fn(f64) -> f64,
{
    if let NumberType::Constant(v) = n {
        Some(NumberType::Constant(f(*v)))
    } else {
        None
    }
}

fn fold_list_aggregate<F>(list: &ListType, f: F) -> Option<NumberType>
where
    F: Fn(&[f64]) -> f64,
{
    if let ListType::Explicit(vec) = list {
        // 检查是否所有元素都是 Constant
        let mut constants = Vec::with_capacity(vec.len());
        for item in vec {
            if let NumberType::Constant(c) = item {
                constants.push(*c);
            } else {
                return None; // 只要有一个不是常数（比如是骰子），就不能折叠
            }
        }
        // 全部是常数，计算结果
        Some(NumberType::Constant(f(&constants)))
    } else {
        None
    }
}

fn try_map_constant_list<F>(list: &ListType, f: F) -> Option<ListType>
where
    F: Fn(f64) -> f64,
{
    if let ListType::Explicit(vec) = list {
        vec.iter()
            .map(|item| match item {
                NumberType::Constant(c) => Some(NumberType::Constant(f(*c))),
                _ => None,
            })
            .collect::<Option<Vec<NumberType>>>() // 这里如果有一个 None，直接短路
            .map(ListType::Explicit) // 将成功的 Vec 包装回 ListType
    } else {
        None
    }
}

fn try_get_constant_value(n: &NumberType) -> Option<f64> {
    if let NumberType::Constant(v) = n {
        Some(*v)
    } else {
        None
    }
}

fn try_get_constant_values(list: &ListType) -> Option<Vec<f64>> {
    if let ListType::Explicit(vec) = list {
        let mut constants = Vec::with_capacity(vec.len());
        for item in vec {
            if let NumberType::Constant(c) = item {
                constants.push(*c);
            } else {
                return None; // 只要有一个不是常数（比如是骰子），就不能折叠
            }
        }
        Some(constants)
    } else {
        None
    }
}

// 从列表中保留 count 个元素并保持相对顺序
//keep_highest: true 为取最大 (Max), false 为取最小 (Min)
fn keep_elements_preserve_order(values: Vec<f64>, raw_count: f64, keep_highest: bool) -> ListType {
    if raw_count < 0.0 {
        return ListType::Explicit(Vec::new());
    }

    let count = raw_count as usize;

    if count >= values.len() {
        return ListType::Explicit(values.into_iter().map(NumberType::Constant).collect());
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

    let result = top_k
        .into_iter()
        .map(|(_, val)| NumberType::Constant(val))
        .collect();

    ListType::Explicit(result)
}
