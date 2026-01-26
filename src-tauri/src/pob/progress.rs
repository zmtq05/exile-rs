use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Serialize;
use specta::Type;
use tauri_specta::Event;

// ============================================================================
// Progress Sink Abstraction
// ============================================================================

/// Trait for emitting progress events. Enables testing without Tauri runtime.
pub trait ProgressSink: Send + Sync {
    fn emit(&self, progress: InstallProgress);
}

/// Reporter that holds task_id and sink. Clone-friendly for spawn_blocking.
#[derive(Clone)]
pub struct InstallReporter {
    task_id: String,
    sink: Arc<dyn ProgressSink>,
}

impl InstallReporter {
    pub fn new(task_id: impl Into<String>, sink: Arc<dyn ProgressSink>) -> Self {
        Self {
            task_id: task_id.into(),
            sink,
        }
    }

    /// Report progress with the stored task_id.
    pub fn report(&self, phase: InstallPhase, status: InstallStatus) {
        self.sink
            .emit(InstallProgress::new(&self.task_id, phase, status));
    }

    /// Get the task_id for this reporter.
    pub fn task_id(&self) -> &str {
        &self.task_id
    }
}

/// Tauri implementation of ProgressSink with throttling.
/// Throttles InProgress events to prevent IPC spam during fast operations.
pub struct TauriProgressSink {
    app: tauri::AppHandle,
    last_emit: Mutex<Instant>,
    throttle_duration: Duration,
}

impl TauriProgressSink {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            // Initialize in the past to allow first emit immediately
            last_emit: Mutex::new(Instant::now() - Duration::from_millis(200)),
            throttle_duration: Duration::from_millis(100),
        }
    }
}

impl ProgressSink for TauriProgressSink {
    fn emit(&self, progress: InstallProgress) {
        // Always emit non-InProgress events (Started, Completed, Failed, Cancelled)
        let should_throttle = matches!(progress.status, InstallStatus::InProgress { .. });

        if should_throttle {
            let mut last = self.last_emit.lock().unwrap();
            if last.elapsed() < self.throttle_duration {
                return; // Skip this emit
            }
            *last = Instant::now();
        }

        if let Err(e) = progress.emit(&self.app) {
            tracing::warn!(
                task_id = %progress.task_id,
                phase = ?progress.phase,
                status = ?progress.status,
                error = %e,
                "Failed to emit install progress event"
            );
        }
    }
}

// ============================================================================
// InstallProgress (Event Payload)
// ============================================================================

#[derive(Debug, Clone, Serialize, Type, Event)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub task_id: String,
    pub phase: InstallPhase,
    #[serde(flatten)]
    pub status: InstallStatus,
}

impl InstallProgress {
    pub fn new(task_id: impl Into<String>, phase: InstallPhase, status: InstallStatus) -> Self {
        Self {
            task_id: task_id.into(),
            phase,
            status,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum InstallStatus {
    Started {
        #[serde(skip_serializing_if = "Option::is_none")]
        total_size: Option<NonZeroU32>,
    },
    InProgress {
        percent: f64,
    },
    Completed,
    Failed {
        reason: String,
    },
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Downloading,
    Extracting,
    BackingUp,
    Moving,
    Restoring,
    Finalizing,
    Uninstalling,
}

#[test]
fn test_dummy() {
    let p = InstallProgress {
        task_id: "task1".to_string(),
        phase: InstallPhase::Moving,
        status: InstallStatus::InProgress { percent: 35.7 },
    };

    println!("{}", serde_json::to_string_pretty(&p).unwrap());
}
