use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::{App, ProjectsFocus};

pub fn draw(app: &App, area: Rect, frame: &mut Frame) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(40),
            Constraint::Percentage(35),
        ])
        .split(area);

    draw_project_list(app, cols[0], frame);
    draw_task_list(app, cols[1], frame);
    draw_note_view(app, cols[2], frame);
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

fn draw_project_list(app: &App, area: Rect, frame: &mut Frame) {
    let focused = app.projects_focus == ProjectsFocus::List;
    let block = panel_block("Projects", focused);

    if app.projects.is_empty() {
        let p = Paragraph::new(Span::styled(
            "\n  no projects yet — press p",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let shortcut = crate::app::shortcut_for_index(i)
                .map(|s| format!("[{s}]"))
                .unwrap_or_else(|| "   ".to_string());
            let count = format!(" ({} tasks)", p.tasks.len());
            let line = Line::from(vec![
                Span::styled(shortcut, Style::default().fg(ratatui::style::Color::Yellow)),
                Span::raw(" "),
                Span::raw(p.title.clone()),
                Span::styled(count, Style::default().fg(ratatui::style::Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.project_idx.min(app.projects.len() - 1)));

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

fn draw_task_list(app: &App, area: Rect, frame: &mut Frame) {
    let focused = app.projects_focus == ProjectsFocus::Tasks;
    let block = panel_block("Tasks", focused);

    let project = match app.current_project() {
        Some(p) => p,
        None => {
            let p = Paragraph::new(Span::styled(
                "\n  no project selected",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ))
            .block(block);
            frame.render_widget(p, area);
            return;
        }
    };

    if project.tasks.is_empty() {
        let p = Paragraph::new(Span::styled(
            "\n  no tasks — press t",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = project
        .tasks
        .iter()
        .map(|t| {
            let check = if t.completed { "[x]" } else { "[ ]" };
            let count = format!("  ({} notes)", t.notes.len());
            let title_style = if t.completed {
                Style::default()
                    .fg(ratatui::style::Color::Gray)
                    .add_modifier(Modifier::DIM | Modifier::CROSSED_OUT)
            } else {
                Style::default()
            };
            let line = Line::from(vec![
                Span::styled(
                    check,
                    Style::default().fg(ratatui::style::Color::Yellow),
                ),
                Span::raw(" "),
                Span::styled(t.title.clone(), title_style),
                Span::styled(count, Style::default().fg(ratatui::style::Color::DarkGray)),
            ]);
            ListItem::new(line)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(app.task_idx.min(project.tasks.len() - 1)));

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
    let focused = app.projects_focus == ProjectsFocus::Notes;
    let block = panel_block("Notes", focused);

    let task = match app.current_task() {
        Some(t) => t,
        None => {
            let p = Paragraph::new(Span::styled(
                "\n  no task selected",
                Style::default().fg(ratatui::style::Color::DarkGray),
            ))
            .block(block);
            frame.render_widget(p, area);
            return;
        }
    };

    if task.notes.is_empty() {
        let p = Paragraph::new(Span::styled(
            "\n  no notes — press n",
            Style::default().fg(ratatui::style::Color::DarkGray),
        ))
        .block(block);
        frame.render_widget(p, area);
        return;
    }

    let mut lines: Vec<Line> = Vec::new();
    for (i, note) in task.notes.iter().enumerate() {
        let marker = if i == app.task_note_idx { "▸ " } else { "  " };
        let marker_style = if !focused {
            Style::default().fg(ratatui::style::Color::DarkGray)
        } else if i == app.task_note_idx {
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
