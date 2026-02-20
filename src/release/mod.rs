pub mod lock;

use chrono::Local;

pub struct Release {
    pub name: String,
}

impl Release {
    pub fn new() -> Self {
        let now = Local::now();
        Self {
            name: now.format("%Y%m%d-%H%M%S").to_string(),
        }
    }
}
