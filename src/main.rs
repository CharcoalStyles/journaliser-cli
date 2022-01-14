// Note: this requires the `derive` feature

use clap::{AppSettings, Parser, Subcommand};
use reqwest::header::CONTENT_TYPE;
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::io::{stdin, Write};

#[derive(Parser)]
#[clap(setting(AppSettings::InferSubcommands), about, version, author)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Add a note")]
    Note {
        /// Note Type, if supplied will be checked against available types
        #[clap(short = 'n')]
        note_type: Option<String>,

        /// Collection, if supplied will be checked against available collections
        #[clap(short = 'c')]
        collection: Option<String>,

        /// The actual body of the note
        #[clap()]
        message: Vec<String>,
    },
    #[clap(about = "List the available Note Types, Note Mods, Collections, and the Defaults")]
    Types {},
    #[clap(about = "View or update the configuration")]
    Config {
        /// Update the configuration
        #[clap(short, long)]
        update: bool,
    },
}

fn main() {
    let args = Cli::parse();

    match &args.command {
        Commands::Note {
            collection,
            message,
            note_type,
        } => {
            let config = get_config().unwrap();

            let body = message
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
            println!("concat Message: {:?}", body);

            let note_types = get_note_types(&config.url).unwrap();
            // let note_mods = get_note_mods(&config.url).unwrap();
            let collections = get_collections(&config.url).unwrap();
            let defaults = get_defaults(&config.url).unwrap();

            let note_type = note_type
                .clone()
                .unwrap_or(defaults.defaultNoteType.to_string());
            let collection = collection
                .clone()
                .unwrap_or(defaults.defaultCollection.to_string());

            let final_note_type = get_final_note_type(&note_type, &note_types);
            println!("final_note_type: {:?}", final_note_type);

            let final_collection_id = get_final_collection_id(&collection, &collections);
            println!("final_collection_id: {:?}", final_collection_id);

            let post_body: NotePostBody = NotePostBody {
                noteType: final_note_type,
                collectionId: final_collection_id,
                body: body,
                modifiers: [].to_vec(),
            };

            let client = reqwest::blocking::Client::new();
            let res = client
                .post(format!("{}{}", config.url, "api/note"))
                // .header(CONTENT_TYPE, "application/json")
                .body(serde_json::to_string(&post_body).unwrap())
                .send();

            match res {
                Ok(res) => {
                    println!("Status: {}", res.status());
                    println!("Body: {}", res.text().unwrap());
                }
                Err(e) => {
                    println!("Error: {}", e);
                }
            }
            return;
        }
        Commands::Types {} => {
            let config = get_config().unwrap();
            let note_types = get_note_types(&config.url).unwrap();
            let note_mods = get_note_mods(&config.url).unwrap();
            let collections = get_collections(&config.url).unwrap();
            let defaults = get_defaults(&config.url).unwrap();

            println!("Config:");
            println!("\t{:#?}", config.url);
            println!("Note Types:");
            note_types.iter().for_each(|nt| println!("\t{:?}", nt.name));
            println!("Note Mods:");
            note_mods.iter().for_each(|nm| println!("\t{:?}", nm));
            println!("Collections:");
            collections
                .iter()
                .for_each(|c| println!("\t{:?}", c.name));
            println!("Defaults:");
            println!("\tdefaultNoteType: {:?}", defaults.defaultNoteType);
            println!("\tdefaultCollection: {:?}", defaults.defaultCollection);
        }
        Commands::Config { update } => match update {
            true => {
                let raw_config = get_config();
                let mut config: Config = Config {
                    url: "".to_string(),
                };
                match raw_config.ok() {
                    Some(cfg) => {
                        config = cfg;
                        println!("{:#?}", config);
                    }
                    None => {
                        println!("Current config is missing or invalid");
                    }
                }

                let mut s = String::new();
                println!("Current URL: {:?}", config.url);
                println!("Enter new URL (or blank to not change):");
                stdin()
                    .read_line(&mut s)
                    .expect("Did not enter a correct string");

                match s.trim() {
                    "" => println!("No change"),
                    _ => {
                        let new_url = s.trim().to_string();
                        config.url = new_url;
                    }
                }

                let config_str = serde_json::to_string(&config).unwrap();
                let mut file = fs::File::create(get_config_file().unwrap()).unwrap();
                file.write_all(config_str.as_bytes()).unwrap();
            }
            false => {
                let config = get_config().unwrap();
                println!("{:#?}", config);
            }
        },
    }
}

