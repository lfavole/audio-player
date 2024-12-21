//! The code for the random player.
use std::{
    sync::mpsc::sync_channel,
    thread::{scope, sleep},
    time::{Duration, SystemTime},
};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};
use keyboard_controls::controls;
use media_controls::media_controls;
use rodio::{Decoder, OutputStream, Sink};
use tinyrand::{Rand, Seeded, StdRand, Wyrand};

use crate::{
    secrets::commands::check_secrets_once,
    song::{check_double_songs, EBox, Song},
};

mod keyboard_controls;
mod media_controls;
#[cfg(windows)]
pub mod window;

/// Prints the given text in non-raw mode
///
/// i.e. disables the raw mode, prints the text and re-enables it.
macro_rules! println_not_raw {
    ($($arg:tt)*) => {
        if is_raw_mode_enabled()? {
            disable_raw_mode()?;
            println!($($arg)*);
            enable_raw_mode()?;
        } else {
            println!($($arg)*);
        }
    };
}

/// The status of an active player.
pub(crate) struct Status {
    /// Should we go to the next song when the current one is finished?
    pub change_pos: bool,
    /// The length of the queue.
    pub length: usize,
    /// The actual position in the queue.
    pub position: usize,
    /// The random number generator to shuffle the queue.
    pub rng: Wyrand,
    /// Should we stop the player?
    pub stop: bool,
    /// Was the song paused before the call to [`Command::ForcePause`]?
    pub was_paused: bool,
}

impl Status {
    /// Shuffle the queue if `status.position == status.length`.
    fn shuffle_if_needed<'queue, 'name, T: Song<'name> + 'name>(&mut self, queue: &'queue mut [T]) {
        if self.position == self.length {
            self.position = 0;
            // Shuffle the queue
            for i in (1..queue.len()).rev() {
                queue.swap(i, self.rng.next_lim_usize(i + 1));
            }
            check_double_songs(queue);
        }
    }
}

/// A command that can be sent to an active player to change its behavior.
pub enum Command {
    /// Pauses the player.
    ForcePause,
    /// Plays the next song.
    Next,
    /// Pauses the player.
    Pause,
    /// Plays the player.
    Play,
    /// Plays (or pauses) the player.
    PlayPause,
    /// Plays the previous song.
    Previous,
    /// Closes the player.
    Quit,
    /// Plays the player if it was previously playing before the [`Command::ForcePause`] command.
    RestorePlayback,
    /// Seeks backwards of the given duration.
    SeekLeft(Duration),
    /// Seeks forwards of the given duration.
    SeekRight(Duration),
    /// Seeks to a given position.
    SeekTo(Duration),
}

/// The seek step when seeking with arrow keys.
static SEEK_STEP: Duration = Duration::from_secs(5);

impl Command {
    /// Apply a command on a [`Sink`] and on a [`Status`].
    fn handle(self, sink: &Sink, status: &mut Status) {
        match self {
            Self::ForcePause => sink.pause(),
            Self::Next => sink.skip_one(),
            Self::Pause => {
                sink.pause();
                status.was_paused = true;
            }
            Self::Play => {
                sink.play();
                status.was_paused = false;
            }
            Self::PlayPause => {
                if sink.is_paused() {
                    sink.play();
                    status.was_paused = false;
                } else {
                    sink.pause();
                    status.was_paused = true;
                }
            }
            Self::Previous => {
                sink.skip_one();
                status.change_pos = false;
                if status.position == 0 {
                    status.position = status.length;
                } else {
                    status.position -= 1;
                }
            }
            Self::Quit => {
                status.stop = true;
                sink.skip_one();
            }
            Self::RestorePlayback => {
                if !status.was_paused {
                    sink.play();
                }
            }
            // Skip seeking if it's not supported
            Self::SeekLeft(duration) => {
                if sink.get_pos() < Duration::from_secs(2) {
                    sink.skip_one();
                    status.change_pos = false;
                    if status.position == 0 {
                        status.position = status.length;
                    } else {
                        status.position -= 1;
                    }
                    return;
                }
                if let Err(err) = sink.try_seek(sink.get_pos().saturating_sub(duration)) {
                    eprintln!("Seek failed: {err:?}");
                }
            }
            Self::SeekRight(duration) => {
                if let Err(err) = sink.try_seek(sink.get_pos().saturating_add(duration)) {
                    eprintln!("Seek failed: {err:?}");
                }
            }
            Self::SeekTo(pos) => {
                if let Err(err) = sink.try_seek(pos) {
                    eprintln!("Seek failed: {err:?}");
                }
            }
        }
    }
}

/// Owned metadata for a [`Song`].
pub struct Metadata {
    /// The title of the [`Song`] (currently the file name).
    title: String,
}

/// Plays the given list of [`Song`]s.
///
/// # Errors
/// Fails:
/// * if the current time cannot be determined
/// * if the output stream or sink cannot be created
/// * if a song cannot be fetched
/// * if a song cannot be decoded
pub fn play_songs<'name, T: Song<'name> + 'name>(songs: &mut [T]) -> Result<(), EBox> {
    scope(|s| -> Result<(), EBox> {
        let mut stop_list = vec![];
        let mut get_stop_rx = || {
            let (stop_tx, stop_rx) = sync_channel(1);
            stop_list.push(stop_tx);
            stop_rx
        };

        let (status_tx, status_rx) = sync_channel(1);

        let (tx, rx) = sync_channel(1);
        let tx2 = tx.clone();
        check_secrets_once(&tx.clone())?;
        let stop_rx1 = get_stop_rx();
        s.spawn(move || controls(&tx.clone(), &stop_rx1));
        let stop_rx2 = get_stop_rx();
        s.spawn(move || media_controls(tx2, &status_rx, &stop_rx2));

        let rng = StdRand::seed(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        );
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        let queue = &mut *songs;

        if queue.is_empty() {
            println_not_raw!("No songs to play");
            return Ok(());
        }

        let mut status = Status {
            change_pos: true,
            length: queue.len(),
            position: queue.len(),
            rng,
            stop: false,
            was_paused: false,
        };

        'mainloop: loop {
            status.shuffle_if_needed(queue);
            let song = &mut queue[status.position];
            println_not_raw!("{}", song.get_path());

            let source = Decoder::new_mp3(song.get_data()?)?;
            sink.append(source);

            status_tx.send(Metadata {
                title: song.get_path().to_owned(),
            })?;

            scope(|s2| -> Result<(), EBox> {
                if status.position + 1 != status.length {
                    let pending_song = &mut queue[status.position + 1];
                    s2.spawn(move || pending_song.preload());
                }

                while !sink.empty() {
                    if let Ok(resp) = rx.try_recv() {
                        resp.handle(&sink, &mut status);
                    }
                    sleep(Duration::from_millis(100));
                }
                if status.change_pos {
                    status.position += 1;
                }
                status.change_pos = true;
                Ok(())
            })?;
            if status.stop {
                for tx in stop_list {
                    tx.send(())?;
                }
                break 'mainloop;
            }
        }
        Ok(())
    })
}
