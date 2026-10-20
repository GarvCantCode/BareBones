use std::env;
use std::fs;
use std::path::Path;
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

        match self.version_file("26.3") {
            Ok(version) => {
                match self.download_client("26.3", &version) {
                    Ok(m) => {println!(" I DONT KNOW");}
                    Err(e) => println!("{}", e)
                }
            }
            Err(e) => println!("Could not load 26.3 version file: {}", e),
        }
    }

    fn version_file(&self, wanted: &str) -> Result<serde_json::Value, String> {
        let manifest = self.fetch_json("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")?;
        let url = self
            .find_version_url(&manifest, wanted)
            .ok_or(format!("version {} is not in the manifest", wanted))?;

        let version_file = self.fetch_json(&url)?;
        Ok(version_file)
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
        match self.version_dir("26.3") {
            Ok(dir) => match fs::create_dir_all(&dir) {
                Ok(()) => println!("BareBones folder ready: {}", dir.display()),
                Err(e) => println!("Error while creating directory. {}", e)
            }
            Err(e) => println!("Cannot find the APPDATA path: {}", e)
        }
    }

    fn data_dir(&self) -> Result<PathBuf, env::VarError> {
        let appdata = env::var("APPDATA")?;
        Ok(PathBuf::from(appdata).join("BareBones"))
    }

    fn version_dir(&self, version: &str) -> Result<PathBuf, env::VarError> {
        let base = self.data_dir()?;
        Ok(base.join("versions").join(version))
    }

    fn download_file(&self, url: &str, destination: &Path) -> Result<(), String> {
        let binary = reqwest::blocking::get(url)
            .map_err(|e| format!("download of {} failed: {}", url, e))?
            .bytes().map_err(|e| format!("download of {} failed: {}", url, e))?;

        let path = format!("{}.part", destination.display());
        let temp_path = PathBuf::from(path);

        fs::write(&temp_path, &binary).map_err(|e| format!("could not save {} file: {}", destination.display(), e))?;
        fs::rename(&temp_path, destination).map_err(|e| format!("could not finish {} file: {}", destination.display(), e))?;
        Ok(())
    }

    fn download_client(&self, id: &str, version_file: &serde_json::Value) -> Result<(), String> {
        let client_url = version_file["downloads"]["client"]["url"]
            .as_str()
            .ok_or(format!("version file does not have client url"))?;

        let destination = self.version_dir(id).map_err(|e | format!("could not find the BareBones folder"))?.join("client.jar");

        println!("Downloading client.jar");
        self.download_file(client_url, &destination)?;
        println!("Downloded client.jar");
        Ok(())
    }
}