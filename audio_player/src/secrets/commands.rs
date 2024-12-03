//! The implementation of the secret commands.
use std::{io::Cursor, sync::mpsc::SyncSender};

use crate::{player::Command, song::EBox};
use chrono::{Datelike, Local};
use rodio::{Decoder, OutputStream, Sink};
use super::obfuscation::deobfuscate;

/// Checks if one or many [`Secret`]s can be triggered.
///
/// # Errors
/// Depends on the implementation of the [`Secret`]s.
pub fn check_secrets(tx: &SyncSender<Command>, stack: &mut String) -> Result<(), EBox> {
    Secret1 {}.check(tx, stack)?;
    Ok(())
}

/// Checks if one or many [`SecretOnce`]s can be triggered when the program starts.
///
/// # Errors
/// Depends on the implementation of the [`SecretOnce`]s.
pub fn check_secrets_once(tx: &SyncSender<Command>) -> Result<(), EBox> {
    Secret2 {}.check(tx)?;
    Ok(())
}

/// A secret feature.
#[allow(unused_variables)]
pub trait Secret {
    /// Checks if the [`Secret`] can be triggered.
    ///
    /// # Errors
    /// Depends on the implementation.
    fn check(&mut self, tx: &SyncSender<Command>, stack: &mut String) -> Result<(), EBox> {
        if self.can_be_triggered(tx, stack)? {
            self.trigger(tx, stack)?;
        }
        Ok(())
    }
    /// Checks if the [`Secret`] can be triggered.
    ///
    /// # Errors
    /// Depends on the implementation.
    fn can_be_triggered(&mut self, tx: &SyncSender<Command>, stack: &mut String) -> Result<bool, EBox>;
    /// Trigger the [`Secret`].
    ///
    /// # Errors
    /// Depends on the implementation.
    fn trigger(&mut self, tx: &SyncSender<Command>, stack: &mut String) -> Result<(), EBox>;
}

/// A secret feature that triggers when the program starts.
#[allow(unused_variables)]
pub trait SecretOnce {
    /// Checks if the [`Secret`] can be triggered.
    ///
    /// # Errors
    /// Depends on the implementation.
    fn check(&mut self, tx: &SyncSender<Command>) -> Result<(), EBox> {
        if self.can_be_triggered(tx)? {
            self.trigger(tx)?;
        }
        Ok(())
    }
    /// Checks if the [`Secret`] can be triggered.
    ///
    /// # Errors
    /// Depends on the implementation.
    fn can_be_triggered(&mut self, tx: &SyncSender<Command>) -> Result<bool, EBox>;
    /// Trigger the [`Secret`].
    ///
    /// # Errors
    /// Depends on the implementation.
    fn trigger(&mut self, tx: &SyncSender<Command>) -> Result<(), EBox>;
}

struct Secret1;
impl Secret for Secret1 {
    fn can_be_triggered(&mut self, _tx: &SyncSender<Command>, stack: &mut String) -> Result<bool, EBox> {
        let chars: Vec<u8> = "tblkqmvawbicfizraysbwftntbpyaypnnjhxtflo".into();
        let pwd_chars = deobfuscate(&chars);
        let pwd = String::from_utf8(pwd_chars)?;
        let mut real_pwd = pwd.chars().step_by((chars[2] - chars[1]) as usize / 2).collect::<String>();
        let now = Local::now().date_naive();
        let day = format!("{:02}{:02}", now.day(), now.month());
        real_pwd.push_str(&day);
        Ok(stack.ends_with(&real_pwd))
    }

    fn trigger(&mut self, tx: &SyncSender<Command>, _stack: &mut String) -> Result<(), EBox> {
        tx.send(Command::ForcePause)?;
        let secret = include_bytes!("secret1.bin");
        let real_data = deobfuscate(secret);
        let (_stream, stream_handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&stream_handle)?;
        let source = Decoder::new_mp3(Cursor::new(real_data))?;
        sink.append(source);
        sink.sleep_until_end();
        tx.send(Command::RestorePlayback)?;
        Ok(())
    }
}

#[allow(clippy::cast_possible_truncation)]
fn d(n: u32) -> u32 {
    let s = n.to_string();
    let p: u32 = s.chars().map(|x| x.to_string().parse::<u32>().unwrap()).product();
    p - s.len() as u32
}

#[must_use]
pub fn decode(a: usize, l: usize, r: bool) -> Vec<u8> {
    let s = include_bytes!("secrets.bin");
    let mut s = s.iter()
    .enumerate()
    .skip(a)
    .step_by((s[0] - s[1]) as usize)
    .take(l)
    .map(|(i, c)| {
        if i % 2 == 0 {
            255 - *c
        } else {
            *c
        }
    })
    .collect::<Vec<_>>();
    if r {
        s.reverse();
    }
    s
}

#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn decode_string(a: usize, l: usize, r: bool) -> String {
    String::from_utf8(decode(a, l, r)).unwrap()
}

#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn decode_number(chars: &[u8]) -> u32 {
    let mut n: u32 = 0;
    for (i, c) in chars.iter().rev().enumerate() {
        n += u32::from(*c) * 256u32.pow(i as u32);
    }
    n
}

struct Secret2;
impl SecretOnce for Secret2 {
    fn can_be_triggered(&mut self, _tx: &SyncSender<Command>) -> Result<bool, EBox> {
        let n = d(decode_number(&decode(20, 3, false)));
        let now = Local::now().date_naive();
        let (a, b) = (now.day(), now.month());
        Ok((a.saturating_sub(b) * (a + b)).saturating_sub(a + b) == n)
    }

    fn trigger(&mut self, _tx: &SyncSender<Command>) -> Result<(), EBox> {
        println!("{}", decode_string(23, 24, false));
        Ok(())
    }
}
