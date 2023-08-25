use ash::vk;

#[derive(Debug, Clone)]
pub struct AppError {
    pub error_type: AppErrorType,
    pub message: String,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Application error `{:?}`: {}",
            self.error_type, self.message
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AppErrorType {
    VulkanError(vk::Result),
    VulkanLoadingError,
    NoSuitableDevice,
}

impl AppErrorType {
    const MSG_VULKAN_LOADING_ERROR: &'static str = "Couldn't load the Vulkan library.";
    const MSG_NO_SUITABLE_DEVICE: &'static str = "No suitable physical device is avaible.";
}

impl AppError {
    pub fn new(error_type: AppErrorType) -> Self {
        let message = match error_type {
            AppErrorType::VulkanError(vk_result) => vk_result.to_string(),
            AppErrorType::VulkanLoadingError => {
                String::from(AppErrorType::MSG_VULKAN_LOADING_ERROR)
            }
            AppErrorType::NoSuitableDevice => String::from(AppErrorType::MSG_NO_SUITABLE_DEVICE),
        };

        Self {
            error_type,
            message,
        }
    }
}

impl From<vk::Result> for AppError {
    fn from(value: vk::Result) -> Self {
        AppError {
            error_type: AppErrorType::VulkanError(value),
            message: value.to_string(),
        }
    }
}
