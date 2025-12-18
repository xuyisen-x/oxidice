use super::hir::*;

// ==========================================
// 遍历辅助结构 (Mutable Visitor Pattern)
// ==========================================

pub trait HirVisitor {
    // ==========================================
    // 顶层入口 (HIR)
    // ==========================================
    fn visit_hir(&mut self, h: &mut HIR) {
        self.visit_hir_children(h);
        self.visit_hir_self(h);
    }

    fn visit_hir_self(&mut self, _h: &mut HIR) {
        // 默认为空，用户在此处覆盖逻辑
    }

    fn visit_hir_children(&mut self, h: &mut HIR) {
        match h {
            HIR::Number(n) => self.visit_number(n),
            HIR::List(l) => self.visit_list(l),
        }
    }

    // ==========================================
    // NumberType
    // ==========================================
    fn visit_number(&mut self, n: &mut NumberType) {
        self.visit_number_children(n);
        self.visit_number_self(n);
    }

    fn visit_number_self(&mut self, _n: &mut NumberType) {}

    fn visit_number_children(&mut self, n: &mut NumberType) {
        use NumberType::*;
        match n {
            Constant(_) => {} // 叶子节点，无需递归
            DicePool(d) => self.visit_dice_pool(d),
            SuccessPool(s) => self.visit_success_pool(s),
            NumberBinary(b) => self.visit_number_binary(b),
            NumberFunction(f) => self.visit_number_function(f),
            // Rust 的 Deref Coercion 会自动将 &mut Box<T> 视为 &mut T
            Neg(x) => self.visit_number(x),
        }
    }

    // ==========================================
    // DicePoolType
    // ==========================================
    fn visit_dice_pool(&mut self, d: &mut DicePoolType) {
        self.visit_dice_pool_children(d);
        self.visit_dice_pool_self(d);
    }

    fn visit_dice_pool_self(&mut self, _d: &mut DicePoolType) {}

    fn visit_dice_pool_children(&mut self, d: &mut DicePoolType) {
        use DicePoolType::*;
        match d {
            Standard(x, y) => {
                self.visit_number(x);
                self.visit_number(y);
            }
            Fudge(x) => self.visit_number(x),
            Coin(x) => self.visit_number(x),
            KeepHigh(d, n)
            | KeepLow(d, n)
            | DropHigh(d, n)
            | DropLow(d, n)
            | Min(d, n)
            | Max(d, n) => {
                self.visit_dice_pool(d);
                self.visit_number(n);
            }
            // 处理 Option 类型
            Explode(d, mp, lim) | CompoundExplode(d, mp, lim) => {
                self.visit_dice_pool(d);
                if let Some(m) = mp {
                    self.visit_mod_param(m);
                }
                if let Some(l) = lim {
                    self.visit_limit(l);
                }
            }
            Reroll(d, mp, lim) => {
                self.visit_dice_pool(d);
                self.visit_mod_param(mp);
                if let Some(l) = lim {
                    self.visit_limit(l);
                }
            }
            SubtractFailures(d, mp) => {
                self.visit_dice_pool(d);
                self.visit_mod_param(mp);
            }
        }
    }

    // ==========================================
    // SuccessPoolType
    // ==========================================
    fn visit_success_pool(&mut self, s: &mut SuccessPoolType) {
        self.visit_success_pool_children(s);
        self.visit_success_pool_self(s);
    }

    fn visit_success_pool_self(&mut self, _s: &mut SuccessPoolType) {}

    fn visit_success_pool_children(&mut self, s: &mut SuccessPoolType) {
        use SuccessPoolType::*;
        match s {
            CountSuccessesFromDicePool(d, mp) | DeductFailuresFromDicePool(d, mp) => {
                self.visit_dice_pool(d);
                self.visit_mod_param(mp);
            }
            CountSuccesses(sp, mp) | DeductFailures(sp, mp) => {
                self.visit_success_pool(sp);
                self.visit_mod_param(mp);
            }
        }
    }

    // ==========================================
    // NumberBinaryType
    // ==========================================
    fn visit_number_binary(&mut self, nb: &mut NumberBinaryType) {
        self.visit_number_binary_children(nb);
        self.visit_number_binary_self(nb);
    }

    fn visit_number_binary_self(&mut self, _nb: &mut NumberBinaryType) {}

