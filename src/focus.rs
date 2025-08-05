use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;
use crate::task::{Task, Priority};
use crate::storage::TaskStorage;
use crate::claude_integration;

#[derive(Debug, Clone, PartialEq)]
pub enum ValueCategory {
    Strategic,     // High business/user value, aligned with goals
    Tactical,      // Important but not strategic
    Maintenance,   // Necessary but not value-adding
    Speculative,   // Nice-to-have or exploratory
}

#[derive(Debug)]
pub struct FocusAssessment {
    pub task: Task,
    pub value_category: ValueCategory,
    pub value_score: f32,           // 0.0 to 1.0
    pub effort_score: f32,          // 0.0 to 1.0
    pub impact_score: f32,          // 0.0 to 1.0
    pub alignment_score: f32,       // 0.0 to 1.0 (with PRD if provided)
    pub blocking_value: f32,        // How much value this unblocks
    pub recommendation: String,
    pub reasoning: Vec<String>,
}

pub struct FocusAnalyzer {
    storage: TaskStorage,
    prd_content: Option<String>,
}

#[derive(Debug)]
pub struct FocusReport {
    pub assessments: Vec<FocusAssessment>,
    pub strategic_tasks: Vec<u32>,
    pub tactical_tasks: Vec<u32>,
    pub maintenance_tasks: Vec<u32>,
    pub speculative_tasks: Vec<u32>,
    pub reprioritization_suggestions: Vec<ReprioritizationSuggestion>,
    pub focus_recommendations: Vec<String>,
}

#[derive(Debug)]
pub struct ReprioritizationSuggestion {
    pub task_id: u32,
    pub current_priority: Priority,
    pub suggested_priority: Priority,
    pub reason: String,
}

impl FocusAnalyzer {
    pub fn new(storage: TaskStorage) -> Self {
        Self {
            storage,
            prd_content: None,
        }
    }

    pub fn load_prd(&mut self, prd_path: &PathBuf) -> Result<()> {
        self.prd_content = Some(fs::read_to_string(prd_path)?);
        Ok(())
    }

    pub fn analyze_all_tasks(&self) -> Result<FocusReport> {
        let all_tasks = self.storage.list_all_tasks()?;
        let pending_tasks: Vec<Task> = all_tasks.into_iter()
            .filter(|t| matches!(t.status, crate::task::TaskStatus::Pending | crate::task::TaskStatus::InProgress))
            .collect();

        if pending_tasks.is_empty() {
            return Ok(FocusReport {
                assessments: vec![],
                strategic_tasks: vec![],
                tactical_tasks: vec![],
                maintenance_tasks: vec![],
                speculative_tasks: vec![],
                reprioritization_suggestions: vec![],
                focus_recommendations: vec!["No pending tasks to analyze.".to_string()],
            });
        }

        // Use Claude to analyze tasks if available
        let ai_analysis = if self.prd_content.is_some() || pending_tasks.len() > 3 {
            self.get_ai_value_analysis(&pending_tasks)?
        } else {
            None
        };

        // Assess each task
        let mut assessments = Vec::new();
        for task in pending_tasks {
            let assessment = if let Some(ref ai) = ai_analysis {
                self.assess_task_with_ai(&task, ai)
            } else {
                self.assess_task_locally(&task)
            };
            assessments.push(assessment);
        }

        // Generate report
        self.generate_report(assessments)
    }

    fn assess_task_locally(&self, task: &Task) -> FocusAssessment {
        let mut reasoning = Vec::new();
        
        // Determine value category based on heuristics
        let value_category = self.categorize_task_value(task, &mut reasoning);
        
        // Calculate scores
        let value_score = self.calculate_value_score(task, &value_category);
        let effort_score = self.calculate_effort_score(task);
        let impact_score = self.calculate_impact_score(task);
        let blocking_value = self.calculate_blocking_value(task);
        
        // Generate recommendation
        let recommendation = self.generate_recommendation(&value_category, value_score, effort_score);
        
        FocusAssessment {
            task: task.clone(),
            value_category,
            value_score,
            effort_score,
            impact_score,
            alignment_score: 0.5, // Default without PRD
            blocking_value,
            recommendation,
            reasoning,
        }
    }

