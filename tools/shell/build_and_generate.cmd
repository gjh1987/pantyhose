@echo off
REM Combined script to build proto-id-tool and generate message IDs
REM This script performs both build and generation in one step

echo ========================================
echo Proto ID Tool - Build and Generate
echo ========================================

REM Call build script
echo.
echo Step 1: Building proto-id-tool...
echo ----------------------------------------
call "%~dp0build_protobuf_message_id.cmd"

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Build failed! Aborting...
    exit /b 1
)

REM Call generate script
echo.
echo Step 2: Generating message IDs...
echo ----------------------------------------
call "%~dp0generate_protobuf_message_id.cmd"

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ERROR: Generation failed!
    exit /b 1
)

echo.
echo ========================================
echo Complete! All steps finished successfully.
echo ========================================