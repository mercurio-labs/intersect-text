//! One graph-only, paid epsilon/action DAG kernel for explicit compiler and
//! runtime consumers. This is a topology summary, never a predicate, input,
//! recursion, model, completed-token or caller-FOLLOW result.
//!
//! Eager sorted ports deliberately select a different finite scheduling order.
//! Every effect port returns to the SAME canonical consumer with its original
//! alternative and complete ordered return stack. The default workers do not
//! construct this sidecar and retain their literal bodies and fees.
use super::adjacent::{AnalysisPop, ClosureQueue, GeometricAdjacentQueue};
use super::analysis_calls::{Calls, SharedCalls, VecCalls};
use super::frontier::ClosureKeys;
use super::{
    AdjacentWorkObservation, AnalysisConfiguration, Edge, RecursiveAnalysisCounters, ScheduleSite,
    State, analysis_charge,
};
use std::collections::{BTreeMap, BTreeSet};

// Fixed portable logical units, not allocator/RSS or whole-pipeline bounds.
// Maps use a prepaid portable logical lookup/placement policy based on their
// bounded entry count. This is NOT a Rust BTreeMap comparison/CPU upper bound.
// No target-sized record or pointer determines these fees.
const HEADER_BYTES: usize = 128;
const COLOR_BYTES: usize = 4;
const COLOR_BITS: u32 = 4;
const MAX_EPOCH: u32 = u32::MAX >> COLOR_BITS;
const FRAME_BYTES: usize = 32;
const PORT_BYTES: usize = 32;
const VECTOR_BYTES: usize = 32;
const SUMMARY_BYTES: usize = 64;
const CONFIGURATION_BYTES: usize = 48;
const RETURN_BYTES: usize = 16;
const MAX_KEYS: usize = 4096;
const MAX_LOGICAL_BYTES: usize = 1_048_576;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Port {
    pub(super) state: usize,
    pub(super) action_seen: bool,
}

/// Cumulative kernel operations/admission only. These are not source-cache
/// prediction hits/publications, allocator measurements or saved work claims.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SummaryStats {
    pub(super) lookups: usize,
    pub(super) builds: usize,
    pub(super) publications: usize,
    pub(super) hits: usize,
    pub(super) fallbacks: usize,
    pub(super) logical_bytes: usize,
    #[cfg(test)]
    pub(super) port_copies: usize,
}

#[cfg(test)]
#[derive(Clone, Copy)]
struct Frame {
    state: usize,
    action_seen: bool,
    exit: bool,
}

// A cursor retains the actual unprocessed suffix of this immutable graph's
// edge slice, never another source/program's rows or an effect interpretation.
#[derive(Clone, Copy)]
struct CursorFrame<'graph> {
    state: usize,
    action_seen: bool,
    remaining: &'graph [Edge],
}
// Fixed logical policy remains32 on both supported targets; target layout
// cannot silently enlarge the record beyond its separately prepaid bound.
const _: () = assert!(std::mem::size_of::<CursorFrame<'static>>() <= FRAME_BYTES);

struct CursorWalk<'graph> {
    start: usize,
    stack: Vec<CursorFrame<'graph>>,
    visited_keys: usize,
    ports: BTreeSet<Port>,
    reuse_enabled: bool,
}

// Only the explicit new queue stores completed-component node bounds.
// The historical alias remains the exact Vec<Port> entry representation.
pub(super) trait ComponentPolicy {
    type Entry: std::ops::Deref<Target = [Port]>;
    const REUSE: bool;
    // Explicit new consumer only: lend one immutable map entry across admissions.
    const BORROW_PORTS: bool = false;
    fn retain(ports: Vec<Port>, nodes: usize) -> Self::Entry;
    fn nodes(entry: &Self::Entry) -> Option<usize>;
}
pub(super) struct LiteralComponents;
impl ComponentPolicy for LiteralComponents {
    type Entry = Vec<Port>;
    const REUSE: bool = false;
    fn retain(ports: Vec<Port>, _: usize) -> Self::Entry {
        ports
    }
    fn nodes(_: &Self::Entry) -> Option<usize> {
        None
    }
}
#[cfg(test)]
pub(super) struct CompletedComponents;
pub(super) struct CompletedPorts {
    ports: Vec<Port>,
    nodes: usize,
}
impl std::ops::Deref for CompletedPorts {
    type Target = [Port];
    fn deref(&self) -> &Self::Target {
        &self.ports
    }
}
#[cfg(test)]
impl ComponentPolicy for CompletedComponents {
    type Entry = CompletedPorts;
    const REUSE: bool = true;
    fn retain(ports: Vec<Port>, nodes: usize) -> Self::Entry {
        CompletedPorts { ports, nodes }
    }
    fn nodes(entry: &Self::Entry) -> Option<usize> {
        Some(entry.nodes)
    }
}
// Same completed proof and row representation; only redundant map borrows and
// repeated opaque-boundary membership operations are removed. Historical queues
// remain on their literal policies and fees.
pub(super) struct BorrowedCompletedComponents;
impl ComponentPolicy for BorrowedCompletedComponents {
    type Entry = CompletedPorts;
    const REUSE: bool = true;
    const BORROW_PORTS: bool = true;
    fn retain(ports: Vec<Port>, nodes: usize) -> Self::Entry {
        CompletedPorts { ports, nodes }
    }
    fn nodes(entry: &Self::Entry) -> Option<usize> {
        Some(entry.nodes)
    }
}
// The SAME fixed64 logical publication slot still covers this portable entry.
// This is not an allocator/BTreeMap-node/RSS bound.
const _: () = assert!(std::mem::size_of::<(usize, Option<CompletedPorts>)>() <= SUMMARY_BYTES);
pub(super) type Summaries<'graph> = Kernel<'graph, LiteralComponents>;
#[cfg(test)]
type ComponentSummaries<'graph> = Kernel<'graph, CompletedComponents>;
pub(super) struct Kernel<'graph, P: ComponentPolicy> {
    graph: &'graph [State],
    // A completed None means structural ineligibility/cycle ONLY. Errors are
    // never published, and false recursion facts are never derived from None.
    completed: BTreeMap<usize, Option<P::Entry>>,
    admitted: bool,
    poisoned: bool,
    // One paid dense scratch, reused by generation rather than clearing
    // or rebuilding a map per root. Two 2-bit action colors per state.
    colors: Vec<u32>,
    epoch: u32,
    stats: SummaryStats,
}

