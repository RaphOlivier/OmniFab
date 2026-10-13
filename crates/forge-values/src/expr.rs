//! Parseur et évaluateur du sous-ensemble déterministe d'expressions
//! (ADR-0002 : pas d'horloge, pas d'aléa, pas d'accès disque — le rejeu
//! du journal d'événements donne toujours le même résultat).

use std::collections::BTreeSet;

use crate::error::ValuesError;
use crate::NodeId;

/// Opérateur binaire.
#[derive(Debug, Clone)]
pub(crate) enum BinOp {
    /// Addition.
    Add,
    /// Soustraction.
    Sub,
    /// Multiplication.
    Mul,
    /// Division.
    Div,
}

/// Fonction déterministe utilisable dans une expression.
#[derive(Debug, Clone)]
pub(crate) enum Function {
    /// abs(x)
    Abs,
    /// sqrt(x)
    Sqrt,
    /// min(a, b)
    Min,
    /// max(a, b)
    Max,
    /// sin(x), x en radians
    Sin,
    /// cos(x), x en radians
    Cos,
    /// tan(x), x en radians
    Tan,
    /// floor(x)
    Floor,
    /// ceil(x)
    Ceil,
    /// round(x)
    Round,
}

impl Function {
    /// Nom des fonctions reconnues.
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "abs" => Some(Function::Abs),
            "sqrt" => Some(Function::Sqrt),
            "min" => Some(Function::Min),
            "max" => Some(Function::Max),
            "sin" => Some(Function::Sin),
            "cos" => Some(Function::Cos),
            "tan" => Some(Function::Tan),
            "floor" => Some(Function::Floor),
            "ceil" => Some(Function::Ceil),
            "round" => Some(Function::Round),
            _ => None,
        }
    }

    /// Nombre d'arguments attendus.
    fn arity(&self) -> usize {
        match self {
            Function::Min | Function::Max => 2,
            _ => 1,
        }
    }
}

/// Arbre de syntaxe d'une expression.
#[derive(Debug, Clone)]
pub(crate) enum Expr {
    /// Littéral numérique.
    Number(f64),
    /// Référence résolue vers un nœud du graphe.
    Ref(NodeId),
    /// Négation unaire.
    Negate(Box<Expr>),
    /// Opération binaire.
    Binary(BinOp, Box<Expr>, Box<Expr>),
    /// Appel de fonction.
    Call(Function, Vec<Expr>),
}

/// Jeton du lexer.
#[derive(Debug, Clone)]
enum Token {
    /// Littéral numérique.
    Number(f64),
    /// Identifiant (nom de valeur ou de fonction).
    Ident(String),
    /// +
    Plus,
    /// -
    Minus,
    /// *
    Star,
    /// /
    Slash,
    /// (
    LParen,
    /// )
    RParen,
    /// ,
    Comma,
}

