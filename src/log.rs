use core::ops::Index;
use serde::{de::DeserializeOwned, Serialize};
use super::Term;

pub type LogIndex = usize;

pub struct LogItem<Command> {
    term: Term,
    command: Command,
}

pub trait Log<Command>: Index<LogIndex> + Serialize + DeserializeOwned {
    fn append(&mut self, item: LogItem<Command>) -> LogIndex;
    fn truncate(&mut self, index: LogIndex);
    fn check(&self, index: LogIndex, term: Term) -> bool;
}
