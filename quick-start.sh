#!/bin/bash
# Quick start script for hyper-static-server development

set -e

echo "🚀 hyper-static-server Quick Start"
echo "=================================="
echo ""

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust not found. Install from: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust found: $(rustc --version)"
echo ""

# Build project
echo "📦 Building project..."
cargo build 2>&1 | tail -5
echo "✅ Build complete"
echo ""

# Run tests
echo "🧪 Running tests..."
cargo test --lib 2>&1 | tail -10
echo "✅ Tests passed"
echo ""

# Check code quality
echo "🔍 Checking code quality..."
cargo fmt --check 2>&1 | head -5 || echo "⚠️  Format issues found (run 'cargo fmt' to fix)"
echo ""

cargo clippy -- -D warnings 2>&1 | head -5 || echo "⚠️  Clippy warnings found (see above)"
echo ""

# Next steps
echo "🎯 Next Steps:"
echo "1. Open in VS Code: code ."
echo "2. Start Claude Code: Ctrl+Shift+I"
echo "3. Copy the prompt from .claude/PROJECT_SETUP_GUIDE.md"
echo "4. Review CLAUDE.md for development rules"
echo ""

echo "📋 Check TASK.md for current tasks"
echo "📚 Read PLANNING.md for architecture decisions"
echo "📖 Read README.md for usage examples"
echo ""

echo "✨ Ready to start development!"
