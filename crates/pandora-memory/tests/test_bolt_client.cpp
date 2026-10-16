#include <catch2/catch_test_macros.hpp>

#include "bolt_client.hpp"

using namespace pandora::bolt;

TEST_CASE("Bolt pool connect and ping", "[bolt]") {
    Config cfg;
    cfg.uri = "bolt://localhost:7687";
    cfg.username = "neo4j";
    cfg.password = "test";

    ConnectionPool pool(cfg);
    REQUIRE(!pool.is_healthy());
    REQUIRE(pool.connect());
    REQUIRE(pool.is_healthy());

    auto ping = pool.run("RETURN 1 AS ok", {});
    REQUIRE(ping.ok());
    REQUIRE(ping.records.size() == 1);
    REQUIRE(ping.records[0].fields.at("ok") == "1");

    pool.disconnect();
    REQUIRE(!pool.is_healthy());
}

TEST_CASE("Parameterized query validation", "[bolt]") {
    auto err = ConnectionPool::validate_parameters(
        "MATCH (c:Context {id: $id}) RETURN c",
        Params{{"other", "x"}});
    REQUIRE(err.has_value());

    auto ok = ConnectionPool::validate_parameters(
        "MATCH (c:Context {id: $id}) RETURN c",
        Params{{"id", "ctx:1"}});
    REQUIRE(!ok.has_value());
}

TEST_CASE("Bolt transaction commit", "[bolt]") {
    Config cfg{.uri = "bolt://localhost:7687"};
    ConnectionPool pool(cfg);
    REQUIRE(pool.connect());

    auto tx = pool.begin_transaction();
    REQUIRE(tx.active());
    auto result = tx.run(
        "MERGE (c:Context {id: $id}) SET c.label = $label",
        Params{{"id", "ctx:a"}, {"label", "A"}});
    REQUIRE(result.ok());
    REQUIRE(tx.commit());

    auto stats = pool.stats();
    REQUIRE(stats.queries_executed >= 1);
}
