use crate::coord::geo::GeoPoint;
use std::{collections::HashMap, rc::Rc};
// Map of immutable string slices
pub type Tagmap = HashMap<Rc<str>, Rc<str>>;
// type Tagmap = HashMap<String, String>;

#[derive(Debug, Clone, PartialEq)]
pub enum NWR {
    Node(OSMNode),
    Way(OSMWay),
    Relation(OSMRelation),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OSMNode {
    pub osm_id: u64,
    pub pos: GeoPoint,
    pub tags: Tagmap, // Box is even better than Rc if you don't need to clone.
}
#[derive(Debug, Clone, PartialEq)]
pub struct OSMWay {
    pub osm_id: u64,
    // pub nodes_ids: Rc<[u64]>, // Box is even better than Rc if you don't need to clone.
    pub nodes: Rc<[OSMNode]>, // Box is even better than Rc if you don't need to clone.
    pub tags: Tagmap,
}
#[derive(Debug, Clone, PartialEq)]
pub struct OSMRelation {}

