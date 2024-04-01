#!/bin/bash
set -e
root="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"
export RXQLITED_DIR=$root/../rxqlite/target/release
#RUST_LOG=info cargo test notifications2_insecure_ssl --release -- --nocapture
#RUST_LOG="error" 
cargo test --release 
#-- --nocapture > /mnt/c/Ha/test/topenraft-sqlite/n.log

