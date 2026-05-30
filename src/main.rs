use glam::Vec3;
use renderer::datatypes::Vertex;
use renderer::forward_pipeline::PipelineForward;
use renderer::framebuffer::Framebuffer;
use renderer::mesh::Mesh;
use renderer::renderer::Renderer;
use sdl3::keyboard::Keycode;
use sdl3::rect::Rect;
use sdl3::{event::Event, pixels::PixelFormat};
use std::time::{Duration, Instant};

pub fn main() {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("RNTB3DE", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();
    let binding = canvas.texture_creator();
    let mut tex = binding
        .create_texture(
            PixelFormat::XRGB8888,
            sdl3::render::TextureAccess::Streaming,
            800,
            600,
        )
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut second_start = Instant::now();
    let mut frames = 0;

    let mut renderer = Renderer::new();
    let fb_id = renderer.create_framebuffer(800, 600);
    let mut pipeline = PipelineForward::new(
        |v: Vertex<Vec3>| v,
        |f| glam::Vec4::new(f.data.x, f.data.y, f.data.z, 1.0),
    );

    pipeline.attach_render_buffer(fb_id);
    let mut display_buffer = vec![0i32; 800 * 600];
    let mut i = 0;
    'running: loop {
        canvas.clear();
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

        let mesh = Mesh::new(
            vec![
                Vertex::new(
                    glam::Vec3::new(0.0, 0.5, 0.0),
                    Vec3::new((i % 255) as f32 / 255.0, 0.0, 0.0),
                ),
                Vertex::new(glam::Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),
                Vertex::new(glam::Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 1.0)),
            ],
            None,
        );
        renderer.clear_framebuffer(fb_id);
        pipeline.assemble_and_run(&mut renderer, &mesh);
        let fb = renderer.borrow_framebuffer(fb_id);
        buffer_to_u8(fb, &mut display_buffer);
        tex.update(
            Rect::new(0, 0, 800, 600),
            bytemuck::cast_slice(&display_buffer),
            800 * 4,
        )
        .unwrap();

        canvas.copy(&tex, None, None).unwrap();
        canvas.present();
        if second_start.elapsed() >= Duration::new(1, 0) {
            println!("FPS: {}", frames);
            frames = 0;
            second_start = Instant::now();
        }
        frames += 1;
        i += 1;
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

#[inline]
fn buffer_to_u8(buffer: &Framebuffer, out: &mut Vec<i32>) {
    let (ra, ga, ba, _) = buffer.get_color();
    for i in 0..ra.len() {
        let r: i32 = unsafe { (ra[i] * 255.0).to_int_unchecked() };
        let g: i32 = unsafe { (ga[i] * 255.0).to_int_unchecked() };
        let b: i32 = unsafe { (ba[i] * 255.0).to_int_unchecked() };
        let combined = ((r as i32) << 16) + ((g as i32)  << 8) + (b as i32);
        unsafe { *out.get_unchecked_mut(i) = combined };
    }
}
