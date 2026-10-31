@echo off
setlocal
chcp 65001>nul
title Sushi Calendar Online Dependency Setup
echo Downloading or updating dependencies once...
set "CARGO_NET_RETRY=2"
cargo fetch
if errorlevel 1 (
 echo Cargo fetch failed. Check internet/proxy and try again.
 pause
 exit /b 1
)
echo Dependencies fetched. Building offline...
call build.bat
