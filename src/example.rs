use crate::grammar::{self, BinOp, CompareOp, Expr, ModifierOp, ModifierParam};
use crate::ir::{CompareIr, Ir, NewDiceItem};
use crate::typecheck::{self, DiceItem}; // 假设 DiceItem 在 typecheck 中定义

pub struct Preprocessor;

impl Preprocessor {
    /// 入口函数
    pub fn process(expr: Expr) -> Ir {
        Self::transform(expr)
    }

    /// 核心转换逻辑
    fn transform(expr: Expr) -> Ir {
        match expr {
            // --- 1. 基础值 ---
            Expr::Number(n) => Ir::Constant(n),

            // --- 2. 骰子 ---
            Expr::Dice { count, side } => {
                // 递归处理子节点（虽然 TypeCheck 保证它们是常数，但 AST 结构上它们是 Expr）
                let c_ir = Self::transform(*count);
                let s_ir = Self::transform(*side);

                // 提取常数值。TypeCheck 保证这里一定是 Constant。
                let c_val = Self::expect_constant(&c_ir, "Dice count").round() as i64;
                let s_val = Self::expect_constant(&s_ir, "Dice side").round() as i64;

                Ir::DicePool((c_val, s_val))
            }

            // --- 3. 列表 ---
            Expr::List(items) => {
                let ir_list = items.into_iter().map(Self::transform).collect();
                Ir::List(ir_list)
            }

            // --- 4. 二元运算 (包含常量折叠和列表展开) ---
            Expr::Binary { lhs, op, rhs } => {
                let l_ir = Self::transform(*lhs);
                let r_ir = Self::transform(*rhs);

                match op {
                    // 深度加法折叠: (((1+2)+d6)+3) -> 6+d6
                    BinOp::Add => Self::flatten_and_fold_add(l_ir, r_ir),

                    // 列表乘法展开: [1d6] * 2 -> [1d6, 1d6]
                    BinOp::Mul => Self::handle_mul(l_ir, r_ir),

                    // 其他运算尝试简单折叠 (1-2 -> -1)
                    _ => Self::simple_fold(l_ir, op, r_ir),
                }
            }

            // --- 5. 函数调用 (rpdice 宏展开 + 普通函数映射) ---
            Expr::Call { func_name, args } => {
                if func_name == "rpdice" {
                    Self::expand_rpdice(args)
                } else {
                    Self::map_function(func_name, args)
                }
            }

            // --- 6. 修饰符 (合并 !! 和 l) ---
            Expr::Modifier { lhs, op, param } => Self::handle_modifier(*lhs, op, param),

            // --- 7. 成功判定 ---
            Expr::SuccessCheck { lhs, compare_expr } => {
                let lhs_ir = Self::transform(*lhs);
                // 提取比较条件。TypeCheck 保证 compare_expr.val 是常量
                let val_ir = Self::transform(compare_expr.val);
                let val = Self::expect_constant(&val_ir, "Success check target");

                Ir::SuccessCheck(Box::new(lhs_ir), (compare_expr.op, val))
            }
        }
    }

    // ==========================================
    // 具体处理逻辑
    // ==========================================

    /// 处理二元乘法：如果是 列表 * 常数，则展开列表
    fn handle_mul(lhs: Ir, rhs: Ir) -> Ir {
        // 检查是否有一边是列表，一边是常数
        let (list_opt, count_opt) = match (&lhs, &rhs) {
            (Ir::List(l), Ir::Constant(c)) => (Some(l), Some(*c)),
            (Ir::Constant(c), Ir::List(l)) => (Some(l), Some(*c)),
            _ => (None, None),
        };

        if let (Some(list), Some(count)) = (list_opt, count_opt) {
            let count = count.round() as usize;
            let mut new_items = Vec::with_capacity(list.len() * count);
            for _ in 0..count {
                // clone 列表中的 IR 节点
                new_items.extend(list.clone());
            }
            Ir::List(new_items)
        } else {
            // 普通乘法，尝试常量折叠
            Self::simple_fold(lhs, BinOp::Mul, rhs)
        }
    }

