@echo off
chcp 65001 >nul
REM Self-elevate: if not admin, relaunch this .bat as admin (accept the UAC prompt).
net session >nul 2>&1
if %errorlevel% neq 0 (
  echo Requesting administrator rights - please click Yes on the UAC prompt...
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)

set "DST=C:\Program Files\Unterm\unterm.exe"
set "SRC=D:\code\unterm\target\release\unterm.exe"

echo Installing new Unterm...
if not exist "%SRC%" ( echo ERROR: build not found at "%SRC%" & pause & exit /b 1 )

REM Rename the old binary out of the way (works even while Unterm is running),
REM then drop the new one in. Running instances keep using the old file until
REM you restart them.
if exist "C:\Program Files\Unterm\unterm.exe.bak-0.50.2" del /F /Q "C:\Program Files\Unterm\unterm.exe.bak-0.50.2"
if exist "%DST%" ren "%DST%" "unterm.exe.bak-0.50.2"
copy /Y "%SRC%" "%DST%"

echo.
if exist "%DST%" (
  echo Done. Close any open Unterm window and reopen it from the Start menu.
) else (
  echo ERROR: copy failed.
)
pause
