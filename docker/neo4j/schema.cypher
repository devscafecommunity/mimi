// MiMi Pandora graph schema (M2.1.2 / M2.1.3)
// Apply with: ./scripts/neo4j-apply-schema.sh

// --- Uniqueness constraints ---
CREATE CONSTRAINT context_id IF NOT EXISTS
FOR (c:Context) REQUIRE c.id IS UNIQUE;

CREATE CONSTRAINT skill_id IF NOT EXISTS
FOR (s:Skill) REQUIRE s.id IS UNIQUE;

CREATE CONSTRAINT parameter_id IF NOT EXISTS
FOR (p:Parameter) REQUIRE p.id IS UNIQUE;

CREATE CONSTRAINT resource_id IF NOT EXISTS
FOR (r:Resource) REQUIRE r.id IS UNIQUE;

CREATE CONSTRAINT metadata_id IF NOT EXISTS
FOR (m:Metadata) REQUIRE m.id IS UNIQUE;

CREATE CONSTRAINT odlaguna_audit_id IF NOT EXISTS
FOR (a:OdlagunaAudit) REQUIRE a.id IS UNIQUE;

// Property EXISTENCE constraints require Neo4j Enterprise — enforce in app/seed on Community.

// --- Indexes for hot-path queries ---
CREATE INDEX context_heat IF NOT EXISTS
FOR (c:Context) ON (c.heat);

CREATE INDEX context_last_accessed IF NOT EXISTS
FOR (c:Context) ON (c.last_accessed);

CREATE INDEX skill_status IF NOT EXISTS
FOR (s:Skill) ON (s.status);

CREATE INDEX resource_type IF NOT EXISTS
FOR (r:Resource) ON (r.resource_type);
