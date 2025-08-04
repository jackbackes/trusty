# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-08-04

### Added
- Initial release of Trusty task manager
- Core task management: add, edit, delete, complete tasks
- Hierarchical task support with subtasks
- Task dependencies tracking
- Priority levels (high, medium, low) and complexity tracking
- AI-powered features:
  - Task decomposition using Claude AI
  - Natural language task creation
  - Intelligent task pruning with exponential backoff
- Smart task filtering:
  - Hide completed tasks older than 5 minutes by default
  - `--all`, `--completed`, and `--recent N` flags
- Next task recommendation system
- Beautiful terminal UI with progress visualization
- Interactive demo mode (`trusty demo`)
- Comprehensive test suite
- Git-friendly JSON storage