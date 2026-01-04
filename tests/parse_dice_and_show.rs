use oxidice::parse_dice_and_show;

fn test_legal_input(input: &str, expected: &str) {
    match parse_dice_and_show(input) {
        Ok(output) => assert_eq!(output, expected),
        Err(e) => panic!("Expected legal input, but got error: {}", e),
    }
}

fn test_illegal_input(input: &str) {
    match parse_dice_and_show(input) {
        Ok(output) => panic!("Expected illegal input, but got output: {}", output),
        Err(_) => {} // Expected
    }
}

#[test]
fn constant_fold() {
    test_legal_input("2 + 3 * 4", "14");
    test_legal_input("10 - 2 - 3", "5");
    test_legal_input("10 // 2 - 3", "2");
    test_legal_input("2d6 + 2d6", "4d6");
    test_legal_input("1d20 + 5 + 3", "1d20+8");
    test_legal_input("4 * (2 + 3)", "20");
    test_legal_input("2d(3 + 3)", "2d6");
    test_legal_input("((1 + 2) * (3 + 4))", "21");
    test_legal_input("max(2 + 3, 4 * 2)", "8");
    test_legal_input("min(10 - 3, 2)", "2");
    test_legal_input("abs(-5 + 2)", "3");
    test_legal_input("floor(5 / 2)", "2");
    test_legal_input("ceil(5 / 2)", "3");
    test_legal_input("round(7 / 3)", "2");
    test_legal_input("round(8 / 3)", "3");
    test_legal_input("1-(-2)", "3");
    test_legal_input("-1d6 + 2d6", "2d6-1d6");
    test_legal_input("sum([1, 2, 3] + [4, 5])", "15");
    test_legal_input("sum(1, 2, 3, 4, 5)", "15");
    test_legal_input("len(1, 2, 3, 4, 5)", "5");
    test_legal_input("sum([1d8, 2d8, 3d8] + [4d6, 5d6])", "6d8+9d6");
    test_legal_input("sum(tolist(1d6))", "sum(tolist(1d6))");
    test_legal_input("avg(tolist(1d6))", "avg(tolist(1d6))");
    test_legal_input("min(tolist(1d6))", "min(tolist(1d6))");
    test_legal_input("max(tolist(1d6))", "max(tolist(1d6))");
    test_legal_input("tolist(1d6cs>3)", "tolist(1d6cs>3)");
    test_legal_input("avg(1,2,3)", "2");
    test_legal_input("avg([])", "0");
    test_legal_input("len([1d8, 2d8, 3d8] + [4d6, 5d6])", "5");
    test_legal_input("len(tolist(1d6))", "len(tolist(1d6))");
    test_legal_input("rpdice(sum([1d8, 2d8, 3d8] + [4d6, 5d6]))", "12d8+18d6");
    test_legal_input("(1d6)d(1d20)", "(1d6)d(1d20)");
    test_legal_input("floor([1.2, 2.5, 3.7])", "[1,2,3]");
    test_legal_input("ceil([1.2, 2.5, 3.7])", "[2,3,4]");
    test_legal_input("round([1.2, 2.5, 3.7])", "[1,3,4]");
    test_legal_input("abs([-1.5, 2.5, -3.7])", "[1.5,2.5,3.7]");
    test_legal_input("max([1,2,5,4,3], 4-2)", "[5,4]");
    test_legal_input("max([1,2,5,4,1d6], 4-2)", "max([1,2,5,4,1d6],2)");
    test_legal_input("max([1,2,5,4,3], 1d6)", "max([1,2,5,4,3],1d6)");
    test_legal_input("max([1,2,5,4,3], 7)", "[1,2,5,4,3]");
    test_legal_input("max([1,2,5,4,3], 0)", "[]");
    test_legal_input("max([1,2,5,4,3], -1)", "[]");
    test_legal_input("min([1,2,5,4,3], 4-2)", "[1,2]");
    test_legal_input("min([1,2,5,4,1d6], 4-2)", "min([1,2,5,4,1d6],2)");
    test_legal_input("min([1,2,5,4,3], 1d6)", "min([1,2,5,4,3],1d6)");
    test_legal_input("min([1,2,5,4,3], 7)", "[1,2,5,4,3]");
    test_legal_input("min([1,2,5,4,3], 0)", "[]");
    test_legal_input("sum([])", "0");
    test_legal_input("avg([])", "0");
    test_legal_input("sort([3,1,4,2])", "[1,2,3,4]");
    test_legal_input("sort(3,1,4,2)", "[1,2,3,4]");
    test_legal_input("sort([3,1,4,2,1d6])", "sort([3,1,4,2,1d6])");
    test_legal_input("sortd([3,1,4,2])", "[4,3,2,1]");
    test_legal_input("sortd(3,1,4,2)", "[4,3,2,1]");
    test_legal_input("sortd([3,1,4,2,1d6])", "sortd([3,1,4,2,1d6])");
    test_legal_input("filter<>3([1,2,3,4,5])", "[1,2,4,5]");
    test_legal_input("filter<>3(1,2,3,4,5)", "[1,2,4,5]");
    test_legal_input("filter>3([1,2,3,4,5])", "[4,5]");
    test_legal_input("filter>=3([1,2,3,4,5])", "[3,4,5]");
    test_legal_input("filter<3([1,2,3,4,5])", "[1,2]");
    test_legal_input("filter<=3([1,2,3,4,5])", "[1,2,3]");
    test_legal_input("filter=3([1,2,3,4,5])", "[3]");
    test_legal_input("filter=(1d6)([1,2,3,4,5])", "filter=(1d6)([1,2,3,4,5])");
    test_legal_input("filter<3([1d6,2,3,4,5])", "filter<3([1d6,2,3,4,5])");
    test_legal_input("[1,2,3] + tolist(1d6)", "[1,2,3]+tolist(1d6)");
    test_legal_input("[1,2,3]**3", "[1,2,3,1,2,3,1,2,3]");
    test_legal_input("[1,2,3]**(2 * 1 + 1)", "[1,2,3,1,2,3,1,2,3]");
    test_legal_input("3**[1,2,3]", "[1,2,3,1,2,3,1,2,3]");
    test_legal_input("[1d6,2d6,3d6]**3", "[1d6,2d6,3d6,1d6,2d6,3d6,1d6,2d6,3d6]");
    test_legal_input("[1,2,3] + 1", "[2,3,4]");
    test_legal_input("[1,2,3] * 2", "[2,4,6]");
    test_legal_input("[1,2,3] - 1", "[0,1,2]");
    test_legal_input("[1,2,3] / 2", "[0.5,1,1.5]");
    test_legal_input("[1,2,3] // 2", "[0,1,1]");
    test_legal_input("[1,2,3] % 2", "[1,0,1]");
    test_legal_input("2 + [1,2,3]", "[3,4,5]");
    test_legal_input("2 * [1,2,3]", "[2,4,6]");
    test_legal_input("2 - [1,2,3]", "[1,0,-1]");
    test_legal_input("6 / [1,2,3]", "[6,3,2]");
    test_legal_input("6 // [1,2,3]", "[6,3,2]");
    test_legal_input("6 % [1,2,3]", "[0,0,0]");
    test_legal_input("[1,2,3] + 1d6", "[1,2,3]+1d6");
    test_legal_input("[1,2,3] * 1d6", "[1,2,3]*1d6");
    test_legal_input("[1,2,3] - 1d6", "[1,2,3]-1d6");
    test_legal_input("[1,2,3] / 1d6", "[1,2,3]/1d6");
    test_legal_input("[1,2,3] // 1d6", "[1,2,3]//1d6");
    test_legal_input("[1,2,3] % 1d6", "[1,2,3]%1d6");
    test_legal_input("1d6 + [1,2,3]", "[1,2,3]+1d6");
    test_legal_input("1d6 * [1,2,3]", "[1,2,3]*1d6");
    test_legal_input("1d6 - [1,2,3]", "1d6-[1,2,3]");
    test_legal_input("1d6 / [1,2,3]", "1d6/[1,2,3]");
    test_legal_input("1d6 // [1,2,3]", "1d6//[1,2,3]");
    test_legal_input("1d6 % [1,2,3]", "1d6%[1,2,3]");
    test_legal_input("[1d6,2,3] + 1", "[1d6,2,3]+1");
    test_legal_input("[1,2d6,3] * 2", "[1,2d6,3]*2");
    test_legal_input("[1,2,3d6] - 1", "[1,2,3d6]-1");
    test_legal_input("[1d6,2d6,3d6] / 2", "[1d6,2d6,3d6]/2");
    test_legal_input("[1d6,2d6,3d6] // 2", "[1d6,2d6,3d6]//2");
    test_legal_input("[1d6,2d6,3d6] % 2", "[1d6,2d6,3d6]%2");
    test_legal_input("(5/2)d6", "2d6");
    test_legal_input("0d6", "0");
    test_legal_input("6d0", "0");
    test_legal_input("6d(-1)", "0");
    test_legal_input("6d2.7", "6d2");
    test_legal_input("6df", "6dF");
    test_legal_input("6.6df", "6dF");
    test_legal_input("(-1)df", "0");
    test_legal_input("6dc", "6dC");
    test_legal_input("6.6dc", "6dC");
    test_legal_input("(-1)dc", "0");
    test_legal_input("floor(1d6,2,3)", "floor([1d6,2,3])");
    test_legal_input("ceil(1d6,2,3)", "ceil([1d6,2,3])");
    test_legal_input("round(1d6,2,3)", "round([1d6,2,3])");
    test_legal_input("abs(1d6,2,3)", "abs([1d6,2,3])");
    test_legal_input("floor(1d6/2)", "floor(1d6/2)");
    test_legal_input("ceil(1d6/2)", "ceil(1d6/2)");
    test_legal_input("round(1d6/2)", "round(1d6/2)");
    test_legal_input("abs(-1d6)", "abs(-(1d6))");
    test_legal_input("-abs(-1d6)", "-abs(-(1d6))");
    test_legal_input("rpdice(1d6 + 1dF + 1dC)", "2dC+2dF+2d6");
}