    /// 深度加法折叠
    fn flatten_and_fold_add(lhs: Ir, rhs: Ir) -> Ir {
        let mut terms = Vec::new();
        let mut constant_sum = 0.0;

        // 递归收集器
        fn collect(ir: Ir, terms: &mut Vec<Ir>, sum: &mut f64) {
            match ir {
                Ir::Add(l, r) => {
                    collect(*l, terms, sum);
                    collect(*r, terms, sum);
                }
                Ir::Constant(c) => *sum += c,
                // 列表拼接：List + List
                Ir::List(mut items) => {
                    // 如果遇到列表，因为列表加法也是 flatten 的一种，我们可以选择
                    // 1. 如果 TypeCheck 保证了加法两边类型一致，这里不需要担心 Number + List
                    // 2. 将 List 作为一个 Term (作为整体) 还是拆开？
                    //    通常 List+List 得到一个大 List。
                    //    但这里我们正在构建 Ir::Add 树。
                    //    如果两个操作数都是 List，simple_fold 会处理拼接。
                    //    如果走到这里，说明是混合结构？
                    //    **修正**：为了简化，如果遇到 List，直接作为 Term 放进去，
                    //    后续 Evaluator 处理 List + List 的逻辑。
                    //    或者在这里就做 List 拼接优化。
                    //    鉴于 Ir::Add 定义是 Box<Ir>，我们作为 Term 放入。
                    terms.push(Ir::List(items));
                }
                other => terms.push(other),
            }
        }

        collect(lhs, &mut terms, &mut constant_sum);
        collect(rhs, &mut terms, &mut constant_sum);

        // 如果只有常数，或者有常数项
        if constant_sum != 0.0 || terms.is_empty() {
            // 优化：将常数放在最前面或最后面
            terms.push(Ir::Constant(constant_sum));
        }

        // 重新构建左结合的加法树
        let mut iter = terms.into_iter();
        let first = iter.next().unwrap(); // terms 不会为空，因为至少有 lhs 和 rhs 产生的项

        iter.fold(first, |acc, term| {
            // 这里还可以再次做一个微小的优化：如果两边都是 List，直接合并成一个新的 List IR
            if let (Ir::List(mut l), Ir::List(r)) = (&acc, &term) {
                l.extend(r.clone());
                Ir::List(l)
            } else {
                Ir::Add(Box::new(acc), Box::new(term))
            }
        })
    }

    /// 简单的二元运算折叠
    fn simple_fold(lhs: Ir, op: BinOp, rhs: Ir) -> Ir {
        // 1. 数值常量折叠
        if let (Ir::Constant(l), Ir::Constant(r)) = (&lhs, &rhs) {
            return match op {
                BinOp::Add => Ir::Constant(l + r),
                BinOp::Sub => Ir::Constant(l - r),
                BinOp::Mul => Ir::Constant(l * r),
                BinOp::Div => Ir::Constant(l / r), // 注意除零在 Evaluator 或 TypeCheck 处理
                BinOp::Mod => Ir::Constant(l % r),
                _ => unreachable!(), // 比较运算符不在这里处理
            };
        }

        // 2. 列表加法折叠
        if let (Ir::List(mut l), Ir::List(r)) = (lhs.clone(), rhs.clone()) {
            if op == BinOp::Add {
                l.extend(r);
                return Ir::List(l);
            }
        }

        // 无法折叠，返回 IR 节点
        match op {
            BinOp::Add => Ir::Add(Box::new(lhs), Box::new(rhs)),
            BinOp::Sub => Ir::Sub(Box::new(lhs), Box::new(rhs)),
            BinOp::Mul => Ir::Mul(Box::new(lhs), Box::new(rhs)),
            BinOp::Div => Ir::Div(Box::new(lhs), Box::new(rhs)),
            BinOp::Mod => Ir::Mod(Box::new(lhs), Box::new(rhs)),
            // 你之前提到的 Idiv
            // BinOp::Idiv => Ir::Idiv(Box::new(lhs), Box::new(rhs)),
            _ => unreachable!(),
        }
    }

