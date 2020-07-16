use super::log;
use super::rpc::RPCMessage;
use super::state::ServerState;
use super::state_machine::Receiver;

pub struct Server<Command, Log: log::Log<Command = Command>> {
    state: ServerState<Log>,
    receiver: Box<Receiver<Command>>,
}

impl<Command: 'static, Log: log::Log<Command = Command>> Server<Command, Log> {
    /// Prehandling of all RPC requests before the receiver logic
    fn pre_handle(&mut self, message: impl RPCMessage) {
        // If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
        if message.term() > self.state.current_term() {
            self.state.set_current_term(message.term());
        }
    }

    fn post_handle(&mut self) {
        self.state.apply_commited(&mut self.receiver)
    }
}

/// All Servers:
/// • If commitIndex > lastApplied: increment lastApplied, apply log[lastApplied] to state machine (§5.3)
/// • If RPC request or response contains term T > currentTerm: set currentTerm = T, convert to follower (§5.1)
#[cfg(test)]
mod all_server_rules {
    use super::*;
    use crate::rpc::RequestVoteRequest;
    use core::sync::atomic::{AtomicI32, Ordering};
    use std::rc::Rc;

    #[test]
    fn apply_commited() {
        let acc = Rc::new(AtomicI32::new(0));
        let acc2 = acc.clone();
        let sm = move |&command: &i32| {
            acc2.fetch_add(command, Ordering::SeqCst);
        };
        let mut s: Server<i32, log::InVec<i32>> = Server {
            state: Default::default(),
            receiver: Box::new(sm),
        };
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
        let mut s: Server<bool, log::InVec<bool>> = Server {
            state: Default::default(),
            receiver: Box::new(|_: &bool| {}),
        };
        s.pre_handle(RequestVoteRequest {
            term: 5,
            candidate_id: 0,
            last_log_term: 1,
            last_log_index: 1,
        });
        assert_eq!(s.state.current_term(), 5);
    }
}
