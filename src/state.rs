use super::log;
use super::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RequestVoteRequest, RequestVoteResponse,
};
use super::state_machine::Receiver;
use super::{ServerId, Term};
use core::cmp::Ordering;

/// Persistent state on all servers:(Updated on stable storage before responding to RPCs)
// @todo operations here need to be persisted to disk and therefore async
#[derive(Default)]
pub struct Persistent<Log> {
    current_term: Term,
    voted_for: Option<ServerId>,
    log: Log,
}

impl<Log> Persistent<Log> {
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
pub struct Leader {
    next_index: Vec<log::Index>,
    match_index: Vec<log::Index>,
}

impl Leader {
    fn new(num_servers: ServerId, last_log_index: log::Index) -> Leader {
        let mut next_index = Vec::new();
        next_index.resize(num_servers as usize, last_log_index + 1);
        let mut match_index = Vec::new();
        match_index.resize(num_servers as usize, 0);
        Leader {
            next_index,
            match_index,
        }
    }

    /// # Panics
    /// In case the match index tries to decrease
    fn set_match_index(&mut self, server: ServerId, match_index: log::Index) {
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
    Leader(Leader),
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
    persistent_state: Persistent<Log>,
    commit_index: log::Index,
    last_applied: log::Index,
}

impl<Log: log::Log> ServerState<Log> {
    pub fn current_term(&self) -> Term {
        self.persistent_state.current_term
    }

    pub fn voted_for(&self) -> Option<ServerId> {
        self.persistent_state.voted_for
    }

    pub fn commit_index(&self) -> log::Index {
        self.commit_index
    }

    pub fn is_follower(&self) -> bool {
        States::Follower == self.state
    }

    pub fn is_candidate(&self) -> bool {
        States::Candidate == self.state
    }

    pub fn is_leader(&self) -> bool {
        if let States::Leader(_) = self.state {
            true
        } else {
            false
        }
    }

    pub fn follow_new_term(&mut self, term: Term) {
        self.persistent_state.set_current_term(term);
        self.state = States::Follower;
    }

    /// No messages have been received over the election timeout.
    /// # Followers (§5.2):
    /// -- If election timeout elapses without receiving AppendEntriesRPC from current leader or granting vote to candidate: convert to candidate
    /// # Candidates (§5.2):
    /// - On conversion to candidate, start election:
    /// -- Increment currentTerm
    /// -- Vote for self
    /// -- Reset election timer (to be handled by caller)
    /// -- Send RequestVote RPCs to all other servers
    pub fn start_election(&mut self, server_id: ServerId) -> RequestVoteRequest {
        // convert to candidate
        self.state = States::Candidate;
        // Increment currentTerm
        self.persistent_state.current_term += 1;
        // Vote for self
        self.persistent_state.voted_for = Some(server_id);
        // Reset election timer - job of caller
        // Send RequestVote RPCs to all other servers
        RequestVoteRequest {
            term: self.persistent_state.current_term,
            candidate_id: server_id,
            last_log_index: self.persistent_state.log.last_log_index(),
            last_log_term: self.persistent_state.log.last_log_term(),
        }
    }

    pub fn become_leader(&mut self, num_servers: ServerId) {
        self.state = States::Leader(Leader::new(
            num_servers,
            self.persistent_state.log.last_log_index(),
        ))
    }

    /// # Panics
    /// In case the term tries to decrease
    pub fn set_commit_index(&mut self, commit_index: log::Index) {
        assert!(
            commit_index >= self.commit_index,
            "Commit index must increase monotonically, tried to decrease from {} to {}",
            self.commit_index,
            commit_index
        );
        self.commit_index = commit_index;
    }

    pub fn last_applied(&self) -> log::Index {
        self.last_applied
    }

    /// # Panics
    /// In case the term tries to decrease
    pub fn set_last_applied(&mut self, last_applied: log::Index) {
        assert!(
            last_applied >= self.last_applied,
            "Last applied must increase monotonically, tried to decrease from {} to {}",
            self.last_applied,
            last_applied
        );
        self.last_applied = last_applied;
    }

