mod commands;
mod users_database;
pub use commands::CmdTypes;
use serde::{Serialize, Deserialize};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use rand::rngs::OsRng; //générateur de nombres aléatoires cryptographiquement sécurisé
use std::{str,path::PathBuf};
use aes_gcm::{
    aead::{ Aead, AeadCore, KeyInit},
    Aes256Gcm, Key // Or `Aes128Gcm`
};
use argon2::Argon2;
use std::fs::OpenOptions;
use std::io::Read;
use rsa::pkcs1::EncodeRsaPublicKey;
use musshTransport;
use rand::prelude::*;
use std::net::TcpStream;
use rsa::pkcs1::DecodeRsaPublicKey;
use rsa::pkcs1v15::{SigningKey, VerifyingKey};
use rsa::sha2::Sha256;
use rsa::signature::Verifier;
use std::fs::File;
use std::io::Write;

use rsa::signature::{RandomizedSigner, SignatureEncoding};

/*
cargo add zeroize
cargo add serde serde_json
cargo add rsa
cargo add rand
cargo add aes-gcm
Donc si je résume, au début le client et le serveur posèdent une clé privé et une clé publique
Dans un premier temps sur un chanelle non protégé le cllient envoi un message au serveur pour faire une demande 
de connection ( en utilisant par exemple un syn que j'aurais créé qui contient un identifiant utilisateur (clé publique ?))
Ensuite le serveur répond avec un défi chiffré avec sa clé privée (défi calcul et signature )
le client résoud le  mini défi et le signe et l'envoi 
le serveur check l'intégrité du message si c'est bon il répond au client avec une clé de communication pour 
qu'ils se placent sur un chanel de comunication chiffré (la clé est chiffré avec la clé public du client )

/// Pour le moment on créé les clés et on le met dans un fichier local chifré avec 
/// le mot de passe utilisateur 
generate_Certiﬁcat_key(username, password){
	// generate two keys
	private, public = key_generator;
	// writes the public key on the file called (server_database)
	// write the private key and username and hash on a file called username_key
	
}
// sets the value for the structure that contains username and private key
get_Certiﬁcat(file, password, *private_key){
	//ouvrir le fichier
	//recuperer la ligne avec deseralize pour remettre username et private key dans la structure
	// déchifrée la clé privée avec le mot de passe
	// la comparé avec son hash
}
generate_server_key
*/

