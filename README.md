![Rust](https://github.com/flopezlasanta/merx-rs/actions/workflows/rust.yml/badge.svg)
![Audit](https://github.com/flopezlasanta/merx-rs/actions/workflows/audit.yml/badge.svg)

## merx-rs

A simple matching engine for a crypto exchange.

---

### TODO

- [ ] enforce IOC, FOK, PostOnly policies
- [ ] generate unique trade IDs
- [ ] WS gateway to broadcast order flow and trades, communicating with matching engine via SHM
- [ ] REST gateway to place and cancel orders, communicating with matching engine via SHM
- [ ] Watcher sidecar to compute metrics on the fly (spread, depth, ...), communicating with matching engine via SHM
- [ ] Persister sidecar to record order flow and trades, communicating with matching engine via SH
- [ ] support multiple accounts (with asset balance, self matching prevention...)
- [ ] support multiple pairs (with load balancer...)

...and more.

