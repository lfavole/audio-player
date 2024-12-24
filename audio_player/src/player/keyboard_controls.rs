//! Implementation for the keyboard controls.
use std::{sync::mpsc::Sender, time::Duration};

use crossterm::event::{poll, read, Event, KeyCode, KeyEvent};

use crate::{secrets::commands::check_secrets, song::EBox};

use super::{Command, SEEK_STEP};

/// Wait at most 100 milliseconds for an event and handle it.
///
/// # Errors
/// Fails if sending a command fails or if a secret feature fails.
pub fn handle_events(stack: &mut String, tx: &Sender<Command>) -> Result<(), EBox> {
    while poll(Duration::from_millis(100))? {
        let event = read()?;
        if let Event::Key(KeyEvent { code: keycode, .. }) = event {
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
                KeyCode::Up => {
                    tx.send(Command::ScrollUp)?;
                    stack.push('↑');
                }
                KeyCode::Down => {
                    tx.send(Command::ScrollDown)?;
                    stack.push('↓');
                }
                KeyCode::Esc => {
                    tx.send(Command::ResetScroll)?;
                }
                KeyCode::Enter => {
                    tx.send(Command::PlaySelected)?;
                }
                _ => {}
            }
            check_secrets(tx, stack)?;
        }
    }
    Ok(())
}
