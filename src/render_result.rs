use crate::types::eval_graph::*;
use crate::types::output_node::*;
use crate::types::runtime_value::*;

pub fn render_result(graph: &EvalGraph, memory: &[NodeState]) -> OutputNode {
    let builder = ResultTreeBuilder::new(graph, memory);
    builder.build()
}

struct ResultTreeBuilder<'a> {
    graph: &'a EvalGraph,
    memory: &'a [NodeState],
}

impl<'a> ResultTreeBuilder<'a> {
    fn get_value_summary(&self, idx: usize) -> ValueSummary {
        match &self.memory[idx] {
            NodeState::Computed(rv) => match rv {
                RuntimeValue::Number(n) => ValueSummary::Number(*n),
                RuntimeValue::List(l) => ValueSummary::List(l.clone()),
                RuntimeValue::DicePool(dp) => ValueSummary::DicePool {
                    total: dp.total,
                    face: match dp.face {
                        DiceFace::Number(f) => DiceFaceType::Standard(f),
                        DiceFace::Fudge => DiceFaceType::Fudge,
                        DiceFace::Coin => DiceFaceType::Coin,
                    },
                    details: dp.details.iter().map(|d| self.convert_detail(d)).collect(),
                },
                RuntimeValue::SuccessPool(sp) => ValueSummary::SuccessPool {
                    count: sp.success_count,
                    face: match sp.face {
                        DiceFace::Number(f) => DiceFaceType::Standard(f),
                        DiceFace::Fudge => DiceFaceType::Fudge,
                        DiceFace::Coin => DiceFaceType::Coin,
                    },
                    details: sp.details.iter().map(|d| self.convert_detail(d)).collect(),
                },
            },
            _ => ValueSummary::NotComputed,
        }
    }

    fn convert_detail(&self, d: &DieDetail) -> DieDetailSummary {
        DieDetailSummary {
            result: d.result,
            is_kept: d.is_kept,
            roll_history: d.roll_history.clone(),
            is_rerolled: d.is_rerolled,
            exploded_times: d.exploded_times,
            outcome: match d.outcome {
                DieOutcome::Success => OutcomeType::Success,
                DieOutcome::Failure => OutcomeType::Failure,
                DieOutcome::None => OutcomeType::None,
            },
        }
    }
}

impl<'a> ResultTreeBuilder<'a> {
    pub fn new(graph: &'a EvalGraph, memory: &'a [NodeState]) -> Self {
        Self { graph, memory }
    }

    pub fn build(&self) -> OutputNode {
        // 根节点优先级为 None，且不视为右子节点
        let (node, _) = self.build_recursive(self.graph.root);
        node
    }

