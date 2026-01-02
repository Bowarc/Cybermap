use crate::element::{NWR, OSMNode, OSMRelation, OSMWay, Tagmap};
use std::{collections::HashMap, rc::Rc};

#[derive(serde::Deserialize)]
pub struct RawOsmData {
    pub elements: Vec<RawOsmElement>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RawOsmMember {
    r#type: String,
    r#ref: u64,
    r#role: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct RawOsmElement {
    pub r#type: String,
    pub id: u64,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub nodes: Option<Vec<u64>>,
    pub tags: Option<HashMap<String, String>>,
    #[serde(default)]
    pub members: Vec<RawOsmMember>,
}

struct SplitRawOsmData {
    pub nodes: Vec<RawOsmElement>,
    pub ways: Vec<RawOsmElement>,
    pub relations: Vec<RawOsmElement>,
    #[allow(dead_code)]
    pub others: Vec<RawOsmElement>,
}

impl SplitRawOsmData {
    fn total_count(&self) -> usize {
        self.nodes.len() + self.ways.len() + self.relations.len() + self.others.len()
    }
    fn from_raw_osm_data(osm_data: RawOsmData) -> Self {
        let mut nodes = Vec::new();
        let mut ways = Vec::new();
        let mut relations = Vec::new();
        let mut others = Vec::new();
        for element in osm_data.elements {
            match element.r#type.as_str() {
                "node" => nodes.push(element),
                "way" => ways.push(element),
                "relation" => relations.push(element),
                _ => others.push(element),
            }
        }
        Self {
            nodes,
            ways,
            relations,
            others,
        }
    }
}

fn parse_raw_osm_data(json_data: serde_json::Value) -> Result<SplitRawOsmData, serde_json::Error> {
    let osm_data: RawOsmData = serde_json::from_value(json_data)?;
    Ok(SplitRawOsmData::from_raw_osm_data(osm_data))
}

pub fn parse_osm_json(json_value: serde_json::Value) -> Result<Rc<[NWR]>, serde_json::Error> {
    let split_data = parse_raw_osm_data(json_value)?;
    let mut nodes_map: HashMap<u64, OSMNode> = HashMap::new();
    let mut ways_map: HashMap<u64, OSMWay> = HashMap::new();

    let mut nwr: Vec<NWR> = Vec::new();
    for element in split_data.nodes.into_iter() {
        let (Some(lat), Some(lon)) = (element.lat, element.lon) else {
            println!("Could not process node: {element:?} as it's missing lat or lon");
            continue;
        };
        let geopoint = crate::coord::geo::GeoPoint::new(lat, lon);

        let empty_tags = element.tags.as_ref().map(|t| t.is_empty()).unwrap_or(true);

        let node = OSMNode {
            osm_id: element.id,
            pos: geopoint,
            tags: element
                .tags
                .unwrap_or_default()
                .iter()
                .map(|(k, v)| (Rc::from(k.as_str()), Rc::from(v.as_str())))
                .collect::<Tagmap>(),
        };

        if !empty_tags {
            nwr.push(NWR::Node(node.clone()));
        }

        nodes_map.insert(element.id, node);
    }

    for element in split_data.ways.into_iter() {
        // List of the element's node ids mapped to nodes that we have locally
        let nodes = element
            .nodes
            .map(|node_ids| {
                node_ids
                    .iter()
                    .flat_map(|id| nodes_map.get(id).cloned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let way = OSMWay {
            osm_id: element.id,
            nodes: Rc::from(nodes),
            tags: element
                .tags
                .unwrap_or_default()
                .iter()
                .map(|(k, v)| (Rc::from(k.as_str()), Rc::from(v.as_str())))
                .collect::<Tagmap>(),
        };

        ways_map.insert(element.id, way.clone());
        nwr.push(NWR::Way(way));
    }

    println!("Skipped {} relations", split_data.relations.len());

    Ok(Rc::from(nwr))
}
