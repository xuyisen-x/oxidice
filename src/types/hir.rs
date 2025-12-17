use super::expr::{Limit, ModParam};

// ==========================================
// HIR: 高级中间表示 (High-level Intermediate Representation)
// ==========================================

pub enum HIR {
    Number(NumberType),
    List(ListType),
}

// ==========================================
// 数字类型
// ==========================================

pub enum NumberType {
    Constant(f64),
    DicePool(DicePoolType),
    SuccessPool(SuccessPoolType),
    NumberBinary(NumberBinaryType),
    NumberFunction(NumberFunctionType),
    Neg(Box<NumberType>), // 唯一一个单目运算符，就不单独定义枚举了
}

pub enum DicePoolType {
    Standard(i64, i64),                                                  // XdY
    Fudge(i64),                                                          // XdF
    Coin(i64),                                                           // XdC
    KeepHigh(Box<DicePoolType>, i64),                                    // (XdY)khZ
    KeepLow(Box<DicePoolType>, i64),                                     // (XdY)kl
    DropHigh(Box<DicePoolType>, i64),                                    // (XdY)dhZ
    DropLow(Box<DicePoolType>, i64),                                     // (XdY)dl
    Min(Box<DicePoolType>, f64),                                         // (XdY)minZ
    Max(Box<DicePoolType>, f64),                                         // (XdY)maxZ
    Explode(Box<DicePoolType>, Option<ModParam>, Option<Limit>),         // (XdY)![mod_param][limit]
    CompoundExplode(Box<DicePoolType>, Option<ModParam>, Option<Limit>), // (XdY)!![mod_param][limit]
    Reroll(Box<DicePoolType>, Option<ModParam>, Option<Limit>),          // (XdY)r[mod_param][limit]
    SubtractFailures(Box<DicePoolType>, ModParam),                       // (XdY)sfmod_param
}

pub enum SuccessPoolType {
    CountSuccessesFromDicePool(Box<DicePoolType>, ModParam), // success_pool_type cs dice_pool_type
    DeductFailuresFromDicePool(Box<DicePoolType>, ModParam), // success_pool_type df dice_pool_type
    CountSuccesses(Box<SuccessPoolType>, ModParam),          // success_pool_type cs mod_param
    DeductFailures(Box<SuccessPoolType>, ModParam),          // success_pool_type df mod_param
}

pub enum NumberBinaryType {
    Add(Box<NumberType>, Box<NumberType>),
    Subtract(Box<NumberType>, Box<NumberType>),
    Multiply(Box<NumberType>, Box<NumberType>),
    Divide(Box<NumberType>, Box<NumberType>),
    IntDivide(Box<NumberType>, Box<NumberType>),
    Modulo(Box<NumberType>, Box<NumberType>),
}

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

pub enum ListType {
    Explicit(Vec<NumberType>),      // [num1, num2, num3, ...]
    ListFunction(ListFunctionType), // list_function_type
}

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