#[test]
fn precedence() {
    test_legal_input("(1d6 - 2d6) / 2", "(1d6-2d6)/2");
    test_legal_input("[1,2] + [3,4] * 1d6", "[1,2]+[3,4]*1d6");
    test_legal_input("([1,2] + tolist(1d6)) * 1d6", "([1,2]+tolist(1d6))*1d6");
    test_legal_input("[1,2] + 1d6 + [4,5] + 2d6", "(([1,2]+1d6)+[4,5])+2d6");
    test_legal_input("2d10 - [1,2] - 3", "(2d10-[1,2])-3");
}

#[test]
fn illegal_expressions() {
    test_illegal_input("[1,2,3] ** (2 - 3)");
    test_illegal_input("[1,2,3] ** 1d6");
    test_illegal_input("tolist(1d6) ** 4");
    test_illegal_input("2 / 0");
    test_illegal_input("2 // 0");
    test_illegal_input("2 % 0");
    test_illegal_input("[1,2,3] / 0");
    test_illegal_input("[1,2,3] // 0");
    test_illegal_input("[1,2,3] % 0");
    test_illegal_input("2 / [1,2,0]");
    test_illegal_input("2 // [1,2,0]");
    test_illegal_input("2 % [1,2,0]");
    test_illegal_input("1d6/0/3");
    test_illegal_input("1d6/2/0");
    test_illegal_input("(1/0)d6");
    test_illegal_input("6d(1/0)");
    test_illegal_input("10d6min(1/0)");
    test_illegal_input("(1/0)d6min2");
    test_illegal_input("10d6sf<(1/0)");
    test_illegal_input("10d(1/0)sf<3");
    test_illegal_input("-[1,2,3]");
    test_illegal_input("[1,2,3]d6");
    test_illegal_input("6d[1,2,3]");
    test_illegal_input("[1,2,3]dF");
    test_illegal_input("[1,2,3]dC");
    test_illegal_input("[1,2,[1,2,3]]");
    test_illegal_input("1 ** 2");
    test_illegal_input("[1,2]-[1,2]");
    test_illegal_input("tolist(1,2)");
    test_illegal_input("rpdice(1,2)");
    test_illegal_input("tolist(1)");
    test_illegal_input("10d6cs<3kh");
    test_illegal_input("10d6kh([1,2])");
    test_illegal_input("10d6cs<3!");
    test_illegal_input("6cs<3");
    test_illegal_input("6sf<3");
    test_illegal_input("max(1,2,3,[1,2])");
    test_illegal_input("10d6!!<[1,2]");
    test_illegal_input("10d6!<3lt[1,2]lc10");
    test_illegal_input("10d6!<3lt3lc[1,2]");
    test_illegal_input("max()");
    test_illegal_input("max([])");
    test_illegal_input("min([])");
}

