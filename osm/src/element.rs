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
    /// OSM internal id
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
/// A way, as it is given by OSM
pub struct OSMWay {
    /// OSM internal id
    pub osm_id: u64,
    /// List of nodes that makes the way
    pub nodes: Arc<[OSMNode]>,
    /// Map of tags given by OSM, this Tagmap is supposed to be read only
    pub tags: Tagmap,
}

#[derive(Debug, Clone, PartialEq)]
/// I don't really have a point for this type yet
pub struct OSMRelation;

#[derive(Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MercatorNWR {
    pub nodes: HashMap<u64, MercatorNode>,
    pub ways: HashMap<u64, MercatorWay>,
}
impl MercatorNWR {
    pub fn empty() -> Self {
        Self {
            nodes: HashMap::new(),
            ways: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MercatorNode {
    /// OSM internal id
    pub osm_id: u64,
    /// Position given by OSM, converted to Mercator using crate::coord::convertion
    pub pos: MercatorPoint,
    /// Map of tags given by OSM, this Tagmap is supposed to be read only
    pub tags: Tagmap,
}

impl PartialEq for MercatorNode {
    fn eq(&self, other: &Self) -> bool {
        self.osm_id == other.osm_id
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MercatorWay {
    /// OSM internal id
    pub osm_id: u64,
    /// List of nodes that makes the way, they are given by OSM and converted using crate::coord::convertion
    pub nodes: Arc<[MercatorNode]>,
    /// Map of tags given by OSM, this Tagmap is supposed to be read only
    pub tags: Tagmap,
}

impl PartialEq for MercatorWay {
    fn eq(&self, other: &Self) -> bool {
        self.osm_id == other.osm_id
    }
}
