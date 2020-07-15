use super::log::LogIndex;
use super::{ServerId, Term};

/// Persistent state on all servers:(Updated on stable storage before responding to RPCs)
#[derive(Default)]
pub struct PersistentState<Log> {
    current_term: Term,
    voted_for: Option<ServerId>,
    log: Log,
}

impl<Log> PersistentState<Log> {
    /// # Panics
    /// In case the term tries to decrease
    fn set_current_term(&mut self, current_term: Term) {
        assert!(
            current_term >= self.current_term,
            "Current term must increase monotonically, tried to decrease from {} to {}",
            self.current_term,
            current_term
        );
        self.current_term = current_term;
    }
}

#[derive(Debug, PartialEq)]
pub struct LeaderState {
    next_index: Vec<LogIndex>,
    match_index: Vec<LogIndex>,
}

impl LeaderState {
    fn new(num_servers: usize, last_log_index: LogIndex) -> LeaderState {
        let mut next_index = Vec::new();
        next_index.resize(num_servers, last_log_index + 1);
        let mut match_index = Vec::new();
        match_index.resize(num_servers, 0);
        LeaderState {
            next_index,
            match_index,
        }
    }

    /// # Panics
    /// In case the match index tries to decrease
    fn set_match_index(&mut self, server: ServerId, match_index: LogIndex) {
        assert!(
            match_index >= self.match_index[server as usize],
            "Match index must increase monotonically, tried to decrease from {} to {}",
            self.match_index[server as usize],
            match_index
        );
        self.match_index[server as usize] = match_index;
    }
}

/// Server states. Followers only respond to requestsfrom other servers. If a follower receives no communication,it becomes a candidate and initiates an election. A candidatethat receives votes from a majority of the full cluster becomesthe new leader. Leaders typically operate until they fail.
#[derive(Debug, PartialEq)]
pub enum States {
    Follower,
    Candidate,
    /// Volatile state on leaders:(Reinitialized after election)
    Leader(LeaderState),
}

impl Default for States {
    fn default() -> Self {
        Self::Follower
    }
}

/// Volatile state on all servers
#[derive(Default)]
pub struct ServerState<Log> {
    state: States,
    persistent_state: PersistentState<Log>,
    commit_index: LogIndex,
    last_applied: LogIndex,
}

impl<Log> ServerState<Log> {
    /// # Panics
    /// In case the term tries to decrease
    fn set_commit_index(&mut self, commit_index: LogIndex) {
        assert!(
            commit_index >= self.commit_index,
            "Commit index must increase monotonically, tried to decrease from {} to {}",
            self.commit_index,
            commit_index
        );
        self.commit_index = commit_index;
    }

    /// # Panics
    /// In case the term tries to decrease
    fn set_last_applied(&mut self, last_applied: LogIndex) {
        assert!(
            last_applied >= self.last_applied,
            "Last applied must increase monotonically, tried to decrease from {} to {}",
            self.last_applied,
            last_applied
        );
        self.last_applied = last_applied;
    }
}

#[cfg(test)]
mod test_persistent {
    use super::*;

    #[test]
    fn default() {
        let persistant_state = PersistentState::<Vec<u8>>::default();
        assert_eq!(persistant_state.current_term, 0);
        assert_eq!(persistant_state.voted_for, None);
    }

    #[test]
    fn current_term_increases() {
        let mut persistant_state = PersistentState::<Vec<u8>>::default();
        persistant_state.set_current_term(2);
        assert_eq!(persistant_state.current_term, 2);
    }

    #[test]
    #[should_panic]
    fn current_term_increases_monotonically() {
        let mut persistant_state = PersistentState::<Vec<u8>>::default();
        persistant_state.set_current_term(2);
        persistant_state.set_current_term(1);
    }
}

#[cfg(test)]
mod test_volatile_state {
    use super::*;

    #[test]
    fn default() {
        let volatile_state = ServerState::<Vec<u8>>::default();
        assert_eq!(volatile_state.state, States::Follower);
        assert_eq!(volatile_state.commit_index, 0);
        assert_eq!(volatile_state.last_applied, 0);
    }

    #[test]
    fn commit_index_increases() {
        let mut volatile_state = ServerState::<Vec<u8>>::default();
        volatile_state.set_commit_index(2);
        assert_eq!(volatile_state.commit_index, 2);
    }

    #[test]
    #[should_panic]
    fn commit_index_increases_monotonically() {
        let mut volatile_state = ServerState::<Vec<u8>>::default();
        volatile_state.set_commit_index(2);
        volatile_state.set_commit_index(1);
    }

    #[test]
    fn last_applied_increases() {
        let mut volatile_state = ServerState::<Vec<u8>>::default();
        volatile_state.set_last_applied(2);
        assert_eq!(volatile_state.last_applied, 2);
    }

    #[test]
    #[should_panic]
    fn last_applied_increases_monotonically() {
        let mut volatile_state = ServerState::<Vec<u8>>::default();
        volatile_state.set_last_applied(2);
        volatile_state.set_last_applied(1);
    }
}

#[cfg(test)]
mod test_leader_state {
    use super::*;

    #[test]
    fn initial() {
        let leader_state = LeaderState::new(2, 4);
        assert_eq!(leader_state.next_index.len(), 2);
        assert!(leader_state.next_index.into_iter().all(|e| e == 5));
        assert_eq!(leader_state.match_index.len(), 2);
        assert!(leader_state.match_index.into_iter().all(|e| e == 0));
    }

    #[test]
    fn match_index_increases() {
        let mut leader_state = LeaderState::new(2, 4);
        leader_state.set_match_index(0, 7);
        assert_eq!(leader_state.match_index[0], 7);
    }

    #[test]
    #[should_panic]
    fn match_index_increases_monotonically() {
        let mut leader_state = LeaderState::new(2, 4);
        leader_state.set_match_index(0, 2);
        leader_state.set_match_index(0, 1);
    }
}
