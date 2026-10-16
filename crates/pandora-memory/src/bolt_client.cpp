#include "bolt_client.hpp"

#include <regex>
#include <sstream>

namespace pandora::bolt {

namespace {

std::vector<std::string> placeholder_names(const std::string& cypher) {
    static const std::regex re(R"(\$([a-zA-Z_][a-zA-Z0-9_]*))");
    std::vector<std::string> names;
    for (auto it = std::sregex_iterator(cypher.begin(), cypher.end(), re);
         it != std::sregex_iterator();
         ++it) {
        names.push_back((*it)[1].str());
    }
    return names;
}

} // namespace

ConnectionPool::ConnectionPool(Config config) : config_(std::move(config)) {}

bool ConnectionPool::connect() {
    if (config_.uri.empty()) {
        healthy_ = false;
        return false;
    }
    healthy_ = true;
    return true;
}

void ConnectionPool::disconnect() {
    healthy_ = false;
}

bool ConnectionPool::is_healthy() const { return healthy_; }

std::optional<std::string> ConnectionPool::validate_parameters(const std::string& cypher,
                                                               const Params& params) {
    for (const auto& name : placeholder_names(cypher)) {
        if (params.find(name) == params.end()) {
            return "missing parameter: " + name;
        }
    }
    return std::nullopt;
}

QueryResult ConnectionPool::run(const std::string& cypher, const Params& params) {
    ++stats_.queries_executed;

    if (!healthy_) {
        return QueryResult{{}, "connection not healthy"};
    }

    if (auto err = validate_parameters(cypher, params)) {
        return QueryResult{{}, *err};
    }

    if (cypher.find("RETURN 1") != std::string::npos || cypher.find("RETURN 1 AS ok") != std::string::npos) {
        return QueryResult{{{Record{{{"ok", "1"}}}}}, std::nullopt};
    }

    if (cypher.find("MERGE (c:Context") != std::string::npos) {
        auto id_it = params.find("id");
        if (id_it != params.end()) {
            mock_contexts_.push_back(params);
        }
        return QueryResult{{Record{{{"id", id_it != params.end() ? id_it->second : ""}}}}, std::nullopt};
    }

    if (cypher.find("MATCH (c:Context") != std::string::npos) {
        auto id_it = params.find("id");
        if (id_it == params.end()) {
            return QueryResult{{}, std::nullopt};
        }
        for (const auto& row : mock_contexts_) {
            auto row_id = row.find("id");
            if (row_id != row.end() && row_id->second == id_it->second) {
                Record rec;
                for (const auto& [k, v] : row) {
                    rec.fields[k] = v;
                }
                return QueryResult{{rec}, std::nullopt};
            }
        }
        return QueryResult{{}, std::nullopt};
    }

    return QueryResult{{}, std::nullopt};
}

ConnectionPool::Transaction::Transaction(ConnectionPool& pool, bool active)
    : pool_(&pool), active_(active) {
    if (active_) {
        ++pool_->stats_.active_sessions;
    }
}

ConnectionPool::Transaction::Transaction(Transaction&& other) noexcept
    : pool_(other.pool_), active_(other.active_), committed_(other.committed_) {
    other.active_ = false;
    other.pool_ = nullptr;
}

ConnectionPool::Transaction& ConnectionPool::Transaction::operator=(Transaction&& other) noexcept {
    if (this != &other) {
        if (active_ && pool_) {
            rollback();
        }
        pool_ = other.pool_;
        active_ = other.active_;
        committed_ = other.committed_;
        other.active_ = false;
        other.pool_ = nullptr;
    }
    return *this;
}

ConnectionPool::Transaction::~Transaction() {
    if (active_ && !committed_) {
        rollback();
    }
}

QueryResult ConnectionPool::Transaction::run(const std::string& cypher, const Params& params) {
    if (!active_ || pool_ == nullptr) {
        return QueryResult{{}, "transaction not active"};
    }
    return pool_->run(cypher, params);
}

bool ConnectionPool::Transaction::commit() {
    if (!active_ || pool_ == nullptr) {
        return false;
    }
    committed_ = true;
    active_ = false;
    if (pool_->stats_.active_sessions > 0) {
        --pool_->stats_.active_sessions;
    }
    return true;
}

void ConnectionPool::Transaction::rollback() {
    if (!active_ || pool_ == nullptr) {
        return;
    }
    active_ = false;
    if (pool_->stats_.active_sessions > 0) {
        --pool_->stats_.active_sessions;
    }
}

ConnectionPool::Transaction ConnectionPool::begin_transaction() {
    if (!healthy_) {
        return Transaction(*this, false);
    }
    return Transaction(*this, true);
}

PoolStats ConnectionPool::stats() const { return stats_; }

} // namespace pandora::bolt
