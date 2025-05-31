use std::cell::{Cell, RefCell};

use raylib::math::Vector2;

use crate::app::{App, DependencyGraph};

#[derive(Debug, Clone)]
pub enum NodeKind {
    NAnd,
    And,
    Or,
    XOr,
    Not,

    Input(Cell<bool>),
    Display(Cell<bool>),
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                NodeKind::NAnd => "NAND",
                NodeKind::And => "AND",
                NodeKind::Or => "OR",
                NodeKind::XOr => "XOR",
                NodeKind::Not => "NOT",
                NodeKind::Input(_) => "INPUT",
                NodeKind::Display(_) => "DISPLAY",
            }
        )
    }
}

impl NodeKind {
    pub fn list() -> [NodeKind; 7] {
        use NodeKind::*;
        [
            Input(false.into()),
            Display(false.into()),
            NAnd,
            And,
            Not,
            Or,
            XOr,
        ]
    }

    pub fn apply_binary(&self, a: bool, b: bool) -> Option<bool> {
        Some(match self {
            Self::NAnd => !(a & b),
            Self::And => a & b,
            Self::Or => a | b,
            Self::XOr => a ^ b,

            _ => return None,
        })
    }

    pub fn inputs(&self) -> usize {
        match self {
            NodeKind::Input(_) => 0,
            NodeKind::Not | NodeKind::Display(_) => 1,
            NodeKind::NAnd | NodeKind::And | NodeKind::Or | NodeKind::XOr => 2,
        }
    }

    pub fn outputs(&self) -> usize {
        match self {
            NodeKind::Display(_) => 1,
            NodeKind::Not
            | NodeKind::Input(_)
            | NodeKind::NAnd
            | NodeKind::And
            | NodeKind::Or
            | NodeKind::XOr => 1,
        }
    }

    pub fn build<F: FnMut() -> usize>(&self, position: Vector2, mut id_salt: F) -> Node {
        Node {
            id: id_salt(),
            name: self.to_string(),
            position: position.into(),
            kind: self.clone(),
            inputs: (0..self.inputs())
                .map(|i| {
                    Socket {
                        name: format!("i{i}"),
                        id: id_salt(),
                        kind: SocketKind::Input,
                        absolute_position: None,
                    }
                    .into()
                })
                .collect::<Vec<_>>(),
            outputs: (0..self.outputs())
                .map(|i| {
                    Socket {
                        name: format!("o{i}"),
                        id: id_salt(),
                        kind: SocketKind::Output,
                        absolute_position: None,
                    }
                    .into()
                })
                .collect::<Vec<_>>(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketRef {
    pub node_id: usize,
    pub socket_id: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge {
    pub from: SocketRef,
    pub to: SocketRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub kind: NodeKind,
}

impl Node {
    // FIXME: reduce recursion
    pub fn eval(&self, app: &App, dep_graph: &DependencyGraph) -> Option<bool> {
        let connections = dep_graph.get(&self.id)?.as_slice();
        let nodes = app.nodes.borrow();

        println!("{}", self.kind);

        Some(match &self.kind {
            NodeKind::Input(val) => val.get(),
            NodeKind::Not => {
                let a = connections.first()?;
                for node in nodes.iter() {
                    if node.id == a.1 {
                        return Some(!node.eval(app, dep_graph)?);
                    }
                }

                return None;
            }

            NodeKind::NAnd | NodeKind::And | NodeKind::Or | NodeKind::XOr => {
                let mut iter = connections
                    .iter()
                    .filter(|&&(kind, id)| kind == SocketKind::Input);

                let a = iter.next()?.1;
                let b = iter.next()?.1;

                let [mut a_node, mut b_node] = [None; 2];
                for node in nodes.iter() {
                    if node.id == a {
                        a_node = Some(node);
                    } else if node.id == b {
                        b_node = Some(node);
                    }
                }

                self.kind
                    .apply_binary(a_node?.eval(app, dep_graph)?, b_node?.eval(app, dep_graph)?)?
            }

            NodeKind::Display(output) => {
                let a = connections
                    .iter()
                    .find(|&&(kind, id)| kind == SocketKind::Input)?;
                println!("eval output");

                let mut b = None;
                for node in nodes.iter() {
                    if node.id == a.1 {
                        println!("eval {} for output", node.kind);
                        b = Some(node.eval(app, dep_graph)?);
                        output.set(b.unwrap());
                        break;
                    }
                }

                b?
            }
        })
    }
}

#[derive(Debug, Clone)]
pub struct Socket {
    pub name: String,
    pub id: usize,
    pub kind: SocketKind,

    /// This is set once `draw_nodes` is called
    pub absolute_position: Option<Vector2>,
}
