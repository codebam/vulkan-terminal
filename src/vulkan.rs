use ash::vk;
use ash::{Device, Entry, Instance};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ffi::CString;
use winit::window::Window;

pub struct VulkanContext {
    pub entry: Entry,
    pub instance: Instance,
    pub surface: vk::SurfaceKHR,
    pub surface_loader: ash::khr::surface::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: Device,
    pub graphics_queue: vk::Queue,
    pub present_queue: vk::Queue,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_image_views: Vec<vk::ImageView>,
    pub swapchain_format: vk::Format,
    pub swapchain_extent: vk::Extent2D,
    pub render_pass: vk::RenderPass,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub command_pool: vk::CommandPool,
    pub command_buffers: Vec<vk::CommandBuffer>,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finished_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<Option<vk::Fence>>,
    pub current_frame: usize,
}

const MAX_FRAMES_IN_FLIGHT: usize = 2;

impl VulkanContext {
    pub fn new(window: &Window) -> Result<Self, Box<dyn std::error::Error>> {
        let entry = unsafe { Entry::load()? };

        let app_name = CString::new("Vulkan Terminal")?;
        let engine_name = CString::new("No Engine")?;

        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(0, 1, 0, 0),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(0, 1, 0, 0),
            api_version: vk::API_VERSION_1_0,
            ..Default::default()
        };

        let layer_names = vec![CString::new("VK_LAYER_KHRONOS_validation")?];
        let layers_names_raw: Vec<*const i8> = layer_names
            .iter()
            .map(|raw_name| raw_name.as_ptr())
            .collect();

        let extension_names =
            ash_window::enumerate_required_extensions(window.display_handle()?.as_raw())?;

