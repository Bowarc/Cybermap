use osm::{
    coord::mercator::MercatorBox,
    element::{MercatorNode, MercatorWay},
};
use std::{collections::HashMap, rc::Rc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Chunk {
    pub zone: MercatorBox,
    pub nodes: HashMap<u64, MercatorNode>,
    pub ways: HashMap<u64, MercatorWay>,
    pub non_truncated_ways: HashMap<u64, Vec<u64>>,
}
