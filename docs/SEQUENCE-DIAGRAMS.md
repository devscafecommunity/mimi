# Sequence Diagrams

## CLI query → adapter

```mermaid
sequenceDiagram
    participant U as User
    participant B as Beatrice CLI
    participant R as AdapterRegistry
    participant A as AiAdapter
    participant C as Mimi Core

    U->>B: mimi query "..."
    B->>B: Parse args / validate
    B->>C: Route task (bus / in-process)
    C->>C: StateManager Processing
    C->>R: Select adapter
    R->>A: invoke(AiRequest)
    A-->>R: AiResponse
    R-->>C: Result
    C->>C: Responding → Idle
    C-->>B: Response payload
    B-->>U: stdout (text/json)
```

## WebSocket streaming

```mermaid
sequenceDiagram
    participant C as Client
    participant W as WS Handler
    participant V as Validation
    participant S as Streaming

    C->>W: WsMessage (subscribe, channel=query)
    W->>V: validate message
    V-->>W: OK
    W->>S: start stream
    S-->>C: WsEvent start
    S-->>C: WsEvent progress
    S-->>C: WsEvent result
    S-->>C: WsEvent complete
```

## Authentication (JWT)

```mermaid
sequenceDiagram
    participant C as Client
    participant M as AuthManager
    participant H as HTTP/WS middleware

    C->>M: credentials / identity
    M->>M: generate_token(Identity)
    M-->>C: JWT
    C->>H: Authorization: Bearer JWT
    H->>M: validate_token
    M-->>H: Identity + roles
    H-->>C: authorized request continues
```

## Error path (adapter unavailable)

```mermaid
sequenceDiagram
    participant C as Core
    participant R as Registry
    participant P as Primary Adapter
    participant F as Fallback Adapter

    C->>R: get best adapter
    R->>P: health_check
    P-->>R: fail
    R->>F: fallback by priority
    F-->>R: healthy
    R-->>C: invoke on fallback
```

Message types are defined in `crates/mimi-cli/src/ws/types.rs` and auth in `crates/mimi-cli/src/auth/`.
