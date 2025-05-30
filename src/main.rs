use std::borrow::{Borrow, BorrowMut};
use std::cell::RefCell;
use std::sync::RwLock;

use glam::{Vec2, vec2};
use raylib::prelude::*;

const PIN_RADIUS: f32 = 6.0;

fn id_salt() -> usize {
    static mut COUNTER: usize = 0;

    unsafe {
        COUNTER += 1;
        COUNTER - 1
    }
}

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

#[derive(Debug, Clone, Copy)]
pub struct SocketRef {
    node_id: usize,
    socket_id: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    from: SocketRef,
    to: SocketRef,
}

#[derive(Debug, Clone, Copy)]
pub enum SocketKind {
    Input,
    Output,
}

#[derive(Debug, Clone)]
pub struct Node {
    id: usize,
    name: String,
    position: RefCell<Vector2>,
    inputs: Vec<RefCell<Socket>>,
    outputs: Vec<RefCell<Socket>>,
}

#[derive(Debug, Clone)]
pub struct Socket {
    name: String,
    id: usize,
    kind: SocketKind,

    /// This is set once `draw_nodes` is called
    absolute_position: Option<Vector2>,
}

struct Bezier {
    p0: Vector2,
    p3: Vector2,
}

impl Bezier {
    pub fn draw(&self, thickness: f32, color: Color, d: &mut RaylibDrawHandle) {
        let p1 = Vector2::new(self.p3.x, self.p0.y);
        let p2 = Vector2::new(self.p0.x, self.p3.y);

        d.draw_spline_bezier_cubic(&[self.p0, p1, p2, self.p3], thickness, color);
    }
}

struct App {
    nodes: Vec<Node>,
    mouse_pos: Vector2,
    edges: Vec<RefCell<(Edge, Vector2, Vector2)>>,
    ongoing: Option<(Vector2, SocketRef)>,
    right_click_window: Option<Vector2>,
}

impl App {
    pub fn new() -> Self {
        Self {
            ongoing: None,
            mouse_pos: Vector2::zero(),
            edges: vec![],
            right_click_window: None,
            nodes: vec![
                Node {
                    id: id_salt(),
                    position: Vector2::zero().into(),
                    name: "Sample node1".to_string(),
                    inputs: vec![
                        Socket {
                            id: id_salt(),
                            kind: SocketKind::Input,
                            name: "hydrogen".to_string(),
                            absolute_position: None,
                        }
                        .into(),
                        Socket {
                            id: id_salt(),
                            kind: SocketKind::Input,
                            name: "chlorine".to_string(),
                            absolute_position: None,
                        }
                        .into(),
                    ],

                    outputs: vec![
                        Socket {
                            id: id_salt(),
                            name: "hydrochloric acid".to_string(),
                            kind: SocketKind::Output,
                            absolute_position: None,
                        }
                        .into(),
                    ],
                },
                Node {
                    id: id_salt(),
                    position: Vector2::zero().into(),
                    name: "Sample node 2".to_string(),
                    inputs: vec![
                        Socket {
                            id: id_salt(),
                            kind: SocketKind::Input,
                            name: "hydrochloric acid".to_string(),
                            absolute_position: None,
                        }
                        .into(),
                    ],

                    outputs: vec![
                        Socket {
                            id: id_salt(),
                            name: "idk what to put here".to_string(),
                            kind: SocketKind::Output,
                            absolute_position: None,
                        }
                        .into(),
                    ],
                },
                Node {
                    id: id_salt(),
                    position: Vector2::zero().into(),
                    name: "hydrogen".to_string(),
                    inputs: vec![],
                    outputs: vec![
                        Socket {
                            id: id_salt(),
                            name: "hydrogen".to_string(),
                            kind: SocketKind::Output,
                            absolute_position: None,
                        }
                        .into(),
                    ],
                },
                Node {
                    id: id_salt(),
                    position: Vector2::zero().into(),
                    name: "chlorine".to_string(),
                    inputs: vec![],
                    outputs: vec![
                        Socket {
                            id: id_salt(),
                            name: "chlorine".to_string(),
                            kind: SocketKind::Output,
                            absolute_position: None,
                        }
                        .into(),
                    ],
                },
            ],
        }
    }

