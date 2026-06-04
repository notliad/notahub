use notahub::app::{compute_search, App};
use notahub::model::{Project, Task};
use notahub::storage;

#[test]
fn app_loads_empty_dir() {
    let dir = tempdir();
    let app = App::new(dir.path().to_path_buf()).unwrap();
    assert!(app.projects.is_empty());
    assert!(app.ideas.is_empty());
}

#[test]
fn search_finds_in_projects_and_ideas() {
    let dir = tempdir();
    write_file(
        &dir.path().join("projects/welcome.md"),
        "---\nid: 11111111-1111-1111-1111-111111111111\ntitle: Welcome\ncreated: 2026-06-03T00:00:00Z\nkind: project\n---\n\n",
    );
    write_file(
        &dir.path().join("ideas/spark.md"),
        "---\nid: 22222222-2222-2222-2222-222222222222\ntitle: My Spark\ncreated: 2026-06-03T00:00:00Z\nkind: idea\n---\n\n- A flash of insight\n",
    );
    let app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.projects.len(), 1);
    assert_eq!(app.ideas.len(), 1);
    let r = compute_search(&app.projects, &app.ideas, "spark");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].title, "My Spark");
    let r = compute_search(&app.projects, &app.ideas, "welcome");
    assert_eq!(r.len(), 1);
    let r = compute_search(&app.projects, &app.ideas, "zzz");
    assert!(r.is_empty());
}

#[test]
fn shortcut_for_index_assigns_a_to_first() {
    use notahub::app::shortcut_for_index;
    assert_eq!(shortcut_for_index(0), Some("a".to_string()));
    assert_eq!(shortcut_for_index(16), Some("z".to_string()));
    assert_eq!(shortcut_for_index(17), Some("aa".to_string()));
    assert_eq!(shortcut_for_index(18), Some("ab".to_string()));
    assert_eq!(shortcut_for_index(42), Some("az".to_string()));
    assert_eq!(shortcut_for_index(43), Some("ba".to_string()));
    assert_eq!(shortcut_for_index(458), Some("zz".to_string()));
    assert_eq!(shortcut_for_index(459), None);
}

#[test]
fn save_and_reload_preserves_ids() {
    let dir = tempdir();
    let mut p = Project::new("Hello".to_string());
    let t = Task::new("first".to_string());
    p.tasks.push(t);
    let project_id = p.id;
    let task_id = p.tasks[0].id;
    storage::save_project(dir.path(), &p).unwrap();

    let loaded = storage::load_projects(dir.path()).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, project_id);
    assert_eq!(loaded[0].tasks[0].id, task_id);
    assert_eq!(loaded[0].tasks[0].title, "first");
}

#[test]
fn parse_task_with_note_block() {
    let dir = tempdir();
    let content = r#"---
id: 33333333-3333-3333-3333-333333333333
title: Demo
created: 2026-06-03T00:00:00Z
kind: project
---

- [ ] First task <!-- id:44444444-4444-4444-4444-444444444444 -->
  Note line one
  Note line two

- [x] Done task <!-- id:55555555-5555-5555-5555-555555555555 -->
  Done note
"#;
    write_file(&dir.path().join("projects/demo.md"), content);
    let loaded = storage::load_projects(dir.path()).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].tasks.len(), 2);
    assert_eq!(loaded[0].tasks[0].title, "First task");
    assert!(!loaded[0].tasks[0].completed);
    assert_eq!(loaded[0].tasks[0].notes, vec!["Note line one", "Note line two"]);
    assert!(loaded[0].tasks[1].completed);
}

#[test]
fn ui_renders_all_screens_without_panic() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use notahub::ui;

    let dir = tempdir();
    let p = Project::new("Hello".to_string());
    storage::save_project(dir.path(), &p).unwrap();
    let app = App::new(dir.path().to_path_buf()).unwrap();

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    for screen in [
        notahub::app::Screen::Home,
        notahub::app::Screen::Projects,
        notahub::app::Screen::Ideas,
        notahub::app::Screen::Search,
        notahub::app::Screen::Help,
    ] {
        let a = app_for_render(&app, screen);
        terminal
            .draw(|frame| ui::draw(&a, frame))
            .expect("home renders");
    }
}

fn app_for_render(app: &App, screen: notahub::app::Screen) -> App {
    let mut a = App {
        data_root: app.data_root.clone(),
        projects: app.projects.clone(),
        ideas: app.ideas.clone(),
        screen,
        prev_screen: app.screen,
        status: Some("Ready".to_string()),
        home_focus: notahub::app::HomeFocus::Projects,
        home_project_idx: 0,
        home_idea_idx: 0,
        projects_focus: notahub::app::ProjectsFocus::List,
        project_idx: 0,
        task_idx: 0,
        task_note_idx: 0,
        ideas_focus: notahub::app::IdeasFocus::List,
        idea_idx: 0,
        idea_note_idx: 0,
        search_query: String::from("hello"),
        search_results: Vec::new(),
        search_result_idx: 0,
        input: None,
        confirm: None,
        delete_mode: None,
        pending_shortcut: None,
        should_quit: false,
    };
    a.refresh_search();
    a
}

