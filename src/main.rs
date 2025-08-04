mod agent;
mod cli;
mod claude_integration;
mod display;
mod storage;
mod task;

use anyhow::Result;
use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::{thread, time::Duration};
use std::io::{self, Write};
use std::process::Command;
use std::env;

use crate::cli::{Cli, Commands};
use crate::display::TaskDisplay;
use crate::storage::TaskStorage;
use crate::task::{Priority, Task, TaskStatus};

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Init => init_trusty(),
        _ => {
            let storage = get_storage()?;
            handle_command(cli.command, storage)
        }
    }
}

fn init_trusty() -> Result<()> {
    let tasks_dir = get_tasks_dir()?;
    std::fs::create_dir_all(&tasks_dir)?;
    
    println!("{}", "✅ Trusty initialized successfully!".green());
    println!("Tasks will be stored in: {}", tasks_dir.display());
    
    Ok(())
}

fn handle_command(command: Commands, storage: TaskStorage) -> Result<()> {
    match command {
        Commands::List => {
            let tasks = storage.list_all_tasks()?;
            let project_path = get_tasks_dir()?.display().to_string();
            TaskDisplay::display_task_list(&tasks, &project_path);
        }
        
        Commands::Add { title, description, priority, dependencies, tags, prompt } => {
            let tasks = storage.list_all_tasks()?;
            let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            
            let (final_title, final_description, final_priority, final_tags) = if let Some(prompt_text) = prompt {
                // Generate task from prompt
                println!("🤖 Generating task from prompt...");
                match crate::claude_integration::generate_task_from_prompt(&prompt_text) {
                    Ok(generated) => {
                        println!("{} Generated task details:", "✨".green());
                        println!("  Title: {}", generated.title.cyan());
                        println!("  Priority: {}", generated.priority);
                        println!("  Tags: {}", generated.tags.join(", "));
                        
                        let priority = parse_priority(&generated.priority)?;
                        (generated.title, generated.description, priority, generated.tags)
                    }
                    Err(e) => {
                        eprintln!("{} Failed to generate task: {}", "❌".red(), e);
                        return Err(e);
                    }
                }
            } else {
                // Use provided values
                let title = title.ok_or_else(|| anyhow::anyhow!("Title is required when not using --prompt"))?;
                let priority = parse_priority(&priority)?;
                let tags_vec = tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
                (title, description.unwrap_or_default(), priority, tags_vec)
            };
            
            let mut task = Task::new(
                next_id,
                final_title.clone(),
                final_description,
                final_priority,
            );
            
            task.tags = final_tags;
            
            if let Some(deps) = dependencies {
                for dep in deps.split(',') {
                    if let Ok(dep_id) = dep.trim().parse::<u32>() {
                        task.add_dependency(dep_id);
                    }
                }
            }
            
            storage.save_task(&task)?;
            println!("{} Created task #{}: {}", "✅".green(), next_id, final_title);
        }
        
        Commands::Show { id, with_subtasks } => {
            let task = storage.load_task(id)?;
            let all_tasks = storage.list_all_tasks()?;
            display_task_details(&task, Some(&all_tasks));
            
            if with_subtasks && !task.subtasks.is_empty() {
                println!("\n{}", "Subtasks:".bold());
                println!("{}", "─".repeat(50));
                
                for (i, &subtask_id) in task.subtasks.iter().enumerate() {
                    match storage.load_task(subtask_id) {
                        Ok(subtask) => {
                            println!("  {}. [#{}] {} - {}", 
                                i + 1, 
                                subtask.id, 
                                subtask.title,
                                subtask.status
                            );
                        }
                        Err(_) => {
                            println!("  {}. [#{}] (Task not found)", i + 1, subtask_id);
                        }
                    }
                }
            }
        }
        
        Commands::SetStatus { id, status, cascade } => {
            let mut task = storage.load_task(id)?;
            let new_status = parse_status(&status)?;
            task.set_status(new_status.clone());
            storage.save_task(&task)?;
            
            let mut updated_count = 1;
            
            if cascade && !task.subtasks.is_empty() {
                // Recursively update all subtasks
                fn update_subtasks_status(storage: &TaskStorage, subtask_ids: &[u32], status: &TaskStatus) -> Result<usize> {
                    let mut count = 0;
                    for &subtask_id in subtask_ids {
                        if let Ok(mut subtask) = storage.load_task(subtask_id) {
                            subtask.set_status(status.clone());
                            storage.save_task(&subtask)?;
                            count += 1;
                            
                            // Recursively update this subtask's subtasks
                            if !subtask.subtasks.is_empty() {
                                count += update_subtasks_status(storage, &subtask.subtasks, status)?;
                            }
                        }
                    }
                    Ok(count)
                }
                
                updated_count += update_subtasks_status(&storage, &task.subtasks, &new_status)?;
            }
            
            println!("{} Updated {} task{} to status: {}", 
                "✅".green(), 
                updated_count,
                if updated_count > 1 { "s" } else { "" },
                task.status
            );
        }
        
        Commands::Edit { id, title, description, priority, complexity } => {
            let mut task = storage.load_task(id)?;
            
            if let Some(title) = title {
                task.title = title;
            }
            
            if let Some(description) = description {
                task.description = description;
            }
            
            if let Some(priority) = priority {
                task.priority = parse_priority(&priority)?;
            }
            
            if let Some(complexity) = complexity {
                task.complexity = Some(parse_complexity(&complexity)?);
            }
            
            task.updated_at = chrono::Utc::now();
            storage.save_task(&task)?;
            
            println!("{} Updated task #{}", "✅".green(), id);
        }
        
        Commands::Delete { id } => {
            storage.delete_task(id)?;
            println!("{} Deleted task #{}", "✅".green(), id);
        }
        
        Commands::AddDep { task, dep } => {
            let mut t = storage.load_task(task)?;
            t.add_dependency(dep);
            storage.save_task(&t)?;
            
            println!("{} Added dependency #{} to task #{}", "✅".green(), dep, task);
        }
        
        Commands::RemoveDep { task, dep } => {
            let mut t = storage.load_task(task)?;
            t.remove_dependency(dep);
            storage.save_task(&t)?;
            
            println!("{} Removed dependency #{} from task #{}", "✅".green(), dep, task);
        }
        
        Commands::AddSubtask { task, title, description, priority, tags, prompt } => {
            let parent_task = storage.load_task(task)?;
            let tasks = storage.list_all_tasks()?;
            let next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
            
            let (final_title, final_description, final_priority, final_tags) = if let Some(prompt_text) = prompt {
                // Generate subtask from prompt
                println!("🤖 Generating subtask from prompt...");
                let full_prompt = format!("Parent task: '{}'. {}", parent_task.title, prompt_text);
                match crate::claude_integration::generate_task_from_prompt(&full_prompt) {
                    Ok(generated) => {
                        println!("{} Generated subtask details:", "✨".green());
                        println!("  Title: {}", generated.title.cyan());
                        println!("  Priority: {}", generated.priority);
                        println!("  Tags: {}", generated.tags.join(", "));
                        
                        let priority = parse_priority(&generated.priority)?;
                        (generated.title, generated.description, priority, generated.tags)
                    }
                    Err(e) => {
                        eprintln!("{} Failed to generate subtask: {}", "❌".red(), e);
                        return Err(e);
                    }
                }
            } else {
                // Use provided values or inherit from parent
                let title = title.ok_or_else(|| anyhow::anyhow!("Title is required when not using --prompt"))?;
                let priority = if let Some(p) = priority {
                    parse_priority(&p)?
                } else {
                    parent_task.priority.clone()
                };
                let tags_vec = if let Some(t) = tags {
                    t.split(',').map(|s| s.trim().to_string()).collect()
                } else {
                    parent_task.tags.clone()
                };
                (title, description.unwrap_or_default(), priority, tags_vec)
            };
            
            let mut subtask = Task::new(
                next_id,
                final_title.clone(),
                final_description,
                final_priority,
            );
            subtask.tags = final_tags;
            
            // Save the subtask
            storage.save_task(&subtask)?;
            
            // Update parent task with new subtask
            let mut parent = storage.load_task(task)?;
            parent.add_subtask(next_id);
            storage.save_task(&parent)?;
            
            println!("{} Created subtask #{}: {} for task #{}", "✅".green(), next_id, final_title, task);
        }
        
        Commands::RemoveSubtask { task, subtask } => {
            let mut parent = storage.load_task(task)?;
            let initial_count = parent.subtasks.len();
            parent.subtasks.retain(|&id| id != subtask);
            
            if parent.subtasks.len() < initial_count {
                storage.save_task(&parent)?;
                println!("{} Removed subtask #{} from task #{}", "✅".green(), subtask, task);
            } else {
                println!("{} Subtask #{} was not found in task #{}", "⚠️".yellow(), subtask, task);
            }
        }
        
        Commands::Complete { id, all } => {
            let command = Commands::SetStatus {
                id,
                status: "done".to_string(),
                cascade: all,
            };
            return handle_command(command, storage);
        }
        
        Commands::Init => unreachable!(),
        
        Commands::AddAgent { scope, global, local: _, name, model, color } => {
            let is_global = global || scope.as_deref() == Some("global");
            let agent_config = agent::AgentConfig::new(name, model, color, is_global);
            
            match agent::install_agent(&agent_config) {
                Ok(path) => {
                    println!("{} Successfully installed agent '{}' to:", "✅".green(), agent_config.name);
                    println!("   {}", path.display());
                    println!("\n{} To use this agent in Claude:", "💡".yellow());
                    println!("   1. Start a new Claude conversation");
                    println!("   2. Type: {} {}", "@".cyan(), agent_config.name.cyan());
                    println!("   3. Ask for help managing your project!");
                }
                Err(e) => {
                    eprintln!("{} Failed to install agent: {}", "❌".red(), e);
                    return Err(e);
                }
            }
        }
        
        Commands::Demo { skip_confirm, delay, keep } => {
            run_demo(storage, skip_confirm, delay, keep)?;
        }
        
        Commands::Nuke { force } => {
            let tasks = storage.list_all_tasks()?;
            let task_count = tasks.len();
            
            if task_count == 0 {
                println!("{} No tasks to delete.", "ℹ️".blue());
                return Ok(());
            }
            
            if !force {
                println!("{}", "⚠️  WARNING: This will delete ALL tasks in the current project!".bright_red().bold());
                println!("Found {} task(s) to delete:", task_count);
                
                // Show first 10 tasks as preview
                for (i, task) in tasks.iter().take(10).enumerate() {
                    println!("  #{} - {}", task.id, task.title);
                    if i == 9 && task_count > 10 {
                        println!("  ... and {} more", task_count - 10);
                    }
                }
                
                print!("\nAre you sure you want to delete all tasks? Type 'yes' to confirm: ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if input.trim() != "yes" {
                    println!("{} Cancelled.", "✗".red());
                    return Ok(());
                }
            }
            
            // Delete all tasks
            let mut deleted = 0;
            let mut errors = 0;
            
            for task in tasks {
                match storage.delete_task(task.id) {
                    Ok(_) => deleted += 1,
                    Err(e) => {
                        eprintln!("{} Failed to delete task #{}: {}", "❌".red(), task.id, e);
                        errors += 1;
                    }
                }
            }
            
            if errors > 0 {
                println!("{} Deleted {} task(s) with {} error(s).", "⚠️".yellow(), deleted, errors);
            } else {
                println!("{} Successfully deleted {} task(s)!", "💥".bright_red(), deleted);
            }
        }
        
        Commands::Decompose { id, count, preview } => {
            let task = storage.load_task(id)?;
            
            // Check if task already has subtasks
            if !task.subtasks.is_empty() && !preview {
                println!("{} Task #{} already has {} subtask(s).", "⚠️".yellow(), id, task.subtasks.len());
                print!("Do you want to continue and add more subtasks? (y/n): ");
                io::stdout().flush()?;
                
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                
                if input.trim().to_lowercase() != "y" {
                    println!("{} Cancelled.", "✗".red());
                    return Ok(());
                }
            }
            
            println!("🤖 Decomposing task '{}' into {} subtasks...", task.title.cyan(), count);
            
            // Convert priority to string
            let priority_str = match &task.priority {
                Priority::High => "high",
                Priority::Medium => "medium",
                Priority::Low => "low",
            };
            
            match crate::claude_integration::decompose_task(
                &task.title,
                &task.description,
                priority_str,
                &task.tags,
                count
            ) {
                Ok(decomposed) => {
                    if preview {
                        println!("\n{} Preview of decomposed subtasks:", "👁️".blue());
                        println!("{}", "─".repeat(50));
                        
                        for (i, subtask) in decomposed.subtasks.iter().enumerate() {
                            println!("\n{}. {}", i + 1, subtask.title.cyan().bold());
                            println!("   Priority: {}", subtask.priority);
                            println!("   Tags: {}", subtask.tags.join(", "));
                            if !subtask.description.is_empty() {
                                println!("   Description: {}", subtask.description);
                            }
                        }
                        
                        println!("\n{} This is a preview. Run without --preview to create these subtasks.", "ℹ️".blue());
                    } else {
                        // Create the subtasks
                        let tasks = storage.list_all_tasks()?;
                        let mut next_id = tasks.iter().map(|t| t.id).max().unwrap_or(0) + 1;
                        let mut created_count = 0;
                        
                        for subtask in decomposed.subtasks {
                            let priority = parse_priority(&subtask.priority)?;
                            let mut new_task = Task::new(
                                next_id,
                                subtask.title.clone(),
                                subtask.description,
                                priority,
                            );
                            new_task.tags = subtask.tags;
                            
                            storage.save_task(&new_task)?;
                            
                            // Update parent task with new subtask
                            let mut parent = storage.load_task(id)?;
                            parent.add_subtask(next_id);
                            storage.save_task(&parent)?;
                            
                            println!("{} Created subtask #{}: {}", "✅".green(), next_id, subtask.title);
                            
                            next_id += 1;
                            created_count += 1;
                        }
                        
                        println!("\n{} Successfully created {} subtask(s) for task #{}", "🎉".green(), created_count, id);
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to decompose task: {}", "❌".red(), e);
                    return Err(e);
                }
            }
        }
    }
    
    Ok(())
}

fn get_storage() -> Result<TaskStorage> {
    let tasks_dir = get_tasks_dir()?;
    TaskStorage::new(tasks_dir)
}

fn get_tasks_dir() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    Ok(current_dir.join(".trusty").join("tasks"))
}

