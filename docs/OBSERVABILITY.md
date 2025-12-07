# Observability in zOS

## Overview

zOS provides comprehensive observability through structured logging, metrics, and health checks.

## Structured Logging

### Initialization

Logging is initialized at application startup:

```rust
logging::init_logging();
```

This sets up:
- JSON-formatted log output
- Environment variable-based log level filtering (`RUST_LOG`)
- Automatic inclusion of file, line, thread ID, and timestamp

### Log Levels

Configure via `RUST_LOG` environment variable:

```bash
RUST_LOG=debug cargo run        # Debug level
RUST_LOG=info cargo run         # Info level (default)
RUST_LOG=warn cargo run         # Warnings and errors only
RUST_LOG=zos::pipelines=debug  # Debug for specific module
```

### Structured Fields

All logs include structured fields:

```rust
tracing::info!(
    model = model_name,
    latency_ms = latency,
    task = ?task_type,
    "Model call succeeded"
);
```

**Benefits:**
- Queryable in log aggregation systems (e.g., ELK, Datadog)
- Filterable by any field
- No string parsing required

### Common Log Patterns

**Model Calls:**
```rust
tracing::info!(
    model = model_name,
    latency_ms = latency,
    "Model call succeeded"
);

tracing::warn!(
    model = model_name,
    error = %error,
    "Model call failed, retrying"
);
```

**Routing:**
```rust
tracing::debug!(
    task = ?task_type,
    model = model_name,
    "Routing decision"
);

tracing::warn!(
    primary = primary_model,
    fallback = fallback_model,
    "Primary model unavailable, trying fallback"
);
```

**Cache:**
```rust
tracing::debug!(
    model = model_name,
    prompt_preview = &prompt[..50],
    "Cache hit"
);
```

## Metrics

### Metrics Structure

The `Metrics` struct provides atomic counters:

```rust
pub struct Metrics {
    pub model_latency_ms: Arc<AtomicU64>,
    pub routing_time_ms: Arc<AtomicU64>,
    pub cache_hit_count: Arc<AtomicU64>,
    pub cache_miss_count: Arc<AtomicU64>,
    pub fallback_count: Arc<AtomicU64>,
    pub errors_total: Arc<AtomicU64>,
    pub session_state_transitions: Arc<AtomicU64>,
}
```

### Recording Metrics

```rust
metrics.record_model_latency(latency_ms);
metrics.record_cache_hit();
metrics.record_fallback();
metrics.record_error();
```

### Prometheus Format

Metrics can be exported in Prometheus format:

```rust
let prometheus_text = metrics.to_prometheus();
```

Example output:
```
# HELP model_latency_ms Total model call latency in milliseconds
# TYPE model_latency_ms counter
model_latency_ms 12345

# HELP cache_hit_count Total cache hits
# TYPE cache_hit_count counter
cache_hit_count 42
```

### Metrics Endpoint (TODO)

A `/metrics` endpoint will expose metrics for Prometheus scraping:

```rust
#[tauri::command]
pub async fn get_metrics(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    Ok(state.metrics.to_prometheus())
}
```

## Health Checks

### Health Check Endpoint (TODO)

A `/healthz` endpoint will provide health status:

```rust
#[tauri::command]
pub async fn health_check(
    state: State<'_, Arc<AppState>>,
) -> Result<HealthStatus, String> {
    // Check:
    // - Model availability
    // - Cache health
    // - Disk space
    // - Memory usage
    Ok(HealthStatus::Healthy)
}
```

## Performance Monitoring

### PerfTimer

The `perf` module provides performance timing:

```rust
let _perf = perf::PerfTimer::new("operation_name");
// ... operation ...
// Automatically logs duration on drop
```

### Latency Tracking

All model calls track latency:

```rust
let start = Instant::now();
let result = model.call_json::<T>(prompt).await?;
let latency_ms = start.elapsed().as_millis() as u64;
tracing::info!(latency_ms = latency_ms, "Model call completed");
```

## Log Aggregation

### JSON Output

Logs are emitted as JSON for easy parsing:

```json
{
  "timestamp": "2024-01-15T10:30:45.123Z",
  "level": "INFO",
  "target": "zos::pipelines::router",
  "fields": {
    "model": "deepseek-r1:7b",
    "latency_ms": 1234,
    "message": "Model call succeeded"
  },
  "file": "src/pipelines/router.rs",
  "line": 123
}
```

### Recommended Tools

- **Local Development**: `jq` for filtering JSON logs
- **Production**: ELK stack, Datadog, or similar
- **Debugging**: `RUST_LOG=debug` for verbose output

## Alerting (Future)

Future improvements will include:
- Error rate thresholds
- Latency percentile alerts
- Model availability alerts
- Cache hit rate monitoring

## Best Practices

1. **Use appropriate log levels** - Don't spam with debug logs
2. **Include structured fields** - Make logs queryable
3. **Record metrics for all operations** - Enable monitoring
4. **Log errors with context** - Include relevant state
5. **Avoid sensitive data** - Don't log prompts or responses in production
