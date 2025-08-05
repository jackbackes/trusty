use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::process::Command;
use std::env;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub fn execute_action_interactive(actions: Vec<(String, String, Option<String>)>) -> Result<()> {
    if actions.is_empty() {
        println!("{} No actions available to execute.", "ℹ️".blue());
        return Ok(());
    }

    println!("\n{}", "─".repeat(70));
    println!("{}", "Which action would you like to execute?".bold());
    
    for (i, (action, _desc, cmd)) in actions.iter().enumerate() {
        println!("  {}. {}", (i + 1).to_string().yellow(), action.cyan());
        if let Some(cmd) = cmd {
            println!("     {}: {}", "Command".dimmed(), cmd);
        }
    }
    
    println!("  {}. {}", "c".yellow(), "Chat with Claude about these suggestions".bright_blue());
    println!("  {}. {}", "q".yellow(), "Quit without executing".red());
    
    print!("\nYour choice (1-{}, c, or q): ", actions.len());
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let choice = input.trim().to_lowercase();
    
    if choice == "q" {
        println!("{} Cancelled.", "✗".red());
        return Ok(());
    } else if choice == "c" {
        // Chat with Claude about the suggestions
        println!("\n{}", "🤖 Starting chat with Claude about these task suggestions...".bright_blue().bold());
        
        // Build context for Claude
        let mut context = String::new();
        context.push_str("I'm using Trusty task manager and got these action suggestions:\n\n");
        
        for (i, (action, desc, cmd)) in actions.iter().enumerate() {
            context.push_str(&format!("{}. {}\n", i + 1, action));
            context.push_str(&format!("   Description: {}\n", desc));
            if let Some(command) = cmd {
                context.push_str(&format!("   Command: {}\n", command));
            }
            context.push_str("\n");
        }
        
        context.push_str("I'd like to discuss these suggestions and potentially refine them. What do you think about these recommendations?");
        
        // Call Claude CLI for interactive chat
        match start_claude_chat(&context) {
            Ok(refined_suggestion) => {
                println!("\n{}", "💡 Claude's refined suggestion:".green().bold());
                println!("{}", refined_suggestion);
                
                // Ask if they want to execute any refined command
                println!("\n{}", "Would you like to execute a command based on this discussion?".bold());
                print!("Enter command (or press Enter to skip): ");
                io::stdout().flush()?;
                
                let mut refined_command = String::new();
                io::stdin().read_line(&mut refined_command)?;
                let refined_command = refined_command.trim();
                
                if !refined_command.is_empty() {
                    let full_command = if refined_command.starts_with("trusty") {
                        refined_command.to_string()
                    } else {
                        format!("trusty {}", refined_command)
                    };
                    
                    println!("\n{} Executing: {}", "🚀".yellow(), full_command.cyan());
                    
                    print!("Proceed? (y/n): ");
                    io::stdout().flush()?;
                    
                    let mut confirm = String::new();
                    io::stdin().read_line(&mut confirm)?;
                    
                    if confirm.trim().to_lowercase() == "y" {
                        execute_trusty_command(&full_command)?;
                    } else {
                        println!("{} Cancelled.", "✗".red());
                    }
                }
            }
            Err(e) => {
                eprintln!("{} Failed to start Claude chat: {}", "❌".red(), e);
                println!("You can still manually discuss these suggestions with Claude.");
            }
        }
        
        return Ok(());
    }
    
    match choice.parse::<usize>() {
        Ok(n) if n > 0 && n <= actions.len() => {
            let (action, desc, cmd) = &actions[n - 1];
            
            if let Some(command) = cmd {
                println!("\n{} Executing: {}", "🚀".yellow(), command.cyan());
                
                // Ask for confirmation
                print!("Proceed? (y/n): ");
                io::stdout().flush()?;
                
                let mut confirm = String::new();
                io::stdin().read_line(&mut confirm)?;
                
                if confirm.trim().to_lowercase() == "y" {
                    execute_trusty_command(command)?;
                } else {
                    println!("{} Cancelled.", "✗".red());
                }
            } else {
                println!("{} This action doesn't have an associated command.", "⚠️".yellow());
                println!("Action: {}", action);
                println!("Description: {}", desc);
            }
        }
        _ => {
            println!("{} Invalid choice.", "❌".red());
        }
    }
    
    Ok(())
}