fn parse_priority(s: &str) -> Result<Priority> {
    match s.to_lowercase().as_str() {
        "high" => Ok(Priority::High),
        "medium" => Ok(Priority::Medium),
        "low" => Ok(Priority::Low),
        _ => anyhow::bail!("Invalid priority: {}. Use high, medium, or low", s),
    }
}

fn parse_status(s: &str) -> Result<TaskStatus> {
    match s.to_lowercase().as_str() {
        "pending" => Ok(TaskStatus::Pending),
        "in-progress" => Ok(TaskStatus::InProgress),
        "done" => Ok(TaskStatus::Done),
        "blocked" => Ok(TaskStatus::Blocked),
        "deferred" => Ok(TaskStatus::Deferred),
        "cancelled" => Ok(TaskStatus::Cancelled),
        _ => anyhow::bail!("Invalid status: {}. Use pending, in-progress, done, blocked, deferred, or cancelled", s),
    }
}

fn parse_complexity(s: &str) -> Result<crate::task::Complexity> {
    match s.to_lowercase().as_str() {
        "simple" => Ok(crate::task::Complexity::Simple),
        "medium" => Ok(crate::task::Complexity::Medium),
        "complex" => Ok(crate::task::Complexity::Complex),
        _ => anyhow::bail!("Invalid complexity: {}. Use simple, medium, or complex", s),
    }
}

