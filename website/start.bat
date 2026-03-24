@echo off
title Aztibase Docs Site
echo ============================================
echo   Aztibase Network - Documentation Site
echo ============================================
echo.
echo   Starting dev server...
echo   URL: http://localhost:4321
echo.
echo   Press Ctrl+C to stop
echo ============================================
echo.

cd /d "%~dp0"
npm run dev
pause
