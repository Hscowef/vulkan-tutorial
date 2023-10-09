use std::fs;
use std::process::Command;

const GLSLC_PATH: &str = "C:/VulkanSDK/1.3.250.0/Bin/glslc.exe";

fn main() {
    let paths = fs::read_dir("./src/shaders").unwrap();
    for shader in paths {
        let path = shader.unwrap().path();
        let file_name = path.file_stem().unwrap();
        let output_path: String = format!("./src/spirv/{}.spv", file_name.to_str().unwrap());

        let output = Command::new(GLSLC_PATH)
            .arg(path)
            .arg("-o")
            .arg(output_path)
            .output()
            .unwrap();

        if !output.stderr.is_empty() {
            println!("############## SHADER COMPILATION FAILED ##############");
            println!("{}", String::from_utf8(output.stderr).unwrap());
            println!("#######################################################");
        }
    }
}
