use std::sync::Mutex;

use super::ToolResult;

#[derive(Debug, Clone)]
pub struct TodoItem {
    pub id: usize,
    pub content: String,
    pub status: TodoStatus,
    pub priority: TodoPriority,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Done,
}

#[derive(Debug, Clone)]
pub enum TodoPriority {
    High,
    Medium,
    Low,
}

impl TodoStatus {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "in_progress" | "inprogress" | "doing" | "active" => TodoStatus::InProgress,
            "done" | "completed" | "finished" => TodoStatus::Done,
            _ => TodoStatus::Pending,
        }
    }
    fn symbol(&self) -> &'static str {
        match self {
            TodoStatus::Pending    => "[ ]",
            TodoStatus::InProgress => "[~]",
            TodoStatus::Done       => "[✓]",
        }
    }
}

impl TodoPriority {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "high" | "h" | "critical" | "urgent" => TodoPriority::High,
            "low" | "l" | "minor" => TodoPriority::Low,
            _ => TodoPriority::Medium,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            TodoPriority::High   => "High",
            TodoPriority::Medium => "Med ",
            TodoPriority::Low    => "Low ",
        }
    }
}

static TODOS: Mutex<Vec<TodoItem>> = Mutex::new(Vec::new());

pub fn todo_write(todos_json: &serde_json::Value) -> ToolResult {
    let arr = match todos_json.as_array() {
        Some(a) => a,
        None => return ToolResult::err("todos must be a JSON array"),
    };

    let mut items = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let content = match item.get("content").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => return ToolResult::err(format!("Item {i}: missing 'content' field")),
        };
        let status   = TodoStatus::from_str(item.get("status").and_then(|v| v.as_str()).unwrap_or("pending"));
        let priority = TodoPriority::from_str(item.get("priority").and_then(|v| v.as_str()).unwrap_or("medium"));
        items.push(TodoItem { id: i + 1, content, status, priority });
    }

    let count = items.len();
    *TODOS.lock().unwrap() = items;
    ToolResult::ok(format!("Todo list updated with {count} item(s). Use todo_read to view."))
}

pub fn todo_read() -> ToolResult {
    let todos = TODOS.lock().unwrap();
    if todos.is_empty() {
        return ToolResult::ok("No todos. Use todo_write to create a task list.".to_string());
    }
    let lines: Vec<String> = todos.iter().map(|t| {
        format!("{}  {}  #{}  {}", t.status.symbol(), t.priority.label(), t.id, t.content)
    }).collect();
    ToolResult::ok(lines.join("\n"))
}
