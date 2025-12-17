use oxidice::grammar::*;
use oxidice::types::expr::*;

#[test]
fn test_number_constant() {
    let result = parse_dice("20");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::number(20.0));
}

#[test]
fn test_dice_expr() {
    let result = parse_dice("2D20");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::normal_dice(Expr::number(2.0), Expr::number(20.0))
    );
}

#[test]
fn test_fate_dice_expr() {
    let result = parse_dice("2df");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::fudge_dice(Expr::number(2.0)));
}

#[test]
fn test_coin_dice_expr() {
    let result = parse_dice("3dc");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::coin_dice(Expr::number(3.0)));
}

#[test]
fn test_recursive_dice_expr() {
    let result = parse_dice("(1+2)d6");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::normal_dice(
            Expr::binary(Expr::number(1.0), BinOp::Add, Expr::number(2.0)),
            Expr::number(6.0)
        )
    );
}

#[test]
fn test_recursive_normal_expr() {
    let result = parse_dice("(1 + 2) - (3 - (1 + 1))");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::binary(Expr::number(1.0), BinOp::Add, Expr::number(2.0)),
            BinOp::Sub,
            Expr::binary(
                Expr::number(3.0),
                BinOp::Sub,
                Expr::binary(Expr::number(1.0), BinOp::Add, Expr::number(1.0))
            )
        )
    );
}

#[test]
fn test_implict_dice_expr() {
    let result = parse_dice("d20");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::normal_dice(Expr::number(1.0), Expr::number(20.0))
    );
}

#[test]
fn test_priority_expr() {
    let result = parse_dice("1 + 2d20 * 3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::number(1.0),
            BinOp::Add,
            Expr::binary(
                Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
                BinOp::Mul,
                Expr::number(3.0)
            )
        )
    );
}

#[test]
fn test_div_expr() {
    let result = parse_dice("10 / 2d5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::number(10.0),
            BinOp::Div,
            Expr::normal_dice(Expr::number(2.0), Expr::number(5.0))
        )
    );
}

#[test]
fn test_idiv_expr() {
    let result = parse_dice("10 // 2d5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::number(10.0),
            BinOp::Idiv,
            Expr::normal_dice(Expr::number(2.0), Expr::number(5.0))
        )
    );
}

#[test]
fn test_mod_expr() {
    let result = parse_dice("3d4 % 10");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
            BinOp::Mod,
            Expr::number(10.0)
        )
    );
}

#[test]
fn test_list_expr() {
    let result = parse_dice("[2d6, 3d4, 1d20]");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::list(vec![
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
            Expr::normal_dice(Expr::number(1.0), Expr::number(20.0)),
        ])
    );
}

#[test]
fn test_list_multi_expr() {
    let result = parse_dice("[1d6, 2d8, 3d10] * 2");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::binary(
            Expr::list(vec![
                Expr::normal_dice(Expr::number(1.0), Expr::number(6.0)),
                Expr::normal_dice(Expr::number(2.0), Expr::number(8.0)),
                Expr::normal_dice(Expr::number(3.0), Expr::number(10.0)),
            ]),
            BinOp::Mul,
            Expr::number(2.0)
        )
    );
}

#[test]
fn test_max_list() {
    let result = parse_dice("max([2d6, 3d4, 1d20])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Max,
            vec![Expr::list(vec![
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
                Expr::normal_dice(Expr::number(1.0), Expr::number(20.0)),
            ])]
        )
    );
}

#[test]
fn test_max_args() {
    let result = parse_dice("Max(2d6, 3d4, 1d20)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Max,
            vec![
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
                Expr::normal_dice(Expr::number(1.0), Expr::number(20.0)),
            ]
        )
    )
}

#[test]
fn test_sum_args() {
    let result = parse_dice("sum(2d6, 3d4, 1d20)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Sum,
            vec![
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
                Expr::normal_dice(Expr::number(1.0), Expr::number(20.0)),
            ]
        )
    )
}

