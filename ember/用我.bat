@echo off
chcp 65001 >nul
title EMBER
cd /d "%~dp0"
echo ============================================
echo   EMBER - 输入问题回车提问（可多轮工具）
echo   直接关窗口退出；输入 c + 回车 = 接着上次说
echo ============================================
echo.
:loop
set "Q="
set /p Q=你: 
if "%Q%"=="" goto :eof
if /i "%Q%"=="c" (
  set "Q2="
  set /p Q2=追问: 
  "C:\Users\87465\.workbuddy\binaries\python\versions\3.13.12\python.exe" "%~dp0ember.py" -c "%Q2%"
) else (
  "C:\Users\87465\.workbuddy\binaries\python\versions\3.13.12\python.exe" "%~dp0ember.py" "%Q%"
)
echo.
echo --------------------------------------------
goto :loop
