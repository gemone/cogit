#!/bin/bash
# Quick tmux smoke test for cogit TUI
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
NAME="cogit-test"

echo "=== Building ==="
cd "$PROJECT_DIR"
cargo build --release

echo "=== Cleaning up old session ==="
tmux kill-session -t "$NAME" 2>/dev/null || true

echo "=== Starting cogit in tmux ==="
tmux new-session -d -s "$NAME" "HOME=/Users/muk $PROJECT_DIR/target/release/cogit"
sleep 1

echo "=== Verifying session is running ==="
tmux list-sessions | grep "$NAME"

echo "=== Capturing initial render ==="
tmux capture-pane -t "$NAME" -p | head -30

echo "=== Testing: press q to quit ==="
tmux send-keys -t "$NAME" "q"
sleep 1

# Check if session died
if ! tmux list-sessions 2>/dev/null | grep -q "$NAME"; then
    echo "PASS: Session exited cleanly"
else
    echo "=== Session still alive, sending second q ==="
    tmux send-keys -t "$NAME" "q"
    sleep 1
    if ! tmux list-sessions 2>/dev/null | grep -q "$NAME"; then
        echo "PASS: Session exited on second q"
    else
        echo "FAIL: Session still running"
        tmux capture-pane -t "$NAME" -p
        exit 1
    fi
fi

echo "=== All tests passed ==="
