use std::borrow::Borrow;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use raylib::prelude::*;

use crate::PIN_RADIUS;
use crate::id_salt;
use crate::wire::*;

pub type DependencyGraph = HashMap<usize, Vec<(SocketKind, usize)>>;

// pub struct Evaluator {
//     pub values: HashMap<SocketRef, bool>, // SocketRef -> bool
// }

fn build_dependency_graph(app: &App) -> DependencyGraph {
    let mut deps: DependencyGraph = HashMap::new();

    for edge in &app.edges {
        let Edge {
            from: output,
            to: input,
        } = edge.borrow().0;

        deps.entry(input.node_id).or_default().push((SocketKind::Output, output.node_id));
        deps.entry(output.node_id).or_default().push((SocketKind::Input, input.node_id));
    }

    deps
}

pub struct App {
    // TODO: use a hash map?
    pub nodes: RefCell<Vec<Node>>,
    pub mouse_pos: Vector2,
    pub edges: Vec<RefCell<(Edge, Vector2, Vector2)>>,
    pub ongoing: Option<(Vector2, SocketRef)>,
    pub right_click_window: Cell<Option<Vector2>>,
    pub dependency_graph: DependencyGraph,

    // re-evalutae the graph
    eval: Cell<bool>,
}

impl App {
    pub fn new() -> Self {
        Self {
            ongoing: None,
            eval: false.into(),
            mouse_pos: Vector2::zero(),
            dependency_graph: DependencyGraph::new(),
            edges: vec![],
            right_click_window: None.into(),
            nodes: vec![
                // Node {
                //     id: id_salt(),
                //     position: Vector2::zero().into(),
                //     name: "Sample node1".to_string(),
                //     inputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             kind: SocketKind::Input,
                //             name: "hydrogen".to_string(),
                //             absolute_position: None,
                //         }
                //         .into(),
                //         Socket {
                //             id: id_salt(),
                //             kind: SocketKind::Input,
                //             name: "chlorine".to_string(),
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],

                //     outputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             name: "hydrochloric acid".to_string(),
                //             kind: SocketKind::Output,
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],
                // },
                // Node {
                //     id: id_salt(),
                //     position: Vector2::zero().into(),
                //     name: "Sample node 2".to_string(),
                //     inputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             kind: SocketKind::Input,
                //             name: "hydrochloric acid".to_string(),
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],

