use serde::Serialize;


#[derive(Debug, Serialize)]
pub struct HealthCheck {
    pub database: bool,
    pub kv: bool
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self { database: true, kv: true }
    }
}