use ash::vk;
use ash::Device;
use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;
use std::mem;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PushConstants {
    pub screen_dimensions: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription {
            binding: 0,
            stride: mem::size_of::<Self>() as u32,
            input_rate: vk::VertexInputRate::VERTEX,
        }
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
        [
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 0,
                format: vk::Format::R32G32_SFLOAT,
                offset: memoffset::offset_of!(Self, position) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 1,
                format: vk::Format::R32G32_SFLOAT,
                offset: memoffset::offset_of!(Self, tex_coord) as u32,
            },
            vk::VertexInputAttributeDescription {
                binding: 0,
                location: 2,
                format: vk::Format::R32G32B32A32_SFLOAT,
                offset: memoffset::offset_of!(Self, color) as u32,
            },
        ]
    }
}

pub struct GlyphInfo {
    pub texture_id: u32,
    pub width: u32,
    pub height: u32,
    pub bearing_x: i32,
    pub bearing_y: i32,
    pub advance: f32,
}

pub struct TextRenderer {
    pub font: Font,
    pub glyph_cache: HashMap<char, GlyphInfo>,
    pub font_size: f32,
    pub device: Device,
    pub graphics_pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub vertex_buffer: vk::Buffer,
    pub vertex_buffer_memory: vk::DeviceMemory,
    pub index_buffer: vk::Buffer,
    pub index_buffer_memory: vk::DeviceMemory,
    pub texture_image: vk::Image,
    pub texture_image_memory: vk::DeviceMemory,
    pub texture_image_view: vk::ImageView,
    pub texture_sampler: vk::Sampler,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub atlas_x: u32,
    pub atlas_y: u32,
    pub atlas_data: Vec<u8>,
}

impl TextRenderer {
    pub fn new(
        device: Device,
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let font_data = include_bytes!("/home/codebam/Documents/rust/vulkan-terminal/result/share/fonts/truetype/NerdFonts/FiraCode/FiraCodeNerdFont-Regular.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())?;
        let font_size = 16.0;

        let descriptor_set_layout = Self::create_descriptor_set_layout(&device)?;
        let (graphics_pipeline, pipeline_layout) = Self::create_graphics_pipeline(
            &device,
            render_pass,
            extent,
            descriptor_set_layout,
        )?;

        let (vertex_buffer, vertex_buffer_memory) = Self::create_vertex_buffer(
            &device,
            physical_device,
            instance,
        )?;

        let (index_buffer, index_buffer_memory) = Self::create_index_buffer(
            &device,
            physical_device,
            instance,
        )?;

        let atlas_width = 1024;
        let atlas_height = 1024;
        let atlas_data = vec![0; (atlas_width * atlas_height) as usize];

        let (texture_image, texture_image_memory) = Self::create_texture_image(
            &device,
            physical_device,
            instance,
            atlas_width,
            atlas_height,
        )?;

        let texture_image_view = Self::create_texture_image_view(&device, texture_image)?;
        let texture_sampler = Self::create_texture_sampler(&device)?;

        let descriptor_pool = Self::create_descriptor_pool(&device)?;
        let descriptor_sets = Self::create_descriptor_sets(
            &device,
            descriptor_pool,
            descriptor_set_layout,
            texture_image_view,
            texture_sampler,
        )?;

        let mut text_renderer = TextRenderer {
            font,
            glyph_cache: HashMap::new(),
            font_size,
            device,
            graphics_pipeline,
            pipeline_layout,
            descriptor_set_layout,
            vertex_buffer,
            vertex_buffer_memory,
            index_buffer,
            index_buffer_memory,
            texture_image,
            texture_image_memory,
            texture_image_view,
            texture_sampler,
            descriptor_pool,
            descriptor_sets,
            atlas_width,
            atlas_height,
            atlas_x: 0,
            atlas_y: 0,
            atlas_data,
        };

        text_renderer.initialize_texture(command_pool, graphics_queue, physical_device, instance)?;

        Ok(text_renderer)
    }

    fn create_descriptor_set_layout(device: &Device) -> Result<vk::DescriptorSetLayout, vk::Result> {
        let sampler_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let bindings = [sampler_layout_binding];

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);

        unsafe { device.create_descriptor_set_layout(&layout_info, None) }
    }

