use serde::{Serialize, Deserialize};
use rsa::{RsaPublicKey,RsaPrivateKey};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::fs;
use rand::rngs::OsRng; 

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    username: String,
    pub id: u32,                 // Permet de trouver le "home" de l'utilisateur
    //verifying_key: String, // Permet de verifier la signature // use ub fn new(key: RsaPublicKey) -> Self
    public_key: RsaPublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Users {
    pub server_public_key: RsaPublicKey,
    pub server_private_key:RsaPrivateKey,
    users: Vec<User>,
}

impl Users {
    pub fn new() -> Self {
        let bits = 2048; // key lenght
        // Key generation
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, bits).unwrap();
        let public_key = private_key.to_public_key();
        
        Users { server_public_key: public_key,
                server_private_key:private_key,
                users: Vec::new() }
    }

    fn add_user(&mut self, user: User) {
        self.users.push(user);
    }

    fn save_to_file(&self) -> io::Result<()> {
        let json_data = serde_json::to_string_pretty(&self)?;
        let mut file = File::create("users.json")?;
        file.write_all(json_data.as_bytes())?;
        Ok(())
    }

    pub fn load_from_file() -> io::Result<Self> {
        let mut file = OpenOptions::new().read(true).open("users.json")?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;
        let users: Users = serde_json::from_str(&data)?;
        Ok(users)
    }

    fn new_user_id(&self) -> u32 {
        match self.users.last() {
            None => 0,
            Some(last_user) => last_user.id + 1,
        }
    }
}

pub fn add_user(username: String, public_key: RsaPublicKey) -> io::Result<()> {
    // ajout de l'utilisateur dans le fichier d'utilisateurs
    let mut users = match Users::load_from_file() {
        Ok(users) => users,
        Err(_) => Users::new(),
    };

    let id = users.new_user_id();

    let new_user = User {
        username,
        id,
        public_key,
    };

    users.add_user(new_user);

    // création du "home" de l'utilisateur
    let mut path = PathBuf::from("users");
    path.push(id.to_string());
    path.push("home");
    fs::create_dir_all(path)?;
    
    users.save_to_file()
}

pub fn find_user(public_key: RsaPublicKey)->  Result<User, std::io::Error> {
    let users = match Users::load_from_file() {
        Ok(users) => users,
        Err(_) => Users::new(),
    };
    for user in users.users.iter(){
        if user.public_key == public_key{
            return Ok(user.clone());
        }
    }
    return Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Cannot find the user",
    ));

}
