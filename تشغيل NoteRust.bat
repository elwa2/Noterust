@echo off
chcp 65001 >nul
title NoteRust
cd /d "%~dp0"

if not exist "NoteRust.exe" (
    echo NoteRust.exe not found in this folder.
    echo Build it first with: cargo build --release -p noterust
    pause
    exit /b 1
)

start "" "NoteRust.exe"
exit /b 0
