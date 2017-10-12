mod foon;
use foon::*;

fn main() {
    let test = r#"
i32: a = 10
    "#;
    
    let lexer = lexer(&mut test.chars());
    
    for n in lexer {
        println!("{:#?}", n)
    }
}
