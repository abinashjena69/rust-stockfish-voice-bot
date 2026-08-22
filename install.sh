#!/bin/bash

echo "=========================================="
echo "   Installing Abinash's Chess Voice-Bot   "
echo "=========================================="

echo "[1/4] Installing required packages..."
pkg update -y && pkg upgrade -y
pkg install rust stockfish termux-api git wget -y

echo "[2/4] Downloading project from GitHub..."
cd ~
if [ -d "abinash_chessbot" ]; then
    echo "Folder exists, updating code..."
    cd abinash_chessbot
    git pull origin main
else
    # Downloads the repo and saves it in a folder named abinash_chessbot
    git clone https://github.com/abinashjena69/rust-stockfish-voice-bot.git abinash_chessbot
    cd abinash_chessbot
fi

echo "[3/4] Compiling the Rust engine (this takes a minute)..."
cargo build --release

echo "[4/4] Setting up custom play command..."
if ! grep -q "alias abinash_chessbot" ~/.bashrc; then
    echo "alias abinash_chessbot='cd ~/abinash_chessbot && cargo run --release'" >> ~/.bashrc
fi

echo "=========================================="
echo " ✅ INSTALLATION COMPLETE! ✅ "
echo ""
echo " IMPORTANT: Restart your Termux app right now."
echo " Then, just type: abinash_chessbot"
echo "=========================================="
