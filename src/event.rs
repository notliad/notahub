use std::time::Duration;

use anyhow::Result;
use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};

use crate::app::{
    index_for_two_letter_shortcut, is_shortcut_letter, open_shortcut_by_index,
    shortcut_has_two_letter_forms, single_shortcut_index, App, ConfirmAction, ConfirmState,
    DeleteMode, DeleteTarget, HomeFocus, IdeasFocus, InputAction, InputState, ProjectsFocus, Screen,
};
use crate::model::{Idea, Project, Task};
use crate::storage;

pub fn next_poll() -> Result<Option<Event>> {
    if event::poll(Duration::from_millis(200))? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}

pub fn handle_event(app: &mut App, ev: Event) -> Result<()> {
    match ev {
        Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(app, key),
        Event::Mouse(mouse) => handle_mouse(app, mouse),
        _ => Ok(()),
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> Result<()> {
    if app.should_quit {
        return Ok(());
    }

    // Handle pending two-letter shortcut
    if let Some(first) = app.pending_shortcut.take() {
        match key.code {
            KeyCode::Char(second) if second.is_ascii_lowercase() => {
                if let Some(idx) = index_for_two_letter_shortcut(first, second) {
                    if app.delete_mode.is_some() {
                        if lookup_shortcut_by_index(app, idx).is_some() {
                            app.delete_mode = None;
                            if let Some((label, action)) =
                                lookup_shortcut_by_index(app, idx)
                            {
                                app.confirm = Some(ConfirmState {
                                    message: format!("Delete {label}? (y/n)"),
                                    action,
                                });
                            }
                            return Ok(());
                        }
                    } else {
                        let limit = match app.screen {
                            Screen::Home => app.projects.len() + app.ideas.len(),
                            Screen::Projects => app.projects.len(),
                            Screen::Ideas => app.ideas.len(),
                            _ => 0,
                        };
                        if idx < limit {
                            activate_shortcut_by_index(app, idx);
                            return Ok(());
                        }
                    }
                }
                // Two-letter combo didn't match. Try first char as single-letter.
                if app.delete_mode.is_some() {
                    if let Some((label, action)) = lookup_shortcut_by_char(app, first)
                    {
                        app.delete_mode = None;
                        app.confirm = Some(ConfirmState {
                            message: format!("Delete {label}? (y/n)"),
                            action,
                        });
                        return Ok(());
                    }
                } else {
                    activate_shortcut_by_char(app, first);
                }
                // Fall through to process second key normally
            }
            _ => {
                // Non-letter second key — just cancel pending
            }
        }
    }

    if app.confirm.is_some() {
        return handle_confirm_key(app, key);
    }
    if app.input.is_some() {
        return handle_input_key(app, key);
    }
    if app.delete_mode.is_some() {
        return handle_delete_mode_key(app, key);
    }

    match app.screen {
        Screen::Home => handle_home_key(app, key),
        Screen::Projects => handle_projects_key(app, key),
        Screen::Ideas => handle_ideas_key(app, key),
        Screen::Search => handle_search_key(app, key),
        Screen::Help => handle_help_key(app, key),
    }
}

// -------------------- modal handlers --------------------

fn handle_confirm_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(state) = app.confirm.take() {
                execute_confirm(app, state)?;
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.confirm = None;
            app.set_status("Cancelled");
        }
        _ => {}
    }
    Ok(())
}

