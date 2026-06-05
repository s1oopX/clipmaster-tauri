@echo off
REM ClipMaster - Git 推送脚本
REM 使用方法: push.bat "你的提交信息"

echo ========================================
echo   ClipMaster - Git Push Script
echo ========================================
echo.

cd /d "%~dp0"

REM 检查是否提供了提交信息
if "%~1"=="" (
    echo [错误] 请提供提交信息
    echo 使用方法: push.bat "你的提交信息"
    pause
    exit /b 1
)

echo [1/4] 检查 Git 状态...
git status --short

echo.
echo [2/4] 添加所有更改...
git add .

echo.
echo [3/4] 提交更改...
git commit -m "%~1

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"

if %ERRORLEVEL% NEQ 0 (
    echo [提示] 没有更改需要提交
    pause
    exit /b 0
)

echo.
echo [4/4] 推送到 GitHub...
git push origin master

if %ERRORLEVEL% EQU 0 (
    echo.
    echo ========================================
    echo   ✓ 推送成功！
    echo   查看: https://github.com/s1oopX/clipmaster-tauri
    echo ========================================
) else (
    echo.
    echo ========================================
    echo   ✗ 推送失败
    echo ========================================
)

pause
