#include <catch2/catch_test_macros.hpp>

#include "heatmap.hpp"

TEST_CASE("Heatmap decay matches exponential model", "[heatmap]") {
    pandora::HeatmapManager heatmap(0.01, 0.1, 500);

    heatmap.touch("ctx-a", 0);
    REQUIRE(heatmap.temperature_at("ctx-a", 0) == 1.0);

    const double at_70 = heatmap.temperature_at("ctx-a", 70);
    REQUIRE(at_70 > 0.45);
    REQUIRE(at_70 < 0.55);
}

TEST_CASE("Touch resets node to hot", "[heatmap]") {
    pandora::HeatmapManager heatmap;
    heatmap.touch("ctx-b", 0);
    REQUIRE(heatmap.temperature_at("ctx-b", 250) < 0.1);

    heatmap.touch("ctx-b", 250);
    REQUIRE(heatmap.temperature_at("ctx-b", 250) == 1.0);
}

TEST_CASE("hot_nodes filters below threshold", "[heatmap]") {
    pandora::HeatmapManager heatmap;
    heatmap.touch("cold", 0);
    heatmap.touch("hot", 100);

    const auto nodes = heatmap.hot_nodes(250);
    REQUIRE(nodes.size() == 1);
    REQUIRE(nodes.front().id == "hot");
}
