use crate::secret;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use rand::distr::{Distribution, Uniform};
use rand::prelude::*;
use rand_argon_compatible::rngs::OsRng as OsRng08;
use secret::{EncryptedSecret, Secret};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub enum CurrentScreen {
    Main,
    Editing,
    New,
}

pub enum CurrentlyEditing {
    Name,
    Username,
    Password,
}

pub struct App {
    pub secrets: HashMap<String, Secret>,
    pub current_screen: CurrentScreen,
    pub currently_editing: Option<CurrentlyEditing>,
    pub master_password_file: PathBuf,
    pub password_store: PathBuf,
}

impl App {
    pub fn new(password_attempt: &str) -> App {
        let mut app = App {
            secrets: HashMap::new(),
            current_screen: CurrentScreen::Main,
            currently_editing: None,
            master_password_file: PathBuf::from("~/grimoire/password_store/master.txt"),
            password_store: PathBuf::from("~/grimoire/password_store/"),
        };
        // init the master_password and secret store
        app.init();
        let attempt = app.authenticate(password_attempt);
        if attempt.expect("Error") {
            let salt = app.get_salt();
            let mut output_key_material = [0u8; 32];
            Argon2::default()
                .hash_password_into(password_attempt.as_bytes(), &salt, &mut output_key_material)
                .unwrap();
            app.populate_secrets(output_key_material);
            app
        } else {
            std::process::exit(1)
        }
    }
    fn authenticate(&self, master_password: &str) -> Result<bool, argon2::password_hash::Error> {
        let hash = fs::read_to_string(&self.master_password_file).expect("should  have read file");
        let parsed_hash = PasswordHash::new(&hash)?;

        Ok(Argon2::default()
            .verify_password(master_password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    fn get_salt(&self) -> [u8; 16] {
        let hash = fs::read_to_string(&self.master_password_file).expect("Should have read file");
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

    fn set_master_password(&self) {
        //GET INPUT
        let master_password = "1234";
        if let Some(parent) = &self.master_password_file.parent() {
            fs::create_dir_all(parent).expect("Couldn't create parent directories");
        }
        let salt = SaltString::generate(&mut OsRng08);
        let hash = Argon2::default()
            .hash_password(master_password.as_bytes(), &salt)
            .unwrap();

        let mut text = String::new();
        text.push_str(hash.to_string().as_str());

        match fs::write(&self.master_password_file, text) {
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        }
    }

    fn init(&mut self) {
        let contents = fs::read_to_string(&self.master_password_file);
        match contents {
            Ok(text) => {
                if text.is_empty() {
                    self.set_master_password();
                }
            }
            _ => {
                self.set_master_password();
            }
        }
    }
    fn populate_secrets(&mut self, key: [u8; 32]) -> std::io::Result<()> {
        for entry in fs::read_dir(self.password_store.clone())? {
            let entry = entry?;
            let path = entry.path();
            if path == self.master_password_file {
                continue;
            }
            let secret: Secret = EncryptedSecret::decrypt(key, path);
            self.secrets.insert(String::from(secret.get_name()), secret);
        }
        Ok(())
    }
}
