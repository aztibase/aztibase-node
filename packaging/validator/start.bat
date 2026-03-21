@echo off
title Aztibase Validator Node
cd /d "%~dp0"

echo.
echo  ============================================
echo   Aztibase Network - Validator Node v0.1.4
echo  ============================================
echo.

:: Check for validator key
if not exist "keys\validator.json" (
    echo  No validator key found. Generating one now...
    mkdir keys 2>nul
    aztibase.exe wallet generate --validator --output keys\validator.json
    echo.
    echo  Key saved to keys\validator.json
    echo  IMPORTANT: Back up this file!
    echo.
)

:: Kill any running instance
taskkill /F /IM aztibase.exe >nul 2>&1

:: Create data directory
mkdir data 2>nul

:: Start the node
echo  Starting validator node...
echo  Boot node: 102.209.21.247 (Aztibase VPS)
echo.
start /b aztibase.exe --config node.toml --genesis genesis.toml > node.log 2>&1

:: Wait for node to boot
echo  Waiting for node to start...
timeout /t 8 /nobreak >nul

:: Health check
curl -s http://127.0.0.1:9944/health >nul 2>&1
if %errorlevel% equ 0 (
    echo.
    echo  ============================================
    echo   Validator is RUNNING
    echo  ============================================
    echo.
    echo   RPC:       http://127.0.0.1:9944
    echo   Health:    http://127.0.0.1:9944/health
    echo   Metrics:   http://127.0.0.1:9944/metrics/json
    echo.
    echo   NEXT STEPS:
    echo   1. Open dashboard\index.html in Chrome (your node dashboard)
    echo   2. Install the Aztibase Wallet Chrome extension
    echo   3. Click "Faucet" to get testnet AZTB
    echo   4. Click "Become Validator"
    echo   5. Wait for next epoch - you're in!
    echo.
    echo   To stop: close this window or run stop.bat
    echo.
) else (
    echo.
    echo  [WARN] Node may still be starting.
    echo  Check node.log for details.
    echo.
)

:: Keep window open so the node stays running
echo  Press Ctrl+C to stop the node.
echo.

:: Wait for the background process
:loop
timeout /t 30 /nobreak >nul
tasklist /FI "IMAGENAME eq aztibase.exe" 2>nul | find "aztibase.exe" >nul
if %errorlevel% equ 0 goto loop

echo.
echo  Node has stopped.
pause
