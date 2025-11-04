use leptos_struct_table::*;
use serde::{Deserialize, Serialize};

// --- Data Structures ---

/// The structure of the JSON payload we receive from the ESP32.
#[derive(Deserialize, Debug)]
pub struct IncomingData {
    pub id: String,
    pub payload: String,
    pub date: String,
    pub time: String,
}

impl IncomingData {
    /// A `Result` containing a tuple of (Longitude, Latitude, Battery) as u16,
    /// or a boxed error if parsing fails or the payload length is incorrect.
    pub fn parse_hex_payload(&self) -> Result<(u16, u16, u8), Box<dyn std::error::Error>> {
        // Expected length: 4 (Long) + 4 (Lat) + 2 (Batt) = 10 hex digits
        if self.payload.len() != 10 {
            return Err(format!(
                "Payload must be exactly 10 characters long, got {}",
                self.payload.len()
            )
            .into());
        }

        // 1. Longitude (first 4 hex digits)
        let long_hex = &self.payload[0..4];
        let longitude = u16::from_str_radix(long_hex, 16)?;

        // 2. Latitude (next 4 hex digits)
        let lat_hex = &self.payload[4..8];
        let latitude = u16::from_str_radix(lat_hex, 16)?;

        // 3. Battery (last 2 hex digits)
        let batt_hex = &self.payload[8..10];
        let battery = u8::from_str_radix(batt_hex, 16)?;

        Ok((longitude, latitude, battery))
    }
}

/// The structure we store in our "database" and send to the frontend.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, TableRow)]
#[table(impl_vec_data_provider)]
#[table(classes_provider = "TailwindClassesPreset")]
pub struct StoredData {
    pub id: String,
    pub longitude: u16,
    pub latitude: u16,
    pub battery: u8,
    pub timestamp: String,
}
