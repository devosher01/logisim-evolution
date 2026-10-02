//! Connectivity compiler for validated circuit documents.
//!
//! The compiler produces deterministic nets and keeps graph algorithms out of
//! the document model and the future simulator.

use domain_model::{Circuit, EntityId, ModelError, PortRef};
use std::collections::{BTreeMap, HashMap};
use std::fmt;

/// A compiled electrical net.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Net {
    id: usize,
    ports: Vec<PortRef>,
}

impl Net {
    /// Returns the deterministic net index.
    #[must_use]
    pub const fn id(&self) -> usize {
        self.id
    }

    /// Returns the port references belonging to this net.
    #[must_use]
    pub fn ports(&self) -> &[PortRef] {
        &self.ports
    }
}

/// Immutable result of compiling circuit connectivity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Netlist {
    nets: Vec<Net>,
}

impl Netlist {
    /// Returns compiled nets in deterministic order.
    #[must_use]
    pub fn nets(&self) -> &[Net] {
        &self.nets
    }

    /// Finds the net containing a port.
    #[must_use]
    pub fn net_for(&self, port: PortRef) -> Option<&Net> {
        self.nets
            .iter()
            .find(|net| net.ports.iter().any(|candidate| *candidate == port))
    }
}

/// Compiles all component ports into connected nets.
///
/// Every port receives a net, including unconnected ports. Connections are
/// merged with a union-find structure and emitted in stable identity order.
///
/// # Errors
///
/// Returns [`CompileError::Model`] if the input circuit contains invalid
/// connectivity. A valid [`Circuit`] should already have rejected such data.
pub fn compile(circuit: &Circuit) -> Result<Netlist, CompileError> {
    let mut port_indexes = HashMap::new();
    let mut ports = Vec::new();
    for component in circuit.components() {
        for (index, _) in component.ports().iter().enumerate() {
            let index = u16::try_from(index).map_err(|_| CompileError::PortIndexOverflow {
                component: component.id(),
                index,
            })?;
            let port = PortRef::new(component.id(), index);
            port_indexes.insert(port, ports.len());
            ports.push(port);
        }
    }

    let mut union_find = UnionFind::new(ports.len());
    for connection in circuit.connections() {
        let from = *port_indexes
            .get(&connection.from())
            .ok_or_else(|| CompileError::UnknownPort(connection.from()))?;
        let to = *port_indexes
            .get(&connection.to())
            .ok_or_else(|| CompileError::UnknownPort(connection.to()))?;
        union_find.union(from, to);
    }

    let mut grouped = BTreeMap::<usize, Vec<PortRef>>::new();
    for (index, port) in ports.into_iter().enumerate() {
        grouped
            .entry(union_find.find(index))
            .or_default()
            .push(port);
    }

    let nets = grouped
        .into_values()
        .enumerate()
        .map(|(id, ports)| Net { id, ports })
        .collect();
    Ok(Netlist { nets })
}

#[derive(Debug)]
struct UnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
            rank: vec![0; size],
        }
    }

    fn find(&mut self, node: usize) -> usize {
        if self.parent[node] != node {
            let root = self.find(self.parent[node]);
            self.parent[node] = root;
        }
        self.parent[node]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }
        match self.rank[left_root].cmp(&self.rank[right_root]) {
            std::cmp::Ordering::Less => self.parent[left_root] = right_root,
            std::cmp::Ordering::Greater => self.parent[right_root] = left_root,
            std::cmp::Ordering::Equal => {
                self.parent[right_root] = left_root;
                self.rank[left_root] += 1;
            }
        }
    }
}

/// Failure while compiling connectivity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompileError {
    /// Invalid domain data was encountered.
    Model(ModelError),
    /// A connection referred to a port absent from the component index.
    UnknownPort(PortRef),
    /// A component declared more ports than the persisted port index supports.
    PortIndexOverflow {
        /// Component whose ports overflowed.
        component: EntityId,
        /// Zero-based index that could not be represented.
        index: usize,
    },
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(formatter, "netlist compilation failed: {error}"),
            Self::UnknownPort(port) => {
                write!(formatter, "port {port:?} is absent from the netlist")
            }
            Self::PortIndexOverflow { component, index } => write!(
                formatter,
                "port index {index} on entity {} exceeds the supported range",
                component.value()
            ),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<ModelError> for CompileError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_model::{Component, Connection, Point, Port, PortDirection, SignalWidth};

    fn component(id: u64) -> Component {
        let port = |name| {
            Port::new(
                name,
                PortDirection::Bidirectional,
                SignalWidth::new(1).expect("one bit is valid"),
            )
            .expect("port is valid")
        };
        Component::new(
            EntityId::new(id),
            "probe",
            Point::new(i32::try_from(id).expect("test identity fits in a point"), 0),
            vec![port("a"), port("b")],
        )
        .expect("component is valid")
    }

    #[test]
    fn compiler_merges_transitive_connections_deterministically() {
        let mut circuit = Circuit::new();
        circuit
            .add_component(component(1))
            .expect("component is valid");
        circuit
            .add_component(component(2))
            .expect("component is valid");
        circuit
            .add_component(component(3))
            .expect("component is valid");
        circuit
            .connect(Connection::new(
                PortRef::new(EntityId::new(1), 0),
                PortRef::new(EntityId::new(2), 0),
            ))
            .expect("connection is valid");
        circuit
            .connect(Connection::new(
                PortRef::new(EntityId::new(2), 0),
                PortRef::new(EntityId::new(3), 1),
            ))
            .expect("connection is valid");

        let netlist = compile(&circuit).expect("circuit compiles");
        let net = netlist
            .net_for(PortRef::new(EntityId::new(1), 0))
            .expect("connected port has a net");
        assert_eq!(net.ports().len(), 3);
        assert_eq!(net.id(), 0);
        assert_eq!(netlist.nets().len(), 4);
    }
}
