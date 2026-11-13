pub use log;
pub use tokio;

pub mod config {
    pub fn is_disable_installation() -> bool {
        false
    }
    /// Korisnička verzija; LAGANI_PUNO=1 glumi verziju s punim postavkama.
    pub fn is_incoming_only() -> bool {
        std::env::var("LAGANI_PUNO").is_err()
    }
    pub const WIPS_SERVER_OPCIJA: &str = "wips-server";
    pub fn wips_ima_rezervni() -> bool {
        true
    }
}
