# ♟️ Abinash Chess Voice-Bot

A professional-grade, voice-controlled chess assistant built in Rust, powered by the Stockfish engine, and designed specifically for Android via Termux.

## ✨ Features
* **Voice Control:** Play completely hands-free by speaking your moves naturally into your phone's microphone.
* **Real-time Engine Coach:** Stockfish evaluates every move dynamically, warning you of blunders and identifying tactical motives.
* **Opening Book Recognition:** Instantly recognizes standard opening theory for the first 3 full moves.
* **Post-Game Grandmaster Review:** Analyzes your entire game upon completion, grading your accuracy, estimating your Elo, and categorizing your moves.
* **Auto-Save:** Automatically saves your completed matches to a standard `.pgn` file.

## 📋 Prerequisites
Before installing, ensure you have the following two apps installed on your Android device from **F-Droid** (Do not use the Google Play Store versions, as they are outdated):
1. **Termux** (The terminal emulator)
2. **Termux:API** (Required to allow the app to use your microphone and text-to-speech)

## 🚀 Installation Guide
Open your Termux app and paste these exact commands in order.

**Step 1: Install the downloader tool**
```bash
pkg update -y && pkg install wget -y
wget -O- [https://raw.githubusercontent.com/abinashjena69/rust-stockfish-voice-bot/main/install.sh](https://raw.githubusercontent.com/abinashjena69/rust-stockfish-voice-bot/main/install.sh) | bash
Note: The first time you install, compiling the Rust engine may take a minute depending on your processor.
​🎮 How to Play
​Once the installation finishes, restart your Termux app. You can launch the game at any time by simply typing:abinash_chessbot
Controls
​Standard Move: Type standard chess notation (e.g., e4, Nf3, O-O) and press ENTER.
​Voice Move: Press SPACEBAR then ENTER (or type v) to trigger the microphone. Speak your move (e.g., "Knight to F3" or "Takes on D4").
​Undo: Type undo to take back your last move.
​Quit: Type quit to exit the current match and view your post-game analysis.
