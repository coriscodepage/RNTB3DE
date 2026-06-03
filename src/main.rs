use internals::imports::model::{MeshData, Model};
use renderer::datatypes::Vertex;
use renderer::forward_pipeline::PipelineForward;
use renderer::renderer::Renderer;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use std::fs::{self, File};
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
    let mut reader = decoder.read_info().unwrap();
    let mut buf = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut buf).unwrap();
    let binding = buf[..info.buffer_size()]
        .iter()
        .copied()
        .map(|c| c as f32 / 255.0)
        .collect::<Vec<f32>>();
    let texture: &[[f32; 3]] = bytemuck::cast_slice(&binding);

    let mut renderer = Renderer::new();
    let fb_id = renderer.create_framebuffer(WIDTH, HEIGHT);
    let mut pipeline = PipelineForward::new(
        |v: Vertex<MeshData>| v,
        |v| {
            // println!("{}, {}", v.data.texture_uv.x, v.data.texture_uv.y);
            let tex_x =
                unsafe { (v.data.texture_uv.x * info.width as f32).to_int_unchecked::<usize>() };
            let tex_y =
                unsafe { (v.data.texture_uv.y * info.height as f32).to_int_unchecked::<usize>() };
            // println!("{}, {}", tex_x, tex_y);
            let tex_idx = tex_y * 1024 + tex_x;
            let color = texture[tex_idx];
            // let g = texture[tex_idx + 1];
            // let b = texture[tex_idx + 2];
            let [r, g, b] = color;
            glam::vec4(r, g, b, 1.0)
            // glam::Vec4::new(1.0, 1.0, 1.0, 1.0)
        },
    );

    pipeline.attach_render_buffer(fb_id);

    // let mut display_buffer = vec![0i32; WIDTH * HEIGHT];
    // let mut i = 0;

    let file = fs::read_to_string("african_head.obj").unwrap();
    let model = Model::from_obj_string(&file);
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

        // let mesh = Mesh::new(
        //     vec![
        //         Vertex::new(
        //             glam::Vec3::new(0.0, 0.5, 0.0),
        //             Vec3::new((i % 255) as f32 / 255.0, 0.0, 0.0),
        //         ),
        //         Vertex::new(glam::Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.0, 1.0, 0.0)),
        //         Vertex::new(glam::Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        //     ],
        //     None,
        // );
        renderer.clear_framebuffer(fb_id);
        pipeline.assemble_and_run(&mut renderer, &model.mesh);
        let mut win_surf = window.surface(&event_pump).unwrap();
        let pixels = unsafe { win_surf.without_lock_mut().unwrap() };
        renderer.buffer_to_u8(fb_id, bytemuck::cast_slice_mut(pixels));
        win_surf.update_window().unwrap();

        // tex.with_lock(None, |pixels: &mut [u8], _| {
        // buffer_to_u8(fb, bytemuck::cast_slice_mut(pixels));
        // })
        // .unwrap();
        // tex.update(
        //     Rect::new(0, 0, 800, 600),
        //     bytemuck::cast_slice(&display_buffer),
        //     800 * 4,
        // )
        // .unwrap();

        // canvas.copy(&tex, None, None).unwrap();
        // canvas.present();
        if second_start.elapsed() >= Duration::new(1, 0) {
            println!("FPS: {}", frames);
            frames = 0;
            second_start = Instant::now();
        }
        frames += 1;
        // i += 1;
        // ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}
