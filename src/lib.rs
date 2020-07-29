#![allow(dead_code)]

pub type ServerId = u8;
/// Time is divided into terms, and each term beginswith an election. After a successful election, a single leadermanages the cluster until the end of the term. Some electionsfail, in which case the term ends without choosing a leader.The transitions between terms may be observed at differenttimes on different servers.
pub type Term = u32;

pub type CommandPtr = (log::Index, Term);

/// Error when a non-leader is asked to do something only a leader can do
#[derive(Debug, Eq, PartialEq)]
pub struct NotLeader;

pub mod log;
pub mod rpc;
pub mod server;
pub mod state;
pub mod state_machine;