fn execute_trusty_command(command: &str) -> Result<()> {
    // Parse the command - it should start with "trusty"
    let parts: Vec<&str> = command.split_whitespace().collect();
    
    if parts.is_empty() || parts[0] != "trusty" {
        anyhow::bail!("Expected a trusty command, got: {}", command);
    }
    
    // Get the trusty executable path
    let trusty_exe = std::env::current_exe()?;
    
    // Execute the command
    let output = Command::new(&trusty_exe)
        .args(&parts[1..])
        .output()?;
    
    // Print output
    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    
    if !output.status.success() {
        anyhow::bail!("Command failed with exit code: {:?}", output.status.code());
    }
    
    println!("\n{} Action completed successfully!", "✅".green());
    
    Ok(())
}

pub fn append_to_bash_history(actions: &[(String, String, Option<String>)]) -> Result<()> {
    // Find the bash history file
    let history_file = get_history_file()?;
    
    if let Some(history_path) = history_file {
        // Open the history file in append mode
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&history_path)?;
        
        // Write each command to the history
        let mut count = 0;
        for (_, _, cmd) in actions {
            if let Some(command) = cmd {
                writeln!(file, "{}", command)?;
                count += 1;
            }
        }
        
        if count > 0 {
            println!("\n{} Added {} command{} to shell history (use ↑ arrow to access)", 
                "💾".green(), 
                count,
                if count > 1 { "s" } else { "" }
            );
        }
    } else {
        // If we can't find the history file, just inform the user
        println!("\n{} Shell history file not found. Commands available above.", "ℹ️".blue());
    }
    
    Ok(())
}

fn get_history_file() -> Result<Option<PathBuf>> {
    // First check if we're in an interactive shell
    if env::var("SHELL").is_err() {
        return Ok(None);
    }
    
    // Get the shell type
    let shell = env::var("SHELL").unwrap_or_default();
    let home_dir = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    
    // Determine history file based on shell
    let history_file = if shell.contains("zsh") {
        // For zsh, check HISTFILE env var first
        if let Ok(histfile) = env::var("HISTFILE") {
            PathBuf::from(histfile)
        } else {
            home_dir.join(".zsh_history")
        }
    } else if shell.contains("bash") {
        // For bash, check HISTFILE env var first
        if let Ok(histfile) = env::var("HISTFILE") {
            PathBuf::from(histfile)
        } else {
            home_dir.join(".bash_history")
        }
    } else {
        // Unknown shell
        return Ok(None);
    };
    
    // Check if the file exists
    if history_file.exists() {
        Ok(Some(history_file))
    } else {
        Ok(None)
    }
}

fn start_claude_chat(context: &str) -> Result<String> {
    // Try to find Claude CLI
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
        .ok_or_else(|| anyhow::anyhow!("Claude CLI not found. Please ensure Claude Code is installed."))?;
    
    // Prepare the prompt for discussion
    let prompt = format!(
        "You are helping a user refine task management suggestions from Trusty. \
         The user wants to discuss these suggestions and potentially come up with better approaches. \
         Please provide thoughtful analysis and refined suggestions.\n\n{}",
        context
    );
    
    // Call Claude CLI
    let output = Command::new(&claude_cmd)
        .arg("--model")
        .arg("sonnet")
        .arg("-p")
        .arg("--output-format")
        .arg("text")
        .arg(&prompt)
        .output()
        .map_err(|e| anyhow::anyhow!("Failed to execute Claude CLI: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Claude CLI failed: {}", stderr);
    }
    
    let response = String::from_utf8(output.stdout)
        .map_err(|_| anyhow::anyhow!("Failed to parse Claude output as UTF-8"))?;
    
    Ok(response.trim().to_string())
}