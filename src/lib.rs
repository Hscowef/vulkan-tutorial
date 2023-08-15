mod queue_families;
mod swapchain_details;

use crate::queue_families::QueueFamilyIndice;

use std::{
    collections::HashSet,
    ffi::{CStr, CString},
};

#[cfg(debug_assertions)]
use ash::extensions::ext;
use ash::{extensions::khr, vk, Device, Entry, Instance};
use colored::Colorize;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use swapchain_details::SwapChainDetails;
// use swapchain_details::SwapChainDetails;
use winit::{event_loop::EventLoop, window::Window};

#[cfg(debug_assertions)]
const EXTENSIONS: &[&CStr] = &[ext::DebugUtils::name()];
#[cfg(not(debug_assertions))]
const EXTENSIONS: &[&CStr] = &[];

const DEVICE_EXTENSIONS: &[&CStr] = &[khr::Swapchain::name()];

#[cfg(debug_assertions)]
const VALIDATION_LAYERS: &[&CStr] = unsafe {
    &[CStr::from_bytes_with_nul_unchecked(
        b"VK_LAYER_KHRONOS_validation\0",
    )]
};
#[cfg(debug_assertions)]
const LAYER_SEVERITY: vk::DebugUtilsMessageSeverityFlagsEXT =
    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE;

#[allow(dead_code)]
struct SwapChainHolder {
    swapchain_ext: khr::Swapchain,
    swapchain: vk::SwapchainKHR,
    swapchain_images: Vec<vk::Image>,
    swapchain_image_views: Vec<vk::ImageView>,
    image_format: vk::Format,
    extent: vk::Extent2D,
}

struct SurfaceHodlder {
    surface_ext: khr::Surface,
    surface: vk::SurfaceKHR,
}

struct GraphicsPipelineHolder {
    renderpass: vk::RenderPass,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
}

#[cfg(debug_assertions)]
struct DebugMessengerHolder {
    debug_util_ext: ext::DebugUtils,
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

#[allow(dead_code)]
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

    #[cfg(debug_assertions)]
    debug_messenger: DebugMessengerHolder,
}

impl Application {
    /// Creates the application and initialize the Vulkan working environment
    pub fn create<T>(event_loop: &EventLoop<T>, window: &Window) -> Self {
        let entry = unsafe { Entry::load().unwrap() };

        // Getting every requested extension names as an iterator of valid CStr
        let winit_extension_names =
            ash_window::enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();
        let extension_names = EXTENSIONS.iter().copied().chain(
            winit_extension_names
                .iter()
                .map(|&ext| unsafe { CStr::from_ptr(ext) }),
        );

        // Getting every requested validation layers names as an iterator of valid CStr
        #[cfg(debug_assertions)]
        let layer_names = VALIDATION_LAYERS.iter().copied();

        // Creating the VkInstance
        #[cfg(debug_assertions)]
        let instance = { Self::create_instance(&entry, extension_names, layer_names) };
        #[cfg(not(debug_assertions))]
        let instance = { Self::create_instance(&entry, extension_names) };

        // Setting up the VkDebugUtilsMessengerEXT for the validation layers
        #[cfg(debug_assertions)]
        let debug_messenger = Self::setup_debug_messenger(&entry, &instance);

        let surface = Self::create_surface(&entry, &instance, event_loop, window);

        // Choosing the VkPhisicalDevice, create the VkDevice and the graphics queue
        let (physical_device, queue_family_indices) =
            Self::pick_physical_device(&instance, &surface);
        let (device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, physical_device, queue_family_indices);

        let swapchain = Self::create_swapchain(
            &instance,
            &device,
            physical_device,
            &surface,
            queue_family_indices,
        );

        let pipeline = Self::create_graphics_pipeline(&device, &swapchain);

        Self {
            _entry: entry,

            instance,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,
            pipeline,

            #[cfg(debug_assertions)]
            debug_messenger,
        }
    }