    /// 处理 rpdice 宏展开
    fn expand_rpdice(args: Vec<Expr>) -> Ir {
        // rpdice(expr, count?)
        let (expr, count) = match args.len() {
            1 => (args[0].clone(), 1),
            2 => {
                // TypeCheck 保证第二个参数是常数
                let count_ir = Self::transform(args[1].clone());
                let c = Self::expect_constant(&count_ir, "rpdice count").round() as usize;
                (args[0].clone(), c)
            }
            _ => panic!("Invalid rpdice args"), // TypeCheck 应该拦截
        };

        // 展开为列表
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            // 注意：这里必须重新 transform expr，不能只 transform 一次然后 clone IR。
            // 为什么？对于纯随机变量（1d6），Clone IR 是可以的（Ir::DicePool(1,6)）。
            // Evaluator 每次遇到 DicePool 都会新投。
            // 所以 transform 一次后 clone 结果是正确的且效率更高。
            let ir = Self::transform(expr.clone());
            list.push(ir);
        }
        Ir::List(list)
    }

    /// 映射普通函数
    fn map_function(name: String, args: Vec<Expr>) -> Ir {
        let args_ir: Vec<Ir> = args.into_iter().map(Self::transform).collect();

        // 辅助：获取第一个参数，预期它是一个 List (用于 sum/max/min 的聚合模式)
        // 注意：preprocess_call_args 已经在 TypeCheck 里把变长参数处理好了
        // 如果用户写 max(1,2)，Parser->AST 是 Call(args=[1,2])。
        // 这里我们需要把它们合成为一个 List 吗？
        // 根据你的 Ir::Max(Box<Ir>) 定义，Max 接受一个能求值为 List 的 IR。
        // 所以如果 args_ir 有多个，我们需要把它们 wrap 成 Ir::List。

        let target = if args_ir.len() == 1 {
            match args_ir[0] {
                Ir::List(_) => args_ir[0].clone(), // 已经是列表 max([1,2])
                _ => Ir::List(args_ir),            // 单个元素 max(1) -> Max([1])
            }
        } else {
            Ir::List(args_ir) // 多个元素 max(1,2) -> Max([1,2])
        };

        match name.as_str() {
            "sum" => Ir::Sum(Box::new(target)),
            "max" => Ir::Max(Box::new(target)),
            "min" => Ir::Min(Box::new(target)),
            "floor" => Ir::Floor(Box::new(Self::unwrap_single_arg(target))),
            "ceil" => Ir::Ceil(Box::new(Self::unwrap_single_arg(target))),
            "round" => Ir::Round(Box::new(Self::unwrap_single_arg(target))),
            "abs" => Ir::Abs(Box::new(Self::unwrap_single_arg(target))),
            _ => panic!("Unknown function in preprocessor"),
        }
    }

    // 辅助：对于 floor([1]) 这种情况，把 List 拆包回 1
    fn unwrap_single_arg(ir: Ir) -> Ir {
        if let Ir::List(mut items) = ir {
            if items.len() == 1 {
                return items.remove(0);
            }
        }
        // 如果不是 List 或者长度不为1，说明逻辑有误，或者是 floor(1) 这种直接传参
        // 这里只是一个简单的适配
        ir
    }

    /// 处理修饰符 (核心：合并逻辑)
    fn handle_modifier(lhs: Expr, op: ModifierOp, param: Option<ModifierParam>) -> Ir {
        let lhs_ir = Self::transform(lhs);

        // 提取参数 (TypeCheck 保证参数类型匹配)
        let get_int = || -> i64 {
            if let Some(ModifierParam::Value(expr)) = &param {
                let ir = Self::transform(expr.clone());
                Self::expect_constant(&ir, "Modifier param").round() as i64
            } else {
                1
            } // 默认 1
        };

        let get_compare = || -> CompareIr {
            if let Some(ModifierParam::Compare(ce)) = &param {
                let ir = Self::transform(ce.val.clone());
                let val = Self::expect_constant(&ir, "Compare value");
                (ce.op, val)
            } else if let Some(ModifierParam::Value(expr)) = &param {
                // 兼容 r 1 -> r = 1
                let ir = Self::transform(expr.clone());
                let val = Self::expect_constant(&ir, "Compare value");
                (CompareOp::Eq, val)
            } else {
                (CompareOp::Eq, 0.0) // 应该不可达
            }
        };

        match op {
            ModifierOp::KeepHigh => Ir::KeepHigh(Box::new(lhs_ir), get_int()),
            ModifierOp::KeepLow => Ir::KeepLow(Box::new(lhs_ir), get_int()),
            ModifierOp::DropHigh => Ir::DropHigh(Box::new(lhs_ir), get_int()),
            ModifierOp::DropLow => Ir::DropLow(Box::new(lhs_ir), get_int()),

            ModifierOp::Reroll => Ir::Reroll(Box::new(lhs_ir), get_compare()),
            ModifierOp::RerollOnce => Ir::RerollOnce(Box::new(lhs_ir), get_compare()),

            ModifierOp::Explode => Ir::Explode(Box::new(lhs_ir), get_compare()),

            ModifierOp::ExplodeCompound => {
                // !! 生成 ExplodeCompoundLimit，默认 Limit 为 None
                Ir::ExplodeCompoundLimit(Box::new(lhs_ir), get_compare(), None)
            }

            ModifierOp::Limit => {
                let limit_val = get_int();
                // 关键：合并逻辑
                // 检查 lhs_ir 是否是 ExplodeCompoundLimit
                if let Ir::ExplodeCompoundLimit(inner_dice, compare, _) = lhs_ir {
                    // 成功合并：更新 Limit
                    Ir::ExplodeCompoundLimit(inner_dice, compare, Some(limit_val))
                } else {
                    // 如果 TypeChecker 正常工作，l 只能跟在 !! 后面。
                    // 但如果用户写了 limitable_pool l 5，TypeCheck 通过了，
                    // 这里 lhs_ir 就必须是 ExplodeCompoundLimit。
                    // 如果不是（比如 lhs 是一个已经计算好的 List），这在语义上可能无法处理
                    // 假设 TypeChecker 保证了 lhs 必定是 LimitableDicePool 类型，即 IR 必定是 ExplodeCompoundLimit
                    panic!("Limit modifier applied to non-compound-explode IR");
                }
            }
        }
    }

    // ==========================================
    // 辅助函数
    // ==========================================

    fn expect_constant(ir: &Ir, context: &str) -> f64 {
        match ir {
            Ir::Constant(c) => *c,
            _ => panic!(
                "Preprocessor expected constant for {}, got {:?}",
                context, ir
            ),
        }
    }
}
