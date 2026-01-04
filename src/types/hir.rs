use super::expr::CompareOp;

// ==========================================
// HIR: 高级中间表示 (High-level Intermediate Representation)
// ==========================================

#[derive(Debug, Clone, PartialEq)]
pub enum HIR {
    Number(NumberType),
    List(ListType),
}

// ==========================================
// 数字类型
// ==========================================

#[derive(Debug, Clone, PartialEq)]
pub enum NumberType {
    Constant(f64),
    DicePool(DicePoolType),
    SuccessPool(SuccessPoolType),
    NumberBinary(NumberBinaryType),
    NumberFunction(NumberFunctionType),
    Neg(Box<NumberType>), // 唯一一个单目运算符，就不单独定义枚举了
}

#[derive(Debug, Clone, PartialEq)]
pub enum DicePoolType {
    Standard(Box<NumberType>, Box<NumberType>),   // XdY
    Fudge(Box<NumberType>),                       // XdF
    Coin(Box<NumberType>),                        // XdC
    KeepHigh(Box<DicePoolType>, Box<NumberType>), // (XdY)khZ
    KeepLow(Box<DicePoolType>, Box<NumberType>),  // (XdY)kl
    DropHigh(Box<DicePoolType>, Box<NumberType>), // (XdY)dhZ
    DropLow(Box<DicePoolType>, Box<NumberType>),  // (XdY)dl
    Min(Box<DicePoolType>, Box<NumberType>),      // (XdY)minZ
    Max(Box<DicePoolType>, Box<NumberType>),      // (XdY)maxZ
    Explode(Box<DicePoolType>, Option<ModParam>, Option<Limit>), // (XdY)![mod_param][limit]
    CompoundExplode(Box<DicePoolType>, Option<ModParam>, Option<Limit>), // (XdY)!![mod_param][limit]
    Reroll(Box<DicePoolType>, ModParam, Option<Limit>),                  // (XdY)r[mod_param][limit]
    SubtractFailures(Box<DicePoolType>, ModParam),                       // (XdY)sfmod_param
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuccessPoolType {
    CountSuccessesFromDicePool(Box<DicePoolType>, ModParam), // success_pool_type cs dice_pool_type
    DeductFailuresFromDicePool(Box<DicePoolType>, ModParam), // success_pool_type df dice_pool_type
    CountSuccesses(Box<SuccessPoolType>, ModParam),          // success_pool_type cs mod_param
    DeductFailures(Box<SuccessPoolType>, ModParam),          // success_pool_type df mod_param
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberBinaryType {
    Add(Box<NumberType>, Box<NumberType>),
    Subtract(Box<NumberType>, Box<NumberType>),
    Multiply(Box<NumberType>, Box<NumberType>),
    Divide(Box<NumberType>, Box<NumberType>),
    IntDivide(Box<NumberType>, Box<NumberType>),
    Modulo(Box<NumberType>, Box<NumberType>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberFunctionType {
    Floor(Box<NumberType>),
    Ceil(Box<NumberType>),
    Round(Box<NumberType>),
    Abs(Box<NumberType>),
    Max(Box<ListType>),
    Min(Box<ListType>),
    Sum(Box<ListType>),
    Avg(Box<ListType>),
    Len(Box<ListType>),
}

// ==========================================
// 列表类型
// ==========================================

#[derive(Debug, Clone, PartialEq)]
pub enum ListType {
    Explicit(Vec<NumberType>),      // [num1, num2, num3, ...]
    ListFunction(ListFunctionType), // list_function_type
    ListBinary(ListBinaryType),     // list_binary_type
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListBinaryType {
    AddList(Box<ListType>, Box<ListType>), // list_type + list_type
    // broadcast operations between list and number
    Add(Box<ListType>, Box<NumberType>),
    Multiply(Box<ListType>, Box<NumberType>),
    // 顺序敏感的操作
    Subtract(Box<ListType>, Box<NumberType>),
    SubtractReverse(Box<NumberType>, Box<ListType>),
    Divide(Box<ListType>, Box<NumberType>),
    DivideReverse(Box<NumberType>, Box<ListType>),
    IntDivide(Box<ListType>, Box<NumberType>),
    IntDivideReverse(Box<NumberType>, Box<ListType>),
    Modulo(Box<ListType>, Box<NumberType>),
    ModuloReverse(Box<NumberType>, Box<ListType>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListFunctionType {
    Floor(Box<ListType>),                  // list_function_type floor number_type
    Ceil(Box<ListType>),                   // list_function_type ceil number_type
    Round(Box<ListType>),                  // list_function_type round number_type
    Abs(Box<ListType>),                    // list_function_type abs number_type
    Max(Box<ListType>, Box<NumberType>),   // list_function_type max number_type
    Min(Box<ListType>, Box<NumberType>),   // list_function_type min number_type
    Sort(Box<ListType>),                   // list_function_type sort
    SortDesc(Box<ListType>),               // list_function_type sortdesc
    ToListFromDicePool(Box<DicePoolType>), // tolist dice_pool_type
    ToListFromSuccessPool(Box<SuccessPoolType>), // tolist success_pool_type
    Filter(Box<ListType>, ModParam),       // list_function_type filter mod_param
}

// ==========================================
// 辅助类型
// ==========================================

#[derive(Debug, Clone, PartialEq)]
pub struct ModParam {
    pub operator: CompareOp,
    pub value: Box<NumberType>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub limit_times: Option<Box<NumberType>>,
    pub limit_counts: Option<Box<NumberType>>,
}

// ==========================================
// 构建辅助函数定义
// ==========================================

impl HIR {
    pub fn constant(value: f64) -> Self {
        HIR::Number(NumberType::Constant(value))
    }

    pub fn negate(num: NumberType) -> Self {
        HIR::Number(NumberType::Neg(Box::new(num)))
    }

    pub fn standard_dice_pool(count: NumberType, sides: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Standard(
            Box::new(count),
            Box::new(sides),
        )))
    }

    pub fn coin_dice_pool(count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Coin(Box::new(count))))
    }

    pub fn fudge_dice_pool(count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Fudge(Box::new(count))))
    }

    pub fn explicit_list(elements: Vec<NumberType>) -> Self {
        HIR::List(ListType::Explicit(elements))
    }

    pub fn add_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::Add(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn sub_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::Subtract(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn multiply_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::Multiply(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn divide_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::Divide(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn int_divide_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::IntDivide(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn modulo_number(left: NumberType, right: NumberType) -> Self {
        HIR::Number(NumberType::NumberBinary(NumberBinaryType::Modulo(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn add_list(left: ListType, right: ListType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::AddList(
            Box::new(left),
            Box::new(right),
        )))
    }

    pub fn add_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::Add(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn multiply_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::Multiply(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn sub_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::Subtract(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn sub_reverse_broadcast_list(number: NumberType, list: ListType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::SubtractReverse(
            Box::new(number),
            Box::new(list),
        )))
    }

    pub fn div_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::Divide(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn div_reverse_broadcast_list(number: NumberType, list: ListType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::DivideReverse(
            Box::new(number),
            Box::new(list),
        )))
    }

    pub fn idiv_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::IntDivide(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn idiv_reverse_broadcast_list(number: NumberType, list: ListType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::IntDivideReverse(
            Box::new(number),
            Box::new(list),
        )))
    }

    pub fn modulo_broadcast_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::Modulo(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn modulo_reverse_broadcast_list(number: NumberType, list: ListType) -> Self {
        HIR::List(ListType::ListBinary(ListBinaryType::ModuloReverse(
            Box::new(number),
            Box::new(list),
        )))
    }

    pub fn floor_number(num: NumberType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Floor(
            Box::new(num),
        )))
    }

    pub fn floor_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Floor(Box::new(
            list,
        ))))
    }

    pub fn ceil_number(num: NumberType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Ceil(
            Box::new(num),
        )))
    }

    pub fn ceil_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Ceil(Box::new(
            list,
        ))))
    }

    pub fn round_number(num: NumberType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Round(
            Box::new(num),
        )))
    }

    pub fn round_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Round(Box::new(
            list,
        ))))
    }

    pub fn abs_number(num: NumberType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Abs(
            Box::new(num),
        )))
    }

    pub fn abs_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Abs(Box::new(
            list,
        ))))
    }

    pub fn max_number(list: ListType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Max(
            Box::new(list),
        )))
    }

    pub fn max_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Max(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn min_number(list: ListType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Min(
            Box::new(list),
        )))
    }

    pub fn min_list(list: ListType, number: NumberType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Min(
            Box::new(list),
            Box::new(number),
        )))
    }

    pub fn sum(list: ListType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Sum(
            Box::new(list),
        )))
    }

    pub fn avg(list: ListType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Avg(
            Box::new(list),
        )))
    }

    pub fn len(list: ListType) -> Self {
        HIR::Number(NumberType::NumberFunction(NumberFunctionType::Len(
            Box::new(list),
        )))
    }

    pub fn sort_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Sort(Box::new(
            list,
        ))))
    }

    pub fn sort_desc_list(list: ListType) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::SortDesc(
            Box::new(list),
        )))
    }

    pub fn tolist_from_dice_pool(dice_pool: DicePoolType) -> Self {
        HIR::List(ListType::ListFunction(
            ListFunctionType::ToListFromDicePool(Box::new(dice_pool)),
        ))
    }

    pub fn tolist_from_success_pool(success_pool: SuccessPoolType) -> Self {
        HIR::List(ListType::ListFunction(
            ListFunctionType::ToListFromSuccessPool(Box::new(success_pool)),
        ))
    }

    pub fn compare_param(operator: CompareOp, value: NumberType) -> ModParam {
        ModParam {
            operator,
            value: Box::new(value),
        }
    }

    pub fn limit_param(limit_times: Option<NumberType>, limit_counts: Option<NumberType>) -> Limit {
        Limit {
            limit_times: limit_times.map(Box::new),
            limit_counts: limit_counts.map(Box::new),
        }
    }

    pub fn filter_list(list: ListType, mod_param: ModParam) -> Self {
        HIR::List(ListType::ListFunction(ListFunctionType::Filter(
            Box::new(list),
            mod_param,
        )))
    }

    pub fn keep_high(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::KeepHigh(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }
    pub fn keep_low(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::KeepLow(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }
    pub fn drop_high(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::DropHigh(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }
    pub fn drop_low(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::DropLow(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }
    pub fn min_dice_pool(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Min(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }
    pub fn max_dice_pool(dice_pool: DicePoolType, count: NumberType) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Max(
            Box::new(dice_pool),
            Box::new(count),
        )))
    }

    pub fn reroll(dice_pool: DicePoolType, mod_param: ModParam, limit: Option<Limit>) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Reroll(
            Box::new(dice_pool),
            mod_param,
            limit,
        )))
    }
    pub fn explode(
        dice_pool: DicePoolType,
        mod_param: Option<ModParam>,
        limit: Option<Limit>,
    ) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::Explode(
            Box::new(dice_pool),
            mod_param,
            limit,
        )))
    }
    pub fn compound_explode(
        dice_pool: DicePoolType,
        mod_param: Option<ModParam>,
        limit: Option<Limit>,
    ) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::CompoundExplode(
            Box::new(dice_pool),
            mod_param,
            limit,
        )))
    }

    pub fn subtract_failures(dice_pool: DicePoolType, mod_param: ModParam) -> Self {
        HIR::Number(NumberType::DicePool(DicePoolType::SubtractFailures(
            Box::new(dice_pool),
            mod_param,
        )))
    }
    pub fn count_successes_from_dice_pool(dice_pool: DicePoolType, mod_param: ModParam) -> Self {
        HIR::Number(NumberType::SuccessPool(
            SuccessPoolType::CountSuccessesFromDicePool(Box::new(dice_pool), mod_param),
        ))
    }
    pub fn deduct_failures_from_dice_pool(dice_pool: DicePoolType, mod_param: ModParam) -> Self {
        HIR::Number(NumberType::SuccessPool(
            SuccessPoolType::DeductFailuresFromDicePool(Box::new(dice_pool), mod_param),
        ))
    }
    pub fn count_successes(success_pool: SuccessPoolType, mod_param: ModParam) -> Self {
        HIR::Number(NumberType::SuccessPool(SuccessPoolType::CountSuccesses(
            Box::new(success_pool),
            mod_param,
        )))
    }
    pub fn deduct_failures(success_pool: SuccessPoolType, mod_param: ModParam) -> Self {
        HIR::Number(NumberType::SuccessPool(SuccessPoolType::DeductFailures(
            Box::new(success_pool),
            mod_param,
        )))
    }
}

