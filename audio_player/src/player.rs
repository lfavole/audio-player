//! The code for the random player.
use std::time::SystemTime;

use rodio::{Decoder, OutputStream, Sink, Source};
use tinyrand::{Rand, Seeded, StdRand};

use crate::song::{check_double_songs, EBox, Song};

/// Plays the given list of [`Song`]s.
///
/// # Errors
/// Fails:
/// * if the current time cannot be determined
/// * if the output stream or sink cannot be created
/// * if a song cannot be fetched
/// * if a song cannot be decoded
pub fn play_songs<'name, T: Song<'name>>(songs: &mut [T]) -> Result<(), EBox> {
    let mut rng = StdRand::seed(
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs(),
    );
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let queue = &mut songs[..];
    let length = queue.len();
    let mut position = 0;

    if length == 0 {
        println!("No songs to play");
        return Ok(());
    }

    loop {
        if position == 0 {
            // Shuffle the queue
            for i in (1..length).rev() {
                queue.swap(i, rng.next_lim_usize(i + 1));
            }
            check_double_songs(queue);
        }
        let song = &mut queue[position];
        println!("{}", song.get_path());

        let source = Decoder::new_mp3(song.get_data()?)?.buffered();
        sink.append(source);

        if position + 1 != length {
            queue[position + 1].preload()?;
        }

        sink.sleep_until_end();
        if position == length {
            position = 0;
        } else {
            position += 1;
        }
    }
}
