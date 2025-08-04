# 🚀 Trusty - AI-Powered Task Management for Developers

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Built with Claude](https://img.shields.io/badge/Built%20with-Claude-blueviolet?style=for-the-badge)](https://claude.ai)

**Trusty** is a modern command-line task manager designed specifically for developers. It leverages AI to help you decompose complex tasks, identify stale work, and maintain a clean, actionable task list.

## ✨ Features

### 🤖 AI-Powered Task Management
- **Smart Decomposition**: Break down complex tasks into manageable subtasks using Claude AI
- **Intelligent Pruning**: Automatically identify stale, completed, or irrelevant tasks
- **Natural Language Input**: Create tasks from natural language descriptions

### 📊 Advanced Task Tracking
- **Hierarchical Tasks**: Support for subtasks with automatic status aggregation
- **Dependencies**: Track task dependencies and see what's blocking your progress
- **Smart Filtering**: Hide old completed tasks by default, with flexible viewing options
- **Priority & Complexity**: Track task priority and complexity levels

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
```

### Prerequisites
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))
- Claude CLI (optional, for AI features): `npm install -g @anthropic-ai/claude-code`

## 🚀 Quick Start

```bash
# Initialize trusty in your project
trusty init

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
trusty add --prompt "Create a task for implementing OAuth2"  # AI-generated

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
# Decompose complex tasks
trusty decompose 1               # Break into subtasks
trusty decompose 1 --preview     # Preview without creating
trusty decompose 1 --count 6     # Create 6 subtasks

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
│   Done: 13  In Progress: 1  Pending: 16             │
╰──────────────────────────────────────────────────────╯
```

## 🔧 Configuration

Trusty stores tasks in `.trusty/tasks/` in your project directory. This makes it easy to:
- Share task lists with your team via git
- Keep tasks separate per project
- Back up your task history

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
- Inspired by modern task management needs of developers
- Uses the excellent [clap](https://github.com/clap-rs/clap) CLI framework

---

**Pro tip**: Use `trusty demo` to see an interactive demonstration of all features!