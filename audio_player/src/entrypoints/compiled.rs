//! Macro that generates an entry point for a binary with compiled songs.

/// Compiles the songs into the binary and play them.
#[macro_export]
macro_rules! compiled {
    ($folder:tt) => {
        use macros::include_songs;
        use $crate::player::play_songs;
        use $crate::song::{Compiled, EBox};

        static MUSIC_DIR: &[Compiled] = include_songs!($folder);

        fn main() -> Result<(), EBox> {
            let mut songs = MUSIC_DIR.to_vec();
            play_songs(&mut songs[..])
        }
    };
}
