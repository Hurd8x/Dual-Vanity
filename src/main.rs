#[macro_use] extern crate rocket;
use rocket::serde::{Serialize, Deserialize, json::Json};
use rayon::prelude::*;
use bitcoin::util::address::{Address};
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
    key_length: u16,
    address_type: String,  // P2PKH, P2SH, P2WPKH
}

#[derive(Serialize)]
struct VanityResponse {
    address: String,
    private_key: String,
    public_key: String,
    address_type: String,
    wif: String,
    address_hash: String,
}

#[post("/generate", format = "json", data = "<vanity_request>")]
fn generate_vanity_address(vanity_request: Json<VanityRequest>) -> Json<VanityResponse> {
    let prefix = vanity_request.prefix.to_lowercase();
    let suffix = vanity_request.suffix.to_lowercase();
    let address_type = &vanity_request.address_type;

    let secp = Secp256k1::new();

    // Paralel olarak adres üretecek ve prefix/suffix kontrolü yapacak
    let result = (0..100_000).into_par_iter().find_map_any(|_| {
        // Her zaman 32 byte'lık (256 bit) bir secret key oluştur
        let random_bytes: [u8; 32] = rand::thread_rng().gen();
        let secret_key = SecretKey::from_slice(&random_bytes).ok()?;
        let private_key = PrivateKey {
            compressed: true,
            network: Network::Bitcoin,
            key: secret_key,
        };

        let public_key = private_key.public_key(&secp);

        // Kullanıcının seçtiği adres türüne göre adres oluşturma
        let address = match address_type.as_str() {
            "P2PKH" => Address::p2pkh(&public_key, Network::Bitcoin).to_string(),
            "P2SH" => {
                let redeem_script = Script::new_p2pkh(&public_key.pubkey_hash());
                Address::p2sh(&redeem_script, Network::Bitcoin).to_string()
            },
            "P2WPKH" => Address::p2wpkh(&public_key, Network::Bitcoin).expect("SegWit supported").to_string(),
            _ => return None,
        };

        // Prefix ve Suffix kontrolü
        if address.starts_with(&prefix) && address.ends_with(&suffix) {
            Some((address, private_key, public_key))
        } else {
            None
        }
    });

    match result {
        Some((address, private_key, public_key)) => {
            let wif = private_key.to_wif();

            Json(VanityResponse {
                address: address.clone(),
                private_key: wif.clone(),
                public_key: public_key.to_string(),
                address_type: address_type.to_string(),
                wif,
                address_hash: address.clone(),
            })
        },
        None => Json(VanityResponse {
            address: "".to_string(),
            private_key: "".to_string(),
            public_key: "".to_string(),
            address_type: "No address found".to_string(),
            wif: "".to_string(),
            address_hash: "".to_string(),
        }),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/api", routes![generate_vanity_address])
        .mount("/", FileServer::from(relative!("static")))
}
