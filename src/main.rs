extern crate glow as gl;
use anyhow::{bail, format_err, Context as AnyhowContext, Result};
use gl::HasContext;
use glutin::event::{Event, WindowEvent};
use glutin::event_loop::ControlFlow;

const N_PARTICLES: i32 = 10_000;
const LOCAL_SIZE: i32 = 32;
const WIDTH: i32 = 16 * LOCAL_SIZE;
const HEIGHT: i32 = 16 * LOCAL_SIZE;
const N_ITERS: u32 = 15 * 2;

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
        let gl = gl::Context::from_loader_function(|s| window.get_proc_address(s) as *const _);

        // Particle vertex array
        let particle_vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(particle_vertex_array));

        // Particle buffer
        let particle_buffer = gl.create_buffer().map_err(|e| format_err!("{}", e))?;
        gl.bind_buffer(gl::ARRAY_BUFFER, Some(particle_buffer));
        gl.buffer_data_size(
            gl::ARRAY_BUFFER,
            N_PARTICLES * std::mem::size_of::<f32>() as i32 * 2,
            gl::DYNAMIC_DRAW,
        );
        gl.bind_vertex_array(None);

        // Set up fragment/vertex shaders
        let shader_sources = [
            (gl::VERTEX_SHADER, include_str!("shaders/particles.vert")),
            (gl::FRAGMENT_SHADER, include_str!("shaders/particles.frag")),
        ];
        let particle_shader = create_program(&gl, &shader_sources)?;

        let particle_kernel = create_program(
            &gl,
            &[(gl::COMPUTE_SHADER, include_str!("kernels/particles.comp"))],
        )?;
        let jacobi_kernel = create_program(
            &gl,
            &[(gl::COMPUTE_SHADER, include_str!("kernels/jacobi.comp"))],
        )?;
        let advect_kernel = create_program(
            &gl,
            &[(gl::COMPUTE_SHADER, include_str!("kernels/advect.comp"))],
        )?;

        // Set up textures
        let mut read_texture = gl.create_texture().map_err(|e| format_err!("{}", e))?;
        gl.bind_texture(gl::TEXTURE_2D, Some(read_texture));
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            gl::RG32F as _,
            WIDTH,
            HEIGHT,
            0,
            gl::RG,
            gl::FLOAT,
            None,
        );
        gl.texture_parameter_i32(read_texture, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        gl.texture_parameter_i32(read_texture, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

        let mut write_texture = gl.create_texture().map_err(|e| format_err!("{}", e))?;
        gl.bind_texture(gl::TEXTURE_2D, Some(write_texture));
        gl.tex_image_2d(
            gl::TEXTURE_2D,
            0,
            gl::RG32F as _,
            WIDTH,
            HEIGHT,
            0,
            gl::RG,
            gl::FLOAT,
            None,
        );
        gl.texture_parameter_i32(write_texture, gl::TEXTURE_MIN_FILTER, gl::NEAREST as _);
        gl.texture_parameter_i32(write_texture, gl::TEXTURE_MAG_FILTER, gl::NEAREST as _);

        // Set up GL state
        gl.clear_color(0., 0., 0., 1.0);
        gl.enable(gl::VERTEX_PROGRAM_POINT_SIZE);

        let mut dt = 0.;

        // Event loop
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
                    // Execute jacobi kernel
                    gl.use_program(Some(jacobi_kernel));
                    let parity_loc = gl.get_uniform_location(jacobi_kernel, "parity");
                    for i in 0..N_ITERS {
                        let parity = i % 2;
                        gl.uniform_1_u32(parity_loc.as_ref(), parity);
                        // Set read texture to binding=0
                        gl.bind_image_texture(0, read_texture, 0, false, 0, gl::READ_WRITE, gl::RG32F);
                        // Set write texture to binding=1
                        gl.bind_image_texture(1, write_texture, 0, false, 0, gl::READ_WRITE, gl::RG32F);
                        // Run kernel
                        gl.dispatch_compute((WIDTH / LOCAL_SIZE) as _, (HEIGHT / LOCAL_SIZE) as _, 1);
                        // Memory barrier for vertex shader
                        gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                        std::mem::swap(&mut read_texture, &mut write_texture);
                    }

                    // Execute advection kernel
                    gl.use_program(Some(advect_kernel));
                    // Set read texture to binding=0
                    gl.active_texture(gl::TEXTURE0);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_texture));
                    // Set write texture to binding=1
                    gl.bind_image_texture(1, write_texture, 0, false, 0, gl::READ_WRITE, gl::RG32F);
                    // Set dt
                    let dt_loc = gl.get_uniform_location(advect_kernel, "dt");
                    gl.uniform_1_f32(dt_loc.as_ref(), dt);
                    gl.dispatch_compute((WIDTH / LOCAL_SIZE) as _, (HEIGHT / LOCAL_SIZE) as _, 1);
                    // Memory barrier for vertex shader
                    gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                    std::mem::swap(&mut read_texture, &mut write_texture);

                    // Execute particle kernel
                    gl.use_program(Some(particle_kernel));
                    // Set particle buffer to binding=2
                    gl.bind_buffer_base(gl::SHADER_STORAGE_BUFFER, 2, Some(particle_buffer));
                    // Set read texture to binding=0
                    gl.active_texture(gl::TEXTURE0);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_texture));
                    // Set write texture to binding=1
                    gl.bind_image_texture(1, write_texture, 0, false, 0, gl::READ_WRITE, gl::RG32F);
                    // Set dt
                    let dt_loc = gl.get_uniform_location(particle_kernel, "dt");
                    gl.uniform_1_f32(dt_loc.as_ref(), dt);
                    // Dispatch
                    gl.dispatch_compute(N_PARTICLES as u32, 1, 1);
                    // Memory barrier for vertex shader
                    gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                    // Draw particles
                    gl.clear(gl::COLOR_BUFFER_BIT);
                    gl.use_program(Some(particle_shader));
                    gl.bind_vertex_array(Some(particle_vertex_array));
                    gl.draw_arrays(gl::POINTS, 0, N_PARTICLES);

                    dt = 0.1;

                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        window.resize(*physical_size);
                    }
                    WindowEvent::CloseRequested => {
                        gl.delete_program(particle_shader);
                        gl.delete_vertex_array(particle_vertex_array);
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
pub fn create_program(
    gl: &gl::Context,
    shader_sources: &[(u32, &str)],
) -> Result<gl::NativeProgram> {
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
