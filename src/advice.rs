use anyhow::Result;
use chrono::Utc;
use colored::*;
use crate::task::{Task, TaskStatus};
use crate::storage::TaskStorage;
use crate::claude_integration::ClaudeTaskAdvice;
use std::path::PathBuf;

pub struct TaskAdvisor {
    task: Task,
    all_tasks: Vec<Task>,
}

#[derive(Debug)]
pub struct TaskAdvice {
    pub primary_recommendation: Recommendation,
    pub reasons: Vec<String>,
    pub context: TaskContext,
    pub suggested_actions: Vec<SuggestedAction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Recommendation {
    Complete,
    Continue,
    Cancel,
    Defer,
    Decompose,
    Review,
}

#[derive(Debug)]
pub struct TaskContext {
    pub age_days: i64,
    pub time_in_current_status: i64,
    pub blocking_tasks: Vec<u32>,
    pub dependent_tasks: Vec<u32>,
    pub completion_percentage: Option<(usize, usize)>,
    pub similar_completed_tasks: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SuggestedAction {
    pub command: String,
    pub description: String,
}

impl TaskAdvisor {
    pub fn new(task: Task, storage: &TaskStorage, _tasks_dir: PathBuf) -> Result<Self> {
        let all_tasks = storage.list_all_tasks()?;
        
        Ok(Self {
            task,
            all_tasks,
        })
    }

    pub fn analyze(&mut self) -> TaskAdvice {
        let context = self.build_context();
        let (recommendation, reasons) = self.determine_recommendation(&context);
        let suggested_actions = self.generate_actions(&recommendation, &context);

        TaskAdvice {
            primary_recommendation: recommendation,
            reasons,
            context,
            suggested_actions,
        }
    }

