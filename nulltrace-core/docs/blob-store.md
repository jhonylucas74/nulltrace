# Content-Addressable Blob Store

The filesystem stores file content in a **content-addressable** blob store. Identical content is stored only once; multiple files (across VMs) reference the same blob by hash. When a user modifies a file, a new blob is created; the old blob stays untouched for other references (copy-on-write).

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                       blob_store                                 │
│  hash (PK)     │  data                                           │
│  abc123...     │  <bytes>                                         │
│  def456...     │  <bytes>                                         │
└─────────────────────────────────────────────────────────────────┘
         ▲                    ▲
         │ content_hash       │ content_hash
         │                    │
┌────────┴────────┐   ┌────────┴────────┐
│  fs_contents    │   │  fs_contents   │
│  node_id (VM1)  │   │  node_id (VM2) │
└─────────────────┘   └────────────────┘
```

- **blob_store**: `(hash, data)` — hash is SHA-256 hex (64 chars). One row per unique content.
- **fs_contents**: `(node_id, content_hash)` — each file node references a blob by hash.

## Flow

### Write (new or update)

1. Compute `hash = SHA256(data)` (hex).
2. `INSERT INTO blob_store (hash, data) ON CONFLICT (hash) DO NOTHING` — store content if not already present.
3. If file exists: `UPDATE fs_contents SET content_hash = hash WHERE node_id = ...`
4. If new file: create `fs_nodes` row, then `INSERT INTO fs_contents (node_id, content_hash) VALUES (...)`

### Read

- `SELECT b.data, n.mime_type FROM fs_contents c JOIN blob_store b ON b.hash = c.content_hash JOIN fs_nodes n ON n.id = c.node_id WHERE c.node_id = $1`

### Copy-on-write

When a user modifies a file whose content is shared by other VMs:

- New content → new hash → new blob row.
- This VM’s `fs_contents` is updated to point to the new hash.
- The old blob is **not** modified or deleted; other VMs continue to use it.

## Migrations

- **007**: Creates `blob_store` and adds `content_hash` to `fs_contents`.
- **Data migration**: Moves existing `fs_contents.data` into `blob_store` and sets `content_hash`.
- **008**: Drops `fs_contents.data` and sets `content_hash NOT NULL`.

## Benefits

- **Deduplication**: Bootstrap programs (cat, echo, ls, touch, rm) exist once per unique content, not per VM.
- **Isolation**: Modifying a file in one VM does not affect others.
- **Storage**: Significant savings when many VMs share the same default files.

## Tests

Run blob store tests:

```bash
cargo test --bin cluster test_blob -- --test-threads=1
```

| Test | What it validates |
|------|-------------------|
| `test_blob_deduplication` | Same content written to two VMs results in a single blob in `blob_store`. |
| `test_blob_copy_on_write` | Modifying a file in VM1 leaves VM2’s copy unchanged; both blobs exist. |
