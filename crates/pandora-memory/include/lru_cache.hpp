#pragma once

#include <string>
#include <unordered_map>
#include <list>
#include <memory>
#include <chrono>

namespace pandora {

struct CachedNode {
    std::string id;
    std::string data;
    double temperature;
    uint64_t last_accessed;
    size_t access_count;
};

class LRUCache {
public:
    LRUCache(size_t capacity = 1000);
    
    bool get(const std::string& key, CachedNode& node);
    void put(const std::string& key, const CachedNode& node);
    void evict_expired();
    
    size_t size() const;
    size_t capacity() const;
    
private:
    void decay_temperature();
    
    size_t capacity_;
    std::unordered_map<std::string, std::list<CachedNode>::iterator> cache_map_;
    std::list<CachedNode> lru_list_;
};

}