    fn categorize_task_value(&self, task: &Task, reasoning: &mut Vec<String>) -> ValueCategory {
        // Keywords that suggest strategic importance
        let strategic_keywords = ["user", "customer", "revenue", "core", "critical", "launch", "mvp", "release"];
        let maintenance_keywords = ["fix", "bug", "refactor", "cleanup", "technical debt", "update dependencies"];
        let speculative_keywords = ["experiment", "explore", "investigate", "research", "prototype", "poc"];
        
        let title_lower = task.title.to_lowercase();
        let desc_lower = task.description.to_lowercase();
        let combined = format!("{} {}", title_lower, desc_lower);
        
        // Check for strategic keywords
        if strategic_keywords.iter().any(|&kw| combined.contains(kw)) {
            reasoning.push("Contains strategic keywords suggesting user/business value".to_string());
            return ValueCategory::Strategic;
        }
        
        // High priority tasks are often strategic
        if task.priority == Priority::High && !maintenance_keywords.iter().any(|&kw| combined.contains(kw)) {
            reasoning.push("High priority task without maintenance indicators".to_string());
            return ValueCategory::Strategic;
        }
        
        // Check for maintenance work
        if maintenance_keywords.iter().any(|&kw| combined.contains(kw)) {
            reasoning.push("Appears to be maintenance or technical debt work".to_string());
            return ValueCategory::Maintenance;
        }
        
        // Check for speculative work
        if speculative_keywords.iter().any(|&kw| combined.contains(kw)) {
            reasoning.push("Appears to be exploratory or speculative work".to_string());
            return ValueCategory::Speculative;
        }
        
        // Default to tactical
        reasoning.push("Standard implementation task".to_string());
        ValueCategory::Tactical
    }

    fn calculate_value_score(&self, task: &Task, category: &ValueCategory) -> f32 {
        let base_score = match category {
            ValueCategory::Strategic => 0.8,
            ValueCategory::Tactical => 0.6,
            ValueCategory::Maintenance => 0.4,
            ValueCategory::Speculative => 0.3,
        };
        
        // Adjust based on priority
        let priority_modifier = match task.priority {
            Priority::High => 0.2,
            Priority::Medium => 0.1,
            Priority::Low => 0.0,
        };
        
        (base_score + priority_modifier).min(1.0)
    }

    fn calculate_effort_score(&self, task: &Task) -> f32 {
        let complexity_score = match task.complexity.as_ref() {
            Some(crate::task::Complexity::Simple) => 0.2,
            Some(crate::task::Complexity::Medium) => 0.5,
            Some(crate::task::Complexity::Complex) => 0.8,
            None => 0.5, // Default to medium
        };
        
        // More subtasks = more effort
        let subtask_modifier = (task.subtasks.len() as f32 * 0.1).min(0.2);
        
        (complexity_score + subtask_modifier).min(1.0)
    }

    fn calculate_impact_score(&self, task: &Task) -> f32 {
        // Tasks with many dependents have high impact
        let all_tasks = self.storage.list_all_tasks().unwrap_or_default();
        let dependent_count = all_tasks.iter()
            .filter(|t| t.dependencies.contains(&task.id))
            .count();
        
        let dependency_score = (dependent_count as f32 * 0.2).min(0.6);
        let value_modifier = match self.categorize_task_value(task, &mut vec![]) {
            ValueCategory::Strategic => 0.4,
            ValueCategory::Tactical => 0.2,
            _ => 0.1,
        };
        
        dependency_score + value_modifier
    }

    fn calculate_blocking_value(&self, task: &Task) -> f32 {
        let all_tasks = self.storage.list_all_tasks().unwrap_or_default();
        let blocked_tasks: Vec<&Task> = all_tasks.iter()
            .filter(|t| t.dependencies.contains(&task.id))
            .collect();
        
        // Sum up the value of blocked tasks
        blocked_tasks.iter()
            .map(|t| self.calculate_value_score(t, &self.categorize_task_value(t, &mut vec![])))
            .sum::<f32>()
            .min(1.0)
    }

