//! Implementation for the keyboard controls.
use std::{sync::mpsc::{Receiver, SyncSender}, time::Duration};

use crossterm::{
    event::{poll, read, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{secrets::commands::check_secrets, song::EBox};

use super::{Command, SEEK_STEP};

/// Reads the pressed keys and ask to perform the corresponding actions.
///
/// # Errors
/// Fails if a key [`Event`] cannot be read.
pub fn controls(tx: &SyncSender<Command>, stop_rx: &Receiver<()>) -> Result<(), EBox> {
    enable_raw_mode()?;
    let mut stack = String::with_capacity(50);
    loop {
        if poll(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent { code: keycode, .. }) = read()? {
                match keycode {
                    KeyCode::Char(char) => {
                        match char {
                            ' ' => {
                                tx.send(Command::PlayPause)?;
                            }
                            'n' => {
                                tx.send(Command::Next)?;
                            }
                            'p' => {
                                tx.send(Command::Previous)?;
                            }
                            'q' => {
                                tx.send(Command::Quit)?;
                            }
                            _ => {}
                        }
                        stack.push(char);
                    }
                    KeyCode::Left => {
                        tx.send(Command::SeekLeft(SEEK_STEP))?;
                        stack.push('←');
                    }
                    KeyCode::Right => {
                        tx.send(Command::SeekRight(SEEK_STEP))?;
                        stack.push('→');
                    }
                    KeyCode::Media(c) => {
                        println!("{c:?}");
                    }
                    _ => {}
                }
                check_secrets(tx, &mut stack)?;
            }
        }
        if stop_rx.try_recv().is_ok() {
            break;
        }
    }
    disable_raw_mode()?;
    Ok(())
}
