// use crate::grammar::BinOp;
// use crate::preprocessor::{Ir, expect_constant_integer};

// // ==========================================
// // 入口函数
// // ==========================================
// pub fn fold_binary(lhs: Ir, op: BinOp, rhs: Ir) -> Result<Ir, String> {
//     match op {
//         BinOp::Add => fold_add(lhs, rhs),
//         BinOp::Sub => fold_sub(lhs, rhs),
//         BinOp::Mul => fold_mul(lhs, rhs),
//         BinOp::Div => fold_div(lhs, rhs),
//         BinOp::Idiv => fold_idiv(lhs, rhs),
//         BinOp::Mod => fold_mod(lhs, rhs),
//     }
// }

// // ==========================================
// // 主要函数
// // ==========================================

// fn fold_add(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     let mut builder = LinearBuilder::new();
//     builder.add_ir(lhs, 1.0); // + lhs
//     builder.add_ir(rhs, 1.0); // + rhs
//     Ok(builder.build())
// }

// fn fold_sub(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     let mut builder = LinearBuilder::new();
//     builder.add_ir(lhs, 1.0); // + lhs
//     builder.add_ir(rhs, -1.0); // - rhs
//     Ok(builder.build())
// }

// fn fold_mul(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     match (lhs, rhs) {
//         // 先处理常数乘列表的情况
//         (Ir::Constant(c), Ir::List(items)) | (Ir::List(items), Ir::Constant(c)) => {
//             let count = expect_constant_integer(&Ir::Constant(c))?;
//             let mut new_list = Vec::new();
//             for _ in 0..count {
//                 new_list.extend(items.clone());
//             }
//             Ok(Ir::List(new_list))
//         }
//         // 然后处理常数乘常数的情况
//         (Ir::Constant(c1), Ir::Constant(c2)) => Ok(Ir::Constant(c1 * c2)),
//         // 吸收率与身份率（列表情况已经在上面处理，这里不需要考虑列表）
//         (Ir::Constant(1.0), r) | (r, Ir::Constant(1.0)) => Ok(r),
//         (Ir::Constant(0.0), _) | (_, Ir::Constant(0.0)) => Ok(Ir::Constant(0.0)),
//         // 结合率（同时考虑交换律）
//         (Ir::Mul(in_l, in_r), Ir::Constant(c)) | (Ir::Constant(c), Ir::Mul(in_l, in_r)) => {
//             match (*in_l, *in_r) {
//                 (Ir::Constant(inner_c), other) | (other, Ir::Constant(inner_c)) => Ok(Ir::Mul(
//                     Box::new(Ir::Constant(c * inner_c)),
//                     Box::new(other),
//                 )),
//                 (left, right) => Ok(Ir::Mul(
//                     Box::new(Ir::Constant(c)),
//                     Box::new(Ir::Mul(Box::new(left), Box::new(right))),
//                 )),
//             }
//         }
//         (l, r) => {
//             // 其他情况，无法简化，直接返回乘法节点
//             Ok(Ir::Mul(Box::new(l), Box::new(r)))
//         }
//     }
// }

// fn fold_div(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     generic_fold(lhs, rhs, BinOp::Div)
// }

// fn fold_idiv(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     generic_fold(lhs, rhs, BinOp::Idiv)
// }

// fn fold_mod(lhs: Ir, rhs: Ir) -> Result<Ir, String> {
//     generic_fold(lhs, rhs, BinOp::Mod)
// }

// // ==========================================
// // 辅助结构和函数
// // ==========================================

// struct LinearBuilder {
//     constant: f64,
//     terms: Vec<(Ir, f64)>,            // 为了保证顺序，使用 Vec 而不是 HashMap
//     dice_terms: Vec<(f64, i64, i64)>, // (系数, 骰子数量, 骰子面数)
// }
// impl LinearBuilder {
//     // 创建一个新的 LinearBuilder
//     fn new() -> Self {
//         Self {
//             constant: 0.0,
//             terms: Vec::new(),
//             dice_terms: Vec::new(),
//         }
//     }

//     fn is_merge_able(ir: &Ir) -> bool {
//         match ir {
//             Ir::DicePool(_) => true,
//         }
//     }

//     // 递归地将 IR 拆解并加入构建器
//     fn add_ir(&mut self, ir: Ir, scale: f64) {
//         match ir {
//             // 如果是常数，直接累加到 constant
//             Ir::Constant(c) => self.constant += c * scale,

//             // 如果是加法，递归拆解：k * (A + B) = kA + kB
//             Ir::Add(l, r) => {
//                 self.add_ir(*l, scale);
//                 self.add_ir(*r, scale);
//             }

//             // 如果是减法，递归拆解：k * (A - B) = kA + (-k)B
//             Ir::Sub(l, r) => {
//                 self.add_ir(*l, scale);
//                 self.add_ir(*r, -scale);
//             }

//             // 对于乘法，因为实现了常量折叠，在这里遇到的乘法一定含有骰子项，不能继续拆解

//             // 处理是骰池的情况
//             Ir::DicePool((count, side)) => {
//                 self.push_dice_term(count, side, scale);
//             }

//             other => self.push_term(other, scale),
//         }
//     }

//     fn push_term(&mut self, item: Ir, coeff: f64) {
//         // 同类项不能合并！！！
//         // 2 * 2d6kh + 2 * 2d6kh 绝对不等于 4 * 2d6kh
//         self.terms.push((item, coeff));
//     }

