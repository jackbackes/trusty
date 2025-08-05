use colored::*;
use comfy_table::{Cell, Color, ContentArrangement, Table};
use std::collections::{HashMap, HashSet};

use crate::task::{Task, TaskStatus};

pub struct TaskDisplay;

impl TaskDisplay {
    pub fn display_task_list(tasks: &[Task], project_path: &str) {
        println!("Listing tasks from: {}", project_path.cyan());
        
        let stats = Self::calculate_stats(tasks);
        
        Self::display_dashboard(&stats);
        Self::display_dependency_info(tasks, &stats);
        Self::display_table(tasks);
        
        if let Some(next_task) = Self::get_next_recommended_task(tasks) {
            Self::display_recommended_task(next_task);
        }
    }
    
    pub fn get_next_task(tasks: &[Task]) -> Option<&Task> {
        Self::get_next_recommended_task(tasks)
    }

    fn calculate_stats(tasks: &[Task]) -> TaskStats {
        let mut stats = TaskStats::default();
        
        for task in tasks {
            match task.status {
                TaskStatus::Done => stats.done += 1,
                TaskStatus::InProgress => stats.in_progress += 1,
                TaskStatus::Pending => stats.pending += 1,
                TaskStatus::Blocked => stats.blocked += 1,
                TaskStatus::Deferred => stats.deferred += 1,
                TaskStatus::Cancelled => stats.cancelled += 1,
            }
            
            match task.priority {
                crate::task::Priority::High => stats.high_priority += 1,
                crate::task::Priority::Medium => stats.medium_priority += 1,
                crate::task::Priority::Low => stats.low_priority += 1,
            }
            
            stats.total_deps += task.dependencies.len();
            
            if task.dependencies.is_empty() {
                stats.no_deps += 1;
            }
            
            for &dep in &task.dependencies {
                *stats.dep_count.entry(dep).or_insert(0) += 1;
            }
        }
        
        stats.total = tasks.len();
        
        if stats.total > 0 {
            stats.completion_percent = (stats.done as f32 / stats.total as f32) * 100.0;
            stats.avg_deps = stats.total_deps as f32 / stats.total as f32;
        }
        
        // Find tasks ready to work on
        let completed: HashSet<_> = tasks.iter()
            .filter(|t| t.status == TaskStatus::Done)
            .map(|t| t.id)
            .collect();
        
        stats.ready_tasks = tasks.iter()
            .filter(|t| t.is_ready(&completed))
            .count();
        
        stats.blocked_by_deps = tasks.iter()
            .filter(|t| t.status == TaskStatus::Pending && !t.dependencies.is_empty() && !t.is_ready(&completed))
            .count();
        
        stats
    }

    fn display_dashboard(stats: &TaskStats) {
        let progress_bar = Self::create_progress_bar(stats.completion_percent);
        
        let dashboard = format!(
            "╭──────────────────────────────────────────────────────╮
│                                                      │
│   Project Dashboard                                  │
│   Tasks Progress: {}     │
│   {:.0}% {:.0}%                                              │
│   Done: {}  In Progress: {}  Pending: {}  Blocked: {}   │
│   Deferred: {}  Cancelled: {}                          │
│                                                      │
│   Subtasks Progress:                                 │
│   {} {:.0}% {:.0}%               │
│   Completed: {}/{}  In Progress: {}  Pending: {}         │
│   Blocked: {}  Deferred: {}  Cancelled: {}              │
│                                                      │
│   Priority Breakdown:                                │
│   • High priority: {}                                 │
│   • Medium priority: {}                               │
│   • Low priority: {}                                  │
│                                                      │
╰──────────────────────────────────────────────────────╯",
            progress_bar,
            stats.completion_percent, stats.completion_percent,
            stats.done, stats.in_progress, stats.pending, stats.blocked,
            stats.deferred, stats.cancelled,
            Self::create_progress_bar(0.0), 0.0, 0.0,
            0, 0, 0, 0,
            0, 0, 0,
            stats.high_priority,
            stats.medium_priority,
            stats.low_priority
        );
        
        println!("{}", dashboard);
    }

    fn display_dependency_info(tasks: &[Task], stats: &TaskStats) {
        let most_depended = stats.dep_count.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(id, _)| id);
        
        let next_task = Self::get_next_recommended_task(tasks);
        
