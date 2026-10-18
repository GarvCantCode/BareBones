use std::env;
use std::path::PathBuf;
use std::fs;

pub struct Launcher {
    name: String,
}

impl Launcher {
    pub fn new() -> Launcher {
        Launcher { name: "BareBones".to_string(),}
    }

    pub fn start(&self) {
        println!("Starting {}", self.name);
        self.install();
        
        match self.fetch_text("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json") {
            Ok(manifest) => {
                println!("Bytes arrived of manifest: {}", manifest.len());

                let manifest_json: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(&manifest);

                match manifest_json {
                    Ok(parsed) => {
                        let versions  = parsed["versions"].as_array();

                        if let Some(value) = versions {
                            for i in value {
                                if i["id"] == "26.3" {
                                    if let Some(url) = i["url"].as_str() {
                                        match self.fetch_text(url) {
                                            Ok(file) => {
                                                let version_json: Result<serde_json::Value, serde_json::Error> = serde_json::from_str(&file);

                                                match version_json {
                                                    Ok(parsed) => {println!("{}", parsed["javaVersion"]["majorVersion"])}
                                                    Err(e) => {println!("Cannot parse 26.3 version file: {}", e)}
                                                }
                                            }
                                            Err(e) => {println!("Error fetching 26.3 version file: {}", e)}
                                        }
                                    }
                                    else {
                                        println!("Cannot get the url of 26.3")
                                    }

                                    break;
                                }
                            }
                        }
                        else {
                            println!("Error: manifest 'versions' field is missing or not a list.")
                        }
                        // println!("{}", parsed["versions"][0]["url"]);
                    }
                    Err(e) => println!("Error parsing version manifest: {}", e)
                }
                
            }
            Err(e) => {
                println!("Error while fetching minecraft manifest: {e}")
            }
        }
    }

    fn install(&self) {
        // primary install function for minecraft
        match self.minecraft_dir() {
            Ok(dir) => {
                match fs::create_dir_all(&dir) {
                    Ok(()) => {
                        println!("Created .minecraft in Path: {}", dir.display())
                    }
                    Err(e) => {
                        println!("Error while creating directory. {}", e)
                    }
                }
            }
            Err(e) => {
                println!("Cannot find the APPDATA path: {}", e)
            }
        }
    }

    fn minecraft_dir(&self) -> Result<PathBuf, env::VarError> { 
        // checks for APPDATA and if true joins .minecraft
        let appdata = env::var("APPDATA")?;
        Ok(PathBuf::from(appdata).join(".minecraft"))
    }

    fn fetch_text(&self, url: &str) -> Result<String, reqwest::Error> {
        let idfk = reqwest::blocking::get(url)?.text()?;
        Ok(idfk)
    }

}