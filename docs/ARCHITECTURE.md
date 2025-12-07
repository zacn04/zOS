# zOS Architecture

## Overview

zOS is a defensive-first codebase designed with E4+ engineering standards. All architectural decisions prioritize safety, observability, and maintainability.

## Core Principles

1. **Zero Panics**: No `unwrap()` or `expect()` calls without explicit justification
2. **Async-First I/O**: All filesystem and network operations are async
3. **Explicit State Management**: No global mutable state; all state is passed explicitly via `AppState`
4. **Unified Error Handling**: Single error type (`ZosError`) with rich context
5. **Structured Logging**: All logging uses `tracing` with JSON output
6. **Observability**: Metrics, health checks, and detailed logging

## State Management

### AppState

All mutable application state is centralized in `AppState`:

```rust
pub struct AppState {
    pub skills: Arc<RwLock<Option<SkillVector>>>,
    pub session_state: Arc<RwLock<ProofState>>,
    pub routing_metrics: Arc<RwLock<RoutingMetrics>>,
    pub response_cache: Arc<RwLock<LruCache<u64, CachedResponse>>>,
}
```

**Key Design Decisions:**
- Uses `parking_lot::RwLock` for better performance than `std::sync::RwLock`
- All state is wrapped in `Arc` for shared ownership
- State is passed explicitly to all functions (no globals)
- Tauri manages AppState via `tauri::State` in command handlers

### Migration from Global State

Previously, the codebase used:
- `lazy_static!` with `Mutex` for global state
- `static` variables with `Mutex`

All of these have been replaced with `AppState` passed explicitly.

## Error Handling

### ZosError

All functions return `Result<T, ZosError>` instead of `String` errors:

```rust
pub struct ZosError {
    pub message: String,
    pub stage: String,        // e.g., "routing", "io", "json_parse"
    pub model: Option<String>,
    pub retry_succeeded: bool,
    pub context: Option<String>,
    pub source: Option<String>,
}
```

**Error Propagation:**
- Use `.with_context()` to add context at each layer
- Use `.with_model()` to associate errors with model calls
- Never lose error information through string conversions

**Example:**
```rust
tokio::fs::read_to_string(&path)
    .await
    .map_err(|e| ZosError::new(
        format!("Failed to read file: {}", e),
        "io"
    ).with_context(format!("path: {:?}", path)))
```

## Async I/O

All filesystem operations use `tokio::fs::*` instead of `std::fs::*`:

- `tokio::fs::read_to_string()` instead of `fs::read_to_string()`
- `tokio::fs::write()` instead of `fs::write()`
- `tokio::fs::create_dir_all()` instead of `fs::create_dir_all()`

**Rationale:**
- Non-blocking I/O prevents thread pool exhaustion
- Better integration with async/await patterns
- Allows concurrent I/O operations

## Concurrency Model

### Lock Ordering Prevention

All locks are acquired in a consistent order to prevent deadlocks:
1. `skills` (if needed)
2. `session_state` (if needed)
3. `routing_metrics` (if needed)
4. `response_cache` (if needed)

### Short Critical Sections

Locks are held for minimal time:
- Data is cloned out of locks before processing
- I/O operations are performed outside of lock guards
- Complex computations happen after releasing locks

### Example:
```rust
// BAD: I/O inside lock
let mut cache = RESPONSE_CACHE.lock().unwrap();
let data = fs::read_to_string("file.json")?; // BLOCKING!

// GOOD: I/O outside lock
let data = tokio::fs::read_to_string("file.json").await?;
let mut cache = state.response_cache.write();
cache.put(key, cached);
```

## Logging

### Structured Logging with Tracing

All logging uses the `tracing` crate with structured fields:

```rust
tracing::info!(
    model = model_name,
    latency_ms = latency,
    "Model call succeeded"
);
```

**Benefits:**
- JSON output for log aggregation
- Automatic inclusion of context (file, line, thread)
- Filterable by log level and target
- Zero-cost when disabled

### Log Levels

- `tracing::error!` - Errors that require attention
- `tracing::warn!` - Warnings about degraded functionality
- `tracing::info!` - Important operational events
- `tracing::debug!` - Detailed debugging information
- `tracing::trace!` - Very verbose tracing

## Metrics

### Prometheus-Style Metrics

The `Metrics` struct provides atomic counters for observability:

- `model_latency_ms` - Total model call latency
- `routing_time_ms` - Routing decision time
- `cache_hit_count` - Cache hits
- `cache_miss_count` - Cache misses
- `fallback_count` - Fallback attempts
- `errors_total` - Total errors
- `session_state_transitions` - State transitions

Metrics are exposed via `/metrics` endpoint (to be implemented).

## Caching

### LRU Cache with Bounded Size

Response cache uses `LruCache` with a fixed size (200 entries):

- Prevents unbounded memory growth
- Automatic eviction of least-recently-used entries
- Thread-safe via `RwLock`
- Stored in `AppState` for explicit access

## Model Routing

### Task-Based Routing

Models are selected based on `TaskType`:
- `ProofAnalysis` → proof model
- `ProblemGeneration` → problem model
- `General` → general model

### Fallback Strategy

1. Try primary model
2. If unavailable, try fallback model
3. If fallback fails, return error with context

### Retry Logic

- One automatic retry with 100ms backoff
- Exponential backoff for future improvements
- Retry count tracked in metrics

## Testing Strategy

See [TESTING_STRATEGY.md](./TESTING_STRATEGY.md) for details.

## Future Improvements

1. Circuit breakers for model availability
2. Request cancellation support
3. Streaming responses with backpressure
4. Distributed tracing integration
5. Health check endpoint with dependency checks
