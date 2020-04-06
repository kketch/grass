use std::iter::Peekable;

use crate::common::{Brackets, ListSeparator, Pos};
use crate::error::SassResult;
use crate::scope::Scope;
use crate::selector::Selector;
use crate::utils::{
    devour_whitespace, eat_ident, read_until_closing_curly_brace, read_until_open_curly_brace,
    read_until_semicolon_or_closing_curly_brace,
};
use crate::value::Value;
use crate::{RuleSet, Stmt, Token};

pub(crate) use function::Function;
pub(crate) use if_rule::If;
pub(crate) use kind::AtRuleKind;
pub(crate) use mixin::{eat_include, Mixin};
use parse::{eat_stmts, eat_stmts_at_root};
use unknown::UnknownAtRule;

mod for_rule;
mod function;
mod if_rule;
mod kind;
mod mixin;
mod parse;
mod unknown;

#[derive(Debug, Clone)]
pub(crate) enum AtRule {
    Warn(Pos, String),
    Debug(Pos, String),
    Mixin(String, Box<Mixin>),
    Function(String, Box<Function>),
    Return(Vec<Token>),
    Charset,
    Content,
    Unknown(UnknownAtRule),
    For(Vec<Stmt>),
    Each(Vec<Stmt>),
    While(Vec<Stmt>),
    If(If),
    AtRoot(Vec<Stmt>),
}

