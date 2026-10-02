//! Command engine for transactional document editing.
//!
//! Commands own their inverse operation. This keeps undo and redo independent
//! from UI concerns and makes every edit deterministic and testable.

use domain_model::{Circuit, Component, EntityId, ModelError, Point};
use std::fmt;

/// A reversible edit to a circuit document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Command {
    /// Inserts a new component.
    AddComponent(Component),
    /// Removes an existing component.
    RemoveComponent(EntityId),
    /// Changes the position of an existing component.
    MoveComponent {
        /// Identity of the component to move.
        id: EntityId,
        /// Destination position in document space.
        position: Point,
    },
}

impl Command {
    fn execute(self, circuit: &mut Circuit) -> Result<Self, EngineError> {
        match self {
            Self::AddComponent(component) => {
                let id = component.id();
                circuit.add_component(component)?;
                Ok(Self::RemoveComponent(id))
            }
            Self::RemoveComponent(id) => {
                let component = circuit.remove_component(id)?;
                let position = component.position();
                Ok(Self::AddComponent(component).with_position(position))
            }
            Self::MoveComponent { id, position } => {
                let previous = circuit
                    .component(id)
                    .ok_or(ModelError::UnknownEntity(id))?
                    .position();
                circuit.move_component(id, position)?;
                Ok(Self::MoveComponent {
                    id,
                    position: previous,
                })
            }
        }
    }

    fn with_position(self, position: Point) -> Self {
        match self {
            Self::AddComponent(mut component) => {
                component.move_to(position);
                Self::AddComponent(component)
            }
            command => command,
        }
    }
}

/// A document plus its reversible edit history.
#[derive(Debug, Default)]
pub struct DocumentEngine {
    circuit: Circuit,
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl DocumentEngine {
    /// Creates an empty document engine.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            circuit: Circuit::new(),
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    /// Returns the current immutable circuit state.
    #[must_use]
    pub const fn circuit(&self) -> &Circuit {
        &self.circuit
    }

    /// Applies a command and records its inverse for undo.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Model`] when the command violates a domain
    /// invariant.
    pub fn apply(&mut self, command: Command) -> Result<(), EngineError> {
        let inverse = command.execute(&mut self.circuit)?;
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }

    /// Undoes the most recent command.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when there is nothing to undo.
    pub fn undo(&mut self) -> Result<(), EngineError> {
        let inverse = self.undo.pop().ok_or(EngineError::HistoryEmpty)?;
        let redo = inverse.execute(&mut self.circuit)?;
        self.redo.push(redo);
        Ok(())
    }

    /// Redoes the most recently undone command.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::HistoryEmpty`] when there is nothing to redo.
    pub fn redo(&mut self) -> Result<(), EngineError> {
        let command = self.redo.pop().ok_or(EngineError::HistoryEmpty)?;
        let undo = command.execute(&mut self.circuit)?;
        self.undo.push(undo);
        Ok(())
    }

    /// Returns whether an undo operation is available.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Returns whether a redo operation is available.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

/// Failure while applying or traversing document history.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EngineError {
    /// The command violated a domain invariant.
    Model(ModelError),
    /// The requested history stack is empty.
    HistoryEmpty,
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(formatter, "document command failed: {error}"),
            Self::HistoryEmpty => formatter.write_str("document history is empty"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<ModelError> for EngineError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain_model::{Port, PortDirection, SignalWidth};

    fn switch(id: u64, position: Point) -> Component {
        let port = Port::new(
            "output",
            PortDirection::Output,
            SignalWidth::new(1).expect("one bit is valid"),
        )
        .expect("port is valid");
        Component::new(EntityId::new(id), "switch", position, vec![port])
            .expect("component is valid")
    }

    #[test]
    fn command_history_round_trips_a_move() {
        let mut engine = DocumentEngine::new();
        engine
            .apply(Command::AddComponent(switch(1, Point::new(0, 0))))
            .expect("component can be added");
        engine
            .apply(Command::MoveComponent {
                id: EntityId::new(1),
                position: Point::new(20, 30),
            })
            .expect("component can move");
        assert_eq!(
            engine
                .circuit()
                .component(EntityId::new(1))
                .unwrap()
                .position(),
            Point::new(20, 30)
        );

        engine.undo().expect("move can be undone");
        assert_eq!(
            engine
                .circuit()
                .component(EntityId::new(1))
                .unwrap()
                .position(),
            Point::new(0, 0)
        );
        engine.redo().expect("move can be redone");
        assert_eq!(
            engine
                .circuit()
                .component(EntityId::new(1))
                .unwrap()
                .position(),
            Point::new(20, 30)
        );
    }

    #[test]
    fn applying_after_undo_discards_redo_branch() {
        let mut engine = DocumentEngine::new();
        engine
            .apply(Command::AddComponent(switch(1, Point::new(0, 0))))
            .expect("component can be added");
        engine.undo().expect("addition can be undone");
        engine
            .apply(Command::AddComponent(switch(2, Point::new(10, 10))))
            .expect("new branch can be added");

        assert!(!engine.can_redo());
        assert_eq!(engine.circuit().components().len(), 1);
        assert_eq!(
            engine
                .circuit()
                .component(EntityId::new(2))
                .unwrap()
                .position(),
            Point::new(10, 10)
        );
    }
}
