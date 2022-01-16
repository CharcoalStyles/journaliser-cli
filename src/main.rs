// Note: this requires the `derive` feature

use clap::{AppSettings, Parser, Subcommand};
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
    Data {
        #[clap(short, long)]
        add: bool,

        #[clap(short = 't', long)]
        data_type: Option<String>,

        #[clap()]
        value: Option<String>,
    },
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
            let note_mods = get_note_mods(&config.url).unwrap();
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

            let collected_mods = collect_mods(&message);

            let post_body: NotePostBody = NotePostBody {
                noteTypeId: final_note_type,
                collectionId: final_collection_id,
                body: body,
                modifiers: collected_mods,
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
        Commands::Data {
            add,
            data_type,
            value,
        } => {
            let config = get_config().unwrap();
            let note_types = get_note_types(&config.url).unwrap();
            let note_mods = get_note_mods(&config.url).unwrap();
            let collections = get_collections(&config.url).unwrap();
            let defaults = get_defaults(&config.url).unwrap();
            match add {
                true => {
                    println!("data_type: {:?}", data_type);
                    println!("value: {:?}", value);

                    let data_type = data_type.clone().unwrap_or("".to_string());

                    match data_type.as_str() {
                        "NoteType" => {
                            let note_type = value.clone().unwrap();
                            let post_body = NoteTypePostBody {
                                noteTypeName: value.clone().unwrap(),
                            };
                            let final_note_type = get_final_note_type(&note_type, &note_types);
                            match final_note_type.as_str() {
                                "" => {
                                    let client = reqwest::blocking::Client::new();
                                    let res = client
                                        .post(format!("{}{}", config.url, "api/note-type"))
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
                                _ => {
                                    println!("Note type already exists");
                                    return;
                                }
                            }
                        }
                        "NoteMod" => {
                            println!("Not yet implemented");
                            return;
                        }
                        "Collection" => {
                            println!("Not yet implemented");
                            return;
                        }
                        _ => {
                            println!("Unknown data type: {:?}", data_type);
                            return;
                        }
                    }
                }
                false => {
                    println!("Config:");
                    println!("\t{:#?}", config.url);
                    println!("Note Types:");
                    note_types.iter().for_each(|nt| println!("\t{:?}", nt.name));
                    println!("Note Mods:");
                    note_mods
                        .iter()
                        .for_each(|nm| println!("\t{:?} - {:?}", nm.char, nm.name));
                    println!("Collections:");
                    collections.iter().for_each(|c| println!("\t{:?}", c.name));
                    println!("Defaults:");
                    println!("\tdefaultNoteType: {:?}", defaults.defaultNoteType);
                    println!("\tdefaultCollection: {:?}", defaults.defaultCollection);
                }
            }
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

fn collect_mods(body: &Vec<String>) -> Vec<String> {
    let mut mods = vec![];
    for line in body {
        if line.len() == 1 {
            mods.push(line.to_string());
        } else {
            break;
        }
    }
    mods
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

fn get_note_mods(url: &String) -> Result<Vec<NoteMod>, Box<dyn std::error::Error>> {
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
    mods: Vec<NoteMod>,
}

#[derive(Deserialize, Debug)]
struct CollectionsResponse {
    collections: Vec<Collection>,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Collection {
    id: String,
    name: String,
    otherDateRequired: bool,
}

#[derive(Deserialize, Debug)]
struct NoteType {
    id: String,
    name: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct NoteMod {
    name: String,
    char: String,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct Defaults {
    defaultCollection: String,
    defaultNoteType: String,
}

#[derive(Deserialize, Debug, Serialize)]
struct Config {
    url: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct NotePostBody {
    body: String,
    noteTypeId: String,
    collectionId: String,
    modifiers: Vec<String>,
}

#[allow(non_snake_case)]
#[derive(Debug, Serialize)]
struct NoteTypePostBody {
    noteTypeName: String,
}