    /// Invoked by leader to replicate log entries (§5.3); also used as heartbeat (§5.2).
    /// 1.  Reply false if term < currentTerm (§5.1)
    /// 2.  Reply false if log doesn’t contain an entry at prevLogIndex whose term matches prevLogTerm (§5.3)
    /// 3.  If an existing entry conflicts with a new one (same index but different terms), delete the existing entry and all that follow it (§5.3)
    /// 4.  Append any new entries not already in the log
    /// 5.  If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)
    pub fn receive_append_entries<LogEntries: IntoIterator<Item = log::Item<Log::Command>>>(
        &mut self,
        req: AppendEntriesRequest<Log::Command, LogEntries>,
    ) -> AppendEntriesResponse {
        AppendEntriesResponse {
            success: self.receive_append_entries_int(req),
            term: self.persistent_state.current_term,
        }
    }

    fn receive_append_entries_int<LogEntries: IntoIterator<Item = log::Item<Log::Command>>>(
        &mut self,
        req: AppendEntriesRequest<Log::Command, LogEntries>,
    ) -> bool {
        // 1.  Reply false if term < currentTerm (§5.1)
        if req.term < self.persistent_state.current_term {
            return false;
        }
        // 2.  Reply false if log doesn’t contain an entry at prevLogIndex whose term matches prevLogTerm (§5.3)
        if req.prev_log_index != 0
            && !self
                .persistent_state
                .log
                .log_term_matches(req.prev_log_index, req.prev_log_term)
        {
            return false;
        }
        // 3.  If an existing entry conflicts with a new one (same index but different terms), delete the existing entry and all that follow it (§5.3)
        // 4.  Append any new entries not already in the log
        let last_new_entry_index = self
            .persistent_state
            .log
            .truncate_if_different_and_append(req.prev_log_index, req.entries);
        // 5.  If leaderCommit > commitIndex, set commitIndex = min(leaderCommit, index of last new entry)
        if req.leader_commit > self.commit_index {
            self.set_commit_index(std::cmp::min(req.leader_commit, last_new_entry_index))
        }
        true
    }

    pub fn receive_request_vote(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        // 1.  Reply false if term < currentTerm (§5.1)
        let vote_granted = if req.term < self.persistent_state.current_term {
            false
        }
        // 2.  If votedFor is null or candidateId, and candidate’s log is at least as up-to-date as receiver’s log, grant vote (§5.2, §5.4)
        else if (self.persistent_state.voted_for.is_none()
            || self.persistent_state.voted_for == Some(req.candidate_id))
            && self
                .persistent_state
                .log
                .cmp(req.last_log_index, req.last_log_term)
                != Ordering::Greater
        {
            self.persistent_state.voted_for = Some(req.candidate_id);
            true
        } else {
            false
        };
        RequestVoteResponse {
            vote_granted,
            term: self.persistent_state.current_term,
        }
    }

    pub fn apply_commited(&mut self, receiver: &mut Receiver<Log::Command>) {
        while self.commit_index > self.last_applied {
            self.last_applied += 1;
            receiver(self.persistent_state.log.get_command(self.last_applied));
        }
    }
}

#[cfg(test)]
impl<Log: log::Log> ServerState<Log> {
    pub fn set_log(&mut self, log: Log) {
        self.persistent_state.log = log
    }
}

#[cfg(test)]
mod test_persistent_state {
    use super::*;

    #[test]
    fn default() {
        let persistant_state = Persistent::<Vec<u8>>::default();
        assert_eq!(persistant_state.current_term, 0);
        assert_eq!(persistant_state.voted_for, None);
    }

    #[test]
    fn current_term_increases() {
        let mut persistant_state = Persistent::<Vec<u8>>::default();
        persistant_state.set_current_term(2);
        assert_eq!(persistant_state.current_term, 2);
    }

