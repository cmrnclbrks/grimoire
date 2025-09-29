mod app;
mod config;
mod secret;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::distr::{Distribution, Uniform};
use rand::prelude::*;
use rand_argon_compatible::rngs::OsRng as OsRng08;
use secret::{SavedSecret, Secret};
use std::env;
use std::fs;

fn authenticate(master_password: &str) -> Result<bool, argon2::password_hash::Error> {
    let mut filepath = env::home_dir().expect("No home directory found");
    filepath.push("~grimoire/password_store/master.txt");
    let hash = fs::read_to_string(filepath).expect("should  have read file");
    let parsed_hash = PasswordHash::new(&hash)?;

    Ok(Argon2::default()
        .verify_password(master_password.as_bytes(), &parsed_hash)
        .is_ok())
}

fn get_salt() -> [u8; 16] {
    let mut filepath = env::home_dir().expect("No home directory found");
    filepath.push("grimoire/password_store/master.txt");
    let hash = fs::read_to_string(filepath).expect("Should have read file");
    let hash_obj = PasswordHash::new(&hash).unwrap();
    match hash_obj.salt {
        Some(salt) => {
            let mut buf = [0u8; 16];
            salt.decode_b64(&mut buf).expect("Invalid salt");
            buf
        }
        None => panic!("No salt"),
    }
}

fn generate_password(length: u8, symbols: bool) -> String {
    let distr = Uniform::try_from(33..127).unwrap();
    let mut rng = rand::rng();
    let mut password = String::new();
    if symbols {
        while password.len() as u8 != length {
            password.push(distr.sample(&mut rng) as u8 as char);
        }
    }
    if !symbols {
        while password.len() as u8 != length {
            password.push(rng.sample(rand::distr::Alphanumeric) as char);
        }
    }
    password
}

fn set_master_password() {
    //GET INPUT
    let master_password = "1234";
    let mut filepath = env::home_dir().expect("No home directory found");
    filepath.push("grimoire/password_store/master.txt");
    if let Some(parent) = filepath.parent() {
        fs::create_dir_all(parent).expect("Couldn't create parent directories");
    }
    let salt = SaltString::generate(&mut OsRng08);
    let hash = Argon2::default()
        .hash_password(master_password.as_bytes(), &salt)
        .unwrap();

    let mut text = String::new();
    text.push_str(hash.to_string().as_str());

    match fs::write(filepath, text) {
        Ok(_) => (),
        Err(e) => panic!("{}", e),
    }
}

fn init() {
    let mut filepath = env::home_dir().expect("No home directory found");
    filepath.push("~/grimoire/password_store/master.txt");
    let contents = fs::read_to_string(filepath);
    match contents {
        Ok(text) => {
            if text.is_empty() {
                set_master_password();
            }
        }
        _ => {
            set_master_password();
        }
    }
}

fn main() {
    init();
    let args: Vec<String> = env::args().collect();
    let password_attempt = &args[1];
    let attempt = authenticate(password_attempt);
    if attempt.expect("error") {
        let salt = get_salt();
        let mut output_key_material = [0u8; 32];
        Argon2::default()
            .hash_password_into(password_attempt.as_bytes(), &salt, &mut output_key_material)
            .unwrap();

        //encrypt

        let first_secret = Secret::new("service", "user", "password");
        let json = first_secret.to_json();
        first_secret.save(output_key_material);

        //decrypt
        println!(
            "Secret retrieved successfully! {:?}",
            SavedSecret::decrypt(output_key_material, "grimoire/password_store/service")
        );
    }
}
