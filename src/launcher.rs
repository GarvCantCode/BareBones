use std::env;
use std::fs;
use std::path::PathBuf;

pub struct Launcher {
    name: String,
}

impl Launcher {
    pub fn new() -> Launcher {
        Launcher { name: "BareBones".to_string() }
    }

    pub fn start(&self) {
        println!("Starting {}", self.name);
        self.install();

        match self.java_version_for("26.3") {
            Ok(version) => println!("26.3 needs Java {}", version),
            Err(e) => println!("Error: {}", e),
        }
    }

    fn java_version_for(&self, wanted: &str) -> Result<u64, String> {
        let manifest = self.fetch_json("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")?;
        let url = self
            .find_version_url(&manifest, wanted)
            .ok_or(format!("version {} is not in the manifest", wanted))?;

        let version_file = self.fetch_json(&url)?;
        version_file["javaVersion"]["majorVersion"]
            .as_u64()
            .ok_or("version file has no javaVersion".to_string())
    }

    fn find_version_url(&self, manifest: &serde_json::Value, wanted: &str) -> Option<String> {
        let versions = manifest["versions"].as_array()?;
        for entry in versions {
            if entry["id"] == wanted {
                let url = entry["url"].as_str()?;
                return Some(url.to_string());
            }
        }
        None
    }

    fn fetch_json(&self, url: &str) -> Result<serde_json::Value, String> {
        let text = self
            .fetch_text(url)
            .map_err(|e| format!("download of {} failed: {}", url, e))?;
        serde_json::from_str(&text).map_err(|e| format!("could not parse {}: {}", url, e))
    }

    fn fetch_text(&self, url: &str) -> Result<String, reqwest::Error> {
        let body = reqwest::blocking::get(url)?.text()?;
        Ok(body)
    }

    fn install(&self) {
        match self.minecraft_dir() {
            Ok(dir) => match fs::create_dir_all(&dir) {
                Ok(()) => println!("Created .minecraft in Path: {}", dir.display()),
                Err(e) => println!("Error while creating directory. {}", e)
            }
            Err(e) => println!("Cannot find the APPDATA path: {}", e)
        }
    }

    fn minecraft_dir(&self) -> Result<PathBuf, env::VarError> {
        let appdata = env::var("APPDATA")?;
        Ok(PathBuf::from(appdata).join(".minecraft"))
    }
}