    fn build_context(&self) -> TaskContext {
        let now = Utc::now();
        let age_days = (now - self.task.created_at).num_days();
        
        let time_in_current_status = match self.task.status {
            TaskStatus::Done => {
                self.task.completed_at
                    .map(|completed| (now - completed).num_days())
                    .unwrap_or(0)
            }
            _ => (now - self.task.updated_at).num_days(),
        };

        // Find tasks that depend on this task
        let dependent_tasks: Vec<u32> = self.all_tasks.iter()
            .filter(|t| t.dependencies.contains(&self.task.id))
            .map(|t| t.id)
            .collect();

        // Find tasks this task is waiting on
        let blocking_tasks: Vec<u32> = self.task.dependencies.iter()
            .filter(|&&dep_id| {
                self.all_tasks.iter()
                    .find(|t| t.id == dep_id)
                    .map(|t| t.status != TaskStatus::Done)
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        // Calculate completion percentage if has subtasks
        let completion_percentage = if !self.task.subtasks.is_empty() {
            Some(self.task.subtask_progress(&self.all_tasks))
        } else {
            None
        };

        // Find similar completed tasks
        let similar_completed_tasks = self.find_similar_completed_tasks();

        TaskContext {
            age_days,
            time_in_current_status,
            blocking_tasks,
            dependent_tasks,
            completion_percentage,
            similar_completed_tasks,
        }
    }

    fn find_similar_completed_tasks(&self) -> Vec<String> {
        let task_words: Vec<&str> = self.task.title.split_whitespace().collect();
        
        self.all_tasks.iter()
            .filter(|t| t.status == TaskStatus::Done && t.id != self.task.id)
            .filter(|t| {
                let other_words: Vec<&str> = t.title.split_whitespace().collect();
                // Check if shares significant words
                let common_words = task_words.iter()
                    .filter(|w| w.len() > 3) // Skip short words
                    .filter(|w| other_words.contains(w))
                    .count();
                common_words >= 2 || 
                (common_words == 1 && task_words.len() <= 3)
            })
            .map(|t| t.title.clone())
            .take(3)
            .collect()
    }

    fn determine_recommendation(&self, context: &TaskContext) -> (Recommendation, Vec<String>) {
        let mut reasons = Vec::new();
        
        // Already completed
        if self.task.status == TaskStatus::Done {
            if context.time_in_current_status > 7 {
                reasons.push("Task has been completed for over a week".to_string());
                return (Recommendation::Review, reasons);
            } else {
                reasons.push("Task is already completed".to_string());
                return (Recommendation::Complete, reasons);
            }
        }

        // Blocked by dependencies
        if !context.blocking_tasks.is_empty() {
            reasons.push(format!("Blocked by {} incomplete dependencies", context.blocking_tasks.len()));
            if context.age_days > 30 {
                reasons.push("Has been blocked for over a month".to_string());
                return (Recommendation::Defer, reasons);
            } else {
                return (Recommendation::Continue, reasons);
            }
        }

        // Has subtasks
        if let Some((completed, total)) = context.completion_percentage {
            let percentage = (completed as f32 / total as f32) * 100.0;
            
            if percentage >= 80.0 {
                reasons.push(format!("{}/{} subtasks completed ({:.0}%)", completed, total, percentage));
                reasons.push("Nearly all subtasks are done".to_string());
                return (Recommendation::Complete, reasons);
            } else if percentage >= 50.0 {
                reasons.push(format!("{}/{} subtasks completed ({:.0}%)", completed, total, percentage));
                return (Recommendation::Continue, reasons);
            } else if total > 5 && completed == 0 {
                reasons.push(format!("Has {} subtasks but none completed", total));
                reasons.push("Consider if this task is too large".to_string());
                return (Recommendation::Decompose, reasons);
            }
        }

        // Status-specific logic
        match self.task.status {
            TaskStatus::InProgress => {
                if context.time_in_current_status > 14 {
                    reasons.push(format!("In progress for {} days", context.time_in_current_status));
                    reasons.push("May be stalled".to_string());
                    return (Recommendation::Review, reasons);
                } else {
                    reasons.push("Currently being worked on".to_string());
                    return (Recommendation::Continue, reasons);
                }
            }
            TaskStatus::Blocked => {
                if context.age_days > 30 {
                    reasons.push(format!("Blocked for {} days", context.age_days));
                    return (Recommendation::Cancel, reasons);
                } else {
                    reasons.push("Task is blocked".to_string());
                    return (Recommendation::Review, reasons);
                }
            }
            TaskStatus::Deferred => {
                if context.age_days > 60 {
                    reasons.push(format!("Deferred {} days ago", context.age_days));
                    reasons.push("May no longer be relevant".to_string());
                    return (Recommendation::Cancel, reasons);
                } else {
                    return (Recommendation::Continue, reasons);
                }
            }
            TaskStatus::Cancelled => {
                reasons.push("Task is already cancelled".to_string());
                return (Recommendation::Cancel, reasons);
            }
            TaskStatus::Pending => {
                // Check if similar tasks were completed
                if !context.similar_completed_tasks.is_empty() {
                    reasons.push("Similar tasks have been completed".to_string());
                    reasons.push("May be duplicate or already addressed".to_string());
                    return (Recommendation::Review, reasons);
                }

                // Old pending task
                if context.age_days > 30 {
                    reasons.push(format!("Created {} days ago but not started", context.age_days));
                    if self.task.priority == crate::task::Priority::Low {
                        reasons.push("Low priority task".to_string());
                        return (Recommendation::Cancel, reasons);
                    } else {
                        return (Recommendation::Review, reasons);
                    }
                }

                // Task with many dependents
                if context.dependent_tasks.len() >= 3 {
                    reasons.push(format!("{} other tasks depend on this", context.dependent_tasks.len()));
                    reasons.push("High impact task".to_string());
                    return (Recommendation::Continue, reasons);
                }

                // Complex task without subtasks
                if self.task.complexity == Some(crate::task::Complexity::Complex) && self.task.subtasks.is_empty() {
                    reasons.push("Complex task without subtasks".to_string());
                    return (Recommendation::Decompose, reasons);
                }

                // Default for pending
                reasons.push("Ready to be started".to_string());
                return (Recommendation::Continue, reasons);
            }
            TaskStatus::Done => {
                // This case is handled at the beginning of the function
                unreachable!("Done status should be handled earlier");
            }
        }
    }

    fn generate_actions(&self, recommendation: &Recommendation, context: &TaskContext) -> Vec<SuggestedAction> {
        let mut actions = Vec::new();
        let id = self.task.id;

        match recommendation {
            Recommendation::Complete => {
                if self.task.status != TaskStatus::Done {
                    actions.push(SuggestedAction {
                        command: format!("trusty complete {}", id),
                        description: "Mark this task as complete".to_string(),
                    });
                }
            }
            Recommendation::Continue => {
                if self.task.status == TaskStatus::Pending {
                    actions.push(SuggestedAction {
                        command: format!("trusty set-status --id={} --status=in-progress", id),
                        description: "Start working on this task".to_string(),
                    });
                }
                if !self.task.subtasks.is_empty() && context.completion_percentage.map(|(c, _)| c == 0).unwrap_or(false) {
                    actions.push(SuggestedAction {
                        command: format!("trusty show {} --with-subtasks", id),
                        description: "Review subtasks and start with the first one".to_string(),
                    });
                }
            }
            Recommendation::Cancel => {
                if self.task.status != TaskStatus::Cancelled {
                    actions.push(SuggestedAction {
                        command: format!("trusty set-status --id={} --status=cancelled", id),
                        description: "Cancel this task".to_string(),
                    });
                }
            }
            Recommendation::Defer => {
                if self.task.status != TaskStatus::Deferred {
                    actions.push(SuggestedAction {
                        command: format!("trusty set-status --id={} --status=deferred", id),
                        description: "Defer this task until dependencies are resolved".to_string(),
                    });
                }
                if !context.blocking_tasks.is_empty() {
                    actions.push(SuggestedAction {
                        command: format!("trusty show {}", context.blocking_tasks[0]),
                        description: format!("Review blocking task #{}", context.blocking_tasks[0]),
                    });
                }
            }
            Recommendation::Decompose => {
                actions.push(SuggestedAction {
                    command: format!("trusty decompose {} --preview", id),
                    description: "Preview AI-suggested subtasks".to_string(),
                });
                actions.push(SuggestedAction {
                    command: format!("trusty add-subtask --task={} \"Subtask title\"", id),
                    description: "Manually add subtasks".to_string(),
                });
            }
            Recommendation::Review => {
                actions.push(SuggestedAction {
                    command: format!("trusty show {}", id),
                    description: "Review task details".to_string(),
                });
                
                if self.task.description.is_empty() {
                    actions.push(SuggestedAction {
                        command: format!("trusty edit {} --description \"...\"", id),
                        description: "Add more context with a description".to_string(),
                    });
                }
                
                if !context.similar_completed_tasks.is_empty() {
                    actions.push(SuggestedAction {
                        command: "trusty list --completed --recent=1440".to_string(),
                        description: "Review recently completed similar tasks".to_string(),
                    });
                }
            }
        }

        actions
    }
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recommendation::Complete => write!(f, "{}", "✅ Complete".green()),
            Recommendation::Continue => write!(f, "{}", "▶️  Continue".bright_cyan()),
            Recommendation::Cancel => write!(f, "{}", "❌ Cancel".red()),
            Recommendation::Defer => write!(f, "{}", "⏸️  Defer".yellow()),
            Recommendation::Decompose => write!(f, "{}", "🔀 Decompose".magenta()),
            Recommendation::Review => write!(f, "{}", "👁️  Review".blue()),
        }
    }
}

pub fn display_advice(advice: &TaskAdvice, task: &Task, detailed: bool) -> Vec<SuggestedAction> {
    println!("\n{}", format!("📋 Task #{}: {}", task.id, task.title).bold());
    println!("{}", "─".repeat(60));
    
    // Show recommendation
    println!("\n{}: {}", "Recommendation".bold(), advice.primary_recommendation);
    
    // Show reasons
    println!("\n{}:", "Analysis".bold());
    for reason in &advice.reasons {
        println!("  • {}", reason);
    }
    
    // Show context if detailed
    if detailed {
        println!("\n{}:", "Context".bold());
        println!("  • Age: {} days", advice.context.age_days);
        println!("  • Time in {} status: {} days", task.status, advice.context.time_in_current_status);
        
        if !advice.context.blocking_tasks.is_empty() {
            println!("  • Blocked by tasks: {:?}", advice.context.blocking_tasks);
        }
        
        if !advice.context.dependent_tasks.is_empty() {
            println!("  • {} task(s) depend on this", advice.context.dependent_tasks.len());
        }
        
        if let Some((completed, total)) = advice.context.completion_percentage {
            println!("  • Subtask progress: {}/{} ({:.0}%)", 
                completed, total, (completed as f32 / total as f32) * 100.0);
        }
        
        if !advice.context.similar_completed_tasks.is_empty() {
            println!("  • Similar completed tasks:");
            for similar in &advice.context.similar_completed_tasks {
                println!("    - {}", similar);
            }
        }
    }
    
    // Show suggested actions
    if !advice.suggested_actions.is_empty() {
        println!("\n{}:", "Suggested Actions".bold());
        for (i, action) in advice.suggested_actions.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().yellow(), action.command.cyan());
            println!("     {}", action.description.italic());
        }
    }
    
