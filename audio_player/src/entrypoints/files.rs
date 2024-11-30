//! Macro that generates an entry point for a player using the local file system.

/// Plays the songs in a specified folder.
#[macro_export]
macro_rules! files {
    ($folder:tt) => {
        use std::path::Path;

        use files::RecurseFilesIterator;
        use $crate::player::play_songs;
        use $crate::song::{EBox, File};

        const FOLDER: &str = $folder;

        fn main() -> Result<(), EBox> {
            let files =
                RecurseFilesIterator::new(Path::new(FOLDER))?.collect::<Result<Vec<_>, _>>()?;
            let mut songs = files
                .iter()
                .filter(|file| file.extension().is_some_and(|ext| ext == "mp3"))
                .map(|file| File::new(file))
                .collect::<Vec<_>>();

            play_songs(&mut songs[..])
        }
    };
}
