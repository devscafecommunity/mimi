#include <catch2/catch_test_macros.hpp>
#include "lru_cache.hpp"

TEST_CASE("LRUCache initialization", "[cache]") {
    pandora::LRUCache cache(100);
    REQUIRE(cache.capacity() == 100);
    REQUIRE(cache.size() == 0);
}

TEST_CASE("LRUCache put and get", "[cache]") {
    pandora::LRUCache cache(10);
    
    pandora::CachedNode node;
    node.id = "test1";
    node.data = "data1";
    node.temperature = 1.0;
    node.last_accessed = 0;
    node.access_count = 0;
    
    cache.put("test1", node);
    REQUIRE(cache.size() == 1);
    
    pandora::CachedNode retrieved;
    bool found = cache.get("test1", retrieved);
    REQUIRE(found);
    REQUIRE(retrieved.id == "test1");
    REQUIRE(retrieved.data == "data1");
}

TEST_CASE("LRUCache eviction on overflow", "[cache]") {
    pandora::LRUCache cache(2);
    
    pandora::CachedNode node1, node2, node3;
    node1.id = "node1";
    node2.id = "node2";
    node3.id = "node3";
    
    cache.put("key1", node1);
    cache.put("key2", node2);
    REQUIRE(cache.size() == 2);
    
    cache.put("key3", node3);
    REQUIRE(cache.size() == 2);
    
    pandora::CachedNode retrieved;
    bool found = cache.get("key1", retrieved);
    REQUIRE(!found);
}

TEST_CASE("LRUCache miss", "[cache]") {
    pandora::LRUCache cache(10);
    
    pandora::CachedNode node;
    bool found = cache.get("nonexistent", node);
    REQUIRE(!found);
}
