use proc_macro::{TokenStream, TokenTree};

#[proc_macro]
pub fn load(stream: TokenStream) -> TokenStream {
    let mut iter = stream.into_iter();
    
    // Asset manager arg
    let manager = iter.next().expect("Failed to parse manager arg");
    let manager = match manager {
        TokenTree::Ident(manager) => manager,
        _ => panic!("Manager arg was not an identifier"),
    };

    // Skips comma
    let comma = iter.next().expect("Missing comma");
    match comma {
        TokenTree::Punct(_) => {},
        _ => panic!("Expected comma"),
    }
    if comma.to_string() != String::from(",") {
        panic!("Expected comma");
    }

    // Path arg
    let path = iter.next().expect("Failed to parse path");
    let path: String = match path {
        TokenTree::Literal(path) => path.to_string(),
        _ => panic!("Expected literal"),
    };
    let path = path.trim_matches('"');

    // No more args
    if iter.next().is_some() {
        panic!("Unexpected third argument");
    }

    let path_hash = fxhash::hash64(&path.to_string());
    let result = format!("{manager}.fast_load(\"{path}\", PathHash({path_hash}))");
    result.parse().unwrap()
}
