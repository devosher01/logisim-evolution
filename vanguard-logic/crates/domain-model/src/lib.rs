//! Pure document model for the digital design product.
//!
//! This crate deliberately has no UI, filesystem, simulator, or platform
//! dependencies. It owns only validated domain data and local invariants.

use std::fmt;

/// Stable identifier for an entity inside a document.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntityId(u64);

impl EntityId {
    /// Creates an identifier from its persisted representation.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the persisted representation.
    #[must_use]
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Integer coordinate in document space.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: i32,
    /// Vertical coordinate.
    pub y: i32,
}

impl Point {
    /// Creates a point.
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Valid signal width.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SignalWidth(u16);

impl SignalWidth {
    /// Creates a width, rejecting zero-width signals.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::ZeroSignalWidth`] when `value` is zero.
    pub const fn new(value: u16) -> Result<Self, ModelError> {
        if value == 0 {
            Err(ModelError::ZeroSignalWidth)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the number of bits.
    #[must_use]
    pub const fn bits(self) -> u16 {
        self.0
    }
}

/// Direction of a component port.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PortDirection {
    /// Port receives a signal.
    Input,
    /// Port drives a signal.
    Output,
    /// Port can receive and drive a signal.
    Bidirectional,
}

/// A port declaration owned by a component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Port {
    name: String,
    direction: PortDirection,
    width: SignalWidth,
}

impl Port {
    /// Creates a validated port declaration.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::EmptyName`] when `name` is blank.
    pub fn new(
        name: impl Into<String>,
        direction: PortDirection,
        width: SignalWidth,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ModelError::EmptyName);
        }
        Ok(Self {
            name,
            direction,
            width,
        })
    }

    /// Returns the port name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the port direction.
    #[must_use]
    pub const fn direction(&self) -> PortDirection {
        self.direction
    }

    /// Returns the port width.
    #[must_use]
    pub const fn width(&self) -> SignalWidth {
        self.width
    }
}

/// A component definition placed in a circuit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    id: EntityId,
    kind: String,
    position: Point,
    ports: Vec<Port>,
}

impl Component {
    /// Creates a component with a stable identity and validated kind.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::EmptyName`] for a blank kind or
    /// [`ModelError::ComponentWithoutPorts`] when `ports` is empty.
    pub fn new(
        id: EntityId,
        kind: impl Into<String>,
        position: Point,
        ports: Vec<Port>,
    ) -> Result<Self, ModelError> {
        let kind = kind.into();
        if kind.trim().is_empty() {
            return Err(ModelError::EmptyName);
        }
        if ports.is_empty() {
            return Err(ModelError::ComponentWithoutPorts);
        }
        Ok(Self {
            id,
            kind,
            position,
            ports,
        })
    }

    /// Returns the stable identity.
    #[must_use]
    pub const fn id(&self) -> EntityId {
        self.id
    }

    /// Returns the component kind.
    #[must_use]
    pub fn kind(&self) -> &str {
        &self.kind
    }

    /// Returns the component position.
    #[must_use]
    pub const fn position(&self) -> Point {
        self.position
    }

    /// Returns the declared ports.
    #[must_use]
    pub fn ports(&self) -> &[Port] {
        &self.ports
    }
}

/// A validated circuit document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Circuit {
    components: Vec<Component>,
}

impl Circuit {
    /// Creates an empty circuit.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Adds a component, rejecting duplicate identities.
    ///
    /// # Errors
    ///
    /// Returns [`ModelError::DuplicateEntity`] when another component already
    /// owns the same identity.
    pub fn add_component(&mut self, component: Component) -> Result<(), ModelError> {
        if self
            .components
            .iter()
            .any(|item| item.id() == component.id())
        {
            return Err(ModelError::DuplicateEntity(component.id()));
        }
        self.components.push(component);
        Ok(())
    }

    /// Returns components in insertion order.
    #[must_use]
    pub fn components(&self) -> &[Component] {
        &self.components
    }
}

/// Local domain invariant violation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelError {
    /// A name or kind was empty.
    EmptyName,
    /// A signal cannot contain zero bits.
    ZeroSignalWidth,
    /// Components must expose at least one port.
    ComponentWithoutPorts,
    /// An identity was already used in the circuit.
    DuplicateEntity(EntityId),
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("name cannot be empty"),
            Self::ZeroSignalWidth => formatter.write_str("signal width must be greater than zero"),
            Self::ComponentWithoutPorts => {
                formatter.write_str("component must have at least one port")
            }
            Self::DuplicateEntity(id) => write!(formatter, "entity {} already exists", id.value()),
        }
    }
}

impl std::error::Error for ModelError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn input_port() -> Port {
        Port::new(
            "input",
            PortDirection::Input,
            SignalWidth::new(1).expect("one bit is valid"),
        )
        .expect("port is valid")
    }

    #[test]
    fn circuit_rejects_duplicate_component_ids() {
        let component = Component::new(
            EntityId::new(7),
            "switch",
            Point::new(10, 20),
            vec![input_port()],
        )
        .expect("component is valid");
        let mut circuit = Circuit::new();
        circuit
            .add_component(component.clone())
            .expect("first insertion is valid");

        assert_eq!(
            circuit.add_component(component),
            Err(ModelError::DuplicateEntity(EntityId::new(7)))
        );
    }

    #[test]
    fn invalid_values_are_rejected_at_construction() {
        assert_eq!(SignalWidth::new(0), Err(ModelError::ZeroSignalWidth));
        assert_eq!(
            Port::new(" ", PortDirection::Input, SignalWidth::new(1).unwrap()),
            Err(ModelError::EmptyName)
        );
        assert_eq!(
            Component::new(EntityId::new(1), "gate", Point::new(0, 0), Vec::new()),
            Err(ModelError::ComponentWithoutPorts)
        );
    }
}
