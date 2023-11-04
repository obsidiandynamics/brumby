_help:
    @just --list

# produce the prices for a race (singles and multis)
prices *ARGS:
    cargo run --release --bin prices -- {{ARGS}}

# price singles using fractional overrounds
fractional *ARGS:
    cargo run --release --bin fractional -- {{ARGS}}

# dump a dataset containing the fitted weights and various race parameters to a CSV file
datadump *ARGS:
    cargo run --release --bin datadump -- {{ARGS}}

# backfit a linear regression model from a given dataset
backfit *ARGS:
    cargo run --release --bin backfit -- {{ARGS}}

# evaluate the fitted model against a given dataset
evaluate *ARGS:
    cargo run --release --bin evaluate -- {{ARGS}}

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
