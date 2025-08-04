use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;

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