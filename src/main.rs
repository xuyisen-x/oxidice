pub mod grammar;
pub mod types;

fn main() {
    let input = "10d6!kh3r<3lt2lc2";
    match grammar::parse_dice(input) {
        Ok(ast) => println!("Parsed AST: {:?}", ast),
        Err(e) => println!("Parse error: \n{}", e),
    }
}
