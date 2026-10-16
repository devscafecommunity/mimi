#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace pandora {

/// T(t) = T0 * exp(-lambda * age_seconds) — see planning/specs/HEATMAP-ALGORITHM.md
struct HeatmapNode {
    std::string id;
    double temperature;
    std::uint64_t last_accessed_secs;
    std::uint64_t access_count;
};

class HeatmapManager {
public:
    static constexpr double kDefaultLambda = 0.01;
    static constexpr double kDefaultThreshold = 0.1;
    static constexpr double kInitialTemperature = 1.0;
    static constexpr std::size_t kDefaultMaxNodes = 500;

    HeatmapManager(double lambda = kDefaultLambda,
                   double threshold = kDefaultThreshold,
                   std::size_t max_nodes = kDefaultMaxNodes);

    void touch(const std::string& id, std::uint64_t now_secs);
    double temperature_at(const std::string& id, std::uint64_t now_secs) const;
    std::vector<HeatmapNode> hot_nodes(std::uint64_t now_secs) const;
    void prune_cold(std::uint64_t now_secs);
    std::size_t size() const;

private:
    double lambda_;
    double threshold_;
    std::size_t max_nodes_;
    std::vector<HeatmapNode> nodes_;

    HeatmapNode* find_node(const std::string& id);
    static double decayed_temperature(double t0, double lambda, std::uint64_t age_secs);
};

} // namespace pandora
