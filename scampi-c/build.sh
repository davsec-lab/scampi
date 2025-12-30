#!/bin/sh

rm -rf build
mkdir build
cd build

cmake -DLLVM_DIR=/usr/lib/llvm-18/lib/cmake/llvm ..
make