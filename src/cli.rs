use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "trusty")]
#[command(about = "A task manager for Claude Code", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List all tasks
    List {
        /// Show all tasks including completed ones
        #[arg(long)]
        all: bool,
        
        /// Show only completed tasks
        #[arg(long, conflicts_with = "all")]
        completed: bool,
        
        /// Show completed tasks from the last N minutes
        #[arg(long, value_name = "MINUTES")]
        recent: Option<u64>,
    },
    
    /// Add a new task
    Add {
        /// Task title
        #[arg(required_unless_present = "prompt")]
        title: Option<String>,
        
        /// Task description
        #[arg(long)]
        description: Option<String>,
        
        /// Task priority (high, medium, low)
        #[arg(short, long, default_value = "medium")]
        priority: String,
        
        /// Task dependencies (comma-separated task IDs)
        #[arg(short = 'd', long)]
        dependencies: Option<String>,
        
        /// Task tags (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
        
        /// Generate task from natural language prompt using Claude
        #[arg(long, conflicts_with_all = ["description", "priority", "tags"])]
        prompt: Option<String>,
    },
    
    /// Show task details
    Show {
        /// Task ID
        id: u32,
        
        /// Include subtasks in the display
        #[arg(long)]
        with_subtasks: bool,
    },
    
    /// Update task status
    SetStatus {
        /// Task ID
        #[arg(long)]
        id: u32,
        
        /// New status (pending, in-progress, done, blocked, deferred, cancelled)
        #[arg(long)]
        status: String,
        
        /// Also update all subtasks to the same status
        #[arg(long)]
        cascade: bool,
    },
    
    /// Edit a task
    Edit {
        /// Task ID
        id: u32,
        
        /// New title
        #[arg(long)]
        title: Option<String>,
        
        /// New description
        #[arg(long)]
        description: Option<String>,
        
        /// New priority
        #[arg(long)]
        priority: Option<String>,
        
        /// New complexity
        #[arg(long)]
        complexity: Option<String>,
    },
    
    /// Delete a task
    Delete {
        /// Task ID
        id: u32,
    },
    
    /// Add a dependency to a task
    AddDep {
        /// Task ID
        #[arg(long)]
        task: u32,
        
        /// Dependency task ID
        #[arg(long)]
        dep: u32,
    },
    
    /// Remove a dependency from a task
    RemoveDep {
        /// Task ID
        #[arg(long)]
        task: u32,
        
        /// Dependency task ID
        #[arg(long)]
        dep: u32,
    },
    
    /// Add a subtask to an existing task
    AddSubtask {
        /// Parent task ID
        #[arg(long)]
        task: u32,
        
        /// Subtask title
        #[arg(required_unless_present = "prompt")]
        title: Option<String>,
        
        /// Subtask description
        #[arg(long)]
        description: Option<String>,
        
        /// Priority (inherits from parent if not specified)
        #[arg(long)]
        priority: Option<String>,
        
        /// Tags (inherits from parent if not specified)
        #[arg(long)]
        tags: Option<String>,
        
        /// Generate subtask from natural language prompt
        #[arg(long)]
        prompt: Option<String>,
    },
    
    /// Remove a subtask from a task
    RemoveSubtask {
        /// Parent task ID
        #[arg(long)]
        task: u32,
        
        /// Subtask ID to remove
        #[arg(long)]
        subtask: u32,
    },
    
    /// Mark a task (and optionally all subtasks) as complete
    Complete {
        /// Task ID
        id: u32,
        
        /// Also mark all subtasks as complete
        #[arg(long)]
        all: bool,
    },
    
    /// Initialize trusty in the current directory
    Init,
    
    /// Add trusty project manager agent to Claude
    AddAgent {
        /// Scope: must be either "local" or "global"
        #[arg(long, value_parser = ["local", "global"], required_unless_present_any = ["local", "global"])]
        scope: Option<String>,
        
        /// Install agent globally
        #[arg(long, conflicts_with = "local")]
        global: bool,
        
        /// Install agent locally in project
        #[arg(long, conflicts_with = "global")]
        local: bool,
        
        /// Agent name
        #[arg(long, default_value = "trusty-project-manager")]
        name: String,
        
        /// Model to use (opus or sonnet)
        #[arg(long, default_value = "sonnet", value_parser = ["opus", "sonnet"])]
        model: String,
        
        /// Agent color
        #[arg(long, default_value = "blue", value_parser = ["red", "blue", "green", "yellow", "purple", "orange", "pink", "cyan"])]
        color: String,
    },
    
    /// Run an interactive demonstration of trusty features
    Demo {
        /// Skip confirmation prompts
        #[arg(long)]
        skip_confirm: bool,
        
        /// Delay between steps in milliseconds (default: 1000)
        #[arg(long, default_value = "1000")]
        delay: u64,
        
        /// Keep demo data after completion
        #[arg(long)]
        keep: bool,
    },
    
    /// Delete all tasks in the current project
    Nuke {
        /// Skip confirmation prompt
        #[arg(short = 'y', long = "yes")]
        force: bool,
    },
    
    /// Decompose a task into subtasks using AI
    Decompose {
        /// Task ID to decompose
        id: u32,
        
        /// Number of subtasks to create (default: 3-5)
        #[arg(long, default_value = "4")]
        count: u32,
        
        /// Preview decomposition without creating subtasks
        #[arg(long)]
        preview: bool,
    },
    
    /// Get the recommended next task to work on
    Next {
        /// Automatically set the task to in-progress
        #[arg(long)]
        start: bool,
        
        /// Show detailed task information
        #[arg(long)]
        details: bool,
    },
    
    /// Identify and prune stale or completed tasks
    Prune {
        /// Run in dry-run mode (show what would be pruned without making changes)
        #[arg(long)]
        dry_run: bool,
        
        /// Auto-apply suggested actions without confirmation
        #[arg(long)]
        auto: bool,
        
        /// Maximum number of tasks to suggest for pruning
        #[arg(long, default_value = "10")]
        limit: usize,
    },
    
    /// Import tasks from a file
    Import {
        /// Path to the file to import
        file: String,
        
        /// Format of the import file (json, yaml, markdown)
        #[arg(short, long, value_parser = ["json", "yaml", "markdown"])]
        format: String,
        
        /// How to handle duplicate tasks (skip, overwrite, rename)
        #[arg(long, default_value = "skip", value_parser = ["skip", "overwrite", "rename"])]
        duplicates: String,
        
        /// Preview what would be imported without making changes
        #[arg(long)]
        preview: bool,
    },
    
    /// Task-specific operations
    Task {
        #[command(subcommand)]
        command: TaskCommands,
    },
    
    /// Analyze tasks for business/user value and suggest focus areas
    Focus {
        /// Path to Product Requirements Document (PRD) for alignment analysis
        #[arg(long)]
        prd: Option<String>,
        
        /// Preview analysis without making changes
        #[arg(long)]
        preview: bool,
        
        /// Use Claude AI for deeper value analysis
        #[arg(long)]
        ai: bool,
    },
}

#[derive(Subcommand)]
pub enum TaskCommands {
    /// Get advice on what to do with a specific task
    Advice {
        /// Task ID to analyze
        #[arg(long)]
        id: u32,
        
        /// Show detailed analysis
        #[arg(long)]
        detailed: bool,
        
        /// Use Claude AI to analyze codebase and completed tasks
        #[arg(long)]
        ask_claude: bool,
        
        /// Interactive mode - select and execute suggested actions
        #[arg(short, long)]
        interactive: bool,
        
        /// Add suggested commands to shell history
        #[arg(long)]
        history: bool,
    },
}