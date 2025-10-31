use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Command {
    AddUser { name: String, email: String },
}

pub trait ToCommand {
    fn to_command(&self) -> Command;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogEntry {
    pub index: u32,
    pub term: u32,
    pub command: Command,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Log {
    pub latest_seen: u32,
    pub latest_applied: u32,
    pub entries: Vec<LogEntry>,
}

impl Log {
    pub fn new() -> Log {
        Log {
            latest_seen: 0,
            latest_applied: 0,
            entries: Vec::new(),
        }
    }

    fn update_log(&mut self, term: u32, command: Command) {
        let latest_seen = self.latest_seen + 1;
        self.entries.push(LogEntry {
            index: latest_seen,
            term,
            command,
        });
        self.latest_seen = latest_seen;
    }
}

pub async fn add_to_log<T>(log: Arc<Mutex<Log>>, term: u32, entry: &T)
where
    T: ToCommand,
{
    let mut log = log.lock().await;
    log.update_log(term, entry.to_command());
}
