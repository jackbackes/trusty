use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use crate::task::{Task, Priority, TaskStatus, Complexity};
use crate::storage::TaskStorage;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Json,
    Yaml,
    Markdown,
}

impl ImportFormat {
    pub fn to_lowercase(&self) -> &str {
        match self {
            ImportFormat::Json => "json",
            ImportFormat::Yaml => "yaml",
            ImportFormat::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DuplicateHandling {
    Skip,
    Overwrite,
    Rename,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ImportTask {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u32>,
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default = "default_priority")]
    priority: String,
    #[serde(default = "default_status")]
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    complexity: Option<String>,
    #[serde(default)]
    dependencies: Vec<u32>,
    #[serde(default)]
    subtasks: Vec<u32>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed_at: Option<DateTime<Utc>>,
}

fn default_priority() -> String {
    "medium".to_string()
}

fn default_status() -> String {
    "pending".to_string()
}

pub struct TaskImporter {
    storage: TaskStorage,
    id_mapping: HashMap<u32, u32>, // old_id -> new_id
}

impl TaskImporter {
    pub fn new(storage: TaskStorage) -> Self {
        Self {
            storage,
            id_mapping: HashMap::new(),
        }
    }

    pub fn import_from_file(
        &mut self,
        file_path: &Path,
        format: ImportFormat,
        duplicate_handling: DuplicateHandling,
    ) -> Result<ImportResult> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

        match format {
            ImportFormat::Json => self.import_json(&content, duplicate_handling),
            ImportFormat::Yaml => self.import_yaml(&content, duplicate_handling),
            ImportFormat::Markdown => self.import_markdown(&content, duplicate_handling),
        }
    }

    fn import_json(&mut self, content: &str, duplicate_handling: DuplicateHandling) -> Result<ImportResult> {
        let tasks: Vec<ImportTask> = serde_json::from_str(content)
            .context("Failed to parse JSON")?;
        self.import_tasks(tasks, duplicate_handling)
    }

    fn import_yaml(&mut self, content: &str, duplicate_handling: DuplicateHandling) -> Result<ImportResult> {
        let tasks: Vec<ImportTask> = serde_yaml::from_str(content)
            .context("Failed to parse YAML")?;
        self.import_tasks(tasks, duplicate_handling)
    }

    fn import_markdown(&mut self, content: &str, duplicate_handling: DuplicateHandling) -> Result<ImportResult> {
        let tasks = self.parse_markdown(content)?;
        self.import_tasks(tasks, duplicate_handling)
    }

    fn parse_markdown(&self, content: &str) -> Result<Vec<ImportTask>> {
        let mut tasks = Vec::new();
        let mut current_task: Option<ImportTask> = None;
        let mut in_description = false;
        let mut description_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Task header (e.g., "## Task: Title [high] #tag1 #tag2")
            if let Some(header) = trimmed.strip_prefix("## Task:") {
                // Save previous task if any
                if let Some(mut task) = current_task.take() {
                    task.description = description_lines.join("\n");
                    tasks.push(task);
                    description_lines.clear();
                }

                let header = header.trim();
                let (title, metadata) = self.parse_task_header(header);
                
                current_task = Some(ImportTask {
                    id: None,
                    title,
                    description: String::new(),
                    priority: metadata.priority,
                    status: metadata.status,
                    complexity: metadata.complexity,
                    dependencies: Vec::new(),
                    subtasks: Vec::new(),
                    tags: metadata.tags,
                    created_at: None,
                    updated_at: None,
                    completed_at: None,
                });
                in_description = false;
            }
            // Task metadata
            else if let Some(ref mut task) = current_task {
                if trimmed.starts_with("- Status:") {
                    if let Some(status) = trimmed.strip_prefix("- Status:") {
                        task.status = status.trim().to_lowercase();
                    }
                } else if trimmed.starts_with("- Priority:") {
                    if let Some(priority) = trimmed.strip_prefix("- Priority:") {
                        task.priority = priority.trim().to_lowercase();
                    }
                } else if trimmed.starts_with("- Complexity:") {
                    if let Some(complexity) = trimmed.strip_prefix("- Complexity:") {
                        task.complexity = Some(complexity.trim().to_lowercase());
                    }
                } else if trimmed.starts_with("- Dependencies:") {
                    if let Some(deps) = trimmed.strip_prefix("- Dependencies:") {
                        task.dependencies = deps.trim()
                            .split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect();
                    }
                } else if trimmed.starts_with("- Tags:") {
                    if let Some(tags) = trimmed.strip_prefix("- Tags:") {
                        task.tags = tags.trim()
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                } else if trimmed.starts_with("- Description:") {
                    in_description = true;
                } else if in_description && !trimmed.is_empty() {
                    description_lines.push(line.to_string());
                }
            }
        }

        // Save last task
        if let Some(mut task) = current_task {
            task.description = description_lines.join("\n").trim().to_string();
            tasks.push(task);
        }

        Ok(tasks)
    }

    fn parse_task_header(&self, header: &str) -> (String, TaskMetadata) {
        let parts = header.split_whitespace();
        let mut title_parts = Vec::new();
        let mut priority = "medium".to_string();
        let mut status = "pending".to_string();
        let mut complexity = None;
        let mut tags = Vec::new();

        for part in parts {
            if part.starts_with('[') && part.ends_with(']') {
                // Priority in brackets [high]
                priority = part.trim_matches(|c| c == '[' || c == ']').to_lowercase();
            } else if part.starts_with('#') {
                // Tag
                tags.push(part[1..].to_string());
            } else if part.starts_with('(') && part.ends_with(')') {
                // Status in parentheses (done)
                status = part.trim_matches(|c| c == '(' || c == ')').to_lowercase();
            } else if part.starts_with('{') && part.ends_with('}') {
                // Complexity in braces {complex}
                complexity = Some(part.trim_matches(|c| c == '{' || c == '}').to_lowercase());
            } else {
                title_parts.push(part);
            }
        }

        let title = title_parts.join(" ");

        (title, TaskMetadata {
            priority,
            status,
            complexity,
            tags,
        })
    }

    fn import_tasks(&mut self, import_tasks: Vec<ImportTask>, duplicate_handling: DuplicateHandling) -> Result<ImportResult> {
        let existing_tasks = self.storage.list_all_tasks()?;
        let mut result = ImportResult::default();

        // Get next available ID
        let mut next_id = existing_tasks.iter()
            .map(|t| t.id)
            .max()
            .unwrap_or(0) + 1;

        // First pass: Import tasks without dependencies/subtasks
        for import_task in &import_tasks {
            match self.process_single_task(import_task, &existing_tasks, duplicate_handling, &mut next_id)? {
                ImportAction::Created(id) => {
                    result.created += 1;
                    if let Some(old_id) = import_task.id {
                        self.id_mapping.insert(old_id, id);
                    }
                }
                ImportAction::Updated => result.updated += 1,
                ImportAction::Skipped => result.skipped += 1,
            }
        }

        // Second pass: Update dependencies and subtasks with new IDs
        self.update_relationships()?;

        result.total = import_tasks.len();
        Ok(result)
    }

    fn process_single_task(
        &self,
        import_task: &ImportTask,
        existing_tasks: &[Task],
        duplicate_handling: DuplicateHandling,
        next_id: &mut u32,
    ) -> Result<ImportAction> {
        // Check for duplicates by title
        let duplicate = existing_tasks.iter()
            .find(|t| t.title == import_task.title);

        match (duplicate, duplicate_handling) {
            (Some(_), DuplicateHandling::Skip) => {
                Ok(ImportAction::Skipped)
            }
            (Some(existing), DuplicateHandling::Overwrite) => {
                let task = self.convert_import_task(import_task, existing.id)?;
                self.storage.save_task(&task)?;
                Ok(ImportAction::Updated)
            }
            (Some(_), DuplicateHandling::Rename) | (None, _) => {
                let mut title = import_task.title.clone();
                if duplicate.is_some() {
                    // Find unique name
                    let mut counter = 1;
                    loop {
                        title = format!("{} ({})", import_task.title, counter);
                        if !existing_tasks.iter().any(|t| t.title == title) {
                            break;
                        }
                        counter += 1;
                    }
                }

                let id = *next_id;
                *next_id += 1;

                let mut task = self.convert_import_task(import_task, id)?;
                task.title = title;
                self.storage.save_task(&task)?;
                Ok(ImportAction::Created(id))
            }
        }
    }

    fn convert_import_task(&self, import_task: &ImportTask, id: u32) -> Result<Task> {
        let priority = match import_task.priority.to_lowercase().as_str() {
            "high" => Priority::High,
            "medium" => Priority::Medium,
            "low" => Priority::Low,
            _ => Priority::Medium,
        };

        let status = match import_task.status.to_lowercase().as_str() {
            "pending" => TaskStatus::Pending,
            "in-progress" => TaskStatus::InProgress,
            "done" => TaskStatus::Done,
            "blocked" => TaskStatus::Blocked,
            "deferred" => TaskStatus::Deferred,
            "cancelled" => TaskStatus::Cancelled,
            _ => TaskStatus::Pending,
        };

        let complexity = import_task.complexity.as_ref().and_then(|c| {
            match c.to_lowercase().as_str() {
                "simple" => Some(Complexity::Simple),
                "medium" => Some(Complexity::Medium),
                "complex" => Some(Complexity::Complex),
                _ => None,
            }
        });

        let mut task = Task::new(id, import_task.title.clone(), import_task.description.clone(), priority);
        task.status = status;
        task.complexity = complexity;
        task.tags = import_task.tags.clone();
        
        if let Some(created) = import_task.created_at {
            task.created_at = created;
        }
        if let Some(updated) = import_task.updated_at {
            task.updated_at = updated;
        }
        if let Some(completed) = import_task.completed_at {
            task.completed_at = Some(completed);
        }

        // Dependencies and subtasks will be updated in second pass
        Ok(task)
    }

    fn update_relationships(&self) -> Result<()> {
        let tasks = self.storage.list_all_tasks()?;
        
        for task in tasks {
            let mut updated = false;
            let mut updated_task = task.clone();

            // Update dependencies with new IDs
            let new_deps: Vec<u32> = task.dependencies.iter()
                .filter_map(|&old_id| self.id_mapping.get(&old_id).copied())
                .collect();
            
            if !new_deps.is_empty() {
                updated_task.dependencies.clear();
                for dep in new_deps {
                    updated_task.dependencies.insert(dep);
                }
                updated = true;
            }

            // Update subtasks with new IDs
            let new_subtasks: Vec<u32> = task.subtasks.iter()
                .filter_map(|&old_id| self.id_mapping.get(&old_id).copied())
                .collect();
            
            if !new_subtasks.is_empty() {
                updated_task.subtasks = new_subtasks;
                updated = true;
            }

            if updated {
                self.storage.save_task(&updated_task)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
enum ImportAction {
    Created(u32),
    Updated,
    Skipped,
}

struct TaskMetadata {
    priority: String,
    status: String,
    complexity: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ImportResult {
    pub total: usize,
    pub created: usize,
    pub updated: usize,
    pub skipped: usize,
}

impl ImportResult {
    pub fn summary(&self) -> String {
        format!(
            "Imported {} task(s): {} created, {} updated, {} skipped",
            self.total, self.created, self.updated, self.skipped
        )
    }
}