fn get_config() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(get_config_file().unwrap())
        .expect("Something went wrong reading the file");
    let config: Config = serde_json::from_str(&contents)?;
    Ok(config)
}

fn get_home_dir() -> Result<String, Box<dyn std::error::Error>> {
    match home::home_dir() {
        Some(path) => Ok(path.to_str().unwrap().to_string()),
        None => Err("Impossible to get your home dir!".into()),
    }
}
fn get_config_file() -> Result<String, Box<dyn std::error::Error>> {
    let config_file = get_home_dir().unwrap() + "/.jlzrc";
    Ok(config_file)
}

fn vec_contains(v: &Vec<String>, e: &String) -> Vec<String> {
    v.iter()
        .filter(|x| {
            x.to_ascii_lowercase()
                .contains(e.to_ascii_lowercase().as_str())
        })
        .map(|x| x.to_string())
        .collect::<Vec<String>>()
}

fn get_final_note_type(note_type: &String, note_types: &Vec<NoteType>) -> String {
    let final_note_type = &note_types
    .iter()
    .filter(|x| {
        x.name
            .to_ascii_lowercase()
            .contains(note_type.to_ascii_lowercase().as_str())
    })
    .collect::<Vec<&NoteType>>();
    
    match final_note_type.len() {
        1 => {
            return final_note_type[0].id.to_string();
        }
        _ => {
            return "".to_string();
        }
    }
}

fn get_final_collection_id(collection: &String, collections: &Vec<Collection>) -> String {
    let final_collection = &collections
        .iter()
        .filter(|x| {
            x.name
                .to_ascii_lowercase()
                .contains(collection.to_ascii_lowercase().as_str())
        })
        .collect::<Vec<&Collection>>();
    match final_collection.len() {
        1 => {
            return final_collection[0].id.to_string();
        }
        _ => {
            return "".to_string();
        }
    }
}

fn get_note_types(url: &String) -> Result<Vec<NoteType>, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(format!("{}{}", url, "api/note-type"))?;
    let res = resp.json::<NoteTypeResponse>().unwrap();

    Ok(res.types)
}

fn get_note_mods(url: &String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(format!("{}{}", url, "api/note-mods"))?;
    let res = resp.json::<NoteModsResponse>().unwrap();

    Ok(res.mods)
}

fn get_collections(url: &String) -> Result<Vec<Collection>, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(format!("{}{}", url, "api/collection"))?;
    let res = resp.json::<CollectionsResponse>().unwrap();

    Ok(res.collections)
}

fn get_defaults(url: &String) -> Result<Defaults, Box<dyn std::error::Error>> {
    let resp = reqwest::blocking::get(format!("{}{}", url, "api/defaults"))?;
    let res = resp.json::<Defaults>().unwrap();

    Ok(res)
}

#[derive(Deserialize, Debug)]
struct NoteTypeResponse {
    types: Vec<NoteType>,
}

#[derive(Deserialize, Debug)]
struct NoteModsResponse {
    mods: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct CollectionsResponse {
    collections: Vec<Collection>,
}

#[derive(Deserialize, Debug)]
struct Collection {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct NoteType {
    id: String,
    name: String,
}

#[derive(Deserialize, Debug)]
struct Defaults {
    defaultCollection: String,
    defaultNoteType: String,
}

#[derive(Deserialize, Debug, Serialize)]
struct Config {
    url: String,
}

#[derive(Debug, Serialize)]
struct NotePostBody {
    body: String,
    noteType: String,
    collectionId: String,
    modifiers: Vec<String>,
}
