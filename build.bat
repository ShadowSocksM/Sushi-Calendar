@echo off
setlocal
chcp 65001>nul
title Sushi Calendar v2.5.95 Offline Builder
echo ==================================================
echo Sushi Calendar v2.5.95
echo ==================================================
echo [1/2] Building Rust Release using LOCAL Cargo cache...
set "CARGO_NET_OFFLINE=true"
cargo build --release --offline
if errorlevel 1 (
 echo.
 echo ERROR: Rust compilation failed. Read the compiler errors above.
 echo This does NOT necessarily indicate missing offline dependencies.
 pause
 exit /b 1
)
if not exist Release mkdir Release
copy /Y target\release\mr_calendar.exe Release\Sushi_Calendar_v2.5.95_Portable.exe >nul

echo [2/2] Looking for Inno Setup 6...
set "ISCC="
if exist "%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe" set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if exist "%ProgramFiles%\Inno Setup 6\ISCC.exe" set "ISCC=%ProgramFiles%\Inno Setup 6\ISCC.exe"
if exist "%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe" set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
if defined ISCC goto :setup
for /f "delims=" %%I in ('where ISCC.exe 2^>nul') do if not defined ISCC set "ISCC=%%I"

:setup
if defined ISCC (
 echo Found: %ISCC%
 "%ISCC%" installer.iss
 if errorlevel 1 (
   echo ERROR: Setup build failed.
 ) else (
   echo Setup created successfully.
 )
) else (
 echo.
 echo WARNING: Inno Setup 6 was NOT found.
 echo Portable EXE was created, but Setup EXE cannot be compiled.
 echo Install Inno Setup 6 and run this build.bat again.
)

echo.
echo OUTPUT:
echo Release\Sushi_Calendar_v2.5.95_Portable.exe
echo Release\Sushi_Calendar_v2.5.95_Setup.exe
echo.
pause
