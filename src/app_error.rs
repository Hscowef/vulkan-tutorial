use ash::vk;
use raw_window_handle::HandleError;

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
    NoSuitableMemType,
    IoError,
    HandleError,
}

impl AppErrorType {
    const MSG_VULKAN_LOADING_ERROR: &'static str = "Couldn't load the Vulkan library.";
    const MSG_NO_SUITABLE_DEVICE: &'static str = "No suitable physical device is avaible.";
    const MSG_NO_SUITABLE_MEM_TYPE: &'static str = "Failed to find suitable memory type.";
    const MSG_IO_ERROR: &'static str = "An io error occured.";
    const MSG_HANDLE_ERROR: &'static str = "An error occured while retreiving an handle.";
}

impl AppError {
    pub fn new(error_type: AppErrorType) -> Self {
        let message = match error_type {
            AppErrorType::VulkanError(vk_result) => vk_result.to_string(),
            AppErrorType::VulkanLoadingError => {
                String::from(AppErrorType::MSG_VULKAN_LOADING_ERROR)
            }
            AppErrorType::NoSuitableDevice => String::from(AppErrorType::MSG_NO_SUITABLE_DEVICE),
            AppErrorType::NoSuitableMemType => String::from(AppErrorType::MSG_NO_SUITABLE_MEM_TYPE),
            AppErrorType::IoError => String::from(AppErrorType::MSG_IO_ERROR),
            AppErrorType::HandleError => String::from(AppErrorType::MSG_HANDLE_ERROR),
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

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError {
            error_type: AppErrorType::IoError,
            message: value.to_string(),
        }
    }
}

impl From<image::ImageError> for AppError {
    fn from(value: image::ImageError) -> Self {
        AppError {
            error_type: AppErrorType::IoError,
            message: value.to_string(),
        }
    }
}

impl From<HandleError> for AppError {
    fn from(value: HandleError) -> Self {
        AppError {
            error_type: AppErrorType::HandleError,
            message: value.to_string(),
        }
    }
}
