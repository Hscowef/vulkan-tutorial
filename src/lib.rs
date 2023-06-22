use std::{
    borrow::Borrow,
    ffi::{CStr, CString},
};

use ash::{vk, Entry, Instance};
use ash_window::enumerate_required_extensions;
use raw_window_handle::HasRawDisplayHandle;
use winit::event_loop::EventLoop;

const EXTENSIONS: &[&str] = &[];
#[cfg(debug_assertions)]
const VALIDATION_LAYERS: &[&str] = &["VK_LAYER_KHRONOS_validation"];

#[allow(dead_code)]
pub struct Application {
    entry: Entry,
    instance: Instance,
}

impl Application {
    pub fn new<T>(event_loop: &EventLoop<T>) -> Self {
        let entry = unsafe { Entry::load().unwrap() };

        let winit_extension_names =
            enumerate_required_extensions(event_loop.raw_display_handle()).unwrap();
        let extensions_cstring: Vec<CString> = EXTENSIONS
            .iter()
            .map(|&ext| CString::new(ext).unwrap())
            .collect();
        let extension_names = extensions_cstring
            .iter()
            .map(|ext| ext.as_ptr())
            .chain(winit_extension_names.iter().copied());

        #[cfg(debug_assertions)]
        let layers_cstring: Vec<CString> = VALIDATION_LAYERS
            .iter()
            .map(|&ext| CString::new(ext).unwrap())
            .collect();
        #[cfg(debug_assertions)]
        let layer_name_temp = layers_cstring
            .iter()
            .map(|lay| lay.as_ptr())
            .collect::<Vec<*const i8>>();

        #[cfg(debug_assertions)]
        let layer_names = Some(&*layer_name_temp);
        #[cfg(not(debug_assertions))]
        let layer_names = None;

        let instance = { Self::create_instance(&entry, extension_names, layer_names) };
        Self { entry, instance }
    }

    fn create_instance<T>(
        entry: &Entry,
        extension_names: T,
        layer_names: Option<&[*const i8]>,
    ) -> Instance
    where
        T: IntoIterator<Item = *const i8>,
    {
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
                .map(|ext| unsafe { CStr::from_ptr(ext) })
                .filter(|&ext| {
                    avaible_extensions
                    .iter()
                    .find(|a_ext| unsafe {CStr::from_bytes_until_nul(std::mem::transmute::<&[i8], &[u8]>(&a_ext.extension_name[..])).unwrap()} == ext)
                    .or_else(|| {println!("Extension unsupported: {:?} ", ext); None})
                    .is_some()
                })
                .map(|ext| ext.as_ptr() as *const i8)
                .collect();

        // Define the vulkan instance create info
        let create_info_builder = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        let create_info = match layer_names {
            Some(names) => create_info_builder.enabled_layer_names(&names).build(),
            None => create_info_builder.build(),
        };

        // Create the instance
        unsafe { entry.create_instance(&create_info, None) }.unwrap()
    }

    fn _main_loop() {
        todo!()
    }

    pub fn cleanup(&self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}
