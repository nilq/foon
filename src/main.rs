mod foon;
use foon::*;

fn main() {
    let test = r#"
i32:
  a
  b
  c = 10
  
i128: foo = 123
    "#;

    let lexer = lexer(&mut test.chars());

    let l = lexer.collect();

    println!("{:#?}", l);

    let traveler   = Traveler::new(l);
    let mut parser = Parser::new(traveler);

    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => println!("{:#?}", stuff),
    }
}
