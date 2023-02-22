use serde::Deserialize;
use std::fmt;

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
    pub id: u16,
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
    pub lat: f64,
    pub lon: f64,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct ACARS {
    pub reg: String,
    pub label: String,
    pub blk_id: String,
    pub ack: String,
    pub flight: Option<String>,
    pub msg_num: Option<String>,
    pub msg_num_seq: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct SystablePartial {
    pub part_num: u8,
    pub parts_cnt: u8,
}

impl fmt::Display for SystablePartial {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.part_num, self.parts_cnt)
    }
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct PerfDataFreq {
    pub id: u32,
    pub freq: Option<f64>,
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

    pub version: Option<u8>,
    pub systable_partial: Option<SystablePartial>,

    pub frequency: Option<PerfDataFreq>,

    pub request_data: Option<u16>,
}

impl HFNPDU {
    pub fn msg_type(&self) -> &str {
        match self.pdu_type.id {
            208 => "SystablePart",
            209 => "PerfData",
            210 => "SystableReq",
            213 => "FreqData",
            _ => &self.pdu_type.name,
        }
    }

    pub fn short(&self) -> String {
        match self.pdu_type.id {
            208 => {
                if let Some(systable_partial) = &self.systable_partial {
                    format!("V:{} ({})", self.version.unwrap_or(0), systable_partial)
                } else {
                    "".to_string()
                }
            }
            209 => {
                if let Some(freq) = &self.frequency {
                    match freq.freq {
                        Some(val) => format!("F:{}", val),
                        None => format!("F:#{}", freq.id),
                    }
                } else {
                    "".to_string()
                }
            }
            210 => {
                if let Some(data) = &self.request_data {
                    format!("D:{}", data)
                } else {
                    "".to_string()
                }
            }
            213 => {
                if let Some(data) = &self.freq_data {
                    format!("GS:{:?}", data.iter().map(|x| x.gs.id).collect::<Vec<u8>>())
                } else {
                    "".to_string()
                }
            }
            _ => "".to_string(),
        }
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
    pub assigned_ac_id: Option<u8>,
    pub hfnpdu: Option<HFNPDU>,
}

impl LPDU {
    fn fmt_entity(&self, entity: &Entity) -> String {
        if let Some(name) = &entity.entity_name {
            return name.split(",").next().unwrap_or(name).to_string();
        } else if let Some(ref hfnpdu) = self.hfnpdu {
            if let Some(ref acars) = hfnpdu.acars {
                return format!("Reg[{:>7}]", acars.reg);
            } else if let Some(ref flight_id) = hfnpdu.flight_id {
                return format!("Flt[{:>7}]", flight_id);
            }
        }

        if let Some(ac_info) = &self.ac_info {
            return format!("Hex[{:>7}]", ac_info.icao);
        }

        format!("Aci[{:>7}]", entity.id)
    }

    pub fn source(&self) -> String {
        self.fmt_entity(&self.src)
    }

    pub fn destination(&self) -> String {
        self.fmt_entity(&self.dst)
    }

    pub fn msg_type(&self) -> &str {
        match self.msg_type.id {
            63 => "LogoffReq",
            79 => "LogonRes",
            159 => "LogonCfm",
            191 => "LogonReq",
            _ => &self.msg_type.name,
        }
    }

    pub fn short(&self) -> String {
        match self.msg_type.id {
            63 => {
                if let Some(reason) = &self.reason {
                    format!("R:{}", reason.descr)
                } else {
                    "".to_string()
                }
            }
            159 => {
                if let Some(ac_id) = &self.assigned_ac_id {
                    format!("ID:#{}", ac_id)
                } else {
                    "".to_string()
                }
            }
            _ => "".to_string(),
        }
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
    pub fn source(&self) -> String {
        let name = self.src.entity_name.as_ref().unwrap().clone();
        name.split(",").next().unwrap_or(&name).to_string()
    }

    pub fn short(&self) -> String {
        format!(
            "GS:{:?}",
            self.gs_status.iter().map(|x| x.gs.id).collect::<Vec<u8>>()
        )
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
    pub fn frequency(&self) -> String {
        format!("{:.3}", (self.freq as f32) / 1000000.0)
    }

    pub fn signal(&self) -> String {
        format!("{:.1}", self.sig_level)
    }
}

#[derive(Deserialize, Debug)]
pub struct Frame {
    pub hfdl: HFDL,
}
