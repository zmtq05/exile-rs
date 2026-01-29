use std::{
    num::NonZeroU32,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use serde::Serialize;
use specta::Type;
use tauri_specta::Event;

const PROGRESS_THROTTLE_MS: u64 = 100;

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
            last_emit: Mutex::new(Instant::now() - Duration::from_millis(2 * PROGRESS_THROTTLE_MS)),
            throttle_duration: Duration::from_millis(PROGRESS_THROTTLE_MS),
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
    Preparing,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex as StdMutex};

    /// Mock ProgressSink for testing - captures all emitted progress
    struct MockProgressSink {
        events: Arc<StdMutex<Vec<InstallProgress>>>,
    }

    impl MockProgressSink {
        fn new() -> Self {
            Self {
                events: Arc::new(StdMutex::new(Vec::new())),
            }
        }

        fn get_events(&self) -> Vec<InstallProgress> {
            self.events.lock().unwrap().clone()
        }

        #[allow(dead_code)]
        fn event_count(&self) -> usize {
            self.events.lock().unwrap().len()
        }
    }

    impl ProgressSink for MockProgressSink {
        fn emit(&self, progress: InstallProgress) {
            self.events.lock().unwrap().push(progress);
        }
    }

    #[test]
    fn test_install_reporter_basic() {
        let sink = Arc::new(MockProgressSink::new());
        let reporter = InstallReporter::new("test_task", sink.clone());

        assert_eq!(reporter.task_id(), "test_task");

        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::Started { total_size: None },
        );

        let events = sink.get_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].task_id, "test_task");
        assert!(matches!(events[0].phase, InstallPhase::Downloading));
    }

    #[test]
    fn test_install_reporter_multiple_phases() {
        let sink = Arc::new(MockProgressSink::new());
        let reporter = InstallReporter::new("multi_phase", sink.clone());

        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::Started { total_size: None },
        );
        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::InProgress { percent: 50.0 },
        );
        reporter.report(InstallPhase::Downloading, InstallStatus::Completed);
        reporter.report(
            InstallPhase::Extracting,
            InstallStatus::Started { total_size: None },
        );

        let events = sink.get_events();
        assert_eq!(events.len(), 4);

        // Verify phase transitions
        assert!(matches!(events[0].phase, InstallPhase::Downloading));
        assert!(matches!(events[0].status, InstallStatus::Started { .. }));
        assert!(matches!(events[1].status, InstallStatus::InProgress { .. }));
        assert!(matches!(events[2].status, InstallStatus::Completed));
        assert!(matches!(events[3].phase, InstallPhase::Extracting));
    }

    #[test]
    fn test_install_reporter_clone() {
        let sink = Arc::new(MockProgressSink::new());
        let reporter = InstallReporter::new("clone_test", sink.clone());
        let reporter_clone = reporter.clone();

        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::Started { total_size: None },
        );
        reporter_clone.report(InstallPhase::Downloading, InstallStatus::Completed);

        let events = sink.get_events();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn test_install_progress_serialization() {
        let progress = InstallProgress {
            task_id: "ser_test".to_string(),
            phase: InstallPhase::Moving,
            status: InstallStatus::InProgress { percent: 75.5 },
        };

        let json = serde_json::to_value(&progress).unwrap();
        assert_eq!(json["taskId"], "ser_test");
        assert_eq!(json["phase"], "moving");
        assert_eq!(json["status"], "inProgress");
        assert_eq!(json["percent"], 75.5);
    }

    #[test]
    fn test_install_status_variants() {
        let test_cases = vec![
            (
                InstallStatus::Started {
                    total_size: NonZeroU32::new(1000),
                },
                "started",
            ),
            (InstallStatus::InProgress { percent: 42.0 }, "inProgress"),
            (InstallStatus::Completed, "completed"),
            (
                InstallStatus::Failed {
                    reason: "test error".to_string(),
                },
                "failed",
            ),
            (InstallStatus::Cancelled, "cancelled"),
        ];

        for (status, expected_tag) in test_cases {
            let progress = InstallProgress {
                task_id: "status_test".to_string(),
                phase: InstallPhase::Preparing,
                status,
            };

            let json = serde_json::to_value(&progress).unwrap();
            assert_eq!(json["status"], expected_tag);
        }
    }

    #[test]
    fn test_install_phase_serialization() {
        let phases = vec![
            (InstallPhase::Downloading, "downloading"),
            (InstallPhase::Extracting, "extracting"),
            (InstallPhase::BackingUp, "backingUp"),
            (InstallPhase::Moving, "moving"),
            (InstallPhase::Restoring, "restoring"),
            (InstallPhase::Finalizing, "finalizing"),
            (InstallPhase::Uninstalling, "uninstalling"),
            (InstallPhase::Preparing, "preparing"),
        ];

        for (phase, expected) in phases {
            let progress = InstallProgress {
                task_id: "phase_test".to_string(),
                phase,
                status: InstallStatus::Completed,
            };

            let json = serde_json::to_value(&progress).unwrap();
            assert_eq!(json["phase"], expected);
        }
    }

    #[test]
    fn test_progress_throttle_constant() {
        // Verify constant is set
        assert_eq!(PROGRESS_THROTTLE_MS, 100);
    }
}
