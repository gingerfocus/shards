# SPEC: GPU Lowering & Coherency Protocol

## Overview

When the execution scheduler decides a code region should run on the GPU,
the host VM pages backing that region's data arrays are mapped to wgpu
buffers, and the computation is compiled into a WGSL compute shader.
A coherency protocol using per-page flags ensures that host and GPU
copies stay synchronized.

## Page → GPU Buffer Mapping

### GpuPageMap

```rust
struct GpuPageMap {
    pfn:          u64,
    buffer_id:    u32,
    gpu_offset:   u64,
    map_kind:     GpuMapKind,
    coherent:     bool,
}

enum GpuMapKind {
    Constant,         // .rodata / constants → uniform buffer (read-only)
    InputArray,       // read-only array → storage buffer (read-only flagged)
    OutputArray,      // write-only or read-write → storage buffer
    Code,             // .text → compiled compute shader module
    Local,            // stack → workgroup-local memory (not a buffer)
}
```

### Buffer Merging

Contiguous pages of the same `map_kind` and compatible access patterns are
merged into a single wgpu::Buffer to minimize bind group entries.

```rust
struct GpuBuffer {
    id:           u32,
    buffer:       wgpu::Buffer,
    size:         u64,
    kind:         GpuMapKind,
    page_maps:    Vec<GpuPageMap>,   // sorted by pfn
}

fn merge_contiguous(pages: &[PageDump]) -> Vec<GpuBuffer> {
    let mut buffers = Vec::new();
    let mut current = None;

    for page in pages {
        let map = classify_page(page);

        if let Some(ref mut buf) = current {
            let last = buf.page_maps.last().unwrap();
            // Merge if same kind and physically contiguous in VAS
            if map.map_kind == buf.kind && map.pfn == last.pfn + 1 {
                map.gpu_offset = buf.size;
                map.buffer_id = buf.id;
                buf.size += 4096;
                buf.page_maps.push(map);
                continue;
            }
        }

        // Start new buffer
        let id = next_buffer_id();
        let mut pages = vec![GpuPageMap {
            pfn: map.pfn, buffer_id: id,
            gpu_offset: 0, map_kind: map.map_kind,
            coherent: false,
        }];
        let buf = GpuBuffer {
            id, buffer: todo!("allocate later"),
            size: 4096, kind: map.map_kind,
            page_maps: pages,
        };
        buffers.push(buf);
    }
    buffers
}
```

## WGSL Compute Shader Generation

### Input

The input is a shared IR `MapLoop` node or a RISC-V basic block that
has been classified as GPU-suitable.

### Transformation: IR MapLoop → WGSL

```
IR MapLoop:
  iterator: i, start: 0, end: N
  body: Add(Load(array_a[i]), Load(array_b[i]))
  output: array_c

WGSL:
  @group(0) @binding(0) var<storage, read> a: array<f64>;
  @group(0) @binding(1) var<storage, read> b: array<f64>;
  @group(0) @binding(2) var<storage, read_write> c: array<f64>;

  @compute @workgroup_size(256)
  fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
      let i = gid.x;
      if (i >= arrayLength(&a)) { return; }
      c[i] = a[i] + b[i];
  }
```

### Codegen Rules

| IR ScalarOp | WGSL |
|-------------|------|
| Add(l, r) | `l + r` |
| Sub(l, r) | `l - r` |
| Mul(l, r) | `l * r` |
| Div(l, r) | `l / r` |
| Neg(x) | `-x` |
| Abs(x) | `abs(x)` |
| Sqrt(x) | `sqrt(x)` |
| Min(l, r) | `min(l, r)` |
| Max(l, r) | `max(l, r)` |
| Clamp(x, lo, hi) | `clamp(x, lo, hi)` |
| Select(cond, t, f) | `select(f, t, cond)` |

| IR ScalarType | WGSL Type |
|---------------|-----------|
| I32 | `i32` |
| I64 | `i64` (requires `--feature shader-i64` on some backends) |
| F32 | `f32` |
| F64 | `f64` (requires `--feature shader-f64` on some backends) |

### Workgroup Size Selection

```rust
fn select_workgroup_size(element_count: u64, device: &wgpu::Adapter) -> u32 {
    let limits = device.limits();
    let max = limits.max_compute_workgroup_size_x;

    // Power of two, capped at 256 (good balance for most GPUs)
    let size = 256.min(max).min(element_count as u32);
    size.next_power_of_two().min(max)
}
```

### GPU Execution Flow

```rust
async fn execute_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    lowered: &LoweredRegion,
    vm_pages: &PageTable,
) -> Result<(), GpuError> {
    // 1. Upload dirty host pages to GPU buffers
    for gpu_page in &lowered.gpu_resources {
        if let Some(page) = vm_pages.get(&gpu_page.pfn) {
            if page.flags.contains(PageFlags::DIRTY) {
                queue.write_buffer(
                    &gpu_buffers[gpu_page.buffer_id],
                    gpu_page.gpu_offset,
                    &page.phys_frame.unwrap().as_slice(),
                );
            }
        }
    }

    // 2. Create bind group
    let bind_group = create_bind_group(device, &lowered, &gpu_buffers);

    // 3. Compile shader (or reuse cached)
    let pipeline = get_or_compile_pipeline(device, &lowered.shader_module)?;

    // 4. Dispatch
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let workgroups = (element_count as u32 + workgroup_size - 1) / workgroup_size;
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    // 5. Submit
    let submission_index = queue.submit(Some(encoder.finish()));

    // 6. Wait for completion
    device.poll(wgpu::Maintain::WaitForSubmissionIndex(submission_index));

    // 7. Mark output pages as STALE + clear COHERENT
    for gpu_page in &lowered.gpu_resources {
        if gpu_page.map_kind == GpuMapKind::OutputArray {
            if let Some(page) = vm_pages.get_mut(&gpu_page.pfn) {
                page.flags.insert(PageFlags::STALE);
                page.flags.remove(PageFlags::COHERENT);
            }
        }
        // Input/constant pages stay COHERENT (GPU didn't write them)
    }

    Ok(())
}
```

