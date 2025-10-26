@echo off
echo Building pantyhose_server_tools...

REM Save current directory
set ORIGINAL_DIR=%CD%

REM Navigate to the pantyhose_server_tools directory (project root, not src-tauri)
cd /d "%~dp0\..\pantyhose_server_tools"

echo Running yarn tauri build...
call yarn tauri build

if %errorlevel% neq 0 (
    cd /d "%ORIGINAL_DIR%"
    echo Build failed!
    exit /b 1
)

echo Build successful!

echo Copying executable to tools\bin...
REM Due to .cargo/config.toml, the output is in project root bin/target/release
copy "bin\target\release\pantyhose_server_tool.exe" "..\bin\pantyhose_server_tool.exe" /Y

if %errorlevel% neq 0 (
    echo Copy failed!
    exit /b 1
)

echo Successfully copied pantyhose_server_tool.exe to tools\bin

REM Return to original directory
cd /d "%ORIGINAL_DIR%"
echo Build complete!