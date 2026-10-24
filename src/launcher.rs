// BareBones is Windows-only: building on any other OS stops with this message
#[cfg(not(windows))]
compile_error!("BareBones only supports Windows");

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use serde_json::Value;

const VERSION: &str = "26.3";
const ASSET_BASE_URL: &str = "https://resources.download.minecraft.net";
const OFFLINE_UUID: &str = "2f5b6c1e-8a3d-4e7a-9b0c-5d1e6f7a8b9c"; // offline profle

pub struct Launcher {
    name: String,
    username: String,
    http: reqwest::blocking::Client,
}

impl Launcher {
    pub fn new() -> Launcher {
        Launcher {
            name: "BareBones".to_string(),
            username: "Player".to_string(),
            http: reqwest::blocking::Client::new(),
        }
    }

    pub fn start(&self) {
        println!("Starting {}", self.name);
        self.install();

        if let Err(e) = self.run(VERSION) {
            println!("Error: {}", e);
        }
    }

    // runs the whole client code
    fn run(&self, id: &str) -> Result<(), String> {
        let version = self.version_file(id)?;
        self.download_client(id, &version)?;

        let libraries = self.windows_libraries(&version)?;
        println!("{} libraries apply to Windows", libraries.len());
        self.download_libraries(&libraries)?;

        self.download_assets(&version)?;
        self.launch(id, &version, &libraries)
    }

    // ---------- version manifest ----------

    fn version_file(&self, wanted: &str) -> Result<Value, String> {
        let manifest = self.fetch_json("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")?;
        let url = self
            .find_version_url(&manifest, wanted)
            .ok_or(format!("version {} is not in the manifest", wanted))?;

        let version_file = self.fetch_json(&url)?;
        Ok(version_file)
    }

    fn find_version_url(&self, manifest: &Value, wanted: &str) -> Option<String> {
        let versions = manifest["versions"].as_array()?;
        for entry in versions {
            if entry["id"] == wanted {
                let url = entry["url"].as_str()?;
                return Some(url.to_string());
            }
        }
        None
    }

    fn fetch_json(&self, url: &str) -> Result<Value, String> {
        let text = self
            .fetch_text(url)
            .map_err(|e| format!("download of {} failed: {}", url, e))?;
        serde_json::from_str(&text).map_err(|e| format!("could not parse {}: {}", url, e))
    }

    fn fetch_text(&self, url: &str) -> Result<String, reqwest::Error> {
        let body = self.http.get(url).send()?.text()?;
        Ok(body)
    }

    // ---------- folders ----------

    fn install(&self) {
        match self.version_dir(VERSION) {
            Ok(dir) => match fs::create_dir_all(&dir) {
                Ok(()) => println!("BareBones folder ready: {}", dir.display()),
                Err(e) => println!("Error while creating directory. {}", e),
            },
            Err(e) => println!("Cannot find the APPDATA path: {}", e),
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

    // a folder inside the BareBones data dir
    fn data_path(&self, sub: &str) -> Result<PathBuf, String> {
        let base = self
            .data_dir()
            .map_err(|e| format!("could not find the BareBones folder: {}", e))?;
        Ok(base.join(sub))
    }

    // ---------- downloading ----------

    fn download_file(&self, url: &str, destination: &Path) -> Result<(), String> {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("could not create folder {}: {}", parent.display(), e))?;
        }

        let fail = |e: reqwest::Error| format!("download of {} failed: {}", url, e);
        let binary = self
            .http
            .get(url)
            .send()
            .map_err(&fail)?
            .error_for_status() // a 404 page must not be saved as a jar
            .map_err(&fail)?
            .bytes()
            .map_err(&fail)?;

        let temp_path = PathBuf::from(format!("{}.part", destination.display()));

        fs::write(&temp_path, &binary)
            .map_err(|e| format!("could not save {} file: {}", destination.display(), e))?;
        fs::rename(&temp_path, destination)
            .map_err(|e| format!("could not finish {} file: {}", destination.display(), e))?;
        Ok(())
    }