    pub fn handle_events(&mut self, rl: &mut RaylibHandle) {
        self.mouse_pos = rl.get_mouse_position();

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
            self.right_click_window = match self.right_click_window {
                None => Some(self.mouse_pos),
                _ => None,
            };
        }

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT)
            && self.right_click_window.is_some()
        {
            self.right_click_window = None;
        }

        if self.ongoing.is_none() && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some((node, socket)) = self.get_node_and_pin(self.mouse_pos) {
                if !matches!(socket.borrow().kind, SocketKind::Output) {
                    return;
                }

                let ongoing = Some((
                    self.mouse_pos,
                    SocketRef {
                        node_id: node.id,
                        socket_id: socket.borrow().id,
                    },
                ));

                self.ongoing = ongoing;
            }
        } else if self.ongoing.is_some()
            && rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT)
        {
            if let Some((node, socket)) = self.get_node_and_pin(self.mouse_pos) {
                if !matches!(socket.borrow().kind, SocketKind::Input) {
                    return;
                }

                let edge = Edge {
                    from: SocketRef {
                        node_id: node.id,
                        socket_id: socket.borrow().id,
                    },
                    to: *self.ongoing.unwrap().1.borrow(),
                };

                let v1 = self.ongoing.unwrap().0;
                let v2 = self.mouse_pos;

                self.edges.push((edge, v1, v2).into());
            }

            self.ongoing = None;
        }
    }

    fn get_node_and_pin(&self, point: Vector2) -> Option<(&Node, &RefCell<Socket>)> {
        self.nodes.iter().find_map(
            |node @ Node {
                 inputs, outputs, ..
             }| {
                inputs
                    .iter()
                    .find(|i| {
                        (*i).borrow()
                            .absolute_position
                            .map(|i: Vector2| {
                                (i - self.mouse_pos).length_sqr() <= PIN_RADIUS * PIN_RADIUS
                            })
                            .unwrap_or(false)
                    })
                    .or_else(|| {
                        outputs.iter().find(|o| {
                            (*o).borrow()
                                .absolute_position
                                .map(|i: Vector2| {
                                    (i - self.mouse_pos).length_sqr() <= PIN_RADIUS * PIN_RADIUS
                                })
                                .unwrap_or(false)
                        })
                    })
                    .map(|i| (node, i))
            },
        )
    }

    pub fn draw(&self, d: &mut RaylibDrawHandle) {
        for i in self.edges.iter() {
            let (_, p0, p3) = *i.borrow();
            Bezier { p0, p3 }.draw(2.0, Color::WHITE, d);
        }

        if let Some((p0, _)) = self.ongoing {
            let p3 = self.mouse_pos;
            Bezier { p0, p3 }.draw(2.0, Color::WHITE, d);
        }
    }

    pub fn draw_imgui(&mut self, d: &mut RaylibDrawHandle) {
        d.draw_imgui(|ui| {
            for node in &self.nodes {
                let old_pos = *node.position.borrow();
                ui.window(&node.name)
                    .resizable(false)
                    .collapsible(false)
                    .always_auto_resize(true)
                    .build(|| {
                        let [x, y] = ui.window_pos();
                        *node.position.borrow_mut() = Vector2::new(x, y);

                        let mut input_socket_iterator = node.inputs.iter();
                        let mut output_socket_iterator = node.outputs.iter();

                        loop {
                            let input = input_socket_iterator.next();
                            let output = output_socket_iterator.next();

                            if input.is_none() && output.is_none() {
                                break;
                            }

                            let dl = ui.get_window_draw_list();
                            let button_size = [PIN_RADIUS * 2.0, PIN_RADIUS * 2.0];

                            // TODO: Event handling

                            if let Some(i) = input {
                                let i_borrow = i.borrow();
                                ui.invisible_button(&i_borrow.name, button_size);

                                let mut pin_center = ui.item_rect_min();
                                pin_center[0] += ui.item_rect_size()[0] / 2.0;
                                pin_center[1] += ui.item_rect_size()[1] / 2.0;

                                dl.add_circle(pin_center, PIN_RADIUS, (1.0, 1.0, 0.0))
                                    .build();

                                ui.same_line();
                                ui.text(&i_borrow.name);

                                drop(i_borrow);
                                i.borrow_mut().absolute_position =
                                    Some(Vector2::new(pin_center[0], pin_center[1]));
                            }

                            if let Some(o) = output {
                                let o_borrow = o.borrow();
                                ui.same_line();
                                ui.text(format!("     {}", o_borrow.name));
                                ui.same_line();

                                ui.invisible_button(&o_borrow.name, button_size);

                                let mut pin_center = ui.item_rect_min();
                                pin_center[0] += ui.item_rect_size()[0] / 2.0;
                                pin_center[1] += ui.item_rect_size()[1] / 2.0;

                                dl.add_circle(pin_center, PIN_RADIUS, (1.0, 1.0, 0.0))
                                    .build();

                                drop(o_borrow);
                                o.borrow_mut().absolute_position =
                                    Some(Vector2::new(pin_center[0], pin_center[1]));
                            }
                        }
                    });

                let new_pos = *node.position.borrow();
                let node_id = node.id;
                if new_pos != old_pos {
                    let displacement = new_pos - old_pos;
                    self.edges.iter().for_each(|i| {
                        let mut b = i.borrow_mut();
                        let Edge {
                            from:
                                SocketRef {
                                    node_id: node_id2, ..
                                },
                            to:
                                SocketRef {
                                    node_id: node_id1, ..
                                },
                        } = b.0;

                        if node_id1 == node_id {
                            b.1 += displacement;
                        } else if node_id2 == node_id {
                            b.2 += displacement;
                        }
                    });
                };
            }

            if let Some(Vector2 { x, y }) = self.right_click_window {
                ui.window("right click window")
                    .title_bar(false)
                    .resizable(false)
                    .always_auto_resize(true)
                    .opened(&mut true)
                    .position([x, y], ::imgui::Condition::Always)
                    .collapsed(false, ::imgui::Condition::Always)
                    .build(|| {
                        if ui.button("button 1")
                            | ui.button("button 2")
                            | ui.button("button 3")
                            | ui.button("button 4")
                            | ui.button("button 5")
                            | ui.button("button 6")
                        {
                            println!("pressed");
                        }
                    });
            }
        });
    }
}

// fn generate_bezier_points(control_points: &[Vector2], segments: usize) -> Vec<Vector2> {
//     let mut points = Vec::with_capacity(segments + 1);
//     for i in 0..=segments {
//         let t = i as f32 / segments as f32;
//         let point = de_casteljau(control_points, t);
//         points.push(point);
//     }

//     points
// }

// fn de_casteljau(points: &[Vector2], t: f32) -> Vector2 {
//     if points.len() == 1 {
//         return points[0];
//     }

//     let mut next = Vec::with_capacity(points.len() - 1);
//     for i in 0..points.len() - 1 {
//         let x = (1.0 - t) * points[i].x + t * points[i + 1].x;
//         let y = (1.0 - t) * points[i].y + t * points[i + 1].y;
//         next.push(Vector2 { x, y });
//     }

//     de_casteljau(&next, t)
// }
