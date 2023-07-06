extern crate glow as gl;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Instant;

use anyhow::{bail, format_err, Context as AnyhowContext, Result};
use gl::HasContext;
use glutin::event::{ElementState, Event, MouseButton, TouchPhase, VirtualKeyCode, WindowEvent};
use glutin::event_loop::ControlFlow;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

const N_PARTICLES: i32 = 300_000;
const LOCAL_SIZE: i32 = 32;
const WIDTH: i32 = 13 * LOCAL_SIZE;
const HEIGHT: i32 = 8 * LOCAL_SIZE;
const N_ITERS: u32 = 20;
const MAX_FINGIES: usize = 5;
const INITIAL_DT: f32 = 0.1;

const LEFT_RIGHT_DELTA_DT: f32 = INITIAL_DT;
const UP_DOWN_DELTA_DT: f32 = INITIAL_DT * 10.;

const CLEAR_DT: f32 = 9999.;

const MOUSE_IDX: u64 = 0;

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

        let mut screen_size = (1024., 768.);

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

        let mut hotloader = ShaderHotloader::new("src".into())?;

        // Set up fragment/vertex shaders
        let particle_shader = hotloader.add_program(
            &gl,
            vec![
                (gl::VERTEX_SHADER, "src/shaders/particles.vert".into()),
                (gl::FRAGMENT_SHADER, "src/shaders/particles.frag".into()),
            ],
        )?;

        let particle_kernel = hotloader
            .add_program(
                &gl,
                vec![(gl::COMPUTE_SHADER, "src/kernels/particles.comp".into())],
            )
            .unwrap();
        let jacobi_kernel = hotloader
            .add_program(
                &gl,
                vec![(gl::COMPUTE_SHADER, "src/kernels/jacobi.comp".into())],
            )
            .unwrap();
        let advect_kernel = hotloader
            .add_program(
                &gl,
                vec![(gl::COMPUTE_SHADER, "src/kernels/advect.comp".into())],
            )
            .unwrap();
        let draw_kernel = hotloader
            .add_program(&gl, vec![(gl::COMPUTE_SHADER, "src/kernels/draw.comp".into())])
            .unwrap();

        // Set up textures
        let texture = || -> Result<gl::NativeTexture> {
            let tex = gl.create_texture().map_err(|e| format_err!("{}", e))?;
            gl.bind_texture(gl::TEXTURE_2D, Some(tex));
            gl.tex_image_2d(
                gl::TEXTURE_2D,
                0,
                gl::R32F as _,
                WIDTH,
                HEIGHT,
                0,
                gl::RED,
                gl::FLOAT,
                None,
            );
            gl.texture_parameter_i32(tex, gl::TEXTURE_MIN_FILTER, gl::LINEAR as _);
            gl.texture_parameter_i32(tex, gl::TEXTURE_MAG_FILTER, gl::LINEAR as _);
            gl.texture_parameter_i32(tex, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_BORDER as _);
            gl.texture_parameter_i32(tex, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_BORDER as _);
            gl.tex_parameter_f32_slice(gl::TEXTURE_2D, gl::TEXTURE_BORDER_COLOR, &[0.0; 4]);
            Ok(tex)
        };

        let mut read_u = texture()?;
        let mut write_u = texture()?;

        let mut read_v = texture()?;
        let mut write_v = texture()?;

        // Set up GL state
        gl.clear_color(0., 0., 0., 1.0);
        //gl.enable(gl::BLEND);
        gl.disable(gl::BLEND);
        gl.blend_func(gl::SRC_ALPHA, gl::ONE);
        //gl.enable(gl::VERTEX_PROGRAM_POINT_SIZE);

        let mut dt: Option<f32> = None;
        let mut fingors: HashMap<u64, [f32; 4]> = HashMap::new();

        let time = Instant::now();

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
                    hotloader.update(&gl);

                    // Execute jacobi kernel
                    gl.use_program(Some(hotloader.get_program(jacobi_kernel)));
                    let parity_loc =
                        gl.get_uniform_location(hotloader.get_program(jacobi_kernel), "parity");

                    for i in 0..N_ITERS * 2 {
                        let parity = i % 2;

                        gl.uniform_1_u32(parity_loc.as_ref(), parity);
                        // Set read textures
                        gl.bind_image_texture(0, read_u, 0, false, 0, gl::READ_WRITE, gl::R32F);
                        gl.bind_image_texture(1, read_v, 0, false, 0, gl::READ_WRITE, gl::R32F);
                        // Set write textures
                        gl.bind_image_texture(2, write_u, 0, false, 0, gl::READ_WRITE, gl::R32F);
                        gl.bind_image_texture(3, write_v, 0, false, 0, gl::READ_WRITE, gl::R32F);

                        // Run kernel
                        gl.dispatch_compute(
                            (WIDTH / LOCAL_SIZE) as _,
                            (HEIGHT / LOCAL_SIZE) as _,
                            1,
                        );
                        // Memory barrier for vertex shader
                        gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                        std::mem::swap(&mut read_u, &mut write_u);
                        std::mem::swap(&mut read_v, &mut write_v);
                    }

                    // Execute advection kernel
                    gl.use_program(Some(hotloader.get_program(advect_kernel)));
                    // Set read textures
                    gl.active_texture(gl::TEXTURE0);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_u));
                    gl.active_texture(gl::TEXTURE1);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_v));
                    // Set write textures
                    gl.bind_image_texture(2, write_u, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    gl.bind_image_texture(3, write_v, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    // Set dt
                    let dt_loc = gl.get_uniform_location(hotloader.get_program(advect_kernel), "dt");
                    gl.uniform_1_f32(dt_loc.as_ref(), dt.unwrap_or(CLEAR_DT));
                    gl.dispatch_compute((WIDTH / LOCAL_SIZE) as _, (HEIGHT / LOCAL_SIZE) as _, 1);
                    // Memory barrier for vertex shader
                    gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                    std::mem::swap(&mut read_u, &mut write_u);
                    std::mem::swap(&mut read_v, &mut write_v);

                    // Execute touch/mouse drawing kernel
                    gl.use_program(Some(hotloader.get_program(draw_kernel)));
                    // Set read textures
                    gl.bind_image_texture(0, read_u, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    gl.bind_image_texture(1, read_v, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    // Set pens
                    let mut pen_keys: Vec<u64> = fingors.keys().copied().collect();
                    pen_keys.sort();
                    let mut pens: Vec<f32> =
                        pen_keys.iter().map(|id| fingors[id]).flatten().collect();
                    pens.resize(4 * MAX_FINGIES, 0.);
                    let pen_loc = gl.get_uniform_location(hotloader.get_program(draw_kernel), "pens");
                    gl.uniform_4_f32_slice(pen_loc.as_ref(), &pens);
                    // Set screen size
                    let screen_size_loc = gl.get_uniform_location(hotloader.get_program(draw_kernel), "screen_size");
                    let (sx, sy) = screen_size;
                    gl.uniform_2_f32(screen_size_loc.as_ref(), sx, sy);
                    gl.dispatch_compute((WIDTH / LOCAL_SIZE) as _, (HEIGHT / LOCAL_SIZE) as _, 1);

                    // Execute particle kernel
                    gl.use_program(Some(hotloader.get_program(particle_kernel)));
                    // Set read textures
                    gl.active_texture(gl::TEXTURE0);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_u));
                    gl.active_texture(gl::TEXTURE1);
                    gl.bind_texture(gl::TEXTURE_2D, Some(read_v));
                    // Set write textures
                    gl.bind_image_texture(2, write_u, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    gl.bind_image_texture(3, write_v, 0, false, 0, gl::READ_WRITE, gl::R32F);
                    // Set particle buffer
                    gl.bind_buffer_base(gl::SHADER_STORAGE_BUFFER, 4, Some(particle_buffer));
                    // Set dt
                    let dt_loc = gl.get_uniform_location(hotloader.get_program(particle_kernel), "dt");
                    gl.uniform_1_f32(dt_loc.as_ref(), dt.unwrap_or(CLEAR_DT));
                    // Dispatch
                    gl.dispatch_compute(N_PARTICLES as u32, 1, 1);
                    // Memory barrier for vertex shader
                    gl.memory_barrier(gl::SHADER_IMAGE_ACCESS_BARRIER_BIT);

                    // Draw particles
                    gl.clear(gl::COLOR_BUFFER_BIT);
                    gl.use_program(Some(hotloader.get_program(particle_shader)));

                    // Set time
                    let time_loc = gl.get_uniform_location(hotloader.get_program(particle_shader), "u_time");
                    gl.uniform_1_f32(time_loc.as_ref(), time.elapsed().as_secs_f32());

                    let screen_size_loc = gl.get_uniform_location(hotloader.get_program(particle_shader), "screen_size");
                    let (sx, sy) = screen_size;
                    gl.uniform_2_f32(screen_size_loc.as_ref(), sx, sy);
                    gl.bind_vertex_array(Some(particle_vertex_array));
                    gl.draw_arrays(gl::POINTS, 0, N_PARTICLES);

                    //fingors.clear();

                    if dt.is_none() {
                        dt = Some(INITIAL_DT);
                    }

                    window.swap_buffers().unwrap();
                }
                Event::WindowEvent { ref event, .. } => match event {
                    WindowEvent::Resized(physical_size) => {
                        window.resize(*physical_size);
                        screen_size = (physical_size.width as f32, physical_size.height as f32);
                        gl.scissor(
                            0,
                            0,
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
                        gl.viewport(
                            0,
                            0,
                            physical_size.width as i32,
                            physical_size.height as i32,
                        );
                    }
                    WindowEvent::CloseRequested => {
                        //gl.delete_program(particle_shader);
                        //gl.delete_vertex_array(particle_vertex_array);
                        *control_flow = ControlFlow::Exit
                    }
                    WindowEvent::Touch(touch) => match touch.phase {
                        TouchPhase::Moved => {
                            let x = touch.location.x as f32 / screen_size.0;
                            let y = touch.location.y as f32 / screen_size.1;

                            if let Some([vel_x, vel_y, ..]) = fingors.get(&touch.id) {
                                let pen = [x, y, (x - vel_x), (y - vel_y)];
                                fingors.insert(touch.id, pen);
                            } else {
                                fingors.insert(touch.id, [x, y, 0., 0.]);
                            }
                        }
                        TouchPhase::Ended => {
                            fingors.remove(&touch.id);
                        }
                        _ => (),
                    },
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let (Some(key), ElementState::Pressed) = (input.virtual_keycode, input.state) {
                            match key {
                                VirtualKeyCode::Space => dt = None,
                                VirtualKeyCode::Up => dt = dt.map(|dt| dt - UP_DOWN_DELTA_DT),
                                VirtualKeyCode::Down => dt = dt.map(|dt| dt + UP_DOWN_DELTA_DT),
                                VirtualKeyCode::Left => dt = dt.map(|dt| dt - LEFT_RIGHT_DELTA_DT),
                                VirtualKeyCode::Right => dt = dt.map(|dt| dt + LEFT_RIGHT_DELTA_DT),
                                _ => (),
                            }
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        if let Some([x, y, vel_x, vel_y]) = fingors.get_mut(&MOUSE_IDX) {
                            let px = position.x as f32 / screen_size.0;
                            let py = position.y as f32 / screen_size.1;

                            if (*x, *y) != (0., 0.) {
                                *vel_x = px - *x;
                                *vel_y = py - *y;
                            }

                            *x = px;
                            *y = py;
                        }
                    }
                    WindowEvent::MouseInput {
                        state,
                        button: MouseButton::Left,
                        ..
                    } => {
                        match state {
                            ElementState::Pressed => fingors.insert(MOUSE_IDX, [0.; 4]),
                            ElementState::Released => fingors.remove(&MOUSE_IDX),
                        };
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
    shader_sources: &[(u32, PathBuf)],
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

            let text = std::fs::read_to_string(shader_source)?;
            gl.shader_source(shader, &text);
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

type ProgramSpec = Vec<(u32, PathBuf)>;

struct ShaderHotloader {
    /// Maps source path to program index
    path_to_idx: HashMap<PathBuf, usize>,
    /// Programs, in order of index
    programs: Vec<(gl::Program, ProgramSpec)>,
    event_rx: Receiver<notify::Event>,
    watcher: RecommendedWatcher,
}

impl ShaderHotloader {
    pub fn new(root: PathBuf) -> Result<Self> {
        let (tx, event_rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| match res {
            Ok(event) => tx.send(event).unwrap(),
            Err(e) => println!("watch error: {:?}", e),
        })?;

        watcher.watch(&root, RecursiveMode::Recursive)?;

        Ok(Self {
            watcher,
            path_to_idx: HashMap::new(),
            programs: vec![],
            event_rx,
        })
    }

    pub fn add_program(&mut self, gl: &gl::Context, spec: ProgramSpec) -> Result<usize> {
        let idx = self.programs.len();
        for (_, path) in &spec {
            let path = path.canonicalize()?;
            //self.watcher.watch(&path, RecursiveMode::NonRecursive)?;
            self.path_to_idx.insert(path.clone(), idx);
        }

        let program = create_program(gl, &spec)?;
        self.programs.push((program, spec));

        Ok(idx)
    }

    pub fn update(&mut self, gl: &gl::Context) {
        let mut needs_update: HashSet<usize> = HashSet::new();

        for event in self.event_rx.try_iter() {
            for path in event.paths {
                if let Some(idx) = self.path_to_idx.get(&path) {
                    needs_update.insert(*idx);
                }
            }
        }

        for needed_update in needs_update {
            let (old_program, shader_sources) = &mut self.programs[needed_update];
            match create_program(gl, &shader_sources) {
                Err(e) => eprintln!("Error compiling {:?};\n {:#}", shader_sources, e),
                Ok(program) => {
                    println!("Finished recompiling {:?}", shader_sources);
                    *old_program = program;
                }
            }
        }
    }

    pub fn get_program(&self, idx: usize) -> gl::Program {
        let (prog, _) = &self.programs[idx];
        *prog
    }
}