    fn visit_number_binary_children(&mut self, nb: &mut NumberBinaryType) {
        use NumberBinaryType::*;
        match nb {
            Add(l, r)
            | Subtract(l, r)
            | Multiply(l, r)
            | Divide(l, r)
            | IntDivide(l, r)
            | Modulo(l, r) => {
                self.visit_number(l);
                self.visit_number(r);
            }
        }
    }

    // ==========================================
    // NumberFunctionType
    // ==========================================
    fn visit_number_function(&mut self, nf: &mut NumberFunctionType) {
        self.visit_number_function_children(nf);
        self.visit_number_function_self(nf);
    }

    fn visit_number_function_self(&mut self, _nf: &mut NumberFunctionType) {}

    fn visit_number_function_children(&mut self, nf: &mut NumberFunctionType) {
        use NumberFunctionType::*;
        match nf {
            Floor(n) | Ceil(n) | Round(n) | Abs(n) => self.visit_number(n),
            // 这些函数内部包含 ListType，调用 visit_list
            Max(l) | Min(l) | Sum(l) | Avg(l) | Len(l) => self.visit_list(l),
        }
    }

    // ==========================================
    // ListType
    // ==========================================
    fn visit_list(&mut self, l: &mut ListType) {
        self.visit_list_children(l);
        self.visit_list_self(l);
    }

    fn visit_list_self(&mut self, _l: &mut ListType) {}

    fn visit_list_children(&mut self, l: &mut ListType) {
        use ListType::*;
        match l {
            Explicit(vec) => {
                // 使用 iter_mut 遍历 Vec
                for n in vec.iter_mut() {
                    self.visit_number(n);
                }
            }
            ListFunction(lf) => self.visit_list_function(lf),
            ListBinary(lb) => self.visit_list_binary(lb),
        }
    }

    // ==========================================
    // ListBinaryType
    // ==========================================
    fn visit_list_binary(&mut self, lb: &mut ListBinaryType) {
        self.visit_list_binary_children(lb);
        self.visit_list_binary_self(lb);
    }

    fn visit_list_binary_self(&mut self, _lb: &mut ListBinaryType) {}

    fn visit_list_binary_children(&mut self, lb: &mut ListBinaryType) {
        use ListBinaryType::*;
        match lb {
            AddList(l1, l2) => {
                self.visit_list(l1);
                self.visit_list(l2);
            }
            MultiplyList(l, n)
            | Add(l, n)
            | Multiply(l, n)
            | Subtract(l, n)
            | Divide(l, n)
            | IntDivide(l, n)
            | Modulo(l, n) => {
                self.visit_list(l);
                self.visit_number(n);
            }
            SubtractReverse(n, l)
            | DivideReverse(n, l)
            | IntDivideReverse(n, l)
            | ModuloReverse(n, l) => {
                self.visit_number(n);
                self.visit_list(l);
            }
        }
    }

    // ==========================================
    // ListFunctionType
    // ==========================================
    fn visit_list_function(&mut self, lf: &mut ListFunctionType) {
        self.visit_list_function_children(lf);
        self.visit_list_function_self(lf);
    }

    fn visit_list_function_self(&mut self, _lf: &mut ListFunctionType) {}

    fn visit_list_function_children(&mut self, lf: &mut ListFunctionType) {
        use ListFunctionType::*;
        match lf {
            Floor(l) | Ceil(l) | Round(l) | Abs(l) | Sort(l) | SortDesc(l) => {
                self.visit_list(l);
            }
            Max(l, n) | Min(l, n) => {
                self.visit_list(l);
                self.visit_number(n);
            }
            ToListFromDicePool(d) => self.visit_dice_pool(d),
            ToListFromSuccessPool(s) => self.visit_success_pool(s),
            Filter(l, mp) => {
                self.visit_list(l);
                self.visit_mod_param(mp);
            }
        }
    }

    // ==========================================
    // 辅助结构 (ModParam, Limit)
    // ==========================================

    fn visit_mod_param(&mut self, mp: &mut ModParam) {
        // mp.value 是 Box<NumberType>
        self.visit_number(&mut mp.value);
    }

    fn visit_limit(&mut self, lim: &mut Limit) {
        if let Some(n) = &mut lim.limit_times {
            self.visit_number(n);
        }
        if let Some(n) = &mut lim.limit_counts {
            self.visit_number(n);
        }
    }
}
