use std::path::PathBuf;

pub struct EnvPath {
    includes: Vec<String>,
    pub executables: Vec<Executable>,
}

#[derive(Clone)]
pub struct Executable {
    pub path: String,
    pub name: String,
}

impl EnvPath {
    pub fn new(path: &str) -> Self {
        let mut includes = Vec::new();
        let mut executables = Vec::new();

        for item in path.split(':') {
            let path = PathBuf::from(item);
            if path.is_dir() {
                let items = std::fs::read_dir(path).unwrap();
                for item in items {
                    let item = item.unwrap();
                    let path = item.path().to_str().unwrap().to_string();
                    let name = item.file_name().into_string().unwrap();
                    executables.push(Executable { name, path });
                }
                includes.push(item.to_string());
            }
        }

        Self {
            includes,
            executables,
        }
    }
}

impl Executable {
    pub fn run(&self) -> String {
        // impl Future<Output = String> {
        // async move {
        //     let o = std::process::Command::new(self.path.clone())
        //         .output()
        //         .expect("failed to execute process").clone();
        //     return String::from_utf8_lossy(&o.stdout).to_string();
        // }
        std::process::Command::new(self.path.clone())
            .output()
            .expect("failed to execute process")
            .stdout
            .iter()
            .map(|&c| c as char)
            .collect::<String>()
    }
}
