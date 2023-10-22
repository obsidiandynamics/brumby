_help:
    @just --list

# produce the prices for a race
prices:
    cargo run --release --bin prices

# run the multi example
multi:
    cargo run --example multi --release

# run Criterion bechmarks
bench:
    bash -c 'type cargo-criterion >/dev/null 2>&1 || cargo install cargo-criterion'
    cargo criterion

# run the tests
test:
    cargo test
    cargo test --examples
    cargo doc --no-deps
    cargo bench --no-run --profile dev

# install Rust
install-rust:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
