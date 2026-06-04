@echo off
setlocal
echo.
echo  ╔══════════════════════════════════╗
echo  ║     Bauer Browser — Windows      ║
echo  ╚══════════════════════════════════╝
echo.

:: ── Check Rust ───────────────────────────────────────────────────────────────
where cargo >nul 2>&1
if errorlevel 1 (
    echo  [ERROR] Rust/Cargo not found.
    echo.
    echo  Install Rust from: https://rustup.rs
    echo  Then restart this terminal and run build.bat again.
    echo.
    pause
    exit /b 1
)

for /f "tokens=*" %%v in ('cargo --version') do echo  ✔ %%v

:: ── WebView2 (auto-installed on Win10/11 via Windows Update) ─────────────────
echo  ✔ WebView2: present on Windows 10 build 1803+ by default

:: ── Create dirs ──────────────────────────────────────────────────────────────
if not exist "config\settings.toml" (
    copy "config\defaults.toml" "config\settings.toml" >nul
    echo  ✔ Created config\settings.toml
)

:: ── Build ─────────────────────────────────────────────────────────────────────
echo.
if "%1"=="release" (
    echo  Building release ^(optimized^)...
    cargo build --release
    if errorlevel 1 goto :build_failed
    echo.
    echo  ✔ Build successful: target\release\bauer-browser.exe
    if "%2"=="run" target\release\bauer-browser.exe
) else (
    echo  Building debug...
    cargo build
    if errorlevel 1 goto :build_failed
    echo.
    echo  ✔ Build successful: target\debug\bauer-browser.exe
    if "%1"=="run" target\debug\bauer-browser.exe
    if "%2"=="run" target\debug\bauer-browser.exe
)
goto :eof

:build_failed
echo.
echo  [ERROR] Build failed. Check errors above.
echo.
echo  Common issues:
echo    - Missing WebView2 SDK: cargo will download automatically
echo    - First build downloads all crates (~100MB), takes 2-5 min
pause
exit /b 1
