# SPEC: Virtual Address Space & Page Table

## Overview

The ShardVM uses a flat (non-hierarchical) page table with 4KB pages and a
64-bit virtual address space. Each page is backed by host memory and/or a
GPU buffer, with coherency managed via per-page flags. Every load/store in
the RISC-V interpreter routes through the virtual memory layer, which records
access patterns for later analysis.

## Page Size

- **4 KB** (4096 bytes), matching the standard OS page size and RISC-V Sv39
  page size
- Page frame number (PFN): `address >> 12`
- Page offset: `address & 0xFFF`

## Virtual Address Space Layout

```
Range                        Size        Segment         Perms   Purpose
──────────────────────────────────────────────────────────────────────────
0x0000_0000_0000_0000        4KB         NULL            ---     Always traps (catch null deref)
0x0000_0000_0000_1000        16MB        TEXT            R-X     Executable code (.text)
0x0000_0000_0100_0000        —           GAP             ---     Guard gap (traps on overrun)
0x0001_0000_0000_0000        64MB        RODATA          R--     Read-only data, constants
0x0001_0000_0400_0000        —           GAP             ---
0x0002_0000_0000_0000        64MB        DATA            RW-     Global variables, .data, .bss
0x0002_0000_0400_0000        —           GAP             ---
0x0003_0000_0000_0000        256MB       HEAP            RW-     Dynamically allocated, grows up
0x0003_0000_1000_0000        —           GAP             ---
0x0004_0000_0000_0000        512MB       TENSOR          RW-     Large array/tensor allocations
0x0004_0000_2000_0000        —           GAP             ---
0x7FFF_FFFF_FFFF_F000        128KB       STACK           RW-     Stack, grows downward from top
0xFFFF_FFFF_8000_0000        256MB       MMIO            varies  Memory-mapped I/O, kernel interface
```

- GAP regions are unmapped. Any access to a GAP triggers a page fault trap.
- STACK grows downward: initial `sp` points to `0x8000_0000_0000_0000`; pages
  are lazily allocated as sp decreases.
- HEAP grows upward: initial `brk` is at `0x0003_0000_0000_0000`; `sbrk` syscall
  allocates pages upward.
- TENSOR is a separate region for large array allocations, keeping them
  physically contiguous for GPU buffer merging.

## Segment Descriptor

```rust
struct SegmentDescriptor {
    name:        String,
    base_vaddr:  u64,
    bound_vaddr: u64,          // exclusive end
    default_flags: PageFlags,
    page_count:  u32,          // currently mapped pages in this segment
}
```

Page allocation within a segment is lazy (only when first accessed or
explicitly requested). On first access to an unmapped page within a valid
segment range, a page fault handler allocates a zero-filled page.

## Page Descriptor

```rust
struct PageDescriptor {
    phys_frame:   Option<FrameId>,  // None = unmapped, Some = backing
    flags:        PageFlags,
    access_log:   RingBuffer<Access, 256>,
    summary:      PageAccessSummary,
    gpu_binding:  Option<GpuPageMap>,
}

struct FrameId {
    ptr:   *mut u8,            // pointer to 4096-byte host allocation
}

struct RingBuffer<T, const N: usize> {
    buf:  [T; N],
    head: usize,
    size: usize,               // number of valid entries
}
```

### PageFlags

Bitfield stored as `u16`:

| Bit | Name            | Description                                               |
|-----|-----------------|-----------------------------------------------------------|
| 0   | READABLE        | Page supports load instructions                           |
| 1   | WRITABLE        | Page supports store instructions                          |
| 2   | EXECUTABLE      | Page contains executable code                             |
| 3   | DIRTY           | Page modified since last checkpoint or GPU upload         |
| 4   | GPU_READABLE    | GPU has a readable copy of this page                      |
| 5   | GPU_WRITABLE    | GPU has a writable copy of this page                      |
| 6   | COHERENT        | Host and GPU copies are identical                         |
| 7   | STALE           | GPU copy is newer than host copy                          |
| 8   | ACCESSED        | Page has been accessed (referenced bit)                   |
| 9   | LAZY_ALLOC      | Page not yet physically allocated, traps on access        |
| 10  | TENSOR_CONTIG   | Page is part of a contiguous tensor allocation            |
| 11-15| RESERVED       | For future use                                            |