#[test]
fn fold_binary_op() {
    test_legal_input("1d6//1d6", "1d6//1d6");
    test_legal_input("1d6/1d6", "1d6/1d6");
    test_legal_input("1d6%1d6", "1d6%1d6");
    test_legal_input("5%2", "1");
    test_legal_input("1 + 1d6 - 1d6 + 1d8 - 1d8 - 2", "1d8+1d6-1d8-1d6-1");
    test_legal_input("2d6kh + 3d6kh", "2d6kh1+3d6kh1");
    test_legal_input("1 + 0d6 - 0d6 + 0d8 - 0d8 - 2", "-1");
    test_legal_input("1d6/2/3", "1d6/6");
    test_legal_input("(2*60)d6/2/3/4/5", "120d6/120");
    test_legal_input("1d6+1d6", "2d6");
    test_legal_input("1d6 * 0", "0");
    test_legal_input("1d6 * 1", "1d6");
    test_legal_input("3 * 2 * 1d6 * 1", "1d6*6");
    test_legal_input("3 * 2 * 1d6 * 2d6 * 1", "1d6*2d6*6");
    test_legal_input("dF + dF + dC + dC - dF - dC", "2dC-1dC+2dF-1dF");
    test_legal_input("(1d6)dC + (1d6)dF + (1d6)d6", "(1d6)dC+(1d6)dF+(1d6)d6");
    test_legal_input("-1d6 + 1", "-(1d6)+1");
}

