//! Tests scenarios for clusters using the `raft::server::Server` api without using any timers, serliasation or real async

use core::sync::atomic::{AtomicI32, Ordering};
use raft::server::Server;
use raft::state::Persistent;
use raft::*;
use std::rc::Rc;

struct TestServer {
    api: Server<i32, log::InVec<i32>>,
    state: Rc<AtomicI32>,
}

impl TestServer {
    fn new(server_id: ServerId, current_term: Term, log: log::InVec<i32>) -> TestServer {
        let acc = Rc::new(AtomicI32::new(0));
        let acc2 = acc.clone();
        let sm = move |&command: &i32| {
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        let persistent = Persistent {
            current_term,
            log,
            voted_for: None,
        };
        TestServer {
            api: Server::new(server_id, persistent, Box::new(sm)),
            state: acc,
        }
    }

    fn value(&self) -> i32 {
        self.state.load(Ordering::SeqCst)
    }
}

#[test]
fn happy_path() {
    // cluster of 3 servers
    let mut servers: Vec<TestServer> = (0..3)
        .into_iter()
        .map(|server_id| TestServer::new(server_id, 0, vec![]))
        .collect();

    // If a follower receives no communication over a period of time called the election timeout, then it assumes there is no viable leader and begins an election to choose a new leader.
    let vote_request = servers[0].api.election_timeout();
    // To begin an election, a follower increments its current term and transitions to candidate state. It then votes for itself and issues RequestVote RPCs in parallel to each of the other servers in the cluster.
    let vote = servers[1].api.receive_request_vote(vote_request.clone());
    let announce1 = servers[0].api.receive_vote(vote, 3);
    let vote = servers[2].api.receive_request_vote(vote_request);
    let announce2 = servers[0].api.receive_vote(vote, 3);
    // A candidate wins an election if it receives votes from a majority of the servers in the full cluster for the same
    // term. Each server will vote for at most one candidate in a given term, on a first-come-first-served basis.
    // (In this three server cluster, the first vote is enought to have a majority)
    // Once a candidate wins an election, it becomes leader. It then sends heartbeat messages to all ofthe other servers to establish its authority and prevent new elections.
    assert!(announce1.is_some());
    assert!(announce2.is_none());
    for server_id in 1..3 {
        // the announcement will be a AppendEntriesRequest with no entries
        let req = announce1.clone().unwrap();
        let match_index = req.prev_log_index + req.entries.len();
        assert_eq!(req.entries.len(), 0);
        // it is sent to each of the other servers
        let response = servers[server_id as usize].api.receive_append_entries(req);
        // they respond so the leader can keep track of their progress
        let applied_commands =
            servers[0]
                .api
                .receive_append_entries_response(server_id, match_index, response);
        // there were no commands to apply yet
        assert_eq!(applied_commands.len(), 0);
    }
    // we can now give it a command
    let command_ptr = servers[0].api.command(11).unwrap();
    // at this point the command is not applied
    assert_eq!(servers[0].value(), 0);
    // after a heartbeat the other servers' logs are updated to match the leader
    for (count, (server_id, req)) in servers[0].api.heartbeat().into_iter().enumerate() {
        let match_index = req.prev_log_index + req.entries.len();
        let response = servers[server_id as usize].api.receive_append_entries(req);
        let applied_commands =
            servers[0]
                .api
                .receive_append_entries_response(server_id, match_index, response);
        // once the first server has reponded that it has replicated the command, we have a majority and so the command will be committed and applied
        if count == 0 {
            assert_eq!(applied_commands.len(), 1);
            assert_eq!(applied_commands[0], command_ptr);
            assert_eq!(servers[0].value(), 11);
        } else {
            assert_eq!(applied_commands.len(), 0);
        }
        // the other servers wont know about the commit yet and so wont have applied
        assert_eq!(servers[server_id as usize].value(), 0);
    }
    // after another heartbeat the other servers' will know about the commit an apply the command
    for (server_id, req) in servers[0].api.heartbeat().into_iter() {
        let match_index = req.prev_log_index + req.entries.len();
        let response = servers[server_id as usize].api.receive_append_entries(req);
        assert!(response.success);
        let applied_commands =
            servers[0]
                .api
                .receive_append_entries_response(server_id, match_index, response);
        // the leader has no more commands to apply
        assert_eq!(applied_commands.len(), 0);
        // the server has now applied the commited command
        assert_eq!(servers[server_id as usize].value(), 11);
    }
}

mod elections {
    // While waiting for votes, a candidate may receive an AppendEntries RPC from another server claiming to be leader.
    // If the leader’s term (included in its RPC) is at least as large as the candidate’s current term, then the candidate
    // recognizes the leader as legitimate and returns to follower state.
    #[test]
    fn candidate_loses() {}

    // If the term in the RPC is smaller than the candidate’s current term, then the candidate rejects the RPC and con-tinues in candidate state.
    #[test]
    fn old_leader_returns() {}

    // The third possible outcome is that a candidate neither wins nor loses the election: if many followers become
    // candidates at the same time, votes could be split so that no candidate obtains a majority. When this happens,
    // each candidate will time out and start a new election by incrementing its term and initiating another round
    // of Request-Vote RPCs.
    #[test]
    fn split_vote() {}
}

/// a series of scenarios from Figure 7 in the spec. A leader comes to power and its follower doesn't match and needs either some more
/// commands, some removed, or both. the leader will force the follower to duplicate its own log §5.3
mod leader_catch_up {
    use super::*;
    use raft::rpc::RequestVoteResponse;

    const LEADER_LOG: [u32; 10] = [1, 1, 1, 4, 4, 5, 5, 6, 6, 6];

    fn log_of(terms: &[Term]) -> log::InVec<i32> {
        terms
            .into_iter()
            .map(|&term| log::Item::new(term, term as i32))
            .collect()
    }

    fn leader() -> TestServer {
        let mut server = TestServer::new(0, 7, log_of(&LEADER_LOG[..]));
        let req = server.api.election_timeout();
        assert_eq!(req.term, 8);
        server.api.receive_vote(
            RequestVoteResponse {
                vote_granted: true,
                term: req.term,
            },
            2,
        );
        server
    }

    fn follower(terms: &[Term]) -> TestServer {
        TestServer::new(0, 0, log_of(terms))
    }

    #[test]
    fn a() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 4, 4, 5, 5, 6, 6]);

        for _ in 0..2 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            println!("heartbeat {:?}", req);
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            println!("response {:?}", resp);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            println!("completed {:?}", completed_commands);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }

    #[test]
    fn b() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 4]);

        for _ in 0..7 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }

    #[test]
    fn c() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 4, 4, 5, 5, 6, 6, 6, 6]);

        for _ in 0..2 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }

    #[test]
    fn d() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 4, 4, 5, 5, 6, 6, 6, 7, 7]);

        for _ in 0..2 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }

    #[test]
    fn e() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 4, 4, 4, 4]);

        for _ in 0..7 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }

    #[test]
    fn f() {
        let mut leader = leader();
        let mut follower = follower(&[1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 3]);

        for _ in 0..8 {
            let (follower_id, req) = leader.api.heartbeat().into_iter().next().unwrap();
            let match_index = req.prev_log_index + req.entries.len();
            assert_eq!(follower_id, 1);
            let resp = follower.api.receive_append_entries(req);
            let completed_commands =
                leader
                    .api
                    .receive_append_entries_response(follower_id, match_index, resp);
            assert_eq!(completed_commands.len(), 0);
        }

        assert_eq!(follower.api.get_state().log(), &log_of(&LEADER_LOG));
    }
}