#[derive(Serialize, Deserialize, Debug)]
struct CertificateInfo {
    username: String,
    private_key: RsaPrivateKey, // zeroized on drop par defaut
    public_key:  RsaPublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
struct CertificateChiffre {
    salt:[u8; 12],
    nonce: [u8; 12],
    chiffre:Vec<u8>,
}

fn _generate_certificate_key(username: &str, password: &str) {
	let bits = 2048; // key lenght
	// Key generation
	let mut rng = OsRng;
	let private_key = RsaPrivateKey::new(&mut rng, bits).unwrap();
    let public_key = private_key.to_public_key();
	
	
	let cert_info = CertificateInfo {
        username:    username.to_string(),
        private_key: private_key,
        public_key:  public_key,
    };

    // Sérialiser la structure en JSON
    let cert_info_json = serde_json::to_string(&cert_info).unwrap();
    //let key = Aes256Gcm::generate_key(password.as_bytes());
    
    let salt: &[u8; 12] = b"example salt"; // Salt should be unique per password
    let mut output_key_material = [0u8; 32]; // Can be any desired size
    Argon2::default().hash_password_into(password.as_bytes(), salt, &mut output_key_material).unwrap();

    let aes_key= Key::<Aes256Gcm>::from_slice(&output_key_material);
	let cipher = Aes256Gcm::new(aes_key);
	let nonce: aes_gcm::aead::generic_array::GenericArray<u8, _> = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
	let ciphertext = cipher.encrypt(&nonce, cert_info_json.as_bytes()).unwrap();



    // J'ajoute le salt et le nonce
    let cert_chiff = CertificateChiffre {
        salt: *salt,
        nonce: nonce.into(),
        chiffre: ciphertext,
    };

    let cert_chiff_json = serde_json::to_string(&cert_chiff).unwrap();

    // écrire dans un fichier cert_chiff_json.mdp
    let mut file = File::create("cert_chiff_json.mdp").unwrap();

    // Écrire la chaîne JSON dans le fichier
    let _ = file.write_all(cert_chiff_json.as_bytes());

    println!("Le certificat chiffré a été écrit dans 'cert_chiff_json.mdp'");
    
}


#[derive(Serialize, Deserialize, Debug)]
enum AuthMsg{
    Demande,
    PubKey(RsaPublicKey),
    DemandeCo(Vec<u8>),
    Defi(Vec<u8>),
    ReponseDefi(Vec<u8>),
    OK(Vec<u8>),
    Error(String),
}

fn authentification_serveur(stream: &TcpStream) -> Result<(users_database::User,[u8; 32]), std::io::Error> {
    let users = match users_database::Users::load_from_file() {
        Ok(users) => users,
        Err(_) => users_database::Users::new(),
    };
    let serv_pub_key = users.server_public_key.clone();
    let serv_priv_key = users.server_private_key.clone();

    let mut typed_reader = musshTransport::TypedReader::<_, AuthMsg>::new(stream);
    let mut typed_writer = musshTransport::TypedWriter::<_, AuthMsg>::new(stream);
    let user: users_database::User;
    // Recevoir la demande initiale
    let mut response= typed_reader.recv().unwrap(); // recevoir demande
    match response {
        AuthMsg::Demande => {
            // Envoyer la clé publique du serveur
            typed_writer.send(&AuthMsg::PubKey(serv_pub_key)).unwrap();
        },
        _ => {
            typed_writer.send(&AuthMsg::Error("Invalid initial request".to_string())).unwrap();
            return  Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid initial request",
            ));
        }
    }

    // Recevoir la demande de connexion avec la clé publique du client
    response = typed_reader.recv().unwrap();
    let client_pub_key = match response {
        AuthMsg::DemandeCo(pub_key) => {
            let client_key_tmp = RsaPublicKey::from_pkcs1_der(&pub_key).expect("Invalid public key format");
            //chercher si l'utilisateur existe dans notre base de donnée
            match users_database::find_user(client_key_tmp.clone()){
                Ok(client) => {
                                    user = client;
                                    client_key_tmp
                                    },
                Err(e)    =>{
                                    typed_writer.send(&AuthMsg::Error("User does not exist".to_string())).unwrap();
                                    return Err(e)
                                }
            }
            
        },
        _ => {
             // Si la demande de connexion est invalide, envoyer une erreur au client
            typed_writer.send(&AuthMsg::Error("Invalid response to public key request".to_string())).unwrap();
            return  Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid initial request",
            ));
        }
    };

    // Générer un défi
    let mut rng = OsRng;
    let challenge: [u8; 32] = rng.gen();
    let challenge_enc = client_pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &challenge).expect("failed to encrypt");

    // Envoyer le défi au client
    typed_writer.send(&AuthMsg::Defi(challenge_enc)).unwrap();

    // Recevoir la réponse du client
    let response = match typed_reader.recv().unwrap() {
        AuthMsg::ReponseDefi(response) => response,
        _ => {
            typed_writer.send(&AuthMsg::Error("Invalid challenge response".to_string())).unwrap();
            return  Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Invalid challenge response",
            ));
        }
    };

    let extract_defi= serv_priv_key.decrypt(Pkcs1v15Encrypt, &response).unwrap();
    let signature_from_bytes = rsa::pkcs1v15::Signature::try_from(extract_defi.as_slice()).expect("Failed to convert bytes to signature");

    // Vérifier la réponse
    let verifying_key = VerifyingKey::<Sha256>::new(client_pub_key.clone());
    if verifying_key.verify(&challenge, &signature_from_bytes).is_err() {
        typed_writer.send(&AuthMsg::Error("Invalid challenge response".to_string())).unwrap();
        return  Err(std::io::Error::new( 
            std::io::ErrorKind::PermissionDenied,
            "Invalid challenge response",
        ));
    }

    // Générer une clé de session
    let session_key: [u8; 32] = rng.gen();
    let session_key_enc = client_pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &session_key)
        .expect("Failed to encrypt session key");

    // Envoyer la clé de session au client
    typed_writer.send(&AuthMsg::OK(session_key_enc)).unwrap();

    Ok((user, session_key))
}


