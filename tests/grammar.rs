use dice_roller::grammar::*;

#[test]
fn test_number_constant() {
    let result = parse_dice("20");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Expr::Number(20.0));
}

#[test]
fn test_dice_expr() {
    let result = parse_dice("2d20");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Dice {
            count: Box::new(Expr::Number(2.0)),
            side: Box::new(Expr::Number(20.0))
        }
    );
}

#[test]
fn test_recursive_dice_expr() {
    let result = parse_dice("(1+2)d6");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Dice {
            count: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Number(1.0)),
                op: BinOp::Add,
                rhs: Box::new(Expr::Number(2.0))
            }),
            side: Box::new(Expr::Number(6.0)),
        }
    );
}

#[test]
fn test_recursive_normal_expr() {
    let result = parse_dice("(1 + 2) - (3 - (1 + 1))");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Number(1.0)),
                op: BinOp::Add,
                rhs: Box::new(Expr::Number(2.0))
            }),
            op: BinOp::Sub,
            rhs: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Number(3.0)),
                op: BinOp::Sub,
                rhs: Box::new(Expr::Binary {
                    lhs: Box::new(Expr::Number(1.0)),
                    op: BinOp::Add,
                    rhs: Box::new(Expr::Number(1.0))
                })
            })
        }
    );
}

#[test]
fn test_implict_dice_expr() {
    let result = parse_dice("d20");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Dice {
            count: Box::new(Expr::Number(1.0)),
            side: Box::new(Expr::Number(20.0))
        }
    );
}

#[test]
fn test_priority_expr() {
    let result = parse_dice("1 + 2d20 * 3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Number(1.0)),
            op: BinOp::Add,
            rhs: Box::new(Expr::Binary {
                lhs: Box::new(Expr::Dice {
                    count: Box::new(Expr::Number(2.0)),
                    side: Box::new(Expr::Number(20.0)),
                }),
                op: BinOp::Mul,
                rhs: Box::new(Expr::Number(3.0))
            })
        }
    );
}

#[test]
fn test_div_expr() {
    let result = parse_dice("10 / 2d5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Number(10.0)),
            op: BinOp::Div,
            rhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(5.0)),
            })
        }
    );
}

#[test]
fn test_idiv_expr() {
    let result = parse_dice("10 // 2d5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Number(10.0)),
            op: BinOp::Idiv,
            rhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(5.0)),
            })
        }
    );
}

#[test]
fn test_mod_expr() {
    let result = parse_dice("3d4 % 10");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(3.0)),
                side: Box::new(Expr::Number(4.0)),
            }),
            op: BinOp::Mod,
            rhs: Box::new(Expr::Number(10.0))
        }
    );
}

#[test]
fn test_list_expr() {
    let result = parse_dice("[2d6, 3d4, 1d20]");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::List(vec![
            Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0)),
            },
            Expr::Dice {
                count: Box::new(Expr::Number(3.0)),
                side: Box::new(Expr::Number(4.0)),
            },
            Expr::Dice {
                count: Box::new(Expr::Number(1.0)),
                side: Box::new(Expr::Number(20.0)),
            },
        ])
    );
}

#[test]
fn test_list_multi_expr() {
    let result = parse_dice("[1d6, 2d8, 3d10] * 2");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::List(vec![
                Expr::Dice {
                    count: Box::new(Expr::Number(1.0)),
                    side: Box::new(Expr::Number(6.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(2.0)),
                    side: Box::new(Expr::Number(8.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(3.0)),
                    side: Box::new(Expr::Number(10.0)),
                },
            ])),
            op: BinOp::Mul,
            rhs: Box::new(Expr::Number(2.0))
        }
    );
}

