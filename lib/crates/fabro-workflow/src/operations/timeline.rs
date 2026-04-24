use std::collections::HashMap;
use std::fmt::Write;
use std::str::FromStr;

use anyhow::{Context, Result, bail};
use fabro_checkpoint::META_BRANCH_PREFIX;
use fabro_checkpoint::branch::{BranchStore, CommitInfo};
use fabro_checkpoint::git::Store;
use fabro_graphviz::graph::Graph;
use fabro_graphviz::parser;
use fabro_store::{Database, RunProjection};
use fabro_types::RunId;
use git2::{Oid, Repository, Signature};

use super::run_git;
use crate::error::Error;
use crate::git::{MetadataStore, RUN_BRANCH_PREFIX};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForkTarget {
    Ordinal(usize),
    LatestVisit(String),
    SpecificVisit(String, usize),
}

impl FromStr for ForkTarget {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if let Some(rest) = s.strip_prefix('@') {
            let n: usize = rest
                .parse()
                .with_context(|| format!("invalid ordinal: @{rest}"))?;
            if n == 0 {
                bail!("ordinal must be >= 1");
            }
            return Ok(Self::Ordinal(n));
        }
        if let Some(at_pos) = s.rfind('@') {
            let name = &s[..at_pos];
            let visit_str = &s[at_pos + 1..];
            if !name.is_empty() && !visit_str.is_empty() {
                if let Ok(visit) = visit_str.parse::<usize>() {
                    if visit == 0 {
                        bail!("visit number must be >= 1");
                    }
                    return Ok(Self::SpecificVisit(name.to_string(), visit));
                }
            }
        }
        Ok(Self::LatestVisit(s.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub ordinal:             usize,
    pub node_name:           String,
    pub visit:               usize,
    pub metadata_commit_oid: Oid,
    pub run_commit_sha:      Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunTimeline {
    pub entries:      Vec<TimelineEntry>,
    pub parallel_map: HashMap<String, String>,
}

impl RunTimeline {
    pub fn resolve(&self, target: &ForkTarget) -> Result<&TimelineEntry> {
        match target {
            ForkTarget::Ordinal(n) => {
                self.entries
                    .iter()
                    .find(|e| e.ordinal == *n)
                    .ok_or_else(|| {
                        anyhow::anyhow!("ordinal @{n} out of range (max @{})", self.entries.len())
                    })
            }
            ForkTarget::LatestVisit(name) => {
                let effective_name = self.parallel_map.get(name).unwrap_or(name);
                self.entries
                    .iter()
                    .rev()
                    .find(|e| e.node_name == *effective_name)
                    .ok_or_else(|| {
                        if effective_name == name {
                            anyhow::anyhow!("no checkpoint found for node '{name}'")
                        } else {
                            anyhow::anyhow!(
                                "node '{name}' is inside parallel '{effective_name}'; \
                                 no checkpoint found for '{effective_name}'"
                            )
                        }
                    })
            }
            ForkTarget::SpecificVisit(name, visit) => {
                let effective_name = self.parallel_map.get(name).unwrap_or(name);
                self.entries
                    .iter()
                    .find(|e| e.node_name == *effective_name && e.visit == *visit)
                    .ok_or_else(|| {
                        if effective_name == name {
                            anyhow::anyhow!("no visit {visit} found for node '{name}'")
                        } else {
                            anyhow::anyhow!(
                                "node '{name}' is inside parallel '{effective_name}'; \
                                 no visit {visit} found for '{effective_name}'"
                            )
                        }
                    })
            }
        }
    }
}

pub fn build_timeline(store: &Store, run_id: &str) -> Result<RunTimeline> {
    let branch = MetadataStore::branch_name(run_id);
    let sig = Signature::now("Fabro", "noreply@fabro.sh")?;
    let bs = BranchStore::new(store, &branch, &sig);

    let commits = bs
        .log(10_000)
        .map_err(|e| anyhow::anyhow!("failed to read metadata branch log: {e}"))?;
    let commits: Vec<&CommitInfo> = commits.iter().rev().collect();

    let mut timeline = Vec::new();
    let mut ordinal = 0usize;

    for commit in &commits {
        if !commit.message.starts_with("checkpoint") {
            continue;
        }
        let Some(projection) = read_projection_at_commit(store, commit.oid)? else {
            continue;
        };
        let cp = projection.checkpoint.with_context(|| {
            format!(
                "metadata checkpoint {} is missing projection.checkpoint",
                commit.oid
            )
        })?;

        ordinal += 1;
        let visit = cp.node_visits.get(&cp.current_node).copied().unwrap_or(1);

        timeline.push(TimelineEntry {
            ordinal,
            node_name: cp.current_node.clone(),
            visit,
            metadata_commit_oid: commit.oid,
            run_commit_sha: cp.git_commit_sha.clone(),
        });
    }

    backfill_run_shas(store, run_id, &mut timeline);
    Ok(RunTimeline {
        entries:      timeline,
        parallel_map: load_parallel_map(store, run_id),
    })
}

pub async fn timeline(store: &Database, run_id: &RunId) -> Result<Vec<TimelineEntry>, Error> {
    let run_id = *run_id;
    run_git::with_run_git_store(store, run_id, move |git_store| {
        build_timeline(&git_store, &run_id.to_string())
            .map(|timeline| timeline.entries)
            .map_err(|err| Error::engine(err.to_string()))
    })
    .await
}

fn backfill_run_shas(store: &Store, run_id: &str, timeline: &mut [TimelineEntry]) {
    if !timeline.iter().any(|e| e.run_commit_sha.is_none()) {
        return;
    }

    let node_commits = run_commit_shas_by_node(store, run_id);
    let mut node_indices: HashMap<String, usize> = HashMap::new();

    for entry in timeline.iter_mut() {
        if entry.run_commit_sha.is_some() {
            continue;
        }
        if let Some(shas) = node_commits.get(&entry.node_name) {
            let idx = node_indices.entry(entry.node_name.clone()).or_insert(0);
            if *idx < shas.len() {
                entry.run_commit_sha = Some(shas[*idx].clone());
                *idx += 1;
            }
        }
    }
}

pub(crate) fn run_commit_shas_by_node(store: &Store, run_id: &str) -> HashMap<String, Vec<String>> {
    let run_branch = format!("{RUN_BRANCH_PREFIX}{run_id}");
    let Ok(sig) = Signature::now("Fabro", "noreply@fabro.sh") else {
        return HashMap::new();
    };
    let bs = BranchStore::new(store, &run_branch, &sig);
    let Ok(run_commits) = bs.log(10_000) else {
        return HashMap::new();
    };

    let prefix = format!("fabro({run_id}): ");
    let mut node_commits: HashMap<String, Vec<String>> = HashMap::new();
    for commit in &run_commits {
        if let Some(rest) = commit.message.strip_prefix(&prefix) {
            if let Some(node_name) = rest.split_whitespace().next() {
                node_commits
                    .entry(node_name.to_string())
                    .or_default()
                    .push(commit.oid.to_string());
            }
        }
    }

    for shas in node_commits.values_mut() {
        shas.reverse();
    }

    node_commits
}

fn detect_parallel_interior(graph: &Graph) -> HashMap<String, String> {
    let mut interior_map = HashMap::new();

    for node in graph.nodes.values() {
        if node.handler_type() != Some("parallel") {
            continue;
        }
        let parallel_id = &node.id;
        let mut queue: Vec<String> = graph
            .outgoing_edges(parallel_id)
            .iter()
            .map(|e| e.to.clone())
            .collect();
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(n) = graph.nodes.get(&current) {
                if n.handler_type() == Some("parallel.fan_in") {
                    continue;
                }
            }
            interior_map.insert(current.clone(), parallel_id.clone());
            for edge in graph.outgoing_edges(&current) {
                queue.push(edge.to.clone());
            }
        }
    }

    interior_map
}

pub fn find_run_id_by_prefix(repo: &Repository, prefix: &str) -> Result<RunId> {
    find_run_id_by_prefix_opt(repo, prefix)?
        .ok_or_else(|| anyhow::anyhow!("no run found matching '{prefix}'"))
}

/// Resolve a run id from the metadata branch refs. `Ok(None)` when no run
/// matches; `Err` when the prefix matches more than one.
pub(super) fn find_run_id_by_prefix_opt(repo: &Repository, prefix: &str) -> Result<Option<RunId>> {
    let refs = repo.references()?;
    let pattern = format!("refs/heads/{META_BRANCH_PREFIX}");
    let mut matches = Vec::new();

    for reference in refs.flatten() {
        let Some(name) = reference.name() else {
            continue;
        };
        let Some(run_id) = name.strip_prefix(&pattern) else {
            continue;
        };
        let Ok(run_id) = run_id.parse::<RunId>() else {
            continue;
        };
        if run_id.to_string() == prefix {
            return Ok(Some(run_id));
        }
        if run_id.to_string().starts_with(prefix) {
            matches.push(run_id);
        }
    }

    match matches.len() {
        0 => Ok(None),
        1 => Ok(matches.into_iter().next()),
        _ => {
            let mut msg = format!("ambiguous run ID prefix '{prefix}', matches:\n");
            for run_id in &matches {
                let _ = writeln!(msg, "  {run_id}");
            }
            bail!("{msg}")
        }
    }
}

fn load_parallel_map(store: &Store, run_id: &str) -> HashMap<String, String> {
    let Ok(Some(projection)) = MetadataStore::read_run_projection(store.repo_dir(), run_id) else {
        return HashMap::new();
    };

    if let Some(spec) = projection.spec {
        return detect_parallel_interior(&spec.graph);
    }

    let Some(dot_source) = projection.graph_source else {
        return HashMap::new();
    };
    let Ok(graph) = parser::parse(&dot_source) else {
        return HashMap::new();
    };
    detect_parallel_interior(&graph)
}

fn read_projection_at_commit(store: &Store, oid: Oid) -> Result<Option<RunProjection>> {
    let blob = store
        .read_blob_at(oid, "run.json")
        .map_err(|e| anyhow::anyhow!("failed to read projection blob: {e}"))?;
    let Some(bytes) = blob else {
        return Ok(None);
    };
    let projection = serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse projection at {oid}"))?;
    Ok(Some(projection))
}

#[cfg(test)]
mod tests {
    use fabro_store::RunProjection;
    use fabro_types::RunId;
    use git2::Oid;

    use super::super::test_support::*;
    use super::*;

    fn parse_run_id(value: &str) -> RunId {
        value.parse().unwrap()
    }

    fn checkpoint_projection_json(
        current_node: &str,
        visit: usize,
        git_commit_sha: Option<&str>,
    ) -> Vec<u8> {
        let mut projection = RunProjection::default();
        projection.checkpoint = Some(
            serde_json::from_slice(&make_checkpoint_bytes(current_node, visit, git_commit_sha))
                .unwrap(),
        );
        serde_json::to_vec_pretty(&projection).unwrap()
    }

    #[test]
    fn parse_target_ordinal() {
        assert_eq!("@4".parse::<ForkTarget>().unwrap(), ForkTarget::Ordinal(4));
    }

    #[test]
    fn parse_target_latest_visit() {
        assert_eq!(
            "step2".parse::<ForkTarget>().unwrap(),
            ForkTarget::LatestVisit("step2".to_string())
        );
    }

    #[test]
    fn build_timeline_simple() {
        let (_dir, store) = temp_repo();
        let sig = test_sig();
        let branch = MetadataStore::branch_name("test-run-1");
        let bs = BranchStore::new(&store, &branch, &sig);
        bs.ensure_branch().unwrap();

        bs.write_entry("run.json", b"{}", "init run").unwrap();
        let cp1 = checkpoint_projection_json("start", 1, Some("aaa"));
        bs.write_entry("run.json", &cp1, "checkpoint").unwrap();
        let cp2 = checkpoint_projection_json("build", 1, Some("bbb"));
        bs.write_entry("run.json", &cp2, "checkpoint").unwrap();

        let timeline = build_timeline(&store, "test-run-1").unwrap();
        assert_eq!(timeline.entries.len(), 2);
        assert_eq!(timeline.entries[0].node_name, "start");
        assert_eq!(timeline.entries[1].node_name, "build");
    }

    #[test]
    fn resolve_latest_visit() {
        let timeline = RunTimeline {
            entries:      vec![
                TimelineEntry {
                    ordinal:             1,
                    node_name:           "start".to_string(),
                    visit:               1,
                    metadata_commit_oid: Oid::zero(),
                    run_commit_sha:      Some("aaa".to_string()),
                },
                TimelineEntry {
                    ordinal:             2,
                    node_name:           "build".to_string(),
                    visit:               1,
                    metadata_commit_oid: Oid::zero(),
                    run_commit_sha:      Some("bbb".to_string()),
                },
                TimelineEntry {
                    ordinal:             3,
                    node_name:           "build".to_string(),
                    visit:               2,
                    metadata_commit_oid: Oid::zero(),
                    run_commit_sha:      Some("ccc".to_string()),
                },
            ],
            parallel_map: HashMap::new(),
        };

        let entry = timeline
            .resolve(&ForkTarget::LatestVisit("build".to_string()))
            .unwrap();
        assert_eq!(entry.ordinal, 3);
    }

    #[test]
    fn parallel_interior_detection() {
        let mut graph = Graph::new("test");
        let mut parallel_node = fabro_graphviz::graph::Node::new("parallel1");
        parallel_node.attrs.insert(
            "shape".to_string(),
            fabro_graphviz::graph::AttrValue::String("component".to_string()),
        );
        graph.nodes.insert("parallel1".to_string(), parallel_node);

        let mut fan_in = fabro_graphviz::graph::Node::new("fan_in1");
        fan_in.attrs.insert(
            "shape".to_string(),
            fabro_graphviz::graph::AttrValue::String("tripleoctagon".to_string()),
        );
        graph.nodes.insert("fan_in1".to_string(), fan_in);

        let mut a = fabro_graphviz::graph::Node::new("a");
        a.attrs.insert(
            "shape".to_string(),
            fabro_graphviz::graph::AttrValue::String("box".to_string()),
        );
        graph.nodes.insert("a".to_string(), a);

        graph.edges.push(fabro_graphviz::graph::Edge {
            from:  "parallel1".to_string(),
            to:    "a".to_string(),
            attrs: HashMap::new(),
        });
        graph.edges.push(fabro_graphviz::graph::Edge {
            from:  "a".to_string(),
            to:    "fan_in1".to_string(),
            attrs: HashMap::new(),
        });

        let map = detect_parallel_interior(&graph);
        assert_eq!(map.get("a"), Some(&"parallel1".to_string()));
        assert!(!map.contains_key("parallel1"));
    }

    #[test]
    fn find_run_id_prefix_match() {
        let (_dir, store) = temp_repo();
        let sig = test_sig();
        let run_id = parse_run_id("01ARZ3NDEKTSV4RRFFQ69G5FAV");
        let branch = MetadataStore::branch_name(&run_id.to_string());
        let bs = BranchStore::new(&store, &branch, &sig);
        bs.ensure_branch().unwrap();

        let result = find_run_id_by_prefix(store.repo(), "01ARZ3").unwrap();
        assert_eq!(result, run_id);
    }
}