fn bump(value: &mut usize) -> Result<(), String> {
    *value = value
        .checked_add(1)
        .ok_or("Transparent port counter overflow")?;
    Ok(())
}

fn map_work<F: FnMut() -> Result<(), String>>(
    entries: usize,
    charge: &mut F,
) -> Result<(), String> {
    // One actual map lookup/placement invokes this chosen logical fee:
    // 1 + bit_length(entries + 1). It is not a measured or conservative bound
    // on std's internal comparisons. Integer division is target-independent.
    let mut levels = entries
        .checked_add(1)
        .ok_or("Transparent port size overflow")?;
    charge()?;
    while levels != 0 {
        charge()?;
        levels /= 2;
    }
    Ok(())
}

impl<'graph, P: ComponentPolicy> Kernel<'graph, P> {
    /// No heap allocation, graph scan or work-pool initialization occurs here.
    pub(super) fn new(graph: &'graph [State]) -> Self {
        Self {
            graph,
            completed: BTreeMap::new(),
            admitted: false,
            poisoned: false,
            colors: Vec::new(),
            epoch: 0,
            stats: SummaryStats::default(),
        }
    }

    pub(super) fn stats(&self) -> SummaryStats {
        self.stats
    }

    pub(super) fn invalidate(&mut self) {
        self.completed.clear();
        self.colors = Vec::new();
        self.poisoned = true;
    }

    fn admit<F: FnMut() -> Result<(), String>>(
        &mut self,
        bytes: usize,
        charge: &mut F,
    ) -> Result<(), String> {
        Self::admit_fields(&mut self.stats, bytes, charge)
    }

    fn admit_fields<F: FnMut() -> Result<(), String>>(
        stats: &mut SummaryStats,
        bytes: usize,
        charge: &mut F,
    ) -> Result<(), String> {
        let total = stats
            .logical_bytes
            .checked_add(bytes)
            .ok_or("Transparent port logical size overflow")?;
        if total > MAX_LOGICAL_BYTES {
            return Err("Transparent port logical retention bound exceeded".into());
        }
        let units = bytes
            .checked_add(63)
            .ok_or("Transparent port logical size overflow")?
            / 64;
        charge()?;
        for _ in 0..units {
            charge()?;
        }
        // Cumulative admission is never refunded when scratch/copies drop.
        stats.logical_bytes = total;
        Ok(())
    }

    #[cfg(test)]
    fn push<F: FnMut() -> Result<(), String>>(
        &mut self,
        stack: &mut Vec<Frame>,
        frame: Frame,
        charge: &mut F,
    ) -> Result<(), String> {
        if stack.len() >= MAX_KEYS * 2 {
            return Err("Transparent port pending bound exceeded".into());
        }
        self.admit(FRAME_BYTES, charge)?;
        charge()?;
        stack
            .try_reserve(1)
            .map_err(|_| "Transparent port stack allocation failed")?;
        stack.push(frame);
        Ok(())
    }

