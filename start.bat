@echo off
echo ========================================
echo   ClipMaster - Tauri 开发模式启动
echo ========================================
echo.

cd /d "%~dp0"

REM 刷新环境变量（添加 Cargo 到 PATH）
echo [提示] 正在加载 Rust 环境...
if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
    set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
    echo [成功] Cargo 已加载
) else (
    echo [错误] 未找到 Cargo
    echo 请确认 Rust 已安装: %USERPROFILE%\.cargo\bin\cargo.exe
    echo.
    echo 安装 Rust: https://rustup.rs/
    pause
    exit /b 1
)

echo.

REM 检查 Node.js
where node >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [错误] 未找到 Node.js
    echo 请先安装 Node.js: https://nodejs.org/
    pause
    exit /b 1
)

echo [1/3] 检查依赖...
if not exist "node_modules" (
    echo [提示] 正在安装 npm 依赖...
    call npm install
)

echo.
echo [2/3] 启动中...
echo [提示] 首次启动需要编译 Rust 代码，请耐心等待（可能需要 5-10 分钟）...
echo [提示] 如果编译失败，请在新的命令提示符窗口中运行此脚本
echo.

REM 启动 Tauri 开发服务器
call npm run tauri:dev

echo.
echo ========================================
echo 应用已关闭
echo ========================================
pause
