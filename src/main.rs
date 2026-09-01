mod presentation;
use core::f32;
use internals::dag::render_command::{Renderer, RendererHandle};
use internals::imports::model::{MeshData, Model};
use internals::samplers::context::{RequestedSamplers, with_acquired};
use internals::samplers::texture::{TextureSrc, TextureStorage};
use internals::world::World;
use internals::world::material::{Material, MaterialHandle, StdProgram};
use internals::world::transform::Transform;
use renderer::abstraction::camera::Camera;
use renderer::abstraction::context::{RequestedWriters, TextureUnit};
use renderer::abstraction::program::{Program, ProgramStorage};
use renderer::datatypes::{Vertex, VertexHomogenous};
use renderer::forward_pipeline::PipelineForward;
use renderer::framebuffer_storage::FramebufferStore;
use renderer::texture::Texture;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::cell::RefCell;
use std::fs::{self};
use std::sync::Arc;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::mpsc::channel;
use std::thread;
use std::time::{Duration, Instant};

use crate::presentation::PresentationBuffer;

static WIDTH: usize = 1920;
static HEIGHT: usize = 1080;

pub fn main() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("RNTB3DE", WIDTH as u32, HEIGHT as u32)
        .build()
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut second_start = Instant::now();
    // let mut frames = 0;

    let presentation = Arc::new(PresentationBuffer::new(WIDTH, HEIGHT));
    // let mut texture_store = TextureStorage::new();

    let source1 = TextureSrc::Png("african_head_diffuse.png");
    // texture_store.add(source1);

    let render_present = presentation.clone();
    let mut world = World::new();
    let (tx, rx) = channel();
    let mut render_handle = RendererHandle::new(tx);
    let fb_id = render_handle.create_framebuffer(WIDTH, HEIGHT);
    render_handle.create_texture(source1);
    let file = fs::read_to_string("african_head.obj").unwrap();
    let model1 = Model::from_obj_string(&file);
    let transform1 = Transform::new(
        glam::vec3(0.0, 0.0, 0.0),
        glam::quat(0.0, 0.0, 0.0, 0.0),
        glam::vec3(1.0, 1.0, 1.0),
    );

    let transform2 = Transform::new(
        glam::vec3(0.8, 0.0, 0.0),
        glam::quat(0.0, 0.0, 0.0, 0.0),
        glam::vec3(1.0, 1.0, 1.0),
    );

    let i = Arc::new(AtomicI32::new(0));
    let l_i = i.clone();
    let program = Program::new(
        move |mut v: Vertex<MeshData>, uni: &(glam::Mat4, glam::Mat4, glam::Mat4)| {
            // let i = l_i.load(std::sync::atomic::Ordering::Relaxed);

            let &(proj, view, model) = uni;
            v.position = (proj * view * model * v.position.extend(1.0)).truncate();

            VertexHomogenous::from_vertex(v, 1.0)
        },
        &[
            move |v: &renderer::datatypes::FragmentInput<MeshData>, ctx, _: &()| {
                // let i = l_i.load(std::sync::atomic::Ordering::Relaxed);
                let color =
                ctx.sample_texture(TextureUnit(0), v.data.texture_uv.x, v.data.texture_uv.y);
                // let color = texture.sample(v.data.texture_uv.x, v.data.texture_uv.y);
                // let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
                color
            },
        ],
    );

    // let mut program_store = ProgramStorage::new();
    // let handle = program_store.put(program);
    let handle = render_handle.create_program(program);

    let mut samplers = RequestedSamplers::new();
    samplers.bind_texture(source1).unwrap();
    let mut writers = RequestedWriters::new();
    writers.bind_framebuffers_write(fb_id).unwrap();

    let material = Material::new(handle, samplers, writers);
    let material_handle = render_handle.create_material(material);

    world.place_model_with_transform(model1.clone(), transform1, material_handle);

    world.place_model_with_transform(model1, transform2, material_handle);

    thread::spawn(move || {
        let mut frames_r = 0;

        // let mut framebuffer_store = FramebufferStore::new();
        let mut renderer = Renderer::new(rx);

        let view = Camera::new(
            glam::Vec3::new(0.0, 0.0, 5.0),
            glam::Vec3::ZERO,
            glam::Vec3::Y,
        );

        let proj: glam::prelude::Mat4 =
            glam::camera::rh::proj::opengl::perspective(75f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);

        // let texture = internals::samplers::texture::from_png("african_head_diffuse.png");
        // let tex_id_1 = renderer.texture_store.insert_texture(texture);

        let mut pipeline = PipelineForward::new();

        loop {
            renderer.poll_commands();
            renderer.framebuffer_store.clear_framebuffer(fb_id);

            world.with_world(|m| {
                for (model, transform, &material_handle) in m
                    .query::<(&Model<MeshData>, &Transform, &MaterialHandle)>()
                    .iter()
                {
                    let material = renderer.material_store.get(material_handle);
                    with_acquired(
                        &mut renderer.texture_store,
                        &mut renderer.framebuffer_store,
                        material.get_samplers(),
                        material.get_writers(),
                        |samplers_resolved, writers_resolved| {
                            let program = renderer.program_store.get(material.get_handle());
                            program.run_forward(
                                &mut pipeline,
                                samplers_resolved,
                                writers_resolved,
                                (proj, view.to_mat4(), transform.to_mat4()),
                                (),
                                &model.mesh,
                            );
                        },
                    );
                }
            });

            render_present.write(renderer.framebuffer_store.borrow_framebuffer_mut(fb_id));

            if second_start.elapsed() >= Duration::new(1, 0) {
                println!("render FPS: {}", frames_r);
                frames_r = 0;
                second_start = Instant::now();
            }
            frames_r += 1;

            i.store(
                i.load(std::sync::atomic::Ordering::Relaxed) + 1,
                std::sync::atomic::Ordering::Relaxed,
            );
            // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
        }
    });

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    break 'running;
                }
                _ => {}
            }
        }

        let mut win_surf = window.surface(&event_pump).unwrap();
        let pixels = unsafe { win_surf.without_lock_mut().unwrap() };
        presentation.read(|framebuffer| framebuffer.buffer_to_u8(bytemuck::cast_slice_mut(pixels)));
        win_surf.update_window().unwrap();
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
    }
}

