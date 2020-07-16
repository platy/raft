# A RAFT implementation in Rust

This is a (work in progress) implementation of the RAFT consensus algorithm in the rust language.

See the [raft spec](https://raft.github.io/raft.pdf).

See the [documentation](https://platy.github.io/raft/raft/).

## Progress

- [x] in-memory state
- [x] append entries RPC handling
- [x] request vote RPC handling
- [ ] rules for state changes, timers and making RPC calls
- [ ] handle client requests
- [ ] cluster tests for scenarios
- [ ] write to disk
- [ ] serialise RPCs and run over networking
- [ ] run as separate processes
