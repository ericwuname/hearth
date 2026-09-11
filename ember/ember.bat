@echo off
chcp 65001 >nul
set PY=C:\Users\87465\.workbuddy\binaries\python\versions\3.13.12\python.exe
"%PY%" "%~dp0ember.py" %*
