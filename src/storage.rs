use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

use crate::model::{Idea, Project, Task};

const PROJECTS_DIR: &str = "projects";
const IDEAS_DIR: &str = "ideas";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Frontmatter {
    id: uuid::Uuid,
    title: String,
    created: DateTime<Utc>,
    #[serde(default)]
    kind: String,
}

pub fn data_root() -> PathBuf {
    if let Ok(custom) = std::env::var("NOTAHUB_HOME") {
        return PathBuf::from(custom);
    }
    if let Some(mut base) = dirs::data_local_dir() {
        base.push("notahub");
        return base;
    }
    PathBuf::from(".notahub")
}

fn ensure_dirs(root: &Path) -> Result<()> {
    fs::create_dir_all(root.join(PROJECTS_DIR))
        .with_context(|| format!("creating {}", PROJECTS_DIR))?;
    fs::create_dir_all(root.join(IDEAS_DIR))
        .with_context(|| format!("creating {}", IDEAS_DIR))?;
    Ok(())
}

fn slugify(title: &str) -> String {
    let mut s: String = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}

fn project_path(root: &Path, project: &Project) -> PathBuf {
    let slug = if slugify(&project.title).is_empty() {
        project.id.to_string()
    } else {
        slugify(&project.title)
    };
    root.join(PROJECTS_DIR).join(format!("{slug}.md"))
}

fn idea_path(root: &Path, idea: &Idea) -> PathBuf {
    let slug = if slugify(&idea.title).is_empty() {
        idea.id.to_string()
    } else {
        slugify(&idea.title)
    };
    root.join(IDEAS_DIR).join(format!("{slug}.md"))
}

pub fn load_projects(root: &Path) -> Result<Vec<Project>> {
    let dir = root.join(PROJECTS_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            match parse_project(&raw) {
                Ok(p) => out.push(p),
                Err(e) => eprintln!("skip {}: {e}", path.display()),
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(out)
}

pub fn load_ideas(root: &Path) -> Result<Vec<Idea>> {
    let dir = root.join(IDEAS_DIR);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let raw = fs::read_to_string(&path)
                .with_context(|| format!("reading {}", path.display()))?;
            match parse_idea(&raw) {
                Ok(p) => out.push(p),
                Err(e) => eprintln!("skip {}: {e}", path.display()),
            }
        }
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    Ok(out)
}

pub fn save_project(root: &Path, project: &Project) -> Result<()> {
    ensure_dirs(root)?;
    let path = project_path(root, project);
    let raw = serialize_project(project);
    atomic_write(&path, &raw)
}

pub fn save_idea(root: &Path, idea: &Idea) -> Result<()> {
    ensure_dirs(root)?;
    let path = idea_path(root, idea);
    let raw = serialize_idea(idea);
    atomic_write(&path, &raw)
}

pub fn delete_project(root: &Path, project: &Project) -> Result<()> {
    let path = project_path(root, project);
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

pub fn delete_idea(root: &Path, idea: &Idea) -> Result<()> {
    let path = idea_path(root, idea);
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
    }
    Ok(())
}

fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("md.tmp");
    fs::write(&tmp, content).with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("renaming {}", path.display()))?;
    Ok(())
}

fn split_frontmatter(raw: &str) -> Result<(&str, &str)> {
    let trimmed = raw.trim_start_matches('\u{feff}');
    let rest = trimmed
        .strip_prefix("---\n")
        .or_else(|| trimmed.strip_prefix("---\r\n"))
        .ok_or_else(|| anyhow!("missing frontmatter"))?;
    let end = rest
        .find("\n---")
        .ok_or_else(|| anyhow!("unterminated frontmatter"))?;
    let fm = &rest[..end];
    let body = rest[end..].trim_start_matches("\n---").trim_start_matches('\r');
    let body = body.trim_start_matches('\n');
    Ok((fm, body))
}

