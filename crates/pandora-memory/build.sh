#!/bin/bash

mkdir -p build
cd build

cmake -DCMAKE_BUILD_TYPE=Release \
      -DCMAKE_C_COMPILER=gcc \
      -DCMAKE_CXX_COMPILER=g++ \
      -DBUILD_TESTS=ON \
      ..

cmake --build . -j$(nproc)
ctest --output-on-failure
