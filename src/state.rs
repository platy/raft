use super::log::LogIndex;
use super::{ServerId, Term};

/// Persistent state on all servers:(Updated on stable storage before responding to RPCs)
pub struct PersistentState<Log> {
    current_term: Term,
    voted_for: ServerId,
    log: Log,
}

/// Server states. Followers only respond to requestsfrom other servers. If a follower receives no communication,it becomes a candidate and initiates an election. A candidatethat receives votes from a majority of the full cluster becomesthe new leader. Leaders typically operate until they fail.
pub enum States {
    Follower,
    Candidate,
    /// Volatile state on leaders:(Reinitialized after election)
    Leader {
        next_index: Vec<LogIndex>,
        match_index: Vec<LogIndex>,
    },
}

/// Volatile state on all servers
pub struct ServerState<Log> {
    state: States,
    persistent_state: PersistentState<Log>,
    commit_index: LogIndex,
    last_applied: LogIndex,
}
