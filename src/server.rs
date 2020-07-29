use super::log;
use super::rpc::{
    AppendEntriesRequest, AppendEntriesResponse, RPCMessage, RequestVoteRequest,
    RequestVoteResponse,
};
use super::state::{Persistent, ServerState};
use super::state_machine::Receiver;
use super::ServerId;
use super::NotLeader;
use core::iter;
use super::CommandPtr;

/// The logic of a raft server except for delay, timouts, messaging and configuration which are handled by the client
pub struct Server<Command: Clone, Log: log::Log<Command = Command>> {
    /// state and state logic of a raft server
    state: ServerState<Log>,
    /// state machine receiver
    receiver: Box<Receiver<Command>>,
    /// id of this server
    server_id: ServerId,
    /// number of votes received in the current term
    votes_this_term: u8,
}

impl<Command: 'static + Clone, Log: log::Log<Command = Command> + Default> Server<Command, Log> {
    pub fn new(
        server_id: ServerId,
        persistent_state: Persistent<Log>,
        state_machine: Box<Receiver<Command>>,
    ) -> Self {
        Self {
            state: ServerState::<Log>::new(persistent_state),
            receiver: state_machine,
            server_id,
            votes_this_term: 0,
        }
    }
}

impl<Command: 'static + Clone, Log: log::Log<Command = Command>> Server<Command, Log> {
    /// Prehandling of all RPC requests before the receiver logic
    fn pre_handle(&mut self, message: &impl RPCMessage) {
        // If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
        if message.term() > self.state.current_term() {
            self.state.follow_new_term(message.term());
        }
    }

    /// Post handling applies any commited commands
    fn post_handle(&mut self) -> Vec<CommandPtr> {
        self.state.apply_commited(&mut self.receiver)
    }

    /// No messages have been received over the election timeout.
    /// # Followers (§5.2):
    /// -- If election timeout elapses without receiving `AppendEntriesRPC` from current leader or granting vote to candidate: convert to candidate
    /// # Candidates (§5.2):
    /// - On conversion to candidate, start election:
    /// -- Increment currentTerm
    /// -- Vote for self
    /// -- Reset election timer
    /// -- Send `RequestVote` RPCs to all other servers
    /// ...
    /// -- If election timeout elapses: start new election
    pub fn election_timeout(&mut self) -> RequestVoteRequest {
        assert!(self.state.is_follower() || self.state.is_candidate());
        self.votes_this_term = 1;
        self.state.start_election(self.server_id)
    }

    /// While waiting for votes, a candidate may receive an `AppendEntries` RPC from another server claiming to be leader. If the leader’s term
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
        if req.term >= self.state.current_term() {
            self.state.follow_new_term(req.term);
        }
        let resp = self.state.receive_append_entries(req);
        self.post_handle();
        resp
    }

    pub fn receive_request_vote(&mut self, req: RequestVoteRequest) -> RequestVoteResponse {
        self.pre_handle(&req);
        self.state.receive_request_vote(req)
    }

    /// Receives a `RequestVote` response
    /// # Return
    /// if the server determines it has won the election with the received vote, an `AppendEntriesRequest` heartbeat is
    /// returned which the client needs to send to all other servers in the cluster. At this point the client ust also
    /// start a heartbeat timer and start calling the heartbeat method on it
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
            self.state.become_leader(self.server_id, total_servers);
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

    /// Receive response from `AppendEntriesRequest`
    /// - If successful: update nextIndex and matchIndex for follower (§5.3)
    /// - If `AppendEntriesRequest` fails because of log inconsistency:decrement nextIndex and retry (§5.3)
    /// # Args
    /// Requires the id of the server responding the the append entries request, and the `match_index` which is the index of the last log entry sent in the request.
    /// # Return
    /// `Vec` of command pointers which have been newly applied, the client will need to send any client responses associated
    /// with those commands
    pub fn receive_append_entries_response(
        &mut self,
        from: ServerId,
        match_index: log::Index,
        res: AppendEntriesResponse,
    ) -> Vec<CommandPtr> {
        // @todo also check for new term
        if res.success {
            self.state.update_follower(from, match_index);
            self.post_handle()
        } else {
            self.state.follower_inconsistent(from);
            vec![]
        }
    }

    /// Client adds a command
    /// # Return
    /// Command pointer, the client can be informed of the successful application of the command once this command
    /// pointer s returned by `receive_append_entries_response`
    pub fn command(&mut self, command: Log::Command) -> Result<CommandPtr, NotLeader> {
        self.state.add_command(command)
    }

    /// Call this on a timer from when an election is won, until a `pre_handle` indicates that a new election has been called
    pub fn heartbeat(
        &mut self,
    ) -> Vec<(
        ServerId,
        AppendEntriesRequest<Command, Vec<log::Item<Command>>>,
    )> {
        self.state.produce_append_entries(self.server_id)
    }
}

