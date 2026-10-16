#include <catch2/catch_test_macros.hpp>
#include <cstdlib>

#include "bolt_client.hpp"
#include "pandora_memory.hpp"

TEST_CASE("Bolt client integration contract", "[neo4j][integration]") {
    pandora::bolt::Config cfg;
    cfg.uri = "bolt://localhost:7687";
    cfg.username = "neo4j";
    cfg.password = "mimi-password";

    pandora::bolt::ConnectionPool pool(cfg);
    REQUIRE(pool.connect());
    REQUIRE(pool.is_healthy());

    auto ping = pool.run("RETURN 1 AS ok", {});
    REQUIRE(ping.ok());

    if (const char* live = std::getenv("NEO4J_INTEGRATION")) {
        REQUIRE(std::string(live) == "1");
        pandora::MemoryManager mgr;
        REQUIRE(mgr.initialize(cfg.uri, cfg.username, cfg.password));
        REQUIRE(mgr.is_connected());
        REQUIRE(mgr.queries_executed() >= 1);
        REQUIRE(mgr.shutdown());
    }
}