    advice.suggested_actions.clone()
}

pub fn display_claude_advice(advice: &ClaudeTaskAdvice, task: &Task) -> Vec<(String, String, Option<String>)> {
    println!("\n{}", format!("🤖 Claude AI Analysis for Task #{}: {}", task.id, task.title).bold().bright_cyan());
    println!("{}", "═".repeat(70));
    
    // Show recommendation with color
    let colored_recommendation = match advice.recommendation.as_str() {
        "complete" => "✅ Complete".green(),
        "continue" => "▶️  Continue".bright_cyan(),
        "cancel" => "❌ Cancel".red(),
        "defer" => "⏸️  Defer".yellow(),
        "decompose" => "🔀 Decompose".magenta(),
        "review" => "👁️  Review".blue(),
        "reprioritize" => "🔄 Reprioritize".bright_yellow(),
        _ => advice.recommendation.normal(),
    };
    
    println!("\n{}: {}", "Recommendation".bold(), colored_recommendation);
    
    // Show reasoning
    if !advice.reasoning.is_empty() {
        println!("\n{}:", "AI Analysis".bold());
        for reason in &advice.reasoning {
            println!("  • {}", reason);
        }
    }
    
    // Show code context
    if !advice.code_context.is_empty() {
        println!("\n{}:", "Codebase Context".bold());
        for context in &advice.code_context {
            println!("  📁 {}", context.cyan());
        }
    }
    
    // Show similar completed tasks
    if !advice.similar_completed_tasks.is_empty() {
        println!("\n{}:", "Similar Completed Tasks".bold());
        for similar in &advice.similar_completed_tasks {
            println!("  ✓ {}", similar.green());
        }
    }
    
    // Show priority adjustment
    if let Some(priority_adj) = &advice.priority_adjustment {
        let (icon, color) = if priority_adj.starts_with("higher") {
            ("⬆️", "red")
        } else if priority_adj.starts_with("lower") {
            ("⬇️", "blue")
        } else {
            ("➡️", "green")
        };
        println!("\n{}: {} {}", "Priority Assessment".bold(), icon, priority_adj.color(color));
    }
    
    // Show complexity estimate
    if let Some(complexity) = &advice.estimated_complexity {
        let colored_complexity = match complexity.as_str() {
            "simple" => complexity.green(),
            "medium" => complexity.yellow(),
            "complex" => complexity.red(),
            _ => complexity.normal(),
        };
        println!("{}: {}", "Estimated Complexity".bold(), colored_complexity);
    }
    
    // Show suggested actions
    let mut actions = Vec::new();
    if !advice.suggested_actions.is_empty() {
        println!("\n{}:", "Suggested Actions".bold());
        for (i, action) in advice.suggested_actions.iter().enumerate() {
            println!("  {}. {}", (i + 1).to_string().yellow(), action.action.bold());
            println!("     {}", action.description.italic());
            if let Some(cmd) = &action.command {
                println!("     {}: {}", "Command".dimmed(), cmd.cyan());
            }
            actions.push((action.action.clone(), action.description.clone(), action.command.clone()));
        }
    }
    
    println!("\n{}", "─".repeat(70));
    println!("{}", "💡 This advice is based on AI analysis of your codebase and task history".italic().dimmed());
    
    actions
}