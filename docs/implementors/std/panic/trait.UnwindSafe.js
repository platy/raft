(function() {var implementors = {};
implementors["raft"] = [{"text":"impl&lt;Command&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/log/struct.LogItem.html\" title=\"struct raft::log::LogItem\">LogItem</a>&lt;Command&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;Command: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a>,&nbsp;</span>","synthetic":true,"types":["raft::log::LogItem"]},{"text":"impl&lt;Log&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/state/struct.PersistentState.html\" title=\"struct raft::state::PersistentState\">PersistentState</a>&lt;Log&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;Log: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a>,&nbsp;</span>","synthetic":true,"types":["raft::state::PersistentState"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/state/struct.LeaderState.html\" title=\"struct raft::state::LeaderState\">LeaderState</a>","synthetic":true,"types":["raft::state::LeaderState"]},{"text":"impl&lt;Log&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/state/struct.ServerState.html\" title=\"struct raft::state::ServerState\">ServerState</a>&lt;Log&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;Log: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a>,&nbsp;</span>","synthetic":true,"types":["raft::state::ServerState"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"enum\" href=\"raft/state/enum.States.html\" title=\"enum raft::state::States\">States</a>","synthetic":true,"types":["raft::state::States"]},{"text":"impl&lt;Command, LogEntries&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesRequest.html\" title=\"struct raft::rpc::AppendEntriesRequest\">AppendEntriesRequest</a>&lt;Command, LogEntries&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;LogEntries: <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a>,&nbsp;</span>","synthetic":true,"types":["raft::rpc::AppendEntriesRequest"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesResponse.html\" title=\"struct raft::rpc::AppendEntriesResponse\">AppendEntriesResponse</a>","synthetic":true,"types":["raft::rpc::AppendEntriesResponse"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/rpc/struct.RequestVoteRequest.html\" title=\"struct raft::rpc::RequestVoteRequest\">RequestVoteRequest</a>","synthetic":true,"types":["raft::rpc::RequestVoteRequest"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/std/panic/trait.UnwindSafe.html\" title=\"trait std::panic::UnwindSafe\">UnwindSafe</a> for <a class=\"struct\" href=\"raft/rpc/struct.RequestVoteResponse.html\" title=\"struct raft::rpc::RequestVoteResponse\">RequestVoteResponse</a>","synthetic":true,"types":["raft::rpc::RequestVoteResponse"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()