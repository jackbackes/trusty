# 🚀 Trusty - Task Management for Claude Code

> **AI Task Management that you can Trust!**

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Built for Claude Code](https://img.shields.io/badge/Built%20for-Claude%20Code-blueviolet?style=for-the-badge)](https://claude.ai/code)

**Trusty** is a task manager designed specifically to work with [Claude Code](https://claude.ai/code), Anthropic's AI coding assistant. It leverages Claude's capabilities to help you decompose complex tasks, identify stale work, and maintain a clean, actionable task list.

## ✨ Features

### 🤖 AI-Powered Task Management with Claude
- **Smart Decomposition**: Break down complex tasks into manageable subtasks using Claude
- **Natural Language Input**: Create tasks from natural language descriptions
- **Intelligent Suggestions**: Claude helps identify and organize your work

### 📊 Advanced Task Tracking (fully functional)
- **Hierarchical Tasks**: Support for subtasks with automatic status aggregation
- **Dependencies**: Track task dependencies and see what's blocking your progress
- **Smart Filtering**: Hide old completed tasks by default, with flexible viewing options
- **Priority & Complexity**: Track task priority and complexity levels
- **Intelligent Pruning**: Rule-based identification of stale, completed, or irrelevant tasks (no AI required)

### 🎯 Developer-Focused Workflow
- **Next Task Recommendation**: Get intelligent suggestions for what to work on next
- **Progress Visualization**: Beautiful terminal UI with progress bars and statistics
- **Exponential Backoff**: Prune suggestions get smarter over time, avoiding repeated prompts
- **Git-Friendly**: Tasks stored as JSON files, perfect for version control

## 📦 Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/yourusername/trusty.git
cd trusty

# Build and install
cargo install --path .

# get an interactive demonstration of all features with "trusty demo"!
trusty demo 
```

### Prerequisites
- [Claude Code](https://claude.ai/code) - Required for AI-powered features
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

Trusty is designed to be used alongside Claude Code. The AI features (task decomposition and natural language input) require Claude Code to be installed and available on your system.

## 🚀 Quick Start

```bash
# Initialize trusty in your project
trusty init

# Install the Trusty agent for Claude Code
trusty add-agent --scope local   # For current project only
# OR
trusty add-agent --scope global  # For all projects

# Add your first task
trusty add "Implement user authentication" --priority high

# See your tasks (recently completed tasks hidden by default)
trusty list

# Get recommended next task
trusty next

# Start working on it
trusty next --start

# Mark it complete
trusty complete 1
```

## 📚 Core Commands

### Task Management

```bash
# Add tasks
trusty add "Task title" --description "Details" --priority high
trusty add --prompt "Create a task for implementing OAuth2"  # Uses Claude to generate task

# View tasks
trusty list              # Active tasks only (default)
trusty list --all        # Include all completed tasks
trusty list --completed   # Only completed tasks
trusty list --recent 30   # Tasks completed in last 30 minutes

# Update tasks
trusty edit 1 --title "New title" --priority medium
trusty set-status --id 1 --status in-progress
trusty complete 1        # Mark as done
trusty complete 1 --all  # Complete task and all subtasks
```

### AI Features

```bash
# Decompose complex tasks (powered by Claude)
trusty decompose 1               # Claude breaks task into subtasks
trusty decompose 1 --preview     # Preview Claude's suggestions
trusty decompose 1 --count 6     # Ask Claude for 6 subtasks

# Prune stale tasks
trusty prune                     # Interactive review
trusty prune --dry-run          # Preview only
trusty prune --auto             # Apply all suggestions
```

### Organization

```bash
# Dependencies
trusty add-dep --task 2 --dep 1     # Task 2 depends on task 1
trusty remove-dep --task 2 --dep 1

# Subtasks
trusty add-subtask --task 1 "Subtask title"
trusty remove-subtask --task 1 --subtask 2

# Next task recommendation
trusty next              # Show next recommended task
trusty next --start      # Show and start working on it
```

## 🎨 Task List Display

Trusty provides a beautiful, informative display of your tasks:

```
╭──────────────────────────────────────────────────────╮
│   Project Dashboard                                  │
│   Tasks Progress: ████████████░░░░░░░░░░░░░░░░░░     │
│   43% Complete                                       │
│   Done: 13  In Progress: 1  Pending: 16              │
╰──────────────────────────────────────────────────────╯
```

## 🤖 Claude Code Integration

### Installing the Trusty Agent

The Trusty agent enables Claude Code to understand and manage your project tasks:

```bash
# Install globally (recommended for personal use)
trusty add-agent --scope global

# Install locally (for team projects)
trusty add-agent --scope local

# Customize the agent
trusty add-agent --scope global --name my-trusty --model opus --color blue
```

Once installed, you can ask Claude Code to:
- "Show me my current tasks"
- "What should I work on next?"
- "Break down this complex feature into subtasks"
- "Mark task #5 as complete"

## 🔧 Configuration

Trusty stores tasks in `.trusty/tasks/` in your project directory. This makes it easy to:
- Share task lists with your team via git
- Keep tasks separate per project
- Back up your task history

### Agent Configuration

- **Global agents**: Installed in `~/.claude/agents/`
- **Local agents**: Installed in `.claude/agents/` in your project
- Agents are configured with model selection (opus/sonnet) and custom colors

### Prune History

The prune command remembers previous suggestions using exponential backoff, stored in `.trusty/prune_history.json`.

## 🧪 Testing

Trusty comes with a comprehensive test suite:

```bash
# Run all tests
cargo test --all

# Run specific test module
cargo test prune::tests
```

## 🦀 Why Rust?

Trusty is built in Rust for several key reasons:

- **Speed**: Instant startup and response times - no waiting for interpreters or VMs
- **Reliability**: Rust's type system and memory safety prevent crashes and data corruption
- **Single Binary**: Compiles to a single executable with no runtime dependencies
- **Cross-Platform**: Works seamlessly on Linux, macOS, and Windows
- **Low Resource Usage**: Minimal memory footprint, perfect for running alongside development tools
- **Data Integrity**: Strong guarantees that your task data won't be corrupted

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request. Make sure to:

1. Add tests for new functionality
2. Run `cargo test --all` before committing
3. Follow Rust best practices and idioms
4. Update documentation as needed

## 📄 License

This project is licensed under the MIT License - see the LICENSE file for details.

## 🙏 Acknowledgments

- Built with [Claude](https://claude.ai) by Anthropic
- Initial inspiration from [task-master-ai](https://www.task-master.dev) - a great task management tool that showed the potential of AI-assisted task organization
- Uses the excellent [clap](https://github.com/clap-rs/clap) CLI framework
- Inspired by modern task management needs of developers

---

**Pro tip**: Use `trusty demo` to see an interactive demonstration of all features!
