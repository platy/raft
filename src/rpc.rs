//! Raft servers communicate using remote procedure calls(RPCs), and the basic consensus algorithm requires only two types of RPCs. `RequestVote` RPCs are initiated bycandidates during elections (Section 5.2), and Append-Entries RPCs are initiated by leaders to replicate log en-tries and to provide a form of heartbeat (Section 5.3). Sec-tion 7 adds a third RPC for transferring snapshots betweenservers. Servers retry RPCs if they do not receive a re-sponse in a timely manner, and they issue RPCs in parallelfor best performance.

use super::log;
use super::{ServerId, Term};
use actix::prelude::*;
use serde::{Deserialize, Serialize};

pub trait RPCMessage {
    fn term(&self) -> Term;
}

/// Invoked by leader to replicate log entries (§5.3); also used asheartbeat (§5.2).
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Message)] // todo add feature flag
#[rtype(result = "AppendEntriesResponse")]
pub struct AppendEntriesRequest<Command: Clone, LogEntries: IntoIterator<Item = log::Item<Command>>>
{
    /// leader’s term
    pub term: Term,
    /// so follower can redirect clients
    pub leader_id: ServerId,
    /// index of log entry immediately precedingnew ones
    pub prev_log_index: log::Index,
    /// term of prevlog::Index entry
    pub prev_log_term: Term,
    /// log entries to store (empty for heartbeat;may send more than one for efficiency)
    pub entries: LogEntries,
    /// leader’s commitIndex
    pub leader_commit: log::Index,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// currentTerm, for leader to update itself
    pub term: Term,
    /// true if follower contained entry matching prevLogIndex and prevLogTerm
    pub success: bool,
}

/// Invoked by candidates to gather votes (§5.2).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Message)]
#[rtype(result = "RequestVoteResponse")]
pub struct RequestVoteRequest {
    /// candidate’s term
    pub term: Term,
    /// candidate requesting vote
    pub candidate_id: ServerId,
    /// index of candidate’s last log entry (§5.4)
    pub last_log_index: log::Index,
    /// term of candidate’s last log entry (§5.4)
    pub last_log_term: Term,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// currentTerm, for candidate to update itself
    pub term: Term,
    /// true means candidate received vote
    pub vote_granted: bool,
}

impl<Command: Clone, LogEntries: IntoIterator<Item = log::Item<Command>>> RPCMessage
    for AppendEntriesRequest<Command, LogEntries>
{
    fn term(&self) -> Term {
        self.term
    }
}

impl RPCMessage for RequestVoteRequest {
    fn term(&self) -> Term {
        self.term
    }
}
