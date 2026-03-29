use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use std::collections::HashMap;
use std::path::Path;

use crate::resolvers::{file_to_package, Resolver};
use crate::types::{Ecosystem, Package, PackageId, ProjectGraph};

/// MavenResolver detects Maven multi-module projects via `pom.xml` with `<modules>`.
///
/// Uses `quick-xml` for XML parsing. Walks the XML events manually to extract
/// `<modules>/<module>`, `<groupId>`, `<artifactId>`, and `<dependencies>/<dependency>`.
pub struct MavenResolver;

impl Resolver for MavenResolver {
    fn ecosystem(&self) -> Ecosystem {
        Ecosystem::Maven
    }

    fn detect(&self, root: &Path) -> bool {
        let pom = root.join("pom.xml");
        if !pom.exists() {
            return false;
        }
        std::fs::read_to_string(&pom)
            .map(|c| c.contains("<modules>"))
            .unwrap_or(false)
    }

    fn resolve(&self, root: &Path) -> Result<ProjectGraph> {
        let root_pom_path = root.join("pom.xml");
        let root_content = std::fs::read_to_string(&root_pom_path)
            .context("Failed to read root pom.xml")?;

        let root_info = parse_pom(&root_content)?;

        tracing::debug!(
            "Maven: root groupId={:?}, artifactId={:?}, {} modules",
            root_info.group_id,
            root_info.artifact_id,
            root_info.modules.len()
        );

        let root_group_id = root_info
            .group_id
            .clone()
            .unwrap_or_default();

        let mut packages = HashMap::new();
        let mut coord_to_id: HashMap<String, PackageId> = HashMap::new();

        for module_name in &root_info.modules {
            let module_dir = root.join(module_name);
            let module_pom_path = module_dir.join("pom.xml");
            if !module_pom_path.exists() {
                tracing::debug!("Maven: module '{}' has no pom.xml, skipping", module_name);
                continue;
            }

            let content = std::fs::read_to_string(&module_pom_path)
                .with_context(|| format!("Failed to read {}", module_pom_path.display()))?;
            let info = parse_pom(&content)?;

            let artifact_id = info
                .artifact_id
                .clone()
                .unwrap_or_else(|| module_name.clone());
            let group_id = info.group_id.clone().unwrap_or_else(|| root_group_id.clone());

            let pkg_id = PackageId(module_name.clone());
            let coord = format!("{}:{}", group_id, artifact_id);

            tracing::debug!("Maven: discovered module '{}' ({})", module_name, coord);

            coord_to_id.insert(coord, pkg_id.clone());
            packages.insert(
                pkg_id.clone(),
                Package {
                    id: pkg_id,
                    name: artifact_id,
                    version: info.version.clone(),
                    path: module_dir.clone(),
                    manifest_path: module_pom_path,
                },
            );
        }

        // Build dependency edges
        let mut edges = Vec::new();

        for module_name in &root_info.modules {
            let module_pom_path = root.join(module_name).join("pom.xml");
            if !module_pom_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&module_pom_path)?;
            let info = parse_pom(&content)?;

            let from_id = PackageId(module_name.clone());

            for dep in &info.dependencies {
                let dep_group = dep.group_id.as_deref().unwrap_or("");
                let dep_artifact = dep.artifact_id.as_deref().unwrap_or("");
                let dep_coord = format!("{}:{}", dep_group, dep_artifact);

                if let Some(to_id) = coord_to_id.get(&dep_coord) {
                    edges.push((from_id.clone(), to_id.clone()));
                }
            }
        }

        Ok(ProjectGraph {
            packages,
            edges,
            root: root.to_path_buf(),
        })
    }

    fn package_for_file(&self, graph: &ProjectGraph, file: &Path) -> Option<PackageId> {
        file_to_package(graph, file)
    }

    fn test_command(&self, package_id: &PackageId) -> Vec<String> {
        vec![
            "mvn".into(),
            "test".into(),
            "-pl".into(),
            package_id.0.clone(),
        ]
    }
}

/// Parsed information from a pom.xml file.
#[derive(Debug, Default)]
struct PomInfo {
    group_id: Option<String>,
    artifact_id: Option<String>,
    version: Option<String>,
    modules: Vec<String>,
    dependencies: Vec<MavenDep>,
}

