//! Raft servers communicate using remote procedure calls(RPCs), and the basic consensus algorithm requires onlytwo types of RPCs. RequestVote RPCs are initiated bycandidates during elections (Section 5.2), and Append-Entries RPCs are initiated by leaders to replicate log en-tries and to provide a form of heartbeat (Section 5.3). Sec-tion 7 adds a third RPC for transferring snapshots betweenservers. Servers retry RPCs if they do not receive a re-sponse in a timely manner, and they issue RPCs in parallelfor best performance.

use super::log::{Item, LogIndex};
use super::{ServerId, Term};
use serde::{Deserialize, Serialize};

/// Invoked by leader to replicate log entries (§5.3); also used asheartbeat (§5.2).
#[derive(Serialize, Deserialize)]
pub struct AppendEntriesRequest<Command, LogEntries: IntoIterator<Item = Item<Command>>> {
    /// leader’s term
    pub term: Term,
    /// so follower can redirect clients
    pub leader_id: ServerId,
    /// index of log entry immediately precedingnew ones
    pub prev_log_index: LogIndex,
    /// term of prevLogIndex entry
    pub prev_log_term: Term,
    /// log entries to store (empty for heartbeat;may send more than one for efficiency)
    pub entries: LogEntries,
    /// leader’s commitIndex
    pub leader_commit: LogIndex,
}

#[derive(Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// currentTerm, for leader to update itself
    pub term: Term,
    /// true if follower contained entry matchingprevLogIndex and prevLogTerm
    pub success: bool,
}

/// Invoked by candidates to gather votes (§5.2).
#[derive(Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// candidate’s term
    term: Term,
    /// candidate requesting vote
    candidate_id: ServerId,
    /// index of candidate’s last log entry (§5.4)
    last_log_index: LogIndex,
    /// term of candidate’s last log entry (§5.4)
    last_log_term: Term,
}

#[derive(Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// currentTerm, for candidate to update itself
    term: Term,
    /// true means candidate received vote
    vote_granted: bool,
}
