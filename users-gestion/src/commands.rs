use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum CmdTypes {
    Cd(PathBuf),
    Ls(bool),
    Cat(PathBuf),
    Mkdir(PathBuf),
}

impl CmdTypes {
    pub fn execute(&self) -> Result<String, std::io::Error> {
        match self {
            CmdTypes::Cd(path)      => cd(path),
            CmdTypes::Ls(all)          => ls(*all),
            CmdTypes::Cat(path)     => cat(path),
            CmdTypes::Mkdir(path)   => mkdir(path),                                 
        }
    }
}

pub fn start_cmd(id: u32 )-> Result<(), std::io::Error> {
   // Aller dans le dossier "users" du dossier courant
   let mut users_dir = std::env::current_dir()?;
   users_dir.push("users");

   // Aller dans le dossier correspondant à l'ID dans le dossier "users"
   let mut user_dir = users_dir.clone();
   user_dir.push(id.to_string());

   // Aller dans le dossier "home" du dossier utilisateur
   let mut home_dir = user_dir.clone();
   home_dir.push("home");

   // Changer le répertoire de travail courant vers le dossier "home"
   env::set_current_dir(home_dir)?;

   Ok(())
}

fn go_back(mut cur_path: PathBuf) -> Result<PathBuf, std::io::Error> {
    if !cur_path.pop() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Cannot go back from the root directory",
        ));
    }

    // Vérifier si nous sommes dans le dossier "home"
    if cur_path.ends_with("home") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Cannot go back above the home directory",
        ));
    }

    Ok(cur_path)
}

fn cd(file_path: &Path) -> Result<String, std::io::Error> {
    let current_dir = std::env::current_dir()?;
    let mut user_dir = current_dir;
    // Vérifier si le chemin est relatif
    if file_path.is_relative() {
        for component in file_path.iter() {
            if let Some(component_str) = component.to_str() {
                if component_str==".." {
                    user_dir = go_back(user_dir)?;
                }else{
                    user_dir.push(component);
                }
            }
        }
        env::set_current_dir(user_dir.clone())?;
        Ok(format!("Répertoire changé avec succès: {:?}", user_dir))
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "La commande 'cd' ne peut pas être utilisée avec des chemins absolus. Veuillez fournir un chemin relatif vers le fichier.",
        ))
    }
}

fn ls(all: bool) -> Result<String, std::io::Error> {
    // Récupérer le répertoire de travail courant
    let current_dir = std::env::current_dir()?;
    let mut output = format!("Contenu du répertoire : ");

    // Lister le contenu du répertoire
    let entries = fs::read_dir(current_dir)?;

    // Parcourir les entrées du répertoire et les accumuler dans une chaîne de caractères
    for entry in entries {
        let entry = entry?;
        if all || entry.file_name().to_string_lossy().chars().next().unwrap() != '.' {
            output.push_str(&format!("{}  ", entry.file_name().to_string_lossy()));
        }
    }
    Ok(output)
}


fn cat(file_path: &Path) -> Result<String, std::io::Error> {
    // Vérifie pour ne pas revenir en arrière avec cat
    for component in file_path.iter() {
        if let Some(component_str) = component.to_str() {
            if component_str==".." {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Le chemin ne doit pas contenir de retour en arrière 'cat'.",
                ));
            }
        }
    }

    // Vérifier si le chemin est relatif
    if file_path.is_relative() {
        // Lire le contenu du fichier
        let content = fs::read_to_string(file_path)?;

        // Afficher le contenu du fichier
        println!("{}", content);

        Ok(content)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "La commande 'cat' ne peut pas être utilisée avec des chemins absolus. Veuillez fournir un chemin relatif vers le fichier.",
        ))
    }
}

/* 
        // cherche si il y à des extentions
        // comme il n'y à pas de couleurs il est préférable de ne pas avoir d'extentions pour les dossiers

        // on ne veux pas être capable de créé un dossier "home" 
        // important pour mon modèle de base de donnée
*/
fn mkdir(path: &Path) -> Result<String, std::io::Error> {
    for component in path.iter() {
        // Vérifier les extensions
        if let Some(component_str) = component.to_str() {
            if component_str.contains('.') {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "Les dossiers ne doivent pas avoir d'extension",
                ));
            }
        }

        // Vérifier si le composant est "home"
        if component == "home" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Le dossier 'home' ne peut pas être créé",
            ));
        }
    }

    // Créer le répertoire
    fs::create_dir_all(path)?;

    println!("Dossier créé : {}", path.display());
    Ok(format!("{:?}", path))
}
