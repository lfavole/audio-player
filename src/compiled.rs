//! Compile the songs into the binary and play them.
use audio_player::include_songs;
use play::play_songs;
use song::CompiledSong;

mod play;
mod song;

static MUSIC_DIR: &[CompiledSong] = include_songs!("/home/lfavole/Musique/Concentration/Compilation piano/splitted");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut songs = MUSIC_DIR.to_vec();
    play_songs(&mut songs[..]).await
}
