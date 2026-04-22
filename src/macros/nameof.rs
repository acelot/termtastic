#[macro_export]
macro_rules! name_of {
    // Covers Bindings
    ($n: ident) => {{
        let name = nameof::name_of!($n);
        crate::macros::leaking_snake_to_camel(name)
    }};

    // Covers Types
    (type $t: ty) => {{
        let name = nameof::name_of!($t);
        crate::macros::leaking_snake_to_camel(name)
    }};

    // Covers Struct Fields
    ($n: ident in $t: ty) => {{
        let name = nameof::name_of!($n in $t);
        crate::macros::leaking_snake_to_camel(name)
    }};

    // Covers Struct Constants
    (const $n: ident in $t: ty) => {{
        let name = nameof::name_of!($n in $t);
        crate::macros::leaking_snake_to_camel(name)
    }};
}

pub fn leaking_snake_to_camel(snake: &'static str) -> &'static str {
    let mut result = String::with_capacity(snake.len());
    let mut uppercase_next = false;

    for c in snake.chars() {
        if c == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            result.push(c.to_ascii_uppercase());
            uppercase_next = false;
        } else {
            result.push(c);
        }
    }

    Box::leak(result.into_boxed_str())
}
