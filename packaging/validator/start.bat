@echo off
setlocal enabledelayedexpansion
title Aztibase Validator Node
cd /d "%~dp0"

echo.
echo  ============================================
echo   Aztibase Network - Validator Node v0.1.5
echo  ============================================
echo.

:: Check for validator key
if not exist "keys\validator.json" (
    echo  [SETUP] No validator key found. Generating one now...
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
echo  [START] Launching validator node...
echo  [START] Boot node: 102.209.21.247 (Aztibase VPS)
echo.

:: Only initialize genesis on first boot (empty data dir).
:: Subsequent boots resume from synced chain data.
if exist "data\db" (
    echo  [START] Resuming from existing chain data...
    start /b aztibase.exe --config node.toml > node.log 2>&1
) else (
    echo  [START] First boot - initializing from genesis...
    start /b aztibase.exe --config node.toml --genesis genesis.toml > node.log 2>&1
)

:: Wait for node to boot
echo  [START] Waiting for RPC to come online...
set READY=0
for /L %%i in (1,1,15) do (
    if !READY! equ 0 (
        curl -s http://127.0.0.1:9944/health >nul 2>&1
        if !errorlevel! equ 0 (
            set READY=1
        ) else (
            timeout /t 1 /nobreak >nul
        )
    )
)

if %READY% equ 0 (
    echo.
    echo  [FAIL] Node did not start within 15 seconds.
    echo  [FAIL] Check node.log for errors.
    echo.
    echo  Common issues:
    echo    - Port 30333 or 9944 already in use
    echo    - Corrupt data directory (delete data\ and restart)
    echo    - Missing aztibase.exe in this folder
    echo.
    pause
    exit /b 1
)

:: ── Node is up — show initial status ──────────────────────────────
echo.
echo  ============================================
echo   NODE IS RUNNING
echo  ============================================
echo.
echo   RPC:       http://127.0.0.1:9944
echo   Dashboard: dashboard\index.html
echo.

:: Fetch initial chain status
call :show_status

echo.
echo  ============================================
echo   NEXT STEPS
echo  ============================================
echo.
echo   1. Open dashboard\index.html in your browser
echo   2. Install the Aztibase Wallet Chrome extension
echo   3. Connect wallet to http://127.0.0.1:9944
echo   4. Click "Faucet" to get testnet AZTB
echo   5. Click "Become Validator" (costs 500,000 AZTB)
echo   6. Wait for next epoch — you're producing blocks!
echo.
echo  ────────────────────────────────────────────
echo   Live status updates every 30 seconds.
echo   Press Ctrl+C to stop the node.
echo  ────────────────────────────────────────────
echo.

:: ── Live monitoring loop ──────────────────────────────────────────
:monitor
timeout /t 30 /nobreak >nul

:: Check if node is still running
tasklist /FI "IMAGENAME eq aztibase.exe" 2>nul | find "aztibase.exe" >nul
if %errorlevel% neq 0 (
    echo.
    echo  [STOPPED] Node process has exited.
    echo  [STOPPED] Check node.log for details.
    pause
    exit /b 1
)

call :show_status
goto monitor

:: ── Status display subroutine ─────────────────────────────────────
:show_status
:: Use /metrics/json for detailed info (block_height, peer_count, validators, epoch)
for /f "usebackq delims=" %%j in (`curl -s http://127.0.0.1:9944/metrics/json 2^>nul`) do set METRICS=%%j

:: Extract block height
set HEIGHT=?
for /f "tokens=2 delims=:," %%a in ('echo !METRICS! ^| findstr /C:"block_height"') do set HEIGHT=%%a
set HEIGHT=!HEIGHT: =!

:: Extract peer count
set PEERS=?
for /f "tokens=2 delims=:," %%a in ('echo !METRICS! ^| findstr /C:"peer_count"') do set PEERS=%%a
set PEERS=!PEERS: =!

:: Extract active validators
set VALS=?
for /f "tokens=2 delims=:," %%a in ('echo !METRICS! ^| findstr /C:"active_validators"') do set VALS=%%a
set VALS=!VALS: =!

:: Extract epoch number
set EPOCH=?
for /f "tokens=2 delims=:,}" %%a in ('echo !METRICS! ^| findstr /C:"epoch_number"') do set EPOCH=%%a
set EPOCH=!EPOCH: =!

:: Determine sync status
set SYNC=syncing
if !PEERS! equ 0 set SYNC=connecting...
if !HEIGHT! gtr 0 if !PEERS! gtr 0 set SYNC=synced

:: Get timestamp
for /f "tokens=1-2 delims= " %%a in ('echo %date% %time:~0,8%') do set TSTAMP=%%a %%b

echo  [%TSTAMP%] Block: !HEIGHT! ^| Peers: !PEERS! ^| Validators: !VALS! ^| Epoch: !EPOCH! ^| Status: !SYNC!
goto :eof
