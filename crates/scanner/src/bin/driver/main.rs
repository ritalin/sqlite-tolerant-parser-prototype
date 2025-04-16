

pub fn main() -> Result<(), anyhow::Error> {
    let source = r#"
    /* 行頭Comment */
    FROM (
        /* なんたらかんたら */
        SELECT t.*, 'abc' || 'xyz' AS x, 123 / 456 AS y 
        FROM foo t 
        WHERE t.code = 10 -- 条件
    )
    "#;

    let mut scanner = scanner::Scanner::create(source)?;

    while let Some(token) = scanner.lookahead() {
        println!("leading : {:?}", token.leading);
        println!("main    : {:?}", token.main);
        println!("trailing: {:?}", token.trailing);
        println!("--------------------");
        scanner.shift();
    }

    Ok(())
}