    fn begin_colors<F: FnMut() -> Result<(), String>>(
        &mut self,
        charge: &mut F,
    ) -> Result<(), String> {
        // Checked portable u32 storage, not usize/pointer-sized admission.
        charge()?; // actual generation read/check
        let epoch = self
            .epoch
            .checked_add(1)
            .filter(|value| *value <= MAX_EPOCH)
            .ok_or("Transparent port visitation epoch exhausted")?;
        charge()?; // reserve the generation write before initialization
        if self.colors.is_empty() {
            charge()?; // actual source length and portable u32 slot bound
            let slots = u32::try_from(self.graph.len())
                .map_err(|_| "Transparent port visitation state bound exceeded")?;
            let length = usize::try_from(slots)
                .map_err(|_| "Transparent port visitation state bound exceeded")?;
            let bytes = length
                .checked_mul(COLOR_BYTES)
                .ok_or("Transparent port visitation size overflow")?;
            let retained = VECTOR_BYTES
                .checked_add(bytes)
                .ok_or("Transparent port visitation size overflow")?;
            self.admit(retained, charge)?;
            // A separate paid initialization event covers every zeroed byte
            // BEFORE allocation/resize. Logical 64-byte units, no CPU bound.
            charge()?;
            for _ in 0..bytes.div_ceil(64) {
                charge()?;
            }
            self.colors
                .try_reserve_exact(length)
                .map_err(|_| "Transparent port visitation allocation failed")?;
            self.colors.resize(length, 0);
        }
        self.epoch = epoch;
        Ok(())
    }

    fn color<F: FnMut() -> Result<(), String>>(
        &self,
        state: usize,
        action_seen: bool,
        charge: &mut F,
    ) -> Result<u32, String> {
        Self::color_fields(&self.colors, self.epoch, state, action_seen, charge)
    }

    fn set_color<F: FnMut() -> Result<(), String>>(
        &mut self,
        state: usize,
        action_seen: bool,
        color: u32,
        charge: &mut F,
    ) -> Result<(), String> {
        Self::set_color_fields(
            &mut self.colors,
            self.epoch,
            state,
            action_seen,
            color,
            charge,
        )
    }

    fn color_fields<F: FnMut() -> Result<(), String>>(
        colors: &[u32],
        epoch: u32,
        state: usize,
        action_seen: bool,
        charge: &mut F,
    ) -> Result<u32, String> {
        charge()?; // actual indexed lookup/load
        let word = *colors
            .get(state)
            .ok_or("Dangling parser prediction state")?;
        if word >> COLOR_BITS != epoch {
            return Ok(0);
        }
        Ok((word >> (u32::from(action_seen) * 2)) & 3)
    }

    fn set_color_fields<F: FnMut() -> Result<(), String>>(
        colors: &mut [u32],
        epoch: u32,
        state: usize,
        action_seen: bool,
        color: u32,
        charge: &mut F,
    ) -> Result<(), String> {
        charge()?; // actual indexed lookup/read
        let slot = colors
            .get_mut(state)
            .ok_or("Dangling parser prediction state")?;
        let prior = if *slot >> COLOR_BITS == epoch {
            *slot
        } else {
            epoch << COLOR_BITS
        };
        let shift = u32::from(action_seen) * 2;
        let word = (prior & !(3 << shift)) | (color << shift);
        charge()?; // actual indexed write, never an unpaid color mutation
        *slot = word;
        Ok(())
    }

    fn push_cursor<F: FnMut() -> Result<(), String>>(
        &mut self,
        stack: &mut Vec<CursorFrame<'graph>>,
        frame: CursorFrame<'graph>,
        charge: &mut F,
    ) -> Result<(), String> {
        if stack.len() >= MAX_KEYS * 2 {
            return Err("Transparent port pending bound exceeded".into());
        }
        self.admit(FRAME_BYTES, charge)?;
        charge()?; // actual reserve/placement before retention
        stack
            .try_reserve(1)
            .map_err(|_| "Transparent port stack allocation failed")?;
        stack.push(frame);
        Ok(())
    }

    fn retain_port<F: FnMut() -> Result<(), String>>(
        &mut self,
        port: Port,
        walk: &mut CursorWalk<'graph>,
        charge: &mut F,
    ) -> Result<(), String> {
        Self::retain_port_fields(&mut self.stats, port, walk, false, charge)
    }

    fn retain_port_fields<F: FnMut() -> Result<(), String>>(
        stats: &mut SummaryStats,
        port: Port,
        walk: &mut CursorWalk<'graph>,
        known_new: bool,
        charge: &mut F,
    ) -> Result<(), String> {
        // A white color proves absence only for the explicit borrowed policy:
        // that policy marks each opaque (state,action) immediately on insertion.
        // Historical policies retain their exact contains/placement operations.
        if !known_new {
            map_work(walk.ports.len(), charge)?;
            if walk.ports.contains(&port) {
                return Ok(());
            }
        }
        if walk.ports.len() >= MAX_KEYS {
            return Err("Transparent port boundary bound exceeded".into());
        }
        Self::admit_fields(stats, PORT_BYTES, charge)?;
        map_work(walk.ports.len(), charge)?;
        walk.ports.insert(port);
        Ok(())
    }

