use trusty::storage::TaskStorage;
use trusty::task::{Task, Priority, TaskStatus};

fn setup_test_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn test_storage_save_and_load() {
    let temp_dir = setup_test_dir();
    let storage = TaskStorage::new(temp_dir.path().to_path_buf()).unwrap();
    
    let task = Task::new(1, "Test Task".to_string(), "Description".to_string(), Priority::High);
    
    // Save task
    storage.save_task(&task).unwrap();
    
    // Load task
    let loaded_task = storage.load_task(1).unwrap();
    
    assert_eq!(loaded_task.id, task.id);
    assert_eq!(loaded_task.title, task.title);
    assert_eq!(loaded_task.description, task.description);
    assert_eq!(loaded_task.priority, task.priority);
}

#[test]
fn test_storage_list_all_tasks() {
    let temp_dir = setup_test_dir();
    let storage = TaskStorage::new(temp_dir.path().to_path_buf()).unwrap();
    
    // Save multiple tasks
    let task1 = Task::new(1, "Task 1".to_string(), "".to_string(), Priority::High);
    let task2 = Task::new(2, "Task 2".to_string(), "".to_string(), Priority::Medium);
    let task3 = Task::new(3, "Task 3".to_string(), "".to_string(), Priority::Low);
    
    storage.save_task(&task1).unwrap();
    storage.save_task(&task2).unwrap();
    storage.save_task(&task3).unwrap();
    
    // List all tasks
    let tasks = storage.list_all_tasks().unwrap();
    
    assert_eq!(tasks.len(), 3);
    assert!(tasks.iter().any(|t| t.id == 1 && t.title == "Task 1"));
    assert!(tasks.iter().any(|t| t.id == 2 && t.title == "Task 2"));
    assert!(tasks.iter().any(|t| t.id == 3 && t.title == "Task 3"));
}

#[test]
fn test_storage_delete_task() {
    let temp_dir = setup_test_dir();
    let storage = TaskStorage::new(temp_dir.path().to_path_buf()).unwrap();
    
    let task = Task::new(1, "Test Task".to_string(), "".to_string(), Priority::High);
    
    // Save task
    storage.save_task(&task).unwrap();
    
    // Verify it exists
    assert!(storage.load_task(1).is_ok());
    
    // Delete task
    storage.delete_task(1).unwrap();
    
    // Verify it's gone
    assert!(storage.load_task(1).is_err());
}

#[test]
fn test_storage_update_task() {
    let temp_dir = setup_test_dir();
    let storage = TaskStorage::new(temp_dir.path().to_path_buf()).unwrap();
    
    let mut task = Task::new(1, "Original Title".to_string(), "".to_string(), Priority::Low);
    storage.save_task(&task).unwrap();
    
    // Update task
    task.title = "Updated Title".to_string();
    task.set_status(TaskStatus::InProgress);
    storage.save_task(&task).unwrap();
    
    // Load and verify
    let loaded_task = storage.load_task(1).unwrap();
    assert_eq!(loaded_task.title, "Updated Title");
    assert_eq!(loaded_task.status, TaskStatus::InProgress);
}

#[test]
fn test_load_nonexistent_task() {
    let temp_dir = setup_test_dir();
    let storage = TaskStorage::new(temp_dir.path().to_path_buf()).unwrap();
    
    let result = storage.load_task(999);
    assert!(result.is_err());
}