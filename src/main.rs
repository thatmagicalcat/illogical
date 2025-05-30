use raylib::prelude::*;

mod app;
mod renderer;
mod wire;

use app::App;

pub const PIN_RADIUS: f32 = 10.0;

fn main() {
    let (mut rl, thread) = raylib::init()
        .title("illogical")
        .width(800)
        .height(800)
        .resizable()
        .build();

    rl.set_target_fps(60);

    let mut app = App::new();

    while !rl.window_should_close() {
        app.handle_events(&mut rl);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(Color::BLACK);

        app.draw_imgui(&mut d);
        app.draw(&mut d);
    }
}

pub fn id_salt() -> usize {
    static mut COUNTER: usize = 0;

    unsafe {
        COUNTER += 1;
        COUNTER - 1
    }
}
