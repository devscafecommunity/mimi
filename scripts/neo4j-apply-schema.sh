#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NEO4J_CONTAINER="${NEO4J_CONTAINER:-mimi-neo4j}"
NEO4J_USER="${NEO4J_USER:-neo4j}"
NEO4J_PASSWORD="${NEO4J_PASSWORD:-mimi-password}"

if ! docker ps --format '{{.Names}}' | grep -qx "${NEO4J_CONTAINER}"; then
  echo "Neo4j container '${NEO4J_CONTAINER}' is not running."
  echo "Start the stack first: docker compose up -d neo4j"
  exit 1
fi

echo "Waiting for Neo4j Bolt (up to 120s)..."
ready=0
for _ in $(seq 1 60); do
  if docker exec "${NEO4J_CONTAINER}" cypher-shell \
    -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" \
    "RETURN 1;" >/dev/null 2>&1; then
    ready=1
    break
  fi
  sleep 2
done

if [[ "${ready}" -ne 1 ]]; then
  echo "Neo4j is not accepting connections yet. Check: docker logs ${NEO4J_CONTAINER}"
  exit 1
fi

apply_cypher() {
  local file="$1"
  echo "Applying ${file}..."
  if ! docker exec -i "${NEO4J_CONTAINER}" cypher-shell \
    -u "${NEO4J_USER}" -p "${NEO4J_PASSWORD}" \
    < "${ROOT_DIR}/docker/neo4j/${file}"; then
    echo "Warning: ${file} reported errors (see above). Continuing."
  fi
}

apply_cypher schema.cypher
apply_cypher seed.cypher

echo "Neo4j schema and seed applied successfully."
