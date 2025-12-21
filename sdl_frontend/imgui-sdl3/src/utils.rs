use std::error::Error;

use sdl3::gpu::{Device, TextureCreateInfo, TextureFormat, TextureType, TextureUsage, *};

pub fn create_buffer_with_data<T: Copy>(
    device: &Device,
    transfer_buffer: &TransferBuffer,
    copy_pass: &CopyPass,
    usage: BufferUsageFlags,
    data: &[T],
) -> Result<Buffer, sdl3::Error> {
    // Figure out the length of the data in bytes
    let len_bytes = std::mem::size_of_val(data);

    // Create the buffer with the size and usage we want
    let buffer = device
        .create_buffer()
        .with_size(len_bytes as u32)
        .with_usage(usage)
        .build()?;

    // Map the transfer buffer's memory into a place we can copy into, and copy the data
    //
    // Note: We set `cycle` to true since we're reusing the same transfer buffer to
    // initialize both the vertex and index buffer. This makes SDL synchronize the transfers
    // so that one doesn't interfere with the other.
    let mut map = transfer_buffer.map::<T>(device, true);
    let mem = map.mem_mut();
    mem[..data.len()].copy_from_slice(data);
    map.unmap();

    // Finally, add a command to the copy pass to upload this data to the GPU
    //
    // Note: We also set `cycle` to true here for the same reason.
    copy_pass.upload_to_gpu_buffer(
        TransferBufferLocation::new()
            .with_offset(0)
            .with_transfer_buffer(transfer_buffer),
        BufferRegion::new()
            .with_offset(0)
            .with_size(len_bytes as u32)
            .with_buffer(&buffer),
        true,
    );

    Ok(buffer)
}

pub fn create_texture(
    device: &Device,
    copy_pass: &CopyPass,
    image_data: &[u8],
    width: u32,
    height: u32,
) -> Result<Texture<'static>, Box<dyn Error>> {
    let size_bytes = width * height * 4; // Assuming RGBA8 format

    let texture = device.create_texture(
        TextureCreateInfo::new()
            .with_format(TextureFormat::R8g8b8a8Unorm)
            .with_type(TextureType::_2D)
            .with_width(width)
            .with_height(height)
            .with_layer_count_or_depth(1)
            .with_num_levels(1)
            .with_usage(TextureUsage::SAMPLER),
    )?;

    let transfer_buffer = device
        .create_transfer_buffer()
        .with_size(size_bytes)
        .with_usage(TransferBufferUsage::UPLOAD)
        .build()?;

    let mut buffer_mem = transfer_buffer.map::<u8>(device, true);
    buffer_mem.mem_mut().copy_from_slice(image_data);
    buffer_mem.unmap();

    copy_pass.upload_to_gpu_texture(
        TextureTransferInfo::new()
            .with_transfer_buffer(&transfer_buffer)
            .with_offset(0),
        TextureRegion::new()
            .with_texture(&texture)
            .with_layer(0)
            .with_width(width)
            .with_height(height)
            .with_depth(1),
        false,
    );

    Ok(texture)
}

pub fn update_texture(
    device: &Device,
    texture: &Texture<'static>,
    copy_pass: &CopyPass,
    image_data: &[u8],
    offset: (u32, u32),
    texture_size: (u32, u32),
) -> Result<(), Box<dyn Error>> {
    let width = texture_size.0;
    let height = texture_size.1;
    let x = offset.0;
    let y = offset.1;
    let size_bytes = width * height * 4; // Assuming RGBA8 format
    let transfer_buffer = device
        .create_transfer_buffer()
        .with_size(size_bytes)
        .with_usage(TransferBufferUsage::UPLOAD)
        .build()?;

    let mut buffer_mem = transfer_buffer.map::<u8>(device, true);
    buffer_mem.mem_mut().copy_from_slice(image_data);
    buffer_mem.unmap();

    copy_pass.upload_to_gpu_texture(
        TextureTransferInfo::new()
            .with_transfer_buffer(&transfer_buffer)
            .with_offset(0),
        TextureRegion::new()
            .with_texture(texture)
            .with_layer(0)
            .with_x(x)
            .with_y(y)
            .with_width(width)
            .with_height(height)
            .with_depth(1),
        false,
    );

    Ok(())
}