        let create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info,
            enabled_layer_count: layers_names_raw.len() as u32,
            pp_enabled_layer_names: layers_names_raw.as_ptr(),
            enabled_extension_count: extension_names.len() as u32,
            pp_enabled_extension_names: extension_names.as_ptr(),
            ..Default::default()
        };

        let instance = unsafe { entry.create_instance(&create_info, None)? };

        let surface = unsafe {
            ash_window::create_surface(
                &entry,
                &instance,
                window.display_handle()?.as_raw(),
                window.window_handle()?.as_raw(),
                None,
            )?
        };

        let surface_loader = ash::khr::surface::Instance::new(&entry, &instance);

        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        let physical_device = physical_devices[0];

        let queue_family_properties =
            unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

        let graphics_queue_family_index = queue_family_properties
            .iter()
            .enumerate()
            .position(|(index, queue_family)| {
                queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    && unsafe {
                        surface_loader
                            .get_physical_device_surface_support(
                                physical_device,
                                index as u32,
                                surface,
                            )
                            .unwrap_or(false)
                    }
            })
            .expect("Could not find suitable queue family")
            as u32;

        let queue_priorities = [1.0];
        let queue_info = vk::DeviceQueueCreateInfo {
            queue_family_index: graphics_queue_family_index,
            queue_count: queue_priorities.len() as u32,
            p_queue_priorities: queue_priorities.as_ptr(),
            ..Default::default()
        };

        let device_extension_names_raw = [ash::khr::swapchain::NAME.as_ptr()];
        let features = vk::PhysicalDeviceFeatures::default();

        let device_create_info = vk::DeviceCreateInfo {
            queue_create_info_count: 1,
            p_queue_create_infos: &queue_info,
            enabled_extension_count: device_extension_names_raw.len() as u32,
            pp_enabled_extension_names: device_extension_names_raw.as_ptr(),
            p_enabled_features: &features,
            ..Default::default()
        };

        let device = unsafe { instance.create_device(physical_device, &device_create_info, None)? };

        let graphics_queue = unsafe { device.get_device_queue(graphics_queue_family_index, 0) };
        let present_queue = graphics_queue;

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);

        let surface_capabilities = unsafe {
            surface_loader.get_physical_device_surface_capabilities(physical_device, surface)?
        };

        let surface_formats = unsafe {
            surface_loader.get_physical_device_surface_formats(physical_device, surface)?
        };

        let surface_format = surface_formats
            .iter()
            .find(|format| {
                format.format == vk::Format::B8G8R8A8_SRGB
                    && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or(&surface_formats[0]);

        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(physical_device, surface)?
        };

        let present_mode = present_modes
            .iter()
            .cloned()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);

        let extent = if surface_capabilities.current_extent.width != u32::MAX {
            surface_capabilities.current_extent
        } else {
            let window_size = window.inner_size();
            vk::Extent2D {
                width: window_size.width.clamp(
                    surface_capabilities.min_image_extent.width,
                    surface_capabilities.max_image_extent.width,
                ),
                height: window_size.height.clamp(
                    surface_capabilities.min_image_extent.height,
                    surface_capabilities.max_image_extent.height,
                ),
            }
        };

        let image_count = surface_capabilities.min_image_count + 1;
        let image_count = if surface_capabilities.max_image_count > 0 {
            image_count.min(surface_capabilities.max_image_count)
        } else {
            image_count
        };

        let swapchain_create_info = vk::SwapchainCreateInfoKHR {
            surface,
            min_image_count: image_count,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: surface_capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: vk::TRUE,
            ..Default::default()
        };

        let swapchain = unsafe { swapchain_loader.create_swapchain(&swapchain_create_info, None)? };

        let swapchain_images = unsafe { swapchain_loader.get_swapchain_images(swapchain)? };

        let swapchain_image_views: Vec<vk::ImageView> = swapchain_images
            .iter()
            .map(|&image| {
                let create_info = vk::ImageViewCreateInfo {
                    image,
                    view_type: vk::ImageViewType::TYPE_2D,
                    format: surface_format.format,
                    subresource_range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    },
                    ..Default::default()
                };

                unsafe { device.create_image_view(&create_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let color_attachment = vk::AttachmentDescription {
            format: surface_format.format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        };

        let color_attachment_ref = vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        };

        let subpass = vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: 1,
            p_color_attachments: &color_attachment_ref,
            ..Default::default()
        };

        let dependency = vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        };

        let render_pass_info = vk::RenderPassCreateInfo {
            attachment_count: 1,
            p_attachments: &color_attachment,
            subpass_count: 1,
            p_subpasses: &subpass,
            dependency_count: 1,
            p_dependencies: &dependency,
            ..Default::default()
        };

        let render_pass = unsafe { device.create_render_pass(&render_pass_info, None)? };

        let framebuffers: Vec<vk::Framebuffer> = swapchain_image_views
            .iter()
            .map(|&image_view| {
                let attachments = [image_view];
                let framebuffer_info = vk::FramebufferCreateInfo {
                    render_pass,
                    attachment_count: attachments.len() as u32,
                    p_attachments: attachments.as_ptr(),
                    width: extent.width,
                    height: extent.height,
                    layers: 1,
                    ..Default::default()
                };

                unsafe { device.create_framebuffer(&framebuffer_info, None) }
            })
            .collect::<Result<Vec<_>, _>>()?;

        let command_pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: graphics_queue_family_index,
            ..Default::default()
        };

        let command_pool = unsafe { device.create_command_pool(&command_pool_info, None)? };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: MAX_FRAMES_IN_FLIGHT as u32,
            ..Default::default()
        };

        let command_buffers =
            unsafe { device.allocate_command_buffers(&command_buffer_allocate_info)? };

        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };

        let mut image_available_semaphores = Vec::new();
        let mut render_finished_semaphores = Vec::new();
        let mut in_flight_fences = Vec::new();
        let images_in_flight: Vec<Option<vk::Fence>> = vec![None; swapchain_images.len()];

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores
                .push(unsafe { device.create_semaphore(&semaphore_info, None)? });
            render_finished_semaphores
                .push(unsafe { device.create_semaphore(&semaphore_info, None)? });
            in_flight_fences.push(unsafe { device.create_fence(&fence_info, None)? });
        }

        Ok(VulkanContext {
            entry,
            instance,
            surface,
            surface_loader,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            swapchain_loader,
            swapchain_images,
            swapchain_image_views,
            swapchain_format: surface_format.format,
            swapchain_extent: extent,
            render_pass,
            framebuffers,
            command_pool,
            command_buffers,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            current_frame: 0,
        })
    }

    pub fn draw_frame<F>(
        &mut self,
        mut record_commands: F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(vk::CommandBuffer) -> Result<(), Box<dyn std::error::Error>>,
    {
        unsafe {
            let fence = self.in_flight_fences[self.current_frame];
            self.device.wait_for_fences(&[fence], true, u64::MAX)?;

            let result = self.swapchain_loader.acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.image_available_semaphores[self.current_frame],
                vk::Fence::null(),
            );

            let image_index = match result {
                Ok((image_index, _)) => image_index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    // Handle swapchain recreation here if needed
                    return Ok(());
                }
                Err(e) => return Err(e.into()),
            };

            if let Some(image_in_flight_fence) = self.images_in_flight[image_index as usize] {
                self.device
                    .wait_for_fences(&[image_in_flight_fence], true, u64::MAX)?;
            }

            self.images_in_flight[image_index as usize] = Some(fence);

            self.device.reset_command_buffer(
                self.command_buffers[self.current_frame],
                vk::CommandBufferResetFlags::empty(),
            )?;

            self.record_command_buffer(
                self.command_buffers[self.current_frame],
                image_index as usize,
                &mut record_commands,
            )?;

            let wait_semaphores = [self.image_available_semaphores[self.current_frame]];
            let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];
            let command_buffers = [self.command_buffers[self.current_frame]];

            let submit_info = vk::SubmitInfo::default()
                .wait_semaphores(&wait_semaphores)
                .wait_dst_stage_mask(&wait_stages)
                .command_buffers(&command_buffers)
                .signal_semaphores(&signal_semaphores);

            self.device.reset_fences(&[fence])?;

            self.device
                .queue_submit(self.graphics_queue, &[submit_info], fence)?;

            let swapchains = [self.swapchain];
            let image_indices = [image_index];
            let present_wait_semaphores = [self.render_finished_semaphores[self.current_frame]];

            let present_info = vk::PresentInfoKHR::default()
                .wait_semaphores(&present_wait_semaphores)
                .swapchains(&swapchains)
                .image_indices(&image_indices);

            let result = self
                .swapchain_loader
                .queue_present(self.present_queue, &present_info);

            match result {
                Ok(true) | Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    // Handle swapchain recreation here if needed
                }
                Err(e) => return Err(e.into()),
                _ => {}
            }

            self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
        }

        Ok(())
    }

    fn record_command_buffer<F>(
        &self,
        command_buffer: vk::CommandBuffer,
        image_index: usize,
        record_commands: &mut F,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnMut(vk::CommandBuffer) -> Result<(), Box<dyn std::error::Error>>,
    {
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::default();

            self.device
                .begin_command_buffer(command_buffer, &begin_info)?;

            let clear_values = [vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.2, 0.3, 0.8, 1.0], // Blue color instead of black
                },
            }];

            let render_pass_info = vk::RenderPassBeginInfo::default()
                .render_pass(self.render_pass)
                .framebuffer(self.framebuffers[image_index])
                .render_area(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: self.swapchain_extent,
                })
                .clear_values(&clear_values);

            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            record_commands(command_buffer)?;

            self.device.cmd_end_render_pass(command_buffer);

            self.device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }
}

impl Drop for VulkanContext {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            for &semaphore in &self.image_available_semaphores {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &semaphore in &self.render_finished_semaphores {
                self.device.destroy_semaphore(semaphore, None);
            }
            for &fence in &self.in_flight_fences {
                self.device.destroy_fence(fence, None);
            }

            self.device.destroy_command_pool(self.command_pool, None);

            for &framebuffer in &self.framebuffers {
                self.device.destroy_framebuffer(framebuffer, None);
            }

            self.device.destroy_render_pass(self.render_pass, None);

            for &image_view in &self.swapchain_image_views {
                self.device.destroy_image_view(image_view, None);
            }

            self.swapchain_loader
                .destroy_swapchain(self.swapchain, None);
            self.device.destroy_device(None);
            self.surface_loader.destroy_surface(self.surface, None);
            self.instance.destroy_instance(None);
        }
    }
}
