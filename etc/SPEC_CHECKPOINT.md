# SPEC: Checkpoint & Cloud Migration Protocol

## VMCheckpoint Data Structure

```rust
struct VMCheckpoint {
    header:           CheckpointHeader,
    registers:        RegisterState,
    page_table_dump:  PageTableDump,
    segments:         Vec<SegmentDescriptor>,
    metadata:         Vec<TlvBlock>,
    checksum:         [u8; 32],
}

struct CheckpointHeader {
    magic:            u32,       // 0x5348564D ("SHVM")
    version:          u32,       // protocol version (currently 1)
    total_length:     u64,
    checkpoint_id:    u64,       // monotonically increasing
    timestamp:        u64,       // UNIX microseconds
    kind:             CheckpointKind,
}

enum CheckpointKind {
    Full,                        // all mapped pages included
    Incremental { base_id: u64 }, // only dirty pages since base_id
    Final { base_id: u64 },      // last incremental before pause+migrate
}

struct RegisterState {
    x:      [u64; 32],           // x0-x31 (x0 always reads 0, not serialized)
    f:      [u64; 32],           // f0-f31 (IEEE 754 double in u64 bits)
    pc:     u64,
    csr:    CsrFile,
}

struct CsrFile {
    mstatus:   u64,
    mepc:      u64,
    mcause:    u64,
    mtvec:     u64,
    mie:       u64,
    mip:       u64,
    mhartid:   u64,
    mscratch:  u64,
    // + vendor-defined as needed
}

struct PageTableDump {
    page_count: u32,
    pages:      Vec<PageDump>,
}

struct PageDump {
    pfn:             u64,
    flags:           u16,       // PageFlags bitfield
    access_summary:  TlvBlock,  // PageAccessSummary, TLV encoded
    dirty_mask:      u64,       // 64-bit cache line bitmap
    data:            [u8; 4096],
}
```

## TLV Encoding

For variable-length fields (metadata, summaries, segment lists):

```
[2 bytes]  type_tag
[4 bytes]  length     (payload length in bytes)
[length]   payload    (type-specific data)
```

Type tags:
```
0x0001:  PageAccessSummary
0x0002:  SegmentDescriptor
0x0003:  HotBlockInfo
0x0004:  LoopInfo
0x0005:  LoweredRegion
0x0006:  GpuPageMap
0x0007:  ExecutionMetadata
```

### TLV: PageAccessSummary (0x0001)

```
[1]     access_pattern    (u8 enum)
[8]     total_accesses    (u64)
[8]     read_count        (u64)
[8]     write_count       (u64)
[8]     fetch_count       (u64)
[4]     histogram_entries  (u32)
[for each entry:
  [8]     stride          (i64)
  [8]     count           (u64)
]
[8]     last_address      (u64)
```

### TLV: HotBlockInfo (0x0003)

```
[8]  start_pc           (u64)
[8]  end_pc             (u64)
[8]  execution_count    (u64)
[4]  avg_instructions   (u32)
```

### TLV: LoweredRegion (0x0005)

```
[8]  start_vaddr        (u64)
[8]  end_vaddr          (u64)
[1]  lowering_kind      (u8: 0=scalar, 1=simd, 2=gpu)
[4]  gpu_page_count     (u32)
[for each gpu page:
  TLV 0x0006 (GpuPageMap)
]
[2]  shader_source_len  (u16, 0 if not GPU)
[shader_source_len]  shader_source  (WGSL string)
[8]  lowered_at         (u64 checkpoint ID)
```

### TLV: GpuPageMap (0x0006)

```
[8]  pfn                (u64)
[4]  gpu_buffer_id       (u32)
[8]  gpu_offset          (u64)
[1]  map_kind            (u8: 0=constant, 1=input, 2=output, 3=local)
[1]  coherent            (bool)
```

## Binary File Format

The full checkpoint is serialized as a flat binary blob:

```
Offset  Size    Field
──────  ────    ─────
0       4       magic (0x5348564D)
4       4       version (1)
8       8       total_length
16      8       checkpoint_id
24      8       timestamp
32      1       checkpoint_kind (0=full, 1=incremental, 2=final)
33      4       padding
37      2056    register_state (8*32 + 8*32 = 512 bytes for x+f;
                then 8 pc + ~1500 CSRs = ~1500 bytes;
                256 bytes reserved for CSR expansion)

2093    4       segment_count
2097    [TLV]   segments (0x0002, one per segment)
[var]   4       page_count
[for each page:
  [8]      pfn
  [2]      flags
  [TLV]    access_summary (0x0001)
  [8]      dirty_mask
  [4096]   page_data
]
[var]   [TLV]   metadata blocks
  [TLV] hotspot_blocks (0x0003, repeated)
  [TLV] lowered_regions (0x0005, repeated)
[4]     tlv_count
[32]    SHA-256 checksum (of everything above)
```

