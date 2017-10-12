mod foon;
use foon::*;

fn main() {
    let test = r#"
10 + a
    "#;
    
    let lexer = lexer(&mut test.chars());
    
    let traveler   = Traveler::new(lexer.collect());
    let mut parser = Parser::new(traveler);
    
    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => println!("{:#?}", stuff),
    }
}
