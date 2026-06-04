pub mod header;
pub mod help;
pub mod home;
pub mod ideas;
pub mod modal;
pub mod projects;
pub mod search;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, InputAction, Screen};

pub fn draw(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(5),    // body
            Constraint::Length(2), // footer / hints
        ])
        .split(frame.area());

    header::draw(app, chunks[0], frame);

    // When help is open, render the previous screen as the background, then overlay
    // the help as a centered modal.
    let body_screen = if app.screen == Screen::Help {
        app.prev_screen
    } else {
        app.screen
    };
    match body_screen {
        Screen::Home => home::draw(app, chunks[1], frame),
        Screen::Projects => projects::draw(app, chunks[1], frame),
        Screen::Ideas => ideas::draw(app, chunks[1], frame),
        Screen::Search => search::draw(app, chunks[1], frame),
        Screen::Help => {} // unreachable
    }

    draw_footer(app, chunks[2], frame);

    // Overlays
    if let Some(state) = app.input.as_ref() {
        // Search uses the top bar as the input; skip the centered modal for it.
        if !matches!(state.action, InputAction::Search) {
            modal::draw_input(state, frame);
        }
    } else if let Some(state) = app.confirm.as_ref() {
        modal::draw_confirm(state, frame);
    } else if let Some(mode) = app.delete_mode.as_ref() {
        modal::draw_delete_prompt(mode, frame);
    }

    if app.screen == Screen::Help {
        modal::draw_help(frame);
    }
}

fn draw_footer(app: &App, area: Rect, frame: &mut Frame) {
    use ratatui::style::{Modifier, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Block, Borders, Paragraph};

    let hints = match app.screen {
        crate::app::Screen::Home => {
            "p new project · i new idea · tab switch focus · x delete · / search · ? help · Q quit"
        }
        crate::app::Screen::Projects => {
            "tab cycle · p new project · t new task · n new note · space toggle · x delete · esc back"
        }
        crate::app::Screen::Ideas => {
            "tab cycle · i new idea · n new note · x delete · esc back"
        }
        crate::app::Screen::Search => {
            "type to search · ↓ ↑ / j k navigate · enter open · esc close"
        }
        crate::app::Screen::Help => "press ? / enter / esc to close",
    };

    let status_span = app
        .status
        .as_deref()
        .map(|s| Span::styled(format!(" {s} "), Style::default().add_modifier(Modifier::BOLD)))
        .unwrap_or_else(|| Span::raw(""));

    let line = Line::from(vec![
        Span::styled(hints, Style::default().fg(ratatui::style::Color::DarkGray)),
        Span::raw("  "),
        status_span,
    ]);

    let p = Paragraph::new(line).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(ratatui::style::Color::DarkGray)),
    );
    frame.render_widget(p, area);
}