    // A completed Some proved every pure path acyclic and every effect port
    // against this SAME immutable graph. A current gray ancestor reachable
    // through it would contradict that complete proof. None is never reused.
    fn reuse_component<F: FnMut() -> Result<(), String>>(
        &mut self,
        state: usize,
        action_seen: bool,
        walk: &mut CursorWalk<'graph>,
        charge: &mut F,
    ) -> Result<bool, String> {
        if P::BORROW_PORTS {
            return self.reuse_component_borrowed(state, action_seen, walk, charge);
        }
        map_work(self.completed.len(), charge)?; // actual entry lookup
        let Some(Some(entry)) = self.completed.get(&state) else {
            return Ok(false);
        };
        charge()?; // actual retained certificate header read before inspection
        let nodes = P::nodes(entry).ok_or("Missing completed component node proof")?;
        if nodes == 0 || nodes > MAX_KEYS {
            return Err("Invalid completed component node proof".into());
        }
        let Some(bound) = walk
            .visited_keys
            .checked_add(nodes)
            .filter(|n| *n <= MAX_KEYS)
        else {
            return Ok(false);
        }; // literal walk retains the real structural guard
        let length = entry.len();
        for index in 0..length {
            map_work(self.completed.len(), charge)?; // fresh paid borrow, no cloned Vec
            charge()?; // actual descriptor lookup/read before transfer
            let entry = self
                .completed
                .get(&state)
                .and_then(Option::as_ref)
                .ok_or("Missing immutable completed component")?;
            let port = *entry.get(index).ok_or("Changed completed component port")?;
            self.retain_port(
                Port {
                    state: port.state,
                    action_seen: action_seen || port.action_seen,
                },
                walk,
                charge,
            )?;
        }
        // The upper bound can overcount joins; never admits over4096. Interior
        // colors are not forged. Later finite structural denial can be earlier.
        charge()?; // actual composed-bound write before committing the proof
        walk.visited_keys = bound;
        self.set_color(state, action_seen, 2, charge)?;
        bump(&mut self.stats.hits)?; // completed lookup hits, including this internal Some
        Ok(true)
    }

    fn reuse_component_borrowed<F: FnMut() -> Result<(), String>>(
        &mut self,
        state: usize,
        action_seen: bool,
        walk: &mut CursorWalk<'graph>,
        charge: &mut F,
    ) -> Result<bool, String> {
        // These borrows remain disjoint throughout the merge. The immutable
        // completed entry is looked up ONCE, not re-found for every descriptor.
        let Self {
            completed,
            stats,
            colors,
            epoch,
            ..
        } = self;
        map_work(completed.len(), charge)?;
        let Some(Some(entry)) = completed.get(&state) else {
            return Ok(false);
        };
        charge()?; // actual retained certificate header read
        let nodes = P::nodes(entry).ok_or("Missing completed component node proof")?;
        if nodes == 0 || nodes > MAX_KEYS {
            return Err("Invalid completed component node proof".into());
        }
        let Some(bound) = walk
            .visited_keys
            .checked_add(nodes)
            .filter(|n| *n <= MAX_KEYS)
        else {
            return Ok(false);
        };
        for port in entry.iter() {
            charge()?; // actual descriptor read/transfer remains paid
            let port = Port {
                state: port.state,
                action_seen: action_seen || port.action_seen,
            };
            match Self::color_fields(colors, *epoch, port.state, port.action_seen, charge)? {
                0 => {
                    Self::retain_port_fields(stats, port, walk, true, charge)?;
                    Self::set_color_fields(
                        colors,
                        *epoch,
                        port.state,
                        port.action_seen,
                        2,
                        charge,
                    )?;
                }
                2 => {} // the exact opaque port was already emitted in this walk
                _ => return Err("Completed component port intersects active pure state".into()),
            }
        }
        charge()?; // same conservative bound write
        walk.visited_keys = bound;
        Self::set_color_fields(colors, *epoch, state, action_seen, 2, charge)?;
        bump(&mut stats.hits)?;
        Ok(true)
    }

    fn enter_cursor<F: FnMut() -> Result<(), String>>(
        &mut self,
        state: usize,
        action_seen: bool,
        walk: &mut CursorWalk<'graph>,
        charge: &mut F,
        topology: Option<&super::PoolWork>,
    ) -> Result<bool, String> {
        // SAME complete color key and gray-cycle/black-join distinction,
        // before a child frame is allocated. All color reads remain paid.
        let color = self.color(state, action_seen, charge)?;
        if color != 0 {
            return Ok(color != 1);
        }
        charge()?; // actual immutable graph lookup
        let graph = self.graph;
        let node = graph.get(state).ok_or("Dangling parser prediction state")?;
        if state != walk.start && !self.is_pure(node, charge)? {
            let port = Port { state, action_seen };
            if P::BORROW_PORTS {
                // This exact key was just observed white. An immutable opaque
                // boundary has no internal edge or gray DFS obligation. Marking
                // it black removes repeated eligibility and set-contains work at
                // joins, while the sorted port still executes once canonically.
                Self::retain_port_fields(&mut self.stats, port, walk, true, charge)?;
                self.set_color(state, action_seen, 2, charge)?;
            } else {
                self.retain_port(port, walk, charge)?;
            }
            return Ok(true); // effect remains opaque: no child/exit frame
        }
        if walk.reuse_enabled
            && state != walk.start
            && self.reuse_component(state, action_seen, walk, charge)?
        {
            return Ok(true);
        }
        if walk.visited_keys >= MAX_KEYS {
            return Err("Transparent port node bound exceeded".into());
        }
        self.set_color(state, action_seen, 1, charge)?;
        walk.visited_keys += 1;
        self.push_cursor(
            &mut walk.stack,
            CursorFrame {
                state,
                action_seen,
                remaining: &node.edges,
            },
            charge,
        )?;
        if let Some(work) = topology {
            work.topology_expansion(state, action_seen);
        }
        Ok(true)
    }