    fn build_recursive(&self, node_id: NodeId) -> (OutputNode, Precedence) {
        let idx = node_id.to_index();
        let eval_node = &self.graph.nodes[idx];
        // 获取当前节点的值摘要
        let value_summary = self.get_value_summary(idx);

        let (label, layout, current_prec) = match eval_node {
            // 原子节点
            EvalNode::Constant(n) => (n.to_string(), NodeLayout::Atom, Precedence::Call),
            EvalNode::ListConstruct(ids) => {
                let children = ids.iter().map(|id| self.build_recursive(*id).0).collect();
                ("".to_string(), NodeLayout::List(children), Precedence::Call)
            }
            // 单目运算
            EvalNode::NumNegate(id) => {
                let prec = Precedence::Prefix;
                let (mut child, child_prec) = self.build_recursive(*id);
                if child_prec < prec {
                    child.wrap_in_parentheses = true;
                }
                ("-".to_string(), NodeLayout::Prefix(Box::new(child)), prec)
            }
            // 数字二元运算
            EvalNode::NumAdd(l, r) => self.math_infix("+", *l, *r, Precedence::Sum),
            EvalNode::NumSubtract(l, r) => self.math_infix("-", *l, *r, Precedence::Sum),
            EvalNode::NumMultiply(l, r) => self.math_infix("*", *l, *r, Precedence::Product),
            EvalNode::NumDivide(l, r) => self.math_infix("/", *l, *r, Precedence::Product),
            EvalNode::NumIntDivide(l, r) => self.math_infix("//", *l, *r, Precedence::Product),
            EvalNode::NumModulo(l, r) => self.math_infix("%", *l, *r, Precedence::Product),
            // 列表二元运算
            EvalNode::Concat(l, r) => self.list_infix("+", *l, *r, Precedence::Sum),
            EvalNode::ListAdd(l, r) => self.list_infix("+", *l, *r, Precedence::Sum),
            EvalNode::ListSubtract(l, r) | EvalNode::ListSubtractReverse(l, r) => {
                self.list_infix("-", *l, *r, Precedence::Sum)
            }
            EvalNode::ListMultiply(l, r) => self.list_infix("*", *l, *r, Precedence::Product),
            EvalNode::ListDivide(l, r) | EvalNode::ListDivideReverse(l, r) => {
                self.list_infix("/", *l, *r, Precedence::Product)
            }
            EvalNode::ListIntDivide(l, r) | EvalNode::ListIntDivideReverse(l, r) => {
                self.list_infix("//", *l, *r, Precedence::Product)
            }
            EvalNode::ListModulo(l, r) | EvalNode::ListModuloReverse(l, r) => {
                self.list_infix("%", *l, *r, Precedence::Product)
            }
            // 普通函数调用
            EvalNode::NumFloor(id) | EvalNode::ListFloor(id) => self.func("floor", vec![*id]),
            EvalNode::NumCeil(id) | EvalNode::ListCeil(id) => self.func("ceil", vec![*id]),
            EvalNode::NumRound(id) | EvalNode::ListRound(id) => self.func("round", vec![*id]),
            EvalNode::NumAbs(id) | EvalNode::ListAbs(id) => self.func("abs", vec![*id]),
            EvalNode::NumMax(id) => self.func("max", vec![*id]),
            EvalNode::NumMin(id) => self.func("min", vec![*id]),
            EvalNode::NumSum(id) => self.func("sum", vec![*id]),
            EvalNode::NumAvg(id) => self.func("avg", vec![*id]),
            EvalNode::NumLen(id) => self.func("len", vec![*id]),
            EvalNode::ListMax(id1, id2) => self.func("max", vec![*id1, *id2]),
            EvalNode::ListMin(id1, id2) => self.func("min", vec![*id1, *id2]),
            EvalNode::ListSort(id) => self.func("sort", vec![*id]),
            EvalNode::ListSortDesc(id) => self.func("sortd", vec![*id]),
            EvalNode::ListToListFromDicePool(id) | EvalNode::ListToListFromSuccessPool(id) => {
                self.func("tolist", vec![*id])
            }
            // Filter函数调用
            EvalNode::ListFilter(l, mp) => {
                let prec = Precedence::Call;
                let (list_node, _) = self.build_recursive(*l);
                let (mut val_node, val_prec) = self.build_recursive(mp.value);
                // 如果值节点优先级低于函数调用优先级，则加括号
                if val_prec < prec {
                    val_node.wrap_in_parentheses = true;
                }
                (
                    "filter".to_string(),
                    NodeLayout::Filter(
                        Box::new(mp.operator.to_string()),
                        Box::new(list_node),
                        Box::new(val_node),
                    ),
                    Precedence::Call,
                )
            }
            // 3个基础骰子类型
            EvalNode::DiceStandard(count, sides) => {
                let prec = Precedence::Dice;
                let (mut l_node, l_prec) = self.build_recursive(*count);
                let (mut r_node, r_prec) = self.build_recursive(*sides);

                if l_prec <= prec {
                    l_node.wrap_in_parentheses = true;
                }
                if r_prec <= prec {
                    r_node.wrap_in_parentheses = true;
                }

                (
                    "d".to_string(),
                    NodeLayout::TightInfix(Box::new(l_node), Box::new(r_node)),
                    prec,
                )
            }
            EvalNode::DiceFudge(count) => {
                let prec = Precedence::Dice;
                let (mut child, c_prec) = self.build_recursive(*count);
                if c_prec <= prec {
                    child.wrap_in_parentheses = true;
                }
                (
                    "dF".to_string(),
                    NodeLayout::TightPostfix(Box::new(child)),
                    prec,
                )
            }
            EvalNode::DiceCoin(count) => {
                let prec = Precedence::Dice;
                let (mut child, c_prec) = self.build_recursive(*count);
                if c_prec <= prec {
                    child.wrap_in_parentheses = true;
                }
                (
                    "dC".to_string(),
                    NodeLayout::TightPostfix(Box::new(child)),
                    prec,
                )
            }
            EvalNode::DiceKeepHigh(p, n) => self.simple_dice_mod("kh", *p, *n),
            EvalNode::DiceKeepLow(p, n) => self.simple_dice_mod("kl", *p, *n),
            EvalNode::DiceDropHigh(p, n) => self.simple_dice_mod("dh", *p, *n),
            EvalNode::DiceDropLow(p, n) => self.simple_dice_mod("dl", *p, *n),
            EvalNode::DiceMin(p, n) => self.simple_dice_mod("min", *p, *n),
            EvalNode::DiceMax(p, n) => self.simple_dice_mod("max", *p, *n),
            EvalNode::DiceCountSuccesses(p, mp)
            | EvalNode::DiceCountSuccessesFromDicePool(p, mp) => {
                let op = format!("cs{}", mp.operator);
                self.simple_dice_mod(&op, *p, mp.value)
            }
            EvalNode::DiceDeductFailures(p, mp)
            | EvalNode::DiceDeductFailuresFromDicePool(p, mp) => {
                let op = format!("df{}", mp.operator);
                self.simple_dice_mod(&op, *p, mp.value)
            }
            EvalNode::DiceSubtractFailures(p, mp) => {
                let op = format!("sf{}", mp.operator);
                self.simple_dice_mod(&op, *p, mp.value)
            }
            EvalNode::DiceExplode(pool, mp, limit) => self.explode("!", *pool, mp, limit),
            EvalNode::DiceCompoundExplode(pool, mp, limit) => self.explode("!!", *pool, mp, limit),
            EvalNode::DiceReroll(pool, mp, limit) => self.reroll("r", *pool, mp, limit),
        };

        let node = OutputNode {
            id: node_id.0,
            label: label,
            value: value_summary,
            layout: layout,
            wrap_in_parentheses: false, // 自己不加括号，是否加括号由父节点决定
        };

        (node, current_prec)
    }

