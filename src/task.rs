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
        
        let subtask_statuses: Vec<_> = self.subtasks.iter()
            .filter_map(|&id| all_tasks.iter().find(|t| t.id == id))
            .map(|t| t.compute_effective_status(all_tasks))
            .collect();
        
        if subtask_statuses.is_empty() {
            return self.status.clone();
        }
        
        // If all subtasks are done, task is effectively done
        if subtask_statuses.iter().all(|s| matches!(s, TaskStatus::Done)) {
            return TaskStatus::Done;
        }
        
        // If any subtask is in progress, task is effectively in progress
        if subtask_statuses.iter().any(|s| matches!(s, TaskStatus::InProgress)) {
            return TaskStatus::InProgress;
        }
        
        // If any subtask is blocked, task is effectively blocked
        if subtask_statuses.iter().any(|s| matches!(s, TaskStatus::Blocked)) {
            return TaskStatus::Blocked;
        }
        
        // Otherwise, use the stored status
        self.status.clone()
    }
    
    pub fn subtask_progress(&self, all_tasks: &[Task]) -> (usize, usize) {
        let total = self.subtasks.len();
        let completed = self.subtasks.iter()
            .filter_map(|&id| all_tasks.iter().find(|t| t.id == id))
            .filter(|t| t.compute_effective_status(all_tasks) == TaskStatus::Done)
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
            TaskStatus::Cancelled => write!(f, "◈ cancelled"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = Task::new(1, "Test Task".to_string(), "Description".to_string(), Priority::High);
        
        assert_eq!(task.id, 1);
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.description, "Description");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::High);
        assert!(task.dependencies.is_empty());
        assert!(task.subtasks.is_empty());
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_set_status_updates_timestamp() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::Medium);
        let initial_updated = task.updated_at;
        
        // Small delay to ensure timestamp difference
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        task.set_status(TaskStatus::InProgress);
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.updated_at > initial_updated);
        assert!(task.completed_at.is_none());
    }

    #[test]
    fn test_set_status_done_sets_completed_at() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::Low);
        
        task.set_status(TaskStatus::Done);
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.completed_at.is_some());
    }

    #[test]
    fn test_dependency_management() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        
        // Add dependencies
        task.add_dependency(2);
        task.add_dependency(3);
        assert_eq!(task.dependencies.len(), 2);
        assert!(task.dependencies.contains(&2));
        assert!(task.dependencies.contains(&3));
        
        // Remove dependency
        task.remove_dependency(2);
        assert_eq!(task.dependencies.len(), 1);
        assert!(!task.dependencies.contains(&2));
        assert!(task.dependencies.contains(&3));
    }

    #[test]
    fn test_subtask_management() {
        let mut task = Task::new(1, "Parent".to_string(), "".to_string(), Priority::High);
        
        task.add_subtask(2);
        task.add_subtask(3);
        
        assert_eq!(task.subtasks.len(), 2);
        assert_eq!(task.subtasks[0], 2);
        assert_eq!(task.subtasks[1], 3);
    }

    #[test]
    fn test_is_ready_with_no_dependencies() {
        let task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        let completed = HashSet::new();
        
        assert!(task.is_ready(&completed));
    }

    #[test]
    fn test_is_ready_with_completed_dependencies() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        task.add_dependency(2);
        task.add_dependency(3);
        
        let mut completed = HashSet::new();
        completed.insert(2);
        completed.insert(3);
        
        assert!(task.is_ready(&completed));
    }

    #[test]
    fn test_is_ready_with_incomplete_dependencies() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        task.add_dependency(2);
        task.add_dependency(3);
        
        let mut completed = HashSet::new();
        completed.insert(2); // Only one dependency completed
        
        assert!(!task.is_ready(&completed));
    }

    #[test]
    fn test_is_ready_non_pending_status() {
        let mut task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        task.set_status(TaskStatus::InProgress);
        
        let completed = HashSet::new();
        assert!(!task.is_ready(&completed)); // Not ready because it's already in progress
    }

    #[test]
    fn test_compute_effective_status_no_subtasks() {
        let task = Task::new(1, "Test".to_string(), "".to_string(), Priority::High);
        let all_tasks = vec![task.clone()];
        
        assert_eq!(task.compute_effective_status(&all_tasks), TaskStatus::Pending);
    }

    #[test]
    fn test_compute_effective_status_all_subtasks_done() {
        let mut parent = Task::new(1, "Parent".to_string(), "".to_string(), Priority::High);
        parent.add_subtask(2);
        parent.add_subtask(3);
        
        let mut subtask1 = Task::new(2, "Sub1".to_string(), "".to_string(), Priority::High);
        subtask1.set_status(TaskStatus::Done);
        
        let mut subtask2 = Task::new(3, "Sub2".to_string(), "".to_string(), Priority::High);
        subtask2.set_status(TaskStatus::Done);
        
        let all_tasks = vec![parent.clone(), subtask1, subtask2];
        
        assert_eq!(parent.compute_effective_status(&all_tasks), TaskStatus::Done);
    }

    #[test]
    fn test_compute_effective_status_mixed_subtasks() {
        let mut parent = Task::new(1, "Parent".to_string(), "".to_string(), Priority::High);
        parent.add_subtask(2);
        parent.add_subtask(3);
        
        let mut subtask1 = Task::new(2, "Sub1".to_string(), "".to_string(), Priority::High);
        subtask1.set_status(TaskStatus::Done);
        
        let mut subtask2 = Task::new(3, "Sub2".to_string(), "".to_string(), Priority::High);
        subtask2.set_status(TaskStatus::InProgress);
        
        let all_tasks = vec![parent.clone(), subtask1, subtask2];
        
        assert_eq!(parent.compute_effective_status(&all_tasks), TaskStatus::InProgress);
    }

    #[test]
    fn test_completed_task_filtering() {
        let mut task1 = Task::new(1, "Recent".to_string(), "".to_string(), Priority::High);
        task1.set_status(TaskStatus::Done);
        
        let mut task2 = Task::new(2, "Old".to_string(), "".to_string(), Priority::High);
        task2.set_status(TaskStatus::Done);
        // Manually set completed_at to 10 minutes ago
        task2.completed_at = Some(Utc::now() - chrono::Duration::minutes(10));
        
        let task3 = Task::new(3, "Pending".to_string(), "".to_string(), Priority::High);
        
        let cutoff_time = Utc::now() - chrono::Duration::minutes(5);
        
        // Task 1 should be kept (recently completed)
        assert!(task1.completed_at.map_or(false, |completed| completed > cutoff_time));
        
        // Task 2 should be filtered out (old completion)
        assert!(!task2.completed_at.map_or(false, |completed| completed > cutoff_time));
        
        // Task 3 should be kept (not completed)
        assert!(matches!(task3.status, TaskStatus::Pending));
    }

    #[test]
    fn test_subtask_progress() {
        let mut parent = Task::new(1, "Parent".to_string(), "".to_string(), Priority::High);
        parent.add_subtask(2);
        parent.add_subtask(3);
        parent.add_subtask(4);
        
        let mut subtask1 = Task::new(2, "Sub1".to_string(), "".to_string(), Priority::High);
        subtask1.set_status(TaskStatus::Done);
        
        let subtask2 = Task::new(3, "Sub2".to_string(), "".to_string(), Priority::High);
        
        let mut subtask3 = Task::new(4, "Sub3".to_string(), "".to_string(), Priority::High);
        subtask3.set_status(TaskStatus::Done);
        
        let all_tasks = vec![parent.clone(), subtask1, subtask2, subtask3];
        
        let (completed, total) = parent.subtask_progress(&all_tasks);
        assert_eq!(completed, 2);
        assert_eq!(total, 3);
    }
}