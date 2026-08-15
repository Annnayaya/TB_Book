#!/bin/bash
# 一键为 Trimui Brick (ARM64 Linux) 构建发布包
set -e

# 自动加载 Rust 环境变量 (如果存在)
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

# 检查是否安装了 rustup
if ! command -v rustup &> /dev/null; then
    echo "================================================================"
    echo "  [错误] WSL 环境中尚未安装 Rust 工具链！"
    echo "  请在 WSL 终端中运行以下命令安装："
    echo "    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
    echo "    source \"\$HOME/.cargo/env\""
    echo "================================================================"
    exit 1
fi

# 检查 aarch64 交叉编译器
if ! command -v aarch64-linux-gnu-gcc &> /dev/null; then
    echo "==> 正在安装 aarch64 交叉编译工具链..."
    sudo apt update && sudo apt install -y gcc-aarch64-linux-gnu
fi

# 指定交叉编译 C 编译器与链接器环境变量
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc

TARGET="aarch64-unknown-linux-gnu"

echo "==> [1/3] 正在配置 $TARGET 目标平台..."
rustup target add $TARGET

echo "==> [2/3] 正在为 Trimui Brick (A133P) 编译 Release 二进制..."
cargo build --release --target $TARGET

echo "==> [3/3] 正在组装掌机安装包..."
mkdir -p package/Apps/BrickReader/bin
cp target/$TARGET/release/brick_reader package/Apps/BrickReader/bin/
chmod +x package/Apps/BrickReader/bin/brick_reader
chmod +x package/Apps/BrickReader/launch.sh

echo "================================================================"
echo "  ✓ 构建成功！"
echo "  可执行文件: package/Apps/BrickReader/bin/brick_reader"
echo "  安装方式: 将 package/Apps/BrickReader 文件夹复制到 TF 卡的 Apps/ 目录下即可！"
echo "================================================================"
