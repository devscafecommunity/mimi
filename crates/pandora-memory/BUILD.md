# Pandora Memory Manager - C++ Build Guide

## Prerequisites

- C++20 compatible compiler (GCC 10+, Clang 12+, MSVC 2019+)
- CMake 3.20+
- Neo4j Bolt Driver for C++ (optional for full build)

## Build Instructions

### Linux/macOS

```bash
cd crates/pandora-memory
chmod +x build.sh
./build.sh
```

### Windows (MSVC)

```cmd
cd crates\pandora-memory
mkdir build
cd build
cmake -DCMAKE_BUILD_TYPE=Release -G "Visual Studio 16 2019" ..
cmake --build . --config Release
ctest --output-on-failure
```

### Windows (MinGW/GCC)

```cmd
cd crates\pandora-memory
mkdir build
cd build
cmake -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_COMPILER=g++ ..
cmake --build .
ctest --output-on-failure
```

## Build Targets

- `pandora-memory` — Main library
- `test_lru_cache` — LRU cache unit tests
- `test_memory_manager` — Memory manager unit tests

## Key Features

- **LRU Cache**: L1 in-memory cache for Neo4j context with thermal decay
- **Memory Manager**: Main interface for Neo4j operations
- **C++20**: Modern C++ with concepts and ranges
- **Catch2**: Comprehensive unit test framework

## Neo4j Setup (Optional)

For full integration testing:

```bash
docker run -d \
  -p 7687:7687 \
  -e NEO4J_AUTH=neo4j/password \
  neo4j:latest
```

Then update connection string in tests:
```cpp
mgr.initialize("bolt://localhost:7687", "neo4j", "password");
```
