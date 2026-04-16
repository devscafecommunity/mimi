#include <catch2/catch_test_macros.hpp>
#include "pandora_memory.hpp"

TEST_CASE("MemoryManager initialization", "[memory_manager]") {
    pandora::MemoryManager mgr;
    REQUIRE(!mgr.is_connected());
    
    bool result = mgr.initialize("bolt://localhost:7687", "neo4j", "password");
    REQUIRE(result);
    REQUIRE(mgr.is_connected());
    
    result = mgr.shutdown();
    REQUIRE(result);
    REQUIRE(!mgr.is_connected());
}