#[test]
fn test_max_list() {
    let result = parse_dice("max([2d6, 3d4, 1d20])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Call {
            func_name: "max".to_string(),
            args: vec![Expr::List(vec![
                Expr::Dice {
                    count: Box::new(Expr::Number(2.0)),
                    side: Box::new(Expr::Number(6.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(3.0)),
                    side: Box::new(Expr::Number(4.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(1.0)),
                    side: Box::new(Expr::Number(20.0)),
                }
            ])],
        }
    );
}

#[test]
fn test_max_args() {
    let result = parse_dice("max(2d6, 3d4, 1d20)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Call {
            func_name: "max".to_string(),
            args: vec![
                Expr::Dice {
                    count: Box::new(Expr::Number(2.0)),
                    side: Box::new(Expr::Number(6.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(3.0)),
                    side: Box::new(Expr::Number(4.0)),
                },
                Expr::Dice {
                    count: Box::new(Expr::Number(1.0)),
                    side: Box::new(Expr::Number(20.0)),
                },
            ],
        }
    )
}

#[test]
fn test_max_empty() {
    let result = parse_dice("max()");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Call {
            func_name: "max".to_string(),
            args: vec![],
        }
    )
}

#[test]
fn test_min_list_empty() {
    let result = parse_dice("min([])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Call {
            func_name: "min".to_string(),
            args: vec![Expr::List(vec![]),],
        }
    )
}

#[test]
fn test_rpdice_list_empty() {
    let result = parse_dice("rpdice([])");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Call {
            func_name: "rpdice".to_string(),
            args: vec![Expr::List(vec![]),],
        }
    )
}

#[test]
fn test_keephigh() {
    let result = parse_dice("2d20kh");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::KeepHigh,
            count: Box::new(Expr::Number(1.0)),
        }
    );
}

#[test]
fn test_keeplow() {
    let result = parse_dice("3d20kl");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(3.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::KeepLow,
            count: Box::new(Expr::Number(1.0)),
        }
    );
}

#[test]
fn test_drophigh() {
    let result = parse_dice("4d20dh");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(4.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::DropHigh,
            count: Box::new(Expr::Number(1.0)),
        }
    );
}

#[test]
fn test_droplow() {
    let result = parse_dice("5d20dl");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(5.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::DropLow,
            count: Box::new(Expr::Number(1.0)),
        }
    );
}

#[test]
fn test_keephigh_with_param() {
    let result = parse_dice("2d20kh1");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::KeepHigh,
            count: Box::new(Expr::Number(1.0)),
        }
    );
}

#[test]
fn test_keeplow_with_param() {
    let result = parse_dice("3d20kl2");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(3.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::KeepLow,
            count: Box::new(Expr::Number(2.0)),
        }
    );
}

#[test]
fn test_drophigh_with_param() {
    let result = parse_dice("4d20dh3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(4.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::DropHigh,
            count: Box::new(Expr::Number(3.0)),
        }
    );
}

#[test]
fn test_droplow_with_param() {
    let result = parse_dice("5d20dl4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(5.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::DropLow,
            count: Box::new(Expr::Number(4.0)),
        }
    );
}

#[test]
fn test_pos() {
    let result = parse_dice("+5d20dl4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::KeepOrDropModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(5.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: KeepOrDropModifierOp::DropLow,
            count: Box::new(Expr::Number(4.0)),
        }
    );
}

#[test]
fn test_neg() {
    let result = parse_dice("-5d20dl4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::Binary {
            lhs: Box::new(Expr::Number(0.0)),
            op: BinOp::Sub,
            rhs: Box::new(Expr::KeepOrDropModifier {
                lhs: Box::new(Expr::Dice {
                    count: Box::new(Expr::Number(5.0)),
                    side: Box::new(Expr::Number(20.0)),
                }),
                op: KeepOrDropModifierOp::DropLow,
                count: Box::new(Expr::Number(4.0)),
            })
        }
    );
}

#[test]
fn test_compare_expr() {
    let result = parse_dice("2d20<=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::SuccessCheck {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0))
            },),
            compare_expr: CompareExpr {
                op: CompareOp::LessEqual,
                val: Box::new(Expr::Number(15.0)),
            },
        }
    );
}

#[test]
fn test_compare_expr2() {
    let result = parse_dice("2d20>=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::SuccessCheck {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0))
            },),
            compare_expr: CompareExpr {
                op: CompareOp::GreaterEqual,
                val: Box::new(Expr::Number(15.0)),
            },
        }
    );
}

