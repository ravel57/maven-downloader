mod dependencies;
mod dependency;
mod project;
mod properties;

use std::collections::HashMap;
use crate::dependency::Dependency;
use async_recursion::async_recursion;
use project::Project;
use std::env;
use std::fs;
use std::path::Path;
use regex::Regex;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: maven-downloader <path-to-pom.xml>");
        std::process::exit(1);
    }
    let path = &args[1];

    let mut props = HashMap::new();
    process_file(path).await.expect("ERROR processing file");
}

#[async_recursion]
async fn process_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let xml = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Не удалось открыть файл {}: {}", path, e));
    let project: Project = quick_xml::de::from_str(&xml)?;

    let mut props = HashMap::new();
    if let Some(p) = project.properties {
        props.extend(p.values);
    }
    if let Some(v) = project.version.clone() {
        props.insert("project.version".to_string(), v);
    }

    if let Some(parent) = project.parent {
        process_artifact(parent, &props)
    }.await.expect("");
    for dep in project.dependencies.expect("").dependency {
        process_artifact(dep, &props).await.expect("");
    }

    Ok(())
}

async fn process_artifact(dep: Dependency, props: &HashMap<String, String>) -> Result<(), Box<dyn std::error::Error>> {
    let version_raw = dep.version.clone().unwrap_or_else(|| {
        props.get("project.version").cloned().unwrap_or("UNKNOWN".to_string())
    });
    let version = resolve_placeholders(&version_raw, props);

    let local_repository_path = if let Some(mut path) = dirs::home_dir() {
        path.push(".m2");
        path.push("repository");
        path.to_string_lossy().into_owned()
    } else {
        panic!("Не удалось найти домашнюю папку");
    };
    let maven_repository_path = "https://repo.maven.apache.org/maven2";

    let jar_path = format!(
        "{}/{}/{}/{}-{}.jar",
        &dep.group_id.replace(".", "/"),
        &dep.artifact_id,
        &dep.version,
        &dep.artifact_id,
        &dep.version,
    );
    let local_jar_path = format!("{local_repository_path}/{jar_path}");
    if !Path::new(&local_jar_path).exists() {
        let url = format!("{maven_repository_path}/{jar_path}");
        download_artifact(&local_jar_path, &url)
            .await
            .expect("ERROR downloading jar");
    }

    let pom_path = format!(
        "{}/{}/{}/{}-{}.pom",
        &dep.group_id.replace(".", "/"),
        &dep.artifact_id,
        &dep.version,
        &dep.artifact_id,
        &dep.version,
    );
	let local_pom_path = format!("{local_repository_path}/{pom_path}");

    if !Path::new(&local_pom_path).exists() {
        let url = format!("{maven_repository_path}/{pom_path}");
        download_artifact(&local_pom_path, &url).await
            .await
            .expect("ERROR downloading pom");
        process_file(local_pom_path.as_str())
            .await
            .expect("ERROR processing file");
    }

    Ok(())
}
async fn download_artifact(path: &str, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading {}", path);

    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        eprintln!("Ошибка: {} при скачивании {}", response.status(), url);
        return Ok(()); // не падаем
    }

    let bytes = response.bytes().await?;

    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, &bytes)?;

    Ok(())
}

fn resolve_placeholders(s: &str, props: &HashMap<String, String>) -> String {
    let re = Regex::new(r"\$\{([^}]+)}").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let key = &caps[1];
        props.get(key).cloned().unwrap_or(caps[0].to_string())
    }).to_string()
}