fn handle_input_key(app: &mut App, key: KeyEvent) -> Result<()> {
    let in_search = app
        .input
        .as_ref()
        .map(|s| matches!(s.action, InputAction::Search))
        .unwrap_or(false);

    // j/k type normally in the input. Use Up/Down to navigate search results.
    if in_search {
        match key.code {
            KeyCode::Down => {
                if !app.search_results.is_empty() {
                    let n = app.search_results.len();
                    app.search_result_idx = (app.search_result_idx + 1) % n;
                }
                return Ok(());
            }
            KeyCode::Up => {
                if !app.search_results.is_empty() {
                    let n = app.search_results.len();
                    if app.search_result_idx == 0 {
                        app.search_result_idx = n - 1;
                    } else {
                        app.search_result_idx -= 1;
                    }
                }
                return Ok(());
            }
            _ => {}
        }
    }

    let mut close = false;
    let mut submit = false;
    if let Some(state) = app.input.as_mut() {
        match key.code {
            KeyCode::Esc => close = true,
            KeyCode::Enter => submit = true,
            KeyCode::Backspace => {
                state.value.pop();
            }
            KeyCode::Char(c) => {
                if !key.modifiers.contains(KeyModifiers::CONTROL) {
                    state.value.push(c);
                }
            }
            _ => {}
        }
    }
    if close {
        app.input = None;
        if in_search {
            app.go(app.prev_screen);
        } else {
            app.set_status("Cancelled");
        }
        return Ok(());
    }
    if submit {
        if let Some(state) = app.input.take() {
            if matches!(state.action, InputAction::Search) {
                app.search_query = state.value.trim().to_string();
                app.refresh_search();
                if app.open_search_result() {
                    app.set_status("Opened");
                } else if !app.search_results.is_empty() {
                    app.set_status("No match to open");
                } else {
                    app.input = None;
                    app.go(app.prev_screen);
                }
            } else {
                execute_input(app, state)?;
            }
        }
        return Ok(());
    }
    if in_search {
        if let Some(state) = app.input.as_ref() {
            if matches!(state.action, InputAction::Search) {
                app.search_query = state.value.clone();
                app.refresh_search();
            }
        }
    }
    Ok(())
}

fn handle_delete_mode_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.delete_mode = None;
            app.set_status("Delete cancelled");
        }
        KeyCode::Char(c) if c.is_ascii_lowercase() && is_shortcut_letter(c) => {
            let total = app.projects.len() + app.ideas.len();
            if total > 17 && shortcut_has_two_letter_forms(c, total) {
                app.pending_shortcut = Some(c);
            } else if let Some((label, action)) = lookup_shortcut_by_char(app, c) {
                app.confirm = Some(ConfirmState {
                    message: format!("Delete {label}? (y/n)"),
                    action,
                });
                app.delete_mode = None;
            } else {
                app.set_status(format!("No item with shortcut '{c}'"));
                app.delete_mode = None;
            }
        }
        _ => {}
    }
    Ok(())
}

fn lookup_shortcut_by_index(app: &App, idx: usize) -> Option<(String, ConfirmAction)> {
    let total = app.projects.len() + app.ideas.len();
    if idx >= total {
        return None;
    }
    if idx < app.projects.len() {
        let p = &app.projects[idx];
        Some((
            format!("project '{}'", p.title),
            ConfirmAction::DeleteProject(p.id),
        ))
    } else {
        let i = &app.ideas[idx - app.projects.len()];
        Some((
            format!("idea '{}'", i.title),
            ConfirmAction::DeleteIdea(i.id),
        ))
    }
}

fn lookup_shortcut_by_char(app: &App, c: char) -> Option<(String, ConfirmAction)> {
    let idx = single_shortcut_index(c)?;
    lookup_shortcut_by_index(app, idx)
}

fn activate_shortcut_by_index(app: &mut App, idx: usize) {
    match app.screen {
        Screen::Home => open_shortcut_by_index(app, idx),
        Screen::Projects => {
            if idx < app.projects.len() {
                app.project_idx = idx;
                app.task_idx = 0;
                app.task_note_idx = 0;
            }
        }
        Screen::Ideas if app.ideas_focus == IdeasFocus::List => {
            if idx < app.ideas.len() {
                app.idea_idx = idx;
                app.idea_note_idx = 0;
            }
        }
        _ => {}
    }
}

