use vulkan_tutorial::Application;

use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const NAME: &str = "Vulkan tutorial";
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(NAME)
        .with_inner_size(LogicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    let mut application = Application::create(&event_loop, &window).unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                window_id,
            } if window_id == window.id() => application.request_resize(),
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                application.draw_frame().unwrap();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,

            _ => (),
        }

        if let ControlFlow::Exit = *control_flow {
            application.cleanup();
        }

        window.request_redraw();
    });
}
