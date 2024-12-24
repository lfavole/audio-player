//! The code for the random player.
use std::{
    sync::mpsc::{channel, sync_channel},
    thread::scope,
    time::{Duration, SystemTime},
};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled};
use media_controls::media_controls;
use rodio::{Decoder, OutputStream, Sink, Source};
use terminal_ui::{terminal_ui, PartialStatus};
use tinyrand::{Rand, Seeded, StdRand, Wyrand};

use crate::{
    scroll_position::Scrollable,
    secrets::commands::check_secrets_once,
    song::{check_double_songs, EBox, Song},
};

mod keyboard_controls;
mod media_controls;
mod terminal_ui;
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

/// A status message.
#[derive(Clone)]
#[must_use]
pub struct StatusMessage {
    /// The text of the message.
    message: String,
    /// The time when the message will be cleared (represented by a [`Duration`]).
    max_time: SystemTime,
}

impl Default for StatusMessage {
    fn default() -> Self {
        Self {
            message: String::new(),
            max_time: SystemTime::UNIX_EPOCH,
        }
    }
}

impl StatusMessage {
    /// Creates a new [`StatusMessage`].
    pub fn new(message: String, max_time: SystemTime) -> Self {
        Self { message, max_time }
    }

    /// Creates a new [`StatusMessage`] that is cleared after the given [`Duration`].
    pub fn with_duration(message: String, duration: Duration) -> Self {
        Self::new(message, SystemTime::now() + duration)
    }

    /// Creates a new [`StatusMessage`] that is cleared after 5 seconds.
    pub fn five_seconds(message: String) -> Self {
        Self::with_duration(message, Duration::from_secs(5))
    }

    /// Creates a new infinite [`StatusMessage`].
    pub fn infinite(message: String) -> Self {
        Self {
            message,
            // Quite infinite...
            max_time: SystemTime::now() + Duration::from_secs(365 * 86400),
        }
    }
}

/// The status of an active player.
pub(crate) struct Status {
    /// Should we go to the next song when the current one is finished?
    pub go_next: bool,
    /// The length of the queue.
    pub length: usize,
    /// The messages stack.
    pub messages: Vec<StatusMessage>,
    /// The actual position in the queue.
    pub position: usize,
    /// The random number generator to shuffle the queue.
    pub rng: Wyrand,
    /// The position of the currently pointed element.
    pub scrollbar_position: usize,
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
    /// Displays a message.
    DisplayMessage(StatusMessage),
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
    /// Plays the selected song.
    PlaySelected,
    /// Plays the previous song.
    Previous,
    /// Closes the player.
    Quit,
    /// Selects the currently playing song.
    ResetScroll,
    /// Plays the player if it was previously playing before the [`Command::ForcePause`] command.
    RestorePlayback,
    /// Selects one element down.
    ScrollDown,
    /// Selects one element up.
    ScrollUp,
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
        let update_scrollbar_position = status.position == status.scrollbar_position;
        let old_position = status.position;

        match self {
            Self::DisplayMessage(message) => status.messages.insert(0, message),
            Self::ForcePause => sink.pause(),
            Self::Next => {
                status.go_next = false;
                status.position += 1;
                sink.skip_one();
            }
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
                    Self::Play
                } else {
                    Self::Pause
                }
                .handle(sink, status);
            }
            Self::PlaySelected => {
                status.position = status.scrollbar_position;
                status.go_next = false;
                sink.skip_one();
                Self::Play.handle(sink, status);
            }
            Self::Previous => {
                status.go_next = false;
                status.position = status.position.previous(status.length);
                sink.skip_one();
            }
            Self::Quit => {
                status.stop = true;
                sink.skip_one();
            }
            Self::ResetScroll => status.scrollbar_position = status.position,
            Self::RestorePlayback => {
                if !status.was_paused {
                    sink.play();
                }
            }
            Self::ScrollDown => {
                status.scrollbar_position = status.scrollbar_position.next(status.length);
            }
            Self::ScrollUp => {
                status.scrollbar_position = status.scrollbar_position.previous(status.length);
            }
            // Skip seeking if it's not supported
            Self::SeekLeft(duration) => {
                if sink.get_pos() < Duration::from_secs(2) {
                    Self::Previous.handle(sink, status);
                } else {
                    Self::try_seek(sink, sink.get_pos().saturating_sub(duration), status);
                }
            }
            Self::SeekRight(duration) => {
                Self::try_seek(sink, sink.get_pos().saturating_add(duration), status);
            }
            Self::SeekTo(pos) => Self::try_seek(sink, pos, status),
        }

        if old_position != status.position && update_scrollbar_position {
            status.scrollbar_position = status.position;
        }
    }

    fn try_seek(sink: &Sink, pos: Duration, status: &mut Status) {
        if let Err(err) = sink.try_seek(pos) {
            Self::DisplayMessage(StatusMessage::five_seconds(format!("Seek failed: {err:?}")))
                .handle(sink, status);
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

        let (metadata_tx, metadata_rx) = sync_channel(1);

        let (commands_tx, commands_rx) = channel();
        let commands_tx2 = commands_tx.clone();

        check_secrets_once(&commands_tx.clone())?;

        let stop_rx1 = get_stop_rx();
        s.spawn(move || media_controls(commands_tx2, &metadata_rx, &stop_rx1));

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
            go_next: true,
            length: queue.len(),
            messages: vec![],
            position: queue.len(),
            scrollbar_position: 0,
            rng,
            stop: false,
            was_paused: false,
        };

        let (status_tx, status_rx) = sync_channel(1);
        let stop_rx2 = get_stop_rx();
        s.spawn(move || terminal_ui(&status_rx, &stop_rx2, &commands_tx));

        let mut song_names: Vec<String>;

        'mainloop: loop {
            status.shuffle_if_needed(queue);
            song_names = queue.iter().map(|x| x.get_path().to_string()).collect();
            let song = &mut queue[status.position];

            let source = Decoder::new_mp3(song.get_data()?)?;
            let total_time = source.total_duration().unwrap_or(Duration::ZERO);
            sink.append(source);

            metadata_tx.send(Metadata {
                title: song.get_path().to_owned(),
            })?;

            scope(|s2| -> Result<(), EBox> {
                status.go_next = true;

                if status.position + 1 != status.length {
                    let pending_song = &mut queue[status.position + 1];
                    s2.spawn(move || pending_song.preload());
                }

                let mut last_time = sink.get_pos();
                while !sink.empty() {
                    while let Ok(resp) = commands_rx.try_recv() {
                        resp.handle(&sink, &mut status);
                        last_time = Duration::MAX; // force update
                    }
                    if (last_time.abs_diff(sink.get_pos())) > Duration::from_secs(1) {
                        last_time = sink.get_pos();
                        let mut message = String::new();
                        while !status.messages.is_empty() {
                            if SystemTime::now() <= status.messages[0].max_time {
                                message.clone_from(&status.messages[0].message);
                                break;
                            }
                            let _ = status.messages.remove(0);
                        }
                        status_tx.send(PartialStatus {
                            song_names: song_names.clone(),
                            position: status.position,
                            scrollbar_position: status.scrollbar_position,
                            time: sink.get_pos(),
                            total_time,
                            paused: sink.is_paused(),
                            message,
                        })?;
                    }
                }
                if status.go_next {
                    Command::Next.handle(&sink, &mut status);
                }
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