#[test]
fn modifiers() {
    test_legal_input("10d6kh(1+1)", "10d6kh2");
    test_legal_input("10d6kl(2*2)", "10d6kl4");
    test_legal_input("10d6dh(5-2)", "10d6dh3");
    test_legal_input("10d6dl(8//2)", "10d6dl4");
    test_legal_input("10d6cs>3df=1", "10d6cs>3df=1");
    test_legal_input("10d6df=1cs>3", "10d6df=1cs>3");
    test_legal_input("10d6cs>3", "10d6cs>3");
    test_legal_input("-(10d6cs>3)", "-(10d6cs>3)");
    test_legal_input("10d6cs>=3", "10d6cs>=3");
    test_legal_input("10d6cs<3", "10d6cs<3");
    test_legal_input("10d6cs<=3", "10d6cs<=3");
    test_legal_input("10d6cs<>3", "10d6cs<>3");
    test_legal_input("10d6df=1", "10d6df=1");
    test_legal_input("10d6max(2*3-1)", "10d6max5");
    test_legal_input("10d6min2", "10d6min2");
    test_legal_input("10d6sf<3", "10d6sf<3");
    test_legal_input("10d6!<3lt3lc10", "10d6!<3lt3lc10");
    test_legal_input("10d6!!<3lt3lc10", "10d6!!<3lt3lc10");
    test_legal_input("10d6!<3lc10", "10d6!<3lc10");
    test_legal_input("10d6!<3lt3", "10d6!<3lt3");
    test_legal_input("10d6!!<3", "10d6!!<3");
    test_legal_input("10d6!!", "10d6!!");
    test_legal_input("10d6r<3lt3lc10", "10d6r<3lt3lc10");
}