    fn math_infix(
        &self,
        op: &str,
        l: NodeId,
        r: NodeId,
        prec: Precedence,
    ) -> (String, NodeLayout, Precedence) {
        let (mut l_node, l_prec) = self.build_recursive(l);
        let (mut r_node, r_prec) = self.build_recursive(r);

        if l_prec < prec {
            l_node.wrap_in_parentheses = true;
        }
        if r_prec <= prec {
            r_node.wrap_in_parentheses = true;
        }

        (
            op.to_string(),
            NodeLayout::Infix(Box::new(l_node), Box::new(r_node)),
            prec,
        )
    }

    fn list_infix(
        &self,
        op: &str,
        l: NodeId,
        r: NodeId,
        prec: Precedence,
    ) -> (String, NodeLayout, Precedence) {
        let (mut l_node, l_prec) = self.build_recursive(l);
        let (mut r_node, r_prec) = self.build_recursive(r);

        if l_prec <= prec {
            l_node.wrap_in_parentheses = true;
        }
        if r_prec <= prec {
            r_node.wrap_in_parentheses = true;
        }

        (
            op.to_string(),
            NodeLayout::Infix(Box::new(l_node), Box::new(r_node)),
            prec,
        )
    }

    fn func(&self, name: &str, args: Vec<NodeId>) -> (String, NodeLayout, Precedence) {
        let children = args.iter().map(|&id| self.build_recursive(id).0).collect();
        (
            name.to_string(),
            NodeLayout::Function(children),
            Precedence::Call,
        )
    }

