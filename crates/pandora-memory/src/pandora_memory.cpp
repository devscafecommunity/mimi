#include "pandora_memory.hpp"

namespace pandora {

class MemoryManager::Impl {
public:
    Impl() : connected_(false) {}
    
    bool initialize(const std::string& uri, const std::string& user, const std::string& pass);
    bool shutdown();
    bool is_connected() const { return connected_; }
    
private:
    bool connected_;
};

}

namespace pandora {

MemoryManager::MemoryManager() : pimpl_(std::make_unique<Impl>()) {}

MemoryManager::~MemoryManager() = default;

bool MemoryManager::initialize(const std::string& neo4j_uri, const std::string& username, const std::string& password) {
    return pimpl_->initialize(neo4j_uri, username, password);
}

bool MemoryManager::shutdown() {
    return pimpl_->shutdown();
}

bool MemoryManager::is_connected() const {
    return pimpl_->is_connected();
}

}

namespace pandora {

bool MemoryManager::Impl::initialize(const std::string& uri, const std::string& user, const std::string& pass) {
    connected_ = true;
    return true;
}

bool MemoryManager::Impl::shutdown() {
    connected_ = false;
    return true;
}

}