    fn download_client(&self, id: &str, version_file: &Value) -> Result<(), String> {
        let client_url = version_file["downloads"]["client"]["url"]
            .as_str()
            .ok_or("version file does not have client url")?;

        let destination = self
            .version_dir(id)
            .map_err(|e| format!("could not find the BareBones folder: {}", e))?
            .join("client.jar");

        if destination.exists() {
            println!("client.jar already exists, skipping");
            return Ok(());
        }

        println!("Downloading client.jar");
        self.download_file(client_url, &destination)?;
        println!("Downloaded client.jar");
        Ok(())
    }

    // ---------- libraries ----------

    fn rule_matches(&self, rule: &Value) -> bool {
        if !rule["features"].is_null() {
            return false;
        }

        let os = &rule["os"];
        if os.is_null() {
            return true;
        }

        if let Some(name) = os["name"].as_str() {
            if name != "windows" {
                return false;
            }
        }

        if let Some(arch) = os["arch"].as_str() {
            let ok = match arch {
                "x86" => env::consts::ARCH == "x86",
                "x86_64" => env::consts::ARCH == "x86_64",
                "arm64" => env::consts::ARCH == "aarch64",
                _ => false,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    // no rules = allowed
    fn rules_allow(&self, rules: &Value) -> bool {
        let rules = match rules.as_array() {
            Some(r) => r,
            None => return true,
        };

        let mut allowed = false;
        for rule in rules {
            if self.rule_matches(rule) {
                allowed = rule["action"] == "allow";
            }
        }
        allowed
    }

    // returns path and url
    fn windows_libraries(&self, version_file: &Value) -> Result<Vec<(String, String)>, String> {
        let libraries = version_file["libraries"]
            .as_array()
            .ok_or("version file has no libraries list")?;

        let mut result = Vec::new();
        for library in libraries {
            if !self.rules_allow(&library["rules"]) {
                continue;
            }

            let name = library["name"].as_str().unwrap_or("unknown");
            let path = library["downloads"]["artifact"]["path"]
                .as_str()
                .ok_or_else(|| format!("library {} has no path", name))?;
            let url = library["downloads"]["artifact"]["url"]
                .as_str()
                .ok_or_else(|| format!("library {} has no url", name))?;

            result.push((path.to_string(), url.to_string()));
        }
        Ok(result)
    }

    fn download_libraries(&self, libraries: &[(String, String)]) -> Result<(), String> {
        let dir = self.data_path("libraries")?;
        let total = libraries.len();

        for (i, (path, url)) in libraries.iter().enumerate() {
            let destination = dir.join(path);
            if destination.exists() {
                continue;
            }
            println!("[{}/{}] {}", i + 1, total, path);
            self.download_file(url, &destination)?;
        }
        println!("Libraries ready");
        Ok(())
    }

    // ---------- assets ----------

    fn download_assets(&self, version_file: &Value) -> Result<(), String> {
        let index_id = version_file["assetIndex"]["id"]
            .as_str()
            .ok_or("version file has no asset index id")?;
        let index_url = version_file["assetIndex"]["url"]
            .as_str()
            .ok_or("version file has no asset index url")?;

        let assets_dir = self.data_path("assets")?;
        let index_path = assets_dir.join("indexes").join(format!("{}.json", index_id));

        if !index_path.exists() {
            println!("Downloading asset index {}", index_id);
            self.download_file(index_url, &index_path)?;
        }

        let text = fs::read_to_string(&index_path)
            .map_err(|e| format!("could not read {}: {}", index_path.display(), e))?;
        let index: Value = serde_json::from_str(&text)
            .map_err(|e| format!("could not parse {}: {}", index_path.display(), e))?;
        let objects = index["objects"]
            .as_object()
            .ok_or("asset index has no objects")?;

        let total = objects.len();
        let mut downloaded = 0;
        for (i, (name, object)) in objects.iter().enumerate() {
            let hash = object["hash"]
                .as_str()
                .ok_or_else(|| format!("asset {} has no hash", name))?;
            if hash.len() < 2 {
                return Err(format!("asset {} has a bad hash", name));
            }
            let prefix = &hash[..2];

            let destination = assets_dir.join("objects").join(prefix).join(hash);
            if destination.exists() {
                continue;
            }

            let url = format!("{}/{}/{}", ASSET_BASE_URL, prefix, hash);
            self.download_file(&url, &destination)?;

            downloaded += 1;
            if downloaded % 100 == 0 {
                println!("assets: {}/{}", i + 1, total);
            }
        }
        println!("Assets ready ({} total)", total);
        Ok(())
    }

    // ---------- launching ----------

    // replaces every ${key} in the text with its value
    fn fill(&self, text: &str, vars: &[(&str, String)]) -> String {
        let mut result = text.to_string();
        for (key, value) in vars {
            result = result.replace(&format!("${{{}}}", key), value);
        }
        result
    }

    // turns the "jvm" or "game" list from the version file into plain strings
    fn collect_args(&self, list: &Value, vars: &[(&str, String)]) -> Result<Vec<String>, String> {
        let items = list
            .as_array()
            .ok_or("version file has no argument list")?;

        let mut args = Vec::new();
        for item in items {
            // plain string argument
            if let Some(text) = item.as_str() {
                args.push(self.fill(text, vars));
                continue;
            }

            // object argument: has rules, and a value that is a string or a list
            if !self.rules_allow(&item["rules"]) {
                continue;
            }
            match &item["value"] {
                Value::String(s) => args.push(self.fill(s, vars)),
                Value::Array(values) => {
                    for v in values {
                        if let Some(s) = v.as_str() {
                            args.push(self.fill(s, vars));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(args)
    }

    fn java_path(&self) -> String {
        match env::var("JAVA_HOME") {
            Ok(home) => format!("{}\\bin\\java.exe", home),
            Err(_) => "java".to_string(),
        }
    }

    fn launch(
        &self,
        id: &str,
        version_file: &Value,
        libraries: &[(String, String)],
    ) -> Result<(), String> {
        let version_dir = self
            .version_dir(id)
            .map_err(|e| format!("could not find the BareBones folder: {}", e))?;
        let libraries_dir = self.data_path("libraries")?;
        let assets_dir = self.data_path("assets")?;
        let game_dir = self.data_path("game")?;
        let natives_dir = version_dir.join("natives");
        let client_jar = version_dir.join("client.jar");

        fs::create_dir_all(&game_dir)
            .map_err(|e| format!("could not create {}: {}", game_dir.display(), e))?;
        fs::create_dir_all(&natives_dir)
            .map_err(|e| format!("could not create {}: {}", natives_dir.display(), e))?;

        // classpath = every library jar + the client jar, separated by ; on Windows
        let mut entries: Vec<String> = Vec::new();
        for (path, _) in libraries {
            entries.push(libraries_dir.join(path).display().to_string());
        }
        entries.push(client_jar.display().to_string());
        let classpath = entries.join(";");

        let asset_index = version_file["assetIndex"]["id"]
            .as_str()
            .ok_or("version file has no asset index id")?;
        let main_class = version_file["mainClass"]
            .as_str()
            .ok_or("version file has no mainClass")?;
        let version_type = version_file["type"].as_str().unwrap_or("release");

        let vars: Vec<(&str, String)> = vec![
            ("auth_player_name", self.username.clone()),
            ("version_name", id.to_string()),
            ("game_directory", game_dir.display().to_string()),
            ("assets_root", assets_dir.display().to_string()),
            ("assets_index_name", asset_index.to_string()),
            ("auth_uuid", OFFLINE_UUID.to_string()),
            ("auth_access_token", "0".to_string()),
            ("clientid", "0".to_string()),
            ("auth_xuid", "0".to_string()),
            ("user_type", "legacy".to_string()),
            ("version_type", version_type.to_string()),
            ("natives_directory", natives_dir.display().to_string()),
            ("launcher_name", self.name.clone()),
            ("launcher_version", "0.1".to_string()),
            ("classpath", classpath),
        ];

        let jvm_args = self.collect_args(&version_file["arguments"]["jvm"], &vars)?;
        let game_args = self.collect_args(&version_file["arguments"]["game"], &vars)?;

        println!("Launching {} as {}", id, self.username);

        let status = Command::new(self.java_path())
            .arg("-Xms1G")
            .arg("-Xmx2G")
            .args(&jvm_args)
            .arg(main_class)
            .args(&game_args)
            .current_dir(&game_dir)
            .status()
            .map_err(|e| format!("could not start java: {}", e))?;

        println!("Game exited with: {}", status);
        Ok(())
    }
}