fn authentification_client(cert: &CertificateInfo, stream: &TcpStream) -> Result<[u8; 32], std::io::Error> {
    let mut typed_reader = musshTransport::TypedReader::<_, AuthMsg>::new(stream);
    let mut typed_writer = musshTransport::TypedWriter::<_, AuthMsg>::new(stream);

    // Envoyer une demande initiale
    typed_writer.send(&AuthMsg::Demande).unwrap();

    // Recevoir la clé publique du serveur
    let serv_pub_key = match typed_reader.recv().unwrap() {
        AuthMsg::PubKey(key) => key,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Failed to receive server public key",
            ));
        }
    };

    // Envoyer la clé publique du client chiffrée avec la clé publique du serveur
    let pub_key_der = cert.public_key.to_pkcs1_der().unwrap().to_vec();
    let mut rng = OsRng;
    let enc_pub_key = serv_pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &pub_key_der)
        .expect("Failed to encrypt client public key");
    typed_writer.send(&AuthMsg::DemandeCo(enc_pub_key)).unwrap();

    // Recevoir le défi du serveur
    let challenge_enc = match typed_reader.recv().unwrap() {
        AuthMsg::Defi(enc) => enc,
        _ => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Failed to receive challenge from server",
            ));
        }
    };

    // Déchiffrer le défi avec la clé privée du client
    let challenge = cert.private_key.decrypt(Pkcs1v15Encrypt, &challenge_enc)
        .expect("Failed to decrypt challenge");

    // Signer le défi avec la clé privée du client
    let signing_key = SigningKey::<Sha256>::new(cert.private_key.clone());
    let signature = signing_key.sign_with_rng(&mut rng, &challenge);

    // Envoyer la signature au serveur
    let enc_defi = serv_pub_key.encrypt(&mut rng, Pkcs1v15Encrypt, &signature.to_bytes())
        .expect("Failed to encrypt défi");
    typed_writer.send(&AuthMsg::ReponseDefi(enc_defi)).expect("Failed to send défi");

    // Recevoir la clé de session ou un message d'erreur
    match typed_reader.recv().unwrap() {
        AuthMsg::OK(session_key_enc) => {
            // Déchiffrer la clé de session
            let session_key = cert.private_key.decrypt(Pkcs1v15Encrypt, &session_key_enc)
                .expect("Failed to decrypt session key");
            Ok(session_key.try_into().expect("Session key length mismatch"))
        },
        AuthMsg::Error(msg) => {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                msg,
            ))
        },
        _ => {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Unexpected server response",
            ))
        }
    }
}

/*
Loop principal du serveur
 */
pub fn server_loop(stream: &TcpStream){
    let (user,session_key):(users_database::User,[u8; 32]) = authentification_serveur(&stream).unwrap(); 

    // Aller dans le dossier du "user"
    commands::start_cmd(user.id).unwrap();

    let mut encrypted_reader = musshTransport::EncryptedTypedReader::try_new(stream, &session_key).unwrap();
    let mut encrypted_writer = musshTransport::EncryptedTypedWriter::try_new(stream, &session_key).unwrap();
    loop{
        let cmd: commands::CmdTypes = match encrypted_reader.recv() {
            Ok(cmd) => cmd,
            Err(e) => {
                eprintln!("Failed to receive command: {}", e);
                encrypted_writer.send(&e.to_string()).unwrap();
                continue; // Skip the rest of the loop and wait for the next command
            },
        };
        let answer = match cmd.execute() {
            Ok(message)    => message,
            Err(e)          => e.to_string(),
        };
        encrypted_writer.send(&answer).unwrap();
    }
}   


pub fn client_connect(certificate: PathBuf,password: String, stream: &TcpStream)->[u8; 32]{
    // get key dans le fichier path
    // Ouvrir le fichier en mode lecture
    let mut file = OpenOptions::new().read(true).open(certificate).unwrap();

    // Lire le contenu du fichier dans un vecteur d'octets
    let mut cert: Vec<u8> = Vec::new();
    file.read_to_end(&mut cert).unwrap();
    let cert_chiffre: CertificateChiffre = serde_json::from_str(str::from_utf8(&cert).unwrap()).unwrap();

    // récuperation du certificat et déchiffrement 
    let mut output_key_material = [0u8; 32]; // Can be any desired size
    Argon2::default().hash_password_into(password.as_bytes(), &cert_chiffre.salt, &mut output_key_material).unwrap();
    let aes_key= Key::<Aes256Gcm>::from_slice(&output_key_material);
	let cipher = Aes256Gcm::new(aes_key);
    let plaintext = cipher.decrypt(&cert_chiffre.nonce.into(), cert_chiffre.chiffre.as_ref()).unwrap();
    let certificat: CertificateInfo = serde_json::from_str(str::from_utf8(&plaintext).unwrap()).unwrap();
    // call auth client

    authentification_client(&certificat, &stream).unwrap()

}