#[test]
fn test_compare_expr3() {
    let result = parse_dice("2d20=15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::SuccessCheck {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0))
            },),
            compare_expr: CompareExpr {
                op: CompareOp::Equal,
                val: Box::new(Expr::Number(15.0)),
            },
        }
    );
}

#[test]
fn test_compare_expr4() {
    let result = parse_dice("2d20>15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::SuccessCheck {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0))
            },),
            compare_expr: CompareExpr {
                op: CompareOp::Greater,
                val: Box::new(Expr::Number(15.0)),
            },
        }
    );
}

#[test]
fn test_compare_expr5() {
    let result = parse_dice("2d20<15");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::SuccessCheck {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0))
            },),
            compare_expr: CompareExpr {
                op: CompareOp::Less,
                val: Box::new(Expr::Number(15.0)),
            },
        }
    );
}

#[test]
fn test_explode_expr() {
    let result = parse_dice("2d6!");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::Explode,
            compare_expr: None,
            limit: None
        }
    );
}

#[test]
fn test_explode_expr_with_param() {
    let result = parse_dice("2d6!3");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::Explode,
            compare_expr: Some(CompareExpr {
                op: CompareOp::Equal,
                val: Box::new(Expr::Number(3.0))
            }),
            limit: None
        }
    );
}

#[test]
fn test_explode_compound_expr() {
    let result = parse_dice("2d6!!");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::CompoundExplode,
            compare_expr: None,
            limit: None
        }
    );
}

#[test]
fn test_explode_compound_expr_with_param() {
    let result = parse_dice("2d6!!<=4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::CompoundExplode,
            compare_expr: Some(CompareExpr {
                op: CompareOp::LessEqual,
                val: Box::new(Expr::Number(4.0))
            }),
            limit: None
        }
    );
}

#[test]
fn test_explode_compound_expr_with_param_and_limit() {
    let result = parse_dice("2d6!!<=4l(1+1)");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::CompoundExplode,
            compare_expr: Some(CompareExpr {
                op: CompareOp::LessEqual,
                val: Box::new(Expr::Number(4.0))
            }),
            limit: Some(Box::new(Expr::Binary {
                lhs: Box::new(Expr::Number(1.0)),
                op: BinOp::Add,
                rhs: Box::new(Expr::Number(1.0))
            }))
        }
    );
}

#[test]
fn test_explode_compound_expr_with_limit() {
    let result = parse_dice("2d6!!l4");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::ExplodeModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(6.0))
            },),
            op: ExplodeModifierOp::CompoundExplode,
            compare_expr: None,
            limit: Some(Box::new(Expr::Number(4.0)))
        }
    );
}

#[test]
fn test_reroll_expr() {
    let result = parse_dice("2d20r<5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::RerollModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: RerollModifierOp::Reroll,
            compare_expr: CompareExpr {
                op: CompareOp::Less,
                val: Box::new(Expr::Number(5.0))
            }
        }
    );
}

#[test]
fn test_reroll_once_expr() {
    let result = parse_dice("2d20ro>=5");
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        Expr::RerollModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: RerollModifierOp::RerollOnce,
            compare_expr: CompareExpr {
                op: CompareOp::GreaterEqual,
                val: Box::new(Expr::Number(5.0))
            }
        }
    );
}

#[test]
fn test_min_expr_without_param() {
    let result = parse_dice("2d20min4");
    assert_eq!(
        result.unwrap(),
        Expr::MinMaxModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: MinMaxModifierOp::Min,
            target: Box::new(Expr::Number(4.0)),
        }
    );
}

#[test]
fn test_max_expr_without_param() {
    let result = parse_dice("2d20max5");
    assert_eq!(
        result.unwrap(),
        Expr::MinMaxModifier {
            lhs: Box::new(Expr::Dice {
                count: Box::new(Expr::Number(2.0)),
                side: Box::new(Expr::Number(20.0)),
            }),
            op: MinMaxModifierOp::Max,
            target: Box::new(Expr::Number(5.0)),
        }
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