fn lex(input: &str, node: &str) -> Result<Vec<Token>, ValuesError> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut text = String::new();
                let mut dots = 0usize;
                while let Some(&c2) = chars.peek() {
                    if c2.is_ascii_digit() {
                        text.push(c2);
                        chars.next();
                    } else if c2 == '.' && dots == 0 {
                        dots += 1;
                        text.push(c2);
                        chars.next();
                    } else {
                        break;
                    }
                }
                let value: f64 = text.parse().map_err(|_| ValuesError::InvalidExpression {
                    node: node.to_string(),
                    reason: format!("nombre invalide « {text} »"),
                })?;
                if !value.is_finite() {
                    return Err(ValuesError::InvalidExpression {
                        node: node.to_string(),
                        reason: format!("littéral non fini « {text} »"),
                    });
                }
                tokens.push(Token::Number(value));
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                chars.next();
                let mut name = String::new();
                name.push(c);
                while let Some(&c2) = chars.peek() {
                    if c2.is_ascii_alphanumeric() || c2 == '_' {
                        name.push(c2);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(name));
            }
            '+' => {
                chars.next();
                tokens.push(Token::Plus);
            }
            '-' => {
                chars.next();
                tokens.push(Token::Minus);
            }
            '*' => {
                chars.next();
                tokens.push(Token::Star);
            }
            '/' => {
                chars.next();
                tokens.push(Token::Slash);
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            ',' => {
                chars.next();
                tokens.push(Token::Comma);
            }
            other => {
                return Err(ValuesError::InvalidExpression {
                    node: node.to_string(),
                    reason: format!("caractère inattendu « {other} »"),
                });
            }
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    node: &'a str,
    resolve: &'a dyn Fn(&str) -> Option<NodeId>,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.get(self.pos).cloned()
    }

    fn expression(&mut self) -> Result<Expr, ValuesError> {
        let mut lhs = self.term()?;
        loop {
            match self.peek() {
                Some(Token::Plus) => {
                    self.pos += 1;
                    lhs = Expr::Binary(BinOp::Add, Box::new(lhs), Box::new(self.term()?));
                }
                Some(Token::Minus) => {
                    self.pos += 1;
                    lhs = Expr::Binary(BinOp::Sub, Box::new(lhs), Box::new(self.term()?));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn term(&mut self) -> Result<Expr, ValuesError> {
        let mut lhs = self.factor()?;
        loop {
            match self.peek() {
                Some(Token::Star) => {
                    self.pos += 1;
                    lhs = Expr::Binary(BinOp::Mul, Box::new(lhs), Box::new(self.factor()?));
                }
                Some(Token::Slash) => {
                    self.pos += 1;
                    lhs = Expr::Binary(BinOp::Div, Box::new(lhs), Box::new(self.factor()?));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }

    fn factor(&mut self) -> Result<Expr, ValuesError> {
        if matches!(self.peek(), Some(Token::Minus)) {
            self.pos += 1;
            return Ok(Expr::Negate(Box::new(self.factor()?)));
        }
        self.atom()
    }

    fn atom(&mut self) -> Result<Expr, ValuesError> {
        match self.tokens.get(self.pos).cloned() {
            Some(Token::Number(value)) => {
                self.pos += 1;
                Ok(Expr::Number(value))
            }
            Some(Token::Ident(name)) => {
                self.pos += 1;
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.pos += 1;
                    let function = Function::from_name(&name).ok_or_else(|| {
                        ValuesError::InvalidExpression {
                            node: self.node.to_string(),
                            reason: format!("fonction inconnue « {name} »"),
                        }
                    })?;
                    let expected_arity = function.arity();
                    let mut args: Vec<Expr> = Vec::new();
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        loop {
                            args.push(self.expression()?);
                            if matches!(self.peek(), Some(Token::Comma)) {
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if !matches!(self.peek(), Some(Token::RParen)) {
                        return Err(ValuesError::InvalidExpression {
                            node: self.node.to_string(),
                            reason: "« ) » attendue".to_string(),
                        });
                    }
                    self.pos += 1;
                    if args.len() != expected_arity {
                        return Err(ValuesError::InvalidExpression {
                            node: self.node.to_string(),
                            reason: format!(
                                "la fonction « {name} » attend {expected_arity} argument(s), {} reçu(s)",
                                args.len()
                            ),
                        });
                    }
                    Ok(Expr::Call(function, args))
                } else {
                    match (self.resolve)(&name) {
                        Some(id) => Ok(Expr::Ref(id)),
                        None => Err(ValuesError::UnknownReference {
                            node: self.node.to_string(),
                            name,
                        }),
                    }
                }
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let inner = self.expression()?;
                if !matches!(self.peek(), Some(Token::RParen)) {
                    return Err(ValuesError::InvalidExpression {
                        node: self.node.to_string(),
                        reason: "« ) » attendue".to_string(),
                    });
                }
                self.pos += 1;
                Ok(inner)
            }
            Some(other) => Err(ValuesError::InvalidExpression {
                node: self.node.to_string(),
                reason: format!("token inattendu « {other:?} »"),
            }),
            None => Err(ValuesError::InvalidExpression {
                node: self.node.to_string(),
                reason: "expression incomplète".to_string(),
            }),
        }
    }
}

/// Parse `input` en arbre de syntaxe, en résolvant les noms référencés via
/// `resolve`. Les références introuvables produisent une erreur OF-VAL-0002.
pub(crate) fn parse(
    input: &str,
    node: &str,
    resolve: &dyn Fn(&str) -> Option<NodeId>,
) -> Result<Expr, ValuesError> {
    let tokens = lex(input, node)?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        node,
        resolve,
    };
    let parsed = parser.expression()?;
    if parser.pos != parser.tokens.len() {
        let reason = match parser.tokens.get(parser.pos) {
            Some(token) => format!("token inattendu « {token:?} »"),
            None => "fin d'expression inattendue".to_string(),
        };
        return Err(ValuesError::InvalidExpression {
            node: node.to_string(),
            reason,
        });
    }
    Ok(parsed)
}

/// Rejette un résultat non fini : ni NaN ni un infini ne doit pouvoir
/// entrer dans le journal via une valeur calculée (OF-VAL-0004).
fn check_finite(value: f64, node: &str) -> Result<f64, ValuesError> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ValuesError::Evaluation {
            node: node.to_string(),
            reason: "résultat non fini (NaN ou infini)".to_string(),
        })
    }
}

/// Évalue une expression ; `deps` lit (et recalcule si nécessaire) la valeur
/// d'un nœud référencé. Déterministe, sans effet de bord (ADR-0002).
pub(crate) fn evaluate(
    expr: &Expr,
    node: &str,
    deps: &mut dyn FnMut(NodeId) -> Result<f64, ValuesError>,
) -> Result<f64, ValuesError> {
    match expr {
        Expr::Number(value) => Ok(*value),
        Expr::Ref(id) => check_finite(deps(*id)?, node),
        Expr::Negate(inner) => check_finite(-evaluate(inner, node, deps)?, node),
        Expr::Binary(op, lhs, rhs) => {
            let left = evaluate(lhs, node, deps)?;
            let right = evaluate(rhs, node, deps)?;
            let result = match op {
                BinOp::Add => left + right,
                BinOp::Sub => left - right,
                BinOp::Mul => left * right,
                BinOp::Div => {
                    if right == 0.0 {
                        return Err(ValuesError::Evaluation {
                            node: node.to_string(),
                            reason: "division par zéro".to_string(),
                        });
                    }
                    left / right
                }
            };
            check_finite(result, node)
        }
        Expr::Call(function, args) => {
            let mut values: Vec<f64> = Vec::with_capacity(args.len());
            for arg in args {
                values.push(evaluate(arg, node, deps)?);
            }
            let result = match function {
                Function::Abs => values[0].abs(),
                Function::Sqrt => {
                    if values[0] < 0.0 {
                        return Err(ValuesError::Evaluation {
                            node: node.to_string(),
                            reason: "racine carrée d'un nombre négatif".to_string(),
                        });
                    }
                    libm::sqrt(values[0])
                }
                Function::Min => values[0].min(values[1]),
                Function::Max => values[0].max(values[1]),
                Function::Sin => libm::sin(values[0]),
                Function::Cos => libm::cos(values[0]),
                Function::Tan => libm::tan(values[0]),
                Function::Floor => values[0].floor(),
                Function::Ceil => values[0].ceil(),
                Function::Round => values[0].round(),
            };
            check_finite(result, node)
        }
    }
}

/// Collecte les nœuds référencés par une expression.
pub(crate) fn collect_references(expr: &Expr, out: &mut BTreeSet<NodeId>) {
    match expr {
        Expr::Number(_) => {}
        Expr::Ref(id) => {
            out.insert(*id);
        }
        Expr::Negate(inner) => collect_references(inner, out),
        Expr::Binary(_, lhs, rhs) => {
            collect_references(lhs, out);
            collect_references(rhs, out);
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_references(arg, out);
            }
        }
    }
}