fn activate_shortcut_by_char(app: &mut App, c: char) {
    if let Some(idx) = single_shortcut_index(c) {
        match app.screen {
            Screen::Home => open_shortcut_by_index(app, idx),
            Screen::Projects => {
                if idx < app.projects.len() {
                    app.project_idx = idx;
                    app.task_idx = 0;
                    app.task_note_idx = 0;
                }
            }
            Screen::Ideas if app.ideas_focus == IdeasFocus::List => {
                if idx < app.ideas.len() {
                    app.idea_idx = idx;
                    app.idea_note_idx = 0;
                }
            }
            _ => {}
        }
    }
}

// -------------------- screen handlers --------------------

fn handle_home_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Char('?') => app.go(Screen::Help),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('1') => app.go(Screen::Home),
        KeyCode::Char('2') => {
            app.go(Screen::Projects);
            app.projects_focus = ProjectsFocus::List;
        }
        KeyCode::Char('3') => {
            app.go(Screen::Ideas);
            app.ideas_focus = IdeasFocus::List;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.home_focus = toggle_home_focus(app.home_focus);
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.input = Some(InputState {
                label: "New project title".to_string(),
                value: String::new(),
                action: InputAction::NewProject,
            });
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.input = Some(InputState {
                label: "New idea title".to_string(),
                value: String::new(),
                action: InputAction::NewIdea,
            });
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            let total = app.projects.len() + app.ideas.len();
            if total == 0 {
                app.set_status("Nothing to delete");
            } else {
                app.delete_mode = Some(DeleteMode {
                    target: DeleteTarget::Project,
                });
                app.set_status("Delete: press shortcut letter (esc to cancel)");
            }
        }
        KeyCode::Enter => {
            open_current_home(app);
        }
        KeyCode::Down | KeyCode::Char('j') => move_home(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_home(app, -1),
        KeyCode::Left | KeyCode::Char('h') => {
            app.home_focus = HomeFocus::Projects;
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.home_focus = HomeFocus::Ideas;
        }
        KeyCode::Char(c) if c.is_ascii_lowercase() && is_shortcut_letter(c) => {
            let total = app.projects.len() + app.ideas.len();
            if total > 17 && shortcut_has_two_letter_forms(c, total) {
                app.pending_shortcut = Some(c);
            } else {
                activate_shortcut_by_char(app, c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn toggle_home_focus(f: HomeFocus) -> HomeFocus {
    match f {
        HomeFocus::Projects => HomeFocus::Ideas,
        HomeFocus::Ideas => HomeFocus::Projects,
    }
}

fn open_current_home(app: &mut App) {
    match app.home_focus {
        HomeFocus::Projects => {
            if app.project_idx < app.projects.len() {
                app.project_idx = app.home_project_idx.min(app.projects.len() - 1);
                app.task_idx = 0;
                app.task_note_idx = 0;
                app.projects_focus = ProjectsFocus::List;
                app.go(Screen::Projects);
            }
        }
        HomeFocus::Ideas => {
            if !app.ideas.is_empty() {
                app.idea_idx = app.home_idea_idx.min(app.ideas.len() - 1);
                app.idea_note_idx = 0;
                app.ideas_focus = IdeasFocus::List;
                app.go(Screen::Ideas);
            }
        }
    }
}

fn move_home(app: &mut App, delta: i32) {
    match app.home_focus {
        HomeFocus::Projects => {
            if !app.projects.is_empty() {
                let n = app.projects.len() as i32;
                let mut i = app.home_project_idx as i32 + delta;
                if i < 0 {
                    i = 0;
                }
                if i >= n {
                    i = n - 1;
                }
                app.home_project_idx = i as usize;
            }
        }
        HomeFocus::Ideas => {
            if !app.ideas.is_empty() {
                let n = app.ideas.len() as i32;
                let mut i = app.home_idea_idx as i32 + delta;
                if i < 0 {
                    i = 0;
                }
                if i >= n {
                    i = n - 1;
                }
                app.home_idea_idx = i as usize;
            }
        }
    }
}

fn handle_projects_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Esc => app.go_home(),
        KeyCode::Char('?') => app.go(Screen::Help),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('1') => app.go_home(),
        KeyCode::Char('2') => app.go(Screen::Projects),
        KeyCode::Char('3') => {
            app.go(Screen::Ideas);
            app.ideas_focus = IdeasFocus::List;
        }
        KeyCode::Tab | KeyCode::BackTab => {
            app.projects_focus = match app.projects_focus {
                ProjectsFocus::List => ProjectsFocus::Tasks,
                ProjectsFocus::Tasks => ProjectsFocus::Notes,
                ProjectsFocus::Notes => ProjectsFocus::List,
            };
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.input = Some(InputState {
                label: "New project title".to_string(),
                value: String::new(),
                action: InputAction::NewProject,
            });
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            if let Some(p) = app.current_project() {
                let id = p.id;
                app.input = Some(InputState {
                    label: "New task title".to_string(),
                    value: String::new(),
                    action: InputAction::NewTask(id),
                });
            } else {
                app.set_status("Create or select a project first");
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if let (Some(p), Some(t)) = (app.current_project(), app.current_task()) {
                let pid = p.id;
                let tid = t.id;
                app.input = Some(InputState {
                    label: "New note".to_string(),
                    value: String::new(),
                    action: InputAction::NewTaskNote(pid, tid),
                });
            } else if let Some(p) = app.current_project() {
                if p.tasks.is_empty() {
                    app.set_status("Add a task first");
                } else {
                    app.set_status("Select a task in the middle column");
                }
            } else {
                app.set_status("Select a project first");
            }
        }
        KeyCode::Char(' ') => {
            if matches!(app.projects_focus, ProjectsFocus::Tasks) {
                toggle_task_completed(app)?;
            }
        }
        KeyCode::Char('x') | KeyCode::Char('X') => confirm_delete_in_projects(app),
        KeyCode::Down | KeyCode::Char('j') => move_in_projects(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_in_projects(app, -1),
        KeyCode::Left | KeyCode::Char('h') => {
            app.projects_focus = match app.projects_focus {
                ProjectsFocus::List => ProjectsFocus::List,
                ProjectsFocus::Tasks => ProjectsFocus::List,
                ProjectsFocus::Notes => ProjectsFocus::Tasks,
            };
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.projects_focus = match app.projects_focus {
                ProjectsFocus::List => ProjectsFocus::Tasks,
                ProjectsFocus::Tasks => ProjectsFocus::Notes,
                ProjectsFocus::Notes => ProjectsFocus::Notes,
            };
        }
        KeyCode::Char(c) if c.is_ascii_lowercase() && is_shortcut_letter(c) => {
            let n = app.projects.len();
            if n > 17 && shortcut_has_two_letter_forms(c, n) {
                app.pending_shortcut = Some(c);
            } else {
                activate_shortcut_by_char(app, c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_ideas_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Esc => app.go_home(),
        KeyCode::Char('?') => app.go(Screen::Help),
        KeyCode::Char('/') => app.open_search(),
        KeyCode::Char('1') => app.go_home(),
        KeyCode::Char('2') => {
            app.go(Screen::Projects);
            app.projects_focus = ProjectsFocus::List;
        }
        KeyCode::Char('3') => app.go(Screen::Ideas),
        KeyCode::Tab | KeyCode::BackTab => {
            app.ideas_focus = match app.ideas_focus {
                IdeasFocus::List => IdeasFocus::Notes,
                IdeasFocus::Notes => IdeasFocus::List,
            };
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            app.input = Some(InputState {
                label: "New idea title".to_string(),
                value: String::new(),
                action: InputAction::NewIdea,
            });
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            if let Some(idea) = app.current_idea() {
                let id = idea.id;
                app.input = Some(InputState {
                    label: "New note".to_string(),
                    value: String::new(),
                    action: InputAction::NewIdeaNote(id),
                });
            } else {
                app.set_status("Select or create an idea first");
            }
        }
        KeyCode::Char('x') | KeyCode::Char('X') => confirm_delete_in_ideas(app),
        KeyCode::Down | KeyCode::Char('j') => move_in_ideas(app, 1),
        KeyCode::Up | KeyCode::Char('k') => move_in_ideas(app, -1),
        KeyCode::Left | KeyCode::Char('h') => app.ideas_focus = IdeasFocus::List,
        KeyCode::Right | KeyCode::Char('l') => app.ideas_focus = IdeasFocus::Notes,
        KeyCode::Char(c) if c.is_ascii_lowercase() && is_shortcut_letter(c) && app.ideas_focus == IdeasFocus::List => {
            let n = app.ideas.len();
            if n > 17 && shortcut_has_two_letter_forms(c, n) {
                app.pending_shortcut = Some(c);
            } else {
                activate_shortcut_by_char(app, c);
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_search_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.input = None;
            app.go(app.prev_screen);
        }
        KeyCode::Enter => {
            if app.search_results.is_empty() {
                app.input = None;
                app.go(app.prev_screen);
            } else if app.open_search_result() {
                app.set_status("Opened");
            } else {
                app.set_status("Item not found");
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.search_results.is_empty() {
                let n = app.search_results.len();
                app.search_result_idx = (app.search_result_idx + 1) % n;
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if !app.search_results.is_empty() {
                let n = app.search_results.len();
                if app.search_result_idx == 0 {
                    app.search_result_idx = n - 1;
                } else {
                    app.search_result_idx -= 1;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_help_key(app: &mut App, key: KeyEvent) -> Result<()> {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') | KeyCode::Char('Q') => {
            app.go(app.prev_screen);
        }
        _ => {}
    }
    Ok(())
}

fn handle_mouse(app: &mut App, mouse: MouseEvent) -> Result<()> {
    if app.confirm.is_some() || app.input.is_some() || app.delete_mode.is_some() {
        return Ok(());
    }
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {}
        MouseEventKind::ScrollUp => match app.screen {
            Screen::Home => move_home(app, -1),
            Screen::Projects => move_in_projects(app, -1),
            Screen::Ideas => move_in_ideas(app, -1),
            _ => {}
        },
        MouseEventKind::ScrollDown => match app.screen {
            Screen::Home => move_home(app, 1),
            Screen::Projects => move_in_projects(app, 1),
            Screen::Ideas => move_in_ideas(app, 1),
            _ => {}
        },
        _ => {}
    }
    Ok(())
}

// -------------------- move / delete helpers --------------------

fn move_in_projects(app: &mut App, delta: i32) {
    match app.projects_focus {
        ProjectsFocus::List => {
            if !app.projects.is_empty() {
                let n = app.projects.len() as i32;
                let mut i = app.project_idx as i32 + delta;
                if i < 0 {
                    i = 0;
                }
                if i >= n {
                    i = n - 1;
                }
                app.project_idx = i as usize;
                app.task_idx = 0;
                app.task_note_idx = 0;
            }
        }
        ProjectsFocus::Tasks => {
            if let Some(p) = app.current_project() {
                if !p.tasks.is_empty() {
                    let n = p.tasks.len() as i32;
                    let mut i = app.task_idx as i32 + delta;
                    if i < 0 {
                        i = 0;
                    }
                    if i >= n {
                        i = n - 1;
                    }
                    app.task_idx = i as usize;
                    app.task_note_idx = 0;
                }
            }
        }
        ProjectsFocus::Notes => {
            if let Some(t) = app.current_task() {
                if !t.notes.is_empty() {
                    let n = t.notes.len() as i32;
                    let mut i = app.task_note_idx as i32 + delta;
                    if i < 0 {
                        i = 0;
                    }
                    if i >= n {
                        i = n - 1;
                    }
                    app.task_note_idx = i as usize;
                }
            }
        }
    }
}

fn move_in_ideas(app: &mut App, delta: i32) {
    match app.ideas_focus {
        IdeasFocus::List => {
            if !app.ideas.is_empty() {
                let n = app.ideas.len() as i32;
                let mut i = app.idea_idx as i32 + delta;
                if i < 0 {
                    i = 0;
                }
                if i >= n {
                    i = n - 1;
                }
                app.idea_idx = i as usize;
                app.idea_note_idx = 0;
            }
        }
        IdeasFocus::Notes => {
            if let Some(idea) = app.current_idea() {
                if !idea.notes.is_empty() {
                    let n = idea.notes.len() as i32;
                    let mut i = app.idea_note_idx as i32 + delta;
                    if i < 0 {
                        i = 0;
                    }
                    if i >= n {
                        i = n - 1;
                    }
                    app.idea_note_idx = i as usize;
                }
            }
        }
    }
}

fn confirm_delete_in_projects(app: &mut App) {
    match app.projects_focus {
        ProjectsFocus::List => {
            if let Some(p) = app.current_project() {
                let id = p.id;
                let title = p.title.clone();
                app.confirm = Some(ConfirmState {
                    message: format!("Delete project '{title}'? (y/n)"),
                    action: ConfirmAction::DeleteProject(id),
                });
            } else {
                app.set_status("Nothing to delete");
            }
        }
        ProjectsFocus::Tasks => {
            if let (Some(p), Some(t)) = (app.current_project(), app.current_task()) {
                let pid = p.id;
                let tid = t.id;
                let title = t.title.clone();
                app.confirm = Some(ConfirmState {
                    message: format!("Delete task '{title}'? (y/n)"),
                    action: ConfirmAction::DeleteTask(pid, tid),
                });
            } else {
                app.set_status("Nothing to delete");
            }
        }
        ProjectsFocus::Notes => {
            if let (Some(p), Some(t)) = (app.current_project(), app.current_task()) {
                let pid = p.id;
                let tid = t.id;
                if app.task_note_idx < t.notes.len() {
                    let idx = app.task_note_idx;
                    app.confirm = Some(ConfirmState {
                        message: format!("Delete note {}? (y/n)", idx + 1),
                        action: ConfirmAction::DeleteProjectNote(pid, tid, idx),
                    });
                } else {
                    app.set_status("No note selected");
                }
            } else {
                app.set_status("Nothing to delete");
            }
        }
    }
}

fn confirm_delete_in_ideas(app: &mut App) {
    match app.ideas_focus {
        IdeasFocus::List => {
            if let Some(idea) = app.current_idea() {
                let id = idea.id;
                let title = idea.title.clone();
                app.confirm = Some(ConfirmState {
                    message: format!("Delete idea '{title}'? (y/n)"),
                    action: ConfirmAction::DeleteIdea(id),
                });
            } else {
                app.set_status("Nothing to delete");
            }
        }
        IdeasFocus::Notes => {
            if let Some(idea) = app.current_idea() {
                if app.idea_note_idx < idea.notes.len() {
                    let id = idea.id;
                    let idx = app.idea_note_idx;
                    app.confirm = Some(ConfirmState {
                        message: format!("Delete note {}? (y/n)", idx + 1),
                        action: ConfirmAction::DeleteIdeaNote(id, idx),
                    });
                } else {
                    app.set_status("No note selected");
                }
            }
        }
    }
}

fn toggle_task_completed(app: &mut App) -> Result<()> {
    let (pid, tid) = match (app.current_project(), app.current_task()) {
        (Some(p), Some(t)) => (p.id, t.id),
        _ => return Ok(()),
    };
    if let Some(p_idx) = app.project_by_id(pid) {
        if let Some(t_idx) = app.task_by_id(p_idx, tid) {
            app.projects[p_idx].tasks[t_idx].completed =
                !app.projects[p_idx].tasks[t_idx].completed;
            storage::save_project(&app.data_root, &app.projects[p_idx])?;
        }
    }
    Ok(())
}

// -------------------- actions --------------------

fn execute_input(app: &mut App, state: InputState) -> Result<()> {
    let value = state.value.trim();
    if value.is_empty() {
        app.set_status("Empty input discarded");
        return Ok(());
    }
    match state.action {
        InputAction::NewProject => {
            let project = Project::new(value.to_string());
            storage::save_project(&app.data_root, &project)?;
            app.projects.push(project);
            sort_all(app);
            app.clamp_indices();
            app.set_status(format!("Created project '{}'", value));
        }
        InputAction::NewTask(project_id) => {
            if let Some(p_idx) = app.project_by_id(project_id) {
                let task = Task::new(value.to_string());
                app.projects[p_idx].tasks.push(task);
                storage::save_project(&app.data_root, &app.projects[p_idx])?;
                app.set_status(format!("Added task '{}'", value));
            }
        }
        InputAction::NewTaskNote(project_id, task_id) => {
            if let Some(p_idx) = app.project_by_id(project_id) {
                if let Some(t_idx) = app.task_by_id(p_idx, task_id) {
                    app.projects[p_idx].tasks[t_idx].notes.push(value.to_string());
                    storage::save_project(&app.data_root, &app.projects[p_idx])?;
                    app.set_status("Added note");
                }
            }
        }
        InputAction::NewIdea => {
            let idea = Idea::new(value.to_string());
            storage::save_idea(&app.data_root, &idea)?;
            app.ideas.push(idea);
            sort_all(app);
            app.clamp_indices();
            app.set_status(format!("Created idea '{}'", value));
        }
        InputAction::NewIdeaNote(idea_id) => {
            if let Some(i_idx) = app.idea_by_id(idea_id) {
                app.ideas[i_idx].notes.push(value.to_string());
                storage::save_idea(&app.data_root, &app.ideas[i_idx])?;
                app.set_status("Added note");
            }
        }
        InputAction::Search => {
            app.search_query = value.to_string();
            app.refresh_search();
        }
    }
    Ok(())
}

fn sort_all(app: &mut App) {
    app.projects
        .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    app.ideas
        .sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
}

fn execute_confirm(app: &mut App, state: ConfirmState) -> Result<()> {
    match state.action {
        ConfirmAction::DeleteProject(id) => {
            if let Some(pos) = app.project_by_id(id) {
                let project = app.projects.remove(pos);
                storage::delete_project(&app.data_root, &project)?;
                app.clamp_indices();
                app.set_status(format!("Deleted project '{}'", project.title));
            }
        }
        ConfirmAction::DeleteIdea(id) => {
            if let Some(pos) = app.idea_by_id(id) {
                let idea = app.ideas.remove(pos);
                storage::delete_idea(&app.data_root, &idea)?;
                app.clamp_indices();
                app.set_status(format!("Deleted idea '{}'", idea.title));
            }
        }
        ConfirmAction::DeleteTask(project_id, task_id) => {
            if let Some(p_idx) = app.project_by_id(project_id) {
                if let Some(t_idx) = app.task_by_id(p_idx, task_id) {
                    let task = app.projects[p_idx].tasks.remove(t_idx);
                    storage::save_project(&app.data_root, &app.projects[p_idx])?;
                    app.clamp_indices();
                    app.set_status(format!("Deleted task '{}'", task.title));
                }
            }
        }
        ConfirmAction::DeleteProjectNote(project_id, task_id, note_idx) => {
            if let Some(p_idx) = app.project_by_id(project_id) {
                if let Some(t_idx) = app.task_by_id(p_idx, task_id) {
                    if note_idx < app.projects[p_idx].tasks[t_idx].notes.len() {
                        app.projects[p_idx].tasks[t_idx].notes.remove(note_idx);
                        storage::save_project(&app.data_root, &app.projects[p_idx])?;
                        app.clamp_indices();
                        app.set_status("Deleted note");
                    }
                }
            }
        }
        ConfirmAction::DeleteIdeaNote(idea_id, note_idx) => {
            if let Some(i_idx) = app.idea_by_id(idea_id) {
                if note_idx < app.ideas[i_idx].notes.len() {
                    app.ideas[i_idx].notes.remove(note_idx);
                    storage::save_idea(&app.data_root, &app.ideas[i_idx])?;
                    app.clamp_indices();
                    app.set_status("Deleted note");
                }
            }
        }
    }
    Ok(())
}
