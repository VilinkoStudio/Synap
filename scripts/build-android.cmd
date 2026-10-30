@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-android.ps1" %*
