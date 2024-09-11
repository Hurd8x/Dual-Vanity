#[macro_use] extern crate rocket;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rayon::prelude::*;
use bitcoin::util::address::Address;
use bitcoin::network::constants::Network;
use bitcoin::util::key::PrivateKey;
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::blockdata::script::Script;
use rocket::fs::{FileServer, relative};
use rand::Rng;

#[derive(Deserialize)]
struct VanityRequest {
    prefix: String,
    suffix: String,
    key_length: u16,      // Key length in bits (e.g., 256)
    address_type: String, // P2PKH, P2SH, P2WPKH
}

#[derive(Serialize)]
struct VanityResponse {
    address: String,
    private_key: String,
    public_key: String,
    address_type: String,
    wif: String,
}

#[post("/generate", format = "json", data = "<vanity_request>")]
fn generate_vanity_address(vanity_request: Json<VanityRequest>) -> Json<VanityResponse> {
    let prefix = vanity_request.prefix.to_lowercase();
    let suffix = vanity_request.suffix.to_lowercase();
    let address_type = vanity_request.address_type.as_str();
    let key_length = vanity_request.key_length;

    let secp = Secp256k1::new();

    // Parallel execution to find the correct vanity address
    let result = (0..100_000).into_par_iter().find_map_any(|_| {
        // Generate a random secret key based on the requested key length
        let key_length_bytes = (key_length / 8) as usize;
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
        let address = match address_type {
            "P2PKH" => Address::p2pkh(&public_key, Network::Bitcoin).to_string(),
            "P2SH" => {
                let redeem_script = Script::new_p2pkh(&public_key.pubkey_hash());
                Address::p2sh(&redeem_script, Network::Bitcoin).to_string()
            },
            "P2WPKH" => Address::p2wpkh(&public_key, Network::Bitcoin).expect("SegWit supported").to_string(),
            _ => return None,
        };

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

            Json(VanityResponse {
                address,
                private_key: wif.clone(),
                public_key: public_key.to_string(),
                address_type: address_type.to_string(),
                wif,
            })
        },
        // If no address was found within the iteration limit
        None => Json(VanityResponse {
            address: "".to_string(),
            private_key: "".to_string(),
            public_key: "".to_string(),
            address_type: "No address found".to_string(),
            wif: "".to_string(),
        }),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![generate_vanity_address])
        .mount("/", FileServer::from(relative!("static")))
}
