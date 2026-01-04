use crate::types::eval_graph::*;
use crate::types::hir::*;

// 编译函数 HIR -> EvalGraph
pub fn compile_hir_to_eval_graph(hir: HIR) -> EvalGraph {
    let compiler = Compiler::new();
    compiler.compile(hir)
}

struct Compiler {
    nodes: Vec<EvalNode>,
}

impl Compiler {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    // 入口函数，将 HIR 编译为 EvalGraph
    pub fn compile(mut self, hir: HIR) -> EvalGraph {
        let root = match hir {
            HIR::Number(n) => self.compile_number(n),
            HIR::List(l) => self.compile_list(l),
        };

        EvalGraph {
            nodes: self.nodes,
            root,
        }
    }
    // 辅助函数：添加节点并返回其 NodeId
    fn push(&mut self, node: EvalNode) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    // ==========================================
    // 编译 NumberType
    // ==========================================
    fn compile_number(&mut self, num: NumberType) -> NodeId {
        match num {
            NumberType::Constant(v) => self.push(EvalNode::Constant(v)),
            NumberType::DicePool(pool) => self.compile_dice_pool(pool),
            NumberType::SuccessPool(pool) => self.compile_success_pool(pool),
            NumberType::NumberBinary(bin) => match bin {
                NumberBinaryType::Add(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumAdd(lid, rid))
                }
                NumberBinaryType::Subtract(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumSubtract(lid, rid))
                }
                NumberBinaryType::Multiply(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumMultiply(lid, rid))
                }
                NumberBinaryType::Divide(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumDivide(lid, rid))
                }
                NumberBinaryType::IntDivide(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumIntDivide(lid, rid))
                }
                NumberBinaryType::Modulo(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::NumModulo(lid, rid))
                }
            },
            NumberType::NumberFunction(func) => match func {
                NumberFunctionType::Floor(n) => {
                    let id = self.compile_number(*n);
                    self.push(EvalNode::NumFloor(id))
                }
                NumberFunctionType::Ceil(n) => {
                    let id = self.compile_number(*n);
                    self.push(EvalNode::NumCeil(id))
                }
                NumberFunctionType::Round(n) => {
                    let id = self.compile_number(*n);
                    self.push(EvalNode::NumRound(id))
                }
                NumberFunctionType::Abs(n) => {
                    let id = self.compile_number(*n);
                    self.push(EvalNode::NumAbs(id))
                }
                NumberFunctionType::Max(list) => {
                    let id = self.compile_list(*list);
                    self.push(EvalNode::NumMax(id))
                }
                NumberFunctionType::Min(list) => {
                    let id = self.compile_list(*list);
                    self.push(EvalNode::NumMin(id))
                }
                NumberFunctionType::Sum(list) => {
                    let id = self.compile_list(*list);
                    self.push(EvalNode::NumSum(id))
                }
                NumberFunctionType::Avg(list) => {
                    let id = self.compile_list(*list);
                    self.push(EvalNode::NumAvg(id))
                }
                NumberFunctionType::Len(list) => {
                    let id = self.compile_list(*list);
                    self.push(EvalNode::NumLen(id))
                }
            },
            NumberType::Neg(n) => {
                let nid = self.compile_number(*n);
                self.push(EvalNode::NumNegate(nid))
            }
        }
    }

    // ==========================================
    // 编译 ListType
    // ==========================================
    fn compile_list(&mut self, list: ListType) -> NodeId {
        match list {
            ListType::Explicit(elements) => {
                let ids = elements
                    .into_iter()
                    .map(|e| self.compile_number(e))
                    .collect();
                self.push(EvalNode::ListConstruct(ids))
            }
            ListType::ListBinary(bin) => match bin {
                ListBinaryType::AddList(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_list(*r);
                    self.push(EvalNode::Concat(lid, rid))
                }
                ListBinaryType::Add(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListAdd(lid, rid))
                }
                ListBinaryType::Multiply(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListMultiply(lid, rid))
                }
                ListBinaryType::Subtract(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListSubtract(lid, rid))
                }
                ListBinaryType::Divide(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListDivide(lid, rid))
                }
                ListBinaryType::IntDivide(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListIntDivide(lid, rid))
                }
                ListBinaryType::Modulo(l, r) => {
                    let lid = self.compile_list(*l);
                    let rid = self.compile_number(*r);
                    self.push(EvalNode::ListModulo(lid, rid))
                }
                ListBinaryType::SubtractReverse(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_list(*r);
                    self.push(EvalNode::ListSubtractReverse(lid, rid))
                }
                ListBinaryType::DivideReverse(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_list(*r);
                    self.push(EvalNode::ListDivideReverse(lid, rid))
                }
                ListBinaryType::IntDivideReverse(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_list(*r);
                    self.push(EvalNode::ListIntDivideReverse(lid, rid))
                }
                ListBinaryType::ModuloReverse(l, r) => {
                    let lid = self.compile_number(*l);
                    let rid = self.compile_list(*r);
                    self.push(EvalNode::ListModuloReverse(lid, rid))
                }
            },
            ListType::ListFunction(func) => match func {
                ListFunctionType::Floor(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListFloor(lid))
                }
                ListFunctionType::Ceil(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListCeil(lid))
                }
                ListFunctionType::Round(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListRound(lid))
                }
                ListFunctionType::Abs(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListAbs(lid))
                }
                ListFunctionType::Max(list, count) => {
                    let lid = self.compile_list(*list);
                    let cid = self.compile_number(*count);
                    self.push(EvalNode::ListMax(lid, cid))
                }
                ListFunctionType::Min(list, count) => {
                    let lid = self.compile_list(*list);
                    let cid = self.compile_number(*count);
                    self.push(EvalNode::ListMin(lid, cid))
                }
                ListFunctionType::Sort(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListSort(lid))
                }
                ListFunctionType::SortDesc(list) => {
                    let lid = self.compile_list(*list);
                    self.push(EvalNode::ListSortDesc(lid))
                }
                ListFunctionType::ToListFromDicePool(dpool) => {
                    let dpid = self.compile_dice_pool(*dpool);
                    self.push(EvalNode::ListToListFromDicePool(dpid))
                }
                ListFunctionType::ToListFromSuccessPool(spool) => {
                    let spid = self.compile_success_pool(*spool);
                    self.push(EvalNode::ListToListFromSuccessPool(spid))
                }
                ListFunctionType::Filter(list, param) => {
                    let lid = self.compile_list(*list);
                    let param_node = self.compile_mod_param(param);
                    self.push(EvalNode::ListFilter(lid, param_node))
                }
            },
        }
    }

    // ==========================================
    // 编译 DicePoolType
    // ==========================================
    fn compile_dice_pool(&mut self, pool: DicePoolType) -> NodeId {
        match pool {
            DicePoolType::Standard(count, sides) => {
                let c = self.compile_number(*count);
                let s = self.compile_number(*sides);
                self.push(EvalNode::DiceStandard(c, s))
            }
            DicePoolType::Fudge(count) => {
                let c = self.compile_number(*count);
                self.push(EvalNode::DiceFudge(c))
            }
            DicePoolType::Coin(count) => {
                let c = self.compile_number(*count);
                self.push(EvalNode::DiceCoin(c))
            }
            DicePoolType::KeepHigh(pool, count) => {
                let source = self.compile_dice_pool(*pool);
                let param = self.compile_number(*count);
                self.push(EvalNode::DiceKeepHigh(source, param))
            }
            DicePoolType::DropLow(pool, count) => {
                let source = self.compile_dice_pool(*pool);
                let param = self.compile_number(*count);
                self.push(EvalNode::DiceDropLow(source, param))
            }
            DicePoolType::DropHigh(pool, count) => {
                let source = self.compile_dice_pool(*pool);
                let param = self.compile_number(*count);
                self.push(EvalNode::DiceDropHigh(source, param))
            }
            DicePoolType::KeepLow(pool, count) => {
                let source = self.compile_dice_pool(*pool);
                let param = self.compile_number(*count);
                self.push(EvalNode::DiceKeepLow(source, param))
            }
            DicePoolType::Max(pool, target) => {
                let source = self.compile_dice_pool(*pool);
                let t = self.compile_number(*target);
                self.push(EvalNode::DiceMax(source, t))
            }
            DicePoolType::Min(pool, target) => {
                let source = self.compile_dice_pool(*pool);
                let t = self.compile_number(*target);
                self.push(EvalNode::DiceMin(source, t))
            }
            DicePoolType::Explode(pool, param, limit) => {
                let source = self.compile_dice_pool(*pool);
                let p = param.map(|x| self.compile_mod_param(x));
                let l = limit.map(|x| self.compile_limit(x));
                self.push(EvalNode::DiceExplode(source, p, l))
            }
            DicePoolType::CompoundExplode(pool, param, limit) => {
                let source = self.compile_dice_pool(*pool);
                let p = param.map(|x| self.compile_mod_param(x));
                let l = limit.map(|x| self.compile_limit(x));
                self.push(EvalNode::DiceCompoundExplode(source, p, l))
            }
            DicePoolType::Reroll(pool, param, limit) => {
                let source = self.compile_dice_pool(*pool);
                let p = self.compile_mod_param(param);
                let l = limit.map(|x| self.compile_limit(x));
                self.push(EvalNode::DiceReroll(source, p, l))
            }
            DicePoolType::SubtractFailures(pool, param) => {
                let source = self.compile_dice_pool(*pool);
                let p = self.compile_mod_param(param);
                self.push(EvalNode::DiceSubtractFailures(source, p))
            }
        }
    }

    // ==========================================
    // 编译 SuccessPoolType
    // ==========================================
    fn compile_success_pool(&mut self, pool: SuccessPoolType) -> NodeId {
        match pool {
            SuccessPoolType::CountSuccessesFromDicePool(dice_pool, param) => {
                let source = self.compile_dice_pool(*dice_pool);
                let p = self.compile_mod_param(param);
                self.push(EvalNode::DiceCountSuccessesFromDicePool(source, p))
            }
            SuccessPoolType::DeductFailuresFromDicePool(dice_pool, param) => {
                let source = self.compile_dice_pool(*dice_pool);
                let p = self.compile_mod_param(param);
                self.push(EvalNode::DiceDeductFailuresFromDicePool(source, p))
            }
            SuccessPoolType::CountSuccesses(success_pool, param) => {
                let source = self.compile_success_pool(*success_pool);
                let p = self.compile_mod_param(param);
                self.push(EvalNode::DiceCountSuccesses(source, p))
            }
            SuccessPoolType::DeductFailures(success_pool, param) => {
                let source = self.compile_success_pool(*success_pool);
                let p = self.compile_mod_param(param);
                self.push(EvalNode::DiceDeductFailures(source, p))
            }
        }
    }

    // ==========================================
    // 辅助：编译 ModParam 和 Limit
    // ==========================================
    fn compile_mod_param(&mut self, param: ModParam) -> ModParamNode {
        let val_id = self.compile_number(*param.value);
        ModParamNode {
            operator: param.operator,
            value: val_id,
        }
    }
    fn compile_limit(&mut self, limit: Limit) -> LimitNode {
        let t = limit.limit_times.map(|n| self.compile_number(*n));
        let c = limit.limit_counts.map(|n| self.compile_number(*n));
        LimitNode {
            limit_times: t,
            limit_counts: c,
        }
    }
}
