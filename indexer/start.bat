@echo off
title Aztibase Indexer
echo ============================================
echo   Aztibase Network - Transaction Indexer
echo ============================================
echo.

cd /d "%~dp0"

REM Delete stale DB on fresh chain restarts
if "%1"=="--fresh" (
    echo   Deleting old indexer.db for fresh start...
    del /f /q indexer.db 2>nul
)

echo   RPC:  %AZTB_RPC% (default: http://102.209.21.247:9944)
echo   API:  http://localhost:3001
echo   DB:   indexer.db
echo.
echo   Press Ctrl+C to stop
echo ============================================
echo.

npm run dev
pause
