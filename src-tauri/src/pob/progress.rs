use std::num::NonZeroU32;

use serde::Serialize;
use specta::Type;
use tauri_specta::Event;

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

    pub fn derived(&self, status: InstallStatus) -> Self {
        Self {
            task_id: self.task_id.clone(),
            phase: self.phase,
            status,
        }
    }

    pub fn report(&self, app: &tauri::AppHandle) {
        if let Err(e) = self.emit(app) {
            tracing::warn!(
                task_id = %self.task_id,
                phase = ?self.phase,
                status = ?self.status,
                error = %e,
                "Failed to emit install progress event"
            );
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