    /// Creates the VkInstance with the requested extension names and validation layers name
    fn create_instance<'a, 'b>(
        entry: &Entry,
        extension_names: impl IntoIterator<Item = &'a CStr>,
        #[cfg(debug_assertions)] layer_names: impl IntoIterator<Item = &'b CStr>,
    ) -> Instance {
        // Define the vulkan application info
        let app_name = CString::new("Vulkan Tutorial").unwrap();
        let engine_name = CString::new("No Engine").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&app_name)
            .application_version(vk::make_api_version(1, 0, 0, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_api_version(1, 0, 0, 0))
            .api_version(vk::API_VERSION_1_0)
            .build();

        // Filter out the the extensions unsupported by the vulkan instance
        let avaible_extensions = entry.enumerate_instance_extension_properties(None).unwrap();
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
                .map(|ext| ext.as_ptr() as *const i8)
                .collect();

        // Filter out the the layers unsupported by the vulkan instance
        #[cfg(debug_assertions)]
        let layers: Vec<*const i8> =
            {
                let avaible_layers = entry.enumerate_instance_layer_properties().unwrap();
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

        // Define the vulkan instance create info
        let create_info_builder = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        #[cfg(debug_assertions)]
        let mut debug_messenger_create_info = Self::debug_messenger_create_info();
        #[cfg(debug_assertions)]
        let create_info = create_info_builder
            .enabled_layer_names(&layers)
            .push_next(&mut debug_messenger_create_info)
            .build();

        #[cfg(not(debug_assertions))]
        let create_info = create_info_builder.build();

        // Create the instance
        // Safety: The instane is the last destroyed object
        unsafe { entry.create_instance(&create_info, None) }.unwrap()
    }

    fn create_surface<T>(
        entry: &Entry,
        instance: &Instance,
        event_loop: &EventLoop<T>,
        window: &Window,
    ) -> SurfaceHodlder {
        let surface_ext = khr::Surface::new(entry, instance);
        let surface = unsafe {
            ash_window::create_surface(
                entry,
                instance,
                event_loop.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )
            .unwrap()
        };

        SurfaceHodlder {
            surface_ext,
            surface,
        }
    }

    /// Chooses the first avaible physical device that suits the needs of the application
    fn pick_physical_device(
        instance: &Instance,
        surface: &SurfaceHodlder,
    ) -> (vk::PhysicalDevice, QueueFamilyIndice) {
        let physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        physical_devices
            .into_iter()
            .find_map(|device| {
                Self::is_device_suitable(instance, device, surface).map(|indices| (device, indices))
            })
            .unwrap()
    }

    /// Checks if the physical device meets the application's requirements
    fn is_device_suitable(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> Option<QueueFamilyIndice> {
        let indices = Self::find_queue_families(instance, device, surface);
        if !indices.is_complete() {
            return None;
        }

        let extensions_supported = Self::check_device_extensions_support(instance, device);
        if !extensions_supported {
            return None;
        }

        let swapchain_details = Self::query_swapchain_support(device, surface);
        let swapchain_adequate =
            !swapchain_details.formats.is_empty() && !swapchain_details.present_modes.is_empty();
        if !swapchain_adequate {
            return None;
        }

        Some(indices)
    }

    /// Finds the needed queue families from the physical device
    fn find_queue_families(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> QueueFamilyIndice {
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
                surface
                    .surface_ext
                    .get_physical_device_surface_support(device, i, surface.surface)
                    .unwrap()
            } {
                indices.present_family = Some(i)
            }
        }

        indices
    }

    fn check_device_extensions_support(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        let avaible_extensions = unsafe {
            instance
                .enumerate_device_extension_properties(device)
                .unwrap()
        };

        let mut avaible_extensions_set = HashSet::new();
        for a_ext in avaible_extensions.into_iter() {
            avaible_extensions_set.insert(unsafe { CStr::from_ptr(a_ext.extension_name.as_ptr()) });
        }

        for &ext in DEVICE_EXTENSIONS.iter() {
            if !avaible_extensions_set.insert(ext) {
                return false;
            }
        }

        true
    }

    fn query_swapchain_support(
        device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
    ) -> SwapChainDetails {
        let capabilities = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_capabilities(device, surface.surface)
                .unwrap()
        };

        let formats = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_formats(device, surface.surface)
                .unwrap()
        };

        let present_modes = unsafe {
            surface
                .surface_ext
                .get_physical_device_surface_present_modes(device, surface.surface)
                .unwrap()
        };

