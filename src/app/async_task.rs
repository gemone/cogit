//! Background task management — git operations run in background threads
//! and results are posted back via channels without blocking the UI.

use std::{
    collections::VecDeque,
    sync::mpsc::{channel, Receiver},
    thread,
};

use crate::gitops::Repository;

/// A completed background task with its result message.
#[derive(Debug, Clone)]
pub struct TaskResult {
    /// Short label shown in notification, e.g. "Commit" or "Push"
    pub label: String,
    /// Human-readable result or error message
    pub message: String,
    /// True if the operation succeeded
    pub ok: bool,
}

/// A running or completed background task.
#[derive(Debug)]
pub enum TaskHandle {
    /// Thread is running, receiver for the result
    Running(Receiver<TaskResult>),
    /// Task completed but hasn't been consumed yet
    Done(TaskResult),
}

impl TaskHandle {
    /// Check if the task has completed and take the result.
    pub fn try_take(&mut self) -> Option<TaskResult> {
        match self {
            TaskHandle::Running(rx) => match rx.try_recv() {
                Ok(result) => {
                    *self = TaskHandle::Done(result.clone());
                    Some(result)
                }
                Err(_) => None,
            },
            TaskHandle::Done(result) => {
                let r = result.clone();
                *self = TaskHandle::Done(r.clone());
                Some(r)
            }
        }
    }

    pub fn is_done(&self) -> bool {
        matches!(self, TaskHandle::Done(_))
    }
}

/// Manages all background tasks for the application.
#[derive(Debug)]
pub struct TaskManager {
    /// Pending tasks with an incrementing ID
    tasks: VecDeque<(usize, TaskHandle)>,
    /// Next task ID to assign
    next_id: usize,
    /// Label of the currently running task (for UI display)
    pub running_label: Option<String>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
            next_id: 1,
            running_label: None,
        }
    }

    /// Spawn a background task that runs `op()` in a thread.
    /// Returns the task ID.
    pub fn spawn<F>(&mut self, label: String, op: F) -> usize
    where
        F: FnOnce() -> TaskResult + Send + 'static,
    {
        let (tx, rx) = channel::<TaskResult>();

        thread::spawn(move || {
            let result = op();
            let _ = tx.send(result);
        });

        let id = self.next_id;
        self.next_id += 1;
        self.running_label = Some(label);
        self.tasks.push_back((id, TaskHandle::Running(rx)));
        id
    }

    /// Spawn a git commit operation in background.
    pub fn spawn_commit(&mut self, repo: Repository, message: String) -> usize {
        let label = "Commit".to_string();
        let label_clone = label.clone();
        self.spawn(label_clone, move || match repo.commit(&message) {
            Ok(_) => TaskResult {
                label: label.clone(),
                message: format!("Committed: {}", message),
                ok: true,
            },
            Err(e) => TaskResult {
                label: label.clone(),
                message: format!("Commit failed: {}", e),
                ok: false,
            },
        })
    }

    /// Spawn a git push operation in background.
    pub fn spawn_push(&mut self, repo: Repository, remote: String, branch: String) -> usize {
        let label = "Push".to_string();
        let label_clone = label.clone();
        self.spawn(label_clone, move || match repo.push(&remote, &branch) {
            Ok(output) => TaskResult {
                label: label.clone(),
                message: format!("Push: {}", output),
                ok: true,
            },
            Err(e) => TaskResult {
                label: label.clone(),
                message: format!("Push failed: {}", e),
                ok: false,
            },
        })
    }

    /// Spawn a git fetch operation in background.
    pub fn spawn_fetch(&mut self, repo: Repository, remote: Option<String>) -> usize {
        let label = "Fetch".to_string();
        let label_clone = label.clone();
        self.spawn(label_clone, move || match remote {
            Some(r) => match repo.fetch(&r) {
                Ok(output) => TaskResult {
                    label: label.clone(),
                    message: format!("Fetch {}: {}", r, output),
                    ok: true,
                },
                Err(e) => TaskResult {
                    label: label.clone(),
                    message: format!("Fetch {} failed: {}", r, e),
                    ok: false,
                },
            },
            None => match repo.fetch_all() {
                Ok(output) => TaskResult {
                    label: label.clone(),
                    message: format!("Fetch all: {}", output),
                    ok: true,
                },
                Err(e) => TaskResult {
                    label: label.clone(),
                    message: format!("Fetch all failed: {}", e),
                    ok: false,
                },
            },
        })
    }

    /// Spawn a git push_current operation in background.
    pub fn spawn_push_current(&mut self, repo: Repository) -> usize {
        let label = "Push".to_string();
        let label_clone = label.clone();
        self.spawn(label_clone, move || match repo.push_current() {
            Ok(output) => TaskResult {
                label: label.clone(),
                message: format!("Push: {}", output),
                ok: true,
            },
            Err(e) => TaskResult {
                label: label.clone(),
                message: format!("Push failed: {}", e),
                ok: false,
            },
        })
    }

    /// Spawn a git fetch_all operation in background.
    pub fn spawn_fetch_all(&mut self, repo: Repository) -> usize {
        let label = "Fetch".to_string();
        let label_clone = label.clone();
        self.spawn(label_clone, move || match repo.fetch_all() {
            Ok(output) => TaskResult {
                label: label.clone(),
                message: format!("Fetch all: {}", output),
                ok: true,
            },
            Err(e) => TaskResult {
                label: label.clone(),
                message: format!("Fetch all failed: {}", e),
                ok: false,
            },
        })
    }

    /// Drain all completed tasks and return their results.
    pub fn drain_completed(&mut self) -> Vec<TaskResult> {
        let mut results = Vec::new();
        for (_, handle) in &mut self.tasks {
            if let Some(result) = handle.try_take() {
                results.push(result);
            }
        }
        self.tasks.retain(|(_, handle)| !handle.is_done());
        if self.tasks.is_empty() {
            self.running_label = None;
        }
        results
    }

    pub fn has_running(&self) -> bool {
        !self.tasks.is_empty()
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