                //     outputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             name: "idk what to put here".to_string(),
                //             kind: SocketKind::Output,
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],
                // },
                // Node {
                //     id: id_salt(),
                //     position: Vector2::zero().into(),
                //     name: "hydrogen".to_string(),
                //     inputs: vec![],
                //     outputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             name: "hydrogen".to_string(),
                //             kind: SocketKind::Output,
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],
                // },
                // Node {
                //     id: id_salt(),
                //     position: Vector2::zero().into(),
                //     name: "chlorine".to_string(),
                //     inputs: vec![],
                //     outputs: vec![
                //         Socket {
                //             id: id_salt(),
                //             name: "chlorine".to_string(),
                //             kind: SocketKind::Output,
                //             absolute_position: None,
                //         }
                //         .into(),
                //     ],
                // },
            ]
            .into(),
        }
    }

    pub fn handle_events(&mut self, rl: &mut RaylibHandle) {
        self.mouse_pos = rl.get_mouse_position();

        if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_RIGHT) {
            // remove the wire if right clicked on the pin
            if let Some((node, _)) = self.get_node_and_pin(self.mouse_pos) {
                let id = node.id;
                if let Some((idx, _)) = self.edges.iter().enumerate().find(|(_, i)| {
                    let Edge { from, to } = (*i).borrow().0;
                    from.node_id == id || to.node_id == id
                }) {
                    self.edges.swap_remove(idx);
                }
            } else {
                self.right_click_window
                    .set(match self.right_click_window.get() {
                        None => Some(self.mouse_pos),
                        _ => None,
                    });
            }
        }

        // Wire start
        if self.ongoing.is_none() && rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            if let Some((node, socket)) = self.get_node_and_pin(self.mouse_pos) {
                if !matches!(socket.borrow().kind, SocketKind::Output) {
                    return;
                }

                let ongoing = Some((
                    socket.borrow().absolute_position.unwrap(),
                    SocketRef {
                        node_id: node.id,
                        socket_id: socket.borrow().id,
                    },
                ));

                self.ongoing = ongoing;
            }
        }
        // Wire end
        else if self.ongoing.is_some()
            && rl.is_mouse_button_released(MouseButton::MOUSE_BUTTON_LEFT)
        {
            let to = *self.ongoing.unwrap().1.borrow();
            if let Some((node, socket)) = self.get_node_and_pin(self.mouse_pos) {
                if !matches!(socket.borrow().kind, SocketKind::Input) {
                    return;
                }

                let b = socket.borrow();
                let edge = Edge {
                    from: SocketRef {
                        node_id: node.id,
                        socket_id: b.id,
                    },
                    to,
                };

                let v2 = b.absolute_position.unwrap();
                drop(b);
                let v1 = self.ongoing.unwrap().0;

                if !self.edges.iter().any(|i| (*i).borrow().0 == edge) {
                    self.edges.push((edge, v1, v2).into());
                }

                let dep_graph = build_dependency_graph(self);
                self.dependency_graph = dep_graph;

                self.eval.set(true);
            }

            self.ongoing = None;
        }
    }

    /// last item is the location of center for snapping
    fn get_node_and_pin(&mut self, point: Vector2) -> Option<(&Node, &RefCell<Socket>)> {
        // * 2 for snapping
        let pred = |i: Vector2| (i - point).length_sqr() <= (PIN_RADIUS).powi(2);

        self.nodes.get_mut().iter().find_map(
            |node @ Node {
                 inputs, outputs, ..
             }| {
                inputs
                    .iter()
                    .find(|i| (*i).borrow().absolute_position.map(pred).unwrap_or(false))
                    .or_else(|| {
                        outputs
                            .iter()
                            .find(|o| (*o).borrow().absolute_position.map(pred).unwrap_or(false))
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
            for (idx, node) in self.nodes.borrow().iter().enumerate() {
                let old_pos = *node.position.borrow();

                // will update old_pos if window is moved
                self.render_node(ui, node, idx, old_pos);

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

            if let Some(node_position @ Vector2 { x, y }) = self.right_click_window.get() {
                ui.window("right click window")
                    .title_bar(false)
                    .resizable(false)
                    .always_auto_resize(true)
                    .opened(&mut true)
                    .position([x, y], ::imgui::Condition::Always)
                    .collapsed(false, ::imgui::Condition::Always)
                    .build(|| {
                        let mut clicked = false;

                        NodeKind::list().into_iter().for_each(|nodekind| {
                            if ui.button(nodekind.to_string()) {
                                clicked = true;
                                self.nodes
                                    .borrow_mut()
                                    .push(nodekind.build(node_position, id_salt));
                            }
                        });

                        if clicked {
                            self.right_click_window.set(None);
                        }
                    });
            }
        });
    }

    fn render_node(&self, ui: &mut ::imgui::Ui, node: &Node, idx: usize, old_pos: Vector2) {
        ui.window(&format!("{}  #{idx}", node.name))
            .resizable(false)
            .collapsible(false)
            .position([old_pos.x, old_pos.y], ::imgui::Condition::Appearing)
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

                    match &node.kind {
                        NodeKind::Input(enabled) => {
                            let b = enabled.get() as u8 as f32;
                            if ui.color_button("    ", [b, b, b, 1.0]) {
                                enabled.set(!enabled.get());
                                self.eval.set(true);
                            }
                        }

                        NodeKind::Display(enabled) => {
                            let b = enabled.get() as u8 as f32;
                            if ui.color_button("    ", [b, b, b, 1.0]) {
                                self.eval.set(true);
                            }
                        }

                        _ => {}
                    }

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
    }

    pub fn update(&self) {
        if self.eval.get() {
            println!("rebuilding");
            dbg!(&self.dependency_graph);
            self.nodes.borrow().iter().for_each(|i| {
                i.eval(self, &self.dependency_graph);
            });

            self.eval.set(false);
        }
    }
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
