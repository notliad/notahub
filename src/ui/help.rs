use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(frame: &mut Frame) {
    let area = centered_rect(70, 80, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(ratatui::style::Color::Cyan))
        .title(Span::styled(
            " Help — press ? / esc / enter to close ",
            Style::default()
                .fg(ratatui::style::Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let left = vec![
        section("Navigation"),
        kv("1 2 3", "switch screen (home / projects / ideas)"),
        kv("/", "open search"),
        kv("tab", "cycle columns / list focus"),
        kv("h l  ← →", "switch column (projects / ideas)"),
        kv("j k  ↓ ↑", "move selection / search results"),
        kv("enter", "open selected (home / search)"),
        kv("esc", "back to home / close modal"),
        kv("Q", "quit"),
        blank(),
        section("Shortcuts"),
        kv("a–z *", "open item by shortcut letter"),
        kv("", "(h/j/k/l reserved for navigation)"),
        kv("aa zz …", "two-letter shortcuts >22 items"),
        blank(),
        section("Search"),
        kv("/", "open search (live, per keystroke)"),
        kv("j k  ↓ ↑", "navigate results"),
        kv("enter", "open selected result"),
        kv("esc", "close search"),
    ];

    let right = vec![
        section("Create"),
        kv("p", "new project (home / projects)"),
        kv("i", "new idea (home / ideas)"),
        kv("t", "new task (projects screen)"),
        kv("n", "new note (task or idea)"),
        blank(),
        section("Delete"),
        kv("x", "delete: home → press shortcut; else selected item"),
        kv("y / n", "confirm / cancel deletion"),
        blank(),
        section("Projects extras"),
        kv("space", "toggle task completed"),
        blank(),
        section("Other"),
        kv("?", "toggle this help"),
        blank(),
        footer(),
    ];

    let lp = Paragraph::new(left);
    let rp = Paragraph::new(right);
    frame.render_widget(lp, cols[0]);
    frame.render_widget(rp, cols[1]);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
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
        .split(popup_v[1])[1]
}

fn section(s: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {s}"),
        Style::default()
            .fg(ratatui::style::Color::Cyan)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
    ))
}

fn kv(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(
            format!("{key:<14}"),
            Style::default().fg(ratatui::style::Color::Yellow),
        ),
        Span::raw(desc.to_string()),
    ])
}

fn blank() -> Line<'static> {
    Line::from("")
}

fn footer() -> Line<'static> {
    Line::from(vec![
        Span::raw("   "),
        Span::styled(
            format!("notahub v{}", env!("CARGO_PKG_VERSION")),
            Style::default().fg(ratatui::style::Color::DarkGray),
        ),
    ])
}
