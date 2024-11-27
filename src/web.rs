//! Play the songs from a specified URL.
use play::play_songs;
use song::{EBox, WebSong};
use ureq::Agent;
use url::Url;
use web_utils::get_files;

mod files;
mod play;
mod song;
mod web_utils;

const URL: &str = "http://127.0.0.1:8000/";

fn main() -> Result<(), EBox> {
    let agent = Agent::new();
    let url = Url::parse(URL)?;
    let files = get_files(&agent, &url)?;

    let mut songs = files
        .iter()
        .map(|url| WebSong::new(url, &agent))
        .collect::<Vec<_>>();
    play_songs(&mut songs[..])
}
