mod presentation;
use core::f32;
use internals::imports::model::{MeshData, Model};
use renderer::abstraction::context::Context;
use renderer::abstraction::program::Program;
use renderer::datatypes::Vertex;
use renderer::forward_pipeline::PipelineForward;
use renderer::renderer::Renderer;
use renderer::texture::Texture;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::cell::RefCell;
use std::fs::{self};
use std::sync::Arc;
use std::sync::atomic::AtomicI32;
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

    let render_present = presentation.clone();
    thread::spawn(move || {
        let mut frames_r = 0;
        let i = AtomicI32::new(0);

        let texture = Texture::from_png("african_head_diffuse.png");

        let texture2 = Texture::from_png("neferiti_deffuse.png");

        let mut renderer = Renderer::new();

        let tex_id_1 = renderer.insert_texture(texture);
        let tex_id_2 = renderer.insert_texture(texture2);

        let fb_id = renderer.create_framebuffer(WIDTH, HEIGHT);

        let program = Program::new(
            |mut v: Vertex<MeshData>| {
                let i = i.load(std::sync::atomic::Ordering::Relaxed);
                let a: f32 = f32::consts::PI / 180.0 * (i % 360) as f32;
                let x_axis = glam::vec3(a.cos(), 0.0, a.sin());
                let y_axis = glam::vec3(0.0, 1.0, 0.0);
                let z_axis = glam::vec3(-a.sin(), 0.0, a.cos());
                let rot = glam::mat3(x_axis, y_axis, z_axis);

                let trans = glam::Mat4::from_translation(glam::vec3(-0.2, 0.0, 0.0));
                let p = glam::vec3(
                    v.position.x / (16.0 / 9.0),
                    v.position.y,
                    v.position.z / (16.0 / 9.0),
                );
                let v3 = 1.0 * rot * p;
                let v4 = trans * glam::vec4(v3.x, v3.y, v3.z, 1.0);
                v.position = glam::vec3(v4.x, v4.y, v4.z);
                v
            },
            &[|v: &renderer::datatypes::FragmentInput<MeshData>, ctx| {
                // let color = ctx.sample_texture(0, v.data.texture_uv.x, v.data.texture_uv.y);
                // let color = texture.sample(v.data.texture_uv.x, v.data.texture_uv.y);
                let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
                color
            }],
        );

        let program2 = Program::new(
            |mut v: Vertex<MeshData>| {
                let i = i.load(std::sync::atomic::Ordering::Relaxed);
                let a: f32 = f32::consts::PI / 180.0 * (i % 360) as f32;
                let x_axis = glam::vec3(a.cos(), 0.0, a.sin());
                let y_axis = glam::vec3(0.0, 1.0, 0.0);
                let z_axis = glam::vec3(-a.sin(), 0.0, a.cos());
                let rot = glam::mat3(x_axis, y_axis, z_axis);

                let trans = glam::Mat4::from_translation(glam::vec3(0.5, 0.2, 0.0));
                let p = glam::vec3(
                    v.position.x / (16.0 / 9.0),
                    v.position.y,
                    v.position.z / (16.0 / 9.0),
                );
                let v3 = 0.4 * rot * p;
                let v4 = trans * glam::vec4(v3.x, v3.y, v3.z, 1.0);
                v.position = glam::vec3(v4.x, v4.y, v4.z);
                v
            },
            &[|v: &renderer::datatypes::FragmentInput<MeshData>, ctx| {
                // let color =ctx.sample_texture_fail_silent(1, v.data.texture_uv.x, v.data.texture_uv.y);
                // let color = texture2.sample_fail_silent(v.data.texture_uv.x, v.data.texture_uv.y);
                let color = glam::vec4(1.0, 1.0, 1.0, 1.0);
                color
            }],
        );
        let mut pipeline = PipelineForward::new();

        let renderer = RefCell::new(renderer);
        let file = fs::read_to_string("african_head.obj").unwrap();
        let model1 = Model::from_obj_string(&file);
        let file = fs::read_to_string("blam2.obj").unwrap();
        let model2 = Model::from_obj_string(&file);

        let mut context = Context::new(&renderer);
        context.bind_framebuffers_write(fb_id).unwrap();
        context.bind_texture(tex_id_1).unwrap();
        context.bind_texture(tex_id_2).unwrap();
        loop {
            renderer.borrow_mut().clear_framebuffer(fb_id);

            pipeline.assemble_and_run(&mut context, &program, &model1.mesh);
            pipeline.assemble_and_run(&mut context, &program2, &model2.mesh);

            render_present.write(renderer.borrow_mut().borrow_framebuffer_mut(fb_id));

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
        // renderer
        //     .borrow()
        //     .buffer_to_u8(fb_id, bytemuck::cast_slice_mut(pixels));
        win_surf.update_window().unwrap();

        // if second_start.elapsed() >= Duration::new(1, 0) {
        //     println!("FPS: {}", frames);
        //     frames = 0;
        //     second_start = Instant::now();
        // }
        // frames += 1;
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

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
