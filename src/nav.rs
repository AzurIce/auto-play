use std::{collections::HashMap, hash::RandomState};

use ap_controller::ControllerTrait;
use petgraph::{
    algo::{all_simple_paths, astar, dijkstra},
    graph::NodeIndex,
    visit::IntoNodeReferences,
    Graph,
};

use crate::AutoPlay;

pub struct Node {
    checker: Option<Box<dyn Fn(&AutoPlay) -> bool>>,
}

pub struct NavGraph {
    ids: HashMap<String, NodeIndex<u32>>,
    names: HashMap<NodeIndex<u32>, String>,
    inner: Graph<Node, Box<dyn Fn(&AutoPlay) -> anyhow::Result<()>>>,
}

impl Default for NavGraph {
    fn default() -> Self {
        Self {
            ids: HashMap::new(),
            names: HashMap::new(),
            inner: Graph::new(),
        }
    }
}

impl NavGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_node(&mut self, id: impl AsRef<str>, node: Node) {
        let id = id.as_ref();
        let idx = self.inner.add_node(node);
        self.ids.insert(id.to_string(), idx.clone());
        self.names.insert(idx, id.to_string());
    }

    pub fn insert_edge(
        &mut self,
        from: impl AsRef<str>,
        to: impl AsRef<str>,
        edge: Box<dyn Fn(&AutoPlay) -> anyhow::Result<()>>,
    ) {
        let from = from.as_ref();
        let to = to.as_ref();
        let from_index = self.ids.get(from).unwrap().clone();
        let to_index = self.ids.get(to).unwrap().clone();
        self.inner.add_edge(from_index, to_index, edge);
    }

    pub fn current_node(&self, ap: &AutoPlay) -> Option<String> {
        self.inner
            .node_references()
            .find(|(_, n)| n.checker.as_ref().map(|c| c(ap)).unwrap_or(false))
            .map(|(idx, _)| self.names[&idx].clone())
    }

    pub fn nav(
        &self,
        ap: &AutoPlay,
        from: impl AsRef<str>,
        to: impl AsRef<str>,
    ) -> anyhow::Result<()> {
        let from = self.ids.get(from.as_ref()).unwrap();
        let to = self.ids.get(to.as_ref()).unwrap();
        let (cost, path) = astar(&self.inner, *from, |n| n == *to, |_| 1, |_| 0)
            .ok_or(anyhow::anyhow!("unreachable"))?;
        println!("cost: {cost}, path: {:?}", path);
        for e in path.windows(2).map(|idxs| {
            self.inner
                .edges_connecting(idxs[0], idxs[1])
                .next()
                .unwrap()
        }) {
            (e.weight())(ap)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyController;

    impl ControllerTrait for DummyController {
        fn screen_size(&self) -> (u32, u32) {
            todo!()
        }

        fn screencap_raw(&self) -> anyhow::Result<(u32, u32, Vec<u8>)> {
            todo!()
        }

        fn screencap(&self) -> anyhow::Result<image::DynamicImage> {
            todo!()
        }

        fn click(&self, x: u32, y: u32) -> anyhow::Result<()> {
            todo!()
        }

        fn swipe(
            &self,
            start: (u32, u32),
            end: (i32, i32),
            duration: std::time::Duration,
            slope_in: f32,
            slope_out: f32,
        ) -> anyhow::Result<()> {
            todo!()
        }
    }

    #[test]
    fn test_nav_graph() {
        let ap = AutoPlay::new(DummyController);
        let mut graph = NavGraph::new();
        graph.insert_node("start", Node { checker: None });
        graph.insert_node("mid", Node { checker: None });
        graph.insert_node("end", Node { checker: None });
        graph.insert_edge(
            "start",
            "mid",
            Box::new(|_| {
                println!("start -> mid");
                Ok(())
            }),
        );
        graph.insert_edge(
            "mid",
            "end",
            Box::new(|_| {
                println!("mid -> end");
                Ok(())
            }),
        );
        graph.insert_edge(
            "start",
            "end",
            Box::new(|_| {
                println!("start -> end");
                Ok(())
            }),
        );
        graph.nav(&ap, "start", "end");
    }
}
