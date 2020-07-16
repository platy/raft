use super::log;
use super::rpc::*;
use super::state::ServerState;
use super::state_machine::Receiver;
use super::ServerId;
use core::iter;

pub struct Server<Command, Log: log::Log<Command = Command>> {
    state: ServerState<Log>,
    receiver: Box<Receiver<Command>>,
    server_id: ServerId,
    votes_this_term: u8,
}

impl<Command: 'static, Log: log::Log<Command = Command> + Default> Server<Command, Log> {
    fn new(server_id: ServerId, state_machine: Box<Receiver<Command>>) -> Self {
        Self {
            state: Default::default(),
            receiver: Box::new(state_machine),
            server_id,
            votes_this_term: 0,
        }
    }
}

impl<Command: 'static, Log: log::Log<Command = Command>> Server<Command, Log> {
    /// Prehandling of all RPC requests before the receiver logic
    fn pre_handle(&mut self, message: impl RPCMessage) {
        // If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
        if message.term() > self.state.current_term() {
            self.state.follow_new_term(message.term());
        }
    }

    fn post_handle(&mut self) {
        self.state.apply_commited(&mut self.receiver)
    }

    /// No messages have been received over the election timeout.
    /// # Followers (§5.2):
    /// -- If election timeout elapses without receiving AppendEntriesRPC from current leader or granting vote to candidate: convert to candidate
    /// # Candidates (§5.2):
    /// - On conversion to candidate, start election:
    /// -- Increment currentTerm
    /// -- Vote for self
    /// -- Reset election timer
    /// -- Send RequestVote RPCs to all other servers
    /// ...
    /// -- If election timeout elapses: start new election
    fn election_timeout(&mut self) -> RequestVoteRequest {
        assert!(self.state.is_follower() || self.state.is_candidate());
        self.votes_this_term = 1;
        self.state.start_election(self.server_id)
    }

    /// While waiting for votes, a candidate may receive an AppendEntries RPC from another server claiming to be leader. If the leader’s term
    /// (included in its RPC) is at least as large as the candidate’s current term, then the candidate recognizes the leader as legitimate and
    /// returns to follower state. If the term in the RPC is smaller than the candidate’s current term, then the candidate rejects the RPC and continues in candidate state.
    pub fn receive_append_entries<LogEntries: IntoIterator<Item = log::Item<Log::Command>>>(
        &mut self,
        req: AppendEntriesRequest<Log::Command, LogEntries>,
    ) -> AppendEntriesResponse {
        if self.state.is_leader() {
            panic!("Append entries not expected for leader"); // @todo this does need to be handled
        }
        // If the term in the RPC is smaller than the candidate’s current term, then the candidate rejects the RPC and continues in candidate state.
        if self.state.is_candidate() && req.term < self.state.current_term() {
            return AppendEntriesResponse {
                term: self.state.current_term(),
                success: false,
            };
        }
        // If the leader’s term (included in its RPC) is at least as large as the candidate’s current term, then the candidate recognizes the leader as legitimate and
        // returns to follower state.
        if self.state.is_candidate() && req.term >= self.state.current_term() {
            self.state.follow_new_term(req.term);
        }
        self.state.receive_append_entries(req)
    }

    pub fn receive_vote(
        &mut self,
        res: RequestVoteResponse,
        total_servers: ServerId,
    ) -> Option<AppendEntriesRequest<Log::Command, iter::Empty<log::Item<Log::Command>>>> {
        if !self.state.is_candidate() {
            return None;
        }
        // count votes
        if res.term == self.state.current_term() && res.vote_granted {
            self.votes_this_term += 1;
        }
        // If votes received from majority of servers:
        if self.votes_this_term > total_servers / 2 {
            // become leader
            self.state.become_leader(total_servers);
            // Upon election: send initial empty AppendEntries RPCs
            Some(AppendEntriesRequest {
                term: self.state.current_term(),
                leader_id: self.server_id,
                prev_log_index: 0,
                prev_log_term: 0,
                entries: iter::empty(),
                leader_commit: self.state.commit_index(),
            })
        } else {
            None
        }
    }
}

/// All Servers:
/// • If commitIndex > lastApplied: increment lastApplied, apply log[lastApplied] to state machine (§5.3)
/// • If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
#[cfg(test)]
mod all_server_rules {
    use super::*;
    use core::sync::atomic::{AtomicI32, Ordering};
    use std::rc::Rc;

    #[test]
    fn apply_commited() {
        let acc = Rc::new(AtomicI32::new(0));
        let acc2 = acc.clone();
        let sm = move |&command: &i32| {
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        let mut s: Server<i32, log::InVec<i32>> = Server::new(55, Box::new(sm));
        s.state.set_log(vec![
            log::Item::new(1, 10),
            log::Item::new(1, -5),
            log::Item::new(1, 1),
        ]);
        s.state.set_commit_index(3);
        s.post_handle();
        assert_eq!(acc.load(Ordering::SeqCst), 6);
        assert_eq!(s.state.last_applied(), 3);
    }

    #[test]
    fn request_updates_term() {
        let mut s: Server<bool, log::InVec<bool>> = Server::new(55, Box::new(|_: &bool| {}));
        // make into a candidate for test
        s.election_timeout();
        // then receive message from another candidate or leader
        s.pre_handle(RequestVoteRequest {
            term: 5,
            candidate_id: 0,
            last_log_term: 1,
            last_log_index: 1,
        });
        assert_eq!(s.state.current_term(), 5);
        assert!(s.state.is_follower());
    }
}

/// # Followers (§5.2):
/// -- If election timeout elapses without receiving AppendEntries RPC from current leader or granting vote to candidate: convert to candidate
/// # Candidates (§5.2):
/// - On conversion to candidate, start election:
/// -- Increment currentTerm
/// -- Vote for self
/// -- Reset election timer
/// -- Send RequestVote RPCs to all other servers
#[cfg(test)]
mod follower_to_candidate_rules {
    use super::*;

