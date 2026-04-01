use crate::coord::{geo::GeoPoint, mercator::MercatorPoint};
use std::{collections::HashMap, sync::Arc};

// Map of immutable string slices
pub type Tagmap = HashMap<Arc<str>, Arc<str>>;

// #[derive(Debug, Clone, PartialEq)]
// pub enum NWR {
//     Node(OSMNode),
//     Way(OSMWay),
//     Relation(OSMRelation),
// }

#[derive(Debug, PartialEq, Default)]
pub struct NWR {
    pub nodes: HashMap<u64, OSMNode>,
    pub ways: HashMap<u64, OSMWay>,
    // pub relations: Rc<[OSMRelation]>,
}

impl NWR {
    pub fn total_count(&self) -> usize {
        self.nodes.len() + self.ways.len()
    }
}

#[derive(Debug, Clone)]
pub struct OSMNode {
    pub osm_id: u64,
    pub pos: GeoPoint,
    pub tags: Tagmap,
}

impl PartialEq for OSMNode {
    fn eq(&self, other: &Self) -> bool {
        self.osm_id == other.osm_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OSMWay {
    pub osm_id: u64,
    pub nodes: Arc<[OSMNode]>,
    pub tags: Tagmap,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OSMRelation {}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MercatorNode {
    pub osm_id: u64,
    pub pos: MercatorPoint,
    pub tags: Tagmap,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MercatorWay {
    pub osm_id: u64,
    pub nodes: Arc<[MercatorNode]>,
    pub tags: Tagmap,
}
