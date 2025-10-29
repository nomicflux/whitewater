use std::sync::Arc;
use tokio::sync::Mutex;

pub enum Command {
    AddUser { name: String, email: String },
}

pub trait ToCommand {
    fn to_command(&self) -> Command;
}

pub struct LogEntry {
    index: u32,
    term: u32,
    command: Command,
}

pub struct Log {
    latest_seen: u32,
    latest_applied: u32,
    entries: Vec<LogEntry>,
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
