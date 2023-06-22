mod queue_families;

use crate::queue_families::QueueFamilyIndice;

use std::ffi::{CStr, CString};

#[cfg(debug_assertions)]
use ash::extensions::ext;
use ash::{vk, Entry, Instance};
use ash_window::enumerate_required_extensions;
use colored::Colorize;
use raw_window_handle::HasRawDisplayHandle;
use winit::event_loop::EventLoop;

#[cfg(debug_assertions)]
const EXTENSIONS: &[&[u8]] = &[b"VK_EXT_debug_utils\0"];
#[cfg(not(debug_assertions))]
const EXTENSIONS: &[&[u8]] = &[];

#[cfg(debug_assertions)]
const VALIDATION_LAYERS: &[&[u8]] = &[b"VK_LAYER_KHRONOS_validation\0"];
#[cfg(debug_assertions)]
const LAYER_SEVERITY: vk::DebugUtilsMessageSeverityFlagsEXT =
    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE;

#[allow(dead_code)]
pub struct Application {
    _entry: Entry,
    instance: Instance,
    physical_device: vk::PhysicalDevice,

    #[cfg(debug_assertions)]
    debug_util_ext: ext::DebugUtils,
    #[cfg(debug_assertions)]
    debug_messenger: vk::DebugUtilsMessengerEXT,
}

impl Application {
    pub fn new<T>(event_loop: &EventLoop<T>) -> Self {
        let entry = unsafe { Entry::load().unwrap() };

        let winit_extension_names =
            enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();
        let extension_names = EXTENSIONS
            .iter()
            .map(|&ext| CStr::from_bytes_with_nul(ext).unwrap())
            .chain(
                winit_extension_names
                    .iter()
                    .map(|&ext| unsafe { CStr::from_ptr(ext) }),
            );

        #[cfg(debug_assertions)]
        let layer_names = VALIDATION_LAYERS
            .iter()
            .map(|&lay| CStr::from_bytes_with_nul(lay).unwrap());

        #[cfg(debug_assertions)]
        let instance = { Self::create_instance(&entry, extension_names, layer_names) };
        #[cfg(not(debug_assertions))]
        let instance = { Self::create_instance(&entry, extension_names) };

        #[cfg(debug_assertions)]
        let (debug_util_ext, debug_messenger) = Self::setup_debug_messenger(&entry, &instance);

        let physical_device = Self::pick_physical_device(&instance);

        Self {
            _entry: entry,
            instance,
            physical_device,

            #[cfg(debug_assertions)]
            debug_util_ext,
            #[cfg(debug_assertions)]
            debug_messenger,
        }
    }

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
        unsafe { entry.create_instance(&create_info, None) }.unwrap()
    }

    fn pick_physical_device(instance: &Instance) -> vk::PhysicalDevice {
        let physical_devices = unsafe { instance.enumerate_physical_devices().unwrap() };
        physical_devices
            .into_iter()
            .find(|&device| Self::is_device_suitable(instance, device))
            .unwrap()
    }

    fn is_device_suitable(instance: &Instance, device: vk::PhysicalDevice) -> bool {
        // let _device_proprieties = unsafe { instance.get_physical_device_properties(device) };
        // let _device_features = unsafe { instance.get_physical_device_features(device) };
        let indices = Self::find_queue_families(instance, device);
        indices.is_complete()
    }

    fn find_queue_families(instance: &Instance, device: vk::PhysicalDevice) -> QueueFamilyIndice {
        let queue_families =
            unsafe { instance.get_physical_device_queue_family_properties(device) };

        let mut indices = QueueFamilyIndice::default();
        for (i, family) in queue_families.iter().enumerate() {
            if indices.is_complete() {
                break;
            }

            if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                indices.graphics_family = Some(i)
            }
        }

        indices
    }

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

    pub fn cleanup(&self) {
        unsafe {
            #[cfg(debug_assertions)]
            self.debug_util_ext
                .destroy_debug_utils_messenger(self.debug_messenger, None);

            self.instance.destroy_instance(None);
        };
    }
}
