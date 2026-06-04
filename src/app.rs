use std::path::PathBuf;

use anyhow::Result;
use uuid::Uuid;

use crate::model::{Idea, Project, Task};
use crate::storage;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Projects,
    Ideas,
    Search,
    Help,
}

impl Screen {
    pub fn label(self) -> &'static str {
        match self {
            Screen::Home => "Home",
            Screen::Projects => "Projects",
            Screen::Ideas => "Ideas",
            Screen::Search => "Search",
            Screen::Help => "Help",
        }
    }

    pub fn tab_index(self) -> usize {
        match self {
            Screen::Home => 0,
            Screen::Projects => 1,
            Screen::Ideas => 2,
            Screen::Search => 3,
            Screen::Help => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeFocus {
    Projects,
    Ideas,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectsFocus {
    List,
    Tasks,
    Notes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdeasFocus {
    List,
    Notes,
}

#[derive(Debug, Clone)]
pub enum InputAction {
    NewProject,
    NewTask(Uuid),
    NewTaskNote(Uuid, Uuid),
    NewIdea,
    NewIdeaNote(Uuid),
    Search,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub label: String,
    pub value: String,
    pub action: InputAction,
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    DeleteProject(Uuid),
    DeleteIdea(Uuid),
    DeleteTask(Uuid, Uuid),
    DeleteProjectNote(Uuid, Uuid, usize),
    DeleteIdeaNote(Uuid, usize),
}

#[derive(Debug, Clone)]
pub struct ConfirmState {
    pub message: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteTarget {
    Project,
    Idea,
}

#[derive(Debug, Clone)]
pub struct DeleteMode {
    pub target: DeleteTarget,
}

pub struct App {
    pub data_root: PathBuf,
    pub projects: Vec<Project>,
    pub ideas: Vec<Idea>,
    pub screen: Screen,
    pub prev_screen: Screen,
    pub status: Option<String>,

    pub home_focus: HomeFocus,
    pub home_project_idx: usize,
    pub home_idea_idx: usize,

    pub projects_focus: ProjectsFocus,
    pub project_idx: usize,
    pub task_idx: usize,
    pub task_note_idx: usize,

    pub ideas_focus: IdeasFocus,
    pub idea_idx: usize,
    pub idea_note_idx: usize,

    pub search_query: String,
    pub search_results: Vec<SearchResult>,
    pub search_result_idx: usize,

    pub input: Option<InputState>,
    pub confirm: Option<ConfirmState>,
    pub delete_mode: Option<DeleteMode>,
    pub pending_shortcut: Option<char>,

    pub should_quit: bool,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub kind: SearchKind,
    pub title: String,
    pub snippet: String,
    pub project_id: Option<Uuid>,
    pub idea_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub note_idx: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    Project,
    Task,
    Idea,
    IdeaNote,
}

impl App {
    pub fn new(data_root: PathBuf) -> Result<Self> {
        let projects = storage::load_projects(&data_root)?;
        let ideas = storage::load_ideas(&data_root)?;
        Ok(Self {
            data_root,
            projects,
            ideas,
            screen: Screen::Home,
            prev_screen: Screen::Home,
            status: None,
            home_focus: HomeFocus::Projects,
            home_project_idx: 0,
            home_idea_idx: 0,
            projects_focus: ProjectsFocus::List,
            project_idx: 0,
            task_idx: 0,
            task_note_idx: 0,
            ideas_focus: IdeasFocus::List,
            idea_idx: 0,
            idea_note_idx: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            search_result_idx: 0,
            input: None,
            confirm: None,
            delete_mode: None,
            pending_shortcut: None,
            should_quit: false,
        })
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = Some(msg.into());
    }

    pub fn clear_status(&mut self) {
        self.status = None;
    }

    pub fn go(&mut self, screen: Screen) {
        if self.screen != screen {
            self.prev_screen = self.screen;
            self.screen = screen;
        }
    }

    pub fn go_home(&mut self) {
        self.go(Screen::Home);
    }

    pub fn home_projects(&self) -> &[Project] {
        &self.projects
    }

    pub fn home_ideas(&self) -> &[Idea] {
        &self.ideas
    }

    pub fn current_project(&self) -> Option<&Project> {
        self.projects.get(self.project_idx)
    }

    pub fn current_project_mut(&mut self) -> Option<&mut Project> {
        self.projects.get_mut(self.project_idx)
    }

    pub fn current_task(&self) -> Option<&Task> {
        self.current_project()?.tasks.get(self.task_idx)
    }

    pub fn current_task_mut(&mut self) -> Option<&mut Task> {
        let p = self.projects.get_mut(self.project_idx)?;
        p.tasks.get_mut(self.task_idx)
    }

    pub fn current_idea(&self) -> Option<&Idea> {
        self.ideas.get(self.idea_idx)
    }

    pub fn current_idea_mut(&mut self) -> Option<&mut Idea> {
        self.ideas.get_mut(self.idea_idx)
    }

    pub fn open_search(&mut self) {
        self.go(Screen::Search);
        self.search_query.clear();
        self.search_results.clear();
        self.search_result_idx = 0;
        self.input = Some(InputState {
            label: "Search".to_string(),
            value: String::new(),
            action: InputAction::Search,
        });
    }

    pub fn refresh_search(&mut self) {
        self.search_results = compute_search(&self.projects, &self.ideas, &self.search_query);
        if self.search_result_idx >= self.search_results.len() {
            self.search_result_idx = 0;
        }
    }

    /// Navigate to the search result at the current cursor position.
    /// Returns true if navigation happened, false if there was no result.
    pub fn open_search_result(&mut self) -> bool {
        let Some(result) = self.search_results.get(self.search_result_idx).cloned() else {
            return false;
        };
        self.input = None;
        match result.kind {
            SearchKind::Project => {
                if let Some(p_idx) = self
                    .projects
                    .iter()
                    .position(|p| Some(p.id) == result.project_id)
                {
                    self.project_idx = p_idx;
                    self.task_idx = 0;
                    self.task_note_idx = 0;
                    self.projects_focus = ProjectsFocus::List;
                    self.go(Screen::Projects);
                    true
                } else {
                    false
                }
            }
            SearchKind::Task => {
                if let Some(p_idx) = self
                    .projects
                    .iter()
                    .position(|p| Some(p.id) == result.project_id)
                {
                    if let Some(t_idx) = self.projects[p_idx]
                        .tasks
                        .iter()
                        .position(|t| Some(t.id) == result.task_id)
                    {
                        self.project_idx = p_idx;
                        self.task_idx = t_idx;
                        self.task_note_idx = 0;
                        self.projects_focus = ProjectsFocus::Notes;
                        self.go(Screen::Projects);
                        return true;
                    }
                }
                false
            }
            SearchKind::Idea => {
                if let Some(i_idx) = self
                    .ideas
                    .iter()
                    .position(|i| Some(i.id) == result.idea_id)
                {
                    self.idea_idx = i_idx;
                    self.idea_note_idx = 0;
                    self.ideas_focus = IdeasFocus::List;
                    self.go(Screen::Ideas);
                    true
                } else {
                    false
                }
            }
            SearchKind::IdeaNote => {
                if let Some(i_idx) = self
                    .ideas
                    .iter()
                    .position(|i| Some(i.id) == result.idea_id)
                {
                    self.idea_idx = i_idx;
                    if let Some(n) = result.note_idx {
                        if n < self.ideas[i_idx].notes.len() {
                            self.idea_note_idx = n;
                        }
                    }
                    self.ideas_focus = IdeasFocus::Notes;
                    self.go(Screen::Ideas);
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn clamp_indices(&mut self) {
        if self.projects.is_empty() {
            self.project_idx = 0;
            self.task_idx = 0;
            self.task_note_idx = 0;
        } else {
            if self.project_idx >= self.projects.len() {
                self.project_idx = self.projects.len() - 1;
            }
            let task_len = self.projects[self.project_idx].tasks.len();
            if self.task_idx >= task_len {
                self.task_idx = task_len.saturating_sub(1);
            }
            let note_len = self.projects[self.project_idx]
                .tasks
                .get(self.task_idx)
                .map(|t| t.notes.len())
                .unwrap_or(0);
            if self.task_note_idx >= note_len {
                self.task_note_idx = note_len.saturating_sub(1);
            }
        }
        if !self.ideas.is_empty() && self.idea_idx >= self.ideas.len() {
            self.idea_idx = self.ideas.len() - 1;
        }
        let idea_note_len = self
            .ideas
            .get(self.idea_idx)
            .map(|i| i.notes.len())
            .unwrap_or(0);
        if self.idea_note_idx >= idea_note_len {
            self.idea_note_idx = idea_note_len.saturating_sub(1);
        }
        if !self.projects.is_empty() && self.home_project_idx >= self.projects.len() {
            self.home_project_idx = self.projects.len() - 1;
        }
        if !self.ideas.is_empty() && self.home_idea_idx >= self.ideas.len() {
            self.home_idea_idx = self.ideas.len() - 1;
        }
    }

    pub fn project_by_id(&self, id: Uuid) -> Option<usize> {
        self.projects.iter().position(|p| p.id == id)
    }

    pub fn idea_by_id(&self, id: Uuid) -> Option<usize> {
        self.ideas.iter().position(|i| i.id == id)
    }

    pub fn task_by_id(&self, project_idx: usize, id: Uuid) -> Option<usize> {
        self.projects
            .get(project_idx)?
            .tasks
            .iter()
            .position(|t| t.id == id)
    }
}

pub fn open_shortcut_by_index(app: &mut App, idx: usize) {
    if idx < app.projects.len() {
        let p_idx = idx;
        if let Some(p) = app.projects.get(p_idx) {
            app.project_idx = p_idx;
            app.task_idx = 0;
            app.task_note_idx = 0;
            app.projects_focus = ProjectsFocus::List;
            let _ = p;
            app.go(Screen::Projects);
        }
    } else if idx < app.projects.len() + app.ideas.len() {
        let i_idx = idx - app.projects.len();
        app.idea_idx = i_idx;
        app.idea_note_idx = 0;
        app.ideas_focus = IdeasFocus::List;
        app.go(Screen::Ideas);
    } else {
        app.set_status("No item with that shortcut");
    }
}

pub fn open_shortcut_by_char(app: &mut App, c: char) {
    if let Some(idx) = SINGLE_SHORTCUT_LETTERS.iter().position(|&sc| sc == c) {
        open_shortcut_by_index(app, idx);
    } else {
        app.set_status("No item with that shortcut");
    }
}

pub fn compute_search(projects: &[Project], ideas: &[Idea], query: &str) -> Vec<SearchResult> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();

    for p in projects {
        if p.title.to_lowercase().contains(&q) {
            out.push(SearchResult {
                kind: SearchKind::Project,
                title: p.title.clone(),
                snippet: snippet_for(&p.title, &q),
                project_id: Some(p.id),
                idea_id: None,
                task_id: None,
                note_idx: None,
            });
        }
        for t in &p.tasks {
            if t.title.to_lowercase().contains(&q)
                || t.notes.iter().any(|n| n.to_lowercase().contains(&q))
            {
                let snippet = if t.title.to_lowercase().contains(&q) {
                    snippet_for(&t.title, &q)
                } else {
                    t.notes
                        .iter()
                        .find(|n| n.to_lowercase().contains(&q))
                        .map(|n| snippet_for(n, &q))
                        .unwrap_or_default()
                };
                out.push(SearchResult {
                    kind: SearchKind::Task,
                    title: format!("{} / {}", p.title, t.title),
                    snippet,
                    project_id: Some(p.id),
                    idea_id: None,
                    task_id: Some(t.id),
                    note_idx: None,
                });
            }
        }
    }

    for i in ideas {
        if i.title.to_lowercase().contains(&q) {
            out.push(SearchResult {
                kind: SearchKind::Idea,
                title: i.title.clone(),
                snippet: snippet_for(&i.title, &q),
                project_id: None,
                idea_id: Some(i.id),
                task_id: None,
                note_idx: None,
            });
        }
        for (n_idx, n) in i.notes.iter().enumerate() {
            if n.to_lowercase().contains(&q) {
                out.push(SearchResult {
                    kind: SearchKind::IdeaNote,
                    title: format!("{} / note {}", i.title, n_idx + 1),
                    snippet: snippet_for(n, &q),
                    project_id: None,
                    idea_id: Some(i.id),
                    task_id: None,
                    note_idx: Some(n_idx),
                });
            }
        }
    }

    out
}

fn snippet_for(text: &str, q: &str) -> String {
    let lower = text.to_lowercase();
    if let Some(pos) = lower.find(q) {
        let start = pos.saturating_sub(20);
        let end = (pos + q.len() + 40).min(text.len());
        let mut s = String::new();
        if start > 0 {
            s.push('…');
        }
        s.push_str(&text[start..end]);
        if end < text.len() {
            s.push('…');
        }
        s
    } else {
        text.chars().take(60).collect()
    }
}

const SINGLE_SHORTCUT_LETTERS: [char; 17] = [
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'm', 'o', 'q', 'r', 's', 'u', 'v', 'w', 'y', 'z',
];

pub fn shortcut_for_index(i: usize) -> Option<String> {
    if i < 17 {
        Some(SINGLE_SHORTCUT_LETTERS[i].to_string())
    } else {
        let remainder = i - 17;
        let first_idx = remainder / 26;
        let second_idx = remainder % 26;
        if first_idx < 17 {
            Some(format!(
                "{}{}",
                SINGLE_SHORTCUT_LETTERS[first_idx],
                (b'a' + second_idx as u8) as char
            ))
        } else {
            None
        }
    }
}

pub fn is_shortcut_letter(c: char) -> bool {
    SINGLE_SHORTCUT_LETTERS.contains(&c)
}

pub fn single_shortcut_index(c: char) -> Option<usize> {
    SINGLE_SHORTCUT_LETTERS.iter().position(|&sc| sc == c)
}

pub fn shortcut_has_two_letter_forms(first_char: char, total_items: usize) -> bool {
    if total_items <= 17 {
        return false;
    }
    let Some(first_idx) = SINGLE_SHORTCUT_LETTERS.iter().position(|&c| c == first_char) else {
        return false;
    };
    let two_letter_start = 17 + first_idx * 26;
    two_letter_start < total_items
}

pub fn index_for_two_letter_shortcut(first: char, second: char) -> Option<usize> {
    let first_idx = SINGLE_SHORTCUT_LETTERS.iter().position(|&c| c == first)?;
    let second_idx = (second as u8 - b'a') as usize;
    Some(17 + first_idx * 26 + second_idx)
}

