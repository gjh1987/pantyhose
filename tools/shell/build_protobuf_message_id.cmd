@echo off
REM Build script for proto-id-tool
REM This script builds the proto-id-tool and copies it to tools/bin

echo ========================================
echo Building proto-id-tool...
echo ========================================

REM Save current directory
set ORIGINAL_DIR=%CD%

REM Navigate to the proto-id-tool directory
cd /d "%~dp0\..\proto\protoIdTool"

REM Build the tool in release mode
echo.
echo Building in release mode...
cargo build --release --target-dir .

REM Check if build was successful
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Build failed!
    cd /d "%ORIGINAL_DIR%"
    exit /b 1
)

REM Create tools/bin directory if it doesn't exist
if not exist "..\..\bin" mkdir "..\..\bin"

REM Copy the executable to tools/bin
echo.
echo Copying proto-id-tool.exe to tools\bin...
copy /Y "release\proto-id-tool.exe" "..\..\bin\" >nul

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Failed to copy executable!
    cd /d "%ORIGINAL_DIR%"
    exit /b 1
)

echo.
echo ========================================
echo Build successful!
echo Executable location: tools\bin\proto-id-tool.exe
echo ========================================

REM Return to original directory
cd /d "%ORIGINAL_DIR%"

echo.
echo Usage: tools\bin\proto-id-tool.exe --proto-path [path] --language [rust] --output-path [path]
echo Example: tools\bin\proto-id-tool.exe --proto-path tools\proto\config --language rust --output-path src\proto\messages
echo.