impl NumberType {
    pub fn is_constant(&self) -> bool {
        matches!(self, NumberType::Constant(_))
    }
}

impl ListType {
    pub fn is_explicit(&self) -> bool {
        matches!(self, ListType::Explicit(_))
    }

    pub fn is_constant_list(&self) -> bool {
        match self {
            ListType::Explicit(elements) => elements.iter().all(|e| e.is_constant()),
            _ => false,
        }
    }
}

impl ModParam {
    pub fn is_constant(&self) -> bool {
        self.value.is_constant()
    }

    pub fn get_compare_function(&self) -> Option<impl Fn(f64) -> bool> {
        let target_value = match *self.value {
            NumberType::Constant(v) => v,
            _ => return None,
        };
        Some(move |x: f64| match self.operator {
            CompareOp::Equal => (x - target_value).abs() < std::f64::EPSILON,
            CompareOp::NotEqual => (x - target_value).abs() >= std::f64::EPSILON,
            CompareOp::Less => x < target_value,
            CompareOp::LessEqual => x <= target_value,
            CompareOp::Greater => x > target_value,
            CompareOp::GreaterEqual => x >= target_value,
        })
    }
}

// ==========================================
// 类型检查函数定义
// ==========================================

impl HIR {
    pub fn is_number(&self) -> bool {
        matches!(self, HIR::Number(_))
    }

    pub fn is_list(&self) -> bool {
        matches!(self, HIR::List(_))
    }

    pub fn is_dice_pool(&self) -> bool {
        matches!(self, HIR::Number(NumberType::DicePool(_)))
    }

    pub fn is_success_pool(&self) -> bool {
        matches!(self, HIR::Number(NumberType::SuccessPool(_)))
    }

    pub fn except_number(self) -> Result<NumberType, ()> {
        match self {
            HIR::Number(n) => Ok(n),
            _ => Err(()),
        }
    }

    pub fn except_list(self) -> Result<ListType, ()> {
        match self {
            HIR::List(l) => Ok(l),
            _ => Err(()),
        }
    }

    pub fn except_dice_pool(self) -> Result<DicePoolType, ()> {
        match self {
            HIR::Number(NumberType::DicePool(dp)) => Ok(dp),
            _ => Err(()),
        }
    }

    pub fn except_success_pool(self) -> Result<SuccessPoolType, ()> {
        match self {
            HIR::Number(NumberType::SuccessPool(sp)) => Ok(sp),
            _ => Err(()),
        }
    }
}
