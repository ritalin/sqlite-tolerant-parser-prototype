mod api;

pub fn main() {
    let source = "SELECT * FROM foo;";
    let scanner = api::Scanner::create(source.into(), 0);

    while let Some(lookahead) = scanner.shift() {
        if let Some(items) = lookahead.leading {
            items.into_iter().for_each(|item| print_token_item(item, "Leading"));
        }

        print_token_item(lookahead.main, "Main");

        if let Some(items) = lookahead.trailing {
            items.into_iter().for_each(|item| print_token_item(item, "Trailing"));
        }
    }
}

fn print_token_item(item: api::TokenItem, ctx: &str) {
    println!("[{:>8}] kind/id: {}, kind/name: {}, offset: {}, len: {}, value: {:?}", 
        ctx, item.kind.id, item.kind.text, item.offset, item.len, item.value
    );
}