    fn build<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<Option<P::Entry>, String> {
        self.build_with_topology(start, charge, None)
    }

    fn build_with_topology<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
        topology: Option<&super::PoolWork>,
    ) -> Result<Option<P::Entry>, String> {
        // prepare already paid/proved the exact immutable root pure.
        // Dense epoch, all original bounds, source identity and terminal port
        // construction stay shared. Only actual DFS storage is replaced.
        self.begin_colors(charge)?;
        if let Some(work) = topology {
            work.topology_walk();
        }
        // Membership eligibility is immutable during this build. First-cold
        // empty scopes pay one real read, then no per-node empty-map probes.
        let reuse_enabled = if P::REUSE {
            charge()?;
            self.stats.publications != 0
        } else {
            false
        };
        self.admit(VECTOR_BYTES * 2, charge)?;
        let mut walk = CursorWalk {
            start,
            stack: Vec::new(),
            visited_keys: 0,
            ports: BTreeSet::new(),
            reuse_enabled,
        };
        if !self.enter_cursor(start, false, &mut walk, charge, topology)? {
            return Ok(None);
        }
        loop {
            charge()?; // actual top-frame lookup/read, including final empty
            let Some(frame) = walk.stack.last().copied() else {
                break;
            };
            if frame.remaining.is_empty() {
                // Retain the historical internal gray-key consistency guard;
                // no other frame can complete this same exact color key.
                if self.color(frame.state, frame.action_seen, charge)? != 1 {
                    return Err("Missing transparent port DFS key".into());
                }
                self.set_color(frame.state, frame.action_seen, 2, charge)?;
                charge()?; // actual finished-frame pop
                walk.stack.pop();
                if frame.state != walk.start
                    && let Some(work) = topology
                {
                    work.topology_completed_interior(frame.state, frame.action_seen);
                }
                continue;
            }
            charge()?; // actual next source edge/suffix read BEFORE inspection
            let (edge, tail) = frame
                .remaining
                .split_first()
                .ok_or("Changed immutable transparent port cursor")?;
            let (state, action_seen) = match edge {
                Edge::Epsilon(state) => (*state, frame.action_seen),
                Edge::Action(state) => (*state, true),
                _ => return Err("Changed immutable transparent port state".into()),
            };
            if let Some(work) = topology {
                work.topology_edge(frame.state, frame.action_seen);
            }
            charge()?; // actual cursor lookup/write before child traversal
            walk.stack
                .last_mut()
                .ok_or("Missing transparent port DFS cursor")?
                .remaining = tail;
            // Advance the parent, then completely traverse this source-order
            // child. No eager reversed children or separate exit frames exist.
            if !self.enter_cursor(state, action_seen, &mut walk, charge, topology)? {
                return Ok(None);
            }
        }
        if walk.ports.is_empty() {
            return Err("Empty completed transparent port summary".into());
        }
        self.admit(VECTOR_BYTES, charge)?;
        for _ in 0..walk.ports.len() {
            self.admit(PORT_BYTES, charge)?;
            charge()?;
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(walk.ports.len())
            .map_err(|_| "Transparent port vector allocation failed")?;
        for port in walk.ports {
            result.push(port);
        }
        let nodes = if P::REUSE {
            charge()?;
            walk.visited_keys
        } else {
            0
        };
        Ok(Some(P::retain(result, nodes)))
    }

    // Exact C17 source-only component witness. Test-only historical DFS,
    // not a production fallback or second recognition/semantic evaluator.
    #[cfg(test)]
    fn build_before_cursor<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<Option<Vec<Port>>, String> {
        // prepare has already paid/proved this exact immutable root pure.
        let graph = self.graph;
        self.begin_colors(charge)?;
        self.admit(VECTOR_BYTES * 2, charge)?; // stack/port scratch headers
        let mut stack = Vec::new();
        let mut visited_keys = 0usize;
        let mut ports = BTreeSet::new();
        self.push(
            &mut stack,
            Frame {
                state: start,
                action_seen: false,
                exit: false,
            },
            charge,
        )?;
        while let Some(frame) = stack.pop() {
            charge()?;
            let color = self.color(frame.state, frame.action_seen, charge)?;
            if frame.exit {
                if color != 1 {
                    return Err("Missing transparent port DFS key".into());
                }
                self.set_color(frame.state, frame.action_seen, 2, charge)?;
                continue;
            }
            if color != 0 {
                if color == 1 {
                    // Same gray/black DFS and both action variants. Epochs
                    // make prior-root colors white without a scan or clear.
                    return Ok(None);
                }
                continue;
            }
            charge()?;
            let node = graph
                .get(frame.state)
                .ok_or("Dangling parser prediction state")?;
            if frame.state != start && !self.is_pure(node, charge)? {
                let port = Port {
                    state: frame.state,
                    action_seen: frame.action_seen,
                };
                map_work(ports.len(), charge)?;
                if !ports.contains(&port) {
                    if ports.len() >= MAX_KEYS {
                        return Err("Transparent port boundary bound exceeded".into());
                    }
                    self.admit(PORT_BYTES, charge)?;
                    map_work(ports.len(), charge)?;
                    ports.insert(port);
                }
                continue;
            }
            if visited_keys >= MAX_KEYS {
                return Err("Transparent port node bound exceeded".into());
            }
            self.set_color(frame.state, frame.action_seen, 1, charge)?;
            visited_keys += 1;
            self.push(
                &mut stack,
                Frame {
                    exit: true,
                    ..frame
                },
                charge,
            )?;
            // Push reverse source order; iterative DFS is bounded by paid
            // frames and avoids relying on native call-stack depth.
            for edge in node.edges.iter().rev() {
                charge()?;
                let (state, action_seen) = match edge {
                    Edge::Epsilon(state) => (*state, frame.action_seen),
                    Edge::Action(state) => (*state, true),
                    _ => return Err("Changed immutable transparent port state".into()),
                };
                self.push(
                    &mut stack,
                    Frame {
                        state,
                        action_seen,
                        exit: false,
                    },
                    charge,
                )?;
            }
        }
        if ports.is_empty() {
            return Err("Empty completed transparent port summary".into());
        }
        self.admit(VECTOR_BYTES, charge)?;
        for _ in 0..ports.len() {
            self.admit(PORT_BYTES, charge)?;
            charge()?;
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(ports.len())
            .map_err(|_| "Transparent port vector allocation failed")?;
        for port in ports {
            result.push(port);
        }
        Ok(Some(result))
    }

    #[cfg(test)]
    fn prepare_before_cursor<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<bool, String> {
        if self.poisoned {
            return Err("Transparent port source is poisoned".into());
        }
        bump(&mut self.stats.lookups)?;
        // Pay the real bounded eligibility scan BEFORE any map lookup/node
        // placement. Trivial effect roots retain nothing; cycle proofs remain
        // cached None only after the SAME complete paid DFS.
        charge()?;
        let root = self
            .graph
            .get(start)
            .ok_or("Dangling parser prediction state")?;
        if !self.is_pure(root, charge)? {
            bump(&mut self.stats.fallbacks)?;
            return Ok(false);
        }
        if !self.admitted {
            self.admit(HEADER_BYTES, charge)?;
            self.admitted = true;
        }
        map_work(self.completed.len(), charge)?;
        if self.completed.contains_key(&start) {
            bump(&mut self.stats.hits)?;
            return Ok(true);
        }
        bump(&mut self.stats.builds)?;
        let value = self
            .build_before_cursor(start, charge)?
            .map(|ports| P::retain(ports, MAX_KEYS));
        if value.is_none() {
            bump(&mut self.stats.fallbacks)?;
        }
        if self.completed.len() >= MAX_KEYS {
            return Err("Transparent port summary bound exceeded".into());
        }
        self.admit(SUMMARY_BYTES, charge)?;
        map_work(self.completed.len(), charge)?;
        charge()?; // completed publication, strictly after the entire proof
        if value.is_some() {
            bump(&mut self.stats.publications)?;
        }
        self.completed.insert(start, value);
        Ok(true)
    }

    #[cfg(test)]
    fn borrow_ports_before_cursor<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        returns: usize,
        charge: &mut F,
    ) -> Result<Option<&[Port]>, String> {
        let result: Result<bool, String> = (|| {
            if !self.prepare_before_cursor(start, charge)? {
                return Ok(false);
            }
            map_work(self.completed.len(), charge)?;
            let length = match self.completed.get(&start).and_then(Option::as_ref) {
                Some(ports) => ports.len(),
                None => return Ok(false),
            };
            for _ in 0..length {
                self.admit_configuration(returns, charge)?;
                self.admit_configuration(returns, charge)?;
            }
            map_work(self.completed.len(), charge)?;
            Ok(true)
        })();
        match result {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(false) => Ok(None),
            Ok(true) => Ok(self.completed.get(&start).and_then(Option::as_deref)),
        }
    }

    fn is_pure<F: FnMut() -> Result<(), String>>(
        &self,
        node: &State,
        charge: &mut F,
    ) -> Result<bool, String> {
        if node.rule_stop || node.accepting || !node.follow.is_empty() || node.edges.is_empty() {
            return Ok(false);
        }
        for edge in &node.edges {
            charge()?;
            if !matches!(edge, Edge::Epsilon(_) | Edge::Action(_)) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    #[cfg(test)]
    pub(super) fn ports<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<Option<&[Port]>, String> {
        let value: Result<bool, String> = (|| {
            if !self.prepare(start, charge)? {
                return Ok(false);
            }
            map_work(self.completed.len(), charge)?;
            Ok(true)
        })();
        match value {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(false) => Ok(None),
            Ok(true) => Ok(self.completed.get(&start).and_then(Option::as_deref)),
        }
    }

    fn prepare<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<bool, String> {
        self.prepare_with_length(start, charge)
            .map(|(prepared, _)| prepared)
    }

    fn prepare_with_length<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<(bool, Option<usize>), String> {
        self.prepare_with_length_and_topology(start, charge, None)
    }

    #[cfg(test)]
    pub(super) fn copy_ports<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<Option<Vec<Port>>, String> {
        let result = self.copy_ports_impl(start, charge);
        if result.is_err() {
            self.invalidate();
        }
        result
    }

    #[cfg(test)]
    fn copy_ports_impl<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        charge: &mut F,
    ) -> Result<Option<Vec<Port>>, String> {
        let length = match self.ports(start, charge)? {
            Some(ports) => ports.len(),
            None => return Ok(None),
        };
        self.admit(VECTOR_BYTES, charge)?;
        for _ in 0..length {
            self.admit(PORT_BYTES, charge)?;
            charge()?;
        }
        let mut result = Vec::new();
        result
            .try_reserve_exact(length)
            .map_err(|_| "Transparent port copy allocation failed")?;
        map_work(self.completed.len(), charge)?;
        let ports = self
            .completed
            .get(&start)
            .and_then(Option::as_deref)
            .ok_or("Missing completed transparent port summary")?;
        for port in ports.iter() {
            result.push(*port);
        }
        self.stats.port_copies = self
            .stats
            .port_copies
            .checked_add(length)
            .ok_or("Transparent port counter overflow")?;
        Ok(Some(result))
    }

    /// Complete the SAME root proof and prepay both full consumer row copies
    /// before lending descriptors. No temporary owned Vec/Port copies occur.
    pub(super) fn borrow_ports_for_configurations<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        returns: usize,
        charge: &mut F,
    ) -> Result<Option<&[Port]>, String> {
        if P::BORROW_PORTS {
            return self.borrow_ports_with_prepared_length(start, returns, charge);
        }
        let result: Result<bool, String> = (|| {
            if !self.prepare(start, charge)? {
                return Ok(false);
            }
            map_work(self.completed.len(), charge)?;
            let length = match self.completed.get(&start).and_then(Option::as_ref) {
                Some(ports) => ports.len(),
                None => return Ok(false),
            };
            for _ in 0..length {
                self.admit_configuration(returns, charge)?;
                self.admit_configuration(returns, charge)?;
            }
            map_work(self.completed.len(), charge)?;
            Ok(true)
        })();
        match result {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(false) => Ok(None),
            Ok(true) => Ok(self.completed.get(&start).and_then(Option::as_deref)),
        }
    }

    fn borrow_ports_with_prepared_length<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        returns: usize,
        charge: &mut F,
    ) -> Result<Option<&[Port]>, String> {
        self.borrow_ports_with_prepared_length_and_topology(start, returns, charge, None)
    }

    pub(super) fn borrow_ports_for_configurations_with_topology<
        F: FnMut() -> Result<(), String>,
    >(
        &mut self,
        start: usize,
        returns: usize,
        charge: &mut F,
        topology: &super::PoolWork,
    ) -> Result<Option<&[Port]>, String> {
        if P::BORROW_PORTS && P::REUSE {
            self.borrow_ports_with_prepared_length_and_topology(
                start,
                returns,
                charge,
                Some(topology),
            )
        } else {
            topology.topology_unsupported_policy();
            self.borrow_ports_for_configurations(start, returns, charge)
        }
    }

    fn borrow_ports_with_prepared_length_and_topology<F: FnMut() -> Result<(), String>>(
        &mut self,
        start: usize,
        returns: usize,
        charge: &mut F,
        topology: Option<&super::PoolWork>,
    ) -> Result<Option<&[Port]>, String> {
        let result: Result<bool, String> = (|| {
            let (prepared, length) =
                self.prepare_with_length_and_topology(start, charge, topology)?;
            let Some(length) = length.filter(|_| prepared) else {
                return Ok(false);
            };
            for _ in 0..length {
                self.admit_configuration(returns, charge)?;
                self.admit_configuration(returns, charge)?;
            }
            // All row admissions remain identical and complete before lending.
            // Only one final real map re-borrow remains after the mutable fees.
            map_work(self.completed.len(), charge)?;
            Ok(true)
        })();
        match result {
            Err(error) => {
                self.invalidate();
                Err(error)
            }
            Ok(false) => Ok(None),
            Ok(true) => Ok(self.completed.get(&start).and_then(Option::as_deref)),
        }
    }

    /// Admission before each additional owned consumer row/return-stack copy.
    /// A consumer retaining two copies pays twice; this does not replace any
    /// existing canonical schedule/reservation/transition debit.
    pub(super) fn admit_configuration<F: FnMut() -> Result<(), String>>(
        &mut self,
        returns: usize,
        charge: &mut F,
    ) -> Result<(), String> {
        let result =
            Self::admit_configuration_fields(&mut self.stats, self.poisoned, returns, charge);
        if result.is_err() {
            self.invalidate();
        }
        result
    }
}

