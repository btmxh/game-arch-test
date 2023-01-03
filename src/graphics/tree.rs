use std::collections::HashMap;

use crate::exec::server::draw::{self, DrawCallback};

use super::GfxHandle;

pub struct DrawTreeNode {
    callback: Option<Box<DrawCallback>>,
    children: Vec<u64>,
}

#[derive(Default)]
pub struct DrawTree {
    nodes: HashMap<u64, DrawTreeNode>,
    root_node: Option<u64>,
}

impl DrawTreeNode {
    pub fn new(callback: Option<Box<DrawCallback>>) -> Self {
        Self {
            callback,
            children: Vec::new(),
        }
    }

    pub fn render_single(&self, server: &draw::Server) -> anyhow::Result<()> {
        if let Some(callback) = self.callback.as_ref() {
            callback(server)?;
        }
        Ok(())
    }
}

impl DrawTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn render(&self, server: &draw::Server) -> anyhow::Result<()> {
        if let Some(root_node) = self.root_node {
            let mut node_stack = Vec::new();
            node_stack.push(root_node);
            while let Some(node_id) = node_stack.pop() {
                let node = self.nodes.get(&node_id).unwrap();
                for child in node.children.iter() {
                    node_stack.push(*child);
                }
                node.render_single(server)?;
            }
        }
        Ok(())
    }

    pub fn create_root<F>(&mut self, handle: u64, callback: F)
    where
        F: Fn(&draw::Server) -> anyhow::Result<()> + 'static,
    {
        self.root_node = Some(handle);
        self.nodes
            .insert(handle, DrawTreeNode::new(Some(Box::new(callback))));
    }
}

pub type DrawTreeNodeHandle = GfxHandle<DrawTreeNode>;
