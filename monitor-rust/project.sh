#!/bin/bash -e

cargo new monitor-rust --vcs none
cd ./monitor-rust
# or "cargo init  --vcs none"

cargo install cargo-edit

# create "lib.rs" to link modules
touch ./src/lib.rs

# "block.rs"
touch ./src/block.rs
cargo add serde --features derive
cargo add serde_json

# "feed.rs"
touch ./src/feed.rs
cargo add tokio --features full

# "server.rs"
touch ./src/server.rs
cargo add http
cargo add futures
cargo add hyper --features full
cargo add tungstenite
cargo add url

# "main.rs"
