# PluralSync WebSocket Push Source — Client Specification

This document describes the WebSocket protocol for pushing fronting status updates to PluralSync. It is intended for developers building external clients.

## 1. Connection

**Endpoint:** `wss://<api-host>/api/user/platform/pluralsync/events`

Connect using any standard WebSocket library. No authentication headers or query parameters are needed — authentication happens via a message handshake after connecting.

All messages are JSON-encoded UTF-8 strings.

## 2. Deployment Feature Flag

The websocket push source may be disabled at the **deployment level**. When disabled, the server returns a 400 Bad Request immediately upon connection upgrade, before any application-level protocol messages are exchanged.

### 2.1 Feature Disabled Response

If the deployment has `enable_websocket_push_source` set to `false`, the HTTP upgrade response is **400 Bad Request** with body:

```json
{"type":"error","result":"feature_disabled","data":"WebSocket push source is not available in this deployment"}
```

Clients should check the `websocket_push_source_available` field in the server's startup metadata (from `/api/user/info`) before attempting to connect. If `false`, the websocket endpoint should not be used.

### 2.2 Login Request

Sent immediately after connecting:

```json
{
  "type": "login",
  "user": "<email>",
  "auth": "<jwt-token>"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | `string` | Must be `"login"` |
| `user` | `string` | The account email address (informational only) |
| `auth` | `string` | A valid JWT issued by PluralSync (same as REST API tokens) |

The JWT must be valid (not expired, correct signature). The `sub` claim inside the JWT identifies the PluralSync user.

### 2.3 Login Response

**On success:**

```json
{
  "type": "login",
  "result": "success",
  "server_info": {
    "version": "<version-string>"
  }
}
```

Additional fields may be present. The connection is now authenticated. The client may send `fronters` messages.

**On failure:**

```json
{
  "type": "error",
  "result": "<reason>",
  "data": "<description>"
}
```

The server then **closes the connection**.

### 2.4 One Connection, One User

Each WebSocket connection may authenticate exactly **one** user. A second `login` message on an authenticated connection is rejected.

## 3. Keepalive

Keepalive messages may be sent in **either direction**. The client may send `ping` and expect `pong`. The server may also send `ping` and expect a `pong` response.

### 3.1 Ping

```json
{
  "type": "ping"
}
```

### 3.2 Pong

```json
{
  "type": "pong"
}
```

## 4. Fronters Update

After authentication, the client may push fronting status updates.

### 4.1 Fronters Request

```json
{
  "type": "fronters",
  "data": {
    "fronters": [
      {
        "id": "abc123",
        "name": "Alice",
        "pronouns": "she/her",
        "avatar_url": "https://example.com/avatar.png",
        "start_time": "2026-05-03T12:00:00Z",
        "privacy": "public"
      }
    ]
  }
}
```

### 4.2 GenericFronter Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | `string` | Yes | A stable, unique identifier for this fronter. Must not change between updates for the same person. |
| `name` | `string` | Yes | The fronter's display name. |
| `pronouns` | `string` | No | Pronouns (e.g. `"she/her"`, `"they/them"`). Omit if not available. |
| `avatar_url` | `string` | No | Avatar URL. Omit if no avatar. |
| `start_time` | `string` | No | ISO 8601 UTC timestamp (e.g. `"2026-05-03T12:00:00Z"`). If omitted, the server uses the current time. |
| `privacy` | `string` | Yes | Must be `"public"` or `"private"`. See §5. |

### 4.3 Fronters Array Semantics

- The `fronters` array is the **complete current fronting state**. It **replaces** the previous state — no merging.
- An **empty array** (`[]`) means nobody is fronting.
- The server distributes fronters to all configured sync targets (VRChat, Discord, PluralKit, website, etc.) after applying privacy filtering (§5).

### 4.4 Fronters Response

**On success**, the server sends **no response**. The absence of an error is the acknowledgement.

**On validation failure** (missing required fields, invalid `privacy` value, etc.), the server sends an error message but **keeps the connection open**:

```json
{
  "type": "fronters.response",
  "result": "error",
  "data": "<error description>"
}
```

Clients should log the `data` field from error responses, as it usually points to bugs in the client software.

## 5. Privacy

Each fronter has a `privacy` field:

| Value | Behavior |
|-------|----------|
| `"public"` | The fronter is included in sync output to all targets |
| `"private"` | The fronter is **filtered out** and never sent to any sync target |

The server applies this filter automatically. Fronters marked `"private"` are silently dropped.

## 6. Examples

### 6.1 Feature Disabled (HTTP 400)

```
Client → Server: (connects to wss://...)
Server → Client: 400 Bad Request
{"type":"error","result":"feature_disabled","data":"WebSocket push source is not available in this deployment"}
```

### 6.2 Authentication

```
Client → Server: {"type":"login","user":"user@example.com","auth":"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."}
Server → Client: {"type":"login","result":"success","server_info":{"version":"2.10.0"}}
```

### 6.3 Sending Fronters

```
Client → Server: {"type":"fronters","data":{"fronters":[{"id":"m1","name":"Alice","pronouns":"she/her","start_time":"2026-05-03T12:00:00Z","privacy":"public"},{"id":"m2","name":"Bob","pronouns":"they/them","privacy":"private"}]}}
Server → (no response — accepted)
```

Bob is filtered out because `privacy: "private"`. Only Alice is distributed.

### 6.4 Keepalive

```
Client → Server: {"type":"ping"}
Server → Client: {"type":"pong"}
```

### 6.5 Authentication Failure

```
Client → Server: {"type":"login","user":"user@example.com","auth":"invalid.token.here"}
Server → Client: {"type":"error","result":"invalid_jwt","data":"Token has expired"}
Server → (closes connection)
```

### 6.6 Invalid Fronters (connection stays open)

```
Client → Server: {"type":"fronters","data":{"fronters":[{"id":"m1","privacy":"public"}]}}
Server → Client: {"type":"fronters.response","result":"error","data":"field 'name' is required"}
Client → Server: {"type":"fronters","data":{"fronters":[{"id":"m1","name":"Alice","privacy":"public"}]}}
Server → (accepted — no response)
```
