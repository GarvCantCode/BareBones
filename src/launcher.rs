use std::env;
use std::path::PathBuf;
use std::fs;

pub struct Launcher {
    name: String,
}

impl Launcher {
    pub fn start(&self) {
        println!("Starting {}", self.name);
        self.install();
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

    pub fn new() -> Launcher {
        Launcher { name: "BareBones".to_string(),}
    }

}