#[test]
fn test_avg_args() {
    let result = parse_dice("avg(2d6, 3d4, 1d20)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Avg,
            vec![
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                Expr::normal_dice(Expr::number(3.0), Expr::number(4.0)),
                Expr::normal_dice(Expr::number(1.0), Expr::number(20.0)),
            ]
        )
    )
}

#[test]
fn test_abs_args() {
    let result = parse_dice("abs(2d6-10)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Abs,
            vec![Expr::binary(
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                BinOp::Sub,
                Expr::number(10.0)
            ),]
        )
    )
}

#[test]
fn test_floor_args() {
    let result = parse_dice("floor(2d6-10)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Floor,
            vec![Expr::binary(
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                BinOp::Sub,
                Expr::number(10.0)
            ),]
        )
    )
}

#[test]
fn test_ceil_args() {
    let result = parse_dice("ceil(2d6-10)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Ceil,
            vec![Expr::binary(
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                BinOp::Sub,
                Expr::number(10.0)
            ),]
        )
    )
}

#[test]
fn test_round_args() {
    let result = parse_dice("round(2d6-10)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Round,
            vec![Expr::binary(
                Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                BinOp::Sub,
                Expr::number(10.0)
            ),]
        )
    )
}

#[test]
fn test_filter_args() {
    let result = parse_dice("filter<>3([2d6-10, 10, 14])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Filter(Expr::mod_param(CompareOp::NotEqual, Expr::number(3.0))),
            vec![Expr::List(vec![
                Expr::binary(
                    Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                    BinOp::Sub,
                    Expr::number(10.0)
                ),
                Expr::number(10.0),
                Expr::number(14.0),
            ]),]
        )
    )
}

#[test]
fn test_len_args() {
    let result = parse_dice("len([2d6-10, 10, 14])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Len,
            vec![Expr::List(vec![
                Expr::binary(
                    Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                    BinOp::Sub,
                    Expr::number(10.0)
                ),
                Expr::number(10.0),
                Expr::number(14.0),
            ]),]
        )
    )
}

#[test]
fn test_sort_args() {
    let result = parse_dice("sort([2d6-10, 10, 14])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Sort,
            vec![Expr::List(vec![
                Expr::binary(
                    Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                    BinOp::Sub,
                    Expr::number(10.0)
                ),
                Expr::number(10.0),
                Expr::number(14.0),
            ]),]
        )
    )
}

#[test]
fn test_sortd_args() {
    let result = parse_dice("sortd([2d6-10, 10, 14])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::Sortd,
            vec![Expr::List(vec![
                Expr::binary(
                    Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
                    BinOp::Sub,
                    Expr::number(10.0)
                ),
                Expr::number(10.0),
                Expr::number(14.0),
            ]),]
        )
    )
}

#[test]
fn test_max_empty() {
    let result = parse_dice("max()");

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::function(FunctionName::Max, vec![]))
}

#[test]
fn test_wrong_dice() {
    let result = parse_dice("2 * 2d");

    assert!(result.is_err());
}

#[test]
fn test_min_list_empty() {
    let result = parse_dice("min([])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(FunctionName::Min, vec![Expr::List(vec![]),],)
    )
}

#[test]
fn test_rpdice_list_empty() {
    let result = parse_dice("rpdice([])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(FunctionName::Rpdice, vec![Expr::List(vec![]),])
    )
}

#[test]
fn test_keephigh() {
    let result = parse_dice("2d20kh");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type1Op::KeepHigh,
            Expr::number(1.0)
        )
    );
}

#[test]
fn test_keeplow() {
    let result = parse_dice("3d20kl");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(3.0), Expr::number(20.0)),
            Type1Op::KeepLow,
            Expr::number(1.0)
        )
    );
}

#[test]
fn test_drophigh() {
    let result = parse_dice("4d20dh");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(4.0), Expr::number(20.0)),
            Type1Op::DropHigh,
            Expr::number(1.0)
        )
    );
}

#[test]
fn test_droplow() {
    let result = parse_dice("5d20dl");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(5.0), Expr::number(20.0)),
            Type1Op::DropLow,
            Expr::number(1.0)
        )
    );
}