fn parse_project(raw: &str) -> Result<Project> {
    let (fm, body) = split_frontmatter(raw)?;
    let front: Frontmatter = serde_yaml::from_str(fm)?;
    let mut tasks = Vec::new();
    let mut current: Option<Task> = None;

    for line in body.lines() {
        if let Some(task) = parse_task_line(line) {
            if let Some(prev) = current.take() {
                tasks.push(prev);
            }
            current = Some(task);
        } else if let Some(note) = line.strip_prefix("  ") {
            if let Some(t) = current.as_mut() {
                if !note.trim().is_empty() {
                    t.notes.push(note.trim_end().to_string());
                }
            }
        } else if line.trim().is_empty() {
            // paragraph break, keep current
        } else if current.is_none() && !line.trim().is_empty() {
            // outside any task, ignore stray lines
        }
    }
    if let Some(prev) = current.take() {
        tasks.push(prev);
    }

    Ok(Project {
        id: front.id,
        title: front.title,
        created: front.created,
        tasks,
    })
}

fn parse_task_line(line: &str) -> Option<Task> {
    let trimmed = line.trim_start();
    let after_bracket = trimmed.strip_prefix("- [ ]").or_else(|| trimmed.strip_prefix("- [x]"))?;
    let completed = trimmed.starts_with("- [x]");
    let rest = after_bracket.trim_start();
    let (title, id) = extract_id_comment(rest);
    Some(Task {
        id,
        title,
        completed,
        notes: Vec::new(),
    })
}

fn extract_id_comment(s: &str) -> (String, Uuid) {
    if let Some(start) = s.find("<!--") {
        if let Some(end) = s[start..].find("-->") {
            let comment = &s[start + 4..start + end];
            let mut id = Uuid::new_v4();
            for part in comment.split_whitespace() {
                if let Some(val) = part.strip_prefix("id:") {
                    if let Ok(parsed) = Uuid::parse_str(val) {
                        id = parsed;
                    }
                }
            }
            let title = s[..start].trim().to_string();
            return (title, id);
        }
    }
    (s.trim().to_string(), Uuid::new_v4())
}

fn parse_idea(raw: &str) -> Result<Idea> {
    let (fm, body) = split_frontmatter(raw)?;
    let front: Frontmatter = serde_yaml::from_str(fm)?;
    let mut notes = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            notes.push(rest.trim().to_string());
        } else if trimmed.is_empty() {
            // skip
        } else if notes.is_empty() {
            // first non-bullet line becomes the body of a single note
            notes.push(trimmed.to_string());
        } else {
            notes.push(trimmed.to_string());
        }
    }
    Ok(Idea {
        id: front.id,
        title: front.title,
        created: front.created,
        notes,
    })
}

fn serialize_project(project: &Project) -> String {
    let front = Frontmatter {
        id: project.id,
        title: project.title.clone(),
        created: project.created,
        kind: "project".to_string(),
    };
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&serde_yaml::to_string(&front).unwrap());
    out.push_str("---\n\n");
    for task in &project.tasks {
        let mark = if task.completed { 'x' } else { ' ' };
        out.push_str(&format!(
            "- [{mark}] {} <!-- id:{} -->\n",
            task.title,
            task.id
        ));
        for note in &task.notes {
            out.push_str(&format!("  {note}\n"));
        }
        out.push('\n');
    }
    out
}

fn serialize_idea(idea: &Idea) -> String {
    let front = Frontmatter {
        id: idea.id,
        title: idea.title.clone(),
        created: idea.created,
        kind: "idea".to_string(),
    };
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&serde_yaml::to_string(&front).unwrap());
    out.push_str("---\n\n");
    for note in &idea.notes {
        out.push_str(&format!("- {note}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_project() {
        let mut p = Project::new("Test".to_string());
        let t = Task::new("First".to_string());
        p.tasks.push(t);
        let raw = serialize_project(&p);
        let parsed = parse_project(&raw).unwrap();
        assert_eq!(parsed.title, "Test");
        assert_eq!(parsed.tasks.len(), 1);
        assert_eq!(parsed.tasks[0].title, "First");
    }

    #[test]
    fn roundtrip_idea() {
        let mut i = Idea::new("Spark".to_string());
        i.notes.push("hello".to_string());
        i.notes.push("world".to_string());
        let raw = serialize_idea(&i);
        let parsed = parse_idea(&raw).unwrap();
        assert_eq!(parsed.title, "Spark");
        assert_eq!(parsed.notes, vec!["hello", "world"]);
    }
}
