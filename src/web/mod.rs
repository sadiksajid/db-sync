use serde::{Deserialize, Serialize};

pub mod auth;
pub mod config_store;
pub mod server;
pub mod state;
pub mod schedule_store;
pub mod scheduler;

pub use config_store::ConfigStore;
pub use schedule_store::ScheduleStore;
pub use scheduler::SchedulerService;
pub use server::start_web_server;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(message: impl Into<String>, data: Option<T>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }
}

