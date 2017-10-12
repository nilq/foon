mod foon;
use foon::*;

fn main() {
    let test = r#"
i32:
  a = 10

foo = 0

(foo) 1, 2
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