### Access Log

```rust
struct Access {
    kind:     AccessKind,
    addr:     u64,             // full virtual address
    size:     u8,              // bytes: 1, 2, 4, 8, or vector width
    cycle:    u64,             // instruction cycle counter
}

enum AccessKind {
    Load,
    Store,
    Fetch,                     // instruction fetch
}
```

The ring buffer holds up to 256 entries per page. When full, the oldest
128 entries are consumed by `summarize()` and the ring advances.

### Page Access Summary

```rust
struct PageAccessSummary {
    total_accesses:    u64,
    read_count:        u64,
    write_count:       u64,
    fetch_count:       u64,
    stride_histogram:  HashMap<i64, u64>,   // stride → frequency
    access_pattern:    AccessPattern,
    last_address:      u64,
}

enum AccessPattern {
    Unknown,
    Sequential { stride: i64 },
    Strided { stride: i64 },
    Indirect,                     // dependent on another load
    Random,
    Broadcast,                    // same address, many reads
    ReductionTarget,              // single address, many writes
}
```

`summarize()` consumes the oldest 128 entries from the ring buffer:

```rust
impl PageDescriptor {
    fn summarize(&mut self) {
        let entries = self.access_log.drain_oldest(128);
        let mut histogram = HashMap::new();
        let mut prev_addr = None;

        for acc in &entries {
            self.summary.total_accesses += 1;
            match acc.kind {
                Load  => self.summary.read_count += 1,
                Store => self.summary.write_count += 1,
                Fetch => self.summary.fetch_count += 1,
            }

            if let Some(prev) = prev_addr {
                let stride = acc.addr as i64 - prev as i64;
                *histogram.entry(stride).or_insert(0) += 1;
            }
            prev_addr = Some(acc.addr);
        }

        // Merge histogram
        for (k, v) in histogram {
            *self.summary.stride_histogram.entry(k).or_insert(0) += v;
        }

        // Reclassify pattern
        self.summary.access_pattern = classify(&self.summary.stride_histogram);
        self.summary.last_address = entries.last().unwrap().addr;
    }
}

fn classify(hist: &HashMap<i64, u64>) -> AccessPattern {
    let total: u64 = hist.values().sum();
    if total == 0 { return AccessPattern::Unknown; }

    let dominant = hist.iter().max_by_key(|(_, c)| *c).unwrap();
    let pct = *dominant.1 as f64 / total as f64;

    if *dominant.0 == 0 && pct > 0.9 {
        AccessPattern::Broadcast
    } else if *dominant.0 > 0 && pct > 0.9 {
        AccessPattern::Sequential { stride: *dominant.0 }
    } else if pct > 0.5 {
        AccessPattern::Strided { stride: *dominant.0 }
    } else {
        AccessPattern::Random
    }
}
```

## Virtual Load/Store Path

### Virtual Load

