use core::f32;
use internals::imports::model::{MeshData, Model};
use renderer::abstraction::program::Program;
use renderer::dag::*;
use renderer::datatypes::Vertex;
use renderer::forward_pipeline::PipelineForward;
use renderer::renderer::Renderer;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::fs::{self, File};
use std::sync::atomic::AtomicI32;
use std::time::{Duration, Instant};

static WIDTH: usize = 800;
static HEIGHT: usize = 600;

pub fn main() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("RNTB3DE", WIDTH as u32, HEIGHT as u32)
        .build()
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    let mut second_start = Instant::now();
    let mut frames = 0;

    let decoder = png::Decoder::new(std::io::BufReader::new(
        File::open("african_head_diffuse.png").unwrap(),
    ));

    let i = AtomicI32::new(0);

    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let binding = buf[..info.buffer_size()]
        .iter()
        .copied()
        .map(|c| c as f32 / 255.0)
        .collect::<Vec<f32>>();
    let texture: &[[f32; 3]] = bytemuck::cast_slice(&binding);

    let decoder2 = png::Decoder::new(std::io::BufReader::new(
        File::open("neferiti_deffuse.png").unwrap(),
    ));

    let mut reader2 = decoder2.read_info().unwrap();
    let mut buf2 = vec![0; reader2.output_buffer_size().unwrap()];
    let info2 = reader2.next_frame(&mut buf2).unwrap();
    let binding2 = buf2[..info2.buffer_size()]
        .iter()
        .copied()
        .map(|c| c as f32 / 255.0)
        .collect::<Vec<f32>>();
    let texture2: &[[f32; 3]] = bytemuck::cast_slice(&binding2);

    let mut renderer = Renderer::new();
    let fb_id = renderer.create_framebuffer(WIDTH, HEIGHT);
    let program = Program::new(
        |mut v: Vertex<MeshData>| {
            // let i = i.load(std::sync::atomic::Ordering::Relaxed);
            // let a: f32 = f32::consts::PI / 180.0 * (i % 360) as f32;
            // let x_axis = glam::vec3(a.cos(), 0.0, a.sin());
            // let y_axis = glam::vec3(0.0, 1.0, 0.0);
            // let z_axis = glam::vec3(-a.sin(), 0.0, a.cos());
            // let rot = glam::mat3(x_axis, y_axis, z_axis);

            // let trans = glam::Mat4::from_translation(glam::vec3(0.0, 0.0, 0.0));
            // let p = glam::vec3(
            //     v.position.x / (16.0 / 9.0),
            //     v.position.y,
            //     v.position.z / (16.0 / 9.0),
            // );
            // let v3 = 0.8 * rot * p;
            // let v4 = trans * glam::vec4(v3.x, v3.y, v3.z, 1.0);
            // v.position = glam::vec3(v4.x, v4.y, v4.z);
            v
        },
        &[|v: &renderer::datatypes::FragmentInput<MeshData>| {
            let tex_x =
                unsafe { (v.data.texture_uv.x * info.width as f32).to_int_unchecked::<usize>() };
            let tex_y =
                unsafe { (v.data.texture_uv.y * info.height as f32).to_int_unchecked::<usize>() };
            let tex_idx = tex_y * info.width as usize + tex_x;
            let color = texture[tex_idx];
            let [r, g, b] = color;
            glam::vec4(r, g, b, 1.0)
        }],
    );
    let mut pipeline = PipelineForward::new(program);

    pipeline.attach_render_buffer(fb_id);

    let file = fs::read_to_string("african_head.obj").unwrap();
    let model1 = Model::from_obj_string(&file);
    let file = fs::read_to_string("teapot.obj").unwrap();
    let model2 = Model::from_obj_string(&file);
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

        renderer.clear_framebuffer(fb_id);
        pipeline.assemble_and_run(&mut renderer, &model1.mesh);
        // pipeline2.assemble_and_run(&mut renderer, &model2.mesh);
        let mut win_surf = window.surface(&event_pump).unwrap();
        let pixels = unsafe { win_surf.without_lock_mut().unwrap() };
        renderer.buffer_to_u8(fb_id, bytemuck::cast_slice_mut(pixels));
        win_surf.update_window().unwrap();

        if second_start.elapsed() >= Duration::new(1, 0) {
            println!("FPS: {}", frames);
            frames = 0;
            second_start = Instant::now();
        }
        frames += 1;
        i.store(
            i.load(std::sync::atomic::Ordering::Relaxed) + 1,
            std::sync::atomic::Ordering::Relaxed,
        );
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