//     fn push_dice_term(&mut self, count: i64, side: i64, coeff: f64) {
//         // 系数相同的裸骰池可以合并
//         for (existing_coeff, existing_count, existing_side) in self.dice_terms.iter_mut() {
//             if *existing_coeff == coeff && *existing_side == side {
//                 *existing_count += count;
//                 return;
//             }
//         }
//         self.dice_terms.push((coeff, count, side));
//     }

//     // 重建 IR
//     fn build(self) -> Ir {
//         // 1. 如果没有变量项，直接返回常数
//         if self.terms.is_empty() && self.dice_terms.is_empty() {
//             return Ir::Constant(self.constant);
//         }

//         // 2. 开始构建树
//         // 策略：先把所有项变成 TermIR，然后用 Add 连起来
//         let mut nodes = Vec::<(BinOp, Ir)>::new();

//         // 优先处理dice_items
//         for (coeff, count, side) in self.dice_terms {
//             // 如果coeff是0，跳过
//             if coeff == 0.0 {
//                 continue;
//             }
//             // 将系数还原回乘法或纯项
//             let dice_node = Ir::DicePool((count, side));
//             let op = if coeff >= 0.0 { BinOp::Add } else { BinOp::Sub };
//             let abs_coeff = coeff.abs();
//             let term_node = if abs_coeff == 1.0 {
//                 dice_node
//             } else {
//                 Ir::Mul(Box::new(Ir::Constant(abs_coeff)), Box::new(dice_node))
//             };
//             nodes.push((op, term_node));
//         }

//         // 然后处理普通项
//         for (item, coeff) in self.terms {
//             if coeff == 0.0 {
//                 continue;
//             }
//             // 将系数还原回乘法或纯项
//             let op = if coeff >= 0.0 { BinOp::Add } else { BinOp::Sub };
//             let abs_coeff = coeff.abs();
//             let term_node = if abs_coeff == 1.0 {
//                 item
//             } else {
//                 Ir::Mul(Box::new(Ir::Constant(abs_coeff)), Box::new(item))
//             };
//             nodes.push((op, term_node));
//         }

//         // 最后处理常数项
//         if self.constant != 0.0 {
//             let op = if self.constant >= 0.0 {
//                 BinOp::Add
//             } else {
//                 BinOp::Sub
//             };
//             let abs_const = self.constant.abs();
//             let const_node = Ir::Constant(abs_const);
//             nodes.push((op, const_node));
//         }

//         // 为了安全，使用前判下空
//         if nodes.is_empty() {
//             return Ir::Constant(0.0); // unreachable
//         }
//         // 3. 用 Add 将所有节点连起来
//         let mut iter = nodes.into_iter();
//         let first = match iter.next().unwrap() {
//             (BinOp::Add, node) => node,
//             (BinOp::Sub, node) => Ir::Sub(Box::new(Ir::Constant(0.0)), Box::new(node)), // 负项开头
//             _ => unreachable!(),
//         };

//         iter.fold(first, |acc, (op, node)| {
//             match op {
//                 BinOp::Add => Ir::Add(Box::new(acc), Box::new(node)),
//                 BinOp::Sub => Ir::Sub(Box::new(acc), Box::new(node)),
//                 _ => unreachable!(), // 这里只会有加减法
//             }
//         })
//     }
// }

// fn generic_fold(lhs: Ir, rhs: Ir, op: BinOp) -> Result<Ir, String> {
//     // 1. 双常数
//     if let (Ir::Constant(l), Ir::Constant(r)) = (&lhs, &rhs) {
//         if *r == 0.0 {
//             return Err("Division by zero".to_string()); // unreachable
//         }
//         let result = match op {
//             BinOp::Div => l / r,
//             BinOp::Idiv => {
//                 let l = expect_constant_integer(&Ir::Constant(*l))?;
//                 let r = expect_constant_integer(&Ir::Constant(*r))?;
//                 (l / r) as f64
//             }
//             BinOp::Mod => {
//                 let l = expect_constant_integer(&Ir::Constant(*l))?;
//                 let r = expect_constant_integer(&Ir::Constant(*r))?;
//                 (l % r) as f64
//             }
//             _ => unreachable!(),
//         };
//         return Ok(Ir::Constant(result));
//     }

//     // 不优化 0 / x = 0 (x!=0)

//     // x / 1 = x ，仅优化除法情况（整除和取模不优化）
//     if let Ir::Constant(r) = &rhs {
//         if *r == 1.0 && op == BinOp::Div {
//             return Ok(lhs);
//         }
//     }

//     // x / x = 1 ，x没有副作用，你做梦呢？

//     // 结合律 (X / C1) / C2 -> X / (C1 * C2)
//     // 仅对普通除法有效
//     match (&lhs, &rhs) {
//         (Ir::Div(l_inner, r_inner), Ir::Constant(r2)) if op == BinOp::Div => {
//             if let Ir::Constant(r1) = **r_inner {
//                 if *r2 == 0.0 {
//                     return Err("Division by zero".to_string()); // unreachable
//                 }
//                 let new_denominator = r1 * r2;
//                 return Ok(Ir::Div(
//                     l_inner.clone(),
//                     Box::new(Ir::Constant(new_denominator)),
//                 ));
//             }
//         }
//         _ => {}
//     }

//     // 构建原节点
//     let l_box = Box::new(lhs);
//     let r_box = Box::new(rhs);
//     Ok(match op {
//         BinOp::Div => Ir::Div(l_box, r_box),
//         BinOp::Idiv => Ir::Idiv(l_box, r_box),
//         BinOp::Mod => Ir::Mod(l_box, r_box),
//         _ => unreachable!(),
//     })
// }