        SwapChainDetails {
            capabilities,
            formats,
            present_modes,
        }
    }

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

    /// Creates the VkDevice
    fn create_logical_device(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
        indices: QueueFamilyIndice,
    ) -> (Device, vk::Queue, vk::Queue) {
        let unique_families = indices.get_unique_families();
        let mut queue_create_infos = Vec::with_capacity(unique_families.len());
        for queue_family in unique_families.into_iter() {
            queue_create_infos.push(
                vk::DeviceQueueCreateInfo::builder()
                    .queue_family_index(queue_family)
                    .queue_priorities(&[1.0])
                    .build(),
            )
        }

        let device_features = vk::PhysicalDeviceFeatures::builder().build();
        let device_extensions = DEVICE_EXTENSIONS
            .iter()
            .map(|&ext| ext.as_ptr())
            .collect::<Vec<*const i8>>();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features)
            .enabled_extension_names(&device_extensions)
            .build();

        // Safety: The Device is destroyed befor the parent Instance, see Application::cleanup()
        let device = unsafe {
            instance
                .create_device(physical_device, &create_info, None)
                .unwrap()
        };

        let graphics_queue =
            unsafe { device.get_device_queue(indices.graphics_family.unwrap(), 0) };
        let present_queue = unsafe { device.get_device_queue(indices.present_family.unwrap(), 0) };

        (device, graphics_queue, present_queue)
    }

    fn create_swapchain(
        instance: &Instance,
        device: &Device,
        physical_device: vk::PhysicalDevice,
        surface: &SurfaceHodlder,
        indices: QueueFamilyIndice,
    ) -> SwapChainHolder {
        let swapchain_support = Self::query_swapchain_support(physical_device, surface);

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

        let create_info_builder = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.surface)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .present_mode(present_mode)
            .image_extent(extent)
            .min_image_count(image_count)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .clipped(true);

        let graphics = indices.graphics_family.unwrap();
        let present = indices.present_family.unwrap();
        let create_info = if graphics != present {
            create_info_builder
                .image_sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(&[graphics, present])
                .build()
        } else {
            create_info_builder
                .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build()
        };

        let swapchain_ext = khr::Swapchain::new(instance, device);
        let swapchain = unsafe { swapchain_ext.create_swapchain(&create_info, None).unwrap() };
        let swapchain_images = unsafe { swapchain_ext.get_swapchain_images(swapchain).unwrap() };
        let swapchain_image_views =
            Self::create_image_views(device, &swapchain_images, surface_format.format);

        SwapChainHolder {
            swapchain_ext,
            swapchain,
            swapchain_images,
            swapchain_image_views,
            image_format: surface_format.format,
            extent,
        }
    }

    fn create_image_views(
        device: &Device,
        images: &Vec<vk::Image>,
        image_format: vk::Format,
    ) -> Vec<vk::ImageView> {
        let mut image_views = Vec::with_capacity(images.len());

        for &image in images {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(image_format)
                .components(vk::ComponentMapping::default())
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build(),
                )
                .build();

            let image_view = unsafe { device.create_image_view(&create_info, None).unwrap() };
            image_views.push(image_view)
        }

        image_views
    }

    fn create_render_pass(device: &Device, swapchain: &SwapChainHolder) -> vk::RenderPass {
        let color_attachent = vk::AttachmentDescription::builder()
            .format(swapchain.image_format)
            .samples(vk::SampleCountFlags::TYPE_1)
            .load_op(vk::AttachmentLoadOp::CLEAR)
            .store_op(vk::AttachmentStoreOp::STORE)
            .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
            .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .final_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .build();

        let color_attachment_ref = vk::AttachmentReference::builder()
            .attachment(0)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
            .build();

        let subpass = vk::SubpassDescription::builder()
            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
            .color_attachments(&[color_attachment_ref])
            .build();

        let renderpass_info = vk::RenderPassCreateInfo::builder()
            .attachments(&[color_attachent])
            .subpasses(&[subpass])
            .build();

        unsafe { device.create_render_pass(&renderpass_info, None).unwrap() }
    }

    fn create_graphics_pipeline(
        device: &Device,
        swapchain: &SwapChainHolder,
    ) -> GraphicsPipelineHolder {
        let renderpass = Self::create_render_pass(device, swapchain);

        let vert_shader_u8 = include_bytes!("spirv/vertex.spv");
        let frag_shader_u8 = include_bytes!("spirv/fragment.spv");

        let vert_shader_code = Self::make_spirv_raw(vert_shader_u8);
        let frag_shader_code = Self::make_spirv_raw(frag_shader_u8);

        let vert_module = Self::create_shader_module(device, &vert_shader_code);
        let frag_module = Self::create_shader_module(device, &frag_shader_code);

        let entry_point = CString::new("main").unwrap();
        let vert_shader_stage_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::VERTEX)
            .module(vert_module)
            .name(&entry_point)
            .build();
        let frag_shader_stage_info = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::FRAGMENT)
            .module(frag_module)
            .name(&entry_point)
            .build();

        let shader_stages_info = [vert_shader_stage_info, frag_shader_stage_info];

        let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states)
            .build();

        let vertex_input_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(&[])
            .vertex_attribute_descriptions(&[])
            .build();

        let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build();

        // let viewport = vk::Viewport::builder()
        //     .x(0.0)
        //     .y(0.0)
        //     .width(swapchain.extent.width as f32)
        //     .height(swapchain.extent.height as f32)
        //     .min_depth(0.0)
        //     .max_depth(1.0)
        //     .build();

        // let scissor = vk::Rect2D::builder()
        //     .offset(vk::Offset2D::builder().x(0).y(0).build())
        //     .extent(swapchain.extent)
        //     .build();

        let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
            .viewport_count(1)
            .scissor_count(1)
            .build();

        let rasterizer = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::BACK)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build();

        let multisampling = vk::PipelineMultisampleStateCreateInfo::builder()
            .sample_shading_enable(false)
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .min_sample_shading(1.0)
            .sample_mask(&[])
            .alpha_to_coverage_enable(false)
            .alpha_to_one_enable(false)
            .build();

        let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
            .color_write_mask(vk::ColorComponentFlags::RGBA)
            .blend_enable(false)
            // .src_color_blend_factor(vk::BlendFactor::ONE)
            // .dst_color_blend_factor(vk::BlendFactor::ZERO)
            // .color_blend_op(vk::BlendOp::ADD)
            // .src_alpha_blend_factor(vk::BlendFactor::ONE)
            // .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
            // .alpha_blend_op(vk::BlendOp::ADD)
            .build();

        let color_blending = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op_enable(false)
            .logic_op(vk::LogicOp::COPY)
            .attachments(&[color_blend_attachment])
            .blend_constants([0.0, 0.0, 0.0, 0.0])
            .build();

        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&[])
            .push_constant_ranges(&[])
            .build();
        let pipeline_layout = unsafe {
            device
                .create_pipeline_layout(&pipeline_layout_info, None)
                .unwrap()
        };

        let pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
            .stages(&shader_stages_info)
            .vertex_input_state(&vertex_input_info)
            .input_assembly_state(&input_assembly_info)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterizer)
            .multisample_state(&multisampling)
            .color_blend_state(&color_blending)
            .dynamic_state(&dynamic_state_create_info)
            .layout(pipeline_layout)
            .render_pass(renderpass)
            .subpass(0)
            .base_pipeline_handle(vk::Pipeline::null())
            .base_pipeline_index(-1)
            .build();

        let pipeline = unsafe {
            device
                .create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .unwrap()[0]
        };

        unsafe {
            device.destroy_shader_module(vert_module, None);
            device.destroy_shader_module(frag_module, None);
        }

        GraphicsPipelineHolder {
            renderpass,
            pipeline,
            pipeline_layout,
        }
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

    fn create_shader_module(device: &Device, bytes: &[u32]) -> vk::ShaderModule {
        let create_info = vk::ShaderModuleCreateInfo::builder().code(bytes).build();
        unsafe { device.create_shader_module(&create_info, None).unwrap() }
    }

    /// Sets up the debug messenger for the validation layers
    #[cfg(debug_assertions)]
    fn setup_debug_messenger(entry: &Entry, instance: &Instance) -> DebugMessengerHolder {
        let debug_util_ext = ext::DebugUtils::new(entry, instance);

        let create_info = Self::debug_messenger_create_info();

        let debug_messenger = unsafe {
            debug_util_ext
                .create_debug_utils_messenger(&create_info, None)
                .unwrap()
        };

        DebugMessengerHolder {
            debug_util_ext,
            debug_messenger,
        }
    }

    /// Creates the VkDebugUtilsMessengerCreateInfoEXT for the debug messenger
    #[cfg(debug_assertions)]
    fn debug_messenger_create_info() -> vk::DebugUtilsMessengerCreateInfoEXT {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                // vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE |
                // vk::DebugUtilsMessageSeverityFlagsEXT::INFO |
                vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::debug_callback))
            .build()
    }

    /// Is called for every validation layers event
    #[cfg(debug_assertions)]
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

    fn _main_loop() {
        todo!()
    }

    /// Destroys the Vulkan objects
    pub fn cleanup(&self) {
        unsafe {
            self.device
                .destroy_pipeline_layout(self.pipeline.pipeline_layout, None);

            self.device.destroy_pipeline(self.pipeline.pipeline, None);

            self.device
                .destroy_render_pass(self.pipeline.renderpass, None);

            for &image_view in self.swapchain.swapchain_image_views.iter() {
                self.device.destroy_image_view(image_view, None)
            }
            self.swapchain
                .swapchain_ext
                .destroy_swapchain(self.swapchain.swapchain, None);
            self.device.destroy_device(None);
            self.surface
                .surface_ext
                .destroy_surface(self.surface.surface, None);

            #[cfg(debug_assertions)]
            self.debug_messenger
                .debug_util_ext
                .destroy_debug_utils_messenger(self.debug_messenger.debug_messenger, None);

            self.instance.destroy_instance(None);
        };
    }
}