    fn create_graphics_pipeline(
        device: &Device,
        render_pass: vk::RenderPass,
        extent: vk::Extent2D,
        descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<(vk::Pipeline, vk::PipelineLayout), Box<dyn std::error::Error>> {
        let vert_shader_code = include_bytes!("../shaders/text.vert.spv");
        let frag_shader_code = include_bytes!("../shaders/text.frag.spv");

        let vert_shader_module = Self::create_shader_module(device, vert_shader_code)?;
        let frag_shader_module = Self::create_shader_module(device, frag_shader_code)?;

        let main_function_name = std::ffi::CString::new("main")?;

        let vert_stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_shader_module)
            .name(&main_function_name);

        let frag_stage_info = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_shader_module)
            .name(&main_function_name);

        let shader_stages = [vert_stage_info, frag_stage_info];

        let binding_description = Vertex::binding_description();
        let attribute_descriptions = Vertex::attribute_descriptions();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(std::slice::from_ref(&binding_description))
            .vertex_attribute_descriptions(&attribute_descriptions);

        let input_assembly = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let viewport = vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: extent.width as f32,
            height: extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        };

        let scissor = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent,
        };

        let viewports = [viewport];
        let scissors = [scissor];

        let viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&viewports)
            .scissors(&scissors);

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
            .depth_bias_enable(false);

        let multisampling = vk::PipelineMultisampleStateCreateInfo::default()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::default()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(true)
            .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
            .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
            .color_blend_op(vk::BlendOp::ADD)
            .src_alpha_blend_factor(vk::BlendFactor::ONE)
            .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            .alpha_blend_op(vk::BlendOp::ADD);

        let color_blending = vk::PipelineColorBlendStateCreateInfo::default()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(std::slice::from_ref(&color_blend_attachment))
            .blend_constants([0.0, 0.0, 0.0, 0.0]);

        let push_constant_range = vk::PushConstantRange::default()
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .offset(0)
            .size(std::mem::size_of::<PushConstants>() as u32);

        let set_layouts = [descriptor_set_layout];
        let push_constant_ranges = [push_constant_range];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges);

        let pipeline_layout = unsafe {
            device.create_pipeline_layout(&pipeline_layout_info, None)?
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&shader_stages)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .layout(pipeline_layout)
            .render_pass(render_pass)
            .subpass(0);

        let graphics_pipeline = unsafe {
            device
                .create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &[pipeline_info],
                    None,
                )
                .map_err(|(_, err)| err)?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_shader_module, None);
            device.destroy_shader_module(frag_shader_module, None);
        }

        Ok((graphics_pipeline, pipeline_layout))
    }

    fn create_shader_module(
        device: &Device,
        code: &[u8],
    ) -> Result<vk::ShaderModule, vk::Result> {
        let mut align_code = Vec::with_capacity(code.len());
        align_code.extend_from_slice(code);

        let (prefix, code, suffix) = unsafe { align_code.align_to::<u32>() };
        if !prefix.is_empty() || !suffix.is_empty() {
            return Err(vk::Result::ERROR_INITIALIZATION_FAILED);
        }

        let create_info = vk::ShaderModuleCreateInfo::default().code(code);

        unsafe { device.create_shader_module(&create_info, None) }
    }

    fn create_vertex_buffer(
        device: &Device,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
        let buffer_size = (mem::size_of::<Vertex>() * 1024) as vk::DeviceSize;

        let buffer_info = vk::BufferCreateInfo {
            size: buffer_size,
            usage: vk::BufferUsageFlags::VERTEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };

        let memory_type_index = Self::find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mem_properties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None)? };

        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

        Ok((buffer, buffer_memory))
    }

    fn create_index_buffer(
        device: &Device,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Result<(vk::Buffer, vk::DeviceMemory), Box<dyn std::error::Error>> {
        let buffer_size = (mem::size_of::<u16>() * 6144) as vk::DeviceSize;

        let buffer_info = vk::BufferCreateInfo {
            size: buffer_size,
            usage: vk::BufferUsageFlags::INDEX_BUFFER,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };

        let mem_requirements = unsafe { device.get_buffer_memory_requirements(buffer) };

        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };

        let memory_type_index = Self::find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mem_properties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };

        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None)? };

        unsafe { device.bind_buffer_memory(buffer, buffer_memory, 0)? };

        Ok((buffer, buffer_memory))
    }

    fn create_texture_image(
        device: &Device,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
        width: u32,
        height: u32,
    ) -> Result<(vk::Image, vk::DeviceMemory), Box<dyn std::error::Error>> {

        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            format: vk::Format::R8_UNORM,
            tiling: vk::ImageTiling::OPTIMAL,
            initial_layout: vk::ImageLayout::UNDEFINED,
            usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            samples: vk::SampleCountFlags::TYPE_1,
            ..Default::default()
        };

        let image = unsafe { device.create_image(&image_info, None)? };

        let mem_requirements = unsafe { device.get_image_memory_requirements(image) };

        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };

        let memory_type_index = Self::find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            &mem_properties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };

        let image_memory = unsafe { device.allocate_memory(&alloc_info, None)? };

        unsafe { device.bind_image_memory(image, image_memory, 0)? };

        Ok((image, image_memory))
    }

    fn create_texture_image_view(
        device: &Device,
        texture_image: vk::Image,
    ) -> Result<vk::ImageView, vk::Result> {
        let view_info = vk::ImageViewCreateInfo {
            image: texture_image,
            view_type: vk::ImageViewType::TYPE_2D,
            format: vk::Format::R8_UNORM,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            ..Default::default()
        };

        unsafe { device.create_image_view(&view_info, None) }
    }

    fn create_texture_sampler(device: &Device) -> Result<vk::Sampler, vk::Result> {
        let sampler_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::FALSE,
            max_anisotropy: 1.0,
            border_color: vk::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: vk::FALSE,
            compare_enable: vk::FALSE,
            compare_op: vk::CompareOp::ALWAYS,
            mipmap_mode: vk::SamplerMipmapMode::LINEAR,
            mip_lod_bias: 0.0,
            min_lod: 0.0,
            max_lod: 0.0,
            ..Default::default()
        };

        unsafe { device.create_sampler(&sampler_info, None) }
    }

    fn create_descriptor_pool(device: &Device) -> Result<vk::DescriptorPool, vk::Result> {
        let pool_size = vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
        };

        let pool_info = vk::DescriptorPoolCreateInfo {
            pool_size_count: 1,
            p_pool_sizes: &pool_size,
            max_sets: 1,
            ..Default::default()
        };

        unsafe { device.create_descriptor_pool(&pool_info, None) }
    }

    fn create_descriptor_sets(
        device: &Device,
        descriptor_pool: vk::DescriptorPool,
        descriptor_set_layout: vk::DescriptorSetLayout,
        texture_image_view: vk::ImageView,
        texture_sampler: vk::Sampler,
    ) -> Result<Vec<vk::DescriptorSet>, vk::Result> {
        let layouts = [descriptor_set_layout];
        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool,
            descriptor_set_count: 1,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        let image_info = vk::DescriptorImageInfo {
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            image_view: texture_image_view,
            sampler: texture_sampler,
            ..Default::default()
        };

        let descriptor_write = vk::WriteDescriptorSet {
            dst_set: descriptor_sets[0],
            dst_binding: 0,
            dst_array_element: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            p_image_info: &image_info,
            ..Default::default()
        };

        unsafe {
            device.update_descriptor_sets(&[descriptor_write], &[]);
        }

        Ok(descriptor_sets)
    }
    
    fn initialize_texture(
        &mut self,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.update_texture(command_pool, graphics_queue, physical_device, instance)?;
        Ok(())
    }
    
    fn transition_image_and_copy_data(
        device: &Device,
        image: vk::Image,
        staging_buffer: vk::Buffer,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        width: u32,
        height: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create a command buffer to transition the image layout and copy data
        let alloc_info = vk::CommandBufferAllocateInfo {
            level: vk::CommandBufferLevel::PRIMARY,
            command_pool,
            command_buffer_count: 1,
            ..Default::default()
        };
        
        let command_buffer = unsafe { device.allocate_command_buffers(&alloc_info)?[0] };
        
        let begin_info = vk::CommandBufferBeginInfo {
            flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
            ..Default::default()
        };
        
        unsafe { device.begin_command_buffer(command_buffer, &begin_info)? };
        
        // First transition: UNDEFINED -> TRANSFER_DST_OPTIMAL
        let barrier1 = vk::ImageMemoryBarrier {
            old_layout: vk::ImageLayout::UNDEFINED,
            new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            ..Default::default()
        };
        
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier1],
            );
        }
        
        // Copy data from staging buffer to image
        let region = vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            },
            image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
            image_extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            ..Default::default()
        };
        
        unsafe {
            device.cmd_copy_buffer_to_image(
                command_buffer,
                staging_buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );
        }
        
        // Second transition: TRANSFER_DST_OPTIMAL -> SHADER_READ_ONLY_OPTIMAL
        let barrier2 = vk::ImageMemoryBarrier {
            old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
            image,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: 1,
                base_array_layer: 0,
                layer_count: 1,
            },
            src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
            dst_access_mask: vk::AccessFlags::SHADER_READ,
            ..Default::default()
        };
        
        unsafe {
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier2],
            );
            
            device.end_command_buffer(command_buffer)?;
        }
        
        // Submit the command buffer
        let submit_info = vk::SubmitInfo {
            command_buffer_count: 1,
            p_command_buffers: &command_buffer,
            ..Default::default()
        };
        
        unsafe {
            device.queue_submit(graphics_queue, &[submit_info], vk::Fence::null())?;
            device.queue_wait_idle(graphics_queue)?;
            device.free_command_buffers(command_pool, &[command_buffer]);
        }
        
        Ok(())
    }

    fn update_texture(
        &self,
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let image_size = (self.atlas_width * self.atlas_height) as usize;
        
        // Create staging buffer
        let buffer_info = vk::BufferCreateInfo {
            size: image_size as vk::DeviceSize,
            usage: vk::BufferUsageFlags::TRANSFER_SRC,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };
        
        let staging_buffer = unsafe { self.device.create_buffer(&buffer_info, None)? };
        let mem_requirements = unsafe { self.device.get_buffer_memory_requirements(staging_buffer) };
        
        let mem_properties = unsafe {
            instance.get_physical_device_memory_properties(physical_device)
        };
        
        let memory_type_index = Self::find_memory_type(
            mem_requirements.memory_type_bits,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &mem_properties,
        )?;
        
        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirements.size,
            memory_type_index,
            ..Default::default()
        };
        
        let staging_buffer_memory = unsafe { self.device.allocate_memory(&alloc_info, None)? };
        unsafe { self.device.bind_buffer_memory(staging_buffer, staging_buffer_memory, 0)? };
        
        // Upload pixel data to staging buffer
        unsafe {
            let data_ptr = self.device.map_memory(
                staging_buffer_memory,
                0,
                image_size as vk::DeviceSize,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy_nonoverlapping(self.atlas_data.as_ptr(), data_ptr as *mut u8, image_size);
            self.device.unmap_memory(staging_buffer_memory);
        }
        
        // Transition image layout and copy data
        Self::transition_image_and_copy_data(&self.device, self.texture_image, staging_buffer, command_pool, graphics_queue, self.atlas_width, self.atlas_height)?;
        
        // Cleanup staging buffer
        unsafe {
            self.device.destroy_buffer(staging_buffer, None);
            self.device.free_memory(staging_buffer_memory, None);
        }
        
        Ok(())
    }

    fn find_memory_type(
        type_filter: u32,
        properties: vk::MemoryPropertyFlags,
        mem_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Result<u32, Box<dyn std::error::Error>> {
        for (i, memory_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter & (1 << i)) != 0
                && memory_type.property_flags.contains(properties)
            {
                return Ok(i as u32);
            }
        }

        Err("Failed to find suitable memory type".into())
    }

    pub fn render_text_to_buffer(
        &mut self,
        vertices: &mut Vec<Vertex>,
        indices: &mut Vec<u16>,
        text: &str,
        x: f32,
        y: f32,
        color: [f32; 4],
        command_pool: vk::CommandPool,
        graphics_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        instance: &ash::Instance,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut current_x = x;
        let mut texture_updated = false;

        for ch in text.chars() {
            if !self.glyph_cache.contains_key(&ch) {
                self.cache_glyph(ch)?;
                texture_updated = true;
            }

            if let Some(glyph_info) = self.glyph_cache.get(&ch) {
                let x_pos = current_x + glyph_info.bearing_x as f32;
                let y_pos = y - (glyph_info.height as i32 - glyph_info.bearing_y) as f32;

                let w = glyph_info.width as f32;
                let h = glyph_info.height as f32;

                let u0 = glyph_info.texture_id as f32 / self.atlas_width as f32;
                let v0 = 0.0;
                let u1 = (glyph_info.texture_id + glyph_info.width) as f32 / self.atlas_width as f32;
                let v1 = glyph_info.height as f32 / self.atlas_height as f32;

                let index_offset = vertices.len() as u16;

                vertices.extend_from_slice(&[
                    Vertex {
                        position: [x_pos, y_pos + h],
                        tex_coord: [u0, v1],
                        color,
                    },
                    Vertex {
                        position: [x_pos, y_pos],
                        tex_coord: [u0, v0],
                        color,
                    },
                    Vertex {
                        position: [x_pos + w, y_pos],
                        tex_coord: [u1, v0],
                        color,
                    },
                    Vertex {
                        position: [x_pos + w, y_pos + h],
                        tex_coord: [u1, v1],
                        color,
                    },
                ]);

                indices.extend_from_slice(&[
                    index_offset,
                    index_offset + 1,
                    index_offset + 2,
                    index_offset + 2,
                    index_offset + 3,
                    index_offset,
                ]);

                current_x += glyph_info.advance;
            }
        }

        if texture_updated {
            self.update_texture(command_pool, graphics_queue, physical_device, instance)?;
        }

        Ok(())
    }

    pub fn update_vertex_buffer(&self, vertices: &[Vertex]) -> Result<(), vk::Result> {
        let data_size = (vertices.len() * mem::size_of::<Vertex>()) as vk::DeviceSize;

        unsafe {
            let data_ptr = self.device.map_memory(
                self.vertex_buffer_memory,
                0,
                data_size,
                vk::MemoryMapFlags::empty(),
            )?;

            std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, data_ptr as *mut u8, data_size as usize);

            self.device.unmap_memory(self.vertex_buffer_memory);
        }

        Ok(())
    }

    pub fn update_index_buffer(&self, indices: &[u16]) -> Result<(), vk::Result> {
        let data_size = (indices.len() * mem::size_of::<u16>()) as vk::DeviceSize;

        unsafe {
            let data_ptr = self.device.map_memory(
                self.index_buffer_memory,
                0,
                data_size,
                vk::MemoryMapFlags::empty(),
            )?;

            std::ptr::copy_nonoverlapping(indices.as_ptr() as *const u8, data_ptr as *mut u8, data_size as usize);

            self.device.unmap_memory(self.index_buffer_memory);
        }

        Ok(())
    }

    pub fn cache_glyph(&mut self, ch: char) -> Result<(), Box<dyn std::error::Error>> {
        if !self.glyph_cache.contains_key(&ch) {
            let (metrics, bitmap) = self.font.rasterize(ch, self.font_size);

            if self.atlas_x + metrics.width as u32 > self.atlas_width {
                self.atlas_x = 0;
                self.atlas_y += self.font_size as u32;
            }

            if self.atlas_y + metrics.height as u32 > self.atlas_height {
                return Err("Font atlas is full".into());
            }

            for y in 0..metrics.height {
                for x in 0..metrics.width {
                    let index =
                        ((self.atlas_y + y as u32) * self.atlas_width + (self.atlas_x + x as u32))
                            as usize;
                    self.atlas_data[index] = bitmap[y * metrics.width + x];
                }
            }

            let glyph_info = GlyphInfo {
                texture_id: self.atlas_x,
                width: metrics.width as u32,
                height: metrics.height as u32,
                bearing_x: metrics.xmin,
                bearing_y: metrics.ymin,
                advance: metrics.advance_width,
            };

            self.glyph_cache.insert(ch, glyph_info);
            self.atlas_x += metrics.width as u32;
        }
        Ok(())
    }
}

impl Drop for TextRenderer {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.device.destroy_sampler(self.texture_sampler, None);
            self.device.destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image, None);
            self.device.free_memory(self.texture_image_memory, None);
            self.device.destroy_buffer(self.index_buffer, None);
            self.device.free_memory(self.index_buffer_memory, None);
            self.device.destroy_buffer(self.vertex_buffer, None);
            self.device.free_memory(self.vertex_buffer_memory, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_pipeline(self.graphics_pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}