mod app_error;
#[allow(dead_code)]
mod geometry;
mod queue_families;

use app_error::{AppError, AppErrorType};
use geometry::*;
use queue_families::QueueFamilyIndice;

use std::{
    collections::HashSet,
    ffi::{c_void, CStr, CString},
    path::Path,
    time::Instant,
};

#[cfg(feature = "vlayers")]
use ash::ext::debug_utils;
use ash::{
    khr::{self, surface, swapchain},
    vk, Device, Entry, Instance,
};
use colored::Colorize;
use image::io::Reader;
use raw_window_handle::{DisplayHandle, HasDisplayHandle, HasWindowHandle, WindowHandle};
use winit::{event_loop::ActiveEventLoop, window::Window};

// Mesh
const VERTICES: [Vertex; 4] = [
    Vertex::new(
        Vec2::new(-0.5, -0.5),
        Vec3::new(1.0, 0.0, 0.0),
        Vec2::new(1.0, 0.0),
    ),
    Vertex::new(
        Vec2::new(0.5, -0.5),
        Vec3::new(0.0, 1.0, 0.0),
        Vec2::new(0.0, 0.0),
    ),
    Vertex::new(
        Vec2::new(0.5, 0.5),
        Vec3::new(0.0, 0.0, 1.0),
        Vec2::new(0.0, 1.0),
    ),
    Vertex::new(
        Vec2::new(-0.5, 0.5),
        Vec3::new(1.0, 1.0, 1.0),
        Vec2::new(1.0, 1.0),
    ),
];
const INDICES: [u16; 6] = [0, 1, 2, 2, 3, 0];

const MAX_FRAMES_IN_FLIGHT: usize = 2;

const DEVICE_EXTENSIONS: &[&CStr] = &[khr::swapchain::NAME];
#[cfg(feature = "vlayers")]
const EXTENSIONS: &[&CStr] = &[debug_utils::NAME];
#[cfg(not(feature = "vlayers"))]
const EXTENSIONS: &[&CStr] = &[];

#[cfg(feature = "vlayers")]
const VALIDATION_LAYERS: &[&CStr] = unsafe {
    &[CStr::from_bytes_with_nul_unchecked(
        b"VK_LAYER_KHRONOS_validation\0",
    )]
};

#[cfg(feature = "vlayers")]
const LAYER_SEVERITY: vk::DebugUtilsMessageSeverityFlagsEXT =
    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE;

pub type AppResult<T> = Result<T, AppError>;

struct SwapChainHolder {
    swapchain_ext: swapchain::Device,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    image_format: vk::Format,
    extent: vk::Extent2D,
}

struct SwapChainDetails {
    pub capabilities: vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<vk::SurfaceFormatKHR>,
    pub present_modes: Vec<vk::PresentModeKHR>,
}

struct SurfaceHodlder {
    surface_ext: surface::Instance,
    surface: vk::SurfaceKHR,
}

struct GraphicsPipelineHolder {
    renderpass: vk::RenderPass,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
}

struct BufferHolder {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
}

impl BufferHolder {
    fn new(buffer: vk::Buffer, memory: vk::DeviceMemory) -> Self {
        Self { buffer, memory }
    }
}

struct MemoryMappedBuffer {
    buffer: vk::Buffer,
    memory: vk::DeviceMemory,
    memory_map: *const c_void,
}

impl MemoryMappedBuffer {
    fn new(buffer: vk::Buffer, memory: vk::DeviceMemory, memory_map: *const c_void) -> Self {
        Self {
            buffer,
            memory,
            memory_map,
        }
    }
}

struct ImageHolder {
    image: vk::Image,
    memory: vk::DeviceMemory,
}

impl ImageHolder {
    fn new(image: vk::Image, memory: vk::DeviceMemory) -> Self {
        Self { image, memory }
    }
}

#[cfg(feature = "vlayers")]
struct DebugMessengerHolder {
    debug_util_ext: debug_utils::Instance,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

pub struct Application {
    _entry: Entry,

    instance: Instance,
    surface: SurfaceHodlder,
    physical_device: vk::PhysicalDevice,
    device: Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: SwapChainHolder,
    pipeline: GraphicsPipelineHolder,
    swapchain_frame_buffers: Vec<vk::Framebuffer>,
    command_pool: vk::CommandPool,
    command_buffers: Vec<vk::CommandBuffer>,
    current_frame: usize,
    vertex_buffer: BufferHolder,
    index_buffer: BufferHolder,
    uniform_buffers: Vec<MemoryMappedBuffer>,
    texture_image: ImageHolder,
    texture_image_view: vk::ImageView,
    texture_sampler: vk::Sampler,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,

    image_avaible_semaphores: Vec<vk::Semaphore>,
    render_done_semaphores: Vec<vk::Semaphore>,
    in_flight_fences: Vec<vk::Fence>,

    start_time: Instant,
    resize_flag: bool,

    #[cfg(feature = "vlayers")]
    debug_messenger: DebugMessengerHolder,
}

impl Application {
    /// Creates the application and initialize the Vulkan working environment
    pub fn create(event_loop: &ActiveEventLoop, window: &Window) -> AppResult<Self> {
        let entry = unsafe {
            Entry::load().or(AppResult::Err(AppError::new(
                AppErrorType::VulkanLoadingError,
            )))?
        };

        // Getting every requested extension names as an iterator of valid CStr
        let display_handle: DisplayHandle = event_loop
            .display_handle()
            .or_else(|r| AppResult::Err(r.into()))?;

        let winit_extension_names =
            ash_window::enumerate_required_extensions(display_handle.as_raw())?;
        let extension_names = EXTENSIONS.iter().copied().chain(
            winit_extension_names
                .iter()
                .map(|&ext| unsafe { CStr::from_ptr(ext) }),
        );

        // Getting every requested validation layers names as an iterator of valid CStr
        #[cfg(feature = "vlayers")]
        let layer_names = VALIDATION_LAYERS.iter().copied();

        // Creating the VkInstance
        #[cfg(feature = "vlayers")]
        let instance = Self::create_instance(&entry, extension_names, layer_names)?;
        #[cfg(not(feature = "vlayers"))]
        let instance = Self::create_instance(&entry, extension_names)?;

        // Setting up the VkDebugUtilsMessengerEXT for the validation layers
        #[cfg(feature = "vlayers")]
        let debug_messenger = Self::setup_debug_messenger(&entry, &instance)?;

        let surface = Self::create_surface(&entry, &instance, event_loop, window)?;

        // Choosing the VkPhisicalDevice, create the VkDevice and the graphics queue
        let (physical_device, queue_family_indices) =
            Self::pick_physical_device(&instance, &surface)?;
        let (device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, physical_device, queue_family_indices)?;

        let swapchain = Self::create_swapchain(
            &instance,
            &device,
            physical_device,
            &surface,
            queue_family_indices,
        )?;

        let pipeline = Self::create_graphics_pipeline(&device, &swapchain)?;

        let swapchain_frame_buffers = Self::create_frame_buffers(&device, &pipeline, &swapchain)?;

        let command_pool = Self::create_command_pool(&device, queue_family_indices)?;

        let command_buffers =
            Self::create_command_buffers(&device, command_pool, MAX_FRAMES_IN_FLIGHT as u32)?;

        let texture_image = Self::create_texture_image(
            &instance,
            &device,
            graphics_queue,
            physical_device,
            command_pool,
            "src/texture.jpg",
        )?;

        let texture_image_view = Self::create_texture_image_view(&device, texture_image.image)?;
        let texture_sampler = Self::create_texture_sampler(&instance, &device, physical_device)?;

        let vertex_buffer = Self::create_vertex_buffer(
            &instance,
            &device,
            graphics_queue,
            physical_device,
            &VERTICES,
            command_pool,
        )?;

        let index_buffer = Self::create_index_buffer(
            &instance,
            &device,
            graphics_queue,
            physical_device,
            &INDICES,
            command_pool,
        )?;

        let uniform_buffers = Self::create_uniform_buffers(
            &instance,
            &device,
            physical_device,
            MAX_FRAMES_IN_FLIGHT,
        )?;

        let descriptor_pool = Self::create_descriptor_pool(&device, MAX_FRAMES_IN_FLIGHT as u32)?;
        let descriptor_sets = Self::create_descriptor_sets(
            &device,
            &uniform_buffers,
            texture_image_view,
            texture_sampler,
            pipeline.descriptor_set_layout,
            descriptor_pool,
            MAX_FRAMES_IN_FLIGHT as u32,
        )?;

        let (image_avaible_semaphores, render_done_semaphores, in_flight_fences) =
            Self::create_sync_objects(&device, MAX_FRAMES_IN_FLIGHT as u32)?;

        Ok(Self {
            _entry: entry,

            instance,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            pipeline,
            swapchain_frame_buffers,
            command_pool,
            command_buffers,
            current_frame: 0,
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            texture_image,
            texture_image_view,
            texture_sampler,
            descriptor_pool,
            descriptor_sets,

            image_avaible_semaphores,
            render_done_semaphores,
            in_flight_fences,

            start_time: Instant::now(),
            resize_flag: false,

            #[cfg(feature = "vlayers")]
            debug_messenger,
        })
    }

