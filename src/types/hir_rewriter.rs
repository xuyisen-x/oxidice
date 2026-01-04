use super::hir::*;

// ==========================================
// 遍历辅助结构 (Mutable Visitor Pattern)
// ==========================================

pub trait HirVisitor {
    // ==========================================
    // 顶层入口 (HIR)
    // ==========================================
    fn visit_hir(&mut self, h: &mut HIR) -> Result<(), String> {
        self.visit_hir_children(h)?;
        self.visit_hir_self(h)?;
        Ok(())
    }

    fn visit_hir_self(&mut self, _h: &mut HIR) -> Result<(), String> {
        Ok(())
    }

    fn visit_hir_children(&mut self, h: &mut HIR) -> Result<(), String> {
        match h {
            HIR::Number(n) => self.visit_number(n),
            HIR::List(l) => self.visit_list(l),
        }
    }

    // ==========================================
    // NumberType
    // ==========================================
    fn visit_number(&mut self, n: &mut NumberType) -> Result<(), String> {
        self.visit_number_children(n)?;
        self.visit_number_self(n)?;
        Ok(())
    }

    fn visit_number_self(&mut self, _n: &mut NumberType) -> Result<(), String> {
        Ok(())
    }

    fn visit_number_children(&mut self, n: &mut NumberType) -> Result<(), String> {
        use NumberType::*;
        match n {
            Constant(_) => Ok(()), // 叶子节点，无需递归
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
    fn visit_dice_pool(&mut self, d: &mut DicePoolType) -> Result<(), String> {
        self.visit_dice_pool_children(d)?;
        self.visit_dice_pool_self(d)?;
        Ok(())
    }

    fn visit_dice_pool_self(&mut self, _d: &mut DicePoolType) -> Result<(), String> {
        Ok(())
    }

    fn visit_dice_pool_children(&mut self, d: &mut DicePoolType) -> Result<(), String> {
        use DicePoolType::*;
        match d {
            Standard(x, y) => {
                self.visit_number(x)?;
                self.visit_number(y)?;
                Ok(())
            }
            Fudge(x) => self.visit_number(x),
            Coin(x) => self.visit_number(x),
            KeepHigh(d, n)
            | KeepLow(d, n)
            | DropHigh(d, n)
            | DropLow(d, n)
            | Min(d, n)
            | Max(d, n) => {
                self.visit_dice_pool(d)?;
                self.visit_number(n)?;
                Ok(())
            }
            // 处理 Option 类型
            Explode(d, mp, lim) | CompoundExplode(d, mp, lim) => {
                self.visit_dice_pool(d)?;
                if let Some(m) = mp {
                    self.visit_mod_param(m)?;
                }
                if let Some(l) = lim {
                    self.visit_limit(l)?;
                }
                Ok(())
            }
            Reroll(d, mp, lim) => {
                self.visit_dice_pool(d)?;
                self.visit_mod_param(mp)?;
                if let Some(l) = lim {
                    self.visit_limit(l)?;
                }
                Ok(())
            }
            SubtractFailures(d, mp) => {
                self.visit_dice_pool(d)?;
                self.visit_mod_param(mp)?;
                Ok(())
            }
        }
    }

    // ==========================================
    // SuccessPoolType
    // ==========================================
    fn visit_success_pool(&mut self, s: &mut SuccessPoolType) -> Result<(), String> {
        self.visit_success_pool_children(s)?;
        self.visit_success_pool_self(s)?;
        Ok(())
    }

    fn visit_success_pool_self(&mut self, _s: &mut SuccessPoolType) -> Result<(), String> {
        Ok(())
    }

    fn visit_success_pool_children(&mut self, s: &mut SuccessPoolType) -> Result<(), String> {
        use SuccessPoolType::*;
        match s {
            CountSuccessesFromDicePool(d, mp) | DeductFailuresFromDicePool(d, mp) => {
                self.visit_dice_pool(d)?;
                self.visit_mod_param(mp)?;
                Ok(())
            }
            CountSuccesses(sp, mp) | DeductFailures(sp, mp) => {
                self.visit_success_pool(sp)?;
                self.visit_mod_param(mp)?;
                Ok(())
            }
        }
    }

    // ==========================================
    // NumberBinaryType
    // ==========================================
    fn visit_number_binary(&mut self, nb: &mut NumberBinaryType) -> Result<(), String> {
        self.visit_number_binary_children(nb)?;
        self.visit_number_binary_self(nb)?;
        Ok(())
    }

    fn visit_number_binary_self(&mut self, _nb: &mut NumberBinaryType) -> Result<(), String> {
        Ok(())
    }

    fn visit_number_binary_children(&mut self, nb: &mut NumberBinaryType) -> Result<(), String> {
        use NumberBinaryType::*;
        match nb {
            Add(l, r)
            | Subtract(l, r)
            | Multiply(l, r)
            | Divide(l, r)
            | IntDivide(l, r)
            | Modulo(l, r) => {
                self.visit_number(l)?;
                self.visit_number(r)?;
                Ok(())
            }
        }
    }

    // ==========================================
    // NumberFunctionType
    // ==========================================
    fn visit_number_function(&mut self, nf: &mut NumberFunctionType) -> Result<(), String> {
        self.visit_number_function_children(nf)?;
        self.visit_number_function_self(nf)?;
        Ok(())
    }

    fn visit_number_function_self(&mut self, _nf: &mut NumberFunctionType) -> Result<(), String> {
        Ok(())
    }

    fn visit_number_function_children(
        &mut self,
        nf: &mut NumberFunctionType,
    ) -> Result<(), String> {
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
    fn visit_list(&mut self, l: &mut ListType) -> Result<(), String> {
        self.visit_list_children(l)?;
        self.visit_list_self(l)?;
        Ok(())
    }

    fn visit_list_self(&mut self, _l: &mut ListType) -> Result<(), String> {
        Ok(())
    }

    fn visit_list_children(&mut self, l: &mut ListType) -> Result<(), String> {
        use ListType::*;
        match l {
            Explicit(vec) => {
                // 使用 iter_mut 遍历 Vec
                for n in vec.iter_mut() {
                    self.visit_number(n)?;
                }
                Ok(())
            }
            ListFunction(lf) => self.visit_list_function(lf),
            ListBinary(lb) => self.visit_list_binary(lb),
        }
    }

    // ==========================================
    // ListBinaryType
    // ==========================================
    fn visit_list_binary(&mut self, lb: &mut ListBinaryType) -> Result<(), String> {
        self.visit_list_binary_children(lb)?;
        self.visit_list_binary_self(lb)?;
        Ok(())
    }

    fn visit_list_binary_self(&mut self, _lb: &mut ListBinaryType) -> Result<(), String> {
        Ok(())
    }

    fn visit_list_binary_children(&mut self, lb: &mut ListBinaryType) -> Result<(), String> {
        use ListBinaryType::*;
        match lb {
            AddList(l1, l2) => {
                self.visit_list(l1)?;
                self.visit_list(l2)?;
                Ok(())
            }
            Add(l, n)
            | Multiply(l, n)
            | Subtract(l, n)
            | Divide(l, n)
            | IntDivide(l, n)
            | Modulo(l, n) => {
                self.visit_list(l)?;
                self.visit_number(n)?;
                Ok(())
            }
            SubtractReverse(n, l)
            | DivideReverse(n, l)
            | IntDivideReverse(n, l)
            | ModuloReverse(n, l) => {
                self.visit_number(n)?;
                self.visit_list(l)?;
                Ok(())
            }
        }
    }

    // ==========================================
    // ListFunctionType
    // ==========================================
    fn visit_list_function(&mut self, lf: &mut ListFunctionType) -> Result<(), String> {
        self.visit_list_function_children(lf)?;
        self.visit_list_function_self(lf)?;
        Ok(())
    }

    fn visit_list_function_self(&mut self, _lf: &mut ListFunctionType) -> Result<(), String> {
        Ok(())
    }

    fn visit_list_function_children(&mut self, lf: &mut ListFunctionType) -> Result<(), String> {
        use ListFunctionType::*;
        match lf {
            Floor(l) | Ceil(l) | Round(l) | Abs(l) | Sort(l) | SortDesc(l) => {
                self.visit_list(l)?;
                Ok(())
            }
            Max(l, n) | Min(l, n) => {
                self.visit_list(l)?;
                self.visit_number(n)?;
                Ok(())
            }
            ToListFromDicePool(d) => self.visit_dice_pool(d),
            ToListFromSuccessPool(s) => self.visit_success_pool(s),
            Filter(l, mp) => {
                self.visit_list(l)?;
                self.visit_mod_param(mp)?;
                Ok(())
            }
        }
    }

    // ==========================================
    // 辅助结构 (ModParam, Limit)
    // ==========================================

    fn visit_mod_param(&mut self, mp: &mut ModParam) -> Result<(), String> {
        // mp.value 是 Box<NumberType>
        self.visit_number(&mut mp.value)?;
        Ok(())
    }

    fn visit_limit(&mut self, lim: &mut Limit) -> Result<(), String> {
        if let Some(n) = &mut lim.limit_times {
            self.visit_number(n)?;
        }
        if let Some(n) = &mut lim.limit_counts {
            self.visit_number(n)?;
        }
        Ok(())
    }
}
