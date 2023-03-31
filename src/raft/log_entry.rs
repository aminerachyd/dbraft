#[derive(Clone)]
pub struct LogEntry {
    pub term: u32,
    pub command: String,
}
