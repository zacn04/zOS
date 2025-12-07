# Error Handling in zOS

## Philosophy

Every error is a `ZosError` with rich context. We never lose error information through string conversions or panics.

## ZosError Structure

```rust
pub struct ZosError {
    pub message: String,           // Human-readable error message
    pub stage: String,             // Stage where error occurred (e.g., "routing", "io")
    pub model: Option<String>,     // Associated model (if applicable)
    pub retry_succeeded: bool,     // Whether a retry succeeded
    pub context: Option<String>,   // Additional context
    pub source: Option<String>,    // Source of the error
}
```

## Error Stages

Common stages used throughout the codebase:

- `"io"` - Filesystem I/O errors
- `"json_parse"` - JSON parsing errors
- `"json_serialize"` - JSON serialization errors
- `"routing"` - Model routing errors
- `"model_call"` - Model API call errors
- `"cache"` - Cache operation errors
- `"timeout"` - Timeout errors
- `"state"` - State management errors
- `"startup"` - Application startup errors

## Error Propagation Patterns

### Pattern 1: Direct Conversion

```rust
use crate::error::ZosError;

fn my_function() -> Result<String, ZosError> {
    tokio::fs::read_to_string("file.txt")
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to read file: {}", e),
            "io"
        ))
}
```

### Pattern 2: With Context

```rust
fn my_function(path: &Path) -> Result<String, ZosError> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ZosError::new(
            format!("Failed to read file: {}", e),
            "io"
        ).with_context(format!("path: {:?}", path)))
}
```

### Pattern 3: With Model

```rust
fn call_model(model: &str, prompt: &str) -> Result<String, ZosError> {
    // ... model call ...
    .map_err(|e| ZosError::new(
        format!("Model call failed: {}", e),
        "model_call"
    ).with_model(model.to_string()))
}
```

### Pattern 4: Chaining Context

```rust
fn process_file(path: &Path) -> Result<Data, ZosError> {
    let content = read_file(path)
        .await
        .map_err(|e| e.with_context("Failed to read file"))?;
    
    parse_content(&content)
        .map_err(|e| e.with_context("Failed to parse content"))
}
```

## Removing unwrap() and expect()

### Before (BAD):
```rust
let data = fs::read_to_string("file.json").unwrap();
let parsed: Data = serde_json::from_str(&data).unwrap();
```

### After (GOOD):
```rust
let data = tokio::fs::read_to_string("file.json")
    .await
    .map_err(|e| ZosError::new(
        format!("Failed to read file: {}", e),
        "io"
    ).with_context("file.json"))?;

let parsed: Data = serde_json::from_str(&data)
    .map_err(|e| ZosError::new(
        format!("Failed to parse JSON: {}", e),
        "json_parse"
    ).with_context("file.json"))?;
```

## Lock Handling

### Before (BAD):
```rust
let mut cache = RESPONSE_CACHE.lock().unwrap();
cache.put(key, value);
```

### After (GOOD):
```rust
let mut cache = state.response_cache.write();
cache.put(key, value);
// Lock is automatically released here
```

If lock acquisition can fail (rare with `parking_lot`), handle it:

```rust
let cache = state.response_cache.try_write()
    .ok_or_else(|| ZosError::new(
        "Failed to acquire cache lock",
        "state"
    ))?;
```

## Justifying Remaining unwrap() Calls

If an `unwrap()` cannot be removed, document why it's provably safe:

```rust
// SAFE: NonZeroUsize::new(200) is guaranteed to be Some(200) since 200 > 0
let cache = LruCache::new(
    NonZeroUsize::new(200).expect("200 > 0")
);
```

## Error Display

`ZosError` implements `Display` and `Error` traits:

```rust
impl fmt::Display for ZosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.stage, self.message)?;
        if let Some(ref model) = self.model {
            write!(f, " (model: {})", model)?;
        }
        if let Some(ref context) = self.context {
            write!(f, " (context: {})", context)?;
        }
        Ok(())
    }
}
```

## Error Conversion

Common error types automatically convert to `ZosError`:

```rust
impl From<std::io::Error> for ZosError { ... }
impl From<serde_json::Error> for ZosError { ... }
impl From<tokio::time::error::Elapsed> for ZosError { ... }
```

Use `?` operator for automatic conversion:

```rust
let data = tokio::fs::read_to_string("file.json").await?;
// Automatically converts io::Error to ZosError
```

## Tauri Command Error Handling

Tauri commands return `Result<T, String>`. Convert `ZosError` to `String` at the boundary:

```rust
#[tauri::command]
pub async fn my_command(
    state: State<'_, Arc<AppState>>,
    input: String,
) -> Result<Response, String> {
    my_function(state.inner(), &input)
        .await
        .map_err(|e| e.to_string())
}
```

## Best Practices

1. **Always add context** - Use `.with_context()` to add relevant information
2. **Preserve error chain** - Don't discard original errors
3. **Use appropriate stages** - Choose descriptive stage names
4. **Document safe unwraps** - If unwrap is necessary, explain why
5. **Handle all error paths** - No silent failures

## Examples

See the codebase for real-world examples:
- `src/skills/store.rs` - Async I/O error handling
- `src/pipelines/router.rs` - Model call error handling
- `src/cache.rs` - Cache operation error handling
