//! Code for the [`include_songs!`] macro.
use files::RecurseFilesIterator;
use std::path::{PathBuf, MAIN_SEPARATOR};

use proc_macro::{TokenStream, TokenTree};
use quote::quote;

/// Returns a slice of [`CompiledSong`](../audio_player/song/struct.CompiledSong.html)s
/// that are in a specified directory.
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
        // Go one level up because we are in the macros directory
        [
            env!("CARGO_MANIFEST_DIR"),
            "..",
            path.to_str().expect("non UTF-8 path"),
        ]
        .iter()
        .collect()
    };
    let path_str = path.to_str().expect("non UTF-8 base path");
    let files = RecurseFilesIterator::new(&path).expect("failed to get the file list");

    let mut tokens = vec![];

    for file in files {
        let file = file.expect("failed to get the file");
        if file.extension().unwrap_or_default() != "mp3" {
            continue;
        }
        let abs = file.to_str().expect("non UTF-8 file in base path");
        let rel = abs
            .strip_prefix(path_str)
            .expect("file is not relative to base path")
            .trim_start_matches(MAIN_SEPARATOR);
        #[cfg(any(clippy, test, doctest, feature = "test"))]
        tokens.push(quote!(audio_player::song::Compiled::empty(#rel)));
        #[cfg(all(not(clippy), not(test), not(doctest), not(feature = "test")))]
        tokens.push(quote!(audio_player::song::Compiled::new(#rel, include_bytes!(#abs))));
    }

    quote! {
        &[#(#tokens),*]
    }
    .into()
}

/// Gets a [`String`] from a [`proc_macro::Literal`].
///
/// Inspired from <https://docs.rs/include_dir_macros/0.7.4/src/include_dir_macros/lib.rs.html#31>.
///
/// # Panics
/// Panics if the [`proc_macro::Literal`] doesn't contain a single string argument.
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