    fn simple_dice_mod(&self, op: &str, l: NodeId, r: NodeId) -> (String, NodeLayout, Precedence) {
        let prec = Precedence::Dice;
        let (l_node, _) = self.build_recursive(l);
        let (mut r_node, r_prec) = self.build_recursive(r);
        // 左边一定是投池子，不会变，不需要判断，直接不加括号
        // 右边如果优先级小于等于骰子优先级，则加括号
        // 如：6d20kh(1d6)
        if r_prec <= prec {
            r_node.wrap_in_parentheses = true;
        }

        (
            op.to_string(),
            NodeLayout::TightInfix(Box::new(l_node), Box::new(r_node)),
            prec,
        )
    }

    fn explode(
        &self,
        label: &str,
        pool: NodeId,
        mp: &Option<ModParamNode>,
        limit: &Option<LimitNode>,
    ) -> (String, NodeLayout, Precedence) {
        let prec = Precedence::Dice;
        let (pool_node, _) = self.build_recursive(pool);
        // 骰子池始终不加括号

        // 取出比较参数，如果优先级低，加括号
        let mp_data = if let Some(m) = mp {
            let (mut val, val_prec) = self.build_recursive(m.value);
            if val_prec <= prec {
                val.wrap_in_parentheses = true;
            }
            Some(Box::new((m.operator.to_string(), val)))
        } else {
            None
        };

        let (mut lt, mut lc) = (None, None);
        if let Some(l) = limit {
            if let Some(id) = l.limit_times {
                let (mut raw, raw_prec) = self.build_recursive(id);
                if raw_prec <= prec {
                    raw.wrap_in_parentheses = true;
                }
                lt = Some(Box::new(raw));
            }
            if let Some(id) = l.limit_counts {
                let (mut raw, raw_prec) = self.build_recursive(id);
                if raw_prec <= prec {
                    raw.wrap_in_parentheses = true;
                }
                lc = Some(Box::new(raw));
            }
        }

        (
            label.to_string(),
            NodeLayout::SpecialModifier(Box::new(pool_node), mp_data, lt, lc),
            prec,
        )
    }

    //为了避免clone，为reroll单独创建一个函数
    fn reroll(
        &self,
        label: &str,
        pool: NodeId,
        mp: &ModParamNode,
        limit: &Option<LimitNode>,
    ) -> (String, NodeLayout, Precedence) {
        let prec = Precedence::Dice;
        let (pool_node, _) = self.build_recursive(pool);
        // 骰子池始终不加括号

        // 取出比较参数，如果优先级低，加括号
        let (mut val, val_prec) = self.build_recursive(mp.value);
        if val_prec <= prec {
            val.wrap_in_parentheses = true;
        }
        let mp_data = Some(Box::new((mp.operator.to_string(), val)));

        let (mut lt, mut lc) = (None, None);
        if let Some(l) = limit {
            if let Some(id) = l.limit_times {
                let (mut raw, raw_prec) = self.build_recursive(id);
                if raw_prec <= prec {
                    raw.wrap_in_parentheses = true;
                }
                lt = Some(Box::new(raw));
            }
            if let Some(id) = l.limit_counts {
                let (mut raw, raw_prec) = self.build_recursive(id);
                if raw_prec <= prec {
                    raw.wrap_in_parentheses = true;
                }
                lc = Some(Box::new(raw));
            }
        }

        (
            label.to_string(),
            NodeLayout::SpecialModifier(Box::new(pool_node), mp_data, lt, lc),
            prec,
        )
    }
}
