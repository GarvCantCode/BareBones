use std::env;
use std::path::PathBuf;
use std::fs;
use reqwest;

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
        
        match self.fetch_manifest() {
            Ok(manifest) => {
                println!("Bytes arrived: {}", manifest.len())
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

    fn fetch_manifest(&self) -> Result<String, reqwest::Error> {
        // fetching minecraft manifest
        let manifest = reqwest::blocking::get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")?.text()?;
        Ok(manifest)
    }

}