## Coherency Protocol

### State Machine

```
                    ┌─────────────────────────────────┐
                    │          PAGE STATE              │
                    └─────────────────────────────────┘

  ┌─────────┐   CPU write    ┌─────────┐   GPU upload    ┌───────────┐
  │ COHERENT│───────────────▶│  DIRTY  │────────────────▶│ COHERENT  │
  │         │                │         │                 │ + GPU buf │
  │ DIRTY=0 │                │ DIRTY=1 │                 │ DIRTY=0   │
  │ STALE=0 │                │ STALE=0 │                 │ STALE=0   │
  └─────────┘                └─────────┘                 └─────┬─────┘
       ▲                                                       │
       │                          GPU compute writes           │
       │                                                       ▼
       │              ┌─────────┐   GPU download   ┌───────────┐
       └──────────────│ COHERENT│◀─────────────────│  STALE    │
                      │ + GPU   │                  │           │
                      │ DIRTY=0 │                  │ DIRTY=0   │
                      │ STALE=0 │                  │ COHERENT=0│
                      └─────────┘                  │ STALE=1   │
                                                   └───────────┘
```

### Transition Rules

| Current State | Event | New State | Action |
|---------------|-------|-----------|--------|
| COHERENT (host) | CPU write | DIRTY | Set DIRTY flag |
| DIRTY | GPU execution starts | COHERENT + GPU | Upload page to GPU buffer; clear DIRTY |
| COHERENT + GPU | CPU reads | COHERENT + GPU | No action; host copy is valid |
| COHERENT + GPU | GPU compute | COHERENT + GPU | No upload needed |
| STALE | CPU reads | COHERENT + GPU | Download GPU buffer to host; clear STALE; set COHERENT |
| STALE | GPU compute | STALE | No action; GPU will overwrite anyway |
| COHERENT + GPU | GPU write | STALE | Clear COHERENT; set STALE |

### Bulk Coherency Operations

```rust
fn prepare_gpu_execution(vm: &mut VM, region: &LoweredRegion) {
    for gpu_page in &region.gpu_resources {
        let page = vm.page_table.get_mut(&gpu_page.pfn).unwrap();

        if page.flags.contains(PageFlags::DIRTY) {
            // Upload dirty host data to GPU
            upload_page_to_gpu(page, gpu_page);
            page.flags.remove(PageFlags::DIRTY);
            page.flags.insert(PageFlags::COHERENT);
        }
    }
}

fn restore_cpu_state(vm: &mut VM, region: &LoweredRegion) {
    for gpu_page in &region.gpu_resources {
        if !gpu_page.map_kind == GpuMapKind::OutputArray { continue; }

        let page = vm.page_table.get_mut(&gpu_page.pfn).unwrap();

        if page.flags.contains(PageFlags::STALE) {
            // Download GPU results to host
            download_page_from_gpu(page, gpu_page);
            page.flags.remove(PageFlags::STALE);
            page.flags.insert(PageFlags::COHERENT);
        }
    }
}
```

## GPU Resource Lifecycle

### Allocation

- GPU buffers are allocated lazily when a region is first lowered
- Buffer sizes are rounded up to `wgpu::COPY_BUFFER_ALIGNMENT` (usually 4 bytes)
- All buffers for a region are created in a single operation to validate
  that the total binding count is within limits

### Deallocation

- When a checkpoint is discarded (older than the last 3), its GPU resources
  are freed
- On migration, GPU resources are destroyed on the source node and
  recreated on the target node

### Persistence in Checkpoints

`LoweredRegion` records in the checkpoint contain enough information to
recreate GPU resources on another node:

```
LoweredRegion {
    start_vaddr, end_vaddr: range of the lowered code/data
    kind: LoweringKind::GPU
    gpu_resources: Vec<GpuPageMap>  // which pages, which buffer, offset, kind
    shader_module: WGSL source      // recompile on target node
    lowered_at: checkpoint_id       // provenance
}
```

## Scheduling Heuristics: CPU SIMD vs GPU

```rust
fn select_backend(loop_info: &MapLoop, data_size: u64) -> LoweringKind {
    let element_count = data_size / loop_info.element_size() as u64;

    // Small arrays: scalar or SIMD on CPU
    if element_count < 1_000 {
        return LoweringKind::Scalar;
    }
    if element_count < 10_000 {
        return LoweringKind::Simd;
    }

    // Medium: check GPU availability and transfer cost
    if element_count < 100_000 {
        // GPU only if data is already GPU-resident
        if all_pages_gpu_coherent(loop_info) {
            return LoweringKind::GPU;
        }
        return LoweringKind::Simd;
    }

    // Large: GPU preferred, but check for PCIe transfer cost
    let transfer_cost = estimate_transfer_time(loop_info);
    let computation_time = element_count as f64 * ESTIMATED_GPU_OPS_PER_ELEMENT;

    if computation_time > transfer_cost * 2.0 {
        LoweringKind::GPU
    } else {
        LoweringKind::Simd
    }
}
```
