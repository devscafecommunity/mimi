#pragma once

#include <cstddef>
#include <memory>
#include <string>

namespace pandora {

/// Pandora memory engine bootstrap (M2.4.1).
class MemoryManager {
public:
    struct BootstrapConfig {
        std::size_t l1_capacity{1000};
        double heatmap_lambda{0.01};
        double heatmap_threshold{0.1};
    };

    MemoryManager();
    ~MemoryManager();

    bool initialize(const std::string& neo4j_uri,
                    const std::string& username,
                    const std::string& password);

    bool initialize(const std::string& neo4j_uri,
                    const std::string& username,
                    const std::string& password,
                    const BootstrapConfig& config);
    bool shutdown();

    [[nodiscard]] bool is_connected() const;
    [[nodiscard]] std::size_t l1_size() const;
    [[nodiscard]] std::size_t heatmap_size() const;
    [[nodiscard]] std::size_t queries_executed() const;

private:
    class Impl;
    std::unique_ptr<Impl> pimpl_;
};

} // namespace pandora
