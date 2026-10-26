@echo off
setlocal
cd /d "%~dp0"
title Radial Menu - build
if not exist logs mkdir logs
del logs\status.txt 2>nul

echo ============================================
echo  Radial Menu - build ban debug va chay thu
echo ============================================
echo.

echo Dong ban Radial Menu dang chay (neu co)...
taskkill /im radial-menu.exe /f >nul 2>&1
echo.

echo [1/4] Kiem tra cong cu...
> logs\tools.log (
  echo --- node & where node & node --version
  echo --- npm & where npm & call npm --version
  echo --- cargo & where cargo & cargo --version
  echo --- rustc & rustc --version & rustup show active-toolchain
  echo --- link.exe & where link.exe
) 2>&1
where node >nul 2>&1 || (echo   Thieu Node.js. Cai Node 20.19 tro len roi chay lai. & goto :fail)
where cargo >nul 2>&1 || (echo   Thieu Rust. Cai tu https://rustup.rs roi chay lai. & goto :fail)
echo   OK. Chi tiet: logs\tools.log
echo.

echo [2/4] npm install...
call npm install --no-audit --no-fund > logs\npm-install.log 2>&1
if errorlevel 1 (
  echo   npm install loi, dang sua cache npm roi thu lai...
  call npm cache verify > logs\npm-cache.log 2>&1
  call npm install --no-audit --no-fund --prefer-online > logs\npm-install.log 2>&1
)
if errorlevel 1 (echo   Loi khi npm install. Xem logs\npm-install.log & goto :fail)
echo   OK.
echo.

echo [3/4] Build Rust + giao dien (lan dau mat 5-10 phut)...
echo   Tien do ghi vao logs\build.log
call npm run tauri build -- --debug --no-bundle > logs\build.log 2>&1
if errorlevel 1 (echo   Build loi. Xem logs\build.log & goto :fail)
echo   OK.
echo.

echo [4/4] Mo Radial Menu...
start "" "src-tauri\target\debug\radial-menu.exe"
echo   Da mo. App chay o khay he thong, cua so Nhat ky hien ra.
echo.
echo BUILD_OK> logs\status.txt
pause
exit /b 0

:fail
echo BUILD_FAIL> logs\status.txt
echo.
pause
exit /b 1
