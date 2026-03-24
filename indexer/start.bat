@echo off
title Aztibase Indexer
echo ============================================
echo   Aztibase Network - Transaction Indexer
echo ============================================
echo.
echo   Starting indexer...
echo   API: http://localhost:3001
echo.
echo   Press Ctrl+C to stop
echo ============================================
echo.

cd /d "%~dp0"
npm run dev
pause