    #[test]
    #[should_panic]
    fn current_term_increases_monotonically() {
        let mut persistant_state = Persistent::<Vec<u8>>::default();
        persistant_state.set_current_term(2);
        persistant_state.set_current_term(1);
    }
}

#[cfg(test)]
mod test_volatile_state {
    use super::*;

    type TestServerState = ServerState<log::InVec<u8>>;

    #[test]
    fn default() {
        let volatile_state = TestServerState::default();
        assert_eq!(volatile_state.state, States::Follower);
        assert_eq!(volatile_state.commit_index, 0);
        assert_eq!(volatile_state.last_applied, 0);
    }

    #[test]
    fn commit_index_increases() {
        let mut volatile_state = TestServerState::default();
        volatile_state.set_commit_index(2);
        assert_eq!(volatile_state.commit_index, 2);
    }

    #[test]
    #[should_panic]
    fn commit_index_increases_monotonically() {
        let mut volatile_state = TestServerState::default();
        volatile_state.set_commit_index(2);
        volatile_state.set_commit_index(1);
    }

    #[test]
    fn last_applied_increases() {
        let mut volatile_state = TestServerState::default();
        volatile_state.set_last_applied(2);
        assert_eq!(volatile_state.last_applied, 2);
    }

    #[test]
    #[should_panic]
    fn last_applied_increases_monotonically() {
        let mut volatile_state = TestServerState::default();
        volatile_state.set_last_applied(2);
        volatile_state.set_last_applied(1);
    }
}

#[cfg(test)]
mod test_leader_state {
    use super::*;

    #[test]
    fn initial() {
        let leader_state = Leader::new(2, 4);
        assert_eq!(leader_state.next_index.len(), 2);
        assert!(leader_state.next_index.into_iter().all(|e| e == 5));
        assert_eq!(leader_state.match_index.len(), 2);
        assert!(leader_state.match_index.into_iter().all(|e| e == 0));
    }

    #[test]
    fn match_index_increases() {
        let mut leader_state = Leader::new(2, 4);
        leader_state.set_match_index(0, 7);
        assert_eq!(leader_state.match_index[0], 7);
    }

    #[test]
    #[should_panic]
    fn match_index_increases_monotonically() {
        let mut leader_state = Leader::new(2, 4);
        leader_state.set_match_index(0, 2);
        leader_state.set_match_index(0, 1);
    }
}

#[cfg(test)]
mod test_append_entries {
    use super::ServerState;
    use crate::log::{self, Item};
    use crate::rpc::AppendEntriesRequest;

