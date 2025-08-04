use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

use crate::task::Task;

pub struct TaskStorage {
    base_path: PathBuf,
}

impl TaskStorage {
    pub fn new(base_path: PathBuf) -> Result<Self> {
        fs::create_dir_all(&base_path)
            .with_context(|| format!("Failed to create task directory: {:?}", base_path))?;
        
        Ok(Self { base_path })
    }

    pub fn get_task_path(&self, id: u32) -> PathBuf {
        self.base_path.join(format!("{}.md", id))
    }

    pub fn save_task(&self, task: &Task) -> Result<()> {
        let content = self.task_to_markdown(task);
        let path = self.get_task_path(task.id);
        
        fs::write(&path, content)
            .with_context(|| format!("Failed to write task file: {:?}", path))?;
        
        Ok(())
    }

    pub fn load_task(&self, id: u32) -> Result<Task> {
        let path = self.get_task_path(id);
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read task file: {:?}", path))?;
        
        self.markdown_to_task(&content)
    }

    pub fn delete_task(&self, id: u32) -> Result<()> {
        let path = self.get_task_path(id);
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete task file: {:?}", path))?;
        
        Ok(())
    }

    pub fn list_all_tasks(&self) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        
        for entry in fs::read_dir(&self.base_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Ok(id) = stem.parse::<u32>() {
                        if let Ok(task) = self.load_task(id) {
                            tasks.push(task);
                        }
                    }
                }
            }
        }
        
        tasks.sort_by_key(|t| t.id);
        Ok(tasks)
    }

    fn task_to_markdown(&self, task: &Task) -> String {
        let mut content = String::new();
        
        // Frontmatter
        content.push_str("---\n");
        content.push_str(&format!("id: {}\n", task.id));
        content.push_str(&format!("title: \"{}\"\n", task.title.replace("\"", "\\\"")));
        content.push_str(&format!("status: {}\n", serde_json::to_string(&task.status).unwrap().trim_matches('"')));
        content.push_str(&format!("priority: {}\n", serde_json::to_string(&task.priority).unwrap().trim_matches('"')));
        
        if let Some(complexity) = &task.complexity {
            content.push_str(&format!("complexity: {}\n", serde_json::to_string(complexity).unwrap().trim_matches('"')));
        }
        
        if !task.dependencies.is_empty() {
            let deps: Vec<_> = task.dependencies.iter().collect();
            content.push_str(&format!("dependencies: {:?}\n", deps));
        }
        
        if !task.subtasks.is_empty() {
            content.push_str(&format!("subtasks: {:?}\n", task.subtasks));
        }
        
        if !task.tags.is_empty() {
            content.push_str(&format!("tags: {:?}\n", task.tags));
        }
        
        content.push_str(&format!("created_at: {}\n", task.created_at.to_rfc3339()));
        content.push_str(&format!("updated_at: {}\n", task.updated_at.to_rfc3339()));
        
        if let Some(completed_at) = task.completed_at {
            content.push_str(&format!("completed_at: {}\n", completed_at.to_rfc3339()));
        }
        
        content.push_str("---\n\n");
        
        // Main content
        content.push_str(&format!("# {}\n\n", task.title));
        content.push_str(&task.description);
        
        content
    }

    fn markdown_to_task(&self, content: &str) -> Result<Task> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            anyhow::bail!("Invalid markdown format: missing frontmatter");
        }
        
        let frontmatter = parts[1].trim();
        let body = parts[2].trim();
        
        // Parse frontmatter
        let mut id = None;
        let mut title = String::new();
        let mut status = crate::task::TaskStatus::Pending;
        let mut priority = crate::task::Priority::Medium;
        let mut complexity = None;
        let mut dependencies = std::collections::HashSet::new();
        let mut subtasks = Vec::new();
        let mut tags = Vec::new();
        let mut created_at = Utc::now();
        let mut updated_at = Utc::now();
        let mut completed_at = None;
        
        for line in frontmatter.lines() {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let key = parts[0].trim();
            let value = parts[1].trim();
            
            match key {
                "id" => id = Some(value.parse::<u32>()?),
                "title" => title = value.trim_matches('"').to_string(),
                "status" => status = serde_json::from_str(&format!("\"{}\"", value))?,
                "priority" => priority = serde_json::from_str(&format!("\"{}\"", value))?,
                "complexity" => complexity = Some(serde_json::from_str(&format!("\"{}\"", value))?),
                "dependencies" => {
                    if !value.is_empty() && value != "[]" {
                        let deps_str = value.trim_start_matches('[').trim_end_matches(']');
                        for dep in deps_str.split(',') {
                            if let Ok(dep_id) = dep.trim().parse::<u32>() {
                                dependencies.insert(dep_id);
                            }
                        }
                    }
                },
                "subtasks" => {
                    if !value.is_empty() && value != "[]" {
                        let subtasks_str = value.trim_start_matches('[').trim_end_matches(']');
                        for subtask in subtasks_str.split(',') {
                            if let Ok(subtask_id) = subtask.trim().parse::<u32>() {
                                subtasks.push(subtask_id);
                            }
                        }
                    }
                },
                "tags" => {
                    if !value.is_empty() && value != "[]" {
                        let tags_str = value.trim_start_matches('[').trim_end_matches(']');
                        for tag in tags_str.split(',') {
                            tags.push(tag.trim().trim_matches('"').to_string());
                        }
                    }
                },
                "created_at" => created_at = value.parse()?,
                "updated_at" => updated_at = value.parse()?,
                "completed_at" => completed_at = Some(value.parse()?),
                _ => {}
            }
        }
        
        let id = id.ok_or_else(|| anyhow::anyhow!("Missing task ID"))?;
        
        // Extract description from body (skip the title line)
        let description = body.lines()
            .skip_while(|line| line.starts_with('#') || line.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        
        Ok(Task {
            id,
            title,
            description,
            status,
            priority,
            complexity,
            dependencies,
            subtasks,
            created_at,
            updated_at,
            completed_at,
            tags,
        })
    }
}