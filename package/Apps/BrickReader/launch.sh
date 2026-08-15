#!/bin/sh
cd $(dirname "$0")

# 赋予可执行权限
chmod +x ./bin/brick_reader

# 导出动态库路径
export LD_LIBRARY_PATH=./lib:$LD_LIBRARY_PATH

# 启动 BrickReader，并将运行日志重定向至当前目录
./bin/brick_reader > log.txt 2>&1
