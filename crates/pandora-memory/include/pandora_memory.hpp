#pragma once

#include <string>
#include <memory>
#include <vector>

namespace pandora {

class MemoryManager {
public:
    MemoryManager();
    ~MemoryManager();
    
    bool initialize(const std::string& neo4j_uri, const std::string& username, const std::string& password);
    bool shutdown();
    
    bool is_connected() const;
    
private:
    class Impl;
    std::unique_ptr<Impl> pimpl_;
};

}
