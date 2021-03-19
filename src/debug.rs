/*
* Copyright 2021 TON DEV SOLUTIONS LTD.
*
* Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
* this file except in compliance with the License.
*
* Unless required by applicable law or agreed to in writing, software
* distributed under the License is distributed on an "AS IS" BASIS,
* WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
* See the License for the specific TON DEV software governing permissions and
* limitations under the License.
*/

use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use ton_types::Cell;

pub type Lines = Vec<Line>;
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub pos: DbgPos
}

impl Line {
    pub fn new(text: &str, filename: &str, line: usize) -> Self {
        Line {
            text: String::from(text),
            pos: DbgPos { filename: String::from(filename), line: line }
        }
    }
}

pub fn lines_to_string(lines: &Lines) -> String {
    let mut res = "".to_string();
    for line in lines {
        res.push_str(line.text.as_str());
    }
    res
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DbgPos {
    pub filename: String,
    pub line: usize,
}

impl std::fmt::Display for DbgPos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let filename = if self.filename.is_empty() {
            "<none>"
        } else {
            self.filename.as_str()
        };
        write!(f, "{}:{}", filename, self.line)
    }
}

impl Default for DbgPos {
    fn default() -> Self {
        Self { filename: String::from(""), line: 0 }
    }
}
#[derive(Clone)]
pub struct DbgNode {
    pub offsets: BTreeMap<usize, DbgPos>,
    pub children: Vec<DbgNode>,
}

impl DbgNode {
    pub fn new() -> Self {
        Self {
            offsets: BTreeMap::new(),
            children: vec![],
        }
    }
    pub fn from(pos: DbgPos) -> Self {
        let mut node = Self::new();
        node.offsets.insert(0, pos);
        node
    }
    pub fn append(self: &mut Self, offset: usize, pos: DbgPos) {
        self.offsets.insert(offset, pos);
    }
    pub fn inline_node(self: &mut Self, offset: usize, dbg: DbgNode) {
        assert!(dbg.children.is_empty());
        for entry in dbg.offsets {
            self.offsets.insert(entry.0 + offset, entry.1);
        }
    }
    pub fn append_node(self: &mut Self, dbg: DbgNode) {
        assert!(self.children.len() <= 4);
        self.children.push(dbg)
    }
}

impl std::fmt::Display for DbgNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for entry in self.offsets.iter() {
            write!(f, "{}:{}\n", entry.0, entry.1)?
        }
        write!(f, "{} children", self.children.len())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DbgInfo {
    pub map: BTreeMap<String, BTreeMap<usize, DbgPos>>
}

impl DbgInfo {
    pub fn new() -> Self {
        DbgInfo { map: BTreeMap::new() }
    }
    pub fn from(cell: &Cell, node: &DbgNode) -> Self {
        let mut info = DbgInfo { map: BTreeMap::new() };
        info.collect(&cell, &node);
        info
    }
    fn collect(self: &mut Self, cell: &Cell, dbg: &DbgNode) {
        let hash = cell.repr_hash().to_hex_string();
        let res = self.map.insert(hash, dbg.offsets.clone());
        assert!(res.is_none());
        for i in 0..cell.references_count() {
            let child_cell = cell.reference(i).unwrap();
            let child_dbg = dbg.children[i].clone();
            self.collect(&child_cell, &child_dbg);
        }
    }
}
