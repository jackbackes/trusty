use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Done,
    Blocked,
    Deferred,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Complexity {
    Simple,
    Medium,
    Complex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub complexity: Option<Complexity>,
    pub dependencies: HashSet<u32>,
    pub subtasks: Vec<u32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

impl Task {
    pub fn new(id: u32, title: String, description: String, priority: Priority) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            description,
            status: TaskStatus::Pending,
            priority,
            complexity: None,
            dependencies: HashSet::new(),
            subtasks: Vec::new(),
            created_at: now,
            updated_at: now,
            completed_at: None,
            tags: Vec::new(),
        }
    }

    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status.clone();
        self.updated_at = Utc::now();
        
        if status == TaskStatus::Done {
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn add_dependency(&mut self, dep_id: u32) {
        self.dependencies.insert(dep_id);
        self.updated_at = Utc::now();
    }

    pub fn remove_dependency(&mut self, dep_id: u32) {
        self.dependencies.remove(&dep_id);
        self.updated_at = Utc::now();
    }

    pub fn add_subtask(&mut self, subtask_id: u32) {
        self.subtasks.push(subtask_id);
        self.updated_at = Utc::now();
    }

    pub fn is_ready(&self, completed_tasks: &HashSet<u32>) -> bool {
        match self.status {
            TaskStatus::Pending => self.dependencies.is_subset(completed_tasks),
            _ => false,
        }
    }

    pub fn compute_effective_status(&self, all_tasks: &[Task]) -> TaskStatus {
        if self.subtasks.is_empty() {
            return self.status.clone();
        }

        let subtask_statuses: Vec<TaskStatus> = self.subtasks
            .iter()
            .filter_map(|&id| all_tasks.iter().find(|t| t.id == id))
            .map(|t| t.compute_effective_status(all_tasks))
            .collect();

        if subtask_statuses.is_empty() {
            return self.status.clone();
        }

        // If all subtasks are done, parent is done
        if subtask_statuses.iter().all(|s| matches!(s, TaskStatus::Done)) {
            return TaskStatus::Done;
        }

        // If any subtask is cancelled, parent is blocked
        if subtask_statuses.iter().any(|s| matches!(s, TaskStatus::Cancelled)) {
            return TaskStatus::Blocked;
        }

        // If any subtask is blocked, parent is blocked
        if subtask_statuses.iter().any(|s| matches!(s, TaskStatus::Blocked)) {
            return TaskStatus::Blocked;
        }

        // If any subtask is in progress, parent is in progress
        if subtask_statuses.iter().any(|s| matches!(s, TaskStatus::InProgress)) {
            return TaskStatus::InProgress;
        }

        // If all subtasks are deferred, parent is deferred
        if subtask_statuses.iter().all(|s| matches!(s, TaskStatus::Deferred)) {
            return TaskStatus::Deferred;
        }

        // Otherwise, parent is pending
        TaskStatus::Pending
    }

    pub fn subtask_progress(&self, all_tasks: &[Task]) -> (usize, usize) {
        let total = self.subtasks.len();
        let completed = self.subtasks
            .iter()
            .filter(|&&id| {
                all_tasks.iter()
                    .find(|t| t.id == id)
                    .map(|t| matches!(t.compute_effective_status(all_tasks), TaskStatus::Done))
                    .unwrap_or(false)
            })
            .count();
        (completed, total)
    }
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "○ pending"),
            TaskStatus::InProgress => write!(f, "◐ in-progress"),
            TaskStatus::Done => write!(f, "● done"),
            TaskStatus::Blocked => write!(f, "◻ blocked"),
            TaskStatus::Deferred => write!(f, "◇ deferred"),
            TaskStatus::Cancelled => write!(f, "✗ cancelled"),
        }
    }
}

impl std::fmt::Display for Priority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
        }
    }
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Complexity::Simple => write!(f, "simple"),
            Complexity::Medium => write!(f, "medium"),
            Complexity::Complex => write!(f, "complex"),
        }
    }
}