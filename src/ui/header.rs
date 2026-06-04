use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::{App, Screen};

pub fn draw(app: &App, area: Rect, frame: &mut Frame) {
    let title = " notahub ";
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title(Span::styled(
            title,
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let tabs: Vec<(Screen, &str, &str)> = vec![
        (Screen::Home, "1", "Home"),
        (Screen::Projects, "2", "Projects"),
        (Screen::Ideas, "3", "Ideas"),
        (Screen::Search, "/", "Search"),
        (Screen::Help, "?", "Help"),
    ];

    let mut left_spans: Vec<Span> = Vec::new();
    left_spans.push(Span::raw(" "));
    for (i, (screen, key, label)) in tabs.iter().enumerate() {
        let active = app.screen == *screen;
        let style = if active {
            Style::default()
                .fg(ratatui::style::Color::Black)
                .bg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(ratatui::style::Color::Gray)
        };
        let label_txt = format!(" {key} {label} ");
        left_spans.push(Span::styled(label_txt, style));
        if i + 1 < tabs.len() {
            left_spans.push(Span::raw(" "));
        }
    }
    let left_line = Line::from(left_spans);

    let right_line = Line::from(vec![
        Span::styled("Q", Style::default().fg(ratatui::style::Color::Yellow)),
        Span::raw(" quit  "),
        Span::styled("?", Style::default().fg(ratatui::style::Color::Yellow)),
        Span::raw(" help "),
    ]);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(right_line.width() as u16)])
        .split(inner);

    frame.render_widget(Paragraph::new(left_line), chunks[0]);
    frame.render_widget(Paragraph::new(right_line), chunks[1]);
}
