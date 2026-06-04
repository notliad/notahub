use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::{App, SearchKind};

pub fn draw(app: &App, area: Rect, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3)])
        .split(area);

    let active = app.input.is_some();
    let border_color = if active {
        ratatui::style::Color::Cyan
    } else {
        ratatui::style::Color::DarkGray
    };
    let title_style = if active {
        Style::default()
            .fg(ratatui::style::Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(ratatui::style::Color::DarkGray)
    };

    let query_text = app.search_query.clone();
    let cursor = if active { "▏" } else { "" };
    let p = Paragraph::new(Line::from(vec![
        Span::styled(
            " / ",
            Style::default().fg(ratatui::style::Color::Yellow),
        ),
        Span::raw(query_text),
        Span::styled(
            cursor,
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(" Search ", title_style)),
    );
    frame.render_widget(p, chunks[0]);

    // results
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::DarkGray))
        .title(Span::styled(
            format!(" Results ({}) ", app.search_results.len()),
            Style::default().fg(ratatui::style::Color::Cyan),
        ));

    if app.search_results.is_empty() {
        let hint = if app.search_query.is_empty() {
            "type to search across projects, tasks, ideas and notes"
        } else {
            "no matches"
        };
        let p = Paragraph::new(Span::styled(
            format!("  {hint}"),
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let kind = match r.kind {
                SearchKind::Project => "[Project]",
                SearchKind::Task => "[Task   ]",
                SearchKind::Idea => "[Idea   ]",
                SearchKind::IdeaNote => "[Note   ]",
            };
            let kind_color = match r.kind {
                SearchKind::Project => ratatui::style::Color::Magenta,
                SearchKind::Task => ratatui::style::Color::Yellow,
                SearchKind::Idea => ratatui::style::Color::Green,
                SearchKind::IdeaNote => ratatui::style::Color::Blue,
            };
            let prefix = if i == app.search_result_idx {
                "▸ "
            } else {
                "  "
            };
            let prefix_style = if i == app.search_result_idx {
                Style::default()
                    .fg(ratatui::style::Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(
                    format!(" {kind} "),
                    Style::default().fg(kind_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(r.title.clone()),
                Span::raw("  "),
                Span::styled(
                    r.snippet.clone(),
                    Style::default().fg(ratatui::style::Color::Gray),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ratatui::widgets::ListState::default();
    if !app.search_results.is_empty() {
        state.select(Some(
            app.search_result_idx.min(app.search_results.len() - 1),
        ));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(ratatui::style::Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");
    frame.render_stateful_widget(list, chunks[1], &mut state);
}