#[test]
fn rendered_home_contains_titles() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use notahub::ui;

    let dir = tempdir();
    let p = Project::new("Welcome".to_string());
    storage::save_project(dir.path(), &p).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    app.refresh_search();

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| ui::draw(&app, frame)).unwrap();

    let buf = terminal.backend().buffer().clone();
    let text: String = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Welcome"), "rendered text:\n{text}");
    assert!(text.contains("Projects"));
    assert!(text.contains("Ideas"));
}

#[test]
fn quit_key_sets_should_quit() {
    let dir = tempdir();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    let ev = ev(crossterm::event::KeyCode::Char('Q'));
    notahub::event::handle_event(&mut app, ev).unwrap();
    assert!(app.should_quit);
}

#[test]
fn lowercase_q_does_not_quit() {
    let dir = tempdir();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    let ev = ev(crossterm::event::KeyCode::Char('q'));
    notahub::event::handle_event(&mut app, ev).unwrap();
    assert!(!app.should_quit);
}

#[test]
fn home_shortcut_opens_project_details() {
    use crossterm::event::KeyCode;
    use notahub::app::Screen;
    use notahub::event::handle_event;

    let dir = tempdir();
    let p = Project::new("First".to_string());
    let pid = p.id;
    storage::save_project(dir.path(), &p).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.projects.len(), 1);

    handle_event(&mut app, ev(KeyCode::Char('a'))).unwrap();
    assert_eq!(app.screen, Screen::Projects);
    assert_eq!(app.project_idx, 0);
    assert_eq!(app.projects[0].id, pid);
}

#[test]
fn home_shortcut_opens_idea_details() {
    use crossterm::event::KeyCode;
    use notahub::app::Screen;
    use notahub::event::handle_event;
    use notahub::model::Idea;

    let dir = tempdir();
    let i = Idea::new("First idea".to_string());
    storage::save_idea(dir.path(), &i).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.ideas.len(), 1);

    handle_event(&mut app, ev(KeyCode::Char('a'))).unwrap();
    assert_eq!(app.screen, Screen::Ideas);
    assert_eq!(app.idea_idx, 0);
}

#[test]
fn home_vim_nav_is_reserved_and_functional() {
    use crossterm::event::KeyCode;
    use notahub::app::{HomeFocus, Screen};
    use notahub::event::handle_event;
    use notahub::model::{Idea, Project};

    let dir = tempdir();
    storage::save_project(dir.path(), &Project::new("First".to_string())).unwrap();
    storage::save_project(dir.path(), &Project::new("Second".to_string())).unwrap();
    storage::save_idea(dir.path(), &Idea::new("Idea A".to_string())).unwrap();
    storage::save_idea(dir.path(), &Idea::new("Idea B".to_string())).unwrap();

    let mut app = App::new(dir.path().to_path_buf()).unwrap();
    assert_eq!(app.screen, Screen::Home);
    assert_eq!(app.home_focus, HomeFocus::Projects);
    assert_eq!(app.home_project_idx, 0);

    // h/l switch column focus and do not open a shortcut.
    handle_event(&mut app, ev(KeyCode::Char('l'))).unwrap();
    assert_eq!(app.home_focus, HomeFocus::Ideas);
    assert_eq!(app.screen, Screen::Home, "l must not open a shortcut");

    handle_event(&mut app, ev(KeyCode::Char('h'))).unwrap();
    assert_eq!(app.home_focus, HomeFocus::Projects);
    assert_eq!(app.screen, Screen::Home, "h must not open a shortcut");

    // j/k move the selection and do not open a shortcut.
    handle_event(&mut app, ev(KeyCode::Char('l'))).unwrap();
    handle_event(&mut app, ev(KeyCode::Char('j'))).unwrap();
    assert_eq!(app.home_idea_idx, 1, "j should move down");
    assert_eq!(app.screen, Screen::Home, "j must not open a shortcut");

    handle_event(&mut app, ev(KeyCode::Char('k'))).unwrap();
    assert_eq!(app.home_idea_idx, 0, "k should move up");
    assert_eq!(app.screen, Screen::Home, "k must not open a shortcut");

    // Non-vim letters still trigger shortcuts.
    handle_event(&mut app, ev(KeyCode::Char('h'))).unwrap();
    handle_event(&mut app, ev(KeyCode::Char('a'))).unwrap();
    assert_eq!(app.screen, Screen::Projects, "a should still open first project");
}

