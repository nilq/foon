mod foon;
use foon::*;

fn main() {
    let test = r#"
[i32; 2]: a = {10, 10}

mut [i32; 2]: b =
  10000

b =
  b - 100
"#;

    let lexer = lexer(&mut test.chars());

    let traveler   = Traveler::new(lexer.collect());
    let mut parser = Parser::new(traveler);

    match parser.parse() {
        Err(why)  => println!("error: {}", why),
        Ok(stuff) => println!("{:#?}", stuff),
    }
}
