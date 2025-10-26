@echo off
setlocal enabledelayedexpansion

echo =============================================
echo Building Pantyhose Server with Notify System...
echo =============================================

:: Change to project root directory where Cargo.toml is located
cd /d "%~dp0.."

:: Verify Cargo.toml exists
if not exist "Cargo.toml" (
    echo ERROR: Cargo.toml not found in current directory!
    echo Current directory: %cd%
    pause
    exit /b 1
)

:: Clean current bin directory executables
echo Cleaning bin directory...
if exist "pantyhose.exe" (
    del /q "pantyhose.exe"
    echo Removed old executables
)

:: Check code first
echo.
echo Checking code...
cargo check
if %errorlevel% neq 0 (
    echo.
    echo =============================================
    echo CODE CHECK FAILED!
    echo =============================================
    pause
    exit /b 1
)

:: Build the project
echo.
echo Building pantyhose server with optimizations...
cargo build --release --bin pantyhose

:: Check build result
if %errorlevel% neq 0 (
    echo.
    echo =============================================
    echo BUILD FAILED!
    echo =============================================
    pause
    exit /b 1
)

:: Verify build output
if not exist "bin\target\release\pantyhose.exe" (
    echo.
    echo =============================================
    echo ERROR: pantyhose.exe not found after build!
    echo =============================================
    pause
    exit /b 1
)

:: Copy executable to bin directory
echo.
echo Copying executable to bin directory...
copy "bin\target\release\pantyhose.exe" "%~dp0pantyhose.exe" >nul
if %errorlevel% equ 0 (
    echo Successfully copied pantyhose.exe to bin directory
) else (
    echo Failed to copy pantyhose.exe
    pause
    exit /b 1
)

:: Display results
echo.
echo =============================================
echo BUILD COMPLETED SUCCESSFULLY!
echo =============================================
echo.
echo Executable: %~dp0pantyhose.exe
for %%f in ("%~dp0pantyhose.exe") do (
    echo Size: %%~zf bytes
    echo Modified: %%~tf
)

echo Features:
echo - Server.notify mechanism for event-driven architecture
echo - NetworkEngine integration with TCP/WebSocket servers
echo - Async notify triggers on network events
echo.
echo To run: pantyhose.exe [server_id]
echo Example: pantyhose.exe 1001
echo.
pause