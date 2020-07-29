# A RAFT implementation in Rust

This is a (work in progress) implementation of the RAFT consensus algorithm in the rust language.

See the [raft spec](https://raft.github.io/raft.pdf).

See the [documentation](https://platy.github.io/raft/api/raft/).

## Progress

- [x] in-memory state
- [x] append entries RPC handling
- [x] request vote RPC handling
- [x] rules for state changes, timers and making RPC calls
- [x] handle client requests
- [/] cluster tests for scenarios
- [ ] serialise RPCs and run over networking
- [ ] run as separate processes
- [ ] write to disk
- [ ] Cluster membership changes §6
- [ ] Log compaction §7
- [ ] Client interaction §8