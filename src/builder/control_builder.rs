use crate::arena::DataArena;
use crate::logic::ControlOp;
use crate::logic::{Logic, OperatorType};

/// Builder for control flow operations.
///
/// This builder provides a fluent interface for creating control flow operations
/// such as if, and, or, etc.
pub struct ControlBuilder<'a> {
    /// The arena in which all allocations will be made.
    arena: &'a DataArena,
}

impl<'a> ControlBuilder<'a> {
    /// Creates a new control flow builder.
    pub fn new(arena: &'a DataArena) -> Self {
        Self { arena }
    }

    /// Creates an 'and' operation.
    pub fn and_op(&self) -> LogicalOperationBuilder<'a> {
        LogicalOperationBuilder::new(self.arena, ControlOp::And)
    }

    /// Creates an 'or' operation.
    pub fn or_op(&self) -> LogicalOperationBuilder<'a> {
        LogicalOperationBuilder::new(self.arena, ControlOp::Or)
    }

    /// Creates a 'not' operation.
    pub fn not_op(&self, value: Logic<'a>) -> Logic<'a> {
        Logic::operator(
            OperatorType::Control(ControlOp::Not),
            vec![value],
            self.arena,
        )
    }

    /// Creates an 'if' operation builder.
    pub fn if_op(&self) -> IfBuilder<'a> {
        IfBuilder::new(self.arena)
    }
}

/// Builder for logical operations (AND, OR).
pub struct LogicalOperationBuilder<'a> {
    /// The arena in which all allocations will be made.
    arena: &'a DataArena,
    /// The logical operation to use.
    operation: ControlOp,
    /// The operands collected so far.
    operands: Vec<Logic<'a>>,
}

impl<'a> LogicalOperationBuilder<'a> {
    /// Creates a new logical operation builder.
    pub fn new(arena: &'a DataArena, operation: ControlOp) -> Self {
        Self {
            arena,
            operation,
            operands: Vec::new(),
        }
    }

    /// Adds an operand to the logical operation.
    pub fn operand(mut self, operand: Logic<'a>) -> Self {
        self.operands.push(operand);
        self
    }

    /// Adds a variable as an operand to the logical operation.
    pub fn var(mut self, path: &str) -> Self {
        let var = Logic::variable(path, None, self.arena);
        self.operands.push(var);
        self
    }

    /// Adds a literal value as an operand to the logical operation.
    pub fn value<T: Into<crate::value::DataValue<'a>>>(mut self, value: T) -> Self {
        let val = Logic::literal(value.into(), self.arena);
        self.operands.push(val);
        self
    }

    /// Adds an integer as an operand to the logical operation.
    pub fn int(mut self, value: i64) -> Self {
        let val = Logic::literal(crate::value::DataValue::integer(value), self.arena);
        self.operands.push(val);
        self
    }

    /// Adds a float as an operand to the logical operation.
    pub fn float(mut self, value: f64) -> Self {
        let val = Logic::literal(crate::value::DataValue::float(value), self.arena);
        self.operands.push(val);
        self
    }

    /// Adds a string as an operand to the logical operation.
    pub fn string(mut self, value: &str) -> Self {
        let val = Logic::literal(
            crate::value::DataValue::string(self.arena, value),
            self.arena,
        );
        self.operands.push(val);
        self
    }

    /// Adds a boolean as an operand to the logical operation.
    pub fn bool(mut self, value: bool) -> Self {
        let val = Logic::literal(crate::value::DataValue::bool(value), self.arena);
        self.operands.push(val);
        self
    }

    /// Builds the logical operation with the collected operands.
    ///
    /// If no operands have been added, it will use a literal true or false
    /// depending on the operation (true for AND, false for OR).
    pub fn build(self) -> Logic<'a> {
        if self.operands.is_empty() {
            // Default for AND is true, for OR is false
            let default_value = match self.operation {
                ControlOp::And => true,
                ControlOp::Or => false,
                _ => true, // Shouldn't happen for this builder
            };
            return Logic::literal(crate::value::DataValue::bool(default_value), self.arena);
        }

        Logic::operator(
            OperatorType::Control(self.operation),
            self.operands,
            self.arena,
        )
    }
}

/// Builder for if-then-else operations.
pub struct IfBuilder<'a> {
    /// The arena in which all allocations will be made.
    arena: &'a DataArena,
    /// The condition.
    condition: Option<Logic<'a>>,
    /// The 'then' branch.
    then_branch: Option<Logic<'a>>,
    /// The 'else' branch.
    else_branch: Option<Logic<'a>>,
}

impl<'a> IfBuilder<'a> {
    /// Creates a new if-then-else builder.
    pub fn new(arena: &'a DataArena) -> Self {
        Self {
            arena,
            condition: None,
            then_branch: None,
            else_branch: None,
        }
    }

    /// Sets the condition for the if-then-else operation.
    pub fn condition(mut self, condition: Logic<'a>) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Sets the 'then' branch of the if-then-else operation.
    pub fn then(mut self, then_branch: Logic<'a>) -> Self {
        self.then_branch = Some(then_branch);
        self
    }

    /// Sets the 'else' branch of the if-then-else operation.
    pub fn else_branch(mut self, else_branch: Logic<'a>) -> Self {
        self.else_branch = Some(else_branch);
        self
    }

    /// Builds the if-then-else operation.
    ///
    /// If condition is not set, it will use a literal false.
    /// If then branch is not set, it will use a literal true.
    /// If else branch is not set, it will use a literal false.
    pub fn build(self) -> Logic<'a> {
        let condition = self
            .condition
            .unwrap_or_else(|| Logic::literal(crate::value::DataValue::bool(false), self.arena));

        let then_branch = self
            .then_branch
            .unwrap_or_else(|| Logic::literal(crate::value::DataValue::bool(true), self.arena));

        let else_branch = self
            .else_branch
            .unwrap_or_else(|| Logic::literal(crate::value::DataValue::bool(false), self.arena));

        // If-then-else is represented as an array where the first element is the condition,
        // the second is the then branch, and the third is the else branch.
        Logic::operator(
            OperatorType::Control(ControlOp::If),
            vec![condition, then_branch, else_branch],
            self.arena,
        )
    }
}