#[test]
fn test_keephigh_with_param() {
    let result = parse_dice("2d20kh1");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type1Op::KeepHigh,
            Expr::number(1.0)
        )
    );
}

#[test]
fn test_keeplow_with_param() {
    let result = parse_dice("3d20kl2");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(3.0), Expr::number(20.0)),
            Type1Op::KeepLow,
            Expr::number(2.0)
        )
    );
}

#[test]
fn test_drophigh_with_param() {
    let result = parse_dice("4d20dh3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(4.0), Expr::number(20.0)),
            Type1Op::DropHigh,
            Expr::number(3.0)
        )
    );
}

#[test]
fn test_droplow_with_param() {
    let result = parse_dice("5d20DL4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(5.0), Expr::number(20.0)),
            Type1Op::DropLow,
            Expr::number(4.0)
        )
    );
}

#[test]
fn test_pos() {
    let result = parse_dice("+5d20dl4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(5.0), Expr::number(20.0)),
            Type1Op::DropLow,
            Expr::number(4.0)
        )
    );
}

#[test]
fn test_neg() {
    let result = parse_dice("-5d20dl4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::neg(Expr::modifier_type1(
            Expr::normal_dice(Expr::number(5.0), Expr::number(20.0)),
            Type1Op::DropLow,
            Expr::number(4.0)
        ))
    );
}

#[test]
fn test_compare_expr() {
    let result = parse_dice("2d20cs<=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::CountSuccesses,
            ModParam {
                operator: CompareOp::LessEqual,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_cs_cf_expr() {
    let result = parse_dice("2d20cs<=15df=20");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::modifier_type3(
                Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
                Type3Op::CountSuccesses,
                ModParam {
                    operator: CompareOp::LessEqual,
                    value: Box::new(Expr::number(15.0)),
                }
            ),
            Type3Op::DeductFailures,
            Expr::mod_param(CompareOp::Equal, Expr::number(20.0))
        )
    );
}

#[test]
fn test_sf_expr() {
    let result = parse_dice("2d20sf<15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::SubtractFailures,
            ModParam {
                operator: CompareOp::Less,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_tolist_expr() {
    let result = parse_dice("tolist(2d20)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::function(
            FunctionName::ToList,
            vec![Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),]
        )
    );
}

#[test]
fn test_compare_expr2() {
    let result = parse_dice("2d20cs>=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::CountSuccesses,
            ModParam {
                operator: CompareOp::GreaterEqual,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_compare_expr3() {
    let result = parse_dice("2d20cs=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::CountSuccesses,
            ModParam {
                operator: CompareOp::Equal,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_compare_expr4() {
    let result = parse_dice("2d20cs>15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::CountSuccesses,
            ModParam {
                operator: CompareOp::Greater,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_compare_expr5() {
    let result = parse_dice("2d20cs<15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type3(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type3Op::CountSuccesses,
            ModParam {
                operator: CompareOp::Less,
                value: Box::new(Expr::number(15.0)),
            }
        )
    );
}

#[test]
fn test_explode_expr() {
    let result = parse_dice("2d6!");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::Explode,
            None,
            None
        )
    );
}

#[test]
fn test_explode_expr_with_param() {
    let result = parse_dice("2d6!3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::Explode,
            Some(ModParam {
                operator: CompareOp::Equal,
                value: Box::new(Expr::number(3.0)),
            }),
            None
        )
    );
}

#[test]
fn test_explode_compound_expr() {
    let result = parse_dice("2d6!!");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::CompoundExplode,
            None,
            None
        )
    );
}

#[test]
fn test_explode_compound_expr_with_param() {
    let result = parse_dice("2d6!!<=4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::CompoundExplode,
            Some(ModParam {
                operator: CompareOp::LessEqual,
                value: Box::new(Expr::number(4.0)),
            }),
            None
        )
    );
}

#[test]
fn test_explode_compound_expr_with_param_and_limit() {
    let result = parse_dice("2d6!!<=4lt(1+1)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::CompoundExplode,
            Some(ModParam {
                operator: CompareOp::LessEqual,
                value: Box::new(Expr::number(4.0)),
            }),
            Some(Limit {
                limit_times: Some(Box::new(Expr::binary(
                    Expr::number(1.0),
                    BinOp::Add,
                    Expr::number(1.0)
                ))),
                limit_counts: None,
            })
        )
    )
}

#[test]
fn test_explode_compound_expr_with_limit() {
    let result = parse_dice("2d6!!lc4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Type2Op::CompoundExplode,
            None,
            Some(Limit {
                limit_times: None,
                limit_counts: Some(Box::new(Expr::number(4.0))),
            })
        )
    );
}

#[test]
fn test_reroll_expr() {
    let result = parse_dice("2d20r<5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type2Op::Reroll,
            Some(ModParam {
                operator: CompareOp::Less,
                value: Box::new(Expr::number(5.0)),
            }),
            None
        )
    );
}

#[test]
fn test_reroll_once_expr() {
    let result = parse_dice("2d20r>=5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type2(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type2Op::Reroll,
            Some(ModParam {
                operator: CompareOp::GreaterEqual,
                value: Box::new(Expr::number(5.0)),
            }),
            None
        )
    );
}