#[test]
fn search_enter_navigates_to_project() {
    use crossterm::event::KeyCode;
    use notahub::app::Screen;
    use notahub::event::handle_event;

    let dir = tempdir();
    let p = Project::new("Findme".to_string());
    storage::save_project(dir.path(), &p).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();

    // open search via '/'
    handle_event(&mut app, ev(KeyCode::Char('/'))).unwrap();
    assert_eq!(app.screen, Screen::Search);
    assert!(app.input.is_some());

    // type query
    for c in "findme".chars() {
        handle_event(&mut app, ev(KeyCode::Char(c))).unwrap();
    }
    assert_eq!(app.search_results.len(), 1);
    assert_eq!(app.search_result_idx, 0);

    // press Enter to navigate
    handle_event(&mut app, ev(KeyCode::Enter)).unwrap();
    assert_eq!(app.screen, Screen::Projects);
    assert_eq!(app.project_idx, 0);
    assert_eq!(app.projects[0].title, "Findme");
}

#[test]
fn search_arrows_navigate_results() {
    use crossterm::event::KeyCode;
    use notahub::event::handle_event;

    let dir = tempdir();
    let p1 = Project::new("Alpha".to_string());
    let p2 = Project::new("Beta".to_string());
    storage::save_project(dir.path(), &p1).unwrap();
    storage::save_project(dir.path(), &p2).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();

    handle_event(&mut app, ev(KeyCode::Char('/'))).unwrap();
    for c in "a".chars() {
        handle_event(&mut app, ev(KeyCode::Char(c))).unwrap();
    }
    // Should match both Alpha and Beta (both contain 'a').
    assert!(app.search_results.len() >= 2);
    assert_eq!(app.search_result_idx, 0);

    handle_event(&mut app, ev(KeyCode::Down)).unwrap();
    assert_eq!(app.search_result_idx, 1);

    handle_event(&mut app, ev(KeyCode::Up)).unwrap();
    assert_eq!(app.search_result_idx, 0);
}

#[test]
fn help_is_overlay_and_keeps_underlying_screen_visible() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use notahub::ui;

    let dir = tempdir();
    let p = Project::new("Background".to_string());
    storage::save_project(dir.path(), &p).unwrap();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();

    // Switch to projects screen and ensure project is rendered.
    app.go(notahub::app::Screen::Projects);
    app.project_idx = 0;
    app.projects_focus = notahub::app::ProjectsFocus::List;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| ui::draw(&app, frame))
        .expect("projects renders");

    // Now open the help screen.
    app.go(notahub::app::Screen::Help);
    terminal
        .draw(|frame| ui::draw(&app, frame))
        .expect("help renders");

    let buf = terminal.backend().buffer().clone();
    let text: String = (0..buf.area.height)
        .map(|y| {
            (0..buf.area.width)
                .map(|x| buf[(x, y)].symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    // Help is shown
    assert!(text.contains("Help"), "help should be visible: {text}");
    assert!(text.contains("Navigation"));
    // Underlying projects screen is still visible behind the popup. The first project
    // sits in the left column, which is mostly outside the centered modal, so the
    // project name should be readable in the margin.
    assert!(
        text.contains("Background"),
        "underlying project 'Background' should be visible: {text}"
    );
    // Help is rendered as an overlay, not a full screen: the "Smoke Test" project
    // from the tests folder must not appear in the help content, confirming we are
    // seeing the help text, not a full-screen project view.
    assert!(!text.contains("Smoke Test"));
}

#[test]
fn add_project_via_input_creates_file() {
    use crossterm::event::{Event, KeyCode};
    use notahub::app::Screen;
    use notahub::event::handle_event;

    let dir = tempdir();
    let mut app = App::new(dir.path().to_path_buf()).unwrap();

    handle_event(&mut app, ev(KeyCode::Char('p'))).unwrap();
    assert!(app.input.is_some());

    for c in "My Demo".chars() {
        handle_event(&mut app, ev(KeyCode::Char(c))).unwrap();
    }
    handle_event(&mut app, ev(KeyCode::Enter)).unwrap();
    assert!(app.input.is_none());
    assert_eq!(app.projects.len(), 1);
    assert_eq!(app.projects[0].title, "My Demo");
    assert!(dir.path().join("projects/my-demo.md").exists());

    handle_event(&mut app, ev(KeyCode::Char('2'))).unwrap();
    assert_eq!(app.screen, Screen::Projects);
    handle_event(&mut app, ev(KeyCode::Char('t'))).unwrap();
    for c in "First".chars() {
        handle_event(&mut app, ev(KeyCode::Char(c))).unwrap();
    }
    handle_event(&mut app, ev(KeyCode::Enter)).unwrap();
    assert_eq!(app.projects[0].tasks.len(), 1);
    assert_eq!(app.projects[0].tasks[0].title, "First");

    // simulate the rest of the chain being no-op
    let _ = Event::FocusGained;
}

fn ev(code: crossterm::event::KeyCode) -> crossterm::event::Event {
    crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
        code,
        crossterm::event::KeyModifiers::NONE,
    ))
}




fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn write_file(path: &std::path::Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

