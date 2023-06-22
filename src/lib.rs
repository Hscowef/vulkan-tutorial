use std::ffi::{CStr, CString};

use ash::{vk, Entry, Instance};
use ash_window::enumerate_required_extensions;
use raw_window_handle::HasRawDisplayHandle;
use winit::event_loop::EventLoop;

#[cfg(debug_assertions)]
const EXTENSIONS: &[&[u8]] = &[b"VK_EXT_debug_utils\0"];
#[cfg(not(debug_assertions))]
const EXTENSIONS: &[&[u8]] = &[];

#[cfg(debug_assertions)]
const VALIDATION_LAYERS: &[&[u8]] = &[b"VK_LAYER_KHRONOS_validation\0"];

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
        let extension_names = EXTENSIONS
            .iter()
            .map(|&ext| CStr::from_bytes_with_nul(ext).unwrap())
            .chain(
                winit_extension_names
                    .iter()
                    .map(|&ext| unsafe { CStr::from_ptr(ext) }),
            );

        #[cfg(debug_assertions)]
        let layer_name_temp = VALIDATION_LAYERS
            .iter()
            .map(|&lay| CStr::from_bytes_with_nul(lay).unwrap().as_ptr())
            .collect::<Vec<*const i8>>();

        #[cfg(debug_assertions)]
        let layer_names = Some(&*layer_name_temp);
        #[cfg(not(debug_assertions))]
        let layer_names = None;

        let instance = { Self::create_instance(&entry, extension_names, layer_names) };
        Self { entry, instance }
    }

    fn create_instance<'a, T>(
        entry: &Entry,
        extension_names: T,
        layer_names: Option<&[*const i8]>,
    ) -> Instance
    where
        T: IntoIterator<Item = &'a CStr>,
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
        let extensions: Vec<*const i8> = extension_names
            .into_iter()
            .filter(|&ext| {
                avaible_extensions
                    .iter()
                    .find(|&a_ext| unsafe { CStr::from_ptr(a_ext.extension_name.as_ptr()) } == ext)
                    .or_else(|| {
                        println!("Extension unsupported: {:?} ", ext);
                        None
                    })
                    .is_some()
            })
            .map(|ext| ext.as_ptr() as *const i8)
            .collect();

        // let avaible_layers = entry.enumerate_instance_layer_properties().unwrap();
        // let mut layers: Option<Vec<*const i8>> = None;
        // if let Some(names) = layer_names {
        //     layers = Some(
        //         names
        //             .iter()
        //             .filter(|&&lay| {
        //                 avaible_layers.iter().find(
        //                     |&a_lay| unsafe { CStr::from_ptr(a_lay.layer_name.as_ptr()) } == lay,
        //                 ).or_else(|| {
        //                     println!("Layer unsupported: {:?} ", lay);
        //                     None
        //                 }).is_some()
        //             })
        //             .map(|&lay| lay.as_ptr())
        //             .collect(),
        //     );
        // }

        // Define the vulkan instance create info
        // TODO: Check validation layers avaibility
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

    #[cfg(debug_assertions)]
    fn setup_debug_messenger() {}

    fn _main_loop() {
        todo!()
    }

    pub fn cleanup(&self) {
        unsafe { self.instance.destroy_instance(None) };
    }
}
