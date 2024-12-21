//! Implementation for the media controls.
use std::{
    sync::mpsc::{sync_channel, Receiver, SyncSender},
    time::Duration,
};

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, PlatformConfig, SeekDirection};

use crate::{generic_error::GenericError, player::SEEK_STEP, song::EBox};

use super::{Command, Metadata};

/// Register media controls.
///
/// Inspired from <https://github.com/Sinono3/souvlaki#example>.
pub fn media_controls(tx: SyncSender<Command>, status_rx: &Receiver<Metadata>, stop_rx: &Receiver<()>) -> Result<(), EBox> {
    #[cfg(target_os = "windows")]
    let hwnd = {
        use super::window::run_window;
        use std::thread::spawn;

        let (tx, rx) = sync_channel(1);
        spawn(|| run_window(tx));
        Some(rx.recv()??.0)
    };
    #[cfg(not(target_os = "windows"))]
    let hwnd = None;

    let config = PlatformConfig {
        dbus_name: "audio_player",
        display_name: "audio-player",
        hwnd,
    };

    let mut controls = MediaControls::new(config).map_err(GenericError::from)?;

    let (tx_error, rx_error) = sync_channel(1);

    let callback = move |event: MediaControlEvent| -> Result<(), EBox> {
        match event {
            MediaControlEvent::Play => tx.send(Command::Play)?,
            MediaControlEvent::Pause => tx.send(Command::Pause)?,
            MediaControlEvent::Toggle => tx.send(Command::PlayPause)?,
            MediaControlEvent::Next => tx.send(Command::Next)?,
            MediaControlEvent::Previous => tx.send(Command::Previous)?,
            MediaControlEvent::Stop => tx.send(Command::Quit)?,
            MediaControlEvent::Seek(direction) => {
                tx.send(match direction {
                    SeekDirection::Backward => Command::SeekLeft(SEEK_STEP),
                    SeekDirection::Forward => Command::SeekRight(SEEK_STEP),
                })?;
            }
            MediaControlEvent::SeekBy(direction, duration) => {
                tx.send(match direction {
                    SeekDirection::Backward => Command::SeekLeft(duration),
                    SeekDirection::Forward => Command::SeekRight(duration),
                })?;
            }
            MediaControlEvent::SetPosition(pos) => tx.send(Command::SeekTo(pos.0))?,
            MediaControlEvent::Quit => tx.send(Command::Quit)?,
            _ => {}
        };
        Ok(())
    };

    controls
        .attach(move |event: MediaControlEvent| {
            #[expect(clippy::unwrap_used)]
            if let Err(err) = callback(event) {
                tx_error.send(Err(err)).unwrap();
            }
        })
        .map_err(GenericError::from)?;

    loop {
        if let Ok(err) = rx_error.try_recv() {
            err?;
        }
        if let Ok(metadata) = status_rx.try_recv() {
            // Update the media metadata
            controls
                .set_metadata(MediaMetadata {
                    title: Some(metadata.title.as_str()),
                    ..Default::default()
                })
                .map_err(GenericError::from)?;
        }
        if stop_rx.recv_timeout(Duration::from_millis(100)).is_ok() {
            break;
        }
    }
    Ok(())
}
