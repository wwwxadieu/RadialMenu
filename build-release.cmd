@echo off
setlocal
cd /d "%~dp0"
title Radial Menu - build bo cai
if not exist logs mkdir logs
del logs\release-status.txt 2>nul

echo ============================================
echo  Radial Menu - build ban release + bo cai
echo ============================================
echo.

where node >nul 2>&1 || (echo   Thieu Node.js. Cai Node 20.19 tro len roi chay lai. & goto :fail)
where cargo >nul 2>&1 || (echo   Thieu Rust. Cai tu https://rustup.rs roi chay lai. & goto :fail)

echo [1/3] npm install...
call npm install --no-audit --no-fund > logs\npm-install.log 2>&1
if errorlevel 1 (
  echo   npm install loi, dang sua cache npm roi thu lai...
  call npm cache verify > logs\npm-cache.log 2>&1
  call npm install --no-audit --no-fund --prefer-online > logs\npm-install.log 2>&1
)
if errorlevel 1 (echo   Loi khi npm install. Xem logs\npm-install.log & goto :fail)
echo   OK.
echo.

echo [2/3] Build release va dong goi bo cai NSIS (lan dau mat 10-15 phut)...
echo   Lan dau Tauri tu tai bo NSIS ve. Tien do ghi vao logs\release.log
call npm run tauri build > logs\release.log 2>&1
if errorlevel 1 (echo   Build loi. Xem logs\release.log & goto :fail)
echo   OK.
echo.

echo [3/3] Mo thu muc chua bo cai...
set "OUT=src-tauri\target\release\bundle\nsis"
if not exist "%OUT%" (echo   Khong thay thu muc %OUT%. Xem logs\release.log & goto :fail)
dir /b "%OUT%\*.exe" > logs\release-files.txt
start "" explorer "%OUT%"
echo   Bo cai: %OUT%
type logs\release-files.txt
echo   Bam dup file -setup.exe de cai (can quyen Admin).
echo.
echo BUILD_OK> logs\release-status.txt
pause
exit /b 0

:fail
echo BUILD_FAIL> logs\release-status.txt
echo.
pause
exit /b 1
