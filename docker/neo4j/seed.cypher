// MiMi Pandora seed data (M2.1.4)
// Safe to re-run: uses MERGE on stable ids.

MERGE (sys:Metadata {id: "meta:system"})
SET sys.kind = "system",
    sys.version = "0.1.0",
    sys.updated_at = datetime();

MERGE (ctx:Context {id: "ctx:system-bootstrap"})
SET ctx.label = "System Bootstrap",
    ctx.content_hash = "bootstrap-v1",
    ctx.heat = 1.0,
    ctx.last_accessed = datetime(),
    ctx.access_count = 1,
    ctx.created_at = datetime();

MERGE (skill:Skill {id: "skill:echo"})
SET skill.name = "echo",
    skill.version = "1.0.0",
    skill.status = "active",
    skill.runtime = "builtin",
    skill.created_at = datetime();

MERGE (param:Parameter {id: "param:echo-message"})
SET param.name = "message",
    param.type = "string",
    param.required = true;

MERGE (res:Resource {id: "res:docs-quickstart"})
SET res.resource_type = "documentation",
    res.uri = "docs/QUICKSTART.md",
    res.description = "Local quick-start guide";

// Relationships (MATCH by id — cypher-shell does not keep variables across statements)
MATCH (ctx:Context {id: "ctx:system-bootstrap"}), (skill:Skill {id: "skill:echo"})
MERGE (ctx)-[:HAS]->(skill);

MATCH (skill:Skill {id: "skill:echo"}), (param:Parameter {id: "param:echo-message"})
MERGE (skill)-[:EXECUTES]->(param);

MATCH (skill:Skill {id: "skill:echo"}), (res:Resource {id: "res:docs-quickstart"})
MERGE (skill)-[:DEPENDS_ON]->(res);

MATCH (ctx:Context {id: "ctx:system-bootstrap"}), (res:Resource {id: "res:docs-quickstart"})
MERGE (ctx)-[:REFERENCES]->(res);

MATCH (sys:Metadata {id: "meta:system"}), (ctx:Context {id: "ctx:system-bootstrap"})
MERGE (sys)-[:HAS]->(ctx);
