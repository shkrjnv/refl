//! The `requires` graph: acyclicity check and transitive closure.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use refl_core::content::EdgeKind;
use refl_core::ids::KcId;

use crate::check::Ids;
use crate::load::Content;
use crate::report::{Code, Diagnostic, Severity};

/// `requires` adjacency between known KCs, with the file of each edge.
#[derive(Debug)]
pub struct Requires<'a> {
    next: BTreeMap<&'a KcId, Vec<(&'a KcId, &'a PathBuf)>>,
}

impl<'a> Requires<'a> {
    pub fn new(c: &'a Content, ids: &Ids) -> Self {
        let mut next: BTreeMap<&KcId, Vec<(&KcId, &PathBuf)>> = BTreeMap::new();
        for e in &c.edges {
            let edge = &e.value;
            if edge.kind == EdgeKind::Requires
                && ids.kcs.contains(&edge.from)
                && ids.kcs.contains(&edge.to)
            {
                next.entry(&edge.from)
                    .or_default()
                    .push((&edge.to, &e.file));
            }
        }
        for targets in next.values_mut() {
            targets.sort();
        }
        Self { next }
    }

    /// Reports every cycle found by a depth-first search, once per back edge.
    pub fn check_acyclic(&self, diags: &mut Vec<Diagnostic>) {
        #[derive(Clone, Copy, PartialEq)]
        enum Color {
            Grey,
            Black,
        }
        fn visit<'a>(
            g: &Requires<'a>,
            kc: &'a KcId,
            color: &mut BTreeMap<&'a KcId, Color>,
            stack: &mut Vec<&'a KcId>,
            diags: &mut Vec<Diagnostic>,
        ) {
            color.insert(kc, Color::Grey);
            stack.push(kc);
            for &(to, file) in g.next.get(kc).into_iter().flatten() {
                match color.get(to) {
                    Some(Color::Grey) => {
                        let start = stack.iter().position(|k| *k == to).unwrap_or(0);
                        let mut cycle: Vec<String> =
                            stack[start..].iter().map(|k| format!("`{k}`")).collect();
                        cycle.push(format!("`{to}`"));
                        diags.push(Diagnostic {
                            severity: Severity::Error,
                            code: Code::RequiresCycle,
                            file: file.clone(),
                            subject: Some(format!("edge `{kc}` requires `{to}`")),
                            message: format!(
                                "`requires` graph has a cycle: {}",
                                cycle.join(" -> ")
                            ),
                        });
                    }
                    Some(Color::Black) => {}
                    None => visit(g, to, color, stack, diags),
                }
            }
            stack.pop();
            color.insert(kc, Color::Black);
        }
        let mut color = BTreeMap::new();
        for &kc in self.next.keys() {
            if !color.contains_key(kc) {
                visit(self, kc, &mut color, &mut Vec::new(), diags);
            }
        }
    }

    /// Every KC in `ids` mapped to all KCs it requires, directly or
    /// transitively. Call only on an acyclic graph.
    pub fn closure(&self, ids: &Ids) -> BTreeMap<KcId, Vec<KcId>> {
        let mut out = BTreeMap::new();
        for kc in &ids.kcs {
            let mut seen: BTreeSet<&KcId> = BTreeSet::new();
            let mut stack = vec![kc];
            while let Some(k) = stack.pop() {
                for &(to, _) in self.next.get(k).into_iter().flatten() {
                    if seen.insert(to) {
                        stack.push(to);
                    }
                }
            }
            out.insert(kc.clone(), seen.into_iter().cloned().collect());
        }
        out
    }
}
