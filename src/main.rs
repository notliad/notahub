use std::io::{stdout, Stdout};

use anyhow::Result;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use notahub::app::App;
use notahub::event::{handle_event, next_poll};
use notahub::storage;
use notahub::ui;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

type Term = Terminal<CrosstermBackend<Stdout>>;

fn main() -> Result<()> {
    let data_root = storage::data_root();
    let mut terminal = setup_terminal()?;
    let result = run(&mut terminal, data_root);
    restore_terminal(&mut terminal)?;
    result
}

fn setup_terminal() -> Result<Term> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(term: &mut Term) -> Result<()> {
    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    Ok(())
}

fn run(terminal: &mut Term, data_root: std::path::PathBuf) -> Result<()> {
    let mut app = App::new(data_root)?;
    while !app.should_quit {
        terminal.draw(|frame| ui::draw(&app, frame))?;
        if let Some(ev) = next_poll()? {
            handle_event(&mut app, ev)?;
        }
    }
    Ok(())
}
