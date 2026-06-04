use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::{App, HomeFocus};
use crate::app::shortcut_for_index;

pub fn draw(app: &App, area: Rect, frame: &mut Frame) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_list(
        app,
        cols[0],
        frame,
        "Projects",
        app.projects.len(),
        app.home_project_idx,
        app.home_focus == HomeFocus::Projects,
        |i, app| {
            let p = &app.projects[i];
            let line = item_line(i, &p.title, app.projects.len());
            (line, p.tasks.len())
        },
    );

    draw_list(
        app,
        cols[1],
        frame,
        "Ideas",
        app.ideas.len(),
        app.home_idea_idx,
        app.home_focus == HomeFocus::Ideas,
        |i, app| {
            let idea = &app.ideas[i];
            let offset = app.projects.len();
            let line = item_line(offset + i, &idea.title, app.ideas.len());
            (line, idea.notes.len())
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_list<F>(
    app: &App,
    area: Rect,
    frame: &mut Frame,
    title: &str,
    len: usize,
    selected: usize,
    focused: bool,
    mut row: F,
) where
    F: FnMut(usize, &App) -> (Line<'static>, usize),
{
    let border_color = if focused {
        ratatui::style::Color::Cyan
    } else {
        ratatui::style::Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    if len == 0 {
        let empty = ratatui::widgets::Paragraph::new(Span::styled(
            "  (empty)",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(empty, area);
        return;
    }

    let items: Vec<ListItem> = (0..len)
        .map(|i| {
            let (line, _meta) = row(i, app);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    if selected < len {
        state.select(Some(selected));
    }

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

fn item_line(global_idx: usize, title: &str, _total: usize) -> Line<'static> {
    let key = shortcut_for_index(global_idx)
        .map(|s| format!("[{s}]"))
        .unwrap_or_else(|| "   ".to_string());
    Line::from(vec![
        Span::styled(key, Style::default().fg(ratatui::style::Color::Yellow)),
        Span::raw(" "),
        Span::raw(title.to_string()),
    ])
}
