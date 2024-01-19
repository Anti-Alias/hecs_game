use wgpu::{Buffer, Device, BufferDescriptor};

/**
 * Ensures that buffer has at least enough space to fit the number of bytes specified.
 * Does NOT copy contents of old buffer in the case of a resize.
 */
pub fn reserve_buffer(buffer: &mut Buffer, size: u64, device: &Device) {
    if size > buffer.size() {
        println!("Size: {size}");
        *buffer = device.create_buffer(&BufferDescriptor {
            label: None,
            size,
            usage: buffer.usage(),
            mapped_at_creation: false,
        });
    }
}