# print options
default:
    @just --list --unsorted

# install cargo tools
init:
    cargo upgrade --incompatible
    cargo update
alias i := init

# check code
check:
    cargo check
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features
alias c := check

# build project
build:
    cargo build --all-targets
alias b := build

# execute tests
test:
    cargo test
alias t := test

# execute benchmarks
bench:
    cargo bench

# generate order requests
gen:
    cargo run --release --bin generator > order_requests.json

# run the simulation
run:
    cargo run --release --bin generator | RUST_LOG=info cargo run --release
alias r := run
