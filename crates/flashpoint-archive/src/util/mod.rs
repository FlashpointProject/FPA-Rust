use std::{fs, path::Path};
use fs_extra::{copy_items, dir::CopyOptions};
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "napi", napi(object))]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentTreeNode {
    pub name: String,
    pub expanded: bool,
    pub size: i64,
    pub node_type: String,
    pub children: Vec<ContentTreeNode>,
    pub count: i64
}

pub fn gen_content_tree(root: &str) -> Result<ContentTreeNode, Box<dyn std::error::Error + Send + Sync>> {
    let children = load_branch(std::path::Path::new(root))?;
    let children_total: i64 = children.iter().map(|n| n.count).sum();
    let count = (children.len() as i64) + children_total;
    let node = ContentTreeNode {
        name: String::from("content"),
        expanded: true,
        node_type: String::from("directory"),
        size: 0,
        children,
        count,
    };
    Ok(node)
}

fn load_branch(root: &std::path::Path) -> Result<Vec<ContentTreeNode>, Box<dyn std::error::Error + Send + Sync>> {
    let mut nodes: Vec<ContentTreeNode> = Vec::new();
    let dir = std::fs::read_dir(root)?;
    for entry in dir {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let children = load_branch(path.as_path())?;
            let children_total: i64 = children.iter().map(|n| n.count).sum();
            let count = (children.len() as i64) + children_total;
            let node = ContentTreeNode {
                name: String::from(path.file_name().unwrap().to_str().unwrap()),
                expanded: true,
                node_type: String::from("directory"),
                children,
                size: 0,
                count: count as i64
            };
            nodes.push(node);
        } else {
            let node = ContentTreeNode {
                name: String::from(path.file_name().unwrap().to_str().unwrap()),
                expanded: true,
                node_type: String::from("file"),
                children: Vec::new(),
                size: path.metadata()?.len() as i64, 
                count: 0
            };
            nodes.push(node);
        }
    }
    Ok(nodes)
}

pub fn copy_folder(src: &str, dest: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let root_path = Path::new(src);
    let dest_path = Path::new(dest);
    fs::create_dir_all(dest_path).unwrap();

    let options = CopyOptions::new(); //Initialize default values for CopyOptions

    // copy dir1 and file1.txt to target/dir1 and target/file1.txt
    let mut from_paths = Vec::new();
    from_paths.push(root_path);
    let copied_items = copy_items(&from_paths, dest_path, &options)?;
    Ok(copied_items)
}