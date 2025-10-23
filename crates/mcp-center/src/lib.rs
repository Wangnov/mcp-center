//! Core utilities shared across mcp-center crates.

pub mod config;
pub mod error;
pub mod paths;
pub mod project;

pub use config::{ServerConfig, ServerDefinition, ServerProtocol};
pub use error::CoreError;
pub use paths::{Layout, default_root};
pub use project::{ProjectId, ProjectRecord, ProjectRegistry};

// CLI 模块
#[path = "cli/i18n.rs"]
pub mod cli_i18n;

// Bridge 模块
pub mod bridge {
    #[path = "../bridge/control.rs"]
    pub mod control;

    // Connect command (bridge entry point)
    pub mod connect;
}

// Daemon 模块
pub mod daemon {
    #[path = "../daemon/control.rs"]
    pub mod control;
    #[path = "../daemon/host.rs"]
    pub mod host;
    #[path = "../daemon/rpc.rs"]
    pub mod rpc;
    #[path = "../daemon/server_manager.rs"]
    pub mod server_manager;

    // Serve command (daemon entry point)
    pub mod serve;
}

// Web / HTTP 模块
pub mod web {
    pub mod http;
}
