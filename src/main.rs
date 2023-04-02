use anyhow::{bail, format_err, Context as AnyhowContext, Result};
use glow::*;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

fn main() -> Result<()> {
    unsafe {
        // Create a context from a glutin window on non-wasm32 targets
        let event_loop = glutin::event_loop::EventLoop::new();
        let window_builder = glutin::window::WindowBuilder::new()
            .with_title("Hello triangle!")
            .with_inner_size(glutin::dpi::LogicalSize::new(1024.0, 768.0));
        let window = glutin::ContextBuilder::new()
            .with_vsync(true)
            .build_windowed(window_builder, &event_loop)
            .unwrap()
            .make_current()
            .unwrap();
        let gl = glow::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);

        // Particle vertex array
        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));

        // Set up fragment/vertex shaders
        let shader_sources = [
            (glow::VERTEX_SHADER, include_str!("shaders/particles.vert")),
            (
                glow::FRAGMENT_SHADER,
                include_str!("shaders/particles.frag"),
            ),
        ];

        let program = create_program(&gl, &shader_sources)?;

        gl.use_program(Some(program));
        gl.clear_color(0., 0., 0., 1.0);
        gl.enable(VERTEX_PROGRAM_POINT_SIZE);

        // We handle events differently between targets

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            match event {
                Event::LoopDestroyed => {
                    return;
                }
                Event::MainEventsCleared => {
                    window.window().request_redraw();
                }
                Event::RedrawRequested(_) => {
                    gl.clear(glow::COLOR_BUFFER_BIT);
                    gl.draw_arrays(glow::POINTS, 0, 3);
                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        window.resize(*physical_size);
                    }
                    WindowEvent::CloseRequested => {
                        gl.delete_program(program);
                        gl.delete_vertex_array(vertex_array);
                        *control_flow = ControlFlow::Exit
                    }
                    _ => (),
                },
                _ => (),
            }
        });
    }
}

/// Compile and link program from sources
pub fn create_program(gl: &Context, shader_sources: &[(u32, &str)]) -> Result<NativeProgram> {
    unsafe {
        let program = gl
            .create_program()
            .map_err(|e| format_err!("{:#}", e))
            .context("Cannot create program")?;

        let mut shaders = Vec::with_capacity(shader_sources.len());

        for (shader_type, shader_source) in shader_sources.iter() {
            // Compile
            let shader = gl
                .create_shader(*shader_type)
                .map_err(|e| format_err!("{:#}", e))
                .context("Cannot create program")?;

            gl.shader_source(shader, &shader_source);
            gl.compile_shader(shader);

            if !gl.get_shader_compile_status(shader) {
                bail!("{}", gl.get_shader_info_log(shader));
            }

            // Attach
            gl.attach_shader(program, shader);
            shaders.push(shader);
        }

        // Link
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            bail!("{}", gl.get_program_info_log(program));
        }

        // Cleanup
        for shader in shaders {
            gl.detach_shader(program, shader);
            gl.delete_shader(shader);
        }

        Ok(program)
    }
}
