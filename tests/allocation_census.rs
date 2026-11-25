#![cfg(feature = "allocation-census")]

use cudarc::driver::{sys, CudaContext};

#[test]
fn device_allocation_counter_follows_context_and_streams() {
    let context = CudaContext::new(0).unwrap();
    let default = context.default_stream();
    let other = context.new_stream().unwrap();
    assert_eq!(context.device_allocations(), 0);

    let mut source = default.alloc_zeros::<u32>(4).unwrap();
    assert_eq!(context.device_allocations(), 1);
    default.memcpy_htod(&[1u32, 2, 3, 4], &mut source).unwrap();
    assert_eq!(context.device_allocations(), 1);

    let copy = source.try_clone().unwrap();
    assert_eq!(context.device_allocations(), 2);
    assert_eq!(default.clone_dtoh(&copy).unwrap(), [1, 2, 3, 4]);
    assert_eq!(context.device_allocations(), 2);

    let other_stream_copy = other.clone_htod(&[7u32, 8]).unwrap();
    assert_eq!(context.device_allocations(), 3);
    assert_eq!(other.clone_dtoh(&other_stream_copy).unwrap(), [7, 8]);

    let sync = unsafe { other.alloc_sync::<u32>(8) }.unwrap();
    assert_eq!(context.device_allocations(), 4);
    drop((sync, other_stream_copy, copy, source));
    other.synchronize().unwrap();
    default.synchronize().unwrap();
    assert_eq!(context.device_allocations(), 4);

    // Managed allocations count on supporting devices. A rejected request
    // does not increment the counter on a device that lacks managed memory.
    let managed_supported = context
        .attribute(sys::CUdevice_attribute::CU_DEVICE_ATTRIBUTE_MANAGED_MEMORY)
        .unwrap()
        != 0;
    let managed = unsafe { context.alloc_unified::<u32>(4, true) };
    assert_eq!(managed.is_ok(), managed_supported);
    assert_eq!(
        context.device_allocations(),
        4 + usize::from(managed_supported)
    );

    let independent = CudaContext::new(0).unwrap();
    assert_eq!(independent.device_allocations(), 0);
    let separate = independent.default_stream().alloc_zeros::<u8>(4).unwrap();
    assert_eq!(independent.device_allocations(), 1);
    assert_eq!(
        context.device_allocations(),
        4 + usize::from(managed_supported)
    );
    drop((separate, managed));
}
