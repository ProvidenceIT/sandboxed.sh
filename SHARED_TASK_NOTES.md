# Encrypted Env Vars for Workspace Templates

## Goal
Implement encryption-at-rest for workspace template environment variables using a PRIVATE_KEY stored in .env. UI displays/edits plaintext (decryption happens on backend).

## Design Decisions

### Where Encryption Happens
- **Encryption**: Rust backend on template save (`Library::save_workspace_template`)
- **Decryption**: Rust backend on template load (`Library::get_workspace_template`)
- **Why backend**: PRIVATE_KEY must never reach browser. Library module handles template CRUD and is the natural boundary.

### Encrypted Field Format
```
<encrypted v="1">BASE64_NONCE:BASE64_CIPHERTEXT</encrypted>
```
- `v="1"` for future version upgrades
- Nonce and ciphertext separated by `:` inside the tag
- Random nonce per encryption (AES-256-GCM requirement)

### Key Management
- **Env var**: `TEMPLATE_ENCRYPTION_KEY` (32-byte hex string or base64)
- **Auto-generation**: If missing at startup, generate and append to `.env`
- **Fail-safe**: If can't write to `.env`, log warning but don't crash (encryption disabled)

### Backward Compatibility
- On load: `is_encrypted()` checks for `<encrypted v="1">` wrapper
  - If wrapped → decrypt
  - If plaintext → pass through unchanged
- On save: Always encrypt sensitive fields (env_vars values)
- No double-encryption: `encrypt_string()` checks if already encrypted

### Implementation Plan

1. **New module**: `src/library/template_crypto.rs`
   - `load_or_create_private_key()` → reads from env, generates if missing
   - `is_encrypted(value: &str) -> bool`
   - `encrypt_string(key: &[u8], plaintext: &str) -> String` → `<encrypted v="1">...</encrypted>`
   - `decrypt_string(key: &[u8], value: &str) -> Result<String>` → passthrough or decrypt
   - Uses same AES-256-GCM as `src/secrets/crypto.rs` but simpler (no PBKDF2 - key is direct)

2. **Modify `src/library/mod.rs`**:
   - `get_workspace_template`: Decrypt env_vars values after JSON parse
   - `save_workspace_template`: Encrypt env_vars values before JSON serialize
   - `list_workspace_templates`: No change (summaries don't include env_vars)

3. **Tests**:
   - Roundtrip: encrypt → save → load → decrypt = original
   - Mixed: some encrypted, some plaintext → both work
   - Legacy: plaintext JSON loads correctly
   - No double-encrypt: encrypting already-encrypted value is no-op

## Files to Modify
- `src/library/mod.rs` - template load/save
- `src/library/template_crypto.rs` - NEW, crypto utilities
- `.env.example` - add TEMPLATE_ENCRYPTION_KEY example
- Tests in `src/library/mod.rs` or new test file

## Current Status
**Iteration 2**: `template_crypto.rs` implemented with all crypto utilities and 8 passing tests.

### Completed
- [x] Design doc in SHARED_TASK_NOTES.md
- [x] `src/library/template_crypto.rs` with:
  - `init_encryption_key()` - loads from env or auto-generates
  - `is_encrypted()` - detects `<encrypted v="1">` wrapper
  - `encrypt_string()` / `decrypt_string()` - AES-256-GCM
  - `encrypt_env_vars()` / `decrypt_env_vars()` - HashMap helpers
  - No double-encrypt protection
  - Plaintext passthrough for backward compatibility
- [x] Added `hex = "0.4"` dependency to Cargo.toml
- [x] Module registered in `src/library/mod.rs`

### Next Steps
1. **Integrate into template load/save** (`src/library/mod.rs`):
   - Call `decrypt_env_vars()` in `get_workspace_template()` after JSON parse
   - Call `encrypt_env_vars()` in `save_workspace_template()` before JSON serialize
   - Call `init_encryption_key()` once at app startup (in `main.rs` or `Library::new()`)

2. **Add integration tests**:
   - Save template with env_vars → verify JSON file has encrypted values
   - Load template → verify plaintext is returned to caller
   - Legacy plaintext templates still load correctly

3. **Update `.env.example`** with `TEMPLATE_ENCRYPTION_KEY` placeholder

4. **Key rotation** (future iteration)

## Test Commands
```bash
cargo test template_crypto
cargo test workspace_template
```

## Pitfalls
- The `OnceLock` for the key means tests may share state - use `setup_test_key()` carefully
- If encryption fails, functions return original value (graceful degradation)
- Key must be initialized before first template load/save
