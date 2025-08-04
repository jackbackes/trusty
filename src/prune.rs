use crate::task::{Task, TaskStatus};
use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneHistory {
    /// Map of task ID to prune suggestion history
    pub suggestions: HashMap<u32, PruneSuggestionHistory>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneSuggestionHistory {
    /// Number of times this task has been suggested for pruning
    pub suggestion_count: u32,
    /// Last time this task was suggested for pruning
    pub last_suggested: DateTime<Utc>,
    /// Next time this task should be considered for pruning (exponential backoff)
    pub next_suggestion_after: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PruneAction {
    Complete,
    Cancel,
    Skip,
}

#[derive(Debug, Clone)]
pub struct PruneSuggestion {
    pub task: Task,
    pub action: PruneAction,
    pub reason: String,
    pub confidence: f32, // 0.0 to 1.0
}

pub struct PruneAnalyzer {
    history: PruneHistory,
    history_path: PathBuf,
}

impl PruneAnalyzer {
    pub fn new(base_path: PathBuf) -> anyhow::Result<Self> {
        let history_path = base_path.join("prune_history.json");
        let history = if history_path.exists() {
            let content = std::fs::read_to_string(&history_path)?;
            serde_json::from_str(&content)?
        } else {
            PruneHistory {
                suggestions: HashMap::new(),
            }
        };
        
        Ok(Self {
            history,
            history_path,
        })
    }
    
    pub fn save_history(&self) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(&self.history)?;
        std::fs::write(&self.history_path, content)?;
        Ok(())
    }
    
    pub fn analyze_tasks(&self, tasks: &[Task]) -> Vec<PruneSuggestion> {
        let mut suggestions = Vec::new();
        let now = Utc::now();
        
        // Calculate age percentiles
        let mut task_ages: Vec<i64> = tasks.iter()
            .filter(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress))
            .map(|t| (now - t.created_at).num_days())
            .collect();
        task_ages.sort();
        
        let p80_age = if !task_ages.is_empty() {
            let idx = (task_ages.len() as f32 * 0.8) as usize;
            task_ages.get(idx).copied().unwrap_or(30)
        } else {
            30 // Default to 30 days if no tasks
        };
        
        for task in tasks {
            // Skip if in backoff period
            if let Some(history) = self.history.suggestions.get(&task.id) {
                if now < history.next_suggestion_after {
                    continue;
                }
            }
            
            // Skip already completed or cancelled tasks
            if matches!(task.status, TaskStatus::Done | TaskStatus::Cancelled) {
                continue;
            }
            
            if let Some(suggestion) = self.analyze_single_task(task, tasks, p80_age, now) {
                suggestions.push(suggestion);
            }
        }
        
        // Sort by confidence (highest first) and limit
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        suggestions
    }
    
    fn analyze_single_task(&self, task: &Task, all_tasks: &[Task], p80_age: i64, now: DateTime<Utc>) -> Option<PruneSuggestion> {
        let age_days = (now - task.created_at).num_days();
        let last_update_days = (now - task.updated_at).num_days();
        
        // Check if all subtasks are complete
        if !task.subtasks.is_empty() {
            let (completed, total) = task.subtask_progress(all_tasks);
            if completed == total && total > 0 {
                return Some(PruneSuggestion {
                    task: task.clone(),
                    action: PruneAction::Complete,
                    reason: format!("All {} subtasks are complete", total),
                    confidence: 0.9,
                });
            }
        }
        
        // Check if blocked by completed or missing dependencies
        if !task.dependencies.is_empty() {
            let deps_status: Vec<_> = task.dependencies.iter().map(|&dep_id| {
                all_tasks.iter()
                    .find(|t| t.id == dep_id)
                    .map(|t| t.status.clone())
            }).collect();
            
            let all_deps_resolved = deps_status.iter().all(|status| {
                matches!(status, Some(TaskStatus::Done))
            });
            
            let has_missing_deps = deps_status.iter().any(|status| status.is_none());
            
            if has_missing_deps {
                return Some(PruneSuggestion {
                    task: task.clone(),
                    action: PruneAction::Cancel,
                    reason: "Task depends on deleted or missing tasks".to_string(),
                    confidence: 0.9,
                });
            }
            
            if all_deps_resolved && last_update_days > 3 {
                return Some(PruneSuggestion {
                    task: task.clone(),
                    action: PruneAction::Cancel,
                    reason: format!("All dependencies completed but task untouched for {} days", last_update_days),
                    confidence: 0.7,
                });
            }
        }
        
        // Check if task is in the P80+ age bracket
        if age_days >= p80_age {
            let confidence = match task.priority {
                crate::task::Priority::Low => 0.8,
                crate::task::Priority::Medium => 0.6,
                crate::task::Priority::High => 0.4,
            };
            
            // If it hasn't been updated in a long time, it's probably stale
            if last_update_days > 30 {
                return Some(PruneSuggestion {
                    task: task.clone(),
                    action: PruneAction::Cancel,
                    reason: format!("Task is {} days old (P80+) with no updates in {} days", age_days, last_update_days),
                    confidence: confidence + 0.1,
                });
            }
        }
        
        // Check if this is an orphaned subtask (parent is complete)
        let parent_task = all_tasks.iter().find(|t| t.subtasks.contains(&task.id));
        if let Some(parent) = parent_task {
            if parent.status == TaskStatus::Done {
                return Some(PruneSuggestion {
                    task: task.clone(),
                    action: PruneAction::Cancel,
                    reason: format!("Orphaned subtask - parent task #{} is already complete", parent.id),
                    confidence: 0.8,
                });
            }
        }
        
        // Check for vague descriptions on old tasks
        if task.description.is_empty() && age_days > 14 {
            return Some(PruneSuggestion {
                task: task.clone(),
                action: PruneAction::Cancel,
                reason: "Old task with no description - likely outdated".to_string(),
                confidence: 0.6,
            });
        }
        
        // Check for repeatedly skipped tasks (would need more tracking for this)
        // For now, use age and priority as a proxy
        if matches!(task.priority, crate::task::Priority::Low) && age_days > 60 {
            return Some(PruneSuggestion {
                task: task.clone(),
                action: PruneAction::Cancel,
                reason: format!("Low priority task ignored for {} days", age_days),
                confidence: 0.7,
            });
        }
        
        None
    }
    
    pub fn record_suggestion(&mut self, task_id: u32, action_taken: PruneAction) {
        let now = Utc::now();
        let entry = self.history.suggestions.entry(task_id).or_insert(PruneSuggestionHistory {
            suggestion_count: 0,
            last_suggested: now,
            next_suggestion_after: now,
        });
        
        entry.suggestion_count += 1;
        entry.last_suggested = now;
        
        // Exponential backoff: 1 week, 2 weeks, 4 weeks, 8 weeks, etc.
        let backoff_days = 7 * (1 << entry.suggestion_count.min(5));
        entry.next_suggestion_after = now + Duration::days(backoff_days);
        
        // If action was Skip, increase backoff more aggressively
        if action_taken == PruneAction::Skip {
            entry.next_suggestion_after = entry.next_suggestion_after + Duration::days(backoff_days / 2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::Priority;
    
    #[test]
    fn test_prune_all_subtasks_complete() {
        let analyzer = PruneAnalyzer::new(std::env::temp_dir()).unwrap();
        
        let mut parent = Task::new(1, "Parent".to_string(), "".to_string(), Priority::High);
        parent.add_subtask(2);
        parent.add_subtask(3);
        
        let mut sub1 = Task::new(2, "Sub1".to_string(), "".to_string(), Priority::High);
        sub1.set_status(TaskStatus::Done);
        
        let mut sub2 = Task::new(3, "Sub2".to_string(), "".to_string(), Priority::High);
        sub2.set_status(TaskStatus::Done);
        
        let tasks = vec![parent.clone(), sub1, sub2];
        let suggestions = analyzer.analyze_tasks(&tasks);
        
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].task.id, 1);
        assert_eq!(suggestions[0].action, PruneAction::Complete);
        assert!(suggestions[0].confidence > 0.8);
    }
    
    #[test]
    fn test_prune_old_task_no_description() {
        let analyzer = PruneAnalyzer::new(std::env::temp_dir()).unwrap();
        
        let mut task = Task::new(1, "Old task".to_string(), "".to_string(), Priority::Low);
        // Manually set created_at to 30 days ago
        task.created_at = Utc::now() - Duration::days(30);
        
        let tasks = vec![task];
        let suggestions = analyzer.analyze_tasks(&tasks);
        
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].action, PruneAction::Cancel);
    }
    
    #[test]
    fn test_exponential_backoff() {
        let mut analyzer = PruneAnalyzer::new(std::env::temp_dir()).unwrap();
        
        // Record first suggestion
        analyzer.record_suggestion(1, PruneAction::Skip);
        let first_backoff = {
            let entry = analyzer.history.suggestions.get(&1).unwrap();
            assert_eq!(entry.suggestion_count, 1);
            entry.next_suggestion_after - entry.last_suggested
        };
        
        // Record second suggestion
        analyzer.record_suggestion(1, PruneAction::Skip);
        let second_backoff = {
            let entry = analyzer.history.suggestions.get(&1).unwrap();
            assert_eq!(entry.suggestion_count, 2);
            entry.next_suggestion_after - entry.last_suggested
        };
        
        // Check that backoff period increases
        assert!(second_backoff > first_backoff);
    }
}