impl AtRule {
    pub fn from_tokens<I: Iterator<Item = Token>>(
        rule: &AtRuleKind,
        pos: Pos,
        toks: &mut Peekable<I>,
        scope: &mut Scope,
        super_selector: &Selector,
    ) -> SassResult<AtRule> {
        devour_whitespace(toks);
        Ok(match rule {
            AtRuleKind::Error => {
                let message = Value::from_vec(
                    read_until_semicolon_or_closing_curly_brace(toks),
                    scope,
                    super_selector,
                )?;

                return Err(message.to_string().into());
            }
            AtRuleKind::Warn => {
                let message = Value::from_vec(
                    read_until_semicolon_or_closing_curly_brace(toks),
                    scope,
                    super_selector,
                )?;
                if toks.peek().unwrap().kind == ';' {
                    toks.next();
                }
                devour_whitespace(toks);
                AtRule::Warn(pos, message.to_string())
            }
            AtRuleKind::Debug => {
                let message = Value::from_vec(
                    read_until_semicolon_or_closing_curly_brace(toks),
                    scope,
                    super_selector,
                )?;
                if toks.peek().unwrap().kind == ';' {
                    toks.next();
                }
                devour_whitespace(toks);
                AtRule::Debug(pos, message.inspect())
            }
            AtRuleKind::Mixin => {
                let (name, mixin) = Mixin::decl_from_tokens(toks, scope, super_selector)?;
                AtRule::Mixin(name, Box::new(mixin))
            }
            AtRuleKind::Function => {
                let (name, func) = Function::decl_from_tokens(toks, scope.clone(), super_selector)?;
                AtRule::Function(name, Box::new(func))
            }
            AtRuleKind::Return => {
                let v = read_until_semicolon_or_closing_curly_brace(toks);
                if toks.peek().unwrap().kind == ';' {
                    toks.next();
                }
                devour_whitespace(toks);
                AtRule::Return(v)
            }
            AtRuleKind::Use => todo!("@use not yet implemented"),
            AtRuleKind::Annotation => todo!("@annotation not yet implemented"),
            AtRuleKind::AtRoot => {
                let mut selector = &Selector::replace(
                    super_selector,
                    Selector::from_tokens(
                        &mut read_until_open_curly_brace(toks).into_iter().peekable(),
                        scope,
                        super_selector,
                    )?,
                );
                let mut is_some = true;
                if selector.is_empty() {
                    is_some = false;
                    selector = super_selector;
                }
                toks.next();
                devour_whitespace(toks);
                let mut body = read_until_closing_curly_brace(toks);
                body.push(toks.next().unwrap());
                devour_whitespace(toks);
                let mut styles = Vec::new();
                let raw_stmts = eat_stmts_at_root(
                    &mut body.into_iter().peekable(),
                    scope,
                    selector,
                    0,
                    is_some,
                )?
                .into_iter()
                .filter_map(|s| match s {
                    Stmt::Style(..) => {
                        styles.push(s);
                        None
                    }
                    _ => Some(s),
                })
                .collect::<Vec<Stmt>>();
                let mut stmts = vec![Stmt::RuleSet(RuleSet {
                    selector: selector.clone(),
                    rules: styles,
                    super_selector: Selector::new(),
                })];
                stmts.extend(raw_stmts);
                AtRule::AtRoot(stmts)
            }
            AtRuleKind::Charset => {
                read_until_semicolon_or_closing_curly_brace(toks);
                if toks.peek().unwrap().kind == ';' {
                    toks.next();
                }
                devour_whitespace(toks);
                AtRule::Charset
            }
            AtRuleKind::Each => {
                let mut stmts = Vec::new();
                devour_whitespace(toks);
                let mut vars = Vec::new();
                loop {
                    match toks.next().ok_or("expected \"$\".")?.kind {
                        '$' => vars.push(eat_ident(toks, scope, super_selector)?),
                        _ => return Err("expected \"$\".".into()),
                    }
                    devour_whitespace(toks);
                    if toks.peek().ok_or("expected \"$\".")?.kind == ',' {
                        toks.next();
                        devour_whitespace(toks);
                    } else {
                        break;
                    }
                }
                if toks.peek().is_none()
                    || eat_ident(toks, scope, super_selector)?.to_ascii_lowercase() != "in"
                {
                    return Err("Expected \"in\".".into());
                }
                devour_whitespace(toks);
                let iterator = match Value::from_vec(
                    read_until_open_curly_brace(toks),
                    scope,
                    super_selector,
                )? {
                    Value::List(v, ..) => v,
                    Value::Map(m) => m
                        .into_iter()
                        .map(|(k, v)| Value::List(vec![k, v], ListSeparator::Space, Brackets::None))
                        .collect(),
                    v => vec![v],
                };
                toks.next();
                devour_whitespace(toks);
                let mut body = read_until_closing_curly_brace(toks);
                body.push(toks.next().unwrap());
                devour_whitespace(toks);

                for row in iterator {
                    let this_iterator = match row {
                        Value::List(v, ..) => v,
                        Value::Map(m) => m
                            .into_iter()
                            .map(|(k, v)| {
                                Value::List(vec![k, v], ListSeparator::Space, Brackets::None)
                            })
                            .collect(),
                        v => vec![v],
                    };

                    if vars.len() == 1 {
                        scope.insert_var(
                            &vars[0],
                            Value::List(this_iterator, ListSeparator::Space, Brackets::None),
                        )?;
                    } else {
                        for (var, val) in vars.clone().into_iter().zip(
                            this_iterator
                                .into_iter()
                                .chain(std::iter::once(Value::Null).cycle()),
                        ) {
                            scope.insert_var(&var, val)?;
                        }
                    }

                    stmts.extend(eat_stmts(
                        &mut body.clone().into_iter().peekable(),
                        scope,
                        super_selector,
                    )?);
                }
                AtRule::Each(stmts)
            }
            AtRuleKind::Extend => todo!("@extend not yet implemented"),
            AtRuleKind::If => AtRule::If(If::from_tokens(toks)?),
            AtRuleKind::Else => todo!("@else not yet implemented"),
            AtRuleKind::For => for_rule::parse_for(toks, scope, super_selector)?,
            AtRuleKind::While => {
                let mut stmts = Vec::new();
                devour_whitespace(toks);
                let cond = read_until_open_curly_brace(toks);

                if cond.is_empty() {
                    return Err("Expected expression.".into());
                }

                toks.next();
                let scope = &mut scope.clone();
                let body = read_until_closing_curly_brace(toks);
                toks.next();

                devour_whitespace(toks);

                while Value::from_vec(cond.clone(), scope, super_selector)?.is_true()? {
                    stmts.extend(eat_stmts(
                        &mut body.clone().into_iter().peekable(),
                        scope,
                        super_selector,
                    )?);
                }
                AtRule::While(stmts)
            }
            AtRuleKind::Keyframes => todo!("@keyframes not yet implemented"),
            AtRuleKind::Unknown(name) => AtRule::Unknown(UnknownAtRule::from_tokens(
                toks,
                name,
                scope,
                super_selector,
            )?),
            AtRuleKind::Content => AtRule::Content,
            _ => todo!("encountered unimplemented at rule"),
        })
    }
}
