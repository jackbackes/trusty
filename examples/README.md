# Examples

This directory contains examples of using Trusty.

## Quick Start Example

```bash
# Initialize a new project
$ trusty init
✅ Trusty initialized successfully!

# Add some tasks
$ trusty add "Set up CI/CD pipeline" --priority high
✅ Created task #1: Set up CI/CD pipeline

$ trusty add "Write unit tests for auth module" --priority high
✅ Created task #2: Write unit tests for auth module

$ trusty add "Update documentation" --priority medium
✅ Created task #3: Update documentation

# View your tasks
$ trusty list

# Get the next recommended task
$ trusty next
🔥 Next Task to Work On: #1 - Set up CI/CD pipeline

# Start working on it
$ trusty next --start
✅ Task #1 is now in progress!

# Decompose a complex task
$ trusty decompose 1 --count 3
🤖 Decomposing task 'Set up CI/CD pipeline' into 3 subtasks...
✅ Created subtask #4: Configure GitHub Actions workflow
✅ Created subtask #5: Set up automated testing
✅ Created subtask #6: Add deployment scripts

# Complete a task
$ trusty complete 4
✅ Updated 1 task to status: ● done

# Prune stale tasks
$ trusty prune --dry-run
🧹 Found 1 task(s) that may need attention:
1. Cancel #3 - Update documentation
   Reason: Low priority task ignored for 60 days
```

## Demo Recording

To see Trusty in action, run:

```bash
trusty demo
```

This will walk you through all the main features interactively.