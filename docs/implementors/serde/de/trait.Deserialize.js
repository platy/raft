(function() {var implementors = {};
implementors["raft"] = [{"text":"impl&lt;'de, Command, LogEntries:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html\" title=\"trait core::iter::traits::collect::IntoIterator\">IntoIterator</a>&lt;Item = <a class=\"struct\" href=\"raft/log/struct.LogItem.html\" title=\"struct raft::log::LogItem\">LogItem</a>&lt;Command&gt;&gt;&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.114/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesRequest.html\" title=\"struct raft::rpc::AppendEntriesRequest\">AppendEntriesRequest</a>&lt;Command, LogEntries&gt; <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;LogEntries: <a class=\"trait\" href=\"https://docs.rs/serde/1.0.114/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt;,&nbsp;</span>","synthetic":false,"types":["raft::rpc::AppendEntriesRequest"]},{"text":"impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.114/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesResponse.html\" title=\"struct raft::rpc::AppendEntriesResponse\">AppendEntriesResponse</a>","synthetic":false,"types":["raft::rpc::AppendEntriesResponse"]},{"text":"impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.114/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"raft/rpc/struct.RequestVoteRequest.html\" title=\"struct raft::rpc::RequestVoteRequest\">RequestVoteRequest</a>","synthetic":false,"types":["raft::rpc::RequestVoteRequest"]},{"text":"impl&lt;'de&gt; <a class=\"trait\" href=\"https://docs.rs/serde/1.0.114/serde/de/trait.Deserialize.html\" title=\"trait serde::de::Deserialize\">Deserialize</a>&lt;'de&gt; for <a class=\"struct\" href=\"raft/rpc/struct.RequestVoteResponse.html\" title=\"struct raft::rpc::RequestVoteResponse\">RequestVoteResponse</a>","synthetic":false,"types":["raft::rpc::RequestVoteResponse"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()