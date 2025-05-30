use std::cell::RefCell;

use raylib::math::Vector2;

#[derive(Debug, Clone, Copy)]
pub struct SocketRef {
    pub node_id: usize,
    pub socket_id: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub from: SocketRef,
    pub to: SocketRef,
}

#[derive(Debug, Clone, Copy)]
pub enum SocketKind {
    Input,
    Output,
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: usize,
    pub name: String,
    pub position: RefCell<Vector2>,
    pub inputs: Vec<RefCell<Socket>>,
    pub outputs: Vec<RefCell<Socket>>,
}

#[derive(Debug, Clone)]
pub struct Socket {
    pub name: String,
    pub id: usize,
    pub kind: SocketKind,

    /// This is set once `draw_nodes` is called
    pub absolute_position: Option<Vector2>,
}