fn display_task_details(task: &Task, all_tasks: Option<&[Task]>) {
    println!("\n{}", format!("Task #{}", task.id).cyan().bold());
    println!("{}", "─".repeat(50));
    println!("{}: {}", "Title".bold(), task.title);
    // Show effective status if different from stored status
    if let Some(tasks) = all_tasks {
        let effective_status = task.compute_effective_status(tasks);
        if !task.subtasks.is_empty() {
            let (completed, total) = task.subtask_progress(tasks);
            if effective_status != task.status {
                println!("{}: {} (effective: {})", "Status".bold(), task.status, effective_status);
            } else {
                println!("{}: {}", "Status".bold(), task.status);
            }
            println!("{}: {}/{} subtasks complete", "Progress".bold(), completed, total);
        } else {
            println!("{}: {}", "Status".bold(), task.status);
        }
    } else {
        println!("{}: {}", "Status".bold(), task.status);
    }
    println!("{}: {}", "Priority".bold(), task.priority);
    
    if let Some(complexity) = &task.complexity {
        println!("{}: {}", "Complexity".bold(), complexity);
    }
    
    if !task.dependencies.is_empty() {
        println!("{}: {:?}", "Dependencies".bold(), task.dependencies.iter().collect::<Vec<_>>());
    }
    
    if !task.subtasks.is_empty() {
        println!("{}: {} subtask(s) - IDs: {:?}", "Subtasks".bold(), task.subtasks.len(), task.subtasks);
    }
    
    if !task.tags.is_empty() {
        println!("{}: {}", "Tags".bold(), task.tags.join(", "));
    }
    
    println!("{}: {}", "Created".bold(), task.created_at.format("%Y-%m-%d %H:%M:%S"));
    println!("{}: {}", "Updated".bold(), task.updated_at.format("%Y-%m-%d %H:%M:%S"));
    
    if let Some(completed_at) = task.completed_at {
        println!("{}: {}", "Completed".bold(), completed_at.format("%Y-%m-%d %H:%M:%S"));
    }
    
    if !task.description.is_empty() {
        println!("\n{}", "Description:".bold());
        println!("{}", task.description);
    }
}