    #[test]
    fn election_timeout() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> = Server::new(server_id, Box::new(|_: &bool| {}));
        assert_eq!(s.state.current_term(), 0);
        let req = s.election_timeout();
        assert!(s.state.is_candidate(), "convert to candidate");
        assert_eq!(s.state.current_term(), 1, "Increment currentTerm");
        assert_eq!(s.state.voted_for(), Some(server_id), "Vote for self");
        // "Reset election timer" - timing is handled by caller
        // "Send RequestVote RPCs to all other servers"; the returned RequestVoteRequest should be sent by the caller to all other servers
        assert_eq!(req.term, 1);
        assert_eq!(req.candidate_id, server_id);
        assert_eq!(req.last_log_index, 0);
        assert_eq!(req.last_log_term, 0);
    }
}

/// - If votes received from majority of servers: become leader
/// - If AppendEntries RPC received from new leader: convert to follower
/// - If election timeout elapses: start new election
#[cfg(test)]
mod candidate_rules {
    use super::*;
    use crate::rpc::AppendEntriesRequest;

    #[test]
    fn become_leader() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> = Server::new(server_id, Box::new(|_: &bool| {}));
        assert_eq!(s.state.current_term(), 0);
        s.election_timeout();
        assert!(s.state.is_candidate());
        assert_eq!(s.state.current_term(), 1, "Increment currentTerm");
        let announce = s.receive_vote(
            RequestVoteResponse {
                term: 1,
                vote_granted: true,
            },
            5,
        );
        assert!(announce.is_none());
        let announce = s.receive_vote(
            RequestVoteResponse {
                term: 1,
                vote_granted: true,
            },
            5,
        );
        assert!(s.state.is_leader(), "become leader");
        // Upon election: send initial empty AppendEntries RPCs(heartbeat) to each server; repeat during idle periods to prevent election timeouts (§5.2)
        let announce =
            announce.expect("send initial empty AppendEntries RPCs(heartbeat) to each server");
        assert_eq!(announce.leader_id, 55);
        assert_eq!(announce.entries.len(), 0);
        assert_eq!(announce.term, 1);
    }

    /// While waiting for votes, a candidate may receive an AppendEntries RPC from another server claiming to be leader. If the leader’s term
    /// (included in its RPC) is at least as large as the candidate’s current term, then the candidate recognizes the leader as legitimate and
    /// returns to follower state. If the term in the RPC is smaller than the candidate’s current term, then the candidate rejects the RPC and continues in candidate state.
    #[test]
    fn new_leader_appends_entries() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> = Server::new(server_id, Box::new(|_: &bool| {}));
        assert_eq!(s.state.current_term(), 0);
        s.election_timeout();
        assert!(s.state.is_candidate());
        assert_eq!(s.state.current_term(), 1, "Increment currentTerm");
        let res = s.receive_append_entries(AppendEntriesRequest {
            term: 1,
            leader_id: 20,
            leader_commit: 0,
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
        });
        assert!(s.state.is_follower(), "returns to follower state");
        assert_eq!(s.state.current_term(), 1);
        assert_eq!(res.success, true);
        assert_eq!(res.term, 1);
        // "Reset election timer" - timing is handled by caller
    }
    #[test]
    fn old_leader_appends_entries() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> = Server::new(server_id, Box::new(|_: &bool| {}));
        assert_eq!(s.state.current_term(), 0);
        s.election_timeout();
        assert_eq!(s.state.current_term(), 1, "Increment currentTerm");
        let res = s.receive_append_entries(AppendEntriesRequest {
            term: 0,
            leader_id: 20,
            leader_commit: 0,
            prev_log_index: 0,
            prev_log_term: 0,
            entries: vec![],
        });
        assert!(s.state.is_candidate(), "continues in candidate state");
        assert_eq!(s.state.current_term(), 1);
        assert_eq!(res.success, false, "the candidate rejects the RPC");
        assert_eq!(res.term, 1);
    }

    #[test]
    fn election_timeout() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> = Server::new(server_id, Box::new(|_: &bool| {}));
        assert_eq!(s.state.current_term(), 0);
        s.election_timeout();
        assert_eq!(s.state.current_term(), 1, "Increment currentTerm");
        let req = s.election_timeout();
        assert!(s.state.is_candidate(), "convert to candidate");
        assert_eq!(s.state.current_term(), 2, "Increment currentTerm");
        assert_eq!(s.state.voted_for(), Some(server_id), "Vote for self");
        // "Reset election timer" - timing is handled by caller
        // "Send RequestVote RPCs to all other servers"; the returned RequestVoteRequest should be sent by the caller to all other servers
        assert_eq!(req.term, 2);
        assert_eq!(req.candidate_id, server_id);
        assert_eq!(req.last_log_index, 0);
        assert_eq!(req.last_log_term, 0);
    }
}

// /// - If command received from client: append entry to local log, respond after entry applied to state machine (§5.3)
// /// - If last log index ≥ nextIndex for a follower: send AppendEntries RPC with log entries starting at nextIndex
// /// - If successful: update nextIndex and matchIndex for follower (§5.3)
// /// - If AppendEntries fails because of log inconsistency: decrement nextIndex and retry (§5.3)
// /// - If there exists an N such that N > commitIndex, a majority of matchIndex[i] ≥ N, and log[N].term == currentTerm: set commitIndex = N (§5.3, §5.4).
// #[cfg(test)]
// mod leader_rules {
//     use super::*;
//     use crate::rpc::AppendEntriesRequest;

// }