    fn generate_recommendation(&self, category: &ValueCategory, value_score: f32, effort_score: f32) -> String {
        let value_effort_ratio = value_score / (effort_score + 0.1); // Avoid division by zero
        
        match (category, value_effort_ratio) {
            (ValueCategory::Strategic, ratio) if ratio > 1.5 => {
                "High-value quick win - prioritize immediately".to_string()
            }
            (ValueCategory::Strategic, _) => {
                "Strategic importance - maintain high priority".to_string()
            }
            (ValueCategory::Tactical, ratio) if ratio > 1.0 => {
                "Good value/effort ratio - schedule soon".to_string()
            }
            (ValueCategory::Maintenance, _) => {
                "Necessary but not urgent - batch with similar work".to_string()
            }
            (ValueCategory::Speculative, _) => {
                "Consider deferring until core work complete".to_string()
            }
            _ => "Standard priority".to_string()
        }
    }

    fn get_ai_value_analysis(&self, tasks: &[Task]) -> Result<Option<HashMap<u32, AIValueInsight>>> {
        // This would call Claude to analyze all tasks for value
        // For now, returning None to use local analysis
        Ok(None)
    }

    fn assess_task_with_ai(&self, task: &Task, ai_insights: &HashMap<u32, AIValueInsight>) -> FocusAssessment {
        // Would combine AI insights with local analysis
        // For now, falling back to local assessment
        self.assess_task_locally(task)
    }

    fn generate_report(&self, mut assessments: Vec<FocusAssessment>) -> Result<FocusReport> {
        // Sort by value score descending
        assessments.sort_by(|a, b| b.value_score.partial_cmp(&a.value_score).unwrap());
        
        // Categorize tasks
        let mut strategic_tasks = Vec::new();
        let mut tactical_tasks = Vec::new();
        let mut maintenance_tasks = Vec::new();
        let mut speculative_tasks = Vec::new();
        
        for assessment in &assessments {
            match assessment.value_category {
                ValueCategory::Strategic => strategic_tasks.push(assessment.task.id),
                ValueCategory::Tactical => tactical_tasks.push(assessment.task.id),
                ValueCategory::Maintenance => maintenance_tasks.push(assessment.task.id),
                ValueCategory::Speculative => speculative_tasks.push(assessment.task.id),
            }
        }
        
        // Generate reprioritization suggestions
        let mut reprioritization_suggestions = Vec::new();
        for assessment in &assessments {
            let suggested_priority = self.suggest_priority(&assessment);
            if suggested_priority != assessment.task.priority {
                reprioritization_suggestions.push(ReprioritizationSuggestion {
                    task_id: assessment.task.id,
                    current_priority: assessment.task.priority.clone(),
                    suggested_priority,
                    reason: assessment.recommendation.clone(),
                });
            }
        }
        
        // Generate focus recommendations
        let focus_recommendations = self.generate_focus_recommendations(&assessments);
        
        Ok(FocusReport {
            assessments,
            strategic_tasks,
            tactical_tasks,
            maintenance_tasks,
            speculative_tasks,
            reprioritization_suggestions,
            focus_recommendations,
        })
    }

    fn suggest_priority(&self, assessment: &FocusAssessment) -> Priority {
        match (assessment.value_category.clone(), assessment.value_score) {
            (ValueCategory::Strategic, score) if score > 0.7 => Priority::High,
            (ValueCategory::Strategic, _) => Priority::Medium,
            (ValueCategory::Tactical, score) if score > 0.8 => Priority::High,
            (ValueCategory::Tactical, score) if score > 0.5 => Priority::Medium,
            (ValueCategory::Maintenance, _) => Priority::Low,
            (ValueCategory::Speculative, _) => Priority::Low,
            _ => Priority::Medium,
        }
    }

    fn generate_focus_recommendations(&self, assessments: &[FocusAssessment]) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Find high-value quick wins
        let quick_wins: Vec<&FocusAssessment> = assessments.iter()
            .filter(|a| a.value_score > 0.7 && a.effort_score < 0.4)
            .take(3)
            .collect();
        