/// New explicit queue only; SeededQueue and every historical queue/holder stay
/// exact. Real intermediate pure full-key visits are removed, not discounted.
pub(super) type PortQueue<'graph> = Queue<'graph, LiteralComponents>;
#[cfg(test)]
pub(super) type ComponentPortQueue<'graph> = Queue<'graph, CompletedComponents>;
pub(super) type BorrowedComponentPortQueue<'graph> = Queue<'graph, BorrowedCompletedComponents>;
pub(super) type SharedCallPortQueue<'graph> =
    Queue<'graph, BorrowedCompletedComponents, SharedCalls, owned_fifo_ranges::RangeQueue<'graph>>;
const _: () =
    assert!(std::mem::size_of::<AnalysisConfiguration<SharedCalls>>() <= CONFIGURATION_BYTES);
pub(super) struct Queue<
    'graph,
    P: ComponentPolicy,
    C = VecCalls,
    I = GeometricAdjacentQueue<'graph, C>,
> {
    inner: I,
    summaries: Kernel<'graph, P>,
    calls: std::marker::PhantomData<C>,
}

impl<'graph, P: ComponentPolicy, C: Calls, I: ClosureQueue<'graph, C>> ClosureQueue<'graph, C>
    for Queue<'graph, P, C, I>
{
    const POOL_QUEUE_KIND: usize = 0;
    fn pop_for_analysis(
        &mut self,
        remaining: &mut usize,
        pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisPop<C>>, String> {
        self.inner.pop_for_analysis(remaining, pool)
    }
    fn share_opaque_boundary(
        &mut self,
        boundary: opaque_port_replay::Boundary<C>,
        remaining: &mut usize,
        pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        if !I::OWNED_PORT_RANGES {
            return Err("Opaque boundary belongs to the selected port FIFO".into());
        }
        self.summaries
            .consume_boundary(boundary, &mut || {
                let debit = analysis_charge(remaining);
                if let Some(pool) = pool {
                    pool.actual_leaf_result(super::PoolBlock::PortKernel, debit.is_ok());
                }
                debit
            })
            .map(Some)
    }
    fn share_with_pool<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
        pool: &super::PoolWork,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_keys_pool(row, keys, remaining, measured, exact, Some(pool))
    }
    fn empty(graph: &'graph [State]) -> Self {
        Self {
            inner: I::empty(graph),
            summaries: Kernel::<P>::new(graph),
            calls: std::marker::PhantomData,
        }
    }
    fn unreserved(
        _: BTreeSet<AnalysisConfiguration<C>>,
        _: &'graph [State],
    ) -> Result<Self, String> {
        Err("Transparent ports require exact-key reservations".into())
    }
    fn reserve_placement_observed(
        &mut self,
        row: &AnalysisConfiguration<C>,
        remaining: &mut usize,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<(), String> {
        self.inner.reserve_placement_observed(row, remaining, exact)
    }
    fn observed_shape(&self) -> (usize, usize) {
        self.inner.observed_shape()
    }
    fn push_reserved(&mut self, row: AnalysisConfiguration<C>) -> Result<(), String> {
        self.inner.push_reserved(row)
    }
    fn pop_front(&mut self) -> Option<AnalysisConfiguration<C>> {
        self.inner.pop_front()
    }
    fn pop_front_with_work(
        &mut self,
        remaining: &mut usize,
        pool: Option<&super::PoolWork>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.inner.pop_front_with_work(remaining, pool)
    }
    fn share_transparent_observed(
        &mut self,
        row: AnalysisConfiguration<C>,
        keys: &mut BTreeSet<AnalysisConfiguration<C>>,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_keys_observed(row, keys, remaining, measured, exact)
    }
    fn share_keys_observed<K: ClosureKeys<C>>(
        &mut self,
        row: AnalysisConfiguration<C>,
        keys: &mut K,
        remaining: &mut usize,
        measured: Option<&RecursiveAnalysisCounters>,
        exact: Option<&AdjacentWorkObservation>,
    ) -> Result<Option<AnalysisConfiguration<C>>, String> {
        self.share_keys_pool(row, keys, remaining, measured, exact, None)
    }
}

#[cfg(test)]
#[path = "transparent_ports_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "completed_components_tests.rs"]
mod completed_components_tests;

#[cfg(test)]
#[path = "borrowed_completed_ports_tests.rs"]
mod borrowed_completed_ports_tests;

#[cfg(test)]
#[path = "first_queue_topology_tests.rs"]
mod first_queue_topology_tests;

#[path = "single_borrowed_ports.rs"]
mod single_borrowed_ports;

#[cfg(test)]
#[path = "single_borrowed_ports_tests.rs"]
mod single_borrowed_ports_tests;

#[path = "owned_fifo_ranges.rs"]
mod owned_fifo_ranges;
#[cfg(test)]
#[path = "owned_fifo_ranges_tests.rs"]
mod owned_fifo_ranges_tests;

#[path = "opaque_port_replay.rs"]
pub(super) mod opaque_port_replay;
#[cfg(test)]
#[path = "opaque_port_replay_tests.rs"]
mod opaque_port_replay_tests;