impl<Command: 'static + Clone, Log: log::Log<Command = Command>> Server<Command, Log> {
    pub fn get_state(&self) -> &ServerState<Log> {
        &self.state
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
        let persistent_state = Persistent {
            current_term: 0,
            log: vec![
                log::Item::new(1, 10),
                log::Item::new(1, -5),
                log::Item::new(1, 1),
            ],
            voted_for: None,
        };
        let mut s: Server<i32, log::InVec<i32>> = Server::new(55, persistent_state, Box::new(sm));
        s.state.set_commit_index(3);
        s.post_handle();
        assert_eq!(acc.load(Ordering::SeqCst), 6);
        assert_eq!(s.state.last_applied(), 3);
    }

    #[test]
    fn request_updates_term() {
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(55, Persistent::default(), Box::new(|_: &bool| {}));
        // make into a candidate for test
        s.election_timeout();
        // then receive message from another candidate or leader
        s.pre_handle(&RequestVoteRequest {
            term: 5,
            candidate_id: 0,
            last_log_term: 1,
            last_log_index: 1,
        });
        assert_eq!(s.state.current_term(), 5);
        assert!(s.state.is_follower());
    }

    // @todo response updates term
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
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(server_id, Persistent::default(), Box::new(|_: &bool| {}));
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
        let server_id = 0;
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(server_id, Persistent::default(), Box::new(|_: &bool| {}));
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
        assert_eq!(announce.leader_id, server_id);
        assert_eq!(announce.entries.len(), 0);
        assert_eq!(announce.term, 1);
    }

    /// While waiting for votes, a candidate may receive an AppendEntries RPC from another server claiming to be leader. If the leader’s term
    /// (included in its RPC) is at least as large as the candidate’s current term, then the candidate recognizes the leader as legitimate and
    /// returns to follower state. If the term in the RPC is smaller than the candidate’s current term, then the candidate rejects the RPC and continues in candidate state.
    #[test]
    fn new_leader_appends_entries() {
        let server_id = 55;
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(server_id, Persistent::default(), Box::new(|_: &bool| {}));
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
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(server_id, Persistent::default(), Box::new(|_: &bool| {}));
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
        let mut s: Server<bool, log::InVec<bool>> =
            Server::new(server_id, Persistent::default(), Box::new(|_: &bool| {}));
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

/// - If command received from client: append entry to local log, respond after entry applied to state machine (§5.3)
/// - If last log index ≥ nextIndex for a follower: send AppendEntries RPC with log entries starting at nextIndex
/// -- If successful: update nextIndex and matchIndex for follower (§5.3)
/// -- If AppendEntries fails because of log inconsistency: decrement nextIndex and retry (§5.3)
/// - If there exists an N such that N > commitIndex, a majority of matchIndex[i] ≥ N, and log[N].term == currentTerm: set commitIndex = N (§5.3, §5.4).
#[cfg(test)]
mod leader_rules {
    use super::log::Log;
    use super::*;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicI32, Ordering};

    #[test]
    fn whole_handle_client_command() {
        let acc = Rc::new(AtomicI32::new(0));
        let acc2 = acc.clone();
        let sm = move |&command: &i32| {
            println!("SM received command {}", command);
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        let persistent_state = Persistent {
            current_term: 1,
            log: vec![log::Item::new(1, 10)],
            voted_for: None,
        };
        let mut s: Server<i32, log::InVec<i32>> = Server::new(1, persistent_state, Box::new(sm));
        s.state.set_commit_index(1);
        s.post_handle();
        assert_eq!(s.state.current_term(), 1);
        s.election_timeout();
        assert!(s.state.is_candidate(), "become candidate");
        assert_eq!(s.state.current_term(), 2, "Increment currentTerm");
        s.receive_vote(
            RequestVoteResponse {
                term: 2,
                vote_granted: true,
            },
            3,
        );
        assert!(s.state.is_leader(), "become leader");
        let command_id = s.command(5); // the caller should call heartbeat when this returns to get the sync messages for the followers
        assert_eq!(
            *s.state.log().get_command(2),
            5,
            "If command received from client: append entry to local log"
        );
        assert_eq!(s.state.log().last_log_term(), 2);
        assert_eq!(
            command_id,
            Ok((2, 2)),
            "the index and term which when commited will activate the response"
        );
        assert_eq!(
            acc.load(Ordering::SeqCst),
            10,
            "command should not yet be commited"
        );
        let heartbeat_reqs = s.heartbeat();
        // - If last log index ≥ nextIndex for a follower: send AppendEntries RPC with log entries starting at nextIndex
        assert_eq!(heartbeat_reqs.len(), 2);
        assert_eq!(
            heartbeat_reqs[0],
            (
                0,
                AppendEntriesRequest {
                    term: 2,
                    leader_id: 1,
                    prev_log_index: 1,
                    prev_log_term: 1,
                    leader_commit: 1,
                    entries: vec![log::Item::new(2, 5)],
                }
            )
        );
        assert_eq!(
            heartbeat_reqs[1],
            (
                2,
                AppendEntriesRequest {
                    term: 2,
                    leader_id: 1,
                    prev_log_index: 1,
                    prev_log_term: 1,
                    leader_commit: 1,
                    entries: vec![log::Item::new(2, 5)],
                }
            )
        );
        // -- If successful: update nextIndex and matchIndex for follower (§5.3)
        let new_commits1 = s.receive_append_entries_response(
            0,
            2,
            AppendEntriesResponse {
                success: true,
                term: 2,
            },
        );
        assert_eq!(s.state.get_leader_state().next_index[0], 3);
        assert_eq!(s.state.get_leader_state().match_index[0], 2);
        // -- If AppendEntries fails because of log inconsistency: decrement nextIndex and retry (§5.3)
        let new_commits2 = s.receive_append_entries_response(
            2,
            2,
            AppendEntriesResponse {
                success: false,
                term: 2,
            },
        );
        assert_eq!(new_commits2.len(), 0);
        assert_eq!(s.state.get_leader_state().next_index[2], 1);
        assert_eq!(s.state.get_leader_state().match_index[2], 0);
        // - If there exists an N such that N > commitIndex, a majority of matchIndex[i] ≥ N, and log[N].term == currentTerm: set commitIndex = N (§5.3, §5.4).
        assert_eq!(s.state.commit_index(), 2);
        assert_eq!(s.state.last_applied(), 2);
        // respond after entry applied to state machine (§5.3)
        // client is responsible for sending any client responses for the commited entries
        assert_eq!(new_commits1.len(), 1);
        assert_eq!(new_commits1[0], (2, 2));
        assert_eq!(acc.load(Ordering::SeqCst), 15, "command should be commited");
    }
}