        let dep_info = format!(
            "╭──────────────────────────────────────────────────────╮
│                                                      │
│   Dependency Status & Next Task                      │
│   Dependency Metrics:                                │
│   • Tasks with no dependencies: {}                    │
│   • Tasks ready to work on: {}                        │
│   • Tasks blocked by dependencies: {}                 │
│   • Most depended-on task: {}                      │
│   • Avg dependencies per task: {:.1}                   │
│                                                      │
│   Next Task to Work On:                              │
│   {}   │
│   {}  {}                 │
│   {}                                    │
│                                                      │
╰──────────────────────────────────────────────────────╯",
            stats.no_deps,
            stats.ready_tasks,
            stats.blocked_by_deps,
            most_depended.map(|id| format!("#{}", id)).unwrap_or_else(|| "None".to_string()),
            stats.avg_deps,
            next_task.map(|t| format!("ID: {} - {}", t.id, Self::truncate(&t.title, 35)))
                .unwrap_or_else(|| "No tasks available".to_string()),
            next_task.map(|t| format!("Priority: {}", t.priority))
                .unwrap_or_else(|| "".to_string()),
            next_task.map(|t| format!("Dependencies: {}", 
                if t.dependencies.is_empty() { "None".to_string() } 
                else { format!("{:?}", t.dependencies.iter().collect::<Vec<_>>()) }
            )).unwrap_or_else(|| "".to_string()),
            next_task.map(|t| format!("Complexity: {}", 
                t.complexity.as_ref().map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())
            )).unwrap_or_else(|| "".to_string())
        );
        
        println!("{}", dep_info);
    }

    fn display_table(tasks: &[Task]) {
        let mut table = Table::new();
        table
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("ID"),
                Cell::new("Title"),
                Cell::new("Status"),
                Cell::new("Priority"),
                Cell::new("Dependencies"),
                Cell::new("Complexity"),
            ]);

        for task in tasks {
            // Compute effective status considering subtasks
            let effective_status = task.compute_effective_status(tasks);
            let status_display = if !task.subtasks.is_empty() && effective_status != task.status {
                let (completed, total) = task.subtask_progress(tasks);
                format!("{} ({}/{} done)", effective_status, completed, total)
            } else {
                task.status.to_string()
            };
            
            let status_cell = match effective_status {
                TaskStatus::Done => Cell::new(status_display).fg(Color::Green),
                TaskStatus::InProgress => Cell::new(status_display).fg(Color::Yellow),
                TaskStatus::Blocked => Cell::new(status_display).fg(Color::Red),
                TaskStatus::Cancelled => Cell::new(status_display).fg(Color::DarkGrey),
                _ => Cell::new(status_display),
            };

            let priority_cell = match task.priority {
                crate::task::Priority::High => Cell::new(task.priority.to_string()).fg(Color::Red),
                crate::task::Priority::Medium => Cell::new(task.priority.to_string()).fg(Color::Yellow),
                crate::task::Priority::Low => Cell::new(task.priority.to_string()).fg(Color::Blue),
            };

            let deps_display = if task.dependencies.is_empty() {
                "None".to_string()
            } else {
                task.dependencies.iter()
                    .map(|id| {
                        if tasks.iter().any(|t| t.id == *id) {
                            id.to_string()
                        } else {
                            format!("{} (Not found)", id)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            };

            let complexity_display = task.complexity
                .as_ref()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "N/A".to_string());

            table.add_row(vec![
                Cell::new(task.id),
                Cell::new(Self::truncate(&task.title, 35)),
                status_cell,
                priority_cell,
                Cell::new(deps_display),
                Cell::new(complexity_display),
            ]);
        }

        println!("{}", table);
    }

    fn display_recommended_task(task: &Task) {
        let recommendation = format!(
            "
╭────────────────────────────────────────── ⚡ RECOMMENDED NEXT TASK ⚡ ───────────────────────────────────────────╮
│                                                                                                                  │
│  🔥 Next Task to Work On: #{} - {}                                                      │
│                                                                                                                  │
│  Priority: {}   Status: {}                                                                              │
│  Dependencies: {}                                                                                              │
│                                                                                                                  │
│  Description: {}                  │
│                                                                                                                  │
│  Start working: trusty set-status --id={} --status=in-progress                                               │
│  View details: trusty show {}                                                                                │
│  Get AI advice: trusty task advice --id={} --ask-claude                                                       │
│                                                                                                                  │
╰──────────────────────────────────────────────────────────────────────────────────────────────────────────────────╯",
            task.id,
            task.title,
            task.priority,
            task.status,
            if task.dependencies.is_empty() { "None".to_string() } else { format!("{:?}", task.dependencies.iter().collect::<Vec<_>>()) },
            Self::truncate(&task.description, 70),
            task.id,
            task.id,
            task.id
        );

        println!("{}", recommendation.bright_yellow());
    }

    fn get_next_recommended_task(tasks: &[Task]) -> Option<&Task> {
        let completed: HashSet<_> = tasks.iter()
            .filter(|t| t.status == TaskStatus::Done)
            .map(|t| t.id)
            .collect();

        tasks.iter()
            .filter(|t| t.is_ready(&completed))
            .min_by_key(|t| {
                (
                    match t.priority {
                        crate::task::Priority::High => 0,
                        crate::task::Priority::Medium => 1,
                        crate::task::Priority::Low => 2,
                    },
                    t.id,
                )
            })
    }

    fn create_progress_bar(percent: f32) -> String {
        let filled = (percent / 100.0 * 30.0) as usize;
        let empty = 30 - filled;
        format!("{}{}", "░".repeat(filled), "░".repeat(empty))
    }

    fn truncate(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len - 3])
        }
    }
}

#[derive(Default)]
struct TaskStats {
    total: usize,
    done: usize,
    in_progress: usize,
    pending: usize,
    blocked: usize,
    deferred: usize,
    cancelled: usize,
    high_priority: usize,
    medium_priority: usize,
    low_priority: usize,
    completion_percent: f32,
    no_deps: usize,
    ready_tasks: usize,
    blocked_by_deps: usize,
    total_deps: usize,
    avg_deps: f32,
    dep_count: HashMap<u32, usize>,
}