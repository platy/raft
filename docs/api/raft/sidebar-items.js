initSidebarItems({"mod":[["log",""],["rpc","Raft servers communicate using remote procedure calls(RPCs), and the basic consensus algorithm requires only two types of RPCs. `RequestVote` RPCs are initiated bycandidates during elections (Section 5.2), and Append-Entries RPCs are initiated by leaders to replicate log en-tries and to provide a form of heartbeat (Section 5.3). Sec-tion 7 adds a third RPC for transferring snapshots betweenservers. Servers retry RPCs if they do not receive a re-sponse in a timely manner, and they issue RPCs in parallelfor best performance."],["server",""],["state",""],["state_machine",""]],"type":[["ServerId",""],["Term","Time is divided into terms, and each term beginswith an election. After a successful election, a single leadermanages the cluster until the end of the term. Some electionsfail, in which case the term ends without choosing a leader.The transitions between terms may be observed at differenttimes on different servers."]]});