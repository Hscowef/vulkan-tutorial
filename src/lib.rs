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
    image_format: vk::Format,
    extent: vk::Extent2D,
}

#[allow(dead_code)]
pub struct Application {
    _entry: Entry,
    surface_ext: khr::Surface,
    #[cfg(debug_assertions)]
    debug_util_ext: ext::DebugUtils,

    instance: Instance,
    surface: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: Device,
    graphics_queue: vk::Queue,
    present_queue: vk::Queue,
    swapchain: SwapChainHolder,

    #[cfg(debug_assertions)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
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
        let (debug_util_ext, debug_messenger) = Self::setup_debug_messenger(&entry, &instance);

        let (surface, surface_ext) = Self::create_surface(&entry, &instance, event_loop, window);

        // Choosing the VkPhisicalDevice, create the VkDevice and the graphics queue
        let (physical_device, queue_family_indices) =
            Self::pick_physical_device(&instance, surface, &surface_ext);
        let (device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, physical_device, queue_family_indices);

        let swapchain = Self::create_swapchain(
            &instance,
            &device,
            physical_device,
            surface,
            &surface_ext,
            queue_family_indices,
        );

        Self {
            _entry: entry,
            surface_ext,
            #[cfg(debug_assertions)]
            debug_util_ext,

            instance,
            surface,
            physical_device,
            device,
            graphics_queue,
            present_queue,
            swapchain,

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
    ) -> (vk::SurfaceKHR, khr::Surface) {
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

        (surface, surface_ext)
    }

    /// Chooses the first avaible physical device that suits the needs of the application
    fn pick_physical_device(
        instance: &Instance,
        surface: vk::SurfaceKHR,
        surface_ext: &khr::Surface,
    ) -> (vk::PhysicalDevice, QueueFamilyIndice) {
        let physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        physical_devices
            .into_iter()
            .find_map(|device| {
                Self::is_device_suitable(instance, device, surface, surface_ext)
                    .map(|indices| (device, indices))
            })
            .unwrap()
    }

    /// Checks if the physical device meets the application's requirements
    fn is_device_suitable(
        instance: &Instance,
        device: vk::PhysicalDevice,
        surface: vk::SurfaceKHR,
        surface_ext: &khr::Surface,
    ) -> Option<QueueFamilyIndice> {
        let indices = Self::find_queue_families(instance, device, surface, surface_ext);
        if !indices.is_complete() {
            return None;
        }

        let extensions_supported = Self::check_device_extensions_support(instance, device);
        if !extensions_supported {
            return None;
        }

        let swapchain_details = Self::query_swapchain_support(device, surface, surface_ext);
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
        surface: vk::SurfaceKHR,
        surface_ext: &khr::Surface,
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
                surface_ext
                    .get_physical_device_surface_support(device, i, surface)
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
        surface: vk::SurfaceKHR,
        surface_ext: &khr::Surface,
    ) -> SwapChainDetails {
        let capabilities = unsafe {
            surface_ext
                .get_physical_device_surface_capabilities(device, surface)
                .unwrap()
        };

        let formats = unsafe {
            surface_ext
                .get_physical_device_surface_formats(device, surface)
                .unwrap()
        };

        let present_modes = unsafe {
            surface_ext
                .get_physical_device_surface_present_modes(device, surface)
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
        surface: vk::SurfaceKHR,
        surface_ext: &khr::Surface,
        indices: QueueFamilyIndice,
    ) -> SwapChainHolder {
        let swapchain_support =
            Self::query_swapchain_support(physical_device, surface, surface_ext);

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
            .surface(surface)
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

        SwapChainHolder {
            swapchain_ext,
            swapchain,
            swapchain_images,
            image_format: surface_format.format,
            extent,
        }
    }

    /// Sets up the debug messenger for the validation layers
    #[cfg(debug_assertions)]
    fn setup_debug_messenger(
        entry: &Entry,
        instance: &Instance,
    ) -> (ext::DebugUtils, vk::DebugUtilsMessengerEXT) {
        let debug_util_ext = ext::DebugUtils::new(entry, instance);

        let create_info = Self::debug_messenger_create_info();

        let debug_util_messenger = unsafe {
            debug_util_ext
                .create_debug_utils_messenger(&create_info, None)
                .unwrap()
        };

        (debug_util_ext, debug_util_messenger)
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
            self.swapchain
                .swapchain_ext
                .destroy_swapchain(self.swapchain.swapchain, None);
            self.device.destroy_device(None);
            self.surface_ext.destroy_surface(self.surface, None);

            #[cfg(debug_assertions)]
            self.debug_util_ext
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.instance.destroy_instance(None);
        };
    }
}
