_help:
    @just --list

# produce prices for a soccer match (singles and multis)
soc_prices *ARGS:
    cargo run --release --bin soc_prices -- {{ARGS}}

soc_prices2 *ARGS:
    cargo run --release --bin soc_prices2 -- {{ARGS}}

# produce prices for a race (singles and multis)
rac_prices *ARGS:
    cargo run --release --bin rac_prices -- {{ARGS}}

# price singles using fractional overrounds
rac_fractional *ARGS:
    cargo run --release --bin rac_fractional -- {{ARGS}}

# dump a dataset containing the fitted weights and various race parameters to a CSV file
rac_datadump *ARGS:
    cargo run --release --bin rac_datadump -- {{ARGS}}

# backfit a linear regression model from a given dataset
rac_backfit *ARGS:
    cargo run --release --bin rac_backfit -- {{ARGS}}

# evaluate the fitted model against a given dataset
rac_evaluate *ARGS:
    cargo run --release --bin rac_evaluate -- {{ARGS}}

# measures the Place/Top-N price departure in the given dataset
rac_departure *ARGS:
    cargo run --release --bin rac_departure -- {{ARGS}}

# run the racing multi example
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

# run clippy with pedantic checks
clippy:
    cargo clippy -- -D clippy::pedantic -A clippy::must-use-candidate -A clippy::struct-excessive-bools -A clippy::single-match-else -A clippy::inline-always -A clippy::cast-possible-truncation -A clippy::cast-precision-loss -A clippy::items-after-statements

# update internal package versions
set-version VERSION:
    cargo set-version {{VERSION}}

# publish packages
publish:
    cargo publish -p brumby
    cargo publish -p brumby-racing
    cargo publish -p brumby-soccer

# install Rust
install-rust:
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
