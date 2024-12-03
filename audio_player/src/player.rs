//! The code for the random player.
use std::{sync::mpsc::{sync_channel, Receiver, SyncSender}, thread::sleep, time::{Duration, SystemTime}};

use crossterm::{event::{Event, KeyCode, KeyEvent}, terminal::{disable_raw_mode, enable_raw_mode, is_raw_mode_enabled}};
use rodio::{Decoder, OutputStream, Sink, Source};
use tinyrand::{Rand, Seeded, StdRand};

use crate::{secrets::commands::{check_secrets, check_secrets_once}, song::{check_double_songs, EBox, Song}};

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

/// A command that can be sent to an active player to change its behavior.
pub enum Command {
    /// Pauses the player.
    ForcePause,
    /// Plays the next song.
    Next,
    /// Plays (or pauses) the player.
    PlayPause,
    /// Plays the previous song.
    Previous,
    /// Closes the player.
    Quit,
    /// Plays the player if it was previously playing before the [`ForcePause`] command.
    RestorePlayback,
    /// Seeks backwards.
    SeekLeft,
    /// Seeks forwards.
    SeekRight,
}

fn controls(tx: &SyncSender<Command>) -> Result<(), EBox> {
    enable_raw_mode()?;
    let _ = std::io::stdout();
    let mut stack = String::with_capacity(50);
    loop {
        if let Event::Key(KeyEvent { code: KeyCode::Char(char), .. }) = crossterm::event::read()? {
            match char {
                ' ' => {tx.send(Command::PlayPause)?;},
                'n' => {tx.send(Command::Next)?;},
                'p' => {tx.send(Command::Previous)?;},
                'q' => {tx.send(Command::Quit)?; break;},
                _ => {},
            }
            stack.push(char);
            check_secrets(tx, &mut stack)?;
        }
    }
    disable_raw_mode()?;
    Ok(())
}

/// Plays the given list of [`Song`]s.
///
/// # Errors
/// Fails:
/// * if the current time cannot be determined
/// * if the output stream or sink cannot be created
/// * if a song cannot be fetched
/// * if a song cannot be decoded
pub fn play_songs<'name, T: Song<'name>>(songs: &mut [T]) -> Result<(), EBox> {
    std::thread::scope(|s| -> Result<(), EBox> {
        let (tx, rx): (SyncSender<Command>, Receiver<Command>) = sync_channel(1);
        check_secrets_once(&tx.clone())?;
        let tx2 = tx.clone();
        s.spawn(move || controls(&tx));

        let mut rng = StdRand::seed(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs(),
        );
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;

        let queue = &mut songs[..];
        let length = queue.len();
        let mut position = length;

        if length == 0 {
            println_not_raw!("No songs to play");
            return Ok(());
        }

        'mainloop: loop {
            if position == length {
                position = 0;
                // Shuffle the queue
                for i in (1..length).rev() {
                    queue.swap(i, rng.next_lim_usize(i + 1));
                }
                check_double_songs(queue);
            }
            let song = &mut queue[position];
            println_not_raw!("{}", song.get_path());

            let source = Decoder::new_mp3(song.get_data()?)?.buffered();
            sink.append(source);

            let mut stop = false;
            std::thread::scope(|s2| -> Result<(), EBox> {
                if position + 1 != length {
                    let pending_song = &mut queue[position + 1];
                    s2.spawn(move || pending_song.preload());
                }

                let mut change_pos = true;
                // Was the song paused before the call to ForcePause?
                let mut was_paused = false;
                while !sink.empty() {
                    let seek_step = Duration::from_secs(5);
                    if let Ok(resp) = rx.try_recv() {
                        match resp {
                            Command::ForcePause => sink.pause(),
                            Command::Next => sink.skip_one(),
                            Command::PlayPause => if sink.is_paused() {sink.play(); was_paused = false} else {sink.pause(); was_paused = true},
                            Command::Previous => {
                                sink.skip_one();
                                change_pos = false;
                                if position == 0 {
                                    position = length;
                                } else {
                                    position -= 1;
                                }
                            },
                            Command::Quit => {stop = true; sink.skip_one()},
                            Command::RestorePlayback => if !was_paused {sink.play()},
                            // Skip seeking if it's not supported
                            Command::SeekLeft => {
                                if sink.get_pos() < Duration::from_secs(2) {
                                    // Ensure the next song doesn't play
                                    sink.pause();
                                    was_paused = true;
                                    tx2.send(Command::Previous)?;
                                } else if let Err(err) = sink.try_seek(sink.get_pos().saturating_sub(seek_step)) {
                                    eprintln!("Seek failed: {err:?}");
                                }
                            },
                            Command::SeekRight => {
                                if let Err(err) = sink.try_seek(sink.get_pos().saturating_add(seek_step)) {
                                    eprintln!("Seek failed: {err:?}");
                                }
                            },
                        };
                    }
                    sleep(Duration::from_millis(100));
                }
                if change_pos {
                    position += 1;
                }
                Ok(())
            })?;
            if stop {
                break 'mainloop;
            }
        }
        Ok(())
    })
}
