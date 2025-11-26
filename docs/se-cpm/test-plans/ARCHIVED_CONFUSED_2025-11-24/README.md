# ARCHIVED: Confused Test Plans - 2025-11-24

**Status:** QUARANTINED - DO NOT USE
**Reason:** These test plans describe components that were never implemented
**Date Archived:** 2025-11-24
**Archived By:** Claude Code (with Patrick's approval)

---

## Why These Were Archived

**The Problem:** Classic manifest-drift scenario
- **Phase 6** (TESTING PLAN): Created L5-TESTPLAN for theoretical "AgentOrchestrator" architecture
- **Phase 9** (IMPLEMENT): Built simpler "Agent" implementation instead
- **Phase 10** (EXECUTE TESTS): Discovered 84% of planned tests were for non-existent code
- **Result:** 546KB of test specifications describing components that don't exist

**Example Mismatch:**

```rust
// ❌ What L5-TESTPLAN expected (NEVER BUILT):
AgentOrchestrator::new(manifest_path, llm_client, state_manager)
orchestrator.tool_registry
orchestrator.quality_gates
LLMClient::new_with_provider("anthropic", key)
LLMClient::new_with_keys(HashMap<provider, key>)

// ✅ What was actually implemented:
Agent::new(manifest, api_key, window)
agent.manifest
agent.llm_client
LLMClient::new(api_key)  // Single provider detection
```

**Impact:**
- Planned: 259 tests for theoretical architecture
- Actually needed: 80-120 tests for actual implementation
- Days lost trying to reconcile the mismatch

---

## What's In This Archive

**Confused Documents:**
- `L5-TESTPLAN-BATTERY-TEST-SPECIFICATIONS-COMPLETE.md` - 546KB, 244 tests for AgentOrchestrator
- `L5-TESTPLAN-BATTERY-TEST-SPECIFICATIONS.md` - 75KB, battery specs
- `L5-TESTPLAN-TestSpecification.md` - 362KB, full theoretical spec
- `BATTERY_TEST_*.md` - Component interaction matrices, edge cases, mock completion
- Multiple backups and versions (`.bak` files)

**Total:** ~3.6MB of documentation for code that doesn't exist

---

## The Fresh Start

**New Approach:**
1. ✅ Quarantine confused documentation (this archive)
2. ✅ Keep actual working tests (59 tests in src-tauri/src/*.rs and src-tauri/tests/*.rs)
3. ⏳ Read the ACTUAL implementation
4. ⏳ Build NEW test plan using CDP-PHASE-06-TESTING-PLAN-Enhanced.md methodology
5. ⏳ Implement 80-120 strategic multi-modal tests (N:1 mapping, not brute-force 1:1)

**New Test Plan Location:** `docs/se-cpm/test-plans/FRESH-TESTPLAN-2025-11-24.md`

---

## Lessons Learned

**What Went Wrong:**
1. Test plan created before implementation was complete
2. No validation that planned tests matched actual code
3. Implementation simplified during Phase 9 without updating test plan
4. Multiple versions created confusion (15 files, 3.5MB)

**How to Prevent:**
1. ✅ Test plans MUST reference actual implementation
2. ✅ Validate Component-IDs exist before writing tests
3. ✅ Use multi-modal testing (one test validates multiple components)
4. ✅ Keep test plans lean (~50-100 pages, not 546KB)
5. ✅ Archive old versions aggressively

---

## If You Need to Reference These

**Don't.** Seriously, don't use these documents. They will lead you astray.

If you absolutely must reference them:
- Remember: They describe code that was NEVER written
- Focus on test CONCEPTS, not specific implementations
- Validate against actual code before copying anything

**Better approach:** Start fresh with actual implementation.

---

**Quarantine Date:** 2025-11-24
**Next Steps:** See `FRESH-TESTPLAN-2025-11-24.md` for the real test plan
**Status:** ✅ ARCHIVED - Ready for fresh start
