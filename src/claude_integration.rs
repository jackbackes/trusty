use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use crate::task::Task;

#[derive(Debug, Serialize, Deserialize)]
pub struct GeneratedTask {
    pub title: String,
    pub description: String,
    pub priority: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecomposedTask {
    pub subtasks: Vec<GeneratedTask>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeTaskAdvice {
    pub recommendation: String,
    pub reasoning: Vec<String>,
    pub code_context: Vec<String>,
    pub similar_completed_tasks: Vec<String>,
    pub suggested_actions: Vec<ClaudeAction>,
    pub priority_adjustment: Option<String>,
    pub estimated_complexity: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaudeAction {
    pub action: String,
    pub description: String,
    pub command: Option<String>,
}

pub fn generate_task_from_prompt(prompt: &str) -> Result<GeneratedTask> {
    // Try to find Claude CLI in common locations
    let claude_paths = vec![
        "/Users/jackbackes/.claude/local/claude",
        "claude",
    ];
    
    let mut claude_path = None;
    for path in &claude_paths {
        if Command::new(path).arg("--version").output().is_ok() {
            claude_path = Some(path.to_string());
            break;
        }
    }
    
    let claude_cmd = claude_path
        .ok_or_else(|| anyhow::anyhow!("Claude CLI not found. Please install it with: npm install -g @anthropic-ai/claude-code"))?;
    
    // Construct a prompt that asks Claude to generate a structured task
    let system_prompt = r#"You are a task generation assistant. Given a user's prompt about something they need to do, generate a structured task with the following JSON format:
{
  "title": "Brief, actionable task title",
  "description": "Detailed description of what needs to be done",
  "priority": "high|medium|low",
  "tags": ["tag1", "tag2", "tag3"]
}

Rules:
- Title should be concise and action-oriented (5-10 words)
- Description should provide context and details
- Priority: "high" for urgent/critical, "medium" for normal, "low" for nice-to-have
- Tags should be relevant categories (e.g., "backend", "frontend", "testing", "documentation", "refactoring", "bugfix", "feature")
- Output ONLY valid JSON, no additional text"#;
    
    let full_prompt = format!("{}\n\nUser prompt: {}", system_prompt, prompt);
    
    // Call Claude CLI
    let output = Command::new(&claude_cmd)
        .arg("--model")
        .arg("sonnet")
        .arg("-p")
        .arg("--output-format")
        .arg("text")
        .arg(&full_prompt)
        .output()
        .context("Failed to execute Claude CLI")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }
    
    let response = String::from_utf8(output.stdout)
        .context("Failed to parse Claude output as UTF-8")?;
    
    // Extract JSON from markdown code blocks if present
    let json_str = if response.contains("```json") {
        let start = response.find("```json").unwrap() + 7;
        let end = response.rfind("```").unwrap();
        response[start..end].trim()
    } else {
        response.trim()
    };
    
    // Parse the JSON response
    let task: GeneratedTask = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse Claude's response as JSON. Response was: {}", json_str))?;
    
    Ok(task)
}

pub fn decompose_task(task_title: &str, task_description: &str, task_priority: &str, task_tags: &[String], count: u32) -> Result<DecomposedTask> {
    // Try to find Claude CLI in common locations
    let claude_paths = vec![
        "/Users/jackbackes/.claude/local/claude",
        "claude",
    ];
    
    let mut claude_path = None;
    for path in &claude_paths {
        if Command::new(path).arg("--version").output().is_ok() {
            claude_path = Some(path.to_string());
            break;
        }
    }
    
    let claude_cmd = claude_path
        .ok_or_else(|| anyhow::anyhow!("Claude CLI not found. Please install it with: npm install -g @anthropic-ai/claude-code"))?;
    
    // Construct a prompt that asks Claude to decompose the task
    let system_prompt = format!(r#"You are a task decomposition assistant. Given a parent task, break it down into {} logical subtasks that, when completed, will accomplish the parent task.

Parent task details:
- Title: {}
- Description: {}
- Priority: {}
- Tags: {}

Generate a JSON response with the following format:
{{
  "subtasks": [
    {{
      "title": "Brief, actionable subtask title",
      "description": "Detailed description of what needs to be done",
      "priority": "high|medium|low",
      "tags": ["tag1", "tag2"]
    }},
    ...
  ]
}}

Rules:
- Each subtask should be a concrete, actionable step
- Subtasks should be logically ordered when possible
- Subtask priorities can be the same as parent or adjusted based on importance
- Tags should include relevant parent tags plus any subtask-specific ones
- Ensure subtasks cover all aspects of the parent task
- Output ONLY valid JSON, no additional text"#, 
        count, task_title, task_description, task_priority, task_tags.join(", "));
    
    // Call Claude CLI
    let output = Command::new(&claude_cmd)
        .arg("--model")
        .arg("sonnet")
        .arg("-p")
        .arg("--output-format")
        .arg("text")
        .arg(&system_prompt)
        .output()
        .context("Failed to execute Claude CLI")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }
    
    let response = String::from_utf8(output.stdout)
        .context("Failed to parse Claude output as UTF-8")?;
    
    // Extract JSON from markdown code blocks if present
    let json_str = if response.contains("```json") {
        let start = response.find("```json").unwrap() + 7;
        let end = response.rfind("```").unwrap();
        response[start..end].trim()
    } else {
        response.trim()
    };
    
    // Parse the JSON response
    let decomposed: DecomposedTask = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse Claude's response as JSON. Response was: {}", json_str))?;
    
    Ok(decomposed)
}

pub fn get_task_advice(task: &Task, all_tasks: &[Task]) -> Result<ClaudeTaskAdvice> {
    // Try to find Claude CLI in common locations
    let claude_paths = vec![
        "/Users/jackbackes/.claude/local/claude",
        "claude",
    ];
    
    let mut claude_path = None;
    for path in &claude_paths {
        if Command::new(path).arg("--version").output().is_ok() {
            claude_path = Some(path.to_string());
            break;
        }
    }
    
    let claude_cmd = claude_path
        .ok_or_else(|| anyhow::anyhow!("Claude CLI not found. Please install it with: npm install -g @anthropic-ai/claude-code"))?;
    
    // Get current directory for codebase context
    let current_dir = std::env::current_dir()?;
    
    // Separate tasks into those completed/modified after this task was created
    let tasks_after_creation: Vec<&Task> = all_tasks.iter()
        .filter(|t| {
            // Include if completed after this task was created
            (t.status == crate::task::TaskStatus::Done && 
             t.completed_at.map(|completed| completed > task.created_at).unwrap_or(false)) ||
            // Or if updated after this task was created (excluding the task itself)
            (t.id != task.id && t.updated_at > task.created_at)
        })
        .collect();
    
    // Prepare summaries
    let completed_after: Vec<String> = tasks_after_creation.iter()
        .filter(|t| t.status == crate::task::TaskStatus::Done)
        .map(|t| format!("- {} (Completed: {}, Priority: {}, Tags: {})", 
            t.title, 
            t.completed_at.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            t.priority, 
            t.tags.join(", ")))
        .collect();
    
    let modified_after: Vec<String> = tasks_after_creation.iter()
        .filter(|t| t.status != crate::task::TaskStatus::Done)
        .map(|t| format!("- {} (Status: {:?}, Updated: {}, Priority: {})", 
            t.title, 
            t.status,
            t.updated_at.format("%Y-%m-%d"),
            t.priority))
        .collect();
    
    // Construct a prompt that asks Claude to analyze the task
    let system_prompt = format!(r#"You are an expert task advisor integrated with a development workflow. Analyze the given task in the context of the codebase and tasks that have been completed or modified since this task was created.

Current task details:
- ID: {}
- Title: {}
- Description: {}
- Status: {:?}
- Priority: {}
- Tags: {}
- Created: {} (on {})
- Dependencies: {:?}
- Subtasks: {} subtasks

Tasks completed AFTER this task was created:
{}

Tasks modified AFTER this task was created:
{}

Current working directory: {}

Analyze this task considering:
1. The codebase structure and existing code
2. Tasks completed or modified since this was created - they may have:
   - Already addressed this task's needs
   - Changed the project context making this task obsolete
   - Shifted priorities or technical direction
3. Whether the task is still relevant given what has happened since
4. Task age and current status
5. Dependencies and blockers

Provide advice in the following JSON format:
{{
  "recommendation": "complete|continue|cancel|defer|decompose|review|reprioritize",
  "reasoning": [
    "First reason based on codebase analysis",
    "Second reason based on completed tasks",
    "Additional contextual reasons"
  ],
  "code_context": [
    "Relevant files or modules in the codebase",
    "Related code patterns or implementations"
  ],
  "similar_completed_tasks": [
    "Title of similar completed task and what we can learn from it"
  ],
  "suggested_actions": [
    {{
      "action": "Brief action title",
      "description": "Detailed description of what to do",
      "command": "Optional trusty command to execute"
    }}
  ],
  "priority_adjustment": "higher|lower|same with reasoning",
  "estimated_complexity": "simple|medium|complex based on codebase analysis"
}}

Output ONLY valid JSON, no additional text."#,
        task.id,
        task.title,
        task.description,
        task.status,
        task.priority,
        task.tags.join(", "),
        (chrono::Utc::now() - task.created_at).num_days(),
        task.created_at.format("%Y-%m-%d"),
        task.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(", "),
        task.subtasks.len(),
        if completed_after.is_empty() { "None".to_string() } else { completed_after.join("\n") },
        if modified_after.is_empty() { "None".to_string() } else { modified_after.join("\n") },
        current_dir.display()
    );
    
    // Call Claude CLI
    let output = Command::new(&claude_cmd)
        .arg("--model")
        .arg("opus")  // Use opus for more complex analysis
        .arg("-p")
        .arg("--output-format")
        .arg("text")
        .arg(&system_prompt)
        .output()
        .context("Failed to execute Claude CLI")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }
    
    let response = String::from_utf8(output.stdout)
        .context("Failed to parse Claude output as UTF-8")?;
    
    // Extract JSON from markdown code blocks if present
    let json_str = if response.contains("```json") {
        let start = response.find("```json").unwrap() + 7;
        let end = response.rfind("```").unwrap();
        response[start..end].trim()
    } else {
        response.trim()
    };
    
    // Parse the JSON response
    let advice: ClaudeTaskAdvice = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse Claude's response as JSON. Response was: {}", json_str))?;
    
    Ok(advice)
}