/// A single `<dependency>` entry from a pom.xml.
#[derive(Debug, Default)]
struct MavenDep {
    group_id: Option<String>,
    artifact_id: Option<String>,
}

/// Parse a pom.xml string, extracting groupId, artifactId, version, modules, and dependencies.
///
/// Walks XML events manually with quick_xml::Reader.
fn parse_pom(xml: &str) -> Result<PomInfo> {
    let mut reader = Reader::from_str(xml);

    let mut info = PomInfo::default();
    let mut buf = Vec::new();

    // Track nested element context.
    // We care about:
    //   /project/groupId, /project/artifactId, /project/version
    //   /project/modules/module
    //   /project/dependencies/dependency/groupId
    //   /project/dependencies/dependency/artifactId
    // We need to ignore <parent>/<groupId> etc.
    let mut tag_stack: Vec<String> = Vec::new();
    let mut current_dep = MavenDep::default();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                tag_stack.push(tag_name);
            }
            Ok(Event::End(_)) => {
                let ended_tag = tag_stack.pop().unwrap_or_default();

                // If we just closed a <dependency>, save the accumulated dep
                if ended_tag == "dependency" && is_in_path(&tag_stack, &["project", "dependencies"])
                {
                    let dep = std::mem::take(&mut current_dep);
                    if dep.group_id.is_some() || dep.artifact_id.is_some() {
                        info.dependencies.push(dep);
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                let text = e.unescape().unwrap_or_default().trim().to_string();
                if text.is_empty() {
                    buf.clear();
                    continue;
                }

                let depth = tag_stack.len();
                if depth == 0 {
                    buf.clear();
                    continue;
                }

                let current_tag = &tag_stack[depth - 1];

                // Top-level project fields (depth 2: project > fieldName)
                if depth == 2 && tag_stack[0] == "project" {
                    match current_tag.as_str() {
                        "groupId" => info.group_id = Some(text),
                        "artifactId" => info.artifact_id = Some(text),
                        "version" => info.version = Some(text),
                        _ => {}
                    }
                }
                // Module entries: project > modules > module
                else if depth == 3
                    && is_in_path(&tag_stack[..depth - 1], &["project", "modules"])
                    && current_tag == "module"
                {
                    info.modules.push(text);
                }
                // Dependency fields: project > dependencies > dependency > (groupId|artifactId)
                else if depth == 4
                    && is_in_path(
                        &tag_stack[..depth - 1],
                        &["project", "dependencies", "dependency"],
                    )
                {
                    match current_tag.as_str() {
                        "groupId" => current_dep.group_id = Some(text),
                        "artifactId" => current_dep.artifact_id = Some(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => anyhow::bail!("Error parsing pom.xml: {}", e),
            _ => {}
        }
        buf.clear();
    }

    Ok(info)
}

/// Check if the tag stack ends with the given path segments.
fn is_in_path(stack: &[String], path: &[&str]) -> bool {
    if stack.len() < path.len() {
        return false;
    }
    // Check from the beginning of the stack
    path.iter()
        .enumerate()
        .all(|(i, &expected)| stack[i] == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_maven_with_modules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
</project>"#,
        )
        .unwrap();

        assert!(MavenResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_maven_no_modules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>single</artifactId>
</project>"#,
        )
        .unwrap();

        assert!(!MavenResolver.detect(dir.path()));
    }

    #[test]
    fn test_detect_no_pom() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!MavenResolver.detect(dir.path()));
    }

    #[test]
    fn test_parse_pom_root() {
        let xml = r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <version>1.0.0</version>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
</project>"#;

        let info = parse_pom(xml).unwrap();
        assert_eq!(info.group_id.as_deref(), Some("com.example"));
        assert_eq!(info.artifact_id.as_deref(), Some("parent"));
        assert_eq!(info.version.as_deref(), Some("1.0.0"));
        assert_eq!(info.modules, vec!["core", "web"]);
        assert!(info.dependencies.is_empty());
    }

    #[test]
    fn test_parse_pom_with_dependencies() {
        let xml = r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>web</artifactId>
    <dependencies>
        <dependency>
            <groupId>com.example</groupId>
            <artifactId>core</artifactId>
            <version>1.0.0</version>
        </dependency>
        <dependency>
            <groupId>org.external</groupId>
            <artifactId>lib</artifactId>
        </dependency>
    </dependencies>
</project>"#;

        let info = parse_pom(xml).unwrap();
        assert_eq!(info.dependencies.len(), 2);
        assert_eq!(
            info.dependencies[0].group_id.as_deref(),
            Some("com.example")
        );
        assert_eq!(
            info.dependencies[0].artifact_id.as_deref(),
            Some("core")
        );
        assert_eq!(
            info.dependencies[1].group_id.as_deref(),
            Some("org.external")
        );
        assert_eq!(
            info.dependencies[1].artifact_id.as_deref(),
            Some("lib")
        );
    }

    #[test]
    fn test_parse_pom_ignores_parent_group_id() {
        let xml = r#"<?xml version="1.0"?>
<project>
    <parent>
        <groupId>com.parent</groupId>
        <artifactId>parent-pom</artifactId>
    </parent>
    <groupId>com.example</groupId>
    <artifactId>mymodule</artifactId>
</project>"#;

        let info = parse_pom(xml).unwrap();
        assert_eq!(info.group_id.as_deref(), Some("com.example"));
        assert_eq!(info.artifact_id.as_deref(), Some("mymodule"));
    }

    #[test]
    fn test_resolve_maven_project() {
        let dir = tempfile::tempdir().unwrap();

        // Root pom.xml
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <version>1.0.0</version>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
</project>"#,
        )
        .unwrap();

        // Core module
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(
            dir.path().join("core/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>core</artifactId>
    <version>1.0.0</version>
</project>"#,
        )
        .unwrap();

        // Web module depends on core
        std::fs::create_dir_all(dir.path().join("web")).unwrap();
        std::fs::write(
            dir.path().join("web/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>web</artifactId>
    <version>1.0.0</version>
    <dependencies>
        <dependency>
            <groupId>com.example</groupId>
            <artifactId>core</artifactId>
            <version>1.0.0</version>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        let graph = MavenResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.packages.contains_key(&PackageId("core".into())));
        assert!(graph.packages.contains_key(&PackageId("web".into())));

        // web depends on core
        assert!(graph.edges.contains(&(
            PackageId("web".into()),
            PackageId("core".into()),
        )));
    }

    #[test]
    fn test_resolve_maven_no_internal_deps() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <modules>
        <module>alpha</module>
        <module>beta</module>
    </modules>
</project>"#,
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("alpha")).unwrap();
        std::fs::write(
            dir.path().join("alpha/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>alpha</artifactId>
</project>"#,
        )
        .unwrap();

        std::fs::create_dir_all(dir.path().join("beta")).unwrap();
        std::fs::write(
            dir.path().join("beta/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>beta</artifactId>
    <dependencies>
        <dependency>
            <groupId>org.external</groupId>
            <artifactId>something</artifactId>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        let graph = MavenResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_resolve_maven_inherits_group_id() {
        let dir = tempfile::tempdir().unwrap();

        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>parent</artifactId>
    <modules>
        <module>core</module>
        <module>api</module>
    </modules>
</project>"#,
        )
        .unwrap();

        // Core has its own groupId
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(
            dir.path().join("core/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <groupId>com.example</groupId>
    <artifactId>core</artifactId>
</project>"#,
        )
        .unwrap();

        // Api inherits groupId from root (no groupId specified)
        std::fs::create_dir_all(dir.path().join("api")).unwrap();
        std::fs::write(
            dir.path().join("api/pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <artifactId>api</artifactId>
    <dependencies>
        <dependency>
            <groupId>com.example</groupId>
            <artifactId>core</artifactId>
        </dependency>
    </dependencies>
</project>"#,
        )
        .unwrap();

        let graph = MavenResolver.resolve(dir.path()).unwrap();
        assert_eq!(graph.packages.len(), 2);
        // api depends on core
        assert!(graph.edges.contains(&(
            PackageId("api".into()),
            PackageId("core".into()),
        )));
    }

    #[test]
    fn test_test_command() {
        let cmd = MavenResolver.test_command(&PackageId("core".into()));
        assert_eq!(cmd, vec!["mvn", "test", "-pl", "core"]);
    }
}
