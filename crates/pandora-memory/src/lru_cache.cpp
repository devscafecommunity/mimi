#include "lru_cache.hpp"
#include <algorithm>
#include <cmath>

namespace pandora {

LRUCache::LRUCache(size_t capacity) : capacity_(capacity) {}

bool LRUCache::get(const std::string& key, CachedNode& node) {
    auto it = cache_map_.find(key);
    if (it == cache_map_.end()) {
        return false;
    }
    
    auto node_it = it->second;
    node = *node_it;
    node.access_count++;
    node.last_accessed = std::chrono::system_clock::now().time_since_epoch().count() / 1000000;
    
    lru_list_.erase(node_it);
    lru_list_.push_front(node);
    cache_map_[key] = lru_list_.begin();
    
    return true;
}

void LRUCache::put(const std::string& key, const CachedNode& node) {
    auto it = cache_map_.find(key);
    
    if (it != cache_map_.end()) {
        lru_list_.erase(it->second);
    }
    
    lru_list_.push_front(node);
    cache_map_[key] = lru_list_.begin();
    
    if (cache_map_.size() > capacity_) {
        auto last = lru_list_.end();
        --last;
        cache_map_.erase(last->id);
        lru_list_.pop_back();
    }
}

void LRUCache::evict_expired() {
    decay_temperature();
}

void LRUCache::decay_temperature() {
    auto now = std::chrono::system_clock::now().time_since_epoch().count() / 1000000;
    const double decay_rate = 0.0001;
    
    for (auto& node : lru_list_) {
        uint64_t time_delta = now - node.last_accessed;
        node.temperature *= std::exp(-decay_rate * time_delta);
    }
}

size_t LRUCache::size() const {
    return cache_map_.size();
}

size_t LRUCache::capacity() const {
    return capacity_;
}

}
