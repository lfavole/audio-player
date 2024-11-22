//! Play the songs in a specified folder.
use std::path::Path;

use files::recurse_files;
use play::play_songs;
use song::{EBox, FileSong};

mod files;
mod play;
mod song;

const FOLDER: &str = "splitted";

#[tokio::main]
async fn main() -> Result<(), EBox> {
    let files = recurse_files(Path::new(FOLDER))?;
    let mut songs = files.iter()
    .filter(| x | x.extension().unwrap_or_default() == "mp3")
    .map(| x | FileSong { path: x.as_path() })
    .collect::<Vec<_>>();

    play_songs(&mut songs[..]).await
}
