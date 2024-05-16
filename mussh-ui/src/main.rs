use mussh_ui::{App, KeyReaction};
use std::error::Error;
use std::path::PathBuf;
use std::net::{TcpStream,TcpListener};
use usersGestion::CmdTypes;
use usersGestion;
use std::env;

fn analyse_input(s: String,stream: &TcpStream ,connected :&mut bool) -> Result<CmdTypes, String> {
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    let command = parts.get(0).ok_or("No command found")?;

    match *command {
        "cd" => {
            if !*connected{ return Err("Not connected".to_string());}
            if parts.len() > 2 {
                return Err("Too many arguments for cd".to_string());
            }
            let path = parts.get(1).ok_or("No path provided for cd")?;
            Ok(CmdTypes::Cd(PathBuf::from(path)))
        }
        "ls" => {
            if !*connected{ return Err("Not connected".to_string());}
            if parts.len() > 2 {
                return Err("Too many arguments for ls".to_string());
            }
            let all_flag = parts.get(1).map_or(false, |&arg| arg == "-a");
            Ok(CmdTypes::Ls(all_flag))
        }
        "cat" => {
            if !*connected{ return Err("Not connected".to_string());}
            if parts.len() > 2 {
                return Err("Too many arguments for cat".to_string());
            }
            let path = parts.get(1).ok_or("No path provided for cat")?;
            Ok(CmdTypes::Cat(PathBuf::from(path)))
        }
        "mkdir" => {
            if !*connected{ return Err("Not connected".to_string());}
            if parts.len() > 2 {
                return Err("Too many arguments for mkdir".to_string());
            }
            let path = parts.get(1).ok_or("No path provided for mkdir")?;
            Ok(CmdTypes::Mkdir(PathBuf::from(path)))
        }
        "connect" => {
            if parts.len() > 3 {
                return Err("Too many arguments for connect".to_string());
            }
            let path = parts.get(1).ok_or("No path provided for connect")?;
            let passwd = parts.get(2).ok_or("No password provided")?;
            usersGestion::client_connect(PathBuf::from(path), passwd.to_string(),stream);
            *connected = true;
            return Err("connected".to_string() );
        }
        _ => Err("Unknown command".to_string()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut connected = false;
    // Vérifiez les arguments de la ligne de commande
    let args: Vec<String> = env::args().collect();
    let is_server = args.len() > 1 && args[1] == "--server";

    if is_server {
            // Créer un serveur TCP qui écoute sur le port spécifié
        let listener = TcpListener::bind("127.0.0.1:1234")?;

        // Afficher un message pour indiquer que le serveur est prêt à accepter les connexions
        println!("Serveur démarré, en attente de connexions...");

        // Boucle infinie pour accepter les connexions des clients
        for stream in listener.incoming() {
            // Pour chaque nouvelle connexion entrante, un nouveau thread est créé pour gérer cette connexion
            match stream {
                Ok(stream) => {
                    // Un client s'est connecté
                    usersGestion::server_loop(&stream);
                }
                Err(e) => {
                    eprintln!("Erreur lors de l'acceptation de la connexion : {}", e);
                }
            }
        }
        drop(listener);

    }else{
        // Se connecter au serveur
        let stream: TcpStream = TcpStream::connect("127.0.0.1:1234").unwrap();
        
        // Etape 1: créer la structure
        let mut app = App::default();

        // Etape 2: on démarre la TUI
        app.start()?;

        loop {
            // Etape 3: on dessine l'application (à faire après chaque évènement lu,
            // y compris des changements de taille de la fenêtre !)
            app.draw()?;

            // Etape 4: on modifie l'état interne de l'application, en fonction des évènements
            // clavier / système. Ici, l'interface est très simple: suite à un évènement, soit:
            // - l'évènement est géré en interne de App, il n'y a rien à faire
            // - soit l'utilisateur veut quitter l'application, il faut interrompre la boucle et retourner
            // - soit l'utilisateur souhaite envoyer une commande verse le serveur

            // TODO par ailleurs, il faudra afficher (via push_message) les données reçues depuis le serveur
            if let Ok(e) = crossterm::event::read() {
                match app.react_to_event(e) {
                    Some(KeyReaction::Quit) => {
                        break;
                    }
                    Some(KeyReaction::UserInput(s)) => {
                        // TODO pour l'instant, le message à envoyer est simplement affiché localement
                        // Il faudra l'envoyer au serveur mini-ssh

                        //app.push_message(s);
                        match analyse_input(s,&stream,&mut connected) {
                            Ok(cmd)    =>{
                                match cmd.execute(){
                                    Ok(answer)=> app.push_message(answer),
                                    Err(er)=> app.push_message(er.to_string()),
                                }
                            },
                            Err(error)     => app.push_message(error),
                        }
                    }
                    None => {} // Rien à faire, géré en interne
                }
            }
        }
            
    }
    Ok(())
}