// pub fn main() {
//     let sdl_context = sdl3::init().unwrap();
//     let video_subsystem = sdl_context.video().unwrap();
//     let window = video_subsystem
//         .window("RNTB3DE", WIDTH as u32, HEIGHT as u32)
//         .build()
//         .unwrap();
//     let mut event_pump = sdl_context.event_pump().unwrap();

//     let mut second_start = Instant::now();
//     // let mut frames = 0;

//     let presentation = Arc::new(PresentationBuffer::new(WIDTH, HEIGHT));

//     let render_present = presentation.clone();
//     thread::spawn(move || {
//         let mut frames_r = 0;
//         let i = AtomicI32::new(0);

//         let texture = internals::samplers::texture::from_png("african_head_diffuse.png");

//         let texture2 = internals::samplers::texture::from_png("neferiti_deffuse.png");

//         let mut renderer = Renderer::new();

//         let tex_id_1 = renderer.texture_store.insert_texture(texture);
//         let tex_id_2 = renderer.texture_store.insert_texture(texture2);

//         let fb_id = renderer.framebuffer_store.create_framebuffer(WIDTH, HEIGHT);

//         let view = glam::camera::rh::view::look_at_mat4(
//             glam::Vec3::new(0.0, 2.0, 5.0), // eye
//             glam::Vec3::ZERO,               // target
//             glam::Vec3::Y,                  // up
//         );

//         let proj = glam::camera::rh::proj::opengl::perspective(60f32.to_radians(), 16.0 / 9.0, 0.1, 100.0);

//         let program = Program::new(
//             |mut v: Vertex<MeshData>| {
//                 let i = i.load(std::sync::atomic::Ordering::Relaxed);
//                 let s = 0.5;
//                 let a: f32 = f32::consts::PI / 180.0 * (i % 360) as f32;
//                 let x_axis = glam::vec3(a.cos(), 0.0, a.sin());
//                 let y_axis = glam::vec3(0.0, 1.0, 0.0);
//                 let z_axis = glam::vec3(-a.sin(), 0.0, a.cos());
//                 let rot = glam::Quat::from_rotation_axes(x_axis, y_axis, z_axis);
//                 let model = glam::Mat4::from_scale_rotation_translation(
//                     glam::vec3(s,s,s),
//                     rot,
//                     glam::vec3(-0.4, 0.0, 0.0),
//                 );
//                 v.position = (proj * view * model * v.position.extend(1.0)).truncate();

//                 VertexHomogenous::from_vertex(v, 1.0)
//             },
//             &[|v: &renderer::datatypes::FragmentInput<MeshData>, ctx| {
//                 let color =
//                     ctx.sample_texture(TextureUnit(0), v.data.texture_uv.x, v.data.texture_uv.y);
//                 // let color = texture.sample(v.data.texture_uv.x, v.data.texture_uv.y);
//                 // let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
//                 color
//             }],
//         );

