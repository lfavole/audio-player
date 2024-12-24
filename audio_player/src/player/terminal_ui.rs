//! Ratatui test.
use ratatui::{
    layout::{Constraint, Layout, Margin},
    style::{Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, LineGauge, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
    Frame,
};
use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

use crate::song::EBox;

use super::{keyboard_controls::handle_events, Command};

pub struct PartialStatus {
    pub song_names: Vec<String>,
    pub position: usize,
    pub scrollbar_position: usize,
    pub time: Duration,
    pub total_time: Duration,
    pub paused: bool,
    pub message: String,
}

/// Runs the terminal UI.
///
/// # Errors
/// Fails if the terminal can't be opened, if the metadata can't be received or if [`handle_events`] fails.
pub fn terminal_ui(
    status_rx: &Receiver<PartialStatus>,
    stop_rx: &Receiver<()>,
    tx: &Sender<Command>,
) -> Result<(), EBox> {
    let mut terminal = ratatui::try_init()?;

    let mut stack = String::with_capacity(50);
    let mut status = status_rx.recv()?;

    loop {
        handle_events(&mut stack, tx)?;

        if let Ok(status_inner) = status_rx.try_recv() {
            status = status_inner;
        }
        terminal.draw(|frame| ui(frame, &status))?;

        if stop_rx.try_recv().is_ok() {
            break;
        }
    }

    ratatui::try_restore()?;
    Ok(())
}

fn format_duration(d: Duration) -> String {
    let seconds = d.as_secs();
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Draws the UI.
fn ui(frame: &mut Frame, status: &PartialStatus) {
    let items: Vec<ListItem> = status
        .song_names
        .iter()
        .enumerate()
        .map(|(i, song)| {
            ListItem::new(
                (if i == status.position { "> " } else { "  " }).to_owned() + song.as_str(),
            )
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(status.scrollbar_position));

    frame.render_widget(
        Block::bordered()
            .title(Line::from("Audio player by lfavole").centered())
            .border_type(BorderType::Rounded),
        frame.area(),
    );

    let [main_area, message_area, status_area] = Layout::vertical([
        Constraint::Min(0),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame.area().inner(Margin::new(1, 1)));

    let widget = List::new(items).highlight_style(Style::new().on_gray());

    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
    let mut scrollbar_state =
        ScrollbarState::new(status.song_names.len()).position(status.scrollbar_position);

    frame.render_stateful_widget(widget, main_area, &mut state);

    frame.render_widget(Text::from(status.message.clone()), message_area);

    frame.render_stateful_widget(
        scrollbar,
        main_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        }),
        &mut scrollbar_state,
    );
    if !status.total_time.is_zero() {
        let ratio = status.time.as_nanos() as f64 / status.total_time.as_nanos() as f64;
        if (0.0..=1.0).contains(&ratio) {
            let label = format!(
                "{}{} / {}",
                if status.paused { "Paused " } else { "" },
                format_duration(status.time),
                format_duration(status.total_time)
            );
            frame.render_widget(
                LineGauge::default()
                    .ratio(ratio)
                    .filled_style(Style::default().blue())
                    .unfilled_style(Style::default().gray())
                    .label(label),
                status_area,
            );
        }
    }
}
