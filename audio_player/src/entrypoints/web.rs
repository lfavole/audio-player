//! Macro that generates an entry point for a player using the internet.

/// Play the songs from a specified URL.
#[macro_export]
macro_rules! web {
    ($url:tt) => {
        use ureq::Agent;
        use url::Url;
        use $crate::player::play_songs;
        use $crate::song::{EBox, WebSong};
        use $crate::web_utils::get_files;

        const URL: &str = $url;

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
    };
}
