@echo off
REM Script to generate protobuf message ID file
REM This script runs the proto-id-tool to generate message ID mappings

echo ========================================
echo Generating Protobuf Message IDs...
echo ========================================

REM Save current directory
set ORIGINAL_DIR=%CD%

REM Navigate to tools directory
cd /d "%~dp0\.."

REM Check if proto-id-tool exists in bin directory
if not exist "bin\proto-id-tool.exe" (
    echo.
    echo ERROR: proto-id-tool.exe not found in tools\bin!
    echo Please run build_protobuf_message_id.cmd first.
    cd /d "%ORIGINAL_DIR%"
    exit /b 1
)

REM Run the tool to generate message IDs
echo.
echo Running proto-id-tool...
echo Input: proto\config
echo Output: ..\src\proto\messages\protobuf_message_id.rs
echo.

bin\proto-id-tool.exe --proto-path proto\config --language rust --output-path ..\src\proto\messages --length-bytes 2

REM Check if generation was successful
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Failed to generate message IDs!
    cd /d "%ORIGINAL_DIR%"
    exit /b 1
)

echo.
echo ========================================
echo Message ID generation successful!
echo Generated file: src\proto\messages\protobuf_message_id.rs
echo ========================================

REM Return to original directory
cd /d "%ORIGINAL_DIR%"