@echo off
setlocal

set VERSION=%1
if "%VERSION%"=="" set VERSION=0.1.0

set ROOT_DIR=%~dp0..\..
set BIN_DIR=%ROOT_DIR%\target\x86_64-pc-windows-msvc\release
set DIST_DIR=%ROOT_DIR%\dist\wix

if not exist "%DIST_DIR%" mkdir "%DIST_DIR%"

candle.exe -dVersion=%VERSION% -dBinDir=%BIN_DIR% -dRootDir=%ROOT_DIR% -out "%DIST_DIR%\sentinel.wixobj" "%~dp0sentinel.wxs"
light.exe -out "%DIST_DIR%\sentinel-%VERSION%.msi" "%DIST_DIR%\sentinel.wixobj"

echo MSI package: %DIST_DIR%\sentinel-%VERSION%.msi
