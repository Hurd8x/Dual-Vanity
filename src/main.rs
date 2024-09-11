#[macro_use] extern crate rocket;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rayon::prelude::*;
use bitcoin::util::address::Address;
use bitcoin::network::constants::Network;
use bitcoin::util::key::PrivateKey;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::blockdata::script::Script;
use rocket::fs::{FileServer, relative};
use rocket::tokio::sync::RwLock;
use rocket::tokio::time::{self, Duration};
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use log::{info, error};

#[derive(Deserialize)]
struct VanityRequest {
    prefix: String,
    suffix: String,
    key_length: u16,       // Key length in bits (e.g., 256)
    address_type: String,  // P2PKH, P2SH, P2WPKH
    max_iterations: Option<u64>, // Optional max iterations
}

#[derive(Serialize)]
struct VanityResponse {
    address: String,
    private_key: String,
    public_key: String,
    address_type: String,
    wif: String,
    progress_id: Option<String>,  // For tracking progress
}

#[derive(Serialize)]
struct ProgressResponse {
    progress: u64,
    total: u64,
    status: String,
}

// Struct to hold ongoing tasks' progress
struct VanityProgress {
    total_attempts: u64,
    found: bool,
}

// Global map to store progress of tasks
type ProgressMap = Arc<RwLock<HashMap<String, VanityProgress>>>;

#[post("/generate", format = "json", data = "<vanity_request>")]
async fn generate_vanity_address(
    vanity_request: Json<VanityRequest>,
    progress_map: &rocket::State<ProgressMap>,
) -> Json<VanityResponse> {
    let prefix = vanity_request.prefix.to_lowercase();
    let suffix = vanity_request.suffix.to_lowercase();
    let address_type = &vanity_request.address_type;
    let max_iterations = vanity_request.max_iterations.unwrap_or(100_000);

    // Ensure valid key length
    if vanity_request.key_length < 256 || vanity_request.key_length > 512 {
        return Json(VanityResponse {
            address: "".to_string(),
            private_key: "".to_string(),
            public_key: "".to_string(),
            address_type: "Invalid key length".to_string(),
            wif: "".to_string(),
            progress_id: None,
        });
    }

    let secp = Secp256k1::new();
    let progress_id = format!("{}", rand::thread_rng().gen::<u64>());

    // Initialize progress tracking
    {
        let mut progress_map_lock = progress_map.write().await;
        progress_map_lock.insert(
            progress_id.clone(),
            VanityProgress {
                total_attempts: 0,
                found: false,
            },
        );
    }

    let result = (0..max_iterations).into_par_iter().find_map_any(|attempt| {
        // Generate a random secret key
        let key_length_bytes = (vanity_request.key_length / 8) as usize;
        let mut random_bytes = vec![0u8; key_length_bytes];
        rand::thread_rng().fill(&mut random_bytes[..]);

        // Ensure the secret key is valid for secp256k1
        let secret_key = SecretKey::from_slice(&random_bytes).ok()?;
        let private_key = PrivateKey {
            compressed: true,
            network: Network::Bitcoin,
            key: secret_key,
        };

        let public_key = private_key.public_key(&secp);

        // Generate address based on requested address type
        let address = match address_type.as_str() {
            "P2PKH" => Address::p2pkh(&public_key, Network::Bitcoin).to_string(),
            "P2SH" => {
                let redeem_script = Script::new_p2pkh(&public_key.pubkey_hash());
                Address::p2sh(&redeem_script, Network::Bitcoin).to_string()
            },
            "P2WPKH" => Address::p2wpkh(&public_key, Network::Bitcoin).expect("SegWit supported").to_string(),
            _ => return None,
        };

        // Update progress in the progress map
        let mut progress_map_lock = progress_map.write().await;
        if let Some(progress) = progress_map_lock.get_mut(&progress_id) {
            progress.total_attempts += 1;
        }

        // Check if the address matches the required prefix and suffix
        if address.starts_with(&prefix) && address.ends_with(&suffix) {
            Some((address, private_key, public_key))
        } else {
            None
        }
    });

    // If a matching vanity address is found, return it in the response
    match result {
        Some((address, private_key, public_key)) => {
            let wif = private_key.to_wif();

            // Update progress to found
            {
                let mut progress_map_lock = progress_map.write().await;
                if let Some(progress) = progress_map_lock.get_mut(&progress_id) {
                    progress.found = true;
                }
            }

            Json(VanityResponse {
                address,
                private_key: wif.clone(),
                public_key: public_key.to_string(),
                address_type: address_type.to_string(),
                wif,
                progress_id: Some(progress_id),
            })
        },
        // If no address was found within the iteration limit
        None => Json(VanityResponse {
            address: "".to_string(),
            private_key: "".to_string(),
            public_key: "".to_string(),
            address_type: "No address found".to_string(),
            wif: "".to_string(),
            progress_id: Some(progress_id),
        }),
    }
}

#[get("/progress/<progress_id>")]
async fn check_progress(
    progress_id: String,
    progress_map: &rocket::State<ProgressMap>,
) -> Json<ProgressResponse> {
    let progress_map_lock = progress_map.read().await;
    if let Some(progress) = progress_map_lock.get(&progress_id) {
        let status = if progress.found {
            "Address Found".to_string()
        } else {
            "In Progress".to_string()
        };

        Json(ProgressResponse {
            progress: progress.total_attempts,
            total: 100_000, // max_iterations could be dynamic
            status,
        })
    } else {
        Json(ProgressResponse {
            progress: 0,
            total: 100_000,
            status: "Progress ID not found".to_string(),
        })
    }
}

#[launch]
fn rocket() -> _ {
    // Initialize progress map as a global state
    let progress_map: ProgressMap = Arc::new(RwLock::new(HashMap::new()));

    rocket::build()
        .manage(progress_map)
        .mount("/api", routes![generate_vanity_address, check_progress])
        .mount("/", FileServer::from(relative!("static")))
}
