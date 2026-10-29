@echo off
REM Valheim Modpack Studio - Windows Build Script
REM Prerequisites: Rust (rustup) installed on Windows (https://rustup.rs/)

echo ========================================================
echo   Valheim Modpack Studio - Windows Build (Release)
echo ========================================================

cargo build --release

if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed!
    exit /b %ERRORLEVEL%
)

if not exist "dist" mkdir dist

copy /Y "target\release\valheim-modpack-studio.exe" "dist\ValheimModpackStudio-0.1.4-x86_64.exe"

echo.
echo [SUCCESS] Binary created at dist\ValheimModpackStudio-0.1.4-x86_64.exe
echo ========================================================
pause
