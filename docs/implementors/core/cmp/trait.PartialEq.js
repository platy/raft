(function() {var implementors = {};
implementors["raft"] = [{"text":"impl&lt;Command:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;<a class=\"struct\" href=\"raft/log/struct.Item.html\" title=\"struct raft::log::Item\">Item</a>&lt;Command&gt;&gt; for <a class=\"struct\" href=\"raft/log/struct.Item.html\" title=\"struct raft::log::Item\">Item</a>&lt;Command&gt;","synthetic":false,"types":["raft::log::Item"]},{"text":"impl&lt;Command:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/clone/trait.Clone.html\" title=\"trait core::clone::Clone\">Clone</a>, LogEntries:&nbsp;<a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a> + <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/iter/traits/collect/trait.IntoIterator.html\" title=\"trait core::iter::traits::collect::IntoIterator\">IntoIterator</a>&lt;Item = <a class=\"struct\" href=\"raft/log/struct.Item.html\" title=\"struct raft::log::Item\">Item</a>&lt;Command&gt;&gt;&gt; <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;<a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesRequest.html\" title=\"struct raft::rpc::AppendEntriesRequest\">AppendEntriesRequest</a>&lt;Command, LogEntries&gt;&gt; for <a class=\"struct\" href=\"raft/rpc/struct.AppendEntriesRequest.html\" title=\"struct raft::rpc::AppendEntriesRequest\">AppendEntriesRequest</a>&lt;Command, LogEntries&gt;","synthetic":false,"types":["raft::rpc::AppendEntriesRequest"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;<a class=\"struct\" href=\"raft/state/struct.Leader.html\" title=\"struct raft::state::Leader\">Leader</a>&gt; for <a class=\"struct\" href=\"raft/state/struct.Leader.html\" title=\"struct raft::state::Leader\">Leader</a>","synthetic":false,"types":["raft::state::Leader"]},{"text":"impl <a class=\"trait\" href=\"https://doc.rust-lang.org/nightly/core/cmp/trait.PartialEq.html\" title=\"trait core::cmp::PartialEq\">PartialEq</a>&lt;<a class=\"enum\" href=\"raft/state/enum.States.html\" title=\"enum raft::state::States\">States</a>&gt; for <a class=\"enum\" href=\"raft/state/enum.States.html\" title=\"enum raft::state::States\">States</a>","synthetic":false,"types":["raft::state::States"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()