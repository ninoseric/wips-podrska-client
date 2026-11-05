pub use log;
pub use tokio;

pub mod config {
    pub fn is_disable_installation() -> bool {
        false
    }
}