        if !quick_wins.is_empty() {
            recommendations.push("🎯 Quick Wins Available:".to_string());
            for win in quick_wins {
                recommendations.push(format!("  • #{} - {} (High value, low effort)", 
                    win.task.id, win.task.title));
            }
        }
        
        // Find high blocking value tasks
        let blockers: Vec<&FocusAssessment> = assessments.iter()
            .filter(|a| a.blocking_value > 0.5)
            .take(3)
            .collect();
        
        if !blockers.is_empty() {
            recommendations.push("🔓 Unblock Progress:".to_string());
            for blocker in blockers {
                recommendations.push(format!("  • #{} - {} (Blocks {:.0} value points)", 
                    blocker.task.id, blocker.task.title, blocker.blocking_value * 100.0));
            }
        }
        
        // Strategic focus
        let strategic_count = assessments.iter()
            .filter(|a| matches!(a.value_category, ValueCategory::Strategic))
            .count();
        
        if strategic_count > 0 {
            recommendations.push(format!("💡 You have {} strategic tasks - focus on these for maximum impact", strategic_count));
        }
        
        recommendations
    }
}

// Placeholder for AI insights structure
struct AIValueInsight {
    task_id: u32,
    business_value: f32,
    user_impact: f32,
    strategic_alignment: f32,
    technical_debt_reduction: f32,
}

impl std::fmt::Display for ValueCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueCategory::Strategic => write!(f, "{}", "Strategic".bright_green()),
            ValueCategory::Tactical => write!(f, "{}", "Tactical".yellow()),
            ValueCategory::Maintenance => write!(f, "{}", "Maintenance".blue()),
            ValueCategory::Speculative => write!(f, "{}", "Speculative".magenta()),
        }
    }
}

pub fn display_focus_report(report: &FocusReport) {
    println!("\n{}", "🎯 Value Focus Analysis".bold().bright_cyan());
    println!("{}", "═".repeat(70));
    
    // Summary
    println!("\n{}", "Task Value Distribution:".bold());
    println!("  {} Strategic:    {} tasks", "●".bright_green(), report.strategic_tasks.len());
    println!("  {} Tactical:     {} tasks", "●".yellow(), report.tactical_tasks.len());
    println!("  {} Maintenance:  {} tasks", "●".blue(), report.maintenance_tasks.len());
    println!("  {} Speculative:  {} tasks", "●".magenta(), report.speculative_tasks.len());
    
    // Focus recommendations
    if !report.focus_recommendations.is_empty() {
        println!("\n{}", "Recommended Focus Areas:".bold());
        for rec in &report.focus_recommendations {
            println!("{}", rec);
        }
    }
    
    // Reprioritization suggestions
    if !report.reprioritization_suggestions.is_empty() {
        println!("\n{}", "Suggested Priority Changes:".bold());
        for suggestion in &report.reprioritization_suggestions {
            println!("  Task #{}: {} → {}", 
                suggestion.task_id,
                suggestion.current_priority,
                match suggestion.suggested_priority {
                    Priority::High => "high".red(),
                    Priority::Medium => "medium".yellow(),
                    Priority::Low => "low".blue(),
                }
            );
            println!("    {}", suggestion.reason.italic());
        }
    }
    
    // Top value tasks
    println!("\n{}", "High-Value Tasks (sorted by value):".bold());
    let top_tasks: Vec<&FocusAssessment> = report.assessments.iter()
        .filter(|a| a.value_score > 0.6)
        .take(5)
        .collect();
    
    for assessment in top_tasks {
        println!("\n  #{} - {} [{}]", 
            assessment.task.id, 
            assessment.task.title.cyan(),
            assessment.value_category
        );
        println!("    Value: {:.0}% | Effort: {:.0}% | Impact: {:.0}%",
            assessment.value_score * 100.0,
            assessment.effort_score * 100.0,
            assessment.impact_score * 100.0
        );
        println!("    {}", assessment.recommendation.italic());
    }
    
    println!("\n{}", "─".repeat(70));
    println!("{}", "💡 Use 'trusty task advice --id=X' for detailed analysis of any task".italic().dimmed());
}