.PHONY: build debug release test fmt clippy clean help

# 默认目标
.DEFAULT_GOAL := help

# 可覆盖的变量
CARGO ?= cargo

# 构建调试版本
build:
	@echo "Building debug version..."
	@$(CARGO) build

debug: build

release:
	@echo "Building release binary..."
	@$(CARGO) build --release

# 运行测试
test:
	@echo "Running tests..."
	@$(CARGO) test

test-verbose:
	@echo "Running tests (verbose)..."
	@$(CARGO) test -- --nocapture

# 格式化代码
fmt:
	@echo "Formatting code..."
	@$(CARGO) fmt

# 运行 Clippy 静态检查
clippy:
	@echo "Running Clippy checks..."
	@$(CARGO) clippy -- -D warnings

# 清理构建文件
clean:
	@echo "Cleaning build files..."
	@$(CARGO) clean

# 显示帮助信息
help:
	@echo "Available targets:"
	@echo "  build        - Build debug version"
	@echo "  debug        - Alias for build"
	@echo "  release      - Build release version"
	@echo "  test         - Run tests"
	@echo "  test-verbose - Run tests with verbose output"
	@echo "  fmt          - Format code"
	@echo "  clippy       - Run Clippy static analysis"
	@echo "  clean        - Clean build files"
	@echo "  help         - Show this help message"
