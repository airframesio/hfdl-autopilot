use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Frequency {
    pub id: u8,
    pub freq: f64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Time {
    pub sec: u64,
    pub usec: u64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PDUType {
    pub id: u8,
    pub name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Entity {
    pub id: u8,

    #[serde(alias = "type")]
    pub entity_type: String,

    #[serde(alias = "name")]
    pub entity_name: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct GroundStation {
    pub gs: Entity,
    pub utc_sync: bool,
    pub freqs: Vec<Frequency>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct FrequencyData {
    pub gs: Entity,
    pub listening_on_freqs: Vec<Frequency>,
    pub heard_on_freqs: Vec<Frequency>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct AircraftInfo {
    pub icao: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Position {
    pub lat: f32,
    pub lon: f32,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ACARS {
    pub reg: String,
    pub label: String,
    pub blk_id: String,
    pub ack: String,
    pub flight: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct HFNPDU {
    pub err: bool,

    #[serde(alias = "type")]
    pub pdu_type: PDUType,

    pub flight_id: Option<String>,
    pub pos: Option<Position>,

    pub acars: Option<ACARS>,
    pub freq_data: Option<Vec<FrequencyData>>,
}

impl HFNPDU {
    pub fn msg_type(&self) -> &str {
        &self.pdu_type.name
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Reason {
    code: u32,
    descr: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct LPDU {
    pub err: bool,
    pub src: Entity,
    pub dst: Entity,

    #[serde(alias = "type")]
    pub msg_type: PDUType,

    pub ac_info: Option<AircraftInfo>,
    pub reason: Option<Reason>,
    pub hfnpdu: Option<HFNPDU>,
}

impl LPDU {
    fn fmt_entity(&self, entity: &Entity) -> String {
        if let Some(name) = &entity.entity_name {
            return name.clone();
        } else if let Some(ref hfnpdu) = self.hfnpdu {
            if let Some(ref acars) = hfnpdu.acars {
                return format!("{:7} (AC)", acars.reg);
            } else if let Some(ref flight_id) = hfnpdu.flight_id {
                return format!("Flt[{:7}] (AC)", flight_id);
            }
        }

        if let Some(ac_info) = &self.ac_info {
            return format!("Hex[{:6}] (AC)", ac_info.icao);
        }

        format!("Id[{:03}] (AC)", entity.id)
    }

    pub fn source(&self) -> String {
        self.fmt_entity(&self.src)
    }

    pub fn destination(&self) -> String {
        self.fmt_entity(&self.dst)
    }

    pub fn msg_type(&self) -> &str {
        &self.msg_type.name
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SPDU {
    pub err: bool,
    pub src: Entity,
    pub gs_status: Vec<GroundStation>,
}

impl SPDU {
    pub fn source(&self) -> &str {
        &self.src.entity_name.as_ref().unwrap()
    }
}

#[derive(Deserialize, Debug)]
pub struct HFDL {
    pub t: Time,
    pub freq: u32,
    pub bit_rate: u16,
    pub sig_level: f64,

    pub spdu: Option<SPDU>,
    pub lpdu: Option<LPDU>,
}

impl HFDL {
    pub fn frequency(&self) -> f32 {
        (self.freq as f32) / 1000000.0
    }
}

#[derive(Deserialize, Debug)]
pub struct Frame {
    pub hfdl: HFDL,
}
