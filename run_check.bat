@echo off
cargo check --workspace
echo CHECK_EXIT:%ERRORLEVEL%
