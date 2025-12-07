# Concurrency Model in zOS

## Overview

zOS uses a defensive concurrency model that prevents deadlocks, race conditions, and lock-ordering hazards.

## Core Principles

1. **No Global Mutable State** - All state is in `AppState` and passed explicitly
2. **Consistent Lock Ordering** - Locks are always acquired in the same order
3. **Short Critical Sections** - Locks are held for minimal time
4. **I/O Outside Locks** - All I/O operations happen outside lock guards
5. **Lock-Free Where Possible** - Use atomic operations for simple counters

## Lock Ordering

To prevent deadlocks, locks are always acquired in this order:

1. `skills` (if needed)
2. `session_state` (if needed)
3. `routing_metrics` (if needed)
4. `response_cache` (if needed)

**Example:**
```rust
// GOOD: Consistent ordering
let skills = state.skills.read();
let session = state.session_state.read();
let cache = state.response_cache.write();

// BAD: Inconsistent ordering (can deadlock)
let cache = state.response_cache.write();
let skills = state.skills.read(); // Potential deadlock!
```

## Short Critical Sections

Locks should be held for the shortest time possible:

### Pattern 1: Clone and Release

```rust
// GOOD: Clone data, release lock, then process
let skills = {
    let guard = state.skills.read();
    guard.as_ref().unwrap().clone()
};
// Lock is released here
process_skills(&skills); // Long operation outside lock

// BAD: Hold lock during long operation
let guard = state.skills.read();
process_skills(guard.as_ref().unwrap()); // Lock held too long!
```

### Pattern 2: I/O Outside Locks

```rust
// GOOD: I/O outside lock
let data = tokio::fs::read_to_string("file.json").await?;
let mut cache = state.response_cache.write();
cache.put(key, cached);

// BAD: I/O inside lock
let mut cache = state.response_cache.write();
let data = fs::read_to_string("file.json")?; // BLOCKING!
cache.put(key, cached);
```

## Lock Types

### parking_lot::RwLock

We use `parking_lot::RwLock` instead of `std::sync::RwLock`:

**Benefits:**
- Better performance
- No poison on panic (safer)
- More efficient lock acquisition

**Usage:**
```rust
// Read lock (multiple readers allowed)
let guard = state.skills.read();
let skills = guard.as_ref().unwrap();

// Write lock (exclusive)
let mut guard = state.skills.write();
*guard = Some(new_skills);
```

### Atomic Operations

For simple counters, use atomics:

```rust
use std::sync::atomic::{AtomicU64, Ordering};

let counter = Arc::new(AtomicU64::new(0));
counter.fetch_add(1, Ordering::Relaxed);
```

## Avoiding Race Conditions

### TOCTOU (Time-of-Check-Time-of-Use)

**Problem:**
```rust
// BAD: Race condition
if !cache.contains(&key) {
    // Another thread might add it here!
    let value = compute_expensive_value();
    cache.put(key, value);
}
```

**Solution:**
```rust
// GOOD: Atomic check-and-set
let mut cache = state.response_cache.write();
if let Some(cached) = cache.get(&key) {
    return Ok(cached.clone());
}
let value = compute_expensive_value(); // Outside lock
cache.put(key, value.clone());
Ok(value)
```

### Cache Updates

**Problem:**
```rust
// BAD: Multiple updates can race
let mut cache = state.response_cache.write();
cache.put(key, old_value);
// Another thread might update here
cache.put(key, new_value); // Lost update!
```

**Solution:**
```rust
// GOOD: Single atomic update
let mut cache = state.response_cache.write();
cache.put(key, value); // LRU cache handles this atomically
```

## Async and Locks

### Never Hold Locks Across Await

**BAD:**
```rust
async fn bad_function(state: &AppState) {
    let mut cache = state.response_cache.write();
    let data = fetch_from_network().await; // Lock held during await!
    cache.put(key, data);
}
```

**GOOD:**
```rust
async fn good_function(state: &AppState) {
    let data = fetch_from_network().await; // No lock held
    let mut cache = state.response_cache.write();
    cache.put(key, data); // Lock held only briefly
}
```

## Memory Safety

### Bounded Growth

All caches have bounded size:

```rust
// LRU cache with fixed size
let cache = LruCache::new(
    NonZeroUsize::new(200).expect("200 > 0")
);
```

This prevents unbounded memory growth.

### Arc for Shared Ownership

State is shared via `Arc`:

```rust
let state = Arc::new(AppState::new());
let state_clone = state.clone(); // Cheap clone, shares data
```

## Testing Concurrency

### Concurrency Tests

Test for race conditions:

```rust
#[tokio::test]
async fn test_concurrent_cache_access() {
    let state = Arc::new(AppState::new());
    let mut handles = vec![];
    
    for i in 0..100 {
        let state_clone = state.clone();
        handles.push(tokio::spawn(async move {
            cache_response(&state_clone, "model", &format!("prompt_{}", i), &i).await
        }));
    }
    
    futures::future::join_all(handles).await;
    // Verify cache state
}
```

## Best Practices

1. **Always acquire locks in consistent order**
2. **Keep critical sections short**
3. **Never hold locks across await points**
4. **Use atomics for simple counters**
5. **Bound all caches and collections**
6. **Test concurrent access patterns**

## Common Pitfalls

1. **Lock ordering violations** - Can cause deadlocks
2. **Holding locks during I/O** - Blocks other threads
3. **Holding locks across await** - Blocks async runtime
4. **Unbounded collections** - Memory leaks
5. **TOCTOU races** - Incorrect cache behavior

## Migration from Global State

Old code used global `Mutex`:

```rust
// OLD: Global state
lazy_static! {
    static ref STATE: Mutex<State> = Mutex::new(State::new());
}

fn get_state() -> State {
    STATE.lock().unwrap().clone()
}
```

New code uses `AppState`:

```rust
// NEW: Explicit state
fn get_state(state: &AppState) -> State {
    state.session_state.read().clone()
}
```

This makes dependencies explicit and prevents hidden lock ordering issues.
