![Rust](https://github.com/flopezlasanta/merx-rs/actions/workflows/rust.yml/badge.svg)
![Audit](https://github.com/flopezlasanta/merx-rs/actions/workflows/audit.yml/badge.svg)

## merx-rs

A simple matching engine for a crypto exchange.

---

### TODO

- [ ] enforce IOC, FOK, PostOnly policies
- [ ] generate unique trade IDs
- [ ] WS gateway to broadcast order flow and trades (using SHM for comms with engine)
- [ ] REST gateway to place and cancel orders (using SHM for comms with engine)
- [ ] Watcher sidecar to compute metrics (spread, depth, ...) on the fly (using SHM for comms with engine)
- [ ] Persister sidecar to record order flow and trades (using SHM for comms with engine)
- [ ] support multiple accounts (asset balance, self matching prevention...) - internal vs external (current) order ID
- [ ] support multiple pairs (with load balancer...)

...and more.

