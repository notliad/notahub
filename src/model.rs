use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub title: String,
    pub created: DateTime<Utc>,
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub completed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Idea {
    pub id: Uuid,
    pub title: String,
    pub created: DateTime<Utc>,
    pub notes: Vec<String>,
}

impl Project {
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            created: Utc::now(),
            tasks: Vec::new(),
        }
    }
}

impl Idea {
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            created: Utc::now(),
            notes: Vec::new(),
        }
    }
}

impl Task {
    pub fn new(title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            completed: false,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Project,
    Idea,
}
