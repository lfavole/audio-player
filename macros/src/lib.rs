//! Code for the `include_songs!` macro.
use files::RecurseFilesIterator;
use std::path::PathBuf;

use proc_macro::{TokenStream, TokenTree};
use quote::quote;

/// Return a slice of `Song`s that are in a specified directory.
/// The songs are embedded into the program.
///
/// # Panics
/// Panics if the input is not a string or if a file has a non-UTF-8 name.
#[proc_macro]
pub fn include_songs(input: TokenStream) -> TokenStream {
    let tokens: Vec<_> = input.into_iter().collect();

    let path: PathBuf = match tokens.as_slice() {
        [TokenTree::Literal(lit)] => unwrap_string_literal(lit),
        _ => panic!("This macro only accepts a single, non-empty string argument"),
    }
    .into();
    // Make the path relative to the manifest directory
    let path = if path.is_absolute() {
        path
    } else {
        [env!("CARGO_MANIFEST_DIR"), "..", path.to_str().unwrap()]
            .iter()
            .collect()
    };
    let files = RecurseFilesIterator::new(&path).unwrap();

    let mut tokens = vec![];

    for file in files {
        let file = file.unwrap();
        if file.extension().unwrap_or_default() != "mp3" {
            continue;
        }
        let abs = file.to_str().unwrap();
        let rel = file.strip_prefix(&path).unwrap().to_str().unwrap();
        tokens.push(
            quote!(audio_player::song::CompiledSong { path: #rel, data: include_bytes!(#abs) }),
        );
    }

    quote! {
        &[#(#tokens),*]
    }
    .into()
}

// Inspired from https://docs.rs/include_dir_macros/0.7.4/src/include_dir_macros/lib.rs.html#31
fn unwrap_string_literal(lit: &proc_macro::Literal) -> String {
    let mut repr = lit.to_string();
    assert!(
        repr.starts_with('"') && repr.ends_with('"'),
        "This macro only accepts a single, non-empty string argument"
    );
    repr.remove(0);
    repr.pop();
    repr
}
