@echo off
title Aztibase Website
echo ============================================
echo   Aztibase Network - Website
echo ============================================
echo.
echo   Starting dev server...
echo   URL: http://localhost:3000
echo.
echo   Press Ctrl+C to stop
echo ============================================
echo.

cd /d "%~dp0"
npm run dev
pause
