use super::*;
use super::super::error::Response::Wrong;

use std::fmt::{ self, Write, Formatter, Display, };

#[derive(Debug, Clone)]
pub enum TypeNode {
  Int,
  Float,
  Number,
  Bool,
  Str,
  Char,
  Nil,
  Id(String),
  Set(Vec<Type>),
}

impl PartialEq for TypeNode {
  fn eq(&self, other: &TypeNode) -> bool {
    use self::TypeNode::*;

    match (self, other) {
      (&Int, &Int)       => true,
      (&Int, &Number)    => true,
      (&Number, &Int)    => true,
      (&Float, &Float)   => true,
      (&Float, &Number)  => true,
      (&Number, &Float)  => true,
      (&Number, &Number) => true,
      (&Bool, &Bool)     => true,
      (&Str, &Str)       => true,
      (&Char, &Char)     => true,
      (&Nil, &Nil)       => true,
      (&Id(ref a), &Id(ref b))   => a == b,
      (&Set(ref a), &Set(ref b)) => a == b,
      _                          => false,
    }
  }
}



#[derive(Debug, Clone)]
pub enum TypeMode {
  Undeclared,
  Immutable,
  Optional,
  Regular,
}

impl Display for TypeNode {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    use self::TypeNode::*;

    match *self {
      Number    => write!(f, "number"),
      Int       => write!(f, "int"),
      Float     => write!(f, "float"),
      Bool      => write!(f, "bool"),
      Str       => write!(f, "string"),
      Char      => write!(f, "char"),
      Nil       => write!(f, "nil"),
      Id(ref n) => write!(f, "{}", n),
      Set(ref content) => {
        write!(f, "(");

        for element in content {
          write!(f, "{},", element)?
        }

        write!(f, ")")
      },
    }
  }
}



impl TypeMode {
  pub fn check(&self, other: &TypeMode) -> bool {
    use self::TypeMode::{ Optional, Immutable, Regular, Undeclared, };

    match (self, other) {
      (&Regular,       &Regular)    => true,
      (&Immutable,     &Immutable)  => true,
      (&Undeclared,    &Undeclared) => true,
      (&Optional,      &Optional)   => true,
      _                             => false,
    }
  }
}

impl Display for TypeMode {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    use self::TypeMode::*;

    match *self {
      Regular    => Ok(()),
      Immutable  => write!(f, "constant "),
      Undeclared => write!(f, "undeclared "),
      Optional   => write!(f, "optional "),
    }
  }
}

impl PartialEq for TypeMode {
  fn eq(&self, other: &TypeMode) -> bool {
    use self::TypeMode::*;

    match (self, other) {
      (&Regular,    &Regular)    => true,
      (&Regular,    &Immutable)  => true,
      (&Immutable,  &Immutable)  => true,
      (&Immutable,  &Regular)    => true,
      (_,           &Optional)   => true,
      (&Optional,   _)           => true,
      (&Undeclared, _)           => false,
      (_,           &Undeclared) => false,
    }
  }
}



#[derive(Debug, Clone, PartialEq)]
pub struct Type {
  pub node: TypeNode,
  pub mode: TypeMode,
}

impl Type {
  pub fn new(node: TypeNode, mode: TypeMode) -> Self {
    Type {
      node, mode,
    }
  }

  pub fn id(id: &str) -> Type {
    Type::new(TypeNode::Id(id.to_owned()), TypeMode::Regular)
  }

  pub fn number() -> Type {
    Type::new(TypeNode::Number, TypeMode::Regular)
  }

  pub fn int() -> Type {
    Type::new(TypeNode::Int, TypeMode::Regular)
  }

  pub fn float() -> Type {
    Type::new(TypeNode::Float, TypeMode::Regular)
  }

  pub fn string() -> Type {
    Type::new(TypeNode::Str, TypeMode::Regular)
  }

  pub fn char() -> Type {
    Type::new(TypeNode::Char, TypeMode::Regular)
  }

  pub fn bool() -> Type {
    Type::new(TypeNode::Bool, TypeMode::Regular)
  }

  pub fn nil() -> Type {
    Type::new(TypeNode::Nil, TypeMode::Regular)
  }
}

impl Display for Type {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", self.mode, self.node)
    }
}



pub struct Visitor<'v> {
  pub symtab:  SymTab<'v>,
  pub typetab: TypeTab<'v>,
  pub source:  &'v Source,
  pub ast:     &'v Vec<Statement<'v>>,
}

impl<'v> Visitor<'v> {
  pub fn new(source: &'v Source, ast: &'v Vec<Statement<'v>>) -> Self {
    Visitor {
      symtab:  SymTab::global(),
      typetab: TypeTab::global(),
      source,
      ast,
    }
  }

  pub fn visit(&mut self) -> Result<(), ()> {
    for statement in self.ast {
      self.visit_statement(&statement)?
    }

    Ok(())
  }

