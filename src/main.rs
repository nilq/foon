mod foon;
use foon::*;

fn main() {
    let test = r#"
i32: a = 100
f32: b = 0.123

mut: c = r"a string idk"
c = "new string ok"

mut i128: d = 2^128
    "#;
    
    let lexer = lexer(&mut test.chars());
    
    let traveler   = Traveler::new(lexer.collect());
    let mut parser = Parser::new(traveler);
    
    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => println!("{:#?}", stuff),
    }
}
