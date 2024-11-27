//! Play the songs in a specified folder.
use std::path::Path;

use files::RecurseFilesIterator;
use play::play_songs;
use song::{EBox, FileSong};

mod files;
mod play;
mod song;

const FOLDER: &str = "splitted";

fn main() -> Result<(), EBox> {
    let files = RecurseFilesIterator::new(Path::new(FOLDER))?.collect::<Result<Vec<_>, _>>()?;
    let mut songs = files
        .iter()
        .filter(|file| file.extension().is_some_and(|ext| ext == "mp3"))
        .map(|file| FileSong {
            path: file.as_path(),
        })
        .collect::<Vec<_>>();

    play_songs(&mut songs[..])
}