  pub fn visit_statement(&mut self, statement: &'v Statement<'v>) -> Result<(), ()> {
    use self::StatementNode::*;

    match statement.node {
      Expression(ref expression) => self.visit_expression(expression),

      Variable(_, ref left, _) => match left.node {
        ExpressionNode::Identifier(_) | ExpressionNode::Set(_) => self.visit_variable(&statement.node),
        _ => Ok(())
      },

      Constant(_, ref left, _) => match left.node {
        ExpressionNode::Identifier(_) | ExpressionNode::Set(_) => self.visit_constant(&statement.node),
        _ => Ok(())
      },
    }
  }

  fn visit_expression(&mut self, expression: &'v Expression<'v>) -> Result<(), ()> {
    use self::ExpressionNode::*;

    match expression.node {
      Identifier(ref name) => if self.symtab.get_name(name).is_none() {
        Err(
          response!(
            Wrong(format!("no such value `{}` in this scope", name)),
            self.source.file,
            expression.pos
          )
        )
      } else {
        Ok(())
      },

      Set(ref content) => {
        for expression in content {
          self.visit_expression(expression)?
        }

        Ok(())
      }

      Block(ref statements) => {
        for statement in statements {
          self.visit_statement(statement)?
        }

        Ok(())
      }

      _ => Ok(())
    }
  }

  fn visit_variable(&mut self, variable: &'v StatementNode) -> Result<(), ()> {
    use self::ExpressionNode::{Identifier, Set};

    if let &StatementNode::Variable(ref variable_type, ref left, ref right) = variable {
      match left.node {
        Identifier(ref name) => {
          let index = if let Some((index, _)) = self.symtab.get_name(name) {
            index
          } else {
            self.symtab.add_name(name)
          };

          self.typetab.grow();

          if let &Some(ref right) = right {
            self.visit_expression(&right)?;

            let right_type = self.type_expression(&right)?;

            if variable_type.node != TypeNode::Nil {
              if variable_type != &right_type {
                return Err(
                  response!(
                    Wrong(format!("mismatched types, expected type `{}` got `{}`", variable_type.node, right_type)),
                    self.source.file,
                    right.pos
                  )
                )
              } else {
                self.typetab.set_type(index, 0, variable_type.to_owned())?
              }
            } else {
              self.typetab.set_type(index, 0, right_type)?
            }
          } else {
            self.typetab.set_type(index, 0, variable_type.to_owned())?
          }
        },

        Set(ref names) => {
          for expression in names {
            if let Identifier(ref name) = expression.node {
              self.symtab.add_name(name);
            } else {
              return Err(
                response!(
                  Wrong("can't declare non-identifier"),
                  self.source.file,
                  expression.pos
                )
              )
            }
          }
        }

        _ => return Err(
          response!(
            Wrong("unexpected variable declaration"),
            self.source.file,
            left.pos
          )
        )
      }

      Ok(())
    } else {
      unreachable!()
    }
  }

  fn visit_constant(&mut self, constant: &'v StatementNode) -> Result<(), ()> {
    use self::ExpressionNode::{Identifier, Set};

    if let &StatementNode::Constant(ref constant_type, ref left, ref right) = constant {
      match left.node {
        Identifier(ref name) => {
          let index = if let Some((index, _)) = self.symtab.get_name(name) {
            index
          } else {
            self.symtab.add_name(name)
          };

          self.typetab.grow();

          self.visit_expression(&right)?;

          let right_type = self.type_expression(right)?;

          if constant_type.node != TypeNode::Nil {
            if constant_type != &right_type {
              return Err(
                response!(
                  Wrong(format!("mismatched types, expected type `{}` got `{}`", constant_type.node, right_type)),
                  self.source.file,
                  right.pos
                )
              )
            } else {
              self.typetab.set_type(index, 0, constant_type.to_owned())?
            }
          } else {
            self.typetab.set_type(index, 0, right_type)?
          }
        },

        Set(ref names) => {
          for expression in names {
            if let Identifier(ref name) = expression.node {
              self.symtab.add_name(name);
            } else {
              return Err(
                response!(
                  Wrong("can't declare non-identifier"),
                  self.source.file,
                  expression.pos
                )
              )
            }
          }
        }

        _ => return Err(
          response!(
            Wrong("unexpected constant declaration"),
            self.source.file,
            left.pos
          )
        )
      }

      Ok(())
    } else {
      unreachable!()
    }
  }



  fn type_expression(&mut self, expression: &'v Expression<'v>) -> Result<Type, ()> {
    use self::ExpressionNode::*;

    let t = match expression.node {
      Identifier(ref name) => if let Some((index, env_index)) = self.symtab.get_name(name) {
        self.typetab.get_type(index, env_index)?
      } else {
        unreachable!()
      },

      String(_) => Type::string(),
      Char(_)   => Type::char(),
      Bool(_)   => Type::bool(),
      Number(_) => Type::number(),

      _ => Type::nil()
    };

    Ok(t)
  }
}