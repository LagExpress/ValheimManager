@echo off
REM Valheim Modpack Studio - Windows Build Script
REM Prerequisites: Rust (rustup) installed on Windows (https://rustup.rs/)
cd /d "%~dp0.."
set "VERSION="
for /f "tokens=3" %%V in ('findstr /b /c:"version =" Cargo.toml') do set "VERSION=%%~V"
if not defined VERSION (
    echo [ERROR] Could not read the app version from Cargo.toml
    exit /b 1
)

echo ========================================================
echo   Valheim Modpack Studio - Windows Build (Release)
echo ========================================================

cargo build --release

if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Build failed!
    exit /b %ERRORLEVEL%
)

if not exist "dist" mkdir dist

copy /Y "target\release\valheim-modpack-studio.exe" "dist\ValheimManager-%VERSION%.exe"
if errorlevel 1 exit /b 1

echo.
echo [SUCCESS] Binary created at dist\ValheimManager-%VERSION%.exe
echo ========================================================
pause
