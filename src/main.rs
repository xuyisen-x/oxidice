use oxidice::parse_dice_and_show;
use oxidice::roll_without_animation;
use std::io::{self, Write};

fn main() {
    loop {
        // 提示符
        print!("请输入表达式（exit 退出）：");
        io::stdout().flush().unwrap();

        // 读取一行输入
        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input).unwrap();

        // Ctrl+D / EOF
        if bytes_read == 0 {
            break;
        }

        let input = input.trim();

        // 空行，跳过
        if input.is_empty() {
            continue;
        }

        // 退出指令
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            break;
        }

        // 调用原来的函数
        match parse_dice_and_show(input) {
            Ok(output) => println!("Parsed HIR: {}", output),
            Err(e) => println!("Error: {}", e),
        }

        match roll_without_animation(input.to_string(), 100, 1000) {
            Ok(result) => println!("Roll result: {:?}", result),
            Err(e) => println!("Error during roll: {}", e),
        }
    }
}
