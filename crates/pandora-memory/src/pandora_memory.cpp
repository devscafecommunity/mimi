#include "pandora_memory.hpp"

#include "bolt_client.hpp"
#include "heatmap.hpp"
#include "lru_cache.hpp"

#include <chrono>

namespace pandora {

class MemoryManager::Impl {
public:
    bool initialize(const std::string& uri,
                    const std::string& user,
                    const std::string& pass,
                    const MemoryManager::BootstrapConfig& config) {
        bolt::Config bolt_cfg;
        bolt_cfg.uri = uri;
        bolt_cfg.username = user;
        bolt_cfg.password = pass;

        pool_ = std::make_unique<bolt::ConnectionPool>(std::move(bolt_cfg));
        if (!pool_->connect()) {
            return false;
        }

        auto ping = pool_->run("RETURN 1 AS ok", {});
        if (!ping.ok()) {
            pool_->disconnect();
            pool_.reset();
            return false;
        }

        lru_ = std::make_unique<LRUCache>(config.l1_capacity);
        heatmap_ = std::make_unique<HeatmapManager>(
            config.heatmap_lambda, config.heatmap_threshold, 500);
        return true;
    }

    bool shutdown() {
        if (pool_) {
            pool_->disconnect();
            pool_.reset();
        }
        lru_.reset();
        heatmap_.reset();
        return true;
    }

    bool is_connected() const { return pool_ && pool_->is_healthy(); }

    std::size_t l1_size() const { return lru_ ? lru_->size() : 0; }

    std::size_t heatmap_size() const { return heatmap_ ? heatmap_->size() : 0; }

    std::size_t queries_executed() const {
        return pool_ ? pool_->stats().queries_executed : 0;
    }

private:
    std::unique_ptr<bolt::ConnectionPool> pool_;
    std::unique_ptr<LRUCache> lru_;
    std::unique_ptr<HeatmapManager> heatmap_;
};

MemoryManager::MemoryManager() : pimpl_(std::make_unique<Impl>()) {}

MemoryManager::~MemoryManager() = default;

bool MemoryManager::initialize(const std::string& neo4j_uri,
                               const std::string& username,
                               const std::string& password) {
    return initialize(neo4j_uri, username, password, BootstrapConfig{});
}

bool MemoryManager::initialize(const std::string& neo4j_uri,
                               const std::string& username,
                               const std::string& password,
                               const BootstrapConfig& config) {
    return pimpl_->initialize(neo4j_uri, username, password, config);
}

bool MemoryManager::shutdown() { return pimpl_->shutdown(); }

bool MemoryManager::is_connected() const { return pimpl_->is_connected(); }

std::size_t MemoryManager::l1_size() const { return pimpl_->l1_size(); }

std::size_t MemoryManager::heatmap_size() const { return pimpl_->heatmap_size(); }

std::size_t MemoryManager::queries_executed() const { return pimpl_->queries_executed(); }

} // namespace pandora
