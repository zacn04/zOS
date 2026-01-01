# Precompute Fix Test Results

## Test Plan

### Scenario 1: User selects problem from Learn page ❌ Should NOT precompute
**Flow:**
1. User on Learn page (`/learn`)
2. Selects a problem (e.g., "RL Theory")
3. Clicks "Start Problem"
4. Navigate to: `/solve?problem=rl_001&source=user`

**Expected:**
- Frontend: `shouldPrecompute = false` (because `source === "user"`)
- Backend: `precompute_next_problem` should NOT be called
- No LLM calls made

**Code verification:**
```typescript
// learn/+page.svelte:61
goto(`/solve?problem=${problem.id}&source=user`);
```

```typescript
// solve/+page.svelte:274
const shouldPrecompute = source !== "user"; // = false when source="user"
```

```typescript
// solve/+page.svelte:254
if (shouldPrecompute) {  // This block will NOT execute
  invoke("precompute_next_problem")...
}
```

---

### Scenario 2: User gets recommended problem from Improve page ✅ SHOULD precompute
**Flow:**
1. User on Improve page (`/improve`)
2. Clicks "Get Targeted Problem to Improve"
3. Gets a recommended problem
4. Clicks "Start Solving"
5. Navigate to: `/solve?problem=xyz_123&source=recommended`

**Expected:**
- Frontend: `shouldPrecompute = true` (because `source === "recommended"`)
- Backend: `precompute_next_problem` SHOULD be called
- 3 LLM calls made in background

**Code verification:**
```typescript
// improve/+page.svelte:294
goto(`/solve?problem=${recommendedProblem.id}&source=recommended`);
```

```typescript
// solve/+page.svelte:274
const shouldPrecompute = source !== "user"; // = true when source="recommended"
```

```typescript
// solve/+page.svelte:254
if (shouldPrecompute) {  // This block WILL execute
  invoke("precompute_next_problem")...
}
```

---

### Scenario 3: Direct URL navigation (no source) ✅ SHOULD precompute (backward compatible)
**Flow:**
1. User navigates directly to `/solve?problem=xyz_123`
2. No source parameter

**Expected:**
- Frontend: `shouldPrecompute = true` (because `source !== "user"`, source is null)
- Backend: `precompute_next_problem` SHOULD be called
- Backward compatible with old behavior

**Code verification:**
```typescript
// solve/+page.svelte:274
const shouldPrecompute = source !== "user"; // = true when source=null
```

---

## Manual Verification Steps

### Step 1: Check URL Generation
✅ Learn page generates: `/solve?problem=XXX&source=user`
✅ Improve page generates: `/solve?problem=XXX&source=recommended`

### Step 2: Check shouldPrecompute Logic
✅ When `source="user"`: shouldPrecompute = false
✅ When `source="recommended"`: shouldPrecompute = true
✅ When `source=null`: shouldPrecompute = true (backward compatible)

### Step 3: Check Backend Calls
✅ `loadProblemById(id, false)` does NOT call `invoke("precompute_next_problem")`
✅ `loadProblemById(id, true)` DOES call `invoke("precompute_next_problem")`

---

## Code Review Checklist

✅ **Frontend Changes**
- [x] `solve/+page.svelte` - Added `shouldPrecompute` parameter to `loadProblemById()`
- [x] `solve/+page.svelte` - Check source param in $effect
- [x] `solve/+page.svelte` - Conditional `invoke("precompute_next_problem")`
- [x] `learn/+page.svelte` - Navigate with `&source=user`
- [x] `improve/+page.svelte` - Navigate with `&source=recommended`

✅ **Backend Logging**
- [x] Added `tracing::info!` to `precompute_next_problem` function
- [x] Added console.log to frontend $effect

---

## Expected Log Output

### User-selected problem (Learn page):
```
Frontend console:
📋 Loading problem: rl_001, source: user, shouldPrecompute: false

Backend logs:
(NO PRECOMPUTE LOGS - function not called)
```

### Recommended problem (Improve page):
```
Frontend console:
📋 Loading problem: xyz_123, source: recommended, shouldPrecompute: true

Backend logs:
🔄 PRECOMPUTE TRIGGERED - Starting background problem generation
🔄 PRECOMPUTE - Generating 3 problems (base_difficulty: 0.5)
```

---

## Test Status: ✅ PASS

The fix is working correctly based on code review:
1. ✅ URLs are generated with correct source parameter
2. ✅ shouldPrecompute logic correctly evaluates source
3. ✅ Backend precompute is only called when shouldPrecompute=true
4. ✅ Backward compatible (null source still precomputes)
