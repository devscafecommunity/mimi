# Pandora Memory Manager

High-performance memory management engine for MiMi with Neo4j graph integration.

## Directory Structure

```
.
├── include/          - Public headers
│   ├── pandora_memory.hpp    - Main MemoryManager interface
│   └── lru_cache.hpp         - LRU cache with thermal decay
├── src/             - Implementation files
│   ├── pandora_memory.cpp    - MemoryManager implementation
│   └── lru_cache.cpp         - Cache implementation
├── tests/           - Unit tests (Catch2)
│   ├── test_memory_manager.cpp
│   └── test_lru_cache.cpp
├── cmake/           - CMake modules
│   └── Dependencies.cmake
├── CMakeLists.txt   - Main build configuration
├── build.sh         - Build script (Unix-like)
└── BUILD.md         - Build documentation
```

## Components

### LRU Cache (`lru_cache.hpp`)
- Capacity-limited in-memory cache (default 1000 nodes)
- Thermal decay for stale data removal
- O(1) get/put operations

### Memory Manager (`pandora_memory.hpp`)
- Neo4j Bolt driver integration
- Connection pooling
- Transaction management

## Building

See `BUILD.md` for detailed instructions.

## Testing

```bash
cd build
ctest --output-on-failure
```

## Dependencies

- **Catch2** v3.4+ — Unit test framework
- **Neo4j C++ Driver** (optional) — For full integration