    #[test]
    fn impl1() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        let req = AppendEntriesRequest {
            term: 1,
            leader_id: 0,
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(!res.success, "Receiver implementation 1 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_no_entry() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        let req = AppendEntriesRequest {
            term: 2,
            leader_id: 0,
            prev_log_index: 1,
            prev_log_term: 2,
            entries: vec![],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(!res.success, "Receiver implementation 2 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_no_term_match() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 2,
            leader_id: 0,
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(!res.success, "Receiver implementation 2 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl3_first_doesnt_match() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 3,
            leader_id: 0,
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![Item::new(3, 98)],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(
            server.persistent_state.log.len() <= 2,
            "Receiver implementation 3 failed"
        );
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl3_second_doesnt_match() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 3,
            leader_id: 0,
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![Item::new(2, 99), Item::new(3, 98)],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(
            server.persistent_state.log.len() <= 3,
            "Receiver implementation 3 failed"
        );
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl3_4() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 3,
            leader_id: 0,
            prev_log_index: 1,
            prev_log_term: 1,
            entries: vec![Item::new(3, 98)],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert!(
            server.persistent_state.log.len() <= 2,
            "Receiver implementation 3 failed"
        );
        assert_eq!(
            server.persistent_state.log.len(),
            2,
            "Receiver implementation 4 failed"
        );
        assert_eq!(
            server.persistent_state.log[1].term, 3,
            "Receiver implementation 4 failed"
        );
        assert_eq!(
            server.persistent_state.log[1].command, 98,
            "Receiver implementation 4 failed"
        );
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl4() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 3,
            leader_id: 0,
            prev_log_index: 3,
            prev_log_term: 2,
            entries: vec![Item::new(3, 98)],
            leader_commit: 0,
        };
        let res = server.receive_append_entries(req);
        assert_eq!(
            server.persistent_state.log.len(),
            4,
            "Receiver implementation 4 failed"
        );
        assert_eq!(
            server.persistent_state.log[3].term, 3,
            "Receiver implementation 4 failed"
        );
        assert_eq!(
            server.persistent_state.log[3].command, 98,
            "Receiver implementation 4 failed"
        );
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl5_leader_commit() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.commit_index = 1;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 2,
            leader_id: 0,
            prev_log_index: 3,
            prev_log_term: 2,
            entries: vec![],
            leader_commit: 2,
        };
        let res = server.receive_append_entries(req);
        assert_eq!(server.commit_index, 2, "Receiver implementation 4 failed");
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl5_leader_commit_passes() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        server.commit_index = 1;
        server.persistent_state.log.push(Item::new(1, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        server.persistent_state.log.push(Item::new(2, 99));
        let req = AppendEntriesRequest {
            term: 3,
            leader_id: 0,
            prev_log_index: 3,
            prev_log_term: 2,
            entries: vec![Item::new(2, 99)],
            leader_commit: 5,
        };
        let res = server.receive_append_entries(req);
        assert_eq!(server.commit_index, 4, "Receiver implementation 4 failed");
        assert!(res.success);
        assert_eq!(res.term, server.persistent_state.current_term);
    }
}

#[cfg(test)]
mod test_request_vote {
    use super::ServerState;
    use crate::log::{self, Item};
    use crate::rpc::RequestVoteRequest;

    #[test]
    fn impl1() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 2;
        let req = RequestVoteRequest {
            term: 1,
            candidate_id: 0,
            last_log_index: 0,
            last_log_term: 1,
        };
        let res = server.receive_request_vote(req);
        assert!(!res.vote_granted, "Receiver implementation 1 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_novote_log_same() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 1;
        server.persistent_state.voted_for = None;
        server.persistent_state.log.push(Item::new(1, 99));
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: 0,
            last_log_index: 1,
            last_log_term: 1,
        };
        let res = server.receive_request_vote(req);
        assert!(res.vote_granted, "Receiver implementation 2 failed");
        assert_eq!(server.persistent_state.voted_for, Some(0));
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_voted_log_shorter() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 1;
        server.persistent_state.voted_for = Some(0);
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: 0,
            last_log_index: 1,
            last_log_term: 1,
        };
        let res = server.receive_request_vote(req);
        assert!(res.vote_granted, "Receiver implementation 2 failed");
        assert_eq!(server.persistent_state.voted_for, Some(0));
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_votedother_log_same() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 1;
        server.persistent_state.voted_for = Some(1);
        server.persistent_state.log.push(Item::new(1, 99));
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: 0,
            last_log_index: 1,
            last_log_term: 1,
        };
        let res = server.receive_request_vote(req);
        assert!(!res.vote_granted, "Receiver implementation 2 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }

    #[test]
    fn impl2_novote_log_longer() {
        let mut server: ServerState<log::InVec<u8>> = ServerState::default();
        server.persistent_state.current_term = 1;
        server.persistent_state.voted_for = Some(1);
        server.persistent_state.log.push(Item::new(1, 99));
        let req = RequestVoteRequest {
            term: 2,
            candidate_id: 0,
            last_log_index: 0,
            last_log_term: 1,
        };
        let res = server.receive_request_vote(req);
        assert!(!res.vote_granted, "Receiver implementation 2 failed");
        assert_eq!(res.term, server.persistent_state.current_term);
    }
}
