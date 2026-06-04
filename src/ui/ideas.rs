use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, IdeasFocus};

pub fn draw(app: &App, area: Rect, frame: &mut Frame) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    draw_idea_list(app, cols[0], frame);
    draw_note_view(app, cols[1], frame);
}

fn panel_block(title: &str, focused: bool) -> Block<'static> {
    let border_color = if focused {
        ratatui::style::Color::Cyan
    } else {
        ratatui::style::Color::DarkGray
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
}

fn draw_idea_list(app: &App, area: Rect, frame: &mut Frame) {
    let focused = app.ideas_focus == IdeasFocus::List;
    let block = panel_block("Ideas", focused);

    if app.ideas.is_empty() {
        let p = Paragraph::new(Span::styled(
            "\n  no ideas yet — press i",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .ideas
        .iter()
        .enumerate()
        .map(|(i, idea)| {
            let shortcut = crate::app::shortcut_for_index(i)
                .map(|s| format!("[{s}]"))
                .unwrap_or_else(|| "   ".to_string());
            let count = format!(" ({} notes)", idea.notes.len());
            let line = Line::from(vec![
                Span::styled(shortcut, Style::default().fg(ratatui::style::Color::Yellow)),
                Span::raw(" "),
                Span::raw(idea.title.clone()),
                Span::styled(count, Style::default().fg(ratatui::style::Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.idea_idx.min(app.ideas.len() - 1)));

    let base_style = if focused {
        Style::default()
    } else {
        Style::default().fg(ratatui::style::Color::DarkGray)
    };
    let highlight_style = if focused {
        Style::default()
            .bg(ratatui::style::Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(ratatui::style::Color::Reset)
            .add_modifier(Modifier::BOLD)
    };

    let list = List::new(items)
        .block(block)
        .style(base_style)
        .highlight_style(highlight_style)
        .highlight_symbol("> ");
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_note_view(app: &App, area: Rect, frame: &mut Frame) {
    let focused = app.ideas_focus == IdeasFocus::Notes;
    let block = panel_block("Notes", focused);

    let idea = match app.current_idea() {
        Some(i) => i,
        None => {
            let p = Paragraph::new(Span::styled(
                "\n  no idea selected",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ))
            .block(block);
            frame.render_widget(p, area);
            return;
        }
    };

    if idea.notes.is_empty() {
        let p = Paragraph::new(Span::styled(
            "\n  no notes — press n",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for (i, note) in idea.notes.iter().enumerate() {
        let marker = if i == app.idea_note_idx { "▸ " } else { "  " };
        let marker_style = if !focused {
            Style::default().fg(ratatui::style::Color::DarkGray)
        } else if i == app.idea_note_idx {
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        let bullet = Span::styled(format!("{marker}• "), marker_style);
        let text_style = if !focused {
            Style::default().fg(ratatui::style::Color::DarkGray)
        } else {
            Style::default()
        };
        let text = Span::styled(note.clone(), text_style);
        lines.push(Line::from(vec![bullet, text]));
    }

    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}
