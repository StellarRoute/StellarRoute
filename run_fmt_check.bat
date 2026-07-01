@echo off
cargo fmt --all -- --check
echo FMT_EXIT:%ERRORLEVEL%
