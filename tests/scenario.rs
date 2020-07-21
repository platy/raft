//! Tests scenarios for clusters using the `raft::server::Server` api without using any timers, serliasation or real async

use core::sync::atomic::{AtomicI32, Ordering};
use raft::server::Server;
use raft::*;
use std::rc::Rc;

struct TestServer {
    api: Server<i32, log::InVec<i32>>,
    state: Rc<AtomicI32>,
}

impl TestServer {
    fn new(server_id: ServerId) -> TestServer {
        let acc = Rc::new(AtomicI32::new(0));
        let acc2 = acc.clone();
        let sm = move |&command: &i32| {
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        TestServer {
            api: Server::new(server_id, Box::new(sm)),
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
        .map(|server_id| TestServer::new(server_id))
        .collect();

    // one times out and becomes a candidate
    let vote_request = servers[0].api.election_timeout();
    // votes are requested from the other servers and they respond
    let vote = servers[1].api.receive_request_vote(vote_request.clone());
    let announce1 = servers[0].api.receive_vote(vote, 3);
    let vote = servers[2].api.receive_request_vote(vote_request);
    let announce2 = servers[0].api.receive_vote(vote, 3);
    // first server becomes leader, and announces with a heartbeat. The first vote is enought to have a majority
    assert!(announce1.is_some());
    assert!(announce2.is_none());
    for server_id in 1..3 {
        // the announcement will be a AppendEntriesRequest with no entries
        let req = announce1.clone().unwrap();
        let match_index = req.prev_log_index + req.entries.len();
        assert_eq!(req.entries.len(), 0);
        // it is sent to each of the other servers
        let response = servers[server_id as usize]
            .api
            .receive_append_entries(req);
        // they respond so the leader can keep track of their progress
        let applied_commands = servers[0].api.receive_append_entries_response(server_id, match_index, response);
        // there were no commands to apply yet
        assert_eq!(applied_commands.len(), 0);
    }
    // we can now give it a command
    let command_ptr = servers[0].api.command(11);
    // at this point the command is not applied
    assert_eq!(servers[0].value(), 0);
    // after a heartbeat the other servers' logs are updated to match the leader
    for (count, (server_id, req)) in servers[0].api.heartbeat().into_iter().enumerate() {
        let match_index = req.prev_log_index + req.entries.len();
        let response = servers[server_id as usize].api.receive_append_entries(req);
        let applied_commands = servers[0].api.receive_append_entries_response(server_id, match_index, response);
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
        let applied_commands = servers[0].api.receive_append_entries_response(server_id, match_index, response);
        // the leader has no more commands to apply
        assert_eq!(applied_commands.len(), 0);
        // the server has now applied the commited command
        assert_eq!(servers[server_id as usize].value(), 11);
    }
}