## REST API

Base URL: `https://{node}/v1`

### Authentication

Bearer token in `Authorization` header.

### Endpoints

#### POST /v1/instances

Create a new VM instance.

```
Request:
  {
    "source_kind": "rv64",       // "rv64" | "dsl"
    "max_pages": 1048576,        // optional, default 1M
    "labels": { "tenant": "..." }  // optional
  }

Response 201:
  {
    "instance_id": "abc-def-123",
    "created_at": 1700000000000000
  }
```

#### POST /v1/instances/{id}/load

Load initial state into an instance. Accepts multipart binary upload.

```
Request:
  Content-Type: application/octet-stream
  Body: VMCheckpoint binary

Response 200:
  {
    "instance_id": "abc-def-123",
    "status": "loaded",
    "pages_loaded": 1024
  }
```

#### GET /v1/instances/{id}/state

Fetch current VM state.

```
Query params:
  ?kind=full            (default: all pages)
  ?kind=incremental     (only dirty pages)
  ?base_id=123          (for incremental: since this checkpoint)

Response 200:
  Content-Type: application/octet-stream
  Body: VMCheckpoint binary
```

#### POST /v1/instances/{id}/execute

Execute for up to `max_cycles` or until a break condition.

```
Request:
  {
    "max_cycles": 1000000,
    "break_on": "syscall",       // optional, also "page_fault", "ecall"
    "timeout_ms": 5000           // optional
  }

Response 200:
  {
    "cycles_executed": 1000000,
    "instructions_executed": 987654,
    "status": "running",         // "running" | "ecall" | "break" | "timeout"
    "hotspot_regions": [
      { "start_pc": 4096, "end_pc": 4200, "count": 10000 },
      ...
    ]
  }
```

#### POST /v1/instances/{id}/migrate

Initiate migration to another node.

```
Request:
  {
    "target_node": "node-east-2.shardvm.internal:443",
    "strategy": "live"           // "live" | "cold"
  }

Response 202:
  {
    "migration_id": "mig-456",
    "status": "initiated"
  }
```

#### GET /v1/instances/{id}/migrations/{mid}/status

Poll migration progress.

```
Response 200:
  {
    "migration_id": "mig-456",
    "status": "in_progress",     // "in_progress" | "completed" | "failed"
    "dirty_pages_remaining": 42,
    "iterations": 3
  }
```

#### DELETE /v1/instances/{id}

Terminate and free all resources.

```
Response 200:
  {
    "instance_id": "abc-def-123",
    "status": "terminated"
  }
```

## Migration Protocol

### Cold Migration

1. Client sends `POST /v1/instances/{id}/migrate {strategy: "cold"}`
2. Source node:
   a. Pause interpreter execution
   b. Capture VMCheckpoint (full snapshot, all pages)
   c. POST checkpoint to target node: `POST /v1/instances/{new_id}/load`
   d. Target validates checksum, allocates pages, rebuilds GPU resources
     from `LoweredRegion` records
   e. Target sends 200 OK
3. Source node:
   a. POST `/v1/instances/{old_id}/migration/{mid}/ack` to target
   b. Destroy local VM, free pages
4. Target node resumes execution

### Live Migration (Incremental)

1. Source node captures incremental checkpoint (dirty pages only).
   Interpreter continues running.
2. Source sends dirty pages to target.
3. Target loads pages, validates.
4. Repeat steps 1-3 until:
   a. Dirty page count < 64 (small enough for quick final transfer)
   b. OR iteration count > 10 (fallback to cold)
5. **Pause**: Source pauses interpreter (microseconds).
6. Capture final incremental: registers + final dirty pages.
7. Transfer final diff to target.
8. Target validates, acknowledges.
9. Source destroys local VM.
10. Target resumes from final checkpoint.

### Migration Wire Format

```
POST /v1/migrate/receive
Content-Type: application/octet-stream

[VMCheckpoint binary]
```

The receiving node validates the checksum, allocates backing store,
rebuilds GPU buffers from LoweredRegion records, and returns an ACK
with the new instance ID.

### Migration Failure Handling

- If checksum fails: target requests retransmission (up to 3 retries)
- If target cannot allocate pages: return 507 (Insufficient Storage)
- If migration fails mid-way: source keeps running, client retries
- If target crashes after ACK but before resume: source retains a
  post-pause snapshot for recovery
