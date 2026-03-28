@echo off
REM Quick start script for hyper-static-server development (Windows)

setlocal enabledelayedexpansion

echo 🚀 hyper-static-server Quick Start
echo ==================================
echo.

REM Check Rust installation
where cargo >nul 2>nul
if errorlevel 1 (
    echo ❌ Rust not found. Install from: https://rustup.rs/
    exit /b 1
)

for /f "tokens=*" %%i in ('rustc --version') do set RUST_VERSION=%%i
echo ✅ Rust found: %RUST_VERSION%
echo.

REM Build project
echo 📦 Building project...
cargo build
if errorlevel 1 (
    echo ❌ Build failed
    exit /b 1
)
echo ✅ Build complete
echo.

REM Run tests
echo 🧪 Running tests...
cargo test --lib
if errorlevel 1 (
    echo ❌ Tests failed
    exit /b 1
)
echo ✅ Tests passed
echo.

REM Check code quality
echo 🔍 Checking code quality...
cargo fmt --check
if errorlevel 1 echo ⚠️  Format issues found (run 'cargo fmt' to fix)
echo.

cargo clippy -- -D warnings
if errorlevel 1 echo ⚠️  Clippy warnings found (see above)
echo.

REM Next steps
echo 🎯 Next Steps:
echo 1. Open in VS Code: code .
echo 2. Start Claude Code: Ctrl+Shift+I
echo 3. Copy the prompt from .claude\PROJECT_SETUP_GUIDE.md
echo 4. Review CLAUDE.md for development rules
echo.

echo 📋 Check TASK.md for current tasks
echo 📚 Read PLANNING.md for architecture decisions
echo 📖 Read README.md for usage examples
echo.

echo ✨ Ready to start development!
echo.

pause
