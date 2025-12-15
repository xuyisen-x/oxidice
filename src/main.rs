pub mod grammar;
pub mod types;

fn main() {
    let input = "2d20cs<=15df=20";
    match grammar::parse_dice(input) {
        Ok(ast) => println!("Parsed AST: {:?}", ast),
        Err(e) => println!("Parse error: {}", e),
    }
}
