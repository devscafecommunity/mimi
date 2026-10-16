#include "heatmap.hpp"

#include <algorithm>
#include <cmath>

namespace pandora {

HeatmapManager::HeatmapManager(double lambda, double threshold, std::size_t max_nodes)
    : lambda_(lambda), threshold_(threshold), max_nodes_(max_nodes) {}

double HeatmapManager::decayed_temperature(double t0, double lambda, std::uint64_t age_secs) {
    return t0 * std::exp(-lambda * static_cast<double>(age_secs));
}

HeatmapNode* HeatmapManager::find_node(const std::string& id) {
    for (auto& node : nodes_) {
        if (node.id == id) {
            return &node;
        }
    }
    return nullptr;
}

void HeatmapManager::touch(const std::string& id, std::uint64_t now_secs) {
    if (auto* existing = find_node(id)) {
        existing->last_accessed_secs = now_secs;
        existing->temperature = kInitialTemperature;
        existing->access_count += 1;
        return;
    }

    if (nodes_.size() >= max_nodes_) {
        prune_cold(now_secs);
    }

    if (nodes_.size() >= max_nodes_ && !nodes_.empty()) {
        nodes_.erase(nodes_.begin());
    }

    nodes_.push_back(HeatmapNode{
        id,
        kInitialTemperature,
        now_secs,
        1,
    });
}

double HeatmapManager::temperature_at(const std::string& id, std::uint64_t now_secs) const {
    for (const auto& node : nodes_) {
        if (node.id != id) {
            continue;
        }
        const std::uint64_t age = now_secs > node.last_accessed_secs
                                        ? now_secs - node.last_accessed_secs
                                        : 0;
        return decayed_temperature(node.temperature, lambda_, age);
    }
    return 0.0;
}

std::vector<HeatmapNode> HeatmapManager::hot_nodes(std::uint64_t now_secs) const {
    std::vector<HeatmapNode> result;
    for (const auto& node : nodes_) {
        const std::uint64_t age = now_secs > node.last_accessed_secs
                                        ? now_secs - node.last_accessed_secs
                                        : 0;
        const double heat = decayed_temperature(node.temperature, lambda_, age);
        if (heat >= threshold_) {
            HeatmapNode copy = node;
            copy.temperature = heat;
            result.push_back(copy);
        }
    }
    std::sort(result.begin(), result.end(), [](const HeatmapNode& a, const HeatmapNode& b) {
        return a.temperature > b.temperature;
    });
    return result;
}

void HeatmapManager::prune_cold(std::uint64_t now_secs) {
    nodes_.erase(
        std::remove_if(nodes_.begin(), nodes_.end(), [&](const HeatmapNode& node) {
            const std::uint64_t age = now_secs > node.last_accessed_secs
                                            ? now_secs - node.last_accessed_secs
                                            : 0;
            return decayed_temperature(node.temperature, lambda_, age) < threshold_;
        }),
        nodes_.end());
}

std::size_t HeatmapManager::size() const {
    return nodes_.size();
}

} // namespace pandora
