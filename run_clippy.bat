@echo off
cargo clippy --workspace --lib --bins --tests -- -D warnings
echo CLIPPY_EXIT:%ERRORLEVEL%