    pub fn draw_frame(&mut self) -> AppResult<()> {
        unsafe {
            self.device.wait_for_fences(
                &[self.in_flight_fences[self.current_frame]],
                true,
                std::u64::MAX,
            )?;

            let result = self.swapchain.swapchain_ext.acquire_next_image(
                self.swapchain.swapchain,
                std::u64::MAX,
                self.image_avaible_semaphores[self.current_frame],
                vk::Fence::null(),
            );

            let image_index = match result {
                Ok((v, _)) => v,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    self.recreate_swapchain()?;
                    return Ok(());
                }
                Err(res) => return AppResult::Err(res.into()),
            };

            self.device
                .reset_fences(&[self.in_flight_fences[self.current_frame]])?;

            self.device.reset_command_buffer(
                self.command_buffers[self.current_frame],
                vk::CommandBufferResetFlags::empty(),
            )?;

            self.update_uniform_buffer();

            self.record_command_buffer(image_index)?;

            let wait_semaphores = [self.image_avaible_semaphores[self.current_frame]];
            let command_buffers = [self.command_buffers[self.current_frame]];
            let signal_semaphores = [self.render_done_semaphores[self.current_frame]];
            let wait_dst_stage_mask = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
            let submit_infos = [vk::SubmitInfo {
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: &wait_semaphores as *const _,
                p_wait_dst_stage_mask: &wait_dst_stage_mask as *const _,
                command_buffer_count: command_buffers.len() as u32,
                p_command_buffers: &command_buffers as *const _,
                signal_semaphore_count: signal_semaphores.len() as u32,
                p_signal_semaphores: &signal_semaphores as *const _,
                ..Default::default()
            }];

            self.device.queue_submit(
                self.graphics_queue,
                &submit_infos,
                self.in_flight_fences[self.current_frame],
            )?;

            let wait_semaphores = signal_semaphores;
            let swapchains = [self.swapchain.swapchain];
            let image_indices = [image_index];
            let present_info = vk::PresentInfoKHR {
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: &wait_semaphores as *const _,
                swapchain_count: swapchains.len() as u32,
                p_swapchains: &swapchains as *const _,
                p_image_indices: &image_indices as *const _,
                ..Default::default()
            };

            let result = self
                .swapchain
                .swapchain_ext
                .queue_present(self.present_queue, &present_info);

            match result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) | Ok(true) | Ok(_) if self.resize_flag => {
                    self.resize_flag = false;
                    self.recreate_swapchain()?;
                    return Ok(());
                }
                Err(res) => return AppResult::Err(res.into()),
                _ => (),
            };
        }

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }

    fn record_command_buffer(&mut self, image_index: u32) -> AppResult<()> {
        let begin_info = vk::CommandBufferBeginInfo::default();

        unsafe {
            self.device
                .begin_command_buffer(self.command_buffers[self.current_frame], &begin_info)?;
        }

        let clear_color = vk::ClearValue {
            color: vk::ClearColorValue {
                float32: [0.0, 0.0, 0.0, 1.0],
            },
        };

        let offset = vk::Offset2D { x: 0, y: 0 };
        let render_area = vk::Rect2D {
            offset,
            extent: self.swapchain.extent,
        };

        let clear_values = [clear_color];
        let render_pass_info = vk::RenderPassBeginInfo {
            render_pass: self.pipeline.renderpass,
            framebuffer: self.swapchain_frame_buffers[image_index as usize],
            render_area,
            clear_value_count: clear_values.len() as u32,
            p_clear_values: &clear_values as *const _,
            ..Default::default()
        };

        let viewports = [vk::Viewport {
            x: 0.0,
            y: 0.0,
            width: self.swapchain.extent.width as f32,
            height: self.swapchain.extent.height as f32,
            min_depth: 0.0,
            max_depth: 1.0,
        }];

        let offset = vk::Offset2D { x: 0, y: 0 };
        let scissors = [vk::Rect2D {
            offset,
            extent: self.swapchain.extent,
        }];

        let command_buffer = self.command_buffers[self.current_frame];
        unsafe {
            self.device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_info,
                vk::SubpassContents::INLINE,
            );

            self.device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline,
            );

            let vertex_buffers = [self.vertex_buffer.buffer];
            let offsets = [0];
            self.device
                .cmd_bind_vertex_buffers(command_buffer, 0, &vertex_buffers, &offsets);

            self.device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.buffer,
                0,
                vk::IndexType::UINT16,
            );

            self.device.cmd_set_viewport(command_buffer, 0, &viewports);
            self.device.cmd_set_scissor(command_buffer, 0, &scissors);

            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline.pipeline_layout,
                0,
                &[self.descriptor_sets[self.current_frame]],
                &[],
            );

            self.device
                .cmd_draw_indexed(command_buffer, INDICES.len() as u32, 1, 0, 0, 0);

            self.device.cmd_end_render_pass(command_buffer);

            self.device.end_command_buffer(command_buffer)?;
        }

        Ok(())
    }

    fn update_uniform_buffer(&mut self) {
        let time = self.start_time.elapsed().as_secs_f32();

        // Rotates 90 degres every 4 seconds
        let model = Mat4::from_angle_z(cgmath::Rad(std::f32::consts::PI / 8.0) * time);

        let view = Mat4::look_at_rh(
            Point3::new(2.0, 2.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        );

        let aspect_ratio = self.swapchain.extent.width as f32 / self.swapchain.extent.height as f32;
        let proj = cgmath::perspective(cgmath::Deg(45.0), aspect_ratio, 0.1, 10.0);

        let ubo = ModelViewProj::new(model, view, proj);

        let src_ptr = &ubo as *const ModelViewProj;
        let dst_ptr = self.uniform_buffers[self.current_frame].memory_map as *mut ModelViewProj;
        unsafe { std::ptr::copy(src_ptr, dst_ptr, 1) };
    }

    pub fn request_resize(&mut self) {
        self.resize_flag = true;
    }

    pub fn recreate_swapchain(&mut self) -> AppResult<()> {
        unsafe {
            self.device.device_wait_idle()?;
        }

        self.cleanup_swapchain();

        let queue_families =
            Self::find_queue_families(&self.instance, self.physical_device, &self.surface)?;
        self.swapchain = Self::create_swapchain(
            &self.instance,
            &self.device,
            self.physical_device,
            &self.surface,
            queue_families,
        )?;

        self.swapchain_frame_buffers =
            Self::create_frame_buffers(&self.device, &self.pipeline, &self.swapchain)?;

        Ok(())
    }

    /// Creates the VkInstance with the requested extension names and validation layers name
    fn create_instance<'a, 'b>(
        entry: &Entry,
        extension_names: impl IntoIterator<Item = &'a CStr>,
        #[cfg(feature = "vlayers")] layer_names: impl IntoIterator<Item = &'b CStr>,
    ) -> AppResult<Instance> {
        // Define the vulkan application info
        let app_name = CString::new("Vulkan Tutorial").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        let app_info = vk::ApplicationInfo {
            p_application_name: app_name.as_ptr(),
            application_version: vk::make_api_version(1, 0, 0, 0),
            p_engine_name: engine_name.as_ptr(),
            engine_version: vk::make_api_version(1, 0, 0, 0),
            api_version: vk::API_VERSION_1_0,
            ..Default::default()
        };

        // Filter out the the extensions unsupported by the vulkan instance
        let avaible_extensions = unsafe { entry.enumerate_instance_extension_properties(None)? };
        let extensions: Vec<*const i8> =
            extension_names
                .into_iter()
                .filter(|&ext| {
                    avaible_extensions
                    .iter()
                    .find(|&a_ext| unsafe { CStr::from_ptr(a_ext.extension_name.as_ptr()) } == ext)
                    .or_else(|| {
                        println!("{} {:?} ", "Extension unsupported:".truecolor(255, 172, 28), ext);
                        None
                    })
                    .is_some()
                })
                .map(|ext| ext.as_ptr())
                .collect();

        // Filter out the the layers unsupported by the vulkan instance
        #[cfg(feature = "vlayers")]
        let layers: Vec<*const i8> =
            {
                let avaible_layers = unsafe { entry.enumerate_instance_layer_properties()? };

                layer_names
                    .into_iter()
                    .filter(|&lay| {
                        avaible_layers
                        .iter()
                        .find(|&a_lay| unsafe { CStr::from_ptr(a_lay.layer_name.as_ptr()) } == lay)
                        .or_else(|| {
                            println!("{} {:?} ","Layer unsupported:".truecolor(255, 172, 28), lay);
                            None
                        })
                        .is_some()
                    })
                    .map(|lay| lay.as_ptr())
                    .collect()
            };

        #[allow(unused_mut)]
        let mut create_info = vk::InstanceCreateInfo {
            p_application_info: &app_info as *const _,
            enabled_extension_count: extensions.len() as u32,
            pp_enabled_extension_names: extensions.as_ptr(),
            ..Default::default()
        };

        #[cfg(feature = "vlayers")]
        let debug_messenger_create_info = Self::debug_messenger_create_info();
        #[cfg(feature = "vlayers")]
        let debug_messenger_create_info_ptr =
            &debug_messenger_create_info as *const vk::DebugUtilsMessengerCreateInfoEXT;
        {
            create_info.p_next = debug_messenger_create_info_ptr as *const _;
            create_info.enabled_layer_count = layers.len() as u32;
            create_info.pp_enabled_layer_names = layers.as_ptr();
        }

        // Create the instance
        // Safety: The instance is the last destroyed object
        unsafe {
            entry
                .create_instance(&create_info, None)
                .or_else(|r| AppResult::Err(r.into()))
        }
    }

    fn create_surface(
        entry: &Entry,
        instance: &Instance,
        event_loop: &ActiveEventLoop,
        window: &Window,
    ) -> AppResult<SurfaceHodlder> {
        let surface_ext = surface::Instance::new(entry, instance);

        let display_handle: DisplayHandle = event_loop
            .display_handle()
            .or_else(|r| AppResult::Err(r.into()))?;

        let window_handle: WindowHandle = window
            .window_handle()
            .or_else(|r| AppResult::Err(r.into()))?;

        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                display_handle.as_raw(),
                window_handle.as_raw(),
                None,
            )?
        };

        Ok(SurfaceHodlder {
            surface_ext,
            surface,
        })
    }

    /// Chooses the first avaible physical device that suits the needs of the application
    fn pick_physical_device(
        instance: &Instance,
        surface: &SurfaceHodlder,
    ) -> AppResult<(vk::PhysicalDevice, QueueFamilyIndice)> {
        let physical_devices = unsafe { instance.enumerate_physical_devices()? };
        physical_devices
            .into_iter()
            .find_map(|device| {
                Self::is_device_suitable(instance, device, surface)
                    .ok()?
                    .map(|indices| (device, indices))
            })
            .ok_or_else(|| AppError::new(AppErrorType::NoSuitableDevice))
    }

    /// Checks if the physical device meets the application's requirements
    fn is_device_suitable(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> AppResult<Option<QueueFamilyIndice>> {
        let indices = Self::find_queue_families(instance, device, surface)?;
        if !indices.is_complete() {
            return Ok(None);
        }

        let extensions_supported = Self::check_device_extensions_support(instance, device)?;
        if !extensions_supported {
            return Ok(None);
        }

        let swapchain_details = Self::query_swapchain_support(device, surface)?;
        let swapchain_adequate =
            !swapchain_details.formats.is_empty() && !swapchain_details.present_modes.is_empty();
        if !swapchain_adequate {
            return Ok(None);
        }

        let supported_features = unsafe { instance.get_physical_device_features(device) };
        if supported_features.sampler_anisotropy == vk::FALSE {
            return Ok(None);
        }

        Ok(Some(indices))
    }

    /// Finds the needed queue families from the physical device
    fn find_queue_families(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> AppResult<QueueFamilyIndice> {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut indices = QueueFamilyIndice::default();
        for (i, family) in queue_families
            .iter()
            .enumerate()
            .map(|(i, f)| (i as u32, f))
        {
            if indices.is_complete() {
                break;
            }

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics_family = Some(i)
            }

            if unsafe {
                surface.surface_ext.get_physical_device_surface_support(
                    device,
                    i,
                    surface.surface,
                )?
            } {
                indices.present_family = Some(i)
            }
        }

        Ok(indices)
    }

    fn check_device_extensions_support(
        instance: &Instance,
        device: vk::PhysicalDevice,
    ) -> AppResult<bool> {
        let avaible_extensions = unsafe { instance.enumerate_device_extension_properties(device)? };

        let mut avaible_extensions_set = HashSet::new();
        for a_ext in avaible_extensions.into_iter() {
            avaible_extensions_set.insert(unsafe { CStr::from_ptr(a_ext.extension_name.as_ptr()) });
        }

        for &ext in DEVICE_EXTENSIONS.iter() {
            if !avaible_extensions_set.insert(ext) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn query_swapchain_support(
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> AppResult<SwapChainDetails> {
        let capabilities = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_capabilities(device, surface.surface)?
        };

        let formats = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_formats(device, surface.surface)?
        };

        let present_modes = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_present_modes(device, surface.surface)?
        };

        Ok(SwapChainDetails {
            capabilities,
            formats,
            present_modes,
        })
    }

    /// Creates the VkDevice
    fn create_logical_device(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        indices: QueueFamilyIndice,
    ) -> AppResult<(Device, vk::Queue, vk::Queue)> {
        let unique_families = indices.get_unique_families();

        let queue_priorities = [1.0f32];
        let mut queue_create_infos = Vec::with_capacity(unique_families.len());
        for &queue_family in unique_families.iter() {
            queue_create_infos.push(vk::DeviceQueueCreateInfo {
                queue_family_index: queue_family,
                queue_count: 1,
                p_queue_priorities: &queue_priorities as *const _,
                ..Default::default()
            })
        }

        let device_features = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
        let device_extensions = DEVICE_EXTENSIONS
            .iter()
            .map(|&ext| ext.as_ptr())
            .collect::<Vec<*const i8>>();

        let create_info = vk::DeviceCreateInfo {
            queue_create_info_count: queue_create_infos.len() as u32,
            p_queue_create_infos: queue_create_infos.as_ptr(),
            enabled_extension_count: device_extensions.len() as u32,
            pp_enabled_extension_names: device_extensions.as_ptr(),
            p_enabled_features: &device_features as *const _,
            ..Default::default()
        };

        // Safety: The Device is destroyed befor the parent Instance, see Application::cleanup()
        let device = unsafe { instance.create_device(physical_device, &create_info, None)? };

        let graphics_queue =
            unsafe { device.get_device_queue(indices.graphics_family.unwrap(), 0) };
        let present_queue = unsafe { device.get_device_queue(indices.present_family.unwrap(), 0) };

        Ok((device, graphics_queue, present_queue))
    }

    fn create_swapchain(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
        indices: QueueFamilyIndice,
    ) -> AppResult<SwapChainHolder> {
        let swapchain_support = Self::query_swapchain_support(physical_device, surface)?;

        let surface_format = Self::choose_swap_surface_format(&swapchain_support.formats);
        let present_mode = Self::choose_swap_present_mode(&swapchain_support.present_modes);
        let extent = Self::choose_swap_extent(swapchain_support.capabilities);

        let mut image_count = swapchain_support.capabilities.min_image_count + 1;
        if swapchain_support.capabilities.max_image_count != 0 {
            image_count = image_count.clamp(
                swapchain_support.capabilities.min_image_count,
                swapchain_support.capabilities.min_image_count,
            );
        }

        let mut create_info = vk::SwapchainCreateInfoKHR {
            surface: surface.surface,
            min_image_count: image_count,
            image_format: surface_format.format,
            image_color_space: surface_format.color_space,
            image_extent: extent,
            image_array_layers: 1,
            image_usage: vk::ImageUsageFlags::COLOR_ATTACHMENT,
            image_sharing_mode: vk::SharingMode::EXCLUSIVE,
            pre_transform: swapchain_support.capabilities.current_transform,
            composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
            present_mode,
            clipped: true.into(),
            ..Default::default()
        };

        let graphics = indices.graphics_family.unwrap();
        let present = indices.present_family.unwrap();
        let indices = [graphics, present];
        if graphics != present {
            create_info.image_sharing_mode = vk::SharingMode::CONCURRENT;
            create_info.queue_family_index_count = indices.len() as u32;
            create_info.p_queue_family_indices = &indices as *const _;
        }

        let swapchain_ext = swapchain::Device::new(instance, device);
        let swapchain = unsafe { swapchain_ext.create_swapchain(&create_info, None)? };

        let swapchain_images = unsafe { swapchain_ext.get_swapchain_images(swapchain)? };
        let swapchain_image_views =
            Self::create_image_views(device, &swapchain_images, surface_format.format)?;

        Ok(SwapChainHolder {
            swapchain_ext,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            image_format: surface_format.format,
            extent,
        })
    }

    /// Chooses the best surface format avaible for the swapchains.
    ///
    /// # Panic
    /// Panics if `avaible_format` is empty.
    fn choose_swap_surface_format(
        avaible_formats: &Vec<vk::SurfaceFormatKHR>,
    ) -> vk::SurfaceFormatKHR {
        assert!(!avaible_formats.is_empty());

        for &format in avaible_formats {
            if format.format == vk::Format::B8G8R8A8_SRGB
                && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
            {
                return format;
            }
        }

        avaible_formats[0]
    }

    fn choose_swap_present_mode(
        avaible_present_modes: &Vec<vk::PresentModeKHR>,
    ) -> vk::PresentModeKHR {
        for &present_mode in avaible_present_modes {
            if present_mode == vk::PresentModeKHR::MAILBOX {
                return present_mode;
            }
        }

        vk::PresentModeKHR::FIFO
    }

    fn choose_swap_extent(capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D {
        if capabilities.current_extent.width != std::u32::MAX {
            return capabilities.current_extent;
        }

        todo!()
    }

    fn create_image_views(
        device: &Device,
        images: &Vec<vk::Image>,
        image_format: vk::Format,
    ) -> AppResult<Vec<vk::ImageView>> {
        let mut image_views = Vec::with_capacity(images.len());

        for &image in images {
            image_views.push(Self::create_image_view(device, image, image_format)?);
        }

        Ok(image_views)
    }

    fn create_graphics_pipeline(
        device: &Device,
        swapchain: &SwapChainHolder,
    ) -> AppResult<GraphicsPipelineHolder> {
        let renderpass = Self::create_render_pass(device, swapchain)?;

        let vert_shader_u8 = include_bytes!("spirv/vertex.spv");
        let frag_shader_u8 = include_bytes!("spirv/fragment.spv");

        let vert_shader_code = Self::make_spirv_raw(vert_shader_u8);
        let frag_shader_code = Self::make_spirv_raw(frag_shader_u8);

        let vert_module = Self::create_shader_module(device, &vert_shader_code)?;
        let frag_module = Self::create_shader_module(device, &frag_shader_code)?;

        let entry_point = CString::new("main").unwrap();
        let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::VERTEX,
            module: vert_module,
            p_name: entry_point.as_ptr(),
            ..Default::default()
        };
        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo {
            stage: vk::ShaderStageFlags::FRAGMENT,
            module: frag_module,
            p_name: entry_point.as_ptr(),
            ..Default::default()
        };

        let shader_stages_infos = [vert_shader_stage_info, frag_shader_stage_info];

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo {
            dynamic_state_count: dynamic_states.len() as u32,
            p_dynamic_states: &dynamic_states as *const _,
            ..Default::default()
        };

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_binding_description_count: Vertex::BINDING_DESCRIPTIONS.len() as u32,
            p_vertex_binding_descriptions: Vertex::BINDING_DESCRIPTIONS.as_ptr(),
            vertex_attribute_description_count: Vertex::ATTRIBUTE_DESCRIPTIONS.len() as u32,
            p_vertex_attribute_descriptions: Vertex::ATTRIBUTE_DESCRIPTIONS.as_ptr(),
            ..Default::default()
        };

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            primitive_restart_enable: false.into(),
            ..Default::default()
        };

        let viewport_state = vk::PipelineViewportStateCreateInfo {
            viewport_count: 1,
            scissor_count: 1,
            ..Default::default()
        };

        let rasterizer = vk::PipelineRasterizationStateCreateInfo {
            depth_clamp_enable: false.into(),
            rasterizer_discard_enable: false.into(),
            polygon_mode: vk::PolygonMode::FILL,
            cull_mode: vk::CullModeFlags::BACK,
            front_face: vk::FrontFace::CLOCKWISE,
            depth_bias_enable: false.into(),
            depth_bias_constant_factor: 0.0,
            depth_bias_clamp: 0.0,
            depth_bias_slope_factor: 0.0,
            line_width: 1.0,
            ..Default::default()
        };

        let multisampling = vk::PipelineMultisampleStateCreateInfo {
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            sample_shading_enable: false.into(),
            min_sample_shading: 1.0,
            alpha_to_coverage_enable: false.into(),
            alpha_to_one_enable: false.into(),
            ..Default::default()
        };

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
            blend_enable: false.into(),
            color_write_mask: vk::ColorComponentFlags::RGBA,
            ..Default::default()
        };

        let color_blend_attachments = [color_blend_attachment];
        let color_blending = vk::PipelineColorBlendStateCreateInfo {
            logic_op_enable: false.into(),
            logic_op: vk::LogicOp::COPY,
            attachment_count: color_blend_attachments.len() as u32,
            p_attachments: &color_blend_attachment as *const _,
            blend_constants: [0.0; 4],
            ..Default::default()
        };

        let descriptor_set_layout = Self::create_descriptor_set_layout(device)?;
        let descriptor_set_layouts = [descriptor_set_layout];
        let push_constant_ranges = [];
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo {
            set_layout_count: descriptor_set_layouts.len() as u32,
            p_set_layouts: &descriptor_set_layouts as *const _,
            push_constant_range_count: push_constant_ranges.len() as u32,
            p_push_constant_ranges: &push_constant_ranges as *const _,
            ..Default::default()
        };

        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        let pipeline_info = vk::GraphicsPipelineCreateInfo {
            stage_count: shader_stages_infos.len() as u32,
            p_stages: shader_stages_infos.as_ptr(),
            p_vertex_input_state: &vertex_input_info as *const _,
            p_input_assembly_state: &input_assembly_info as *const _,
            p_viewport_state: &viewport_state as *const _,
            p_rasterization_state: &rasterizer as *const _,
            p_multisample_state: &multisampling as *const _,
            p_color_blend_state: &color_blending as *const _,
            p_dynamic_state: &dynamic_state_create_info as *const _,
            layout: pipeline_layout,
            render_pass: renderpass,
            subpass: 0,
            base_pipeline_index: -1,
            ..Default::default()
        };

        let pipelines_infos = [pipeline_info];
        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &pipelines_infos, None)
                .or_else(|r| AppResult::Err(r.1.into()))?[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        Ok(GraphicsPipelineHolder {
            renderpass,
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
        })
    }

    fn create_render_pass(
        device: &Device,
        swapchain: &SwapChainHolder,
    ) -> AppResult<vk::RenderPass> {
        let color_attachment = [vk::AttachmentDescription {
            format: swapchain.image_format,
            samples: vk::SampleCountFlags::TYPE_1,
            load_op: vk::AttachmentLoadOp::CLEAR,
            store_op: vk::AttachmentStoreOp::STORE,
            stencil_load_op: vk::AttachmentLoadOp::DONT_CARE,
            stencil_store_op: vk::AttachmentStoreOp::DONT_CARE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
            ..Default::default()
        }];

        let color_attachment_refs = [vk::AttachmentReference {
            attachment: 0,
            layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        }];

        let subpasses = [vk::SubpassDescription {
            pipeline_bind_point: vk::PipelineBindPoint::GRAPHICS,
            color_attachment_count: color_attachment_refs.len() as u32,
            p_color_attachments: color_attachment_refs.as_ptr(),
            ..Default::default()
        }];

        let dependencies = [vk::SubpassDependency {
            src_subpass: vk::SUBPASS_EXTERNAL,
            dst_subpass: 0,
            src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            src_access_mask: vk::AccessFlags::empty(),
            dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            ..Default::default()
        }];

        let renderpass_info = vk::RenderPassCreateInfo {
            attachment_count: color_attachment.len() as u32,
            p_attachments: color_attachment.as_ptr(),
            subpass_count: subpasses.len() as u32,
            p_subpasses: subpasses.as_ptr(),
            dependency_count: dependencies.len() as u32,
            p_dependencies: dependencies.as_ptr(),
            ..Default::default()
        };

        unsafe { Ok(device.create_render_pass(&renderpass_info, None)?) }
    }

    // Code taken from https://github.com/gfx-rs/wgpu
    fn make_spirv_raw(bytes: &[u8]) -> Vec<u32> {
        let mut words = vec![0u32; bytes.len() / std::mem::size_of::<u32>()];
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                words.as_mut_ptr() as *mut u8,
                bytes.len(),
            );
        }

        words
    }

    fn create_shader_module(device: &Device, bytes: &[u32]) -> AppResult<vk::ShaderModule> {
        let create_info = vk::ShaderModuleCreateInfo {
            code_size: bytes.len() * 4,
            p_code: bytes.as_ptr(),
            ..Default::default()
        };

        unsafe { Ok(device.create_shader_module(&create_info, None)?) }
    }

    fn create_descriptor_set_layout(device: &Device) -> AppResult<vk::DescriptorSetLayout> {
        let ubo_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        };

        let sampler_layout_binding = vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 1,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        };

        let bindings = [ubo_layout_binding, sampler_layout_binding];
        let layout_info = vk::DescriptorSetLayoutCreateInfo {
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
            ..Default::default()
        };

        unsafe { Ok(device.create_descriptor_set_layout(&layout_info, None)?) }
    }

    fn create_frame_buffers(
        device: &Device,
        pipeline: &GraphicsPipelineHolder,
        swapchain: &SwapChainHolder,
    ) -> AppResult<Vec<vk::Framebuffer>> {
        let mut frame_buffers = vec![];
        for &attachment in swapchain.swapchain_image_views.iter() {
            let attachments = [attachment];
            let frame_buffer_info = vk::FramebufferCreateInfo {
                render_pass: pipeline.renderpass,
                attachment_count: attachments.len() as u32,
                p_attachments: attachments.as_ptr(),
                width: swapchain.extent.width,
                height: swapchain.extent.height,
                layers: 1,
                ..Default::default()
            };
            frame_buffers.push(unsafe { device.create_framebuffer(&frame_buffer_info, None)? });
        }

        Ok(frame_buffers)
    }

    fn create_command_pool(
        device: &Device,
        queue_families: QueueFamilyIndice,
    ) -> AppResult<vk::CommandPool> {
        let pool_info = vk::CommandPoolCreateInfo {
            flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
            queue_family_index: queue_families.graphics_family.unwrap(),
            ..Default::default()
        };

        unsafe { Ok(device.create_command_pool(&pool_info, None)?) }
    }

    fn create_texture_image_view(device: &Device, image: vk::Image) -> AppResult<vk::ImageView> {
        Self::create_image_view(device, image, vk::Format::R8G8B8A8_SRGB)
    }

    fn create_image_view(
        device: &Device,
        image: vk::Image,
        format: vk::Format,
    ) -> AppResult<vk::ImageView> {
        let subresource_range = vk::ImageSubresourceRange {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            base_mip_level: 0,
            level_count: 1,
            base_array_layer: 0,
            layer_count: 1,
        };

        let create_info = vk::ImageViewCreateInfo {
            flags: vk::ImageViewCreateFlags::empty(),
            image,
            view_type: vk::ImageViewType::TYPE_2D,
            format,
            subresource_range,
            ..Default::default()
        };

        unsafe { Ok(device.create_image_view(&create_info, None)?) }
    }

    fn create_texture_sampler(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
    ) -> AppResult<vk::Sampler> {
        let proprieties = unsafe { instance.get_physical_device_properties(physical_device) };
        let create_info = vk::SamplerCreateInfo {
            mag_filter: vk::Filter::LINEAR,
            min_filter: vk::Filter::LINEAR,
            address_mode_u: vk::SamplerAddressMode::REPEAT,
            address_mode_v: vk::SamplerAddressMode::REPEAT,
            address_mode_w: vk::SamplerAddressMode::REPEAT,
            anisotropy_enable: vk::TRUE,
            max_anisotropy: proprieties.limits.max_sampler_anisotropy,
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

        Ok(unsafe { device.create_sampler(&create_info, None)? })
    }

    fn create_vertex_buffer(
        instance: &Instance,
        device: &Device,
        graphic_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        vertex_data: &[Vertex],
        command_pool: vk::CommandPool,
    ) -> AppResult<BufferHolder> {
        let vertex_buffer_usage =
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER;
        let vertex_buffer_mem_proprieties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        Self::create_buffer_with_data(
            instance,
            device,
            graphic_queue,
            physical_device,
            vertex_data,
            vertex_buffer_usage,
            vertex_buffer_mem_proprieties,
            command_pool,
        )
    }

    fn create_index_buffer(
        instance: &Instance,
        device: &Device,
        graphic_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        index_data: &[u16],
        command_pool: vk::CommandPool,
    ) -> AppResult<BufferHolder> {
        let index_buffer_usage =
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER;
        let index_buffer_mem_proprieties = vk::MemoryPropertyFlags::DEVICE_LOCAL;
        Self::create_buffer_with_data(
            instance,
            device,
            graphic_queue,
            physical_device,
            index_data,
            index_buffer_usage,
            index_buffer_mem_proprieties,
            command_pool,
        )
    }

    fn create_uniform_buffers(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        max_frame_in_flight: usize,
    ) -> AppResult<Vec<MemoryMappedBuffer>> {
        let buffer_size = std::mem::size_of::<ModelViewProj>() as u64;
        let buffer_usage = vk::BufferUsageFlags::UNIFORM_BUFFER;
        let buffer_mem_proprieties =
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;

        let mut uniform_buffers = Vec::new();
        for _ in 0..max_frame_in_flight {
            let buffer = Self::create_buffer(
                instance,
                device,
                physical_device,
                buffer_size,
                buffer_usage,
                buffer_mem_proprieties,
            )?;

            let buffer_memory_map = unsafe {
                device.map_memory(buffer.memory, 0, buffer_size, vk::MemoryMapFlags::empty())?
            };

            uniform_buffers.push(MemoryMappedBuffer::new(
                buffer.buffer,
                buffer.memory,
                buffer_memory_map,
            ));
        }

        Ok(uniform_buffers)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_buffer_with_data<T>(
        instance: &Instance,
        device: &Device,
        graphic_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        data: &[T],
        buffer_usage: vk::BufferUsageFlags,
        buffer_mem_proprieties: vk::MemoryPropertyFlags,
        command_pool: vk::CommandPool,
    ) -> AppResult<BufferHolder> {
        let buffer_size = std::mem::size_of_val(data) as u64;

        let staging_buffer_mem_proprieties =
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE;
        let staging_buffer = Self::create_buffer(
            instance,
            device,
            physical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            staging_buffer_mem_proprieties,
        )?;

        unsafe {
            let data_src = data.as_ptr() as *const c_void;
            let data_dst = device.map_memory(
                staging_buffer.memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy(data_src, data_dst, buffer_size as usize);
            device.unmap_memory(staging_buffer.memory);
        };

        let buffer = Self::create_buffer(
            instance,
            device,
            physical_device,
            buffer_size,
            buffer_usage,
            buffer_mem_proprieties,
        )?;

        Self::copy_buffer(
            device,
            graphic_queue,
            staging_buffer.buffer,
            buffer.buffer,
            buffer_size,
            command_pool,
        )?;

        unsafe {
            device.destroy_buffer(staging_buffer.buffer, None);
            device.free_memory(staging_buffer.memory, None);
        }

        Ok(buffer)
    }

    fn create_buffer(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mem_proprieties: vk::MemoryPropertyFlags,
    ) -> AppResult<BufferHolder> {
        let buffer_info = vk::BufferCreateInfo {
            size,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe { device.create_buffer(&buffer_info, None)? };

        let mem_requirement = unsafe { device.get_buffer_memory_requirements(buffer) };
        let mem_type_index = Self::find_memory_type(
            instance,
            physical_device,
            mem_requirement.memory_type_bits,
            mem_proprieties,
        )?;

        let alloc_info = vk::MemoryAllocateInfo {
            allocation_size: mem_requirement.size,
            memory_type_index: mem_type_index,
            ..Default::default()
        };
        let buffer_memory = unsafe { device.allocate_memory(&alloc_info, None)? };
        unsafe {
            device.bind_buffer_memory(buffer, buffer_memory, 0)?;
        }

        Ok(BufferHolder::new(buffer, buffer_memory))
    }

    fn copy_buffer(
        device: &Device,
        queue: vk::Queue,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        size: vk::DeviceSize,
        command_pool: vk::CommandPool,
    ) -> AppResult<()> {
        let command_buffer = Self::begin_singe_time_command(device, command_pool)?;

        unsafe {
            let copy_regions = [vk::BufferCopy {
                src_offset: 0,
                dst_offset: 0,
                size,
            }];
            device.cmd_copy_buffer(command_buffer, src_buffer, dst_buffer, &copy_regions);
        }

        Self::end_single_time_command(device, queue, command_pool, command_buffer)?;

        Ok(())
    }

    fn find_memory_type(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        mem_type_filter: u32,
        proprieties: vk::MemoryPropertyFlags,
    ) -> AppResult<u32> {
        let mem_proprieties =
            unsafe { instance.get_physical_device_memory_properties(physical_device) };
        for (i, mem_type) in mem_proprieties.memory_types.iter().enumerate() {
            if mem_type_filter & (1 << i) != 0 && mem_type.property_flags.contains(proprieties) {
                return Ok(i as u32);
            }
        }

        Err(AppError::new(AppErrorType::NoSuitableMemType))
    }

    fn create_texture_image<P: AsRef<Path>>(
        instance: &Instance,
        device: &Device,
        graphic_queue: vk::Queue,
        physical_device: vk::PhysicalDevice,
        command_pool: vk::CommandPool,
        texture_path: P,
    ) -> AppResult<ImageHolder> {
        let img = Reader::open(texture_path)?.decode()?.into_rgba8();
        let width = img.width();
        let height = img.height();
        let buffer_size = (width * height * 4) as u64;

        let staging_buffer = Self::create_buffer(
            instance,
            device,
            physical_device,
            buffer_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;
        unsafe {
            let buffer_memory_ptr = device.map_memory(
                staging_buffer.memory,
                0,
                buffer_size,
                vk::MemoryMapFlags::empty(),
            )?;
            std::ptr::copy(
                img.as_ptr(),
                buffer_memory_ptr as *mut _,
                buffer_size as usize,
            );
            device.unmap_memory(staging_buffer.memory)
        }

        let image_format = vk::Format::R8G8B8A8_SRGB;
        let texture_image = Self::create_image(
            instance,
            device,
            physical_device,
            width,
            height,
            image_format,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        Self::transition_image_layout(
            device,
            graphic_queue,
            command_pool,
            texture_image.image,
            image_format,
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        )?;
        Self::copy_buffer_to_image(
            device,
            graphic_queue,
            command_pool,
            staging_buffer.buffer,
            texture_image.image,
            width,
            height,
        )?;
        Self::transition_image_layout(
            device,
            graphic_queue,
            command_pool,
            texture_image.image,
            image_format,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        )?;

        unsafe {
            device.destroy_buffer(staging_buffer.buffer, None);
            device.free_memory(staging_buffer.memory, None);
        }

        Ok(texture_image)
    }

    #[allow(clippy::too_many_arguments)]
    fn create_image(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        proprieties: vk::MemoryPropertyFlags,
    ) -> AppResult<ImageHolder> {
        let image_info = vk::ImageCreateInfo {
            image_type: vk::ImageType::TYPE_2D,
            format,
            extent: vk::Extent3D {
                width,
                height,
                depth: 1,
            },
            mip_levels: 1,
            array_layers: 1,
            samples: vk::SampleCountFlags::TYPE_1,
            tiling,
            usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            initial_layout: vk::ImageLayout::UNDEFINED,
            ..Default::default()
        };
        unsafe {
            let image = device.create_image(&image_info, None)?;
            let mem_requirement = device.get_image_memory_requirements(image);
            let memory_type = Self::find_memory_type(
                instance,
                physical_device,
                mem_requirement.memory_type_bits,
                proprieties,
            )?;

            let alloc_info = vk::MemoryAllocateInfo {
                allocation_size: mem_requirement.size,
                memory_type_index: memory_type,
                ..Default::default()
            };

            let image_memory = device.allocate_memory(&alloc_info, None)?;
            device.bind_image_memory(image, image_memory, 0)?;

            Ok(ImageHolder::new(image, image_memory))
        }
    }

    fn transition_image_layout(
        device: &Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        image: vk::Image,
        _format: vk::Format,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> AppResult<()> {
        let mut src_access_mask = vk::AccessFlags::empty();
        let mut dst_access_mask = vk::AccessFlags::empty();
        let mut src_stage = vk::PipelineStageFlags::empty();
        let mut dst_stage = vk::PipelineStageFlags::empty();
        match (old_layout, new_layout) {
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => {
                dst_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                src_stage = vk::PipelineStageFlags::TOP_OF_PIPE;
                dst_stage = vk::PipelineStageFlags::TRANSFER;
            }
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => {
                src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
                dst_access_mask = vk::AccessFlags::SHADER_READ;

                src_stage = vk::PipelineStageFlags::TRANSFER;
                dst_stage = vk::PipelineStageFlags::FRAGMENT_SHADER;
            }
            _ => (),
        }

        let barriers = [vk::ImageMemoryBarrier {
            src_access_mask,
            dst_access_mask,
            old_layout,
            new_layout,
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
            ..Default::default()
        }];

        unsafe {
            let command_buffer = Self::begin_singe_time_command(device, command_pool)?;
            device.cmd_pipeline_barrier(
                command_buffer,
                src_stage,
                dst_stage,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            );
            Self::end_single_time_command(device, queue, command_pool, command_buffer)?;
        }

        Ok(())
    }

    fn copy_buffer_to_image(
        device: &Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        buffer: vk::Buffer,
        image: vk::Image,
        width: u32,
        height: u32,
    ) -> AppResult<()> {
        let regions = [vk::BufferImageCopy {
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
        }];
        unsafe {
            let command_buffer = Self::begin_singe_time_command(device, command_pool)?;
            device.cmd_copy_buffer_to_image(
                command_buffer,
                buffer,
                image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );
            Self::end_single_time_command(device, queue, command_pool, command_buffer)?;
        }

        Ok(())
    }

    fn create_descriptor_pool(
        device: &Device,
        max_frame_in_flight: u32,
    ) -> AppResult<vk::DescriptorPool> {
        let pool_sizes = [
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: max_frame_in_flight,
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                descriptor_count: max_frame_in_flight,
            },
        ];

        let pool_info = vk::DescriptorPoolCreateInfo {
            max_sets: max_frame_in_flight,
            pool_size_count: 2,
            p_pool_sizes: pool_sizes.as_ptr(),
            ..Default::default()
        };

        unsafe { Ok(device.create_descriptor_pool(&pool_info, None)?) }
    }

    fn create_descriptor_sets(
        device: &Device,
        uniform_buffers: &[MemoryMappedBuffer],
        texture_view: vk::ImageView,
        texture_sampler: vk::Sampler,
        descriptor_set_layout: vk::DescriptorSetLayout,
        descriptor_pool: vk::DescriptorPool,
        max_frame_in_flight: u32,
    ) -> AppResult<Vec<vk::DescriptorSet>> {
        let layouts = vec![descriptor_set_layout; max_frame_in_flight as usize];

        let alloc_info = vk::DescriptorSetAllocateInfo {
            descriptor_pool,
            descriptor_set_count: max_frame_in_flight,
            p_set_layouts: layouts.as_ptr(),
            ..Default::default()
        };

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };
        let mut buffer_infos = vec![];
        let mut image_infos = vec![];
        let mut descriptor_writes = vec![];
        for (i, &desc_set) in descriptor_sets.iter().enumerate() {
            buffer_infos.push(vk::DescriptorBufferInfo {
                buffer: uniform_buffers[i].buffer,
                offset: 0,
                range: std::mem::size_of::<ModelViewProj>() as u64,
            });

            image_infos.push(vk::DescriptorImageInfo {
                image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                image_view: texture_view,
                sampler: texture_sampler,
            });

            descriptor_writes.push(vk::WriteDescriptorSet {
                dst_set: desc_set,
                dst_binding: 0,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                p_buffer_info: &buffer_infos[i] as *const _,
                ..Default::default()
            });
            descriptor_writes.push(vk::WriteDescriptorSet {
                dst_set: desc_set,
                dst_binding: 1,
                dst_array_element: 0,
                descriptor_count: 1,
                descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                p_image_info: &image_infos[i] as *const _,
                ..Default::default()
            });
        }

        unsafe { device.update_descriptor_sets(&descriptor_writes, &[]) };
        Ok(descriptor_sets)
    }

    fn create_command_buffers(
        device: &Device,
        command_pool: vk::CommandPool,
        max_frame_in_flight: u32,
    ) -> AppResult<Vec<vk::CommandBuffer>> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: max_frame_in_flight,
            ..Default::default()
        };
        unsafe { Ok(device.allocate_command_buffers(&alloc_info)?) }
    }

    fn begin_singe_time_command(
        device: &Device,
        command_pool: vk::CommandPool,
    ) -> AppResult<vk::CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo {
            command_pool,
            level: vk::CommandBufferLevel::PRIMARY,
            command_buffer_count: 1,
            ..Default::default()
        };

        unsafe {
            let command_buffer = device.allocate_command_buffers(&alloc_info)?[0];
            let begin_info = vk::CommandBufferBeginInfo {
                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                ..Default::default()
            };
            device.begin_command_buffer(command_buffer, &begin_info)?;
            Ok(command_buffer)
        }
    }

    fn end_single_time_command(
        device: &Device,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
        command_buffer: vk::CommandBuffer,
    ) -> AppResult<()> {
        unsafe {
            device.end_command_buffer(command_buffer)?;

            let submit_infos = [vk::SubmitInfo {
                command_buffer_count: 1,
                p_command_buffers: &command_buffer as *const _,
                ..Default::default()
            }];

            device.queue_submit(queue, &submit_infos, vk::Fence::null())?;
            device.queue_wait_idle(queue)?;
            let command_buffers = [command_buffer];
            device.free_command_buffers(command_pool, &command_buffers);
        }
        Ok(())
    }

    fn create_sync_objects(
        device: &Device,
        max_frame_in_flight: u32,
    ) -> AppResult<(Vec<vk::Semaphore>, Vec<vk::Semaphore>, Vec<vk::Fence>)> {
        let semaphore_info = vk::SemaphoreCreateInfo::default();
        let fence_info = vk::FenceCreateInfo {
            flags: vk::FenceCreateFlags::SIGNALED,
            ..Default::default()
        };
        let mut image_avaible_semaphores = vec![];
        let mut render_done_semaphores = vec![];
        let mut in_flight_fences = vec![];

        for _ in 0..max_frame_in_flight {
            unsafe {
                image_avaible_semaphores.push(device.create_semaphore(&semaphore_info, None)?);

                render_done_semaphores.push(device.create_semaphore(&semaphore_info, None)?);

                in_flight_fences.push(device.create_fence(&fence_info, None)?)
            }
        }

        Ok((
            image_avaible_semaphores,
            render_done_semaphores,
            in_flight_fences,
        ))
    }

    /// Sets up the debug messenger for the validation layers
    #[cfg(feature = "vlayers")]
    fn setup_debug_messenger(
        entry: &Entry,
        instance: &Instance,
    ) -> AppResult<DebugMessengerHolder> {
        let debug_util_ext = debug_utils::Instance::new(entry, instance);

        let create_info = Self::debug_messenger_create_info();

        let debug_messenger =
            unsafe { debug_util_ext.create_debug_utils_messenger(&create_info, None)? };

        Ok(DebugMessengerHolder {
            debug_util_ext,
            debug_messenger,
        })
    }

    /// Creates the VkDebugUtilsMessengerCreateInfoEXT for the debug messenger
    #[cfg(feature = "vlayers")]
    fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT<'static> {
        vk::DebugUtilsMessengerCreateInfoEXT {
            message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            message_type: vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            pfn_user_callback: Some(Self::debug_callback),
            ..Default::default()
        }
    }

    /// Is called for every validation layers event
    #[cfg(feature = "vlayers")]
    extern "system" fn debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        _message_types: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _p_user_data: *mut std::ffi::c_void,
    ) -> vk::Bool32 {
        if message_severity >= LAYER_SEVERITY {
            let message = unsafe { CStr::from_ptr((*p_callback_data).p_message) };
            eprintln!(
                "{} {:?}",
                "Validation layer:".truecolor(255, 172, 28),
                message
            );
        }

        vk::FALSE
    }

    unsafe fn destroy_buffer(&self, buffer: &BufferHolder) {
        self.device.destroy_buffer(buffer.buffer, None);
        self.device.free_memory(buffer.memory, None);
    }

    unsafe fn destroy_memory_mapped_buffer(&self, buffer: &MemoryMappedBuffer) {
        self.device.destroy_buffer(buffer.buffer, None);
        self.device.free_memory(buffer.memory, None);
    }

    fn cleanup_swapchain(&self) {
        unsafe {
            for (i, _) in self.swapchain_frame_buffers.iter().enumerate() {
                self.device
                    .destroy_framebuffer(self.swapchain_frame_buffers[i], None);
            }

            for &image_view in self.swapchain.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None)
            }

            self.swapchain
                .swapchain_ext
                .destroy_swapchain(self.swapchain.swapchain, None);
        }
    }

    /// Destroys the Vulkan objects
    pub fn cleanup(&self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.cleanup_swapchain();

            self.destroy_buffer(&self.vertex_buffer);
            self.destroy_buffer(&self.index_buffer);

            self.device.destroy_sampler(self.texture_sampler, None);
            self.device
                .destroy_image_view(self.texture_image_view, None);
            self.device.destroy_image(self.texture_image.image, None);
            self.device.free_memory(self.texture_image.memory, None);

            for buffer in &self.uniform_buffers {
                self.destroy_memory_mapped_buffer(buffer);
            }

            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.pipeline.descriptor_set_layout, None);

            self.device.destroy_pipeline(self.pipeline.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline.pipeline_layout, None);

            self.device
                .destroy_render_pass(self.pipeline.renderpass, None);

            for i in 0..MAX_FRAMES_IN_FLIGHT {
                self.device
                    .destroy_semaphore(self.image_avaible_semaphores[i], None);
                self.device
                    .destroy_semaphore(self.render_done_semaphores[i], None);
                self.device.destroy_fence(self.in_flight_fences[i], None);
            }

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_device(None);

            #[cfg(feature = "vlayers")]
            self.debug_messenger
                .debug_util_ext
                .destroy_debug_utils_messenger(self.debug_messenger.debug_messenger, None);

            self.surface
                .surface_ext
                .destroy_surface(self.surface.surface, None);

            self.instance.destroy_instance(None);
        };
    }
}