#[test]
fn test_min_expr_without_param() {
    let result = parse_dice("2d20min4");
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type1Op::Min,
            Expr::number(4.0)
        )
    );
}

#[test]
fn test_max_expr_without_param() {
    let result = parse_dice("2d20max5");
    assert_eq!(
        result.unwrap(),
        Expr::modifier_type1(
            Expr::normal_dice(Expr::number(2.0), Expr::number(20.0)),
            Type1Op::Max,
            Expr::number(5.0)
        )
    );
}

#[test]
fn test_reroll_expr_without_param() {
    let result = parse_dice("2d20r");
    assert!(result.is_err());
}

#[test]
fn test_reroll_once_expr_without_param() {
    let result = parse_dice("2d20ro");
    assert!(result.is_err());
}

#[test]
fn test_min_modifier_without_param() {
    let result = parse_dice("2d20min");
    assert!(result.is_err());
}

#[test]
fn test_max_modifier_without_param() {
    let result = parse_dice("2d20max");
    assert!(result.is_err());
}

#[test]
fn test_old_style_success_check_modifier_without_param() {
    let result = parse_dice("2d20<3");
    assert!(result.is_err());
}

#[test]
fn test_implicit_defaults_combo() {
    let result = parse_dice("d6kh");
    assert!(result.is_ok());

    let expected = Expr::modifier_type1(
        Expr::normal_dice(Expr::number(1.0), Expr::number(6.0)), // d6 -> 1d6
        Type1Op::KeepHigh,
        Expr::number(1.0), // kh -> kh1
    );

    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_implicit_compare_op() {
    // 目标: 4d6r3 (r3 省略了符号，默认为 =)
    let result = parse_dice("4d6r3");
    assert!(result.is_ok());

    let expected = Expr::modifier_type2(
        Expr::normal_dice(Expr::number(4.0), Expr::number(6.0)),
        Type2Op::Reroll,
        Some(Expr::mod_param(CompareOp::Equal, Expr::number(3.0))), // r3 -> r=3
        None,
    );

    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_explode_defaults() {
    // 目标: 10d10! (爆炸，省略条件，默认极大值)
    // 注意：explode 默认参数通常是 None，表示“面的最大值”
    // 取决于你的 process_type2_modifier 实现是否塞入了默认值
    let result = parse_dice("10d10!");
    assert!(result.is_ok());

    let expected = Expr::modifier_type2(
        Expr::normal_dice(Expr::number(10.0), Expr::number(10.0)),
        Type2Op::Explode,
        None, // 你的实现里，如果是 None，就是 None，这很好
        None,
    );

    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_modifier_chaining_and_calc() {
    // 目标: 10d6!kh(1+2)r>5
    // 解释: 10d6 -> 爆炸 -> 保留前(1+2)个 -> 重投大于5的
    // 这测试了 postfix 的结合律和参数内的表达式解析
    let result = parse_dice("10d6!kh(1+2)r>5");
    assert!(result.is_ok());

    // 构建顺序：最内层是 10d6，然后一层层包裹
    let step1 = Expr::normal_dice(Expr::number(10.0), Expr::number(6.0));

    let step2 = Expr::modifier_type2(
        // !
        step1,
        Type2Op::Explode,
        None,
        None,
    );

    let step3 = Expr::modifier_type1(
        // kh(1+2)
        step2,
        Type1Op::KeepHigh,
        Expr::binary(Expr::number(1.0), BinOp::Add, Expr::number(2.0)),
    );

    let step4 = Expr::modifier_type2(
        // r>5
        step3,
        Type2Op::Reroll,
        Some(Expr::mod_param(CompareOp::Greater, Expr::number(5.0))),
        None,
    );

    assert_eq!(result.unwrap(), step4);
}

#[test]
fn test_complex_function_call() {
    // 目标: max(2d6, floor(3.5))
    let result = parse_dice("max(2d6, floor(3.5))");
    assert!(result.is_ok());

    let expected = Expr::function(
        FunctionName::Max,
        vec![
            Expr::normal_dice(Expr::number(2.0), Expr::number(6.0)),
            Expr::function(FunctionName::Floor, vec![Expr::number(3.5)]),
        ],
    );
    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_filter_function_syntax() {
    // 目标: filter>3([1, 2, 6])
    // 这是最容易崩的地方，测试 parse_atom 里对 function_name 的特殊处理
    let result = parse_dice("filter>3([1, 2, 6])");
    assert!(result.is_ok());

    let expected = Expr::function(
        FunctionName::Filter(Expr::mod_param(CompareOp::Greater, Expr::number(3.0))),
        vec![Expr::list(vec![
            Expr::number(1.0),
            Expr::number(2.0),
            Expr::number(6.0),
        ])],
    );
    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_chaos_theory() {
    // 目标: (1+1)d6!lt(1+1)kh1 + sum([1, 2])
    let result = parse_dice("(1+1)d6!lt(1+1)kh1 + sum([1, 2])");
    assert!(result.is_ok());

    // 仅仅 assert(is_ok) 本身就是一种胜利，
    // 如果你想手写这个 expected AST，那也是一种修行...
    let dice_part = Expr::modifier_type1(
        Expr::modifier_type2(
            Expr::normal_dice(
                Expr::binary(Expr::number(1.0), BinOp::Add, Expr::number(1.0)),
                Expr::number(6.0),
            ),
            Type2Op::Explode,
            None,
            Some(Limit {
                limit_times: Some(Box::new(Expr::binary(
                    Expr::number(1.0),
                    BinOp::Add,
                    Expr::number(1.0),
                ))),
                limit_counts: None,
            }),
        ),
        Type1Op::KeepHigh,
        Expr::number(1.0),
    );

    let sum_part = Expr::function(
        FunctionName::Sum,
        vec![Expr::list(vec![Expr::number(1.0), Expr::number(2.0)])],
    );

    let expected = Expr::binary(dice_part, BinOp::Add, sum_part);

    assert_eq!(result.unwrap(), expected);
}

#[test]
fn test_missing_dice_sides() {
    // 错误原因：只有 'd' 没有面数
    // Grammar: dice_op ~ atom
    let result = parse_dice("3d");
    assert!(result.is_err());
}

#[test]
fn test_trailing_operator() {
    // 错误原因：算术运算符后缺少操作数
    let result = parse_dice("1 +");
    assert!(result.is_err());
}

#[test]
fn test_unbalanced_parentheses() {
    // 错误原因：括号未闭合
    let result = parse_dice("(3 + 4");
    assert!(result.is_err());
}

#[test]
fn test_unbalanced_list() {
    // 错误原因：列表未闭合
    let result = parse_dice("[1, 2, 3");
    assert!(result.is_err());
}

#[test]
fn test_empty_input() {
    // 错误原因：空字符串
    // 虽然有些 Parser 允许空，但通常 expr 需要至少匹配一个 atom
    let result = parse_dice("");
    assert!(result.is_err());
}

// ==========================================
// 2. 修饰符缺少参数 (Missing Params)
// ==========================================
// 注意：kh, dl 等在你的 grammar 中是可选参数 (atom?)，所以它们不会报错。
// 但是 min, max, lt, lc, filter, cs 通常被定义为必须参数。

#[test]
fn test_filter_without_param() {
    // Grammar: filter = { ^"filter" ~ mod_param }
    let result = parse_dice("filter([1,2,3])");
    assert!(result.is_err());
}

#[test]
fn test_filter_incomplete_comparison() {
    // Grammar: mod_param = { compare_op? ~ atom }
    // 这里只有操作符，没有 atom
    let result = parse_dice("filter>");
    assert!(result.is_err());
}

#[test]
fn test_limit_times_without_value() {
    // Grammar: limit_times = { ^"lt" ~ atom }
    let result = parse_dice("10d6!lt");
    assert!(result.is_err());
}

#[test]
fn test_count_success_without_target() {
    // 假设 cs 定义为必须参数: count_successes = { ^"cs" ~ mod_param }
    let result = parse_dice("10d6cs");
    assert!(result.is_err());
}

// ==========================================
// 3. 符号与逻辑错误 (Symbolic Errors)
// ==========================================

#[test]
fn test_double_dice_operator() {
    // 错误原因：连续两个 d
    // Pest 不会自动跳过这种错误
    let result = parse_dice("3dd6");
    assert!(result.is_err());
}

#[test]
fn test_invalid_compare_op_in_reroll() {
    // 错误原因：reroll 后面跟了非法符号
    // 虽然 pest 的 mod_param 可能允许 compare_op?，但 "r?" 这种是不合法的 token
    let result = parse_dice("1d6r?");
    assert!(result.is_err());
}

#[test]
fn test_space_inside_atomic_filter() {
    // 如果你的 filter 定义了 ${ ... } (复合原子)，则内部不允许有空格
    // 如果你用了 !{ mod_param } 恢复空格，这个测试可能会通过。
    // 但如果 input 是 "fil ter>5"，这就一定挂。
    let result = parse_dice("fil ter>5");
    assert!(result.is_err());
}

// ==========================================
// 4. 函数与标识符错误 (Identifier Errors)
// ==========================================

#[test]
fn test_unknown_function() {
    // 错误原因：grammar 中 func_name 枚举了允许的函数名
    // "magic" 不在列表中，且无法解析为 number 或 list
    let result = parse_dice("magic(10)");
    assert!(result.is_err());
}

#[test]
fn test_function_missing_args_parens() {
    // 错误原因：函数调用必须有括号
    let result = parse_dice("max 1, 2");
    assert!(result.is_err());
}

#[test]
fn test_malformed_float() {
    // 错误原因：多个小数点
    let result = parse_dice("1.2.3");
    // Pest 的 number 规则通常是 DIGIT+ ~ ("." ~ DIGIT+)?
    // 这通常会被解析为 Number(1.2) 然后剩下 ".3" 导致解析无法消耗完 EOI (End of Input)
    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer() {
    let result = parse_dice("2d20khh2");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer2() {
    let result = parse_dice("2d20xx");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer3() {
    let result = parse_dice("2d20r<3lt1lt2");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer4() {
    let result = parse_dice("2d20r<3lc1lc2");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer5() {
    let result = parse_dice("2d20r<3lc1lc");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer6() {
    let result = parse_dice("2d20r< 3lc1lt1");

    assert!(result.is_err());
}

#[test]
fn test_wrong_dice2() {
    let result = parse_dice("2da");

    assert!(result.is_err());
}

#[test]
fn test_wrong_modifer7() {
    let result = parse_dice("2d20css");

    assert!(result.is_err());
}
