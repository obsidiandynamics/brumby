_help:
    @just --list

# run the multi example
multi:
    cargo run --example multi --release

# run Criterion bechmarks
bench:
    bash -c 'type cargo-criterion >/dev/null 2>&1 || cargo install cargo-criterion'
    cargo criterion

# install Rust
install-rust:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
