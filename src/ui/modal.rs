use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{ConfirmState, DeleteMode, InputState};

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn draw_input(state: &InputState, frame: &mut Frame) {
    let area = centered_rect(60, 5, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan))
        .title(Span::styled(
            format!(" {} ", state.label),
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(" > ", Style::default().fg(ratatui::style::Color::Yellow)),
            Span::raw(state.value.clone()),
            Span::styled("▏", Style::default().fg(ratatui::style::Color::Cyan)),
        ]),
        Line::from(Span::styled(
            "   (enter to confirm · esc to cancel)",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

pub fn draw_confirm(state: &ConfirmState, frame: &mut Frame) {
    let area = centered_rect(60, 5, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Red))
        .title(Span::styled(
            " Confirm ",
            Style::default()
                .fg(ratatui::style::Color::Red)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {}", state.message),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(
                " [y]",
                Style::default().fg(ratatui::style::Color::Green),
            ),
            Span::raw(" yes   "),
            Span::styled("[n]", Style::default().fg(ratatui::style::Color::Red)),
            Span::raw(" no "),
        ]),
    ];

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

pub fn draw_delete_prompt(_mode: &DeleteMode, frame: &mut Frame) {
    let area = centered_rect(50, 5, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Red))
        .title(Span::styled(
            " Delete ",
            Style::default()
                .fg(ratatui::style::Color::Red)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Press the shortcut letter of the item to delete",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "  (esc to cancel)",
            Style::default().fg(ratatui::style::Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}

pub fn draw_help(frame: &mut Frame) {
    crate::ui::help::draw(frame);
}