```rust
fn virtual_load(vm: &VM, addr: u64, size: u8) -> Result<u64, Trap> {
    if addr % size as u64 != 0 {
        return Err(Trap::MisalignedAddress(addr));
    }

    let pfn = addr >> 12;
    let offset = addr & 0xFFF;

    let page = vm.page_table
        .get(&pfn)
        .ok_or(Trap::PageFault { addr, cause: "unmapped" })?;

    if !page.flags.contains(PageFlags::READABLE) {
        return Err(Trap::PageFault { addr, cause: "not readable" });
    }

    if page.flags.contains(PageFlags::LAZY_ALLOC) {
        page.phys_frame = Some(allocate_frame()?);
        page.flags.remove(PageFlags::LAZY_ALLOC);
    }

    page.flags.insert(PageFlags::ACCESSED);
    page.record_access(Access {
        kind: Load, addr, size,
        cycle: vm.cycle_count,
    });

    match &page.phys_frame {
        Some(frame) => {
            let ptr = unsafe { frame.ptr.add(offset) };
            match size {
                1 => Ok(unsafe { ptr.read::<u8>() as u64 }),
                2 => Ok(unsafe { ptr.read::<u16>() as u64 }),
                4 => Ok(unsafe { ptr.read::<u32>() as u64 }),
                8 => Ok(unsafe { ptr.read::<u64>() }),
                _ => Err(Trap::UnsupportedSize(size)),
            }
        }
        None => Err(Trap::PageFault { addr, cause: "no backing" }),
    }
}
```

### Virtual Store

```rust
fn virtual_store(vm: &mut VM, addr: u64, size: u8, value: u64) -> Result<(), Trap> {
    // ... alignment check, page lookup, permission check (WRITABLE) ...

    page.flags.insert(PageFlags::DIRTY);       // dirty for checkpoint
    page.flags.remove(PageFlags::COHERENT);    // no longer coherent with GPU

    page.record_access(Access {
        kind: Store, addr, size,
        cycle: vm.cycle_count,
    });

    // ... write to phys_frame ...
}
```

## Page Fault Handler

```rust
fn handle_page_fault(vm: &mut VM, addr: u64, cause: &str) -> Result<(), Trap> {
    let pfn = addr >> 12;

    // Check if address falls within a known segment
    let seg = vm.segments.iter()
        .find(|s| addr >= s.base_vaddr && addr < s.bound_vaddr);

    match seg {
        Some(seg) => {
            // Lazy allocation within valid segment
            let mut page = PageDescriptor {
                phys_frame: Some(allocate_frame()?),
                flags: seg.default_flags,
                access_log: RingBuffer::new(),
                summary: PageAccessSummary::default(),
                gpu_binding: None,
            };
            vm.page_table.insert(pfn, page);
            Ok(())
        }
        None => Err(Trap::PageFault { addr, cause: "address not in any segment" }),
    }
}
```

## Dirty Tracking for Checkpoint

```rust
// Mark a page dirty on write
page.flags.insert(PageFlags::DIRTY);

// Mark a specific 64-byte cache line dirty
fn mark_cacheline_dirty(page: &mut PageDescriptor, offset: u16) {
    let line = (offset / 64) as usize;  // 4096 / 64 = 64 cache lines
    page.dirty_mask |= 1 << line;
    page.flags.insert(PageFlags::DIRTY);
}

// Get dirty pages for incremental checkpoint
fn dirty_pages(table: &PageTable) -> Vec<(u64, &PageDescriptor)> {
    table.iter()
        .filter(|(_, p)| p.flags.contains(PageFlags::DIRTY))
        .map(|(pfn, p)| (*pfn, p))
        .collect()
}

// Clear dirty flags after checkpoint
fn mark_all_clean(table: &mut PageTable) {
    for (_, page) in table.iter_mut() {
        page.flags.remove(PageFlags::DIRTY);
    }
}
```

## Global Trace Buffer

In addition to per-page logs, the VM maintains a global instruction trace:

```rust
struct GlobalTrace {
    entries: Vec<InsnTrace>,
    hot_blocks: HashMap<u64, u64>,  // PC → execution count
}

struct InsnTrace {
    pc:         u64,
    opcode:     u32,
    mem_access: Option<MemAccessRecord>,
}

struct MemAccessRecord {
    addr: u64,
    size: u8,
    kind: AccessKind,
    value: u64,
}
```

The global trace is bounded (e.g., last 10K instructions). When the buffer
fills, it is flushed to a circular file or consumed by the pattern detector.
