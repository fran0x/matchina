![Rust](https://github.com/flopezlasanta/merx-rs/actions/workflows/rust.yml/badge.svg)
![Audit](https://github.com/flopezlasanta/merx-rs/actions/workflows/audit.yml/badge.svg)

## merx-rs

A simple matching engine for a crypto exchange.

---

### TODO

- [x] enforce IOC, FOK, PostOnly policies
- [x] generate unique trade IDs
- [ ] handle multiple accounts (asset balance, self matching prevention...) - internal vs external (current) order ID
- [ ] support multiple pairs (with load balancer...)
- [ ] performance: flamegraph, perf, ...
- [ ] try different structures: critbit, qp-trie, ...

Future components:

- [ ] Outbound: WS gateway to broadcast order flow and trades (using SHM for comms with engine)
- [ ] Inbound: REST gateway to place and cancel orders (using SHM for comms with engine)
- [ ] Summary: sidecar to compute metrics (spread, depth, ...) on the fly (using SHM for comms with engine)
- [ ] Recorder: sidecar to record order flow and trades (using SHM for comms with engine)

...and more.