//         let program2 = Program::new(
//             |mut v: Vertex<MeshData>| {
//                 let i = i.load(std::sync::atomic::Ordering::Relaxed);
//                 let s = 0.25;
//                 let a: f32 = f32::consts::PI / 180.0 * (i % 360) as f32;
//                 let x_axis = glam::vec3(a.cos(), 0.0, a.sin());
//                 let y_axis = glam::vec3(0.0, 1.0, 0.0);
//                 let z_axis = glam::vec3(-a.sin(), 0.0, a.cos());
//                 let rot = glam::Quat::from_rotation_axes(x_axis, y_axis, z_axis);
//                 let model = glam::Mat4::from_scale_rotation_translation(
//                     glam::vec3(s,s,s),
//                     rot,
//                     glam::vec3(0.4, 0.0, 0.0),
//                 );

//                 v.position = (proj * view * model * v.position.extend(1.0)).truncate();

//                 VertexHomogenous::from_vertex(v, 1.0)
//             },
//             &[|v: &renderer::datatypes::FragmentInput<MeshData>, ctx| {
//                 let color = ctx.sample_texture_fail_silent(
//                     TextureUnit(1),
//                     v.data.texture_uv.x,
//                     v.data.texture_uv.y,
//                 );
//                 // let color = texture2.sample_fail_silent(v.data.texture_uv.x, v.data.texture_uv.y);
//                 // let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
//                 color
//             }],
//         );
//         let mut pipeline = PipelineForward::new();

//         let file = fs::read_to_string("african_head.obj").unwrap();
//         let model1 = Model::from_obj_string(&file);
//         let file = fs::read_to_string("blam2.obj").unwrap();
//         let model2 = Model::from_obj_string(&file);
//         let mut samplers = RequestedSamplers::new();
//         samplers.bind_texture(tex_id_1).unwrap();
//         samplers.bind_texture(tex_id_2).unwrap();
//         let mut writers = RequestedWriters::new();
//         writers.bind_framebuffers_write(fb_id).unwrap();
//         loop {
//             renderer.framebuffer_store.clear_framebuffer(fb_id);

//             with_acquired(
//                 &renderer.texture_store,
//                 &mut renderer.framebuffer_store,
//                 &samplers,
//                 &writers,
//                 |samplers_resolved, writers_resolved| {
//                     pipeline.assemble_and_run(
//                         samplers_resolved,
//                         writers_resolved,
//                         &program,
//                         &model1.mesh,
//                     );
//                     pipeline.assemble_and_run(
//                         samplers_resolved,
//                         writers_resolved,
//                         &program2,
//                         &model2.mesh,
//                     );
//                 },
//             );

//             // pipeline.run_pixel(&mut context, |p, context| {
//             //     let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
//             //     color
//             // });

//             render_present.write(renderer.framebuffer_store.borrow_framebuffer_mut(fb_id));

//             if second_start.elapsed() >= Duration::new(1, 0) {
//                 println!("render FPS: {}", frames_r);
//                 frames_r = 0;
//                 second_start = Instant::now();
//             }
//             frames_r += 1;

//             i.store(
//                 i.load(std::sync::atomic::Ordering::Relaxed) + 1,
//                 std::sync::atomic::Ordering::Relaxed,
//             );
//             // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
//         }
//     });

//     'running: loop {
//         for event in event_pump.poll_iter() {
//             match event {
//                 Event::Quit { .. }
//                 | Event::KeyDown {
//                     keycode: Some(Keycode::Escape),
//                     ..
//                 } => {
//                     break 'running;
//                 }
//                 _ => {}
//             }
//         }

//         let mut win_surf = window.surface(&event_pump).unwrap();
//         let pixels = unsafe { win_surf.without_lock_mut().unwrap() };
//         presentation.read(|framebuffer| framebuffer.buffer_to_u8(bytemuck::cast_slice_mut(pixels)));
//         win_surf.update_window().unwrap();

//         // if second_start.elapsed() >= Duration::new(1, 0) {
//         //     println!("FPS: {}", frames);
//         //     frames = 0;
//         //     second_start = Instant::now();
//         // }
//         // frames += 1;
//         ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 120));
//     }
// }

fn hsv_to_rgba(h: f32, s: f32, v: f32) -> glam::Vec4 {
    let c = v * s;
    let x = c * (1.0 - (((h / 60.0) % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h {
        h if h < 60.0 => (c, x, 0.0),
        h if h < 120.0 => (x, c, 0.0),
        h if h < 180.0 => (0.0, c, x),
        h if h < 240.0 => (0.0, x, c),
        h if h < 300.0 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    glam::vec4(r + m, g + m, b + m, 1.0)
}