fn run_demo(_storage: TaskStorage, skip_confirm: bool, delay_ms: u64, keep: bool) -> Result<()> {
    let delay = Duration::from_millis(delay_ms);
    
    // Welcome message
    println!("\n{}", "Welcome to the Trusty Demo!".bright_cyan().bold());
    println!("{}", "═".repeat(50).bright_cyan());
    println!("\nThis interactive demo will show you how to use trusty's features:");
    println!("  • Creating and managing tasks");
    println!("  • Working with subtasks and automatic status aggregation");
    println!("  • Setting up task dependencies");
    println!("  • Using cascade operations");
    println!("  • And more!\n");
    
    if !skip_confirm {
        print!("Press Enter to begin the demo (or Ctrl+C to cancel)... ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
    }
    
    // Create temporary demo directory
    let temp_dir = env::temp_dir();
    let demo_dir_name = format!("trusty-demo-{}", chrono::Utc::now().timestamp());
    let demo_dir = temp_dir.join(demo_dir_name);
    std::fs::create_dir_all(&demo_dir)?;
    
    println!("\n{} Created demo directory: {}", "📁".blue(), demo_dir.display());
    
    // Save current directory to restore later
    let original_dir = env::current_dir()?;
    
    // Change to demo directory
    env::set_current_dir(&demo_dir)?;
    
    // Get the path to the trusty executable
    let trusty_exe = env::current_exe()?;
    
    // Step counter
    let mut step = 1;
    let total_steps = 8;
    
    // Helper function to print step headers
    let print_step = |step: usize, title: &str| {
        println!("\n{}", format!("═══ Step {}/{}: {} ═══", step, total_steps, title).bright_yellow().bold());
    };
    
    // Helper function to run command and capture output
    let run_trusty_command = |args: &[&str]| -> Result<String> {
        println!("\n{} trusty {}", "Running:".bright_green(), args.join(" ").cyan());
        thread::sleep(delay);
        
        let output = Command::new(&trusty_exe)
            .args(args)
            .output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !output.status.success() {
            eprintln!("{}", stderr.red());
            anyhow::bail!("Command failed: trusty {}", args.join(" "));
        }
        
        print!("{}", stdout);
        Ok(stdout.to_string())
    };
    
    // Initialize trusty in demo directory
    println!("\n{} Initializing trusty in demo directory...", "🔧".yellow());
    run_trusty_command(&["init"])?;
    
    // Step 1: Create basic tasks
    print_step(step, "Creating Basic Tasks");
    println!("Let's start by creating a few tasks...");
    thread::sleep(delay);
    
    run_trusty_command(&["add", "Build user authentication", "--priority", "high", "--description", "Implement OAuth2 authentication system"])?;
    thread::sleep(delay);
    
    run_trusty_command(&["add", "Write API documentation", "--priority", "medium", "--description", "Document all REST API endpoints"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 2: List tasks
    print_step(step, "Listing Tasks");
    println!("Now let's see our tasks...");
    run_trusty_command(&["list"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 3: Create subtasks
    print_step(step, "Working with Subtasks");
    println!("Let's break down the authentication task into subtasks...");
    thread::sleep(delay);
    
    run_trusty_command(&["add-subtask", "--task", "1", "Design database schema", "--description", "Create user and session tables", "--priority", "high"])?;
    thread::sleep(delay);
    
    run_trusty_command(&["add-subtask", "--task", "1", "Implement OAuth2 flow", "--description", "Add Google and GitHub OAuth providers", "--priority", "high"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 4: Show automatic status aggregation
    print_step(step, "Automatic Status Aggregation");
    println!("Notice how the parent task status is computed from its subtasks...");
    run_trusty_command(&["show", "1", "--with-subtasks"])?;
    
    thread::sleep(delay);
    
    println!("\nNow let's complete one subtask...");
    run_trusty_command(&["complete", "3"])?;
    
    thread::sleep(delay);
    
    println!("\nThe parent task now shows progress...");
    run_trusty_command(&["show", "1"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 5: Dependencies
    print_step(step, "Task Dependencies");
    println!("Let's create a task that depends on authentication...");
    run_trusty_command(&["add", "Deploy to production", "--dependencies", "1", "--description", "Deploy the application with new auth system"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 6: Cascade operations
    print_step(step, "Cascade Operations");
    println!("Let's create a new task with subtasks to demonstrate cascade operations...");
    run_trusty_command(&["add", "Feature X", "--priority", "high"])?;
    thread::sleep(delay);
    
    run_trusty_command(&["add-subtask", "--task", "6", "Part A"])?;
    thread::sleep(delay);
    
    run_trusty_command(&["add-subtask", "--task", "6", "Part B"])?;
    thread::sleep(delay);
    
    println!("\nNow let's update all tasks at once using cascade...");
    run_trusty_command(&["set-status", "--id", "6", "--status", "in-progress", "--cascade"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 7: Complete all
    print_step(step, "Complete All");
    println!("Now let's complete the parent and all subtasks at once...");
    run_trusty_command(&["complete", "6", "--all"])?;
    thread::sleep(delay);
    
    println!("\nLet's verify all tasks are completed...");
    run_trusty_command(&["show", "6", "--with-subtasks"])?;
    
    step += 1;
    thread::sleep(delay);
    
    // Step 8: Summary
    print_step(step, "Demo Complete!");
    println!("\n{}", "You've learned how to:".bright_green());
    println!("  ✓ Create and list tasks");
    println!("  ✓ Work with subtasks and see automatic status aggregation");
    println!("  ✓ Set up task dependencies");
    println!("  ✓ Use cascade operations to update multiple tasks");
    println!("  ✓ Complete tasks and subtasks efficiently");
    
    // Change back to original directory
    env::set_current_dir(&original_dir)?;
    
    if !keep {
        println!("\n{} Cleaning up demo directory...", "🧹".yellow());
        std::fs::remove_dir_all(&demo_dir)?;
        println!("{} Demo directory cleaned up!", "✅".green());
    } else {
        println!("\n{} Demo directory kept as requested.", "ℹ️".blue());
        println!("Demo location: {}", demo_dir.display());
        println!("To continue working with the demo:");
        println!("  cd {}", demo_dir.display());
        println!("  trusty list");
    }
    
    println!("\n{}", "Thank you for trying Trusty! 🚀".bright_cyan().bold());
    println!("Run {} to see all available commands.", "trusty --help".cyan());
